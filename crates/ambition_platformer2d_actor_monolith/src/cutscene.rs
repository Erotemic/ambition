//! Cutscene playback runtime (the systems that drive the scripts).
//!
//! The cutscene SCRIPT format + stepper + live playback-state resources live in
//! the foundation crate [`ambition_cutscene`] (pure data + ECS resources, no
//! renderer). This module is the gameplay-side *player*: it reads triggers from
//! [`ambition_cutscene::CutsceneTriggerQueue`], starts/advances the active
//! cutscene, and applies its side effects (save-flag writes via
//! [`ambition_persistence::save::AmbitionGameSave`]). The HUD/overlay presentation
//! reads `ActiveCutscene` from the render crate.
//!
//! These systems are gameplay-coupled (rooms, save, schedule) so they live here
//! rather than in `ambition_cutscene` — which sits below this crate and must
//! stay content- and gameplay-free.

use bevy::prelude::*;

use ambition_cutscene::{
    ActiveCutscene, CutsceneAdvanceRequest, CutsceneEvent, CutsceneLibrary, CutsceneRuntime,
    RoomCutsceneBindings,
};

use ambition_cutscene::CutsceneTriggerQueue;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Bevy system: when the active room changes, queue up a cutscene if
/// the new room has a binding and the cutscene hasn't been seen.

pub fn auto_trigger_room_cutscenes(
    bindings: Res<RoomCutsceneBindings>,
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
    mut queue: ResMut<CutsceneTriggerQueue>,
    mut last_room: ResMut<ambition_cutscene::LastCutsceneRoom>,
) {
    let current = room_set.active_spec().id.clone();
    let changed = last_room.0.as_deref() != Some(current.as_str());
    if !changed {
        return;
    }
    last_room.0 = Some(current.clone());
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
        let runtime = CutsceneRuntime::new(script.clone());
        // The first beat's picture, before any tick — so a cutscene that starts
        // and is snapshotted in the same frame is not blank.
        active.presentation = runtime.presentation();
        active.runtime = Some(runtime);
        break;
    }
}

/// Advance the playing cutscene by one SIMULATION step.
///
/// this read `Res<Time>`, which is the wrong clock in two ways. This system runs in the sim
/// schedule, and `sim_schedule()` IS `Update` under the `RenderFrame` host — so a cutscene's
/// beat timings depended on how fast the machine drew frames, and two replays of the same input
/// stream could enter different beats.
///
/// and `WorldTime::sim_dt` is SCALED, which the frame clock is not. A
/// cutscene playing under slow motion now slows with the scene it accompanies
/// instead of running at wall speed over a world in treacle.
///
/// this is the "deterministic elapsed" half of the cutscene-authority row in
/// `tracks.md`: playback state advances on the sim clock, and only presentation
/// reads the wall clock (`PresentationTime::wall_dt`).

/// END a cutscene: mark it seen and tear it down. THE ONE ENDING.
///
/// ⛔⛔ THIS RULE WAS WRITTEN TWICE, once per way a cutscene can stop. Skipping
/// and completing each read `script.seen_flag`, set it, nulled the runtime and
/// reset the presentation -- the same four steps, side by side in one function.
/// ⇒ "What it MEANS for a cutscene to end" had two authorities, and a third
/// ending (a cancel, a room change, an abandoned session) would have had to
/// remember all four steps from scratch. Forgetting the flag is the expensive
/// half: the cutscene has no record of having played, so it plays again every
/// time its trigger is met.
///
/// ⚠ The flag is `Option` because not every script asks to be remembered; a
/// script with no `seen_flag` ends without recording anything, which is a
/// different thing from ending without being marked.
fn end_cutscene(
    active: &mut ActiveCutscene,
    save: &mut ambition_persistence::save::AmbitionGameSave,
) {
    if let Some(seen) = active
        .runtime
        .as_ref()
        .and_then(|runtime| runtime.script.seen_flag.clone())
    {
        save.data_mut().set_flag(seen, true);
    }
    active.runtime = None;
    active.presentation = Default::default();
}

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
        end_cutscene(&mut active, &mut save);
        return;
    }

    let events = runtime.tick(dt, dismiss);
    let mut completed = false;
    for event in events {
        match event {
            // It fires only on the tick a beat begins (`elapsed == 0.0`), so a rollback landing
            // MID-BEAT never saw it again and the banner, camera target and fade were gone for
            // the rest of that beat — while the snapshot's doc claimed the next tick would
            // republish them. It also mutated one field at a time, so a camera beat following a
            // dialogue left the dialogue on screen.
            CutsceneEvent::BeatEntered { .. } => {}
            CutsceneEvent::FlagWritten { id, on } => {
                save.data_mut().set_flag(id, on);
            }
            CutsceneEvent::Skipped | CutsceneEvent::Completed => {
                completed = true;
            }
        }
    }

    if completed {
        end_cutscene(&mut active, &mut save);
        return;
    }

    // THE WHOLE PICTURE, REPLACED, from the state the snapshot carries.
    // A pure function of `(script, beat_index, elapsed)` — so restoring mid-beat
    // restores the picture with it, and no beat can leave another's fields
    // standing because there are no individual fields to leave.
    active.presentation = active
        .runtime
        .as_ref()
        .map(|runtime| runtime.presentation())
        .unwrap_or_default();
}

/// Module-local Bevy plugin: schedules the cutscene chain
/// (`auto_trigger_room_cutscenes` → `drain_cutscene_triggers` →
/// `tick_active_cutscene`) into [`ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::Cutscene`].
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
        app.init_resource::<ambition_cutscene::LastCutsceneRoom>();
        app.init_resource::<ActiveCutscene>();
        app.init_resource::<CutsceneAdvanceRequest>();
        // The input-local half of the skip: an accumulator the HUD draws and the
        // sim never reads. See `CutsceneSkipHold`.
        app.init_resource::<ambition_cutscene::CutsceneSkipHold>();
        app.add_systems(
            sim,
            (
                auto_trigger_room_cutscenes,
                drain_cutscene_triggers,
                tick_active_cutscene,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::Cutscene),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔⛔ ENDING A CUTSCENE MUST RECORD THAT IT WAS SEEN, and until now nothing
    /// checked it: deleting the `set_flag` left every cutscene test green.
    ///
    /// The flag is what `should_play` reads to refuse a replay, so losing it does
    /// not break a frame — it makes the cutscene play AGAIN every time its
    /// trigger is met, which is a bug the player meets on the second visit and no
    /// test meets at all.
    ///
    /// ⭐ One test now covers BOTH endings because there is only one ending:
    /// skipping and completing were four identical steps written side by side and
    /// are now `end_cutscene`. That is the payoff of the merge — the guard did not
    /// have to be written twice either.
    #[test]
    fn ending_a_cutscene_records_that_it_was_seen() {
        let mut active = ActiveCutscene {
            runtime: Some(ambition_cutscene::CutsceneRuntime::new(
                ambition_cutscene::CutsceneScript {
                    id: "probe".to_string(),
                    beats: Vec::new(),
                    seen_flag: Some("probe_seen".to_string()),
                },
            )),
            presentation: Default::default(),
        };
        let mut save = ambition_persistence::save::AmbitionGameSave::default();

        end_cutscene(&mut active, &mut save);

        assert!(
            save.data().flag("probe_seen"),
            "the cutscene ended without recording its seen flag, so its trigger \
             will play it again on the next visit"
        );
        assert!(
            active.runtime.is_none(),
            "the runtime outlived the ending, so the cutscene is still active"
        );
    }

    /// ⚠ AND A SCRIPT THAT ASKS FOR NO FLAG ENDS WITHOUT WRITING ONE. Ending
    /// unrecorded is a different thing from ending unmarked, and a helper that
    /// invented an id would put a row in the save for every unnamed cutscene.
    #[test]
    fn a_cutscene_with_no_seen_flag_records_nothing() {
        let mut active = ActiveCutscene {
            runtime: Some(ambition_cutscene::CutsceneRuntime::new(
                ambition_cutscene::CutsceneScript {
                    id: "unnamed".to_string(),
                    beats: Vec::new(),
                    seen_flag: None,
                },
            )),
            presentation: Default::default(),
        };
        let mut save = ambition_persistence::save::AmbitionGameSave::default();

        end_cutscene(&mut active, &mut save);

        assert!(
            save.data().flags().is_empty(),
            "a script with no `seen_flag` wrote a durable row anyway"
        );
        assert!(active.runtime.is_none());
    }

    #[test]
    fn cutscene_trigger_queue_request_appends() {
        let mut queue = CutsceneTriggerQueue::default();
        queue.request("a");
        queue.request("b");
        assert_eq!(queue.0, vec!["a".to_string(), "b".to_string()]);
    }
}
