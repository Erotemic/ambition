//! Sandbox cutscene player.
//!
//! Reads engine `CutsceneScript`s, runs them via `CutsceneRuntime`,
//! and applies side effects: write banners, pause player input,
//! flip save flags. The HUD picks up the active beat through
//! `ActiveCutscene::current_dialogue` / `current_banner`.
//!
//! Triggers are represented as world flags: a system somewhere else
//! writes `cutscene_intro_pending = true`, this player picks it up
//! and starts the cutscene. That keeps the activation surface tiny
//! and makes the cutscene system trivial to test.

use std::collections::BTreeMap;

use ambition_engine as ae;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct CutsceneLibrary {
    pub scripts: BTreeMap<String, ae::CutsceneScript>,
}

impl CutsceneLibrary {
    pub fn insert(&mut self, script: ae::CutsceneScript) {
        self.scripts.insert(script.id.clone(), script);
    }

    pub fn get(&self, id: &str) -> Option<&ae::CutsceneScript> {
        self.scripts.get(id)
    }
}

/// Live cutscene playback state. `Some` while a cutscene is running.
#[derive(Resource, Default)]
pub struct ActiveCutscene {
    pub runtime: Option<ae::CutsceneRuntime>,
    /// Last-seen dialogue line. Cleared when the beat advances.
    pub current_dialogue: Option<(String, String)>,
    /// Last-seen banner line + remaining seconds.
    pub current_banner: Option<(String, f32)>,
    /// Camera pan target (world coords) while a CameraPan beat is
    /// active. Consumers ease toward it.
    pub camera_target: Option<Vec2>,
    /// Fade overlay alpha [0, 1].
    pub fade_alpha: f32,
}

impl ActiveCutscene {
    pub fn is_playing(&self) -> bool {
        self.runtime.is_some()
    }

    pub fn freezes_player_input(&self) -> bool {
        self.is_playing()
    }
}

/// Default sandbox cutscenes shipped with the sandbox.
pub fn default_cutscene_library() -> CutsceneLibrary {
    let mut lib = CutsceneLibrary::default();
    lib.insert(
        ae::CutsceneScript::new(
            "test_intro",
            vec![
                ae::CutsceneBeat::Banner {
                    text: "// boot sequence".into(),
                    seconds: 1.4,
                },
                ae::CutsceneBeat::Fade {
                    to_alpha: 0.0,
                    seconds: 0.8,
                },
                ae::CutsceneBeat::Dialogue {
                    speaker: "WARDEN".into(),
                    text: "Instance online. You'll know your purpose when you find it.".into(),
                },
                ae::CutsceneBeat::SetFlag {
                    id: "test_intro_seen".into(),
                    on: true,
                },
            ],
        )
        .with_seen_flag("test_intro_seen"),
    );
    lib.insert(
        ae::CutsceneScript::new(
            "boss_intro_gradient_sentinel",
            vec![
                ae::CutsceneBeat::Banner {
                    text: "GRADIENT SENTINEL".into(),
                    seconds: 1.6,
                },
                ae::CutsceneBeat::Wait { seconds: 0.4 },
                ae::CutsceneBeat::Dialogue {
                    speaker: "SENTINEL".into(),
                    text: "Your loss surface is steep. I am its slope.".into(),
                },
            ],
        )
        .with_seen_flag("boss_intro_gradient_sentinel_seen"),
    );
    lib
}

/// Trigger queue: anyone can push a cutscene id and the player picks
/// it up. Cleaner than Bevy events for the simple "fire once when X
/// happens" pattern.
#[derive(Resource, Default)]
pub struct CutsceneTriggerQueue(pub Vec<String>);

/// Mapping from room id → cutscene id to play the first time the
/// player walks into that room. Drained by `auto_trigger_room_cutscenes`.
#[derive(Resource, Default)]
pub struct RoomCutsceneBindings {
    pub bindings: Vec<(String, String)>,
}

impl RoomCutsceneBindings {
    pub fn defaults() -> Self {
        Self {
            bindings: vec![
                // Plays the first time the player enters the hub.
                ("central_hub_main".into(), "test_intro".into()),
                // Plays the first time the player enters the
                // (existing) basement boss arena. The `seen_flag`
                // guards against replays.
                (
                    "basement_boss".into(),
                    "boss_intro_gradient_sentinel".into(),
                ),
            ],
        }
    }
}

/// Bevy system: when the active room changes, queue up a cutscene if
/// the new room has a binding and the cutscene hasn't been seen.
pub fn auto_trigger_room_cutscenes(
    bindings: Res<RoomCutsceneBindings>,
    room_set: Res<crate::rooms::RoomSet>,
    mut queue: ResMut<CutsceneTriggerQueue>,
    mut last_room: Local<Option<String>>,
) {
    let current = room_set.active_spec().id.clone();
    let changed = last_room.as_deref() != Some(current.as_str());
    if !changed {
        return;
    }
    *last_room = Some(current.clone());
    for (room_id, cutscene_id) in &bindings.bindings {
        if room_id == &current {
            queue.request(cutscene_id);
        }
    }
}

impl CutsceneTriggerQueue {
    pub fn request(&mut self, id: impl Into<String>) {
        self.0.push(id.into());
    }
}

/// Drain the trigger queue: start the next cutscene if one isn't
/// already playing. Skips any that have already had their seen flag
/// set.
pub fn drain_cutscene_triggers(
    mut queue: ResMut<CutsceneTriggerQueue>,
    library: Res<CutsceneLibrary>,
    mut active: ResMut<ActiveCutscene>,
    save: Res<crate::save::SandboxSave>,
) {
    if active.is_playing() {
        return;
    }
    let pending = std::mem::take(&mut queue.0);
    for id in pending {
        let Some(script) = library.get(&id) else {
            continue;
        };
        if let Some(seen) = script.seen_flag.as_ref() {
            if save.data().flag(seen) {
                continue;
            }
        }
        active.runtime = Some(ae::CutsceneRuntime::new(script.clone()));
        active.current_dialogue = None;
        active.current_banner = None;
        active.camera_target = None;
        active.fade_alpha = 0.0;
        break;
    }
}

/// Hold duration in seconds the player must keep the skip button held
/// before the cutscene actually skips. Long enough that an accidental
/// tap can't burn through scripted content.
pub const SKIP_HOLD_THRESHOLD_SECS: f32 = 1.2;

/// Tick the active cutscene. The advance signal comes from the input
/// layer (it sets `runtime.advance_dialogue` via the
/// `CutsceneAdvanceRequest` resource so the simulation half doesn't
/// import keyboard state).
///
/// `skip_hold_seconds` is presentation-readable so the HUD can render
/// a "hold to skip" progress bar. The input layer accumulates it
/// while the player is holding the skip button and zeroes it on
/// release. The simulation half flips `skip_cutscene = true` when
/// `skip_hold_seconds >= SKIP_HOLD_THRESHOLD_SECS`; the actual
/// cutscene-skip path consumes `skip_cutscene` and is unchanged.
#[derive(Resource, Default)]
pub struct CutsceneAdvanceRequest {
    pub dismiss_dialogue: bool,
    pub skip_cutscene: bool,
    pub skip_hold_seconds: f32,
}

impl CutsceneAdvanceRequest {
    /// Fraction of the way through the skip-hold window. Useful for
    /// HUD progress bars; clamped to `[0, 1]`.
    pub fn skip_progress(&self) -> f32 {
        if SKIP_HOLD_THRESHOLD_SECS <= 0.0 {
            return 1.0;
        }
        (self.skip_hold_seconds / SKIP_HOLD_THRESHOLD_SECS).clamp(0.0, 1.0)
    }
}

pub fn tick_active_cutscene(
    time: Res<Time>,
    mut active: ResMut<ActiveCutscene>,
    mut request: ResMut<CutsceneAdvanceRequest>,
    mut save: ResMut<crate::save::SandboxSave>,
) {
    let dismiss = std::mem::take(&mut request.dismiss_dialogue);
    let skip = std::mem::take(&mut request.skip_cutscene);
    let dt = time.delta_secs();

    let Some(runtime) = active.runtime.as_mut() else {
        return;
    };

    if skip {
        let _ = runtime.skip();
        if let Some(seen) = runtime.script.seen_flag.clone() {
            save.data_mut().set_flag(seen, true);
        }
        active.runtime = None;
        active.current_dialogue = None;
        active.current_banner = None;
        active.camera_target = None;
        active.fade_alpha = 0.0;
        return;
    }

    let events = runtime.tick(dt, dismiss);
    let mut completed = false;
    for event in events {
        match event {
            ae::CutsceneEvent::BeatEntered { beat, .. } => match beat {
                ae::CutsceneBeat::Dialogue { speaker, text } => {
                    active.current_dialogue = Some((speaker, text));
                    active.current_banner = None;
                }
                ae::CutsceneBeat::Banner { text, seconds } => {
                    active.current_dialogue = None;
                    active.current_banner = Some((text, seconds));
                }
                ae::CutsceneBeat::CameraPan { target, .. } => {
                    active.camera_target = Some(Vec2::new(target[0], target[1]));
                }
                ae::CutsceneBeat::Fade { to_alpha, .. } => {
                    active.fade_alpha = to_alpha.clamp(0.0, 1.0);
                }
                _ => {}
            },
            ae::CutsceneEvent::FlagWritten { id, on } => {
                save.data_mut().set_flag(id, on);
            }
            ae::CutsceneEvent::Skipped | ae::CutsceneEvent::Completed => {
                completed = true;
            }
        }
    }
    // Banner countdown — purely presentational so the HUD can fade out.
    if let Some((_, remaining)) = active.current_banner.as_mut() {
        *remaining = (*remaining - dt).max(0.0);
    }

    if completed {
        if let Some(rt) = active.runtime.as_ref() {
            if let Some(seen) = rt.script.seen_flag.clone() {
                save.data_mut().set_flag(seen, true);
            }
        }
        active.runtime = None;
        active.current_dialogue = None;
        active.current_banner = None;
        active.camera_target = None;
        active.fade_alpha = 0.0;
    }
}

#[cfg(test)]
mod skip_request_tests {
    use super::*;

    #[test]
    fn skip_progress_is_zero_when_no_hold_active() {
        let req = CutsceneAdvanceRequest::default();
        assert_eq!(req.skip_progress(), 0.0);
    }

    #[test]
    fn skip_progress_clamps_to_one() {
        let req = CutsceneAdvanceRequest {
            skip_hold_seconds: SKIP_HOLD_THRESHOLD_SECS * 2.0,
            ..Default::default()
        };
        assert_eq!(req.skip_progress(), 1.0);
    }

    #[test]
    fn skip_progress_is_linear_within_window() {
        let req = CutsceneAdvanceRequest {
            skip_hold_seconds: SKIP_HOLD_THRESHOLD_SECS * 0.5,
            ..Default::default()
        };
        assert!((req.skip_progress() - 0.5).abs() < 1e-4);
    }
}
