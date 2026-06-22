//! Boss lifecycle CONTRACT pins — the headless safety net for Stage R3 of the
//! boss entity-local refactor (`docs/planning/boss-entity-local-refactor.md`).
//!
//! R3 flips boss HP/phase authority off the global `BossEncounterRegistry` onto
//! the entity and deletes the live map. The *consequences* of a boss dying —
//! the save records it Cleared, its reward chest drops, and the adaptive-music
//! request restores to room music — have no other headless coverage and can't
//! be eyeballed in CI. These tests pin that observable contract against the
//! CURRENT (registry) implementation so R3 must preserve it.
//!
//! Stable parts: the ASSERTIONS (music set during the fight; save Cleared +
//! reward chest + music cleared after death). Authority-coupled part: the
//! `force_kill_boss` helper drives the current registry authority — R3 repoints
//! it to the entity (`BossStatus.health`/`encounter.phase`), and the assertions
//! must still pass. The cut-rope victory NPC + in-place replay are content-
//! specific + headless-hard (R5 rewrites cut-rope as an EncounterScript); they
//! remain an explicit in-game verification item, but cut-rope's death
//! consequences share the generic entity-death path pinned here.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use ambition_gameplay_core::actor::BossBrain;
use ambition_gameplay_core::boss_encounter::BossEncounterRegistry;
use ambition_gameplay_core::encounter::BossEncounterMusicRequest;
use ambition_gameplay_core::features::BossRewardChest;
use ambition_gameplay_core::persistence::save::SandboxSave;
use ambition_gameplay_core::persistence::save_data::PersistedEncounterState;
use ambition_gameplay_core::player::{BodyKinematics, PrimaryPlayerOnly};
use bevy::prelude::World;

const MOCKINGBIRD_TRACK: &str = "how_to_kill_a_mockingbird";

fn player_pos(world: &mut World) -> (f32, f32) {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    let kin = q.single(world).expect("primary player exists");
    (kin.pos.x, kin.pos.y)
}

fn spawn_mockingbird(sim: &mut SandboxSim, runtime_id: &str) {
    let (px, py) = player_pos(sim.world_mut());
    sim.spawn_boss_at(
        runtime_id,
        "mockingbird",
        (px, py),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
}

/// Drive the boss to death. **R3: repoint this to mutate the boss ENTITY
/// (`BossStatus.health` = 0 + `encounter.phase = Death`) once the global map is
/// deleted.** The assertions in each test are the stable contract.
fn force_kill_boss(sim: &mut SandboxSim, runtime_id: &str) {
    let mut reg = sim.world_mut().resource_mut::<BossEncounterRegistry>();
    let state = reg
        .encounters
        .get_mut(runtime_id)
        .expect("boss is registered before being killed");
    let _ = state.force_death();
}

fn music_track(sim: &SandboxSim) -> Option<String> {
    sim.world()
        .resource::<BossEncounterMusicRequest>()
        .desired_track
        .clone()
}

fn boss_cleared(sim: &SandboxSim, archetype_id: &str) -> bool {
    matches!(
        sim.world().resource::<SandboxSave>().data().boss(archetype_id),
        PersistedEncounterState::Cleared
    )
}

fn boss_reward_chest_count(world: &mut World) -> usize {
    world.query::<&BossRewardChest>().iter(world).count()
}

/// While a boss is alive and woken, the adaptive-music request carries its
/// track. (R3 must keep boss music playing through the fight.)
#[test]
fn boss_music_plays_during_the_fight() {
    let mut sim = SandboxSim::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    spawn_mockingbird(&mut sim, "music_boss");

    // A few frames to wake the boss (Dormant → Intro) + publish its music.
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }

    assert_eq!(
        music_track(&sim).as_deref(),
        Some(MOCKINGBIRD_TRACK),
        "a woken boss's encounter requests its own music track"
    );
}

/// A defeated boss leaves a Cleared save record, drops its reward chest, and
/// releases the adaptive-music request back to room music. This is the generic
/// entity-death CONTRACT R3 must preserve (cut-rope's environmental kill flows
/// through the same death path).
#[test]
fn defeated_boss_is_recorded_cleared_drops_reward_and_clears_music() {
    let mut sim = SandboxSim::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    spawn_mockingbird(&mut sim, "dying_boss");

    // Wake + register the boss, then confirm the fight music is up.
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }
    assert_eq!(
        music_track(&sim).as_deref(),
        Some(MOCKINGBIRD_TRACK),
        "precondition: the boss fight music is playing before the kill"
    );
    assert!(
        !boss_cleared(&sim, "mockingbird"),
        "precondition: the boss is not pre-marked cleared"
    );

    force_kill_boss(&mut sim, "dying_boss");

    // Step past the death outro (mockingbird `death_seconds` = 2.2s ≈ 132
    // frames) so the death resolves: save Cleared + reward-chest sync + music
    // lifetime restore all run.
    for _ in 0..200 {
        sim.step(AgentAction::default());
    }

    assert!(
        boss_cleared(&sim, "mockingbird"),
        "a defeated boss must be recorded Cleared in the save"
    );
    assert_eq!(
        boss_reward_chest_count(sim.world_mut()),
        1,
        "a defeated boss with a DropChest reward must drop exactly one chest"
    );
    assert_eq!(
        music_track(&sim),
        None,
        "after the fight the boss-music request clears (room music resumes)"
    );
}
