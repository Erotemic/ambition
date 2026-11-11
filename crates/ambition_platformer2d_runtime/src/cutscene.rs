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
//! These systems are gameplay-coupled (rooms, save, schedule) so they live in
//! the runtime that installs them rather than in `ambition_cutscene`, which
//! sits below it and must stay content- and gameplay-free.

use bevy::prelude::*;

use ambition_cutscene::{ActiveCutscene, CutsceneEvent, CutsceneLibrary, CutsceneRuntime};

use ambition_cutscene::CutsceneTriggerQueue;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Bevy system: when a room id becomes live, queue up the cutscene the room
/// says it starts (`RoomMetadata::entry_cutscene`, the LDtk level field
/// `entry_cutscene`). `drain_cutscene_triggers` skips a cutscene that was seen.
///
/// Every live room (OW1 Cut C): the sole live room was read, so while two
/// rooms were live no room-entry cutscene was queued. A second live room of
/// an id already live queues nothing. With no room live the memory is kept.
pub fn auto_trigger_room_cutscenes(
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut queue: ResMut<CutsceneTriggerQueue>,
    mut last_rooms: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<ambition_cutscene::LastCutsceneRoom>,
) {
    // Each live room's id, its entry cutscene, and the room it plays for.
    let mut live: Vec<(String, Option<String>, u32)> = rooms
        .live_rooms()
        .map(|(instance, definition)| {
            let spec = rooms.rooms().spec(definition);
            (spec.id.clone(), spec.metadata.entry_cutscene.clone(), instance.ordinal())
        })
        .collect();
    if live.is_empty() {
        return;
    }
    live.sort();
    live.dedup_by(|a, b| a.0 == b.0);
    let ids: Vec<String> = live.iter().map(|(id, ..)| id.clone()).collect();
    // Read through the immutable deref: on every frame but a change there is
    // nothing to write, and a `DerefMut` would mark the resource changed.
    if last_rooms.0 == ids {
        return;
    }
    // In sorted room order, so a resimulation queues the same cutscenes in
    // the same order.
    for (_, cutscene, room) in live.iter().filter(|(id, ..)| !last_rooms.0.contains(id)) {
        if let Some(cutscene) = cutscene {
            queue.request_in(cutscene, *room);
        }
    }
    last_rooms.0 = ids;
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
    // One trigger starts; the rest wait their turn. Two rooms entered on one
    // tick queue two entry cutscenes, and each room is announced once
    // (`LastCutsceneRoom`), so a trigger dropped here never comes back.
    let mut pending = std::mem::take(&mut queue.0).into_iter();
    while let Some(trigger) = pending.next() {
        let Some(script) = library.get(&trigger.id) else {
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
        active.room = trigger.room;
        break;
    }
    queue.0.extend(pending);
}

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
///
/// Returns the seen flag to set, which the caller writes with the cutscene's
/// other flags ([`write_cutscene_flags`]).
#[must_use]
fn end_cutscene(active: &mut ActiveCutscene) -> Option<String> {
    let seen = active
        .runtime
        .as_ref()
        .and_then(|runtime| runtime.script.seen_flag.clone());
    active.runtime = None;
    active.presentation = Default::default();
    active.room = None;
    seen
}

/// Write the flags a cutscene set this tick, dated in the room it plays for
/// (ISL-2d): through `SetFlagRequested`, so a rewind of that room takes them
/// back and the quests they moved, as it does a switch's. A composition with
/// no flag channel writes the save directly.
fn write_cutscene_flags(
    flags: Vec<(String, bool)>,
    written: Option<ambition_platformer2d_shared_tangle::lifecycle::RoomWrite>,
    requests: Option<&mut Messages<ambition_combat::events::SetFlagRequested>>,
    save: &mut ambition_persistence::save::AmbitionGameSave,
) {
    match requests {
        Some(requests) => {
            for (id, on) in flags {
                requests.write(ambition_combat::events::SetFlagRequested { id, on, written });
            }
        }
        None => {
            for (id, on) in flags {
                save.data_mut().set_flag(id, on);
            }
        }
    }
}

/// Run condition of [`tick_active_cutscene`]: a cutscene plays, or a skip
/// hold from the last one waits to be cleared. Otherwise the tick would only
/// write both resources back unchanged.
pub fn a_cutscene_plays_or_its_hold_remains(
    active: bevy::prelude::Res<ActiveCutscene>,
    skip_hold: bevy::prelude::Res<ambition_cutscene::CutsceneSkipHold>,
) -> bool {
    active.runtime.is_some() || skip_hold.seconds != 0.0
}

/// The seat that answers the playing cutscene: for a room's cutscene, the
/// lowest seat driving a body in that room (so Player 1 in another room does
/// not skip it, ISL-1b); otherwise, and when no seat is in the room, the
/// primary seat.
fn cutscene_seat(
    active: &ActiveCutscene,
    drivers: &Query<(&ambition_characters::control::DrivingParticipant, Entity)>,
    rooms: &ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) -> ambition_characters::control::PlayerSlot {
    let Some(room) = active.room else {
        return ambition_characters::control::PlayerSlot::PRIMARY;
    };
    drivers
        .iter()
        .filter(|(_, body)| rooms.of(*body).map(|at| at.ordinal()) == Some(room))
        .map(|(driver, _)| driver.0)
        .min()
        .unwrap_or(ambition_characters::control::PlayerSlot::PRIMARY)
}

/// Advance the playing cutscene by one SIMULATION step.
///
/// ⛔⛤ **THE CLOCK IS `WorldTime`, NOT `Res<Time>`, FOR TWO SEPARATE REASONS.**
/// This system is registered into
/// `Platformer2dSimulationPhase::Cutscene` (see `:181`), and
/// `sim_schedule()` IS `Update` under the `RenderFrame` host — so on the frame
/// clock a cutscene's beat timings would depend on how fast the machine draws,
/// and two replays of the same input stream could enter different beats.
///
/// ⭐ And `WorldTime::sim_dt` is SCALED (`ambition_time/src/lib.rs:209` returns
/// `sim_dt`), which a frame clock is not: a cutscene playing under slow
/// motion slows with the scene it accompanies instead of running at wall speed
/// over a world in treacle.
///
/// ⇒ Playback state advances on the sim clock and only PRESENTATION reads the
/// wall clock (`PresentationTime::wall_dt`) — the deterministic-elapsed half of
/// `queue.md`'s `CUTSCENE-ROLLBACK-DECISION`. ⚠ That route said "the
/// cutscene-authority row in `tracks.md`" until 2026-09-18; `tracks.md` does
/// not mention cutscenes at all, and this doc block was itself attached to
/// `end_cutscene` rather than to the function it describes.
///
/// ⭐ THE PLAYER'S DISMISS AND SKIP ARE READ FROM THE PRIMARY SEAT'S
/// `ControlFrame` (`confirm_pressed`, `cancel_held`), the input this tick was
/// given. A replay and a peer receive the same frame, so a rewind across the
/// press re-delivers it. The primary seat owns the shell controls, as it owns
/// `MenuControlFrame`.
pub fn tick_active_cutscene(
    time: Res<ambition_time::WorldTime>,
    controls: Res<ambition_characters::control::SlotControls>,
    mut active: ResMut<ActiveCutscene>,
    mut skip_hold: ResMut<ambition_cutscene::CutsceneSkipHold>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    // The seats and the rooms of the bodies they drive: a room's cutscene
    // is skipped and dismissed by a seat in its room. And the flag channel
    // and the tick its flags are dated on.
    (drivers, rooms, mut requests, tick): (
        Query<(&ambition_characters::control::DrivingParticipant, Entity)>,
        ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
        Option<ResMut<Messages<ambition_combat::events::SetFlagRequested>>>,
        Option<Res<ambition_time::SimTick>>,
    ),
) {
    let written = active.room.zip(tick.as_deref()).map(|(room, tick)| {
        ambition_platformer2d_shared_tangle::lifecycle::RoomWrite {
            room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::from_ordinal(room),
            tick: tick.0,
        }
    });
    let slot = cutscene_seat(&active, &drivers, &rooms);
    let seat = controls.get(slot);
    let dt = time.sim_dt();

    let Some(runtime) = active.runtime.as_mut() else {
        // A partial hold belongs to the cutscene that accumulated it.
        skip_hold.seconds = 0.0;
        return;
    };

    let skip = if seat.cancel_held {
        skip_hold.seconds += dt;
        skip_hold.seconds >= ambition_cutscene::SKIP_HOLD_THRESHOLD_SECS
    } else {
        skip_hold.seconds = 0.0;
        false
    };
    // An EDGE: a held confirm does not burn through several beats.
    let dismiss = seat.confirm_pressed;

    if skip {
        skip_hold.seconds = 0.0;
        let _ = runtime.skip();
        let flags = end_cutscene(&mut active).map(|seen| (seen, true)).into_iter().collect();
        write_cutscene_flags(flags, written, requests.as_deref_mut(), &mut save);
        return;
    }

    let events = runtime.tick(dt, dismiss);
    let mut completed = false;
    let mut flags = Vec::new();
    for event in events {
        match event {
            // It fires only on the tick a beat begins (`elapsed == 0.0`), so a rollback landing
            // MID-BEAT never saw it again and the banner, camera target and fade were gone for
            // the rest of that beat — while the snapshot's doc claimed the next tick would
            // republish them. It also mutated one field at a time, so a camera beat following a
            // dialogue left the dialogue on screen.
            CutsceneEvent::BeatEntered { .. } => {}
            CutsceneEvent::FlagWritten { id, on } => flags.push((id, on)),
            CutsceneEvent::Skipped | CutsceneEvent::Completed => {
                completed = true;
            }
        }
    }

    if completed {
        flags.extend(end_cutscene(&mut active).map(|seen| (seen, true)));
    }
    write_cutscene_flags(flags, written, requests.as_deref_mut(), &mut save);
    if completed {
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

/// Log this tick's change to the playing cutscene in the room it plays for.
pub fn log_cutscene_changes_in_rooms(
    active: Res<ActiveCutscene>,
    skip_hold: Res<ambition_cutscene::CutsceneSkipHold>,
    mut log: ResMut<ambition_cutscene::CutscenesInRooms>,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    log.observe(
        ambition_cutscene::CutsceneState::of(&active, &skip_hold),
        tick.map_or(0, |tick| tick.0),
        ambition_platformer2d_shared_tangle::lifecycle::ROOM_REWIND_HORIZON_TICKS,
    );
}

/// The room retractor: put back the cutscene `room` had at `saved_at`.
fn retract_room_cutscenes(
    world: &mut World,
    room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    saved_at: u64,
) {
    let Some(put_back) = world
        .get_resource_mut::<ambition_cutscene::CutscenesInRooms>()
        .and_then(|mut log| log.retract_room(room.ordinal(), saved_at))
    else {
        return;
    };
    world.resource_scope(|world, mut active: Mut<ActiveCutscene>| {
        if let Some(mut skip_hold) = world.get_resource_mut::<ambition_cutscene::CutsceneSkipHold>() {
            put_back.restore(&mut active, &mut skip_hold);
        }
    });
}

/// Module-local Bevy plugin: schedules the cutscene chain
/// (`auto_trigger_room_cutscenes` → `drain_cutscene_triggers` →
/// `tick_active_cutscene`) into [`ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::Cutscene`].
///
/// The presentation overlay (`ambition_render::cutscene::sync_cutscene_ui`) is
/// scheduled separately by the render/app side — this plugin owns only the
/// gameplay-side playback.
pub struct CutsceneSchedulePlugin;

impl Plugin for CutsceneSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // The cutscene state channels + an empty library (anti-god rule 5:
        // the domain plugin owns its init). Content POPULATES the library; a
        // game that pre-inserts it wins (init never clobbers). Which room
        // starts which cutscene is the room's own metadata.
        app.init_resource::<CutsceneLibrary>();
        app.init_resource::<CutsceneTriggerQueue>();
        // The room this session last announced: a component of the session root.
        ambition_platformer2d_shared_tangle::lifecycle::require_on_session_root::<ambition_cutscene::LastCutsceneRoom>(app);
        app.init_resource::<ActiveCutscene>();
        // The skip hold, accumulated by `tick_active_cutscene` from the seat's
        // `cancel_held`. See `CutsceneSkipHold`.
        app.init_resource::<ambition_cutscene::CutsceneSkipHold>();
        // The cutscene's changes, dated in their room, so a room that rewinds
        // puts back the cutscene it had (ISL-1b). Logged at the tail of the
        // tick, after the playback and anything else that writes it.
        app.init_resource::<ambition_cutscene::CutscenesInRooms>();
        ambition_platformer2d_shared_tangle::lifecycle::register_room_write_retractor(
            app,
            "cutscene.changes_in_rooms",
            retract_room_cutscenes,
        );
        app.add_systems(
            sim,
            log_cutscene_changes_in_rooms
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::FeatureViewSync),
        );
        app.add_systems(
            sim,
            (
                auto_trigger_room_cutscenes,
                // Only with a trigger queued: it took the queue each tick.
                drain_cutscene_triggers.run_if(
                    |queue: bevy::prelude::Res<ambition_cutscene::CutsceneTriggerQueue>| !queue.0.is_empty(),
                ),
                tick_active_cutscene.run_if(a_cutscene_plays_or_its_hold_remains),
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::Cutscene),
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
            room: None,
            runtime: Some(ambition_cutscene::CutsceneRuntime::new(
                ambition_cutscene::CutsceneScript {
                    id: "probe".to_string(),
                    beats: Vec::new(),
                    seen_flag: Some("probe_seen".to_string()),
                },
            )),
            presentation: Default::default(),
        };
        let seen = end_cutscene(&mut active);

        assert_eq!(
            seen.as_deref(),
            Some("probe_seen"),
            "the cutscene ended without recording its seen flag, so its trigger \
             will play it again on the next visit"
        );
        assert!(
            active.runtime.is_none(),
            "the runtime outlived the ending, so the cutscene is still active"
        );
    }

    /// The flags a cutscene sets, and its seen flag, under the room it plays
    /// for and the tick (ISL-2d), so a rewind of that room takes them back.
    fn flags_written_by_a_cutscene_in(room: Option<u32>) -> Vec<ambition_combat::events::SetFlagRequested> {
        let mut app = bevy::prelude::App::new();
        app.insert_resource(ambition_time::WorldTime::new(1.0, 1.0));
        app.insert_resource(ambition_time::SimTick(5));
        app.init_resource::<ambition_characters::control::SlotControls>();
        app.init_resource::<ambition_cutscene::CutsceneSkipHold>();
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        app.add_message::<ambition_combat::events::SetFlagRequested>();
        app.insert_resource(ActiveCutscene {
            room,
            runtime: Some(ambition_cutscene::CutsceneRuntime::new(
                ambition_cutscene::CutsceneScript::new(
                    "probe",
                    vec![ambition_cutscene::CutsceneBeat::SetFlag { id: "probe_flag".into(), on: true }],
                )
                .with_seen_flag("probe_seen"),
            )),
            presentation: Default::default(),
        });
        app.add_systems(bevy::prelude::Update, tick_active_cutscene);
        app.update();
        let messages = app.world().resource::<Messages<ambition_combat::events::SetFlagRequested>>();
        messages.get_cursor().read(messages).cloned().collect()
    }

    #[test]
    fn a_room_cutscene_dates_its_flags_in_its_room() {
        let room = ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::from_ordinal(3);
        let dated = Some(ambition_platformer2d_shared_tangle::lifecycle::RoomWrite { room, tick: 5 });
        let written: Vec<_> = flags_written_by_a_cutscene_in(Some(3)).into_iter().map(|flag| (flag.id, flag.on, flag.written)).collect();
        assert_eq!(
            written,
            [("probe_flag".to_string(), true, dated), ("probe_seen".to_string(), true, dated)],
            "a room's cutscene wrote its flags undated, or not at all"
        );
        assert!(
            flags_written_by_a_cutscene_in(None).iter().all(|flag| flag.written.is_none()),
            "a cutscene of the whole world dated its flags in a room"
        );
    }

    /// Two rooms entered on one tick queue two entry cutscenes: the first
    /// plays, and the second waits for it and then plays (review 2026-11-11,
    /// P1: the drain took the whole queue and dropped all but the first).
    #[test]
    fn a_second_rooms_cutscene_waits_its_turn() {
        let mut app = bevy::prelude::App::new();
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        let mut library = CutsceneLibrary::default();
        for id in ["alices_room", "bobs_room"] {
            library.insert(ambition_cutscene::CutsceneScript { id: id.into(), beats: Vec::new(), seen_flag: None });
        }
        app.insert_resource(library);
        app.init_resource::<ActiveCutscene>();
        let mut queue = CutsceneTriggerQueue::default();
        queue.request_in("alices_room", 1);
        queue.request_in("bobs_room", 2);
        app.insert_resource(queue);
        app.add_systems(bevy::prelude::Update, drain_cutscene_triggers);
        let playing = |app: &bevy::prelude::App| {
            let active = app.world().resource::<ActiveCutscene>();
            active.runtime.as_ref().map(|runtime| (runtime.script.id.clone(), active.room))
        };
        app.update();
        assert_eq!(playing(&app), Some(("alices_room".to_string(), Some(1))));
        let _ = end_cutscene(&mut app.world_mut().resource_mut::<ActiveCutscene>());
        app.update();
        assert_eq!(playing(&app), Some(("bobs_room".to_string(), Some(2))), "the second room's cutscene was dropped");
    }

    /// ⚠ AND A SCRIPT THAT ASKS FOR NO FLAG ENDS WITHOUT WRITING ONE. Ending
    /// unrecorded is a different thing from ending unmarked, and a helper that
    /// invented an id would put a row in the save for every unnamed cutscene.
    #[test]
    fn a_cutscene_with_no_seen_flag_records_nothing() {
        let mut active = ActiveCutscene {
            room: None,
            runtime: Some(ambition_cutscene::CutsceneRuntime::new(
                ambition_cutscene::CutsceneScript {
                    id: "unnamed".to_string(),
                    beats: Vec::new(),
                    seen_flag: None,
                },
            )),
            presentation: Default::default(),
        };
        let seen = end_cutscene(&mut active);

        assert_eq!(seen, None, "a script with no `seen_flag` wrote a durable row anyway");
        assert!(active.runtime.is_none());
    }

    /// The tick runs while a cutscene plays ([`a_cutscene_plays_or_its_hold_remains`]);
    /// once it ends, neither the cutscene nor the skip hold is written again.
    #[test]
    fn an_ended_cutscene_leaves_its_resources_unwritten() {
        use bevy::prelude::IntoScheduleConfigs as _;
        let mut app = bevy::prelude::App::new();
        app.insert_resource(ambition_time::WorldTime::new(1.0, 1.0));
        app.init_resource::<ambition_characters::control::SlotControls>();
        app.init_resource::<ambition_cutscene::CutsceneSkipHold>();
        app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
        app.insert_resource(ActiveCutscene {
            room: None,
            runtime: Some(ambition_cutscene::CutsceneRuntime::new(ambition_cutscene::CutsceneScript {
                id: "probe".to_string(),
                beats: Vec::new(),
                seen_flag: None,
            })),
            presentation: Default::default(),
        });
        app.add_systems(bevy::prelude::Update, tick_active_cutscene.run_if(a_cutscene_plays_or_its_hold_remains));
        for _ in 0..8 {
            app.update();
            if app.world().resource::<ActiveCutscene>().runtime.is_none() {
                break;
            }
        }
        assert!(app.world().resource::<ActiveCutscene>().runtime.is_none(), "a script with no beats never ended");
        app.update();
        let ended = app.world().resource_ref::<ActiveCutscene>().last_changed();
        let held = app.world().resource_ref::<ambition_cutscene::CutsceneSkipHold>().last_changed();
        app.update();
        assert_eq!(app.world().resource_ref::<ActiveCutscene>().last_changed(), ended);
        assert_eq!(app.world().resource_ref::<ambition_cutscene::CutsceneSkipHold>().last_changed(), held);
    }

    #[test]
    fn cutscene_trigger_queue_request_appends() {
        let mut queue = CutsceneTriggerQueue::default();
        queue.request("a");
        queue.request("b");
        queue.request_in("c", 7);
        let queued: Vec<_> = queue.0.iter().map(|trigger| (trigger.id.as_str(), trigger.room)).collect();
        assert_eq!(queued, [("a", None), ("b", None), ("c", Some(7))]);
    }

    /// ⭐ **THE CUTSCENE ROOM MEMORY IS THE SESSION'S, AND SO IS THE QUEST ONE
    /// (C03, 2026-10-07).** A new game that starts in the room the last session
    /// ended in must queue that room's entry cutscene again; the memory was a
    /// process-global resource that a reset cleared, and is a component of the
    /// session root now. A candidate beside a live session has none, and the live
    /// session's is not changed by it. Poisoned by giving the new root the old
    /// one's memory at the swap: the second request is skipped.
    #[test]
    fn a_new_session_in_the_same_room_queues_its_entry_cutscene_again() {
        use ambition_cutscene::{CutsceneTriggerQueue, LastCutsceneRoom};
        use ambition_platformer2d_shared_tangle::lifecycle::{
            require_on_session_root, session_world_entity, CandidateSessionRoot, SessionRoot,
            SessionScopeId,
        };
        use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};

        let rooms = || {
            let mut spec = RoomSpec::new(
                "hall",
                ambition_platformer2d_core::World::new(
                    "hall",
                    ambition_platformer2d_core::Vec2::new(640.0, 480.0),
                    ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                    Vec::new(),
                ),
            );
            spec.metadata.entry_cutscene = Some("hall_intro".to_owned());
            RoomSet::from_parts_or_panic("hall", vec![spec], Vec::new())
        };
        let mut app = App::new();
        app.init_resource::<CutsceneTriggerQueue>()
            .add_systems(Update, auto_trigger_room_cutscenes);
        require_on_session_root::<LastCutsceneRoom>(&mut app);
        app.world_mut().spawn((SessionRoot(SessionScopeId(1)), rooms()));
        ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), "hall")
            .expect("the fixture set holds its room");
        let queued = |app: &App| app.world().resource::<CutsceneTriggerQueue>().0.len();
        app.update();
        assert_eq!(queued(&app), 1, "the first frame queues the entry cutscene");
        // Played: the drain takes it (a trigger still waiting is not queued twice).
        app.world_mut().resource_mut::<CutsceneTriggerQueue>().0.clear();
        let a = session_world_entity(app.world()).expect("session A is live");

        let b = app
            .world_mut()
            .spawn((CandidateSessionRoot(SessionScopeId(2)), rooms()))
            .id();
        assert!(app.world().get::<LastCutsceneRoom>(b).is_none());
        app.update();
        assert_eq!(queued(&app), 0, "a candidate queued a cutscene");
        assert_eq!(
            app.world().get::<LastCutsceneRoom>(a).unwrap().0,
            vec!["hall".to_owned()],
            "preparing a candidate changed the live session's memory"
        );

        app.world_mut().despawn(a);
        app.world_mut().entity_mut(b).remove::<CandidateSessionRoot>();
        app.world_mut().entity_mut(b).insert(SessionRoot(SessionScopeId(2)));
        ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), "hall")
            .expect("B holds its room");
        assert_eq!(
            app.world().get::<LastCutsceneRoom>(b).unwrap().0,
            Vec::<String>::new(),
            "B was born with A's memory"
        );
        app.update();
        assert_eq!(
            queued(&app),
            1,
            "a new game that starts in the room the last session ended in skipped its \
             entry cutscene"
        );
    }
}
