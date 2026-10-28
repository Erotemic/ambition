//! Body motion uses one integration phase for home, actor, and boss bodies.
//! These tests drive the real app and the shared body entry.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::{Entity, World};

const ENEMY_ID: &str = "unified_move_enemy";

fn player_x(world: &mut World) -> f32 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos.x
}

fn enemy_entity(world: &mut World) -> Entity {
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world)
        .find(|(_, f)| f.as_str() == ENEMY_ID)
        .map(|(e, _)| e)
        .expect("spawned enemy present")
}

/// Drive a home body (via player input) and a chasing actor body in the same
/// frames and observe BOTH integrating. Since the ONLY scheduled body-movement
/// system is `integrate_sim_bodies`, both bodies moving proves they pass through
/// the SAME integration phase — not two separate routes.
#[test]
fn home_body_and_actor_body_move_through_the_same_integration_phase() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    // Drop the enemy to the player's RIGHT — a chasing brain is drawn toward it.
    let px = player_x(sim.world_mut());
    let p = {
        let mut q = sim
            .world_mut()
            .query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        q.single(sim.world_mut()).expect("primary player").pos
    };
    // ⛔ SPAWN BY CHARACTER, NOT BY ARCHETYPE. A `Custom(..)` row that no longer
    // exists falls back to a generic `combatant`, so the fixture keeps passing
    // while asserting on a body that is not the one it names.

    sim.spawn_enemy_character_at(
        ENEMY_ID,
        "Perfect Cellular Automaton",
        (p.x + 160.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );
    let enemy = enemy_entity(sim.world_mut());
    let enemy_x_before = sim.world_mut().get::<BodyKinematics>(enemy).unwrap().pos.x;

    // Drive the HOME body RIGHT (toward the enemy) while the enemy engages it.
    // Track the actor's CUMULATIVE travel, not its net endpoint: the duel dance
    // (engage 78 / too-close 30, hitstun, knockback) nets close to zero when the
    // player charges into it, and every combat-cadence change re-rolls that net.
    // What this test pins is that the body MOVES through the shared integration
    // phase at all — total travel is the cadence-proof form of that fact.
    let mut enemy_travel = 0.0_f32;
    let mut enemy_x_prev = enemy_x_before;
    for _ in 0..40 {
        sim.step(AgentAction::move_x(1.0));
        let x = sim.world_mut().get::<BodyKinematics>(enemy).unwrap().pos.x;
        enemy_travel += (x - enemy_x_prev).abs();
        enemy_x_prev = x;
    }
    let player_x_after = player_x(sim.world_mut());
    let enemy_x_after = enemy_x_prev;

    assert!(
        player_x_after > px + 5.0,
        "the HOME body integrated its rightward input intent: x {px} -> {player_x_after}",
    );
    // The actor body integrated its brain's locomotion through the SAME phase — proven
    // by MATERIAL horizontal travel (gravity is vertical, so an x-shift can only come
    // from the brain's chase/footsies intent flowing through `integrate_sim_bodies`).
    // Cumulative travel, not net endpoint: the net form failed twice as combat cadence
    // evolved (§A7 moveset folds; the same-frame EFFECTS consumption reorder), while
    // the fact the test pins — the body moves through the shared phase at all — never
    // wavered (duel_arena covers the fight itself).
    assert!(
        enemy_travel > 5.0,
        "the ACTOR body integrated its chase intent in the SAME phase: \
         x {enemy_x_before} -> {enemy_x_after}, total travel {enemy_travel}",
    );
}

/// Real participant input reaches the driven body through the canonical slot
/// publication path and becomes body-facing `ActorControl`. Neutral slot input
/// leaves the body at rest; driving the slot moves it.
#[test]
fn brain_player_uses_the_slot_owned_control_path() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    sim.step(AgentAction::default());
    let x_before = player_x(sim.world_mut());

    for _ in 0..20 {
        sim.step(AgentAction::default());
    }
    let x_after_neutral = player_x(sim.world_mut());
    assert!(
        (x_after_neutral - x_before).abs() < 5.0,
        "neutral participant input must not manufacture horizontal body intent: \
         x {x_before} -> {x_after_neutral}",
    );

    for _ in 0..20 {
        sim.step(AgentAction::move_x(1.0));
    }
    let x_after_slot = player_x(sim.world_mut());
    assert!(
        x_after_slot > x_after_neutral + 5.0,
        "driving the canonical slot input moves the home body right: \
         {x_after_neutral} -> {x_after_slot}",
    );
}

#[test]
fn a_boss_body_uses_the_common_integration_step_and_its_own_envelope() {
    use ambition_platformer2d::characters::control::ActorControl;
    use ambition_platformer2d::combat::BodyEnvelope;
    use ambition_platformer2d::engine_core::WorldVec2;
    use ambition_platformer2d::entity_catalog::placements::BossBrain;
    use bevy::ecs::system::RunSystemOnce;
    let mut sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("switch_lab"),
    )
    .expect("the simulation builds");
    let at = {
        let world = sim.world_mut();
        world
            .query_filtered::<&BodyKinematics, PrimaryPlayerOnly>()
            .single(world)
            .unwrap()
            .pos
            + ambition_platformer2d::engine_core::Vec2::new(300.0, -150.0)
    };
    sim.spawn_boss_at(
        "common_body_boss",
        "clockwork_warden",
        (at.x, at.y),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "clockwork_warden".into(),
        },
    );
    let body = {
        let world = sim.world_mut();
        world
            .query::<(Entity, &FeatureId)>()
            .iter(world)
            .find(|(_, id)| id.as_str() == "common_body_boss")
            .unwrap()
            .0
    };
    let envelope = ambition_platformer2d::engine_core::Vec2::new(80.0, 120.0);
    sim.world_mut()
        .entity_mut(body)
        .insert(BodyEnvelope(envelope));
    sim.world_mut()
        .get_mut::<ActorControl>(body)
        .unwrap()
        .0
        .velocity_target = WorldVec2::new(120.0, 0.0);
    let before = sim.world().get::<BodyKinematics>(body).unwrap().pos;
    sim.world_mut()
        .run_system_once(ambition_platformer2d::actors::body::integrate_sim_bodies)
        .expect("the common body step runs");
    let after = sim.world().get::<BodyKinematics>(body).unwrap().pos;
    assert!(
        after.x > before.x,
        "the common body step did not move the boss: {before:?} -> {after:?}"
    );
    let bounds = sim
        .world()
        .get::<ambition_platformer2d::combat::components::CenteredAabb>(body)
        .unwrap();
    assert_eq!(bounds.center, after);
    assert_eq!(bounds.size(), envelope);

    // Free flight in clear space advances by velocity times one simulation tick.
    let mut travel = 0.0;
    for _ in 0..30 {
        let before = sim.world().get::<BodyKinematics>(body).unwrap().pos;
        sim.step(AgentAction::default());
        let kin = sim.world().get::<BodyKinematics>(body).unwrap();
        let dt = sim
            .world()
            .resource::<ambition_platformer2d::time::WorldTime>()
            .sim_dt();
        let displacement = kin.pos - before;
        assert!((displacement - kin.vel * dt).length() < 0.001,
            "the free-flight body did not advance by one tick: displacement={displacement:?}, velocity={:?}, dt={dt}", kin.vel);
        travel += displacement.length();
    }
    assert!(
        travel > 0.0,
        "the boss remained still throughout the schedule check"
    );
}
