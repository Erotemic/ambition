//! A checkpoint restore that is cancelled changes nothing in the live world.
//!
//! A restore is admitted, then its room is prepared, then the room is
//! published, then the restore commits. A restore can end before the commit:
//! its room preparation fails, or the publication is refused. Then the outcome
//! is `CheckpointRestoreOutcome::Cancelled`, and the live world must be the
//! world before the request.
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
    let (at, balance) = world
        .query_filtered::<(
            &ambition_platformer2d::engine_core::BodyKinematics,
            &ambition_platformer2d::characters::actor::BodyWallet,
        ), With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .map(|(kin, wallet)| ((kin.pos.x.round() as i32, kin.pos.y.round() as i32), wallet.balance))
        .expect("the session has one primary body");
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

/// A checkpoint at 7 coins; after it, a boss defeated, its bounty paid, and
/// the body moved away from the checkpoint. Then a restore is asked for.
/// Returns the facts before the request, the facts after its outcome, and the
/// outcome.
fn a_restore_after_a_boss(preparation_fails: bool) -> (Facts, Facts, Option<CheckpointRestoreOutcome>) {
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
    sim.step_n(AgentAction::default(), 30);
    let before = facts(&mut sim);

    if preparation_fails {
        sim.app_mut().add_systems(
            Update,
            each_preparation_fails
                .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system)
                .before(ambition_platformer2d::runtime::room_transition::finalize_unpresented_room_transition_failure_system),
        );
    }
    let answered_before = sim.world().resource::<SessionCheckpointOutcomes>().latest().cloned();
    sim.world_mut()
        .write_message(ambition_platformer2d::platformer::lifecycle::ResetToCheckpoint);
    let mut outcome = None;
    for _ in 0..300 {
        sim.step(AgentAction::default());
        let latest = sim.world().resource::<SessionCheckpointOutcomes>().latest().cloned();
        if latest != answered_before {
            outcome = latest;
            break;
        }
    }
    sim.step_n(AgentAction::default(), 30);
    (before, facts(&mut sim), outcome)
}

/// THE PROPERTY. The restore's preparation fails, the outcome is
/// `Cancelled`, and each fact is what it was before the request: the body
/// where it stood, the boss cleared, the bounty in the purse, the defeat
/// still recorded since the checkpoint.
#[test]
fn a_cancelled_checkpoint_restore_leaves_the_live_world_as_it_was() {
    let (before, after, outcome) = a_restore_after_a_boss(true);
    assert!(
        outcome.as_ref().is_some_and(|outcome| outcome.cancellation().is_some()),
        "the restore's outcome: {outcome:?}"
    );
    assert_eq!(after, before, "the live world after a cancelled restore, against the world before the request");
}

/// THE CONTROL. The same run with a preparation that succeeds commits, and
/// each fact moves: so the facts above are facts a restore changes.
#[test]
fn a_committed_checkpoint_restore_changes_each_of_those_facts() {
    let (before, after, outcome) = a_restore_after_a_boss(false);
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

/// The authored boss of `mockingbird_arena`, and whether the save records it
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
/// A checkpoint in `mockingbird_arena`, then its authored boss defeated, then
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
/// boss is alive with 28 health and `Defeated` on the first frame of the
/// restored room, and `Active` one frame later. The frame check below sees
/// that frame.
#[test]
fn a_restored_room_builds_the_boss_the_restore_takes_back_alive() {
    use crate::common::fixed_60hz_room_sim;
    let mut sim = fixed_60hz_room_sim("mockingbird_arena");
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

