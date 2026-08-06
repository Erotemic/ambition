//! Cutscene playback runtime (the systems that drive the scripts).
//!
//! The cutscene SCRIPT format + stepper + live playback-state resources live in
//! the foundation crate [`ambition_cutscene`] (pure data + ECS resources, no
//! renderer). This module is the gameplay-side *player*: it reads triggers from
//! [`crate::cutscene_trigger::CutsceneTriggerQueue`], starts/advances the active
//! cutscene, and applies its side effects (save-flag writes via
//! [`ambition_persistence::save::AmbitionGameSave`]). The HUD/overlay presentation
//! reads `ActiveCutscene` from the render crate.
//!
//! These systems are gameplay-coupled (rooms, save, schedule) so they live here
//! rather than in `ambition_cutscene` — which sits below this crate and must
//! stay content- and gameplay-free.

use bevy::prelude::*;

use ambition_cutscene::{
    ActiveCutscene, CutsceneAdvanceRequest, CutsceneBeat, CutsceneEvent, CutsceneLibrary,
    CutsceneRuntime, RoomCutsceneBindings,
};

use crate::cutscene_trigger::CutsceneTriggerQueue;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Bevy system: when the active room changes, queue up a cutscene if
/// the new room has a binding and the cutscene hasn't been seen.
pub fn auto_trigger_room_cutscenes(
    bindings: Res<RoomCutsceneBindings>,
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        crate::rooms::RoomSet,
    >,
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

/// Drain the trigger queue: start the next cutscene if one isn't
/// already playing. Skips any that have already had their seen flag
/// set.
pub fn drain_cutscene_triggers(
    mut queue: ResMut<CutsceneTriggerQueue>,
    library: Res<CutsceneLibrary>,
    mut active: ResMut<ActiveCutscene>,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
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
        active.runtime = Some(CutsceneRuntime::new(script.clone()));
        active.current_dialogue = None;
        active.current_banner = None;
        active.camera_target = None;
        active.fade_alpha = 0.0;
        break;
    }
}

/// Advance the playing cutscene by one SIMULATION step.
///
/// ⛔ **this read `Res<Time>`, which is the wrong clock in two ways.** This
/// system runs in the sim schedule, and `sim_schedule()` IS `Update` under the
/// `RenderFrame` host — so a cutscene's beat timings depended on how fast the
/// machine drew frames, and two replays of the same input stream could enter
/// different beats. Under a fixed or GGRS host the frame clock happens to be
/// deterministic, which is exactly the kind of accident that makes a bug
/// invisible in the tests that would catch it.
///
/// ⚠ **and `WorldTime::sim_dt` is SCALED**, which the frame clock is not. A
/// cutscene playing under slow motion now slows with the scene it accompanies
/// instead of running at wall speed over a world in treacle.
///
/// ⭐ this is the "deterministic elapsed" half of the cutscene-authority row in
/// `tracks.md`: playback state advances on the sim clock, and only presentation
/// reads the wall clock (`PresentationTime::wall_dt`).
pub fn tick_active_cutscene(
    time: Res<ambition_time::WorldTime>,
    mut active: ResMut<ActiveCutscene>,
    mut request: ResMut<CutsceneAdvanceRequest>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
) {
    let dismiss = std::mem::take(&mut request.dismiss_dialogue);
    let skip = std::mem::take(&mut request.skip_cutscene);
    let dt = time.sim_dt();

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
            CutsceneEvent::BeatEntered { beat, .. } => match beat {
                CutsceneBeat::Dialogue { speaker, text } => {
                    active.current_dialogue = Some((speaker, text));
                    active.current_banner = None;
                }
                CutsceneBeat::Banner { text, seconds } => {
                    active.current_dialogue = None;
                    active.current_banner = Some((text, seconds));
                }
                CutsceneBeat::CameraPan { target, .. } => {
                    active.camera_target = Some(Vec2::new(target[0], target[1]));
                }
                CutsceneBeat::Fade { to_alpha, .. } => {
                    active.fade_alpha = to_alpha.clamp(0.0, 1.0);
                }
                _ => {}
            },
            CutsceneEvent::FlagWritten { id, on } => {
                save.data_mut().set_flag(id, on);
            }
            CutsceneEvent::Skipped | CutsceneEvent::Completed => {
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

/// Module-local Bevy plugin: schedules the cutscene chain
/// (`auto_trigger_room_cutscenes` → `drain_cutscene_triggers` →
/// `tick_active_cutscene`) into [`crate::schedule::Platformer2dSimulationPhaseMonolith::Cutscene`].
///
/// The presentation overlay (`ambition_render::cutscene::sync_cutscene_ui`) is
/// scheduled separately by the render/app side — this plugin owns only the
/// gameplay-side playback.
pub struct CutsceneSchedulePlugin;

impl Plugin for CutsceneSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // The cutscene state channels + empty library/bindings (anti-god
        // rule 5: the domain plugin owns its init). Content POPULATES the
        // library/bindings; a game that pre-inserts them wins (init never
        // clobbers).
        app.init_resource::<RoomCutsceneBindings>();
        app.init_resource::<CutsceneLibrary>();
        app.init_resource::<CutsceneTriggerQueue>();
        app.init_resource::<ActiveCutscene>();
        app.init_resource::<CutsceneAdvanceRequest>();
        app.add_systems(
            sim,
            (
                auto_trigger_room_cutscenes,
                drain_cutscene_triggers,
                tick_active_cutscene,
            )
                .chain()
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::Cutscene),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutscene_trigger_queue_request_appends() {
        let mut queue = CutsceneTriggerQueue::default();
        queue.request("a");
        queue.request("b");
        assert_eq!(queue.0, vec!["a".to_string(), "b".to_string()]);
    }
}
