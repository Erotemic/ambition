//! Body recovery and freeze expiry through the real simulation schedule.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::actor::BodyCombat;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::{BodyKinematics, Vec2};
use ambition_platformer2d::entity_catalog::placements::{BossBrain, CharacterBrain};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::time::{ClockState, ProperTimeScale, WorldTime};
use bevy::prelude::Entity;

#[test]
fn a_driven_actors_freeze_expires_for_every_body_and_proper_recovery_resumes() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_fixed_tick(true)
            .with_required_start_room("switch_lab"),
    )
    .expect("the real simulation builds");
    let home = {
        let world = sim.world_mut();
        world
            .query_filtered::<Entity, PrimaryPlayerOnly>()
            .single(world)
            .unwrap()
    };
    let at = sim.world().get::<BodyKinematics>(home).unwrap().pos;
    sim.spawn_enemy_character_at(
        "reaction_actor",
        "Reaction Actor",
        (at.x + 120.0, at.y),
        (14.0, 23.0),
        CharacterBrain::Passive,
        "perfect_cellular_automaton",
    );
    let boss_at = at + Vec2::new(300.0, -150.0);
    sim.spawn_boss_at(
        "reaction_boss",
        "clockwork_warden",
        (boss_at.x, boss_at.y),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "clockwork_warden".into(),
        },
    );
    let [actor, boss] = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &FeatureId)>();
        ["reaction_actor", "reaction_boss"].map(|id| {
            q.iter(world)
                .find(|(_, feature)| feature.as_str() == id)
                .unwrap()
                .0
        })
    };
    let bodies = [(home, 0.5), (actor, 2.0), (boss, 0.25)];
    for (body, rate) in bodies {
        sim.world_mut()
            .entity_mut(body)
            .insert(ProperTimeScale(rate));
        let mut combat = sim.world_mut().get_mut::<BodyCombat>(body).unwrap();
        combat.hitstop_timer = 0.05;
        combat.hitstun_timer = 0.3;
        combat.damage_invuln_timer = 0.3;
    }
    // The participant drives the actor while its home body has no hitstop.
    sim.world_mut()
        .entity_mut(home)
        .remove::<ambition_platformer2d::characters::control::DrivingParticipant>();
    sim.world_mut().entity_mut(actor).insert(
        ambition_platformer2d::characters::control::DrivingParticipant(
            ambition_platformer2d::characters::control::PlayerSlot::PRIMARY,
        ),
    );
    sim.world_mut()
        .get_mut::<BodyCombat>(home)
        .unwrap()
        .hitstop_timer = 0.0;
    // Start with an active freeze and let the normal schedule end it.
    sim.world_mut().resource_mut::<ClockState>().time_scale = 0.0;
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::time::time_control::RequestedClockScale>()
        .sim_clock = 0.0;
    let mut frozen_steps = 0;
    for _ in 0..20 {
        if sim.world().get::<BodyCombat>(actor).unwrap().hitstop_timer == 0.0 {
            break;
        }
        sim.step(AgentAction::default());
        assert_eq!(sim.world().resource::<WorldTime>().sim_dt(), 0.0);
        frozen_steps += 1;
        let remaining = sim.world().get::<BodyCombat>(actor).unwrap().hitstop_timer;
        for (body, _) in bodies {
            let combat = sim.world().get::<BodyCombat>(body).unwrap();
            assert_eq!(combat.hitstun_timer, 0.3);
            assert_eq!(combat.damage_invuln_timer, 0.3);
            assert_eq!(
                combat.hitstop_timer,
                if body == home { 0.0 } else { remaining },
                "body {body:?} uses a different freeze clock"
            );
        }
    }
    assert!(frozen_steps > 0, "the fixture did not freeze the world");
    assert_eq!(
        sim.world().get::<BodyCombat>(actor).unwrap().hitstop_timer,
        0.0,
        "the global freeze stopped its own expiry"
    );
    sim.step(AgentAction::default());
    let dt = sim.world().resource::<WorldTime>().sim_dt();
    assert!(dt > 0.0, "the world did not resume after hitstop expired");
    for (body, rate) in bodies {
        let combat = sim.world().get::<BodyCombat>(body).unwrap();
        assert!((combat.hitstun_timer - (0.3 - dt * rate)).abs() < 1e-6);
        assert!((combat.damage_invuln_timer - (0.3 - dt * rate)).abs() < 1e-6);
        assert_eq!(combat.hitstop_timer, 0.0);
    }
}
