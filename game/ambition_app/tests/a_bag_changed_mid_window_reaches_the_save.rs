//! Durable save state written inside a rollback window.
//!
//! Every arm runs the real composition under a GGRS sync test (or on a timeline
//! this host owns) and asks whether a durable fact survives the rewind exactly
//! once: a conversation's visit, a New Game, a cutscene dismiss and skip, an item
//! used from the menu, a ledger write, a startup load, and a host intent's tick.
//!
//! The arms marked `playtest` are investigations of how a message produced
//! outside, or after, its consumer behaves under a rewind. They are kept for
//! the next such question and are not gates.

#![cfg(feature = "rl_sim")]

use ambition_platformer2d::input::InputOwner;
use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};

type SaveRestored = ambition_platformer2d::platformer::lifecycle::SaveRestored;
type AmbitionGameSave = ambition_platformer2d::persistence::save::AmbitionGameSave;
type OwnedItems = ambition_platformer2d::item::OwnedItems;
type Item = ambition_platformer2d::item::Item;

/// The calibration lab, the room the other rollback arms use.
const ROOM: &str = "combat_calibration_lab";

fn health(sim: &Platformer2dSimHarness) -> Result<(), String> {
    ambition_platformer2d::rollback::session_health(sim.world())
}

/// The schedule's own step count. This file never writes it, which is the
/// point: it is the control column for a frozen bag.
fn sim_tick(sim: &Platformer2dSimHarness) -> u64 {
    sim.world()
        .get_resource::<ambition_platformer2d::time::SimTick>()
        .map(|tick| tick.0)
        .unwrap_or(u64::MAX)
}

/// The room-scoped roster. A full sandbox reset despawns this whole set and
/// respawns it, and despawn bumps the generation, so a reconstruction that
/// really ran leaves NO original `Entity` value behind. Borrowed from
/// `rollback_full_reset.rs`, which uses it for the same discrimination.
fn feature_roster(sim: &mut Platformer2dSimHarness) -> std::collections::HashSet<bevy::prelude::Entity> {
    use bevy::prelude::With;
    let world = sim.world_mut();
    let mut q = world.query_filtered::<bevy::prelude::Entity, With<
        ambition_platformer2d::platformer::lifecycle::FeatureSimEntity,
    >>();
    q.iter(world).collect()
}

/// The dialogue-visit id the two arms below share. Namespaced so a content
/// author's node can never collide with it.
const VISIT_NODE: &str = "probe:visit_counted_from_update";

fn visit_count(sim: &Platformer2dSimHarness) -> u32 {
    sim.world()
        .resource::<AmbitionGameSave>()
        .data()
        .dialog_visit_count(VISIT_NODE)
}

/// Whether the GGRS session was live on the first frame the durable-restore
/// latch was up. Installed as an exclusive `Update` system after
/// `complete_durable_restore`: an unrelated `Update` member like this one is
/// what used to move the session start onto a tick simulated with the latch
/// down.
#[derive(bevy::prelude::Resource, Default)]
struct WithinFrameOrder {
    ggrs_live_just_after_the_latch: Option<bool>,
}

fn sample_ggrs_after_the_latch(world: &mut bevy::prelude::World) {
    let latched = world
        .get_resource::<SaveRestored>()
        .is_some_and(|latch| latch.0);
    if !latched {
        return;
    }
    let live = world.contains_resource::<ambition_platformer2d::rollback::AmbitionGgrsSession>();
    let mut order = world.resource_mut::<WithinFrameOrder>();
    if order.ggrs_live_just_after_the_latch.is_none() {
        order.ggrs_live_just_after_the_latch = Some(live);
    }
}

/// Every `(SimTick, SaveRestored)` pair this world simulated, one row per PASS.
/// Read from inside the simulation schedule, because a read between
/// `sim.step()` calls cannot see a latch that is down only while the schedule
/// runs.
#[derive(bevy::prelude::Resource, Default)]
struct LatchInsideTheSchedule(Vec<(u64, bool)>);

fn record_the_latch_from_inside_the_schedule(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    restored: Option<bevy::prelude::Res<SaveRestored>>,
    mut log: bevy::prelude::ResMut<LatchInsideTheSchedule>,
) {
    log.0
        .push((tick.0, restored.is_some_and(|restored| restored.0)));
}

/// The ticks the stand-in opens a conversation on, and the tick it closes the
/// first one. The first is the first tick of the session, the earliest moment a
/// scripted or room-entry conversation can open. Two openings, because an arm
/// that expects 1 cannot tell "counted on the opening edge" from "counted once
/// ever".
const FIRST_TICK: u64 = 1;
const OPENS_AT: [u64; 2] = [FIRST_TICK, 40];
const CLOSES_AT: u64 = 20;

/// Open and close a conversation on known ticks, from inside the rewinding
/// schedule, standing in for `interact_ecs_actors_and_switches`, which needs a
/// body in reach of an NPC with a compiled Yarn node.
///
/// It mints the instance at the CURRENT tick, as production does, rather than
/// through `LiveConversation::for_test`, which opens at tick zero: the opening
/// tick is the subject.
fn open_a_conversation_on_known_ticks(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut conversation: bevy::prelude::ResMut<
        ambition_platformer2d::conversation::ActiveConversation,
    >,
) {
    use ambition_platformer2d::conversation::{ConversationInstanceId, LiveConversation};
    if tick.0 == CLOSES_AT {
        conversation.close();
        return;
    }
    if !OPENS_AT.contains(&tick.0) {
        return;
    }
    conversation.open(LiveConversation {
        instance: ConversationInstanceId::mint(
            tick.0,
            VISIT_NODE,
            None,
            None,
            &ambition_platformer2d::dialog::DialogueContext::scripted(),
        ),
        initiator: None,
        talker: None,
        input_owner: InputOwner::Primary,
        speaker_name: String::new(),
    });
}

/// The ticks this world simulated before the durable-restore latch rose.
///
/// Ticks, not host frames: which host frame the boot lands on moves under
/// unrelated composition changes, and `FIRST_TICK` is the first tick whatever
/// frame it runs on.
fn ticks_simulated_unrestored(sim: &Platformer2dSimHarness) -> Vec<u64> {
    sim.world()
        .resource::<LatchInsideTheSchedule>()
        .0
        .iter()
        .filter(|(_, restored)| !restored)
        .map(|(tick, _)| *tick)
        .collect()
}

/// A conversation's visit is counted once per opening across a rewound window,
/// including an opening on the first tick of the session.
///
/// `count_the_dialogue_visit_when_a_conversation_opens` fires on
/// `ActiveConversation`'s `opened_at == SimTick`, a pure function of rollback
/// state. 0 visits means the counter never fired or its write was taken back;
/// more than one per opening means the edge has become a level (`opened_at <=
/// now`, or change detection, which a restore fires every frame under GGRS).
///
/// The first-tick opening is the session-start gate's witness:
/// `maintain_local_session` refuses to start a GGRS session over a world whose
/// durable state is still loading. Without the gate the first tick is
/// simulated with the latch down and its visit is dropped. The within-frame
/// sampler is installed because an unrelated `Update` system is what used to
/// move the session start onto that tick.
#[test]
fn a_conversation_opening_counts_exactly_one_visit_across_a_rewound_window() {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            use bevy::prelude::IntoScheduleConfigs as _;
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.init_resource::<LatchInsideTheSchedule>();
            // `Actuate` is where the real opener sits, and the `.before` edge is
            // the order the counter's `.after(interact_ecs_actors_and_switches)`
            // gives the real one: the set alone leaves the stand-in unordered
            // against the counter, and opened after it the edge is never seen.
            app.add_systems(
                label,
                (
                    record_the_latch_from_inside_the_schedule,
                    open_a_conversation_on_known_ticks.before(
                        ambition_platformer2d::session::durable_horizon::count_the_dialogue_visit_when_a_conversation_opens,
                    ),
                )
                    .in_set(
                        ambition_platformer2d::platformer::schedule::FeatureInteractionSet::Actuate,
                    ),
            );
            app.init_resource::<WithinFrameOrder>();
            app.add_systems(
                bevy::prelude::Update,
                sample_ggrs_after_the_latch
                    .after(ambition_platformer2d::session::durable_horizon::complete_durable_restore),
            );
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a conversation opener");
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }

    let compared = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>()
        .live_comparisons;
    assert!(
        compared > 0,
        "no frame was compared across a restore, so this arm ran without the \
         rewind it claims to survive"
    );
    assert!(
        sim.world()
            .resource::<WithinFrameOrder>()
            .ggrs_live_just_after_the_latch
            .is_some(),
        "the within-frame sampler never ran after the latch rose, so the \
         composition this arm guards is not the one it built"
    );
    assert!(
        sim_tick(&sim) > *OPENS_AT.last().expect("two openings"),
        "the window ended before the second opening, so a count of 1 would mean \
         'not yet' rather than 'counted once'"
    );
    assert!(
        sim.world()
            .resource::<LatchInsideTheSchedule>()
            .0
            .iter()
            .any(|(tick, _)| *tick == FIRST_TICK),
        "the world never simulated its first tick; if the durable hydration gate \
         now refuses to start a session at all, this is where it shows"
    );
    assert_eq!(
        ticks_simulated_unrestored(&sim),
        Vec::<u64>::new(),
        "ticks were simulated before the durable restore completed: the timeline \
         is starting over an unhydrated world, which the session-start gate in \
         `maintain_local_session` exists to refuse"
    );
    assert_eq!(
        visit_count(&sim),
        OPENS_AT.len() as u32,
        "two conversation openings (one on the first tick) did not produce two \
         visits. Fewer: the counter never fired, its write was taken back, or the \
         first-tick visit hit a counter still gated on the latch. More: the \
         opening edge has become a level"
    );
}

const LEDGER_ROW_AT: u64 = 5;
const LEDGER_OCCURRENCE: &str = "probe:an_object_the_world_remembers";

/// Write ONE row into the occurrence ledger, once, from inside the rewinding
/// schedule — the minimal shape of what New Game already does.
///
/// ⚠ ONCE, NOT EVERY TICK. A system that rewrites the row each tick IS the live
/// producer `AuthoredOccurrences`'s derived declaration describes, and it would
/// re-establish the row on every replay and hide the defect.
fn leave_one_row_in_the_occurrence_ledger(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut occurrences: bevy::prelude::ResMut<
        ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences,
    >,
) {
    use ambition_platformer2d::platformer::lifecycle::OccurrenceWhereabouts;
    if tick.0 != LEDGER_ROW_AT {
        return;
    }
    let mut rows = std::collections::BTreeMap::new();
    rows.insert(
        ambition_platformer2d::platformer::sim_id::SimId::from_snapshot(
            LEDGER_OCCURRENCE.to_string(),
        ),
        OccurrenceWhereabouts::Placed {
            room: ROOM.to_string(),
            at: bevy::prelude::Vec2::new(32.0, 48.0),
        },
    );
    occurrences.adopt_rows(rows);
}

/// One write to the occurrence ledger from inside the simulation keeps the sync
/// test healthy. `AuthoredOccurrences` is authoritative mutable simulation
/// state, so it has to be snapshotted and restored: carried in no snapshot, a
/// row written once survives a rewind into frames from before the write,
/// `persist_occurrence_horizon_to_save` mirrors it into the hashed save, and two
/// passes of one frame disagree within two ticks. A row naming an unloaded room
/// has no live producer that could republish it.
#[test]
fn one_write_to_the_occurrence_ledger_does_not_desync_the_sync_test() {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.add_systems(label, leave_one_row_in_the_occurrence_ledger);
            Ok(())
        },
    )
    .expect("the sync-test harness builds with one ledger write");
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }

    let rows = sim
        .world()
        .resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
        .rows()
        .count();
    let reached = sim_tick(&sim);
    assert_eq!(
        rows, 1,
        "the seeded row is not in the ledger, so this arm is measuring a world \
         where nothing wrote to it"
    );
    health(&sim).unwrap_or_else(|error| {
        panic!(
            "one `adopt_rows` call inside the rewinding schedule desynced the sync \
             test: the ledger is not restored by the rewind. SimTick reached \
             {reached}\n{error}"
        )
    });
    assert!(
        reached > 25,
        "the timeline stalled at tick {reached}, so the write is holding the \
         session back even though the checksum agrees"
    );
}


/// Every value `SaveRestored` held, sampled once per `Update`. A read between
/// `sim.step()` calls cannot see a latch lowered and raised again inside one
/// frame, which is the shape a New Game that reloaded the save would have.
#[derive(bevy::prelude::Resource, Default)]
struct LatchHistory(Vec<bool>);

fn record_the_latch_every_frame(
    restored: bevy::prelude::Res<SaveRestored>,
    mut history: bevy::prelude::ResMut<LatchHistory>,
) {
    history.0.push(restored.0);
}

/// Every commit of a New Game this world performed, by the `SimTick` it
/// committed on. A commit runs once, on a confirmed frame, and a replay does
/// not run it again.
#[derive(bevy::prelude::Resource, Default)]
struct ResetCommits(Vec<u64>);

/// Runs in the commit's domain-apply schedule, where a New Game is the one
/// restore that installs the fresh-run marker.
fn record_every_new_game_commit(
    fresh: Option<bevy::prelude::Res<ambition_platformer2d::platformer::lifecycle::FreshRunRestore>>,
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut commits: bevy::prelude::ResMut<ResetCommits>,
) {
    if fresh.is_some() {
        commits.0.push(tick.0);
    }
}

/// Step until the durable latch rises, then a little further.
fn step_past_hydration(sim: &mut Platformer2dSimHarness) {
    let mut latched = false;
    for _ in 0..240 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .get_resource::<SaveRestored>()
            .is_some_and(|restored| restored.0)
        {
            latched = true;
            break;
        }
    }
    assert!(
        latched,
        "the durable latch never rose, so this world never reached the state a \
         menu press would be made in"
    );
}

/// Put a row in each occurrence baseline on tick 5, from inside the rewinding
/// schedule, so the New Game below has something to clear.
fn seed_both_baselines(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut occurrence_baseline: ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
        ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline,
    >,
    mut custody_baseline: ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
        ambition_platformer2d::platformer::lifecycle::CustodyBaseline,
    >,
) {
    use ambition_platformer2d::platformer::lifecycle::{
        AuthoredOccurrences, OccurrenceWhereabouts,
    };
    use ambition_platformer2d::platformer::sim_id::SimId;
    if tick.0 != 5 {
        return;
    }
    let subject = SimId::from_snapshot("probe:a_thing_the_checkpoint_remembers".to_string());
    let mut rows = std::collections::BTreeMap::new();
    rows.insert(subject.clone(), OccurrenceWhereabouts::InCustody);
    let mut remembered = AuthoredOccurrences::default();
    remembered.adopt_rows(rows);
    occurrence_baseline.adopt(remembered);
    let mut held = std::collections::BTreeMap::new();
    held.insert(subject, SimId::from_snapshot("probe:a_custodian".to_string()));
    custody_baseline.adopt(held);
}

fn baseline_rows(sim: &Platformer2dSimHarness) -> (usize, usize) {
    use ambition_platformer2d::platformer::lifecycle::{
        session_world_component, CustodyBaseline, OccurrenceBaseline,
    };
    let occurrences = session_world_component::<OccurrenceBaseline>(sim.world())
        .unwrap()
        .remembered()
        .rows()
        .count();
    let custody = session_world_component::<CustodyBaseline>(sim.world())
        .unwrap()
        .rows()
        .count();
    (occurrences, custody)
}

/// A New Game the host asks for commits exactly once under a rewind, rebuilds
/// the room, clears the occurrence baselines, and never lowers the
/// durable-restore latch to do it.
///
/// The menu writes `NewGameRequested` as a host intent; the ledger releases it
/// on its stamped tick in the first pass and in every resimulation, and the
/// checkpoint restore to the fresh baseline commits on the confirmed frame. A
/// rollback-registered latch set from `Update` would be restored away by the
/// next rewind (0 commits); a release that is not an edge would arm a second
/// reset from a replayed tick (2 commits). Resetting the durable domains, not
/// reloading the save, is what keeps the latch up: lowering it re-runs the load
/// chain over rollback-owned state while a session is live.
#[test]
fn a_new_game_asked_for_by_the_host_commits_once_under_a_rewind() {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            use bevy::prelude::IntoScheduleConfigs as _;
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.init_resource::<ResetCommits>();
            app.add_systems(
                ambition_platformer2d::platformer::lifecycle::CheckpointDomainApply,
                record_every_new_game_commit,
            );
            app.add_systems(
                label,
                seed_both_baselines.before(ambition_platformer2d::platformer::lifecycle::CheckpointRestore),
            );
            app.init_resource::<LatchHistory>();
            app.add_systems(
                bevy::prelude::Update,
                record_the_latch_every_frame.before(
                    ambition_platformer2d::session::durable_horizon::adopt_occurrence_checkpoint_from_save,
                ),
            );
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a reset-commit recorder");
    step_past_hydration(&mut sim);
    for _ in 0..20 {
        sim.step(AgentAction::default());
    }
    assert_ne!(
        baseline_rows(&sim),
        (0, 0),
        "the occurrence baselines never held the seeded rows, so 'New Game \
         cleared them' would be a claim about an empty subject"
    );
    // The roster proves the reset committed: a reset rebuilds the room, and
    // despawn bumps the generation, so no original `Entity` survives a rebuild.
    let roster_before = feature_roster(&mut sim);
    assert!(!roster_before.is_empty(), "the room had no roster before the reset");

    ambition_platformer2d::platformer::host_intents::write_host_intent(
        sim.world_mut(),
        ambition_platformer2d::session::reset::NewGameRequested,
    );
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }

    sim.rollback_health()
        .expect("the host's New Game desynced the sync test");
    let commits = sim.world().resource::<ResetCommits>().0.clone();
    assert_eq!(
        commits.len(),
        1,
        "a New Game written as a host intent committed {} time(s) (ticks \
         {commits:?}). 0: the rewind erased it. More than 1: the release is not \
         an edge and a replayed tick armed a second reset",
        commits.len(),
    );
    let survivors = roster_before.intersection(&feature_roster(&mut sim)).count();
    assert_eq!(
        survivors, 0,
        "{survivors} of {} roster entities survived, so the New Game did not \
         rebuild the room",
        roster_before.len()
    );
    assert_eq!(
        baseline_rows(&sim),
        (0, 0),
        "New Game left rows in the occurrence baselines, so the fresh run \
         inherits what the finished run remembered at its last checkpoint"
    );
    let history = &sim.world().resource::<LatchHistory>().0;
    let first_true = history
        .iter()
        .position(|held| *held)
        .expect("the latch rose before the New Game");
    let lowered_after: Vec<usize> = history
        .iter()
        .enumerate()
        .skip(first_true)
        .filter(|(_, held)| !**held)
        .map(|(index, _)| index)
        .collect();
    assert!(
        lowered_after.is_empty(),
        "`SaveRestored` went false again at sampled frames {:?} of {} after first \
         becoming true: New Game must reset its durable domains, not reload them",
        &lowered_after[..lowered_after.len().min(8)],
        history.len()
    );
}

const CUTSCENE_ID: &str = "cutscene_lab_intro";
const CUTSCENE_TRIGGER_AT: u64 = 30;

fn trigger_a_cutscene_from_inside_the_sim(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut queue: bevy::prelude::ResMut<ambition_platformer2d::cutscene::CutsceneTriggerQueue>,
) {
    if tick.0 == CUTSCENE_TRIGGER_AT {
        queue.request(CUTSCENE_ID);
    }
}

fn cutscene_beat(sim: &Platformer2dSimHarness) -> Option<(usize, bool)> {
    sim.world()
        .get_resource::<ambition_platformer2d::cutscene::ActiveCutscene>()
        .and_then(|active| active.runtime.as_ref())
        .map(|runtime| (runtime.beat_index, runtime.finished))
}

fn sim_playing_a_cutscene() -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.add_systems(label, trigger_a_cutscene_from_inside_the_sim);
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a cutscene fixture")
}

/// Step until the cutscene reaches the beat that waits for a dismiss.
fn step_until_the_cutscene_waits(sim: &mut Platformer2dSimHarness) -> usize {
    for _ in 0..200 {
        sim.step(AgentAction::default());
        if let Some((index, _)) = cutscene_beat(sim) {
            if index >= 1 {
                return index;
            }
        }
    }
    panic!(
        "the cutscene never reached its `Dialogue` beat, so nothing below is \
         measuring a dismiss edge. `{CUTSCENE_ID}` must be in `CutsceneLibrary` \
         and `drain_cutscene_triggers` must be installed"
    );
}

/// A cutscene dismiss and a cutscene skip ride the primary seat's input, so a
/// rewind re-delivers them and each is spent exactly once.
///
/// `cutscene_lab_intro` is banner, dialogue, a 0.4 s wait, dialogue. One confirm
/// on the first dialogue must land on the SECOND and stop there: beat 1 means
/// the press was lost to the rewind, past beat 3 means it was spent twice. The
/// skip is a HOLD of the seat's cancel, accumulated on the simulation clock:
/// released before `SKIP_HOLD_THRESHOLD_SECS` (1.2 s) the partial hold resets
/// and the cutscene plays on; held past it the cutscene ends.
#[test]
fn a_cutscene_dismiss_and_skip_on_the_seats_input_survive_the_rewind_once() {
    let mut sim = sim_playing_a_cutscene();
    let waiting = step_until_the_cutscene_waits(&mut sim);
    assert_eq!(waiting, 1, "the premise: the first dialogue beat is waiting");
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    assert_eq!(
        cutscene_beat(&sim),
        Some((1, false)),
        "the control: without a confirm the dialogue beat waits"
    );

    sim.step(AgentAction {
        confirm: true,
        ..AgentAction::default()
    });
    for _ in 0..60 {
        sim.step(AgentAction::default());
        health(&sim).unwrap_or_else(|error| panic!("the dismiss desynced: {error}"));
    }
    assert_eq!(
        cutscene_beat(&sim),
        Some((3, false)),
        "one confirm on the first dialogue beat must advance through the wait to \
         the second dialogue beat and stop: beat 1 means the press was lost to the \
         rewind, past beat 3 means it was spent twice"
    );

    let hold = |sim: &mut Platformer2dSimHarness, ticks: usize| {
        for _ in 0..ticks {
            sim.step(AgentAction {
                cancel_held: true,
                ..AgentAction::default()
            });
        }
        for _ in 0..30 {
            sim.step(AgentAction::default());
            health(sim).unwrap_or_else(|error| panic!("the skip hold desynced: {error}"));
        }
    };
    // 0.5 s: under the threshold.
    hold(&mut sim, 30);
    let partial = sim
        .world()
        .resource::<ambition_platformer2d::cutscene::CutsceneSkipHold>()
        .seconds;
    assert_eq!(
        (cutscene_beat(&sim), partial),
        (Some((3, false)), 0.0),
        "a released-early cancel skipped the cutscene or left its hold standing"
    );
    // 1.5 s: past it.
    hold(&mut sim, 90);
    assert_eq!(
        cutscene_beat(&sim),
        None,
        "a cancel held past the threshold did not skip the cutscene"
    );
}

fn player_health(sim: &mut Platformer2dSimHarness) -> i32 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<
        &ambition_platformer2d::characters::actor::BodyHealth,
        bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >();
    q.iter(world)
        .next()
        .map_or(i32::MIN, |health| health.health.current)
}

/// Damage the primary player so a heal has somewhere to go, on a timeline this
/// host owns.
///
/// It must NOT rebaseline through `rebase_rollback_history`: that calls
/// `start_sync_test_session`, which stamps `SyncTestOwner::Caller` and would
/// flip the ownership `maintainer_owned_rollback_sim` established, leaving the
/// owner stamp and the installed session disagreeing. So it re-arms through the
/// fixture's own seam: stop the stale session, arm the local policy again, and
/// let `maintain_local_session` install a fresh session over the damaged world.
fn damage_the_player_and_reassert_local_maintainer(
    sim: &mut Platformer2dSimHarness,
) {
    let world = sim.world_mut();
    let player = {
        let mut q = world.query_filtered::<
            bevy::prelude::Entity,
            bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        q.single(world)
            .expect("the sim boots exactly one primary player")
    };
    if let Some(mut health) =
        world.get_mut::<ambition_platformer2d::characters::actor::BodyHealth>(player)
    {
        health.health.max = 100;
        health.health.current = 50;
    }
    crate::common::hand_the_timeline_to_the_local_maintainer(sim);
    for _ in 0..4 {
        sim.step(AgentAction::default());
    }
    let boundary = format!(
        "{:?}",
        ambition_platformer2d::rollback::mechanical_mutation_boundary(sim.world())
    );
    assert_eq!(
        boundary, "LocallyRebasable",
        "this arm needs a live timeline THIS host owns after the re-arm and the \
         boundary reports `{boundary}`. `NoTimeline` means no session was \
         installed; `ForeignTimeline` means the ownership flip this helper \
         guards against happened anyway"
    );
    sim.rollback_health().unwrap_or_else(|error| {
        panic!("the re-baselined timeline is not healthy after the damage setup: {error}")
    });
}

/// A Health Cell used from the menu heals once and spends one cell, across a
/// rewind. The menu writes one `ItemUseRequested` host intent and
/// `apply_item_uses` spends the cell and raises the heal on the stamped tick,
/// in the first pass and in every resimulation. A heal written from `Update`
/// instead is revoked by GGRS's first correction.
#[test]
fn a_health_cell_used_from_the_menu_heals_once_and_spends_one_cell() {
    let mut sim = crate::common::maintainer_owned_rollback_sim(40);
    damage_the_player_and_reassert_local_maintainer(&mut sim);
    let before = player_health(&mut sim);
    let cells = |sim: &Platformer2dSimHarness| {
        sim.world()
            .resource::<ambition_platformer2d::items::OwnedItems>()
            .count(ambition_platformer2d::items::Item::HealthCell)
    };
    let cells_before = cells(&sim);
    assert!(cells_before >= 1, "the starter bag holds no Health Cell to use");
    ambition_platformer2d::platformer::host_intents::write_host_intent(
        sim.world_mut(),
        ambition_platformer2d::items::ItemUseRequested { participant: ambition_platformer2d::characters::control::PlayerSlot(0),
            item: ambition_platformer2d::items::Item::HealthCell,
        },
    );
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }
    sim.rollback_health().unwrap_or_else(|error| {
        panic!("the timeline diverged across the menu's item use: {error}")
    });
    assert_eq!(
        player_health(&mut sim),
        before + ambition_platformer2d::item_runtime::narrative::HEALTH_CELL_HEAL,
        "a Health Cell used from the menu did not heal exactly once. Equal to \
         `before`: the rewind revoked it. Twice the heal: a replayed tick spent \
         the intent a second time"
    );
    assert_eq!(
        cells(&sim),
        cells_before - 1,
        "the bag and the heal disagree: one use must spend exactly one cell"
    );
}

/// The tick the in-sim control raises its cycle on, well after the room has
/// settled.
const GRAVITY_CYCLE_TICK: u64 = 100;

fn cycle_gravity_from_inside_the_sim(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut requests: bevy::prelude::MessageWriter<
        ambition_platformer2d::world::AmbientGravityRequest,
    >,
) {
    if tick.0 == GRAVITY_CYCLE_TICK {
        requests.write(ambition_platformer2d::world::AmbientGravityRequest::Cycle);
    }
}

fn sim_cycling_gravity(with_an_in_sim_producer: bool) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            if with_an_in_sim_producer {
                app.add_systems(label, cycle_gravity_from_inside_the_sim);
            }
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a gravity-request producer")
}

/// Where one `AmbientGravityRequest::Cycle` takes `from`, by asking the
/// production rotation rather than restating it — a hand-written `(-1, 0)`
/// here would agree with a broken `BaseGravity::cycle`.
fn one_cycle_from(from: bevy::prelude::Vec2) -> bevy::prelude::Vec2 {
    let mut probe = ambition_platformer2d::world::BaseGravity::in_room(None, from);
    probe.cycle(None);
    probe.dir_in(None)
}

/// The ambient gravity direction the room simulates under. ⭐ `BaseGravity` is
/// `rollback_resource_canonical`
/// (`crates/ambition_platformer2d_shared_tangle/src/rollback_registration.rs:40`),
/// so it is ITSELF rollback state — which is what lets it witness a rollback
/// defect. A visible consequence that a rewind does not own could not.
fn base_gravity(sim: &Platformer2dSimHarness) -> bevy::prelude::Vec2 {
    let world = sim.world();
    // The one live room of this run, which is the room the request turns.
    let room = ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<
        ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
    >(world)
    .copied();
    world
        .get_resource::<ambition_platformer2d::world::BaseGravity>()
        .expect("the sim composition publishes ambient gravity as `BaseGravity`")
        .dir_in(room)
}

/// `AmbientGravityRequest` is written by the developer hotkey `cycle_dev_gravity`
/// in `Update` (`game/ambition_app/src/menu/kaleidoscope_app.rs`) and read by
/// `apply_ambient_gravity_requests` inside the sim schedule: a host-latched edge,
/// spent on a speculative frame, that nothing re-produces after the rewind
/// (Q136). The in-sim arm is the control: `BaseGravity::cycle` is a four-state
/// rotation, so a reader that never ran and one that ran four times both read
/// as unchanged, and only the same fixture flipping it from inside separates
/// "lost" from "this fixture cannot flip gravity".
///
/// Both arms assert the transition, not the endpoint, because
/// `reset_gravity_on_room_reset` restores the authored default on a room
/// replay inside the window; a reset moves gravity TO the default, never one
/// cycle step away from it.
///
/// It asserts the defect is still present and fires when it is fixed, so it is
/// a tracking measurement rather than a contract.
#[test]
#[ignore = "playtest: tracks a known dev-hotkey ingress defect (Q136); it fires when the defect is fixed, so it is not a gate"]
fn an_ambient_gravity_request_raised_outside_the_simulation_is_lost() {
    // CONTROL: the same message, written from inside the timeline.
    let mut inside = sim_cycling_gravity(true);
    let inside_before = base_gravity(&inside);
    let inside_cycled = one_cycle_from(inside_before);
    let mut inside_reached_the_cycle = false;
    for _ in 0..200 {
        inside.step(AgentAction::default());
        if base_gravity(&inside) == inside_cycled {
            inside_reached_the_cycle = true;
        }
    }
    assert!(
        inside_reached_the_cycle,
        "the in-sim control never reached {inside_cycled:?}, one cycle step from \
         {inside_before:?}, so `apply_ambient_gravity_requests` is either absent \
         from this composition or never reached — every number below would then \
         be measuring the fixture rather than the road"
    );

    // The dev hotkey's position: a producer outside the rewinding schedule, on
    // a timeline this host owns, in the SAME composition as the control, so the
    // only difference between the arms is where the message is written (the
    // sandbox composition does not reach `apply_ambient_gravity_requests`).
    let mut outside = sim_cycling_gravity(false);
    crate::common::hand_the_timeline_to_the_local_maintainer(&mut outside);
    for _ in 0..40 {
        outside.step(AgentAction::default());
    }
    let boundary = format!(
        "{:?}",
        ambition_platformer2d::rollback::mechanical_mutation_boundary(outside.world())
    );
    assert_eq!(
        boundary, "LocallyRebasable",
        "this arm needs a live timeline THIS host owns before it writes, and the          boundary reports `{boundary}` — the same check          `common::maintainer_owned_rollback_sim` makes for its callers"
    );
    outside
        .rollback_health()
        .expect("the fixture promises a healthy timeline before the write");
    let before = base_gravity(&outside);
    let cycled = one_cycle_from(before);
    outside
        .world_mut()
        .write_message(ambition_platformer2d::world::AmbientGravityRequest::Cycle);

    // Sampled every frame, because the loss is a REVOCATION and not an absence:
    // both end at `before` and only the transient tells them apart.
    let mut reached_the_cycle = false;
    for _ in 0..200 {
        outside.step(AgentAction::default());
        if base_gravity(&outside) == cycled {
            reached_the_cycle = true;
        }
    }
    // A flat zero, not a revocation: there is no transient to floor, and the
    // control above is what separates "lost" from "cannot flip gravity".
    assert!(
        !reached_the_cycle,
        "an ambient-gravity cycle requested from outside the simulation now \
         reaches {cycled:?} — that is the FIX this arm is waiting for, not a \
         regression. Invert it and record in [Q136] which ingress road reached \
         this message"
    );
    assert_eq!(
        base_gravity(&outside),
        before,
        "ambient gravity ended somewhere other than where it started without ever \
         passing through one cycle step, so something in this window moves \
         `BaseGravity` that is neither the request nor \
         `reset_gravity_on_room_reset` — find it before trusting the line above"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Q136 / SetFlagRequested: a message whose producer is ordered after its own
// consumer, so it straddles a tick boundary.
//
// `emit_intro_flag_chains` runs after `Platformer2dSimulationPhase::GameplayEffects`
// and its consumer `apply_flag_effects` runs inside it, so a request raised on
// tick N is read on tick N+1, and `clear_message_on_rollback` empties the
// buffer on a rewind in between. The arms ask the mechanism, with stand-in
// producers, rather than the intro chain itself.

/// Not a shipped flag id. A shipped one can be set by the world itself, and the
/// arm would be reading somebody else's write.
const WITNESS_FLAG: &str = "a_straddling_setflag_witness";

/// Gated on `SimTick`, not a `Local`: a `Local` is not rollback state, so the
/// replay would skip a branch the original pass took and manufacture its own
/// divergence.
const RAISE_AT: u64 = 40;

fn request_the_flag_once(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut requests: bevy::prelude::MessageWriter<
        ambition_platformer2d::combat::events::SetFlagRequested,
    >,
) {
    if tick.0 == RAISE_AT {
        requests.write(ambition_platformer2d::combat::events::SetFlagRequested {
            id: WITNESS_FLAG.to_string(),
            on: true,
            written: None,
        });
    }
}

/// ⛔ THE CONTROL THAT SEPARATES THE POSITION FROM THE PAYLOAD. Same schedule
/// slot, same `MessageWriter<SetFlagRequested>` parameter, same `SimTick` read —
/// and it never writes. If a system merely PRESENT after `GameplayEffects`
/// desyncs, the finding is about the slot and has nothing to do with straddling
/// messages. Every distinguishing feature measured only on the accused is
/// distinguishing by construction of the search.
fn request_nothing(
    _tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut _requests: bevy::prelude::MessageWriter<
        ambition_platformer2d::combat::events::SetFlagRequested,
    >,
) {
}

/// ⛤ **THE REAL PRODUCER'S SHAPE, WHICH THE ARMS ABOVE DELIBERATELY DO NOT
/// HAVE.** `emit_intro_flag_chains` is not an unconditional writer: it is
/// registered `run_if(resource_exists_and_changed::<AmbitionGameSave>)` and it
/// skips any target flag already present, so it is a DERIVATION over the save
/// rather than an edge. The plugin's argument for why the straddle is safe
/// rests entirely on that gate re-arming when a rewind restores the resource.
/// This reproduces both halves: gated on the same condition, and idempotent on
/// the target.
fn derive_the_flag_from_the_save(
    save: bevy::prelude::Res<AmbitionGameSave>,
    mut requests: bevy::prelude::MessageWriter<
        ambition_platformer2d::combat::events::SetFlagRequested,
    >,
) {
    // ⚠ NO TICK CONDITION, and the first fixture's had one — see
    // `PRIMER_FLAG`. A derivation gated on CHANGED runs on the tick after
    // something moved the save, which is not a tick this arm chooses.
    if !save.data().flag(WITNESS_FLAG) {
        requests.write(ambition_platformer2d::combat::events::SetFlagRequested {
            id: WITNESS_FLAG.to_string(),
            on: true,
            written: None,
        });
    }
}

/// The flag whose landing ARMS the derivation above.
///
/// The flag whose landing arms the derivation above. The derivation only runs
/// on a tick where `AmbitionGameSave` changed, and in a fixture where nothing
/// else writes the save it would never run; `emit_intro_flag_chains` is armed
/// the same way, by a previous flag write.
const PRIMER_FLAG: &str = "a_straddling_setflag_witness_primer";

/// Written once, on the SOUND road (before its consumer), purely to move the
/// save and arm the gate — the role an earlier authored flag plays in the
/// shipped intro chain.
fn prime_the_save_once(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut requests: bevy::prelude::MessageWriter<
        ambition_platformer2d::combat::events::SetFlagRequested,
    >,
) {
    if tick.0 == RAISE_AT {
        requests.write(ambition_platformer2d::combat::events::SetFlagRequested {
            id: PRIMER_FLAG.to_string(),
            on: true,
            written: None,
        });
    }
}

fn witness_flag_is_set(sim: &Platformer2dSimHarness) -> bool {
    sim.world()
        .get_resource::<AmbitionGameSave>()
        .is_some_and(|save| save.data().flag(WITNESS_FLAG))
}

/// The witness composition, parameterised on the producer's slot and on whether
/// a rollback session runs.
fn witness_sim(after_consumer: bool, rollback: bool) -> Platformer2dSimHarness {
    witness_sim_with(after_consumer, rollback, true)
}

/// The gated derivation, always AFTER its consumer — which is the shipped
/// registration's exact shape. `gate` exists so the two things that differ from
/// the one-shot arms can be separated: the `run_if` and the idempotent
/// derivation shape.
fn gated_witness_sim(rollback: bool) -> Platformer2dSimHarness {
    gated_witness_sim_with(rollback, true)
}

fn gated_witness_sim_with(rollback: bool, gate: bool) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    use bevy::prelude::IntoScheduleConfigs;
    let mut options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(ROOM);
    if rollback {
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    Platformer2dSimHarness::build(options, move |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        let label = app.sim_schedule();
        let phase =
            ambition_platformer2d::sim::Platformer2dSimulationPhase::GameplayEffects;
        app.add_systems(label, prime_the_save_once.before(phase));
        if gate {
            app.add_systems(
                label,
                derive_the_flag_from_the_save
                    .after(phase)
                    .run_if(bevy::prelude::resource_exists_and_changed::<AmbitionGameSave>),
            );
        } else {
            app.add_systems(label, derive_the_flag_from_the_save.after(phase));
        }
        Ok(())
    })
    .expect("the gated witness harness builds")
}

fn witness_sim_with(
    after_consumer: bool,
    rollback: bool,
    raise: bool,
) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    use bevy::prelude::IntoScheduleConfigs;
    let mut options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(ROOM);
    if rollback {
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    Platformer2dSimHarness::build(options, move |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        let label = app.sim_schedule();
        let phase =
            ambition_platformer2d::sim::Platformer2dSimulationPhase::GameplayEffects;
        match (after_consumer, raise) {
            (true, true) => app.add_systems(label, request_the_flag_once.after(phase)),
            (true, false) => app.add_systems(label, request_nothing.after(phase)),
            (false, true) => app.add_systems(label, request_the_flag_once.before(phase)),
            (false, false) => app.add_systems(label, request_nothing.before(phase)),
        };
        Ok(())
    })
    .expect("the witness harness builds")
}

fn drive(sim: &mut Platformer2dSimHarness, frames: usize) {
    for _ in 0..frames {
        sim.step(AgentAction::default());
    }
}

/// Step `frames` frames, or until the harness refuses a step because the
/// timeline desynced. For an arm whose subject is the desync: it reads the
/// verdict, and the world as it was when the timeline diverged.
fn drive_until_refused(sim: &mut Platformer2dSimHarness, frames: usize) {
    for _ in 0..frames {
        if sim.try_step(AgentAction::default()).is_err() {
            return;
        }
    }
}

/// The premise for the straddle arms: with no rollback and the producer
/// before its consumer, this composition can set the flag at all.
#[test]
#[ignore = "playtest: premise of the SetFlagRequested straddle investigation; not a gate"]
fn a_flag_requested_inside_the_tick_reaches_the_save_without_rollback() {
    let mut sim = witness_sim(false, false);
    drive(&mut sim, 240);
    assert!(
        witness_flag_is_set(&sim),
        "`apply_flag_effects` never wrote the witness flag in a world with no \
         rollback at all, so this file cannot tell a lost message from a \
         composition that does not carry the effect bus"
    );
}

/// Control for the straddle: the same request ordered BEFORE its consumer, the
/// case `clear_message_on_rollback` is designed for, under a live rewind.
#[test]
#[ignore = "playtest: control of the SetFlagRequested straddle investigation; not a gate"]
fn a_flag_requested_before_its_consumer_survives_the_rewind() {
    let mut sim = witness_sim(false, true);
    drive(&mut sim, 240);
    assert_eq!(
        health(&sim),
        Ok(()),
        "the sync test desynced, so nothing this arm reports is about the flag"
    );
    assert!(
        witness_flag_is_set(&sim),
        "a request raised BEFORE its consumer in the same tick did not reach the \
         save under rollback. That is the case `clear_message_on_rollback` is \
         designed for, so this failing makes the straddling arm unreadable"
    );
}

/// A request raised AFTER its consumer, in `emit_intro_flag_chains`'s slot, is
/// a pending past across the tick boundary: a rollback landing between raise
/// and read replays N+1 with the buffer already cleared, the flag is not
/// written on the replay, and the hashed `AmbitionGameSave` diverges. The result
/// is a checksum mismatch, not a quietly lost flag. It pins the frame and the
/// reporter, so it describes today's rollback host rather than a contract.
#[test]
#[ignore = "playtest: demonstrates a desync mechanism with pinned frame numbers and reporter wording; expected to change as the rollback host evolves"]
fn a_flag_requested_after_its_consumer_desyncs_the_timeline() {
    let mut sim = witness_sim(true, true);
    drive_until_refused(&mut sim, 240);

    let verdict = health(&sim);
    let Err(report) = verdict else {
        panic!(
            "the straddling request no longer desyncs. If `clear_message_on_rollback` \
             or the schedule changed, this arm has been REPAIRED and should be \
             rewritten to assert the clean outcome — do not delete it, the intro \
             chain still sits in this slot"
        );
    };
    // The rewind that loses the request loads frame `RAISE_AT - 1` and runs the
    // next four frames again (the check distance of this sim). Two reporters
    // name that one replay:
    //
    // - The first-run witness of the rollback host, on that step. It names
    //   the newest frame of the replay, `RAISE_AT + 3`: the first run of that
    //   frame has the flag in the save, and its first resimulation does not.
    // - GGRS, one step later. It names the frames that were saved again,
    //   `RAISE_AT..=RAISE_AT + 2`.
    //
    // The witness is first, and the harness does not step a session that is
    // not healthy, so the report is the witness's. A desync at another frame
    // is a different defect wearing this arm's name.
    let reported = RAISE_AT + 3;
    assert!(
        report.contains(&format!("[{reported}]")),
        "the timeline desynced, but not on the step this arm is about \
         (expected frame {reported}, the newest frame of the first replay \
         whose consumer finds the cleared buffer): {report}"
    );
    let reason = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackDiagnosticHistory>()
        .last()
        .map(|diagnostic| diagnostic.reason.clone());
    assert!(
        reason.as_deref().is_some_and(|reason| reason.contains("only the first run")),
        "the reporter is not the first-run witness, so the frame above is \
         another reporter's frame: {reason:?}"
    );
    assert!(
        !witness_flag_is_set(&sim),
        "the flag IS set after the desync, which would mean the divergence is \
         not the missing write this arm attributes it to"
    );
}

/// Slot control for the straddle: the same system shape in the same slot,
/// writing nothing, does not desync.
#[test]
#[ignore = "playtest: control of the SetFlagRequested straddle investigation; not a gate"]
fn a_silent_system_after_the_consumer_does_not_desync() {
    let mut sim = witness_sim_with(true, true, false);
    drive(&mut sim, 240);
    assert_eq!(
        health(&sim),
        Ok(()),
        "a system that writes NOTHING desynced from the slot after \
         `GameplayEffects`, so the straddling arm's desync is about the slot \
         rather than about the message and this file's reading of it is wrong"
    );
}

/// The shipped intro chain's shape survives the straddle: a derivation over the
/// save, gated on `resource_exists_and_changed::<AmbitionGameSave>` and skipping
/// a flag already present, in the slot after its consumer. A restore marks the
/// resource changed, which re-arms the gate, so the replayed tick re-raises the
/// message. A desync here makes `emit_intro_flag_chains` a live desync; a clean
/// timeline with no flag means the chain is lost quietly.
#[test]
fn the_gated_derivation_survives_the_straddle_that_desyncs_without_it() {
    let mut sim = gated_witness_sim(true);
    // The primer lands at tick 40 and the check distance is 4: 80 frames covers
    // the derivation and every replay of it.
    drive(&mut sim, 80);

    let verdict = health(&sim);
    assert_eq!(
        verdict,
        Ok(()),
        "the GATED derivation desynced in the same slot where the ungated one \
         does. The plugin's argument — that a restore re-arms \
         `resource_exists_and_changed` and so re-raises the message on the \
         replayed tick — does not hold, which makes the shipped \
         `emit_intro_flag_chains` a live desync rather than a repaired road"
    );
    assert!(
        witness_flag_is_set(&sim),
        "the timeline stayed in sync but the derived flag never reached the \
         save across the window, so the chain is lost quietly instead of loudly"
    );
}

/// Which half survives the rewind, the `run_if` gate or the idempotent
/// derivation: the same derivation with the gate removed. Either outcome is a
/// finding about Q136's escape three, not a defect.
#[test]
#[ignore = "playtest: research question for Q136 (either outcome is a finding, neither a defect); not a gate"]
fn separating_the_gate_from_the_derivation_shape() {
    let mut sim = gated_witness_sim_with(true, false);
    drive(&mut sim, 240);
    let verdict = health(&sim);
    assert!(
        witness_flag_is_set(&sim),
        "the ungated derivation never set the flag, so this arm separated \
         nothing"
    );
    assert_eq!(
        verdict,
        Ok(()),
        "the UNGATED derivation desyncs while the gated one does not, so the \
         `run_if` is load-bearing rather than a cost optimisation, and `Q136`'s \
         escape three is incomplete without it"
    );
}

/// A startup load with both halves of the durable horizon seeded is applied by
/// the simulation on a live sync-test timeline, and every frame stays healthy.
///
/// The save is read once, at `Startup`; the simulation applies it
/// (`DurableRestoreSet`) from rollback state, so the writes to the hashed
/// `OccurrenceBaseline` and `CustodyBaseline` are an ordinary step that a rewind
/// past it applies again on the same tick. The seeded rows are what make it a
/// measurement: an empty horizon hydrates to the value the baselines already
/// hold. The premise is that the rise is on the timeline: frames with a live
/// session and an unapplied save must exist.
#[test]
fn a_startup_load_is_applied_on_the_timeline_and_resimulates_identically() {
    use ambition_platformer2d::persistence::save_data::{
        PersistedCustody, PersistedOccurrence, PersistedWhereabouts,
    };
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_durable_horizon(
        vec![PersistedOccurrence::new(
            "probe:seeded_occurrence",
            PersistedWhereabouts::Placed {
                room: ROOM.to_string(),
                x: 64,
                y: 64,
            },
        )],
        vec![PersistedCustody::new(
            "probe:seeded_occurrence",
            "probe:seeded_custodian",
        )],
    );
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10)
            .with_save(save),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            app.init_resource::<LiveBeforeRestored>();
            app.add_systems(bevy::prelude::Last, record_a_timeline_over_an_unrestored_save);
            Ok(())
        },
    )
    .expect("the sync-test harness boots with a save file");
    // Hydration completes within the first frames; 60 covers it and well more
    // than one check distance after it.
    for frame in 0..60 {
        sim.step(AgentAction::default());
        health(&sim).unwrap_or_else(|error| panic!("frame {frame} after a startup load desynced: {error}"));
    }
    // The premise: the load happened, and both baselines hold the seeded rows.
    assert!(sim.world().resource::<SaveRestored>().0, "the startup load never completed");
    let occurrence_rows = ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline>(sim.world()).unwrap()
        .remembered()
        .rows()
        .count();
    let custody_rows = ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::platformer::lifecycle::CustodyBaseline>(sim.world()).unwrap()
        .rows()
        .count();
    assert!(
        occurrence_rows > 0 && custody_rows > 0,
        "the seeded horizon never reached the baselines (occurrences {occurrence_rows}, \
         custody {custody_rows}), so nothing below was under pressure"
    );
    let order = sim.world().resource::<LiveBeforeRestored>();
    assert!(order.live > 0, "the GGRS session was never live, so no order was observed");
    assert!(
        order.unrestored > 0,
        "the save was applied before the session went live, so the rise never \
         crossed the timeline and the health checks above say nothing about it"
    );
}

/// Frames with a live GGRS session, and how many of them had an unrestored save.
#[derive(bevy::prelude::Resource, Default)]
struct LiveBeforeRestored {
    live: u32,
    unrestored: u32,
}

fn record_a_timeline_over_an_unrestored_save(world: &mut bevy::prelude::World) {
    if !ambition_platformer2d::rollback::session_is_active(world) {
        return;
    }
    let restored = world.resource::<SaveRestored>().0;
    let mut order = world.resource_mut::<LiveBeforeRestored>();
    order.live += 1;
    order.unrestored += u32::from(!restored);
}

/// A host intent acts on the tick it is stamped for.
///
/// The host writes after tick N and stamps N+1, so the first simulation step
/// after the write must spend the item, and that step is tick N+1. The
/// control is the moment after the write: the host does not act on its own
/// intent. A release that runs before the clock names the step's tick sees
/// tick N, holds the intent, and spends the item one step late, on tick N+2.
#[test]
fn a_host_intent_acts_on_the_tick_it_is_stamped_for() {
    let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_options())
        .expect("the sandbox builds headlessly");
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let cells = |sim: &Platformer2dSimHarness| sim.world().resource::<OwnedItems>().count(Item::HealthCell);
    let tick = |sim: &Platformer2dSimHarness| sim.world().resource::<ambition_platformer2d::time::SimTick>().0;
    let before = cells(&sim);
    assert!(before >= 1, "the starter bag holds no Health Cell to use");
    let written_after = tick(&sim);

    ambition_platformer2d::platformer::host_intents::write_host_intent(
        sim.world_mut(),
        ambition_platformer2d::items::ItemUseRequested { participant: ambition_platformer2d::characters::control::PlayerSlot(0), item: Item::HealthCell },
    );
    assert_eq!(cells(&sim), before, "control: the host spent the item itself");

    sim.step(AgentAction::default());
    assert_eq!(tick(&sim), written_after + 1, "one step is one tick");
    assert_eq!(
        cells(&sim),
        before - 1,
        "an intent stamped for tick {} was not acted on in that tick",
        written_after + 1
    );
}
