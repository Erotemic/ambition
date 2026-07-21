//! GGRS rollback integration gates for the real Ambition simulation harness.
//!
//! `SyncTestSession` is the canary: each harness update saves the world, advances
//! it, rewinds a configurable distance, resimulates with the same inputs, and
//! compares checksums. These tests intentionally exercise the product simulation
//! rather than a toy counter or the retired custom snapshot engine.

#![cfg(feature = "rl_sim")]

use ambition::runtime::rollback::{Rollback, RollbackRegistry};
use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim, SandboxSimOptions, TimestepMode};
use bevy::prelude::With;

fn rollback_sim() -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds")
}

fn scripted_action(frame: usize) -> AgentAction {
    AgentAction {
        move_x: if frame % 24 < 12 { 1.0 } else { -1.0 },
        jump: frame % 17 == 0,
        jump_held: frame % 17 < 5,
        dash: frame % 29 == 3,
        attack: frame % 11 == 2,
        projectile: frame % 13 == 4,
        ..AgentAction::default()
    }
}

#[test]
fn sync_test_session_performs_real_rewinds_and_resimulation() {
    let mut sim = rollback_sim();
    for frame in 0..32 {
        sim.step(scripted_action(frame));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }

    let stats = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed");
    assert!(
        stats.load_runs > 0,
        "SyncTestSession issued LoadWorld requests"
    );
    assert!(
        stats.advance_runs > 32,
        "resimulation executes more GGRS frames than external harness steps: {stats:?}"
    );
}

#[test]
fn two_ggrs_harnesses_match_under_the_same_input_stream() {
    let mut left = rollback_sim();
    let mut right = rollback_sim();

    for frame in 0..48 {
        let action = scripted_action(frame);
        let a = left.step(action);
        let b = right.step(action);
        left.rollback_health().unwrap();
        right.rollback_health().unwrap();

        assert_eq!(
            a.player_pos, b.player_pos,
            "position diverged at frame {frame}"
        );
        assert_eq!(
            a.player_vel, b.player_vel,
            "velocity diverged at frame {frame}"
        );
        assert_eq!(a.hp, b.hp, "health diverged at frame {frame}");
        assert_eq!(
            a.active_room, b.active_room,
            "room diverged at frame {frame}"
        );
        assert_eq!(
            a.enemies.len(),
            b.enemies.len(),
            "entity population diverged at frame {frame}"
        );
        assert_eq!(
            a.pickups.len(),
            b.pickups.len(),
            "pickup population diverged at frame {frame}"
        );
    }
}

#[test]
fn dynamic_actor_churn_survives_ggrs_recreation() {
    let mut sim = rollback_sim();
    sim.spawn_boss_at(
        "ggrs_probe",
        "ggrs_probe",
        (640.0, 400.0),
        (24.0, 36.0),
        ambition::entity_catalog::placements::BossBrain::Dormant,
    );

    let rebased = sim.rollback_execution_stats().unwrap();
    assert_eq!(
        rebased.load_runs, 0,
        "setup discards pre-spawn rollback history"
    );
    assert_eq!(
        rebased.advance_runs, 0,
        "the fresh session has not advanced yet"
    );

    for frame in 0..24 {
        sim.step(scripted_action(frame));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }

    let stats = sim.rollback_execution_stats().unwrap();
    assert!(stats.load_runs > 0);
    assert!(
        sim.observation().enemies.iter().any(|enemy| enemy.alive),
        "the dynamically spawned actor remains represented after repeated rollback recreation"
    );
}

/// Assert every member of an authoritative entity family carries `Rollback`,
/// AND that the family was actually POPULATED when we looked.
///
/// The population check is the whole point. `Iterator::all` is vacuously true on
/// an empty iterator, so `family.all(is_rollback)` against a family with no
/// members is a green assertion that inspected nothing. This test used to sample
/// a freshly built sim in which no projectile had ever been fired, so the
/// projectile anchor "passed" without a single projectile existing — it would
/// have kept passing if `Rollback` were removed from projectiles entirely.
fn assert_family_anchored<'w, D, F>(
    world: &'w mut bevy::prelude::World,
    family: &str,
    query: &mut bevy::ecs::query::QueryState<Option<&'static Rollback>, F>,
) where
    F: bevy::ecs::query::QueryFilter,
    D: ?Sized,
{
    let markers: Vec<bool> = query.iter(world).map(|m| m.is_some()).collect();
    assert!(
        !markers.is_empty(),
        "no `{family}` entities existed when the anchor was checked, so the \
         anchor assertion would pass vacuously and prove nothing — populate the \
         family before asserting on it"
    );
    let unanchored = markers.iter().filter(|anchored| !**anchored).count();
    assert_eq!(
        unanchored,
        0,
        "{unanchored}/{} `{family}` entities are missing `Rollback`: GGRS will \
         not save or restore them, so a rewind silently keeps their post-rewind \
         state",
        markers.len()
    );
}

/// Every authoritative entity family is a GGRS anchor — checked against a
/// world where each family actually has members.
///
/// Projectiles only exist once something fires, so the sim is driven with the
/// scripted stream (which presses `projectile` every 13th frame) until the
/// family is non-empty, and the assertion runs on THAT world.
#[test]
fn authoritative_entity_families_are_ggrs_anchors() {
    let mut sim = rollback_sim();

    // A projectile only exists once one is FIRED, and the production spawner
    // (`fire_held_ranged_system`) fires the CONTROLLED SUBJECT's held item on
    // the attack press. Nothing in the sandbox start room hands the player a
    // ranged weapon, which is exactly why this family used to be empty — so
    // give the subject one and let the real system spawn through the real path.
    // Inserting the authored component, not a hand-built projectile: what is
    // under test is whether the PRODUCTION spawn path anchors what it spawns.
    {
        let world = sim.world_mut();
        let subject = world
            .resource::<ambition::platformer::markers::ControlledSubject>()
            .0
            .expect("the sandbox session has a controlled subject");
        world
            .entity_mut(subject)
            .insert(ambition::combat::held_items::HeldItem::new(
                ambition::characters::brain::HeldItemSpec {
                    id: "desync_canary_bolt_thrower".to_string(),
                    melee: None,
                    ranged: Some(
                        ambition::characters::brain::action_set::RangedActionSpec::bolt(400.0, 1),
                    ),
                    use_behavior: Default::default(),
                },
            ));
    }

    // Drive until a projectile is live, so the projectile family is real
    // rather than empty. Bodies/features/roots exist from setup; this loop is
    // for the one family that has to be created by play.
    let mut projectile_seen = false;
    for frame in 0..120 {
        sim.step(AgentAction {
            attack: true,
            ..scripted_action(frame)
        });
        let world = sim.world_mut();
        let mut live = world
            .query_filtered::<(), With<ambition::platformer::projectile::ProjectileGameplay>>();
        if live.iter(world).next().is_some() {
            projectile_seen = true;
            break;
        }
    }
    assert!(
        projectile_seen,
        "no projectile spawned in 120 frames of holding a ranged item and \
         pressing attack, so this test cannot check the projectile anchor at \
         all — fix the driver, do not delete the assertion"
    );

    let world = sim.world_mut();

    let mut bodies =
        world.query_filtered::<Option<&Rollback>, With<ambition::actors::actor::BodyKinematics>>();
    assert_family_anchored::<(), _>(world, "BodyKinematics", &mut bodies);

    let mut features = world.query_filtered::<Option<&Rollback>, With<ambition::platformer::lifecycle::FeatureSimEntity>>();
    assert_family_anchored::<(), _>(world, "FeatureSimEntity", &mut features);

    let mut projectiles = world.query_filtered::<Option<&Rollback>, With<ambition::platformer::projectile::ProjectileGameplay>>();
    assert_family_anchored::<(), _>(world, "ProjectileGameplay", &mut projectiles);

    let mut roots =
        world.query_filtered::<Option<&Rollback>, With<ambition::actors::rooms::RoomSet>>();
    assert_family_anchored::<(), _>(world, "RoomSet", &mut roots);
}

#[test]
fn registration_dump_and_fingerprint_are_stable() {
    let first = rollback_sim();
    let second = rollback_sim();
    let a = first.world().resource::<RollbackRegistry>();
    let b = second.world().resource::<RollbackRegistry>();
    assert_eq!(a.deterministic_dump(), b.deterministic_dump());
    assert_eq!(a.schema_fingerprint(), b.schema_fingerprint());
}

#[test]
fn ordinary_room_local_motion_does_not_invalidate_the_session_contract() {
    let mut sim = rollback_sim();
    let initial = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition::runtime::PreparedContentIdentity>();
        query
            .single(world)
            .copied()
            .expect("prepared identity exists")
    };

    for frame in 0..20 {
        sim.step(scripted_action(frame));
        sim.rollback_health().unwrap();
    }

    let observed = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition::runtime::PreparedContentIdentity>();
        query
            .single(world)
            .copied()
            .expect("prepared identity remains attached")
    };
    assert_eq!(
        initial, observed,
        "simulation frames never change the content epoch"
    );
}

/// **The mutable-state rewind canary.** Persistent, sim-mutated state that
/// CHANGES during the run must survive rewind + resimulation identically.
///
/// `SyncTestSession` is what makes this sharp: every harness step saves,
/// advances, rewinds, and resimulates the same frames with the same inputs, then
/// compares checksums. State the simulation writes but nothing registered for
/// rollback survives the rewind unchanged instead of being restored, so the
/// resimulated pass computes a different value and `rollback_health` reports the
/// mismatch.
///
/// The teeth are in the "actually changed" assertions. A rewind canary over
/// state that held its initial value for the whole run is vacuous: it would stay
/// green with the state entirely unregistered, because there was nothing to
/// restore. So this drives play that provably mutates dash charges and air jumps
/// (movement resources spent in the air and replenished on the ground), asserts
/// they moved, asserts rewinds actually happened, and only then treats sustained
/// checksum agreement as evidence.
#[test]
fn sim_mutated_state_that_changes_survives_rewind_identically() {
    let mut sim = rollback_sim();
    let start = sim.observation();

    let mut dash_changed = false;
    let mut air_jumps_changed = false;
    let mut position_changed = false;
    for frame in 0..90 {
        let observed = sim.step(scripted_action(frame));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: resimulation diverged: {error}"));

        dash_changed |= observed.dash_charges != start.dash_charges;
        air_jumps_changed |= observed.air_jumps != start.air_jumps;
        position_changed |= observed.player_pos != start.player_pos;
    }

    assert!(
        dash_changed,
        "dash charges never moved, so agreeing checksums prove nothing about \
         restoring them — this canary is only meaningful over state that changed"
    );
    assert!(
        air_jumps_changed,
        "air jumps never moved, so agreeing checksums prove nothing about \
         restoring them — this canary is only meaningful over state that changed"
    );
    assert!(
        position_changed,
        "the body never moved, so the scripted stream is not exercising the \
         simulation at all"
    );

    let stats = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed");
    assert!(
        stats.load_runs > 0,
        "no LoadWorld request was ever issued, so nothing was ever rewound and \
         the checksum agreement above is agreement with itself: {stats:?}"
    );
    assert!(
        stats.advance_runs > 90,
        "resimulation must execute more GGRS frames than the {} harness steps, \
         or the same frames were never replayed: {stats:?}",
        90
    );
}
