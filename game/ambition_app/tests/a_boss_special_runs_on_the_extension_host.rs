//! The Mockingbird's echo fan runs as a procedural module on the extension
//! host (fast-iteration I4), in the assembled game, from a real brain press.
//!
//! The road under test, with nothing injected: the boss pattern presses
//! `Special("echo_fan")` → the boss domain's trigger adapter → the host runs
//! `ambition_content_modules::echo_fan` → the projectile domain's request
//! adapter → `ProjectileSpawnRequest` → seven shots in flight.
//!
//! The second arm runs the same fight under a GGRS sync-test session. A
//! rollback resimulates the strike; if the module's strike latch were not
//! rollback state, the replay would fire a different number of shots and the
//! checksum would disagree.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::BossBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::projectiles::entity::ProjectileOwner;
use bevy::prelude::{Entity, World};

const BOSS: &str = "echo_fan_boss";
const FAN: usize = 7;
/// Long enough for the Mockingbird's attack cycle to reach the echo fan twice.
/// Measured: 900 ticks reach it once.
const TWO_STRIKES: usize = 1800;
/// Long enough to reach it once under the slower sync-test harness.
const ONE_STRIKE: usize = 900;

fn spawn_mockingbird(sim: &mut Platformer2dSimHarness) -> Entity {
    let (px, py) = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        let kin = q.single(world).expect("primary player exists");
        (kin.pos.x, kin.pos.y)
    };
    sim.spawn_boss_at(
        BOSS,
        "mockingbird",
        (px + 120.0, py - 60.0),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world)
        .find(|(_, f)| f.as_str() == BOSS)
        .map(|(e, _)| e)
        .expect("the spawned boss is present")
}

/// How many shots the boss owns that fly at the echo fan's speed. A count,
/// not a set of entities: a rollback may rebuild a shot under a new entity.
fn fan_shots(world: &mut World, boss: Entity) -> usize {
    let mut q = world.query::<(&ProjectileOwner, &BodyKinematics)>();
    q.iter(world)
        .filter(|(owner, kin)| owner.0 == boss && (kin.vel.length() - 300.0).abs() < 1.0)
        .count()
}

/// The size of each rise in the boss's echo-fan shot count, in tick order.
/// A fan lives 2 s and the boss's cycle reaches it every ~7 s, so two fans
/// never overlap.
fn fight(sim: &mut Platformer2dSimHarness, ticks: usize, mut each_tick: impl FnMut(&Platformer2dSimHarness)) -> Vec<usize> {
    let boss = spawn_mockingbird(sim);
    let mut seen = fan_shots(sim.world_mut(), boss);
    let mut bursts = Vec::new();
    for _ in 0..ticks {
        sim.step(AgentAction::default());
        each_tick(sim);
        let now = fan_shots(sim.world_mut(), boss);
        if now > seen {
            bursts.push(now - seen);
        }
        seen = now;
    }
    bursts
}

#[test]
fn the_mockingbird_fires_one_fan_per_strike_through_the_extension_host() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let bursts = fight(&mut sim, TWO_STRIKES, |_| {});
    assert!(
        bursts.len() >= 2,
        "the boss reached its echo fan {} times in {TWO_STRIKES} ticks; the arm needs two \
         strikes to show that one strike fires once: {bursts:?}",
        bursts.len()
    );
    assert!(
        bursts.iter().all(|&n| n == FAN),
        "each strike fires exactly one fan of {FAN}: {bursts:?}"
    );
}

#[test]
fn a_rollback_replays_the_fan_and_its_strike_latch() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the GGRS sync-test harness builds");
    let mut first_error: Option<String> = None;
    let bursts = fight(&mut sim, ONE_STRIKE, |sim| {
        if first_error.is_none() {
            first_error = ambition_platformer2d::rollback::session_health(sim.world()).err();
        }
    });
    // ⭐ The premise: a sync test that never saw a fan certified nothing.
    assert!(
        !bursts.is_empty(),
        "no echo fan fired under the sync test, so no rollback replayed one"
    );
    assert!(
        bursts.iter().all(|&n| n == FAN),
        "each strike fires exactly one fan of {FAN} under rollback: {bursts:?}"
    );
    assert_eq!(first_error, None, "the sync-test session stayed healthy");
}

/// ⭐ THE NO-RELINK ROAD, IN THE ASSEMBLED GAME. The module crate is built for
/// `wasm32-unknown-unknown` (only the module and its port values compile), the
/// same game binary loads it, and the echo fan fires from the file: the
/// admitted entry is the loaded one, and the fight still fires one fan of
/// seven per strike.
#[test]
fn a_module_rebuilt_as_wasm_replaces_the_linked_one_in_the_same_game() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm = ambition_platformer2d::extension::build_module_crate(&root, "ambition_content_modules")
        .expect("the module crate builds for wasm32-unknown-unknown");
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_extension_module_files(vec![wasm]),
    )
    .expect("the sandbox builds with the loaded module");
    let admitted = sim
        .world()
        .resource::<ambition_platformer2d::extension::AdmittedExtensions>()
        .0
        .clone();
    let fan = admitted
        .entries
        .iter()
        .find(|e| e.path == "ambition::echo_fan/fire")
        .expect("the echo fan is admitted");
    assert!(
        matches!(fan.runner, ambition_platformer2d::extension::EntryRunner::Loaded { .. }),
        "the loaded build replaced the linked one: {:?}",
        fan.runner
    );
    assert_eq!(admitted.replaced.len(), 1, "{:?}", admitted.replaced);
    let bursts = fight(&mut sim, TWO_STRIKES, |_| {});
    assert!(bursts.len() >= 2, "{bursts:?}");
    assert!(bursts.iter().all(|&n| n == FAN), "{bursts:?}");
}
