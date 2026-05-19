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
            "cutscene_lab_intro",
            vec![
                ae::CutsceneBeat::Banner {
                    text: "// cutscene proof".into(),
                    seconds: 1.0,
                },
                ae::CutsceneBeat::Dialogue {
                    speaker: "WARDEN".into(),
                    text: "This is the cutscene-proof room. The seen-flag stops me from talking twice."
                        .into(),
                },
                ae::CutsceneBeat::Wait { seconds: 0.4 },
                ae::CutsceneBeat::Dialogue {
                    speaker: "WARDEN".into(),
                    text: "Hold Reset to skip cutscenes -- useful when you've heard a beat already."
                        .into(),
                },
                ae::CutsceneBeat::SetFlag {
                    id: "cutscene_lab_intro_seen".into(),
                    on: true,
                },
            ],
        )
        .with_seen_flag("cutscene_lab_intro_seen"),
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
                // Cutscene proof room reachable from the basement.
                // Demonstrates the entry-trigger + seen-flag + skip
                // flow on a non-default cutscene.
                ("cutscene_lab".into(), "cutscene_lab_intro".into()),
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
    save: Res<crate::persistence::save::SandboxSave>,
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
    mut save: ResMut<crate::persistence::save::SandboxSave>,
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

// ─────────────────────────────────────────────────────────────────────
// Presentation overlay
// ─────────────────────────────────────────────────────────────────────
//
// Two complementary surfaces drive narrative text in the sandbox:
//
// - **Cutscene overlay** (this module): a screen-space Bevy UI panel
//   that draws CutsceneBeat::Dialogue (acknowledge — waits for player
//   input) and CutsceneBeat::Banner (timed — auto-advances). The skip-
//   hold progress bar lives here too. Owned by [`sync_cutscene_ui`].
//
// - **Speech bubbles** (`crate::fx::update_speech_bubbles`): world-
//   space transient quote bubbles that anyone can fire via
//   `VfxMessage::SpeechBubble { pos, text }`. Already used by the
//   combat / damage path so enemies can shout when they get hit. The
//   "real-time dialog where characters just say thing" mode the
//   intro design doc calls for is this — no UI input, no pause, just
//   the line floats up and fades. The cutscene overlay never owns it.
//
// Both run unconditionally in the presentation half; headless / RL
// builds skip the registrations.

/// Root entity for the active cutscene UI panel. One per live cutscene;
/// despawned + respawned each frame `sync_cutscene_ui` runs (cheap —
/// the panel only exists while a cutscene plays).
#[derive(Component)]
pub struct CutsceneOverlayRoot;

/// Build / refresh the cutscene UI overlay. Pattern matches
/// `crate::dialog::sync_dialog_ui`: despawn last frame's overlay,
/// re-spawn this frame's based on `ActiveCutscene` + `CutsceneAdvanceRequest`.
///
/// Layout:
/// - Banner beats: centered card near the top, no input prompt
///   (auto-advances after the beat's timer).
/// - Dialogue beats: speaker + body card near the bottom, with a
///   "Press Interact / Jump to continue" hint (acknowledge mode).
/// - Skip-hold progress: thin bar near the bottom-right, only while
///   the player is holding Reset (Backspace / pad-Select).
pub fn sync_cutscene_ui(
    mut commands: Commands,
    active: Res<ActiveCutscene>,
    request: Res<CutsceneAdvanceRequest>,
    overlays: Query<Entity, With<CutsceneOverlayRoot>>,
    ui_fonts: Option<Res<crate::ui_fonts::UiFonts>>,
) {
    use bevy::ui::{
        AlignItems, BorderRadius, FlexDirection, JustifyContent, Node, PositionType, UiRect, Val,
        ZIndex,
    };

    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }
    if !active.is_playing() {
        return;
    }

    let cutscene_font = |font_size: f32, weight: crate::ui_fonts::UiFontWeight| {
        ui_fonts
            .as_deref()
            .map(|fonts| fonts.text_font(font_size, weight))
            .unwrap_or(TextFont {
                font_size,
                ..default()
            })
    };

    let banner = active.current_banner.as_ref();
    let dialogue = active.current_dialogue.as_ref();
    let skip_progress = request.skip_progress();

    // Bail out early on a fully-empty cutscene state (e.g. between
    // beats during a Fade or CameraPan). The overlay only spawns when
    // there's actually something to show — the cutscene runtime stays
    // active in `ActiveCutscene` either way.
    if banner.is_none() && dialogue.is_none() && skip_progress <= 0.01 {
        return;
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                padding: UiRect::all(Val::Px(24.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            ZIndex(50),
            Name::new("Cutscene Overlay Root"),
            CutsceneOverlayRoot,
        ))
        .with_children(|root| {
            // Top: banner card. Auto-dismisses based on the beat timer
            // (the runtime advances on its own — this UI is purely
            // presentational, the player doesn't have to press anything).
            if let Some((banner_text, _seconds)) = banner {
                root.spawn((
                    Node {
                        max_width: Val::Px(720.0),
                        padding: UiRect::axes(Val::Px(22.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.92)),
                    BorderColor::all(Color::srgba(0.84, 0.72, 0.40, 0.78)),
                    Name::new("Cutscene Banner"),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(banner_text.clone()),
                        cutscene_font(18.0, crate::ui_fonts::UiFontWeight::Semibold),
                        TextColor(Color::srgba(0.96, 0.90, 0.74, 1.0)),
                    ));
                });
            } else {
                // Spacer so the dialogue card always sits at the bottom
                // even when no banner is showing.
                root.spawn(Node::default());
            }

            // Bottom: dialogue card. Waits for player input
            // (Interact / Jump) — handled by `populate_control_frame_from_actions`
            // which flips `request.dismiss_dialogue` on the right press.
            if let Some((speaker, text)) = dialogue {
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        max_width: Val::Px(960.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(16.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.025, 0.030, 0.045, 0.95)),
                    BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.86)),
                    Name::new("Cutscene Dialogue Panel"),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(speaker.clone()),
                        cutscene_font(20.0, crate::ui_fonts::UiFontWeight::Semibold),
                        TextColor(Color::srgba(0.82, 0.94, 1.00, 1.0)),
                    ));
                    panel.spawn((
                        Text::new(text.clone()),
                        cutscene_font(16.0, crate::ui_fonts::UiFontWeight::Regular),
                        TextColor(Color::srgba(0.93, 0.96, 1.00, 1.0)),
                    ));
                    panel.spawn((
                        // The actual bindings live in
                        // `crate::input::presets::ControlPreset::input_map`
                        // (Interact = E by default, Jump = Space/W).
                        // The hint names the *semantic* actions so a
                        // rebound key isn't a lie.
                        Text::new("Press Interact (E) or Jump (Space) to continue. Hold Backspace to skip."),
                        cutscene_font(12.0, crate::ui_fonts::UiFontWeight::Regular),
                        TextColor(Color::srgba(0.66, 0.76, 0.88, 0.96)),
                    ));
                });
            } else {
                root.spawn(Node::default());
            }
        });

    // Skip-hold progress bar — bottom-right corner, separate root so
    // it doesn't interfere with the main column flex layout. Only
    // spawned while the player is actively holding the skip button.
    if skip_progress > 0.01 {
        let fill_pct = (skip_progress * 100.0).clamp(0.0, 100.0);
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(24.0),
                    bottom: Val::Px(24.0),
                    width: Val::Px(220.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                ZIndex(51),
                Name::new("Cutscene Skip Progress"),
                CutsceneOverlayRoot,
            ))
            .with_children(|root| {
                root.spawn((
                    Text::new(format!("hold to skip … {fill_pct:>3.0}%")),
                    cutscene_font(12.0, crate::ui_fonts::UiFontWeight::Regular),
                    TextColor(Color::srgba(0.86, 0.86, 0.92, 0.92)),
                ));
                root.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(6.0),
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.20, 0.22, 0.28, 0.85)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Percent(fill_pct),
                            height: Val::Percent(100.0),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.95, 0.78, 0.32, 0.96)),
                    ));
                });
            });
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

    #[test]
    fn default_cutscene_library_includes_test_intro() {
        let lib = default_cutscene_library();
        assert!(lib.get("test_intro").is_some());
    }

    #[test]
    fn default_cutscene_library_includes_boss_intro() {
        let lib = default_cutscene_library();
        assert!(lib.get("boss_intro_gradient_sentinel").is_some());
    }

    #[test]
    fn default_room_cutscene_bindings_link_hub_to_test_intro() {
        let bindings = RoomCutsceneBindings::defaults();
        // Hub plays the test_intro cutscene on first entry.
        assert!(bindings
            .bindings
            .iter()
            .any(|(room, cs)| room == "central_hub_main" && cs == "test_intro"));
    }

    #[test]
    fn cutscene_trigger_queue_request_appends() {
        let mut queue = CutsceneTriggerQueue::default();
        queue.request("a");
        queue.request("b");
        assert_eq!(queue.0, vec!["a".to_string(), "b".to_string()]);
    }
}
