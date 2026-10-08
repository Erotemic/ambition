//! A checkpoint restore that is cancelled changes nothing in the live world.
//!
//! A restore is admitted, then its room is prepared, then the room is
//! published, then the restore commits. A restore can end before the commit:
//! its room preparation fails, or the publication is refused. A request can
//! also be refused before it is admitted, when the session cannot name the
//! body it returns. Then the outcome is `CheckpointRestoreOutcome::Cancelled`,
//! and the live world must be the world before the request.
//!
//! Review of 2026-10-05, P1: the admission wrote `RoomReplayAdmitted` at once,
//! and fourteen systems read it on that frame. They returned the subject to
//! spawn, made the boss undefeated in the save, took back its reward, and
//! changed the ownership of the timers and the consumed occurrences, all
//! before the room was prepared. So a cancelled restore had changed the world.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::actors::session::checkpoint::{CheckpointRestoreOutcome, SessionCheckpointOutcomes};
use bevy::prelude::*;

use crate::boss_lifecycle::{boss_cleared, kill_boss_with_a_real_hit, spawn_mockingbird};
use crate::death_restores_the_checkpoint::commit_a_checkpoint;

type LoadState = ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;
type LoadPhase = ambition_platformer2d::runtime::room_transition::RoomTransitionLoadPhase;

const BOSS: &str = "cancelled_restore_boss";

/// Each room transaction of this machine fails, as a room whose preparation
/// cannot complete would.
fn each_preparation_fails(mut state: ResMut<LoadState>) {
    if let Some(active) = state.active.as_mut().filter(|active| active.phase != LoadPhase::Committed) {
        active.phase = LoadPhase::Failed;
        active.failure = Some("this test fails each preparation".to_string());
    }
}

/// What the live world says, for the facts a restore changes.
#[derive(Debug, PartialEq)]
struct Facts {
    /// The primary body's position, to the pixel.
    at: (i32, i32),
    /// The save records the boss cleared.
    cleared: bool,
    /// The primary body's purse.
    balance: i32,
    /// The defeats recorded since the checkpoint.
    defeats_since: usize,
}

fn facts(sim: &mut Platformer2dSimHarness) -> Facts {
    let world = sim.world_mut();
    // The body of seat 0, by its identity: one arm gives a second body the
    // primary markers, and the facts are those of the first.
    let (at, balance) = world
        .query_filtered::<(
            &ambition_platformer2d::platformer::sim_id::SimId,
            &ambition_platformer2d::engine_core::BodyKinematics,
            &ambition_platformer2d::characters::actor::BodyWallet,
        ), With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .iter(world)
        .find(|(id, ..)| id.as_str() == "slot:0")
        .map(|(_, kin, wallet)| ((kin.pos.x.round() as i32, kin.pos.y.round() as i32), wallet.balance))
        .expect("the session has the primary body of seat 0");
    let defeats_since = world
        .resource::<ambition_platformer2d::boss_encounter::BossDefeatsSinceCheckpoint>()
        .defeats()
        .count();
    Facts {
        at,
        cleared: boss_cleared(sim, BOSS),
        balance,
        defeats_since,
    }
}

/// How the restore of an arm ends.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Road {
    /// The control: the room is prepared and published, and the restore
    /// commits.
    Commits,
    /// The preparation of its room fails.
    PreparationFails,
    /// Its room is prepared, and the publication is refused.
    PublicationRefused(Refusal),
    /// The session has two primary bodies, so the restore cannot name the
    /// body it returns. No production code builds this world; the second
    /// body is built by the recipe of a home body with the two markers.
    TwoPrimaryBodies,
}

/// Where `verify_and_publish` refuses the room of a restore. The two places
/// are different code: only the second one is beside the line that runs the
/// consequences of a restore.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Refusal {
    /// Before the room is built. Two holders of one identity make a world the
    /// opening baseline cannot describe, as in
    /// `a_refused_world_reload_leaves_the_running_game_untouched`. MEASURED
    /// 2026-10-05: one more holder of an identity that is live in the room
    /// gives the same early refusal, for each of five identities.
    BeforeTheRoomIsBuilt,
    /// At the verdict, after the room is built. The content generation of the
    /// session moves between the preparation and the publication, as a content
    /// reload that lands in that interval moves it. The injector is the one of
    /// `a_room_the_transaction_refuses_leaves_the_room_the_player_is_in_intact`.
    AtTheVerdict,
}

type Verification = ambition_platformer2d::actors::world::rooms::LastConstructionVerification;

/// A checkpoint at 7 coins; after it, a boss defeated, its bounty paid, and
/// the body moved away from the checkpoint. Then a restore is asked for.
/// Returns the facts before the request, the facts after its outcome, the
/// outcome, and what the last room verification concluded.
fn a_restore_after_a_boss(
    road: Road,
) -> (Facts, Facts, Option<CheckpointRestoreOutcome>, Verification, Platformer2dSimHarness) {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    sim.step_n(AgentAction::default(), 15);
    {
        let world = sim.world_mut();
        world
            .query_filtered::<&mut ambition_platformer2d::characters::actor::BodyWallet, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single_mut(world)
            .expect("the player has a wallet")
            .balance = 7;
    }
    sim.rebase_rollback_history().expect("the rollback history rebases over the wallet");
    commit_a_checkpoint(&mut sim);

    spawn_mockingbird(&mut sim, BOSS);
    kill_boss_with_a_real_hit(&mut sim, BOSS, 600);
    for _ in 0..400 {
        if boss_cleared(&sim, BOSS) && facts(&mut sim).balance > 7 {
            break;
        }
        sim.step(AgentAction::default());
    }
    let (x, y) = facts(&mut sim).at;
    sim.teleport_player((x as f32 + 64.0, y as f32));
    // At rest before the facts are read: a body still falling when the
    // request is made is somewhere else when the outcome is read, whatever
    // the restore did. The boss's reward chest can be under the teleport.
    let mut at_rest = 0;
    for _ in 0..240 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let still = world
            .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .iter(world)
            .all(|kin| kin.vel == ambition_platformer2d::engine_core::Vec2::ZERO);
        at_rest = if still { at_rest + 1 } else { 0 };
        if at_rest >= 10 {
            break;
        }
    }
    assert!(at_rest >= 10, "precondition: the body comes to rest after the teleport");
    let before = facts(&mut sim);

    if road == Road::PublicationRefused(Refusal::BeforeTheRoomIsBuilt) {
        for _ in 0..2 {
            sim.world_mut()
                .spawn(ambition_platformer2d::platformer::sim_id::SimId::placement("corrupt_twin"));
        }
    }
    if road == Road::PreparationFails {
        sim.app_mut().add_systems(
            Update,
            each_preparation_fails
                .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system)
                .before(ambition_platformer2d::runtime::room_transition::finalize_unpresented_room_transition_failure_system),
        );
    }
    if road == Road::TwoPrimaryBodies {
        crate::a_home_body_is_built_for_a_seat::build_a_home_body(
            &mut sim,
            ambition_platformer2d::characters::control::PlayerSlot(1),
            (
                ambition_platformer2d::platformer::markers::PrimaryPlayer,
                ambition_platformer2d::platformer::body::PrimaryBody,
            ),
        );
    }
    let outcome = ask_for_a_restore(&mut sim, road);
    sim.step_n(AgentAction::default(), 30);
    let verification = sim.world().resource::<Verification>().clone();
    (before, facts(&mut sim), outcome, verification, sim)
}

/// Ask for a restore to the checkpoint, and step until the session answers
/// it, for 300 frames at most.
fn ask_for_a_restore(sim: &mut Platformer2dSimHarness, road: Road) -> Option<CheckpointRestoreOutcome> {
    let answered_before = ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionCheckpointOutcomes>(sim.world())
        .expect("the live session root carries the checkpoint coordinator").latest().cloned();
    sim.world_mut()
        .write_message(ambition_platformer2d::platformer::lifecycle::ResetToCheckpoint);
    let mut outcome = None;
    let mut epoch = 9_000u64;
    for _ in 0..300 {
        if road == Road::PublicationRefused(Refusal::AtTheVerdict) {
            // A moving generation. A constant one is taken into the plan when
            // the room is prepared, and then it is equal to itself at the
            // verdict.
            epoch += 1;
            ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
                sim.world_mut(),
                ambition_platformer2d::actors::rooms::ActiveContentBinding::content(
                    ambition_platformer2d::engine_core::ContentEpoch(epoch),
                    Default::default(),
                ),
            );
        }
        sim.step(AgentAction::default());
        let latest = ambition_platformer2d::platformer::lifecycle::session_world_component::<SessionCheckpointOutcomes>(sim.world())
        .expect("the live session root carries the checkpoint coordinator").latest().cloned();
        if latest != answered_before {
            outcome = latest;
            break;
        }
    }
    outcome
}

/// THE PROPERTY. The restore's preparation fails, the outcome is
/// `Cancelled`, and each fact is what it was before the request: the body
/// where it stood, the boss cleared, the bounty in the purse, the defeat
/// still recorded since the checkpoint.
#[test]
fn a_cancelled_checkpoint_restore_leaves_the_live_world_as_it_was() {
    let (before, after, outcome, ..) = a_restore_after_a_boss(Road::PreparationFails);
    assert!(
        outcome.as_ref().is_some_and(|outcome| outcome.cancellation().is_some()),
        "the restore's outcome: {outcome:?}"
    );
    assert_eq!(after, before, "the live world after a cancelled restore, against the world before the request");
}

/// THE SAME PROPERTY ON THE SECOND ROAD. The room of the restore is prepared
/// and the publication is refused. The consequences of a restore run when the
/// verdict accepts the room, so none ran. The operation has no commit and no
/// failed preparation to answer it: the retirement of the operation gives the
/// one outcome, `Cancelled` with `NotCommitted`.
///
/// Review of 2026-10-05, P1, the second hole: on this road the intent was
/// taken and the operation was retired with no outcome.
///
/// ⛔ THE THIRD ROAD HAS NO ARM HERE, AND THAT IS A FINDING. A subject that is
/// gone or cannot transit reaches the same retirement. The subject of a
/// restore is the one primary body, and no production code removes that body
/// or its motion model, cluster or combat state while its session lives: the
/// one despawn is the retirement of the session, which takes the operation
/// back. An arm for it would remove the body by hand.
fn a_refused_publication_changes_nothing(refusal: Refusal) -> Verification {
    use ambition_platformer2d::actors::session::checkpoint::RestoreCancellation;
    let (before, after, outcome, verification, _) = a_restore_after_a_boss(Road::PublicationRefused(refusal));
    // ⛔ THE PREMISE: this is the road of a refused publication. A room that
    // was not prepared ends as `PreparationFailed`, and that is the arm above.
    assert!(
        !verification.published,
        "{refusal:?}: the room of the restore was published, so this arm is about a commit"
    );
    assert_eq!(
        outcome.as_ref().and_then(|outcome| outcome.cancellation()),
        Some(RestoreCancellation::NotCommitted),
        "{refusal:?}: the one outcome of a restore whose publication is refused: {outcome:?}"
    );
    assert_eq!(
        after, before,
        "{refusal:?}: the live world after a refused publication, against the world before the request"
    );
    verification
}

/// The refusal before the room is built.
#[test]
fn a_checkpoint_restore_whose_publication_is_refused_leaves_the_live_world_as_it_was() {
    let verification = a_refused_publication_changes_nothing(Refusal::BeforeTheRoomIsBuilt);
    // ⛔ THE PREMISE OF THE NAME: an early refusal records no violation, because
    // it ends before the verification that finds them.
    assert!(
        verification.violations.is_empty()
            && verification.projection_violations.is_empty()
            && verification.staged_violations.is_empty(),
        "this refusal came from the verdict, so it is the arm below: {verification:?}"
    );
}

/// The refusal at the verdict. This is the arm that reads the line in
/// `verify_and_publish` where the consequences of a restore run: MEASURED
/// 2026-10-05, with the consequences run before the verdict is read, this arm
/// is red and the arm above is green, because an early refusal does not come
/// to that line.
#[test]
fn a_checkpoint_restore_whose_room_fails_its_verdict_leaves_the_live_world_as_it_was() {
    let verification = a_refused_publication_changes_nothing(Refusal::AtTheVerdict);
    // ⛔ THE PREMISE OF THE NAME: the room was built and the verdict found it
    // prepared against a generation that is not the live one.
    assert!(
        verification.violations.iter().any(|violation| matches!(
            violation,
            ambition_platformer2d::platformer::construction::RosterViolation::ContentBindingMismatch { .. }
        )),
        "the room of the restore was not refused for its content generation: {verification:?}"
    );
}

/// THE SAME PROPERTY FOR A RESTORE THAT CANNOT NAME ITS SUBJECT. The subject
/// of a restore is the one primary body. With two primary bodies the session
/// has no subject to name, and no frame of play changes that. The restore is
/// refused at once with one outcome, `Cancelled` with `AmbiguousSubject`, and
/// the live world is the world before the request.
///
/// MEASURED 2026-10-05 before the repair: the request stayed owed
/// (`OutstandingCheckpointRequest`) for 300 frames with no operation and no
/// outcome, because a restore with no subject waited for one, and two
/// subjects read as none.
///
/// The refusal spends the request. So when the second body is not primary
/// any more, the next request is a new operation, and it commits.
#[test]
fn a_checkpoint_restore_with_two_primary_bodies_is_refused_and_the_next_one_commits() {
    use ambition_platformer2d::actors::session::checkpoint::{OutstandingCheckpointRequest, RestoreCancellation};
    use ambition_platformer2d::platformer::body::PrimaryBody;
    use ambition_platformer2d::platformer::markers::PrimaryPlayer;
    let (before, after, outcome, _, mut sim) = a_restore_after_a_boss(Road::TwoPrimaryBodies);
    let primaries: Vec<String> = {
        let world = sim.world_mut();
        let mut ids: Vec<String> = world
            .query_filtered::<&ambition_platformer2d::platformer::sim_id::SimId, With<PrimaryPlayer>>()
            .iter(world)
            .map(|id| id.as_str().to_string())
            .collect();
        ids.sort();
        ids
    };
    assert_eq!(primaries, ["slot:0", "slot:1"], "the premise: two primary bodies in the session");
    assert_eq!(
        (
            outcome.as_ref().and_then(|outcome| outcome.cancellation()),
            ambition_platformer2d::platformer::lifecycle::session_world_component::<OutstandingCheckpointRequest>(sim.world())
        .expect("the live session root carries the checkpoint coordinator").0,
        ),
        (Some(RestoreCancellation::AmbiguousSubject), None),
        "(the outcome, the request still owed) of a restore with two primary bodies: {outcome:?}"
    );
    assert_eq!(after, before, "the live world after the refusal, against the world before the request");

    // The second body is an ordinary home body again.
    let second = {
        let world = sim.world_mut();
        world
            .query_filtered::<(Entity, &ambition_platformer2d::platformer::sim_id::SimId), With<PrimaryPlayer>>()
            .iter(world)
            .find(|(_, id)| id.as_str() == "slot:1")
            .map(|(entity, _)| entity)
            .expect("the second primary body")
    };
    sim.world_mut().entity_mut(second).remove::<(PrimaryPlayer, PrimaryBody)>();
    sim.rebase_rollback_history().expect("the rollback history rebases over the markers");
    let next = ask_for_a_restore(&mut sim, Road::Commits);
    sim.step_n(AgentAction::default(), 30);
    assert!(
        next.as_ref().is_some_and(CheckpointRestoreOutcome::committed) && next != outcome,
        "the next restore, with one primary body: {next:?}; the refused one: {outcome:?}"
    );
    let restored = facts(&mut sim);
    assert_eq!(
        (restored.at != before.at, restored.cleared, restored.balance, restored.defeats_since),
        (true, false, 7, 0),
        "(the body moved, cleared, balance, defeats since) after the restore that commits"
    );
}

/// THE CONTROL. The same run with a preparation that succeeds commits, and
/// each fact moves: so the facts above are facts a restore changes.
#[test]
fn a_committed_checkpoint_restore_changes_each_of_those_facts() {
    let (before, after, outcome, verification, _) = a_restore_after_a_boss(Road::Commits);
    assert!(verification.published, "control: the room of a committed restore was published");
    assert!(outcome.as_ref().is_some_and(CheckpointRestoreOutcome::committed), "the restore's outcome: {outcome:?}");
    assert!(
        before.cleared && before.balance > 7 && before.defeats_since == 1,
        "precondition: the boss defeated and its bounty paid after the checkpoint: {before:?}"
    );
    assert_eq!(
        (after.at != before.at, after.cleared, after.balance, after.defeats_since),
        (true, false, 7, 0),
        "(the body moved, cleared, balance, defeats since) after a committed restore; before: {before:?}"
    );
}

/// The authored boss of `trex_arena`, and whether the save records it
/// cleared: (the boss bodies of the room that are alive, those that are not,
/// cleared).
fn the_arena_boss(sim: &mut Platformer2dSimHarness) -> (Option<String>, usize, usize, bool) {
    let world = sim.world_mut();
    let bosses: Vec<(String, bool)> = world
        .query::<(
            &ambition_platformer2d::boss_encounter::BossConfig,
            &ambition_platformer2d::characters::actor::BodyHealth,
        )>()
        .iter(world)
        .map(|(config, health)| (config.id.clone(), health.alive()))
        .collect();
    let id = bosses.first().map(|(id, _)| id.clone());
    let alive = bosses.iter().filter(|(_, alive)| *alive).count();
    let cleared = id.as_deref().is_some_and(|id| boss_cleared(sim, id));
    (id, alive, bosses.len() - alive, cleared)
}

/// AFTER A RESTORE, THE ROOM AND THE SAVE AGREE ABOUT AN AUTHORED BOSS.
///
/// A checkpoint in `trex_arena`, then its authored boss defeated, then
/// a death. After the restore the arena holds its boss alive, and the save does
/// not record it cleared. On each frame of the death and the restore, a boss
/// that is alive is not presented as defeated. Control: before the death the
/// boss is defeated and cleared.
///
/// The room is built from the facts the restore's consequences will leave
/// (`session::checkpoint::prospective_commit_fates`), because those
/// consequences run only when the publication is accepted. Two layers decide
/// a restored boss, measured 2026-10-05. Its LIFE is the encounter driver's:
/// `update_boss_encounters` gives the boss its full health on its first tick
/// unless the save records the placement cleared, and the retraction has
/// already taken that record back. Its first PHASE is construction's: a boss
/// built with the fate `Dead` starts `Defeated`. So with the prospect's boss
/// half poisoned (the room built from the save before the retraction), the
/// boss is alive with full health and `Defeated` on the first frame of the
/// restored room, and `Active` one frame later. The frame check below sees
/// that frame.
#[test]
fn a_restored_room_builds_the_boss_the_restore_takes_back_alive() {
    use crate::common::fixed_60hz_room_sim;
    // A boss arena with a floor: in the Mockingbird's sky, a fall once it is
    // dead drops the player out of the room before its defeat is recorded.
    let mut sim = fixed_60hz_room_sim("trex_arena");
    sim.step_n(AgentAction::default(), 15);
    commit_a_checkpoint(&mut sim);
    let (id, alive, _, cleared) = the_arena_boss(&mut sim);
    let id = id.expect("precondition: the arena authors a boss");
    assert_eq!((alive, cleared), (1, false), "precondition: the arena's boss stands at the checkpoint");
    kill_boss_with_a_real_hit(&mut sim, &id, 600);
    for _ in 0..400 {
        if boss_cleared(&sim, &id) {
            break;
        }
        sim.step(AgentAction::default());
    }
    let (_, alive, _, cleared) = the_arena_boss(&mut sim);
    assert_eq!((alive, cleared), (0, true), "control: (bosses alive, cleared) after the defeat");
    crate::death_restores_the_checkpoint::die_and_watch(&mut sim, |sim, frame| {
        let world = sim.world_mut();
        let shown_dead_alive: Vec<_> = world
            .query::<(
                &ambition_platformer2d::characters::actor::BodyHealth,
                &ambition_platformer2d::combat::components::BossPhase,
            )>()
            .iter(world)
            .filter(|(health, phase)| health.alive() && !phase.is_active())
            .map(|(health, _)| health.health.current)
            .collect();
        assert!(
            shown_dead_alive.is_empty(),
            "frame {frame} of the death: a boss that is alive (health {shown_dead_alive:?}) is presented as defeated"
        );
    });
    let (_, alive, dead, cleared) = the_arena_boss(&mut sim);
    assert_eq!(
        (alive, dead, cleared),
        (1, 0, false),
        "(bosses alive, bosses dead, cleared) after the restore: the room and the save must agree"
    );
}

