//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::boss_encounter::BossEncounterPhase;
use crate::encounter::EncounterMusicRequest;
use crate::features::ecs::boss_clusters::test_support::{test_boss_config, test_boss_status};
use crate::features::ecs::boss_clusters::BossEncounter;
use crate::features::GameplayBanner;
use ambition_encounter::{
    EncounterBeat, EncounterEffect, EncounterGate, EncounterParticipant, EncounterParticipants,
    EncounterRole, EncounterScript, EncounterTrigger,
};
use ambition_time::WorldTime;

fn member(hp: i32) -> (BossEncounter, ambition_characters::actor::BodyHealth) {
    test_boss_status(hp, BossEncounterPhase::Phase1)
}

fn member_health(
    app: &App,
    boss: bevy::prelude::Entity,
) -> &ambition_characters::actor::BodyHealth {
    app.world().entity(boss).get().unwrap()
}

fn test_app() -> App {
    let mut app = App::new();
    app.insert_resource(WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.add_message::<EncounterGate>();
    app.init_resource::<GameplayBanner>();
    ambition_platformer_primitives::lifecycle::insert_session_world_component(
        app.world_mut(),
        <EncounterMusicRequest>::default(),
    );
    app.add_systems(Update, tick_encounter_scripts);
    app
}

/// A gate-triggered ForceKill beat kills the named member when the gate fires.
#[test]
fn gate_beat_force_kills_its_member() {
    let mut app = test_app();
    let boss = app.world_mut().spawn(member(9999)).id();
    app.world_mut().spawn((
        EncounterParticipants::new(vec![EncounterParticipant::adopted(
            "cut_rope_boss",
            boss,
            EncounterRole::PrimaryTarget,
        )]),
        EncounterScript::new(vec![EncounterBeat::new(
            EncounterTrigger::Gate("impact".into()),
            vec![EncounterEffect::ForceKill(0)],
        )]),
    ));

    // No gate yet → the boss lives.
    app.update();
    assert!(member_health(&app, boss).alive());

    // Fire the gate → the script force-kills the member.
    app.world_mut().write_message(EncounterGate::new("impact"));
    app.update();
    let status = app.world().entity(boss).get::<BossEncounter>().unwrap();
    assert!(!member_health(&app, boss).alive());
    assert_eq!(member_health(&app, boss).current(), 0);
    assert_eq!(
        status.encounter.as_ref().unwrap().phase,
        BossEncounterPhase::Death
    );
}

/// Beats advance one per fired trigger; a Timer beat fires after its delay,
/// and effects (Banner) apply.
#[test]
fn beats_advance_in_order_with_timer_and_banner() {
    let mut app = test_app();
    let boss = app.world_mut().spawn(member(10)).id();
    app.world_mut().spawn((
        EncounterParticipants::new(vec![EncounterParticipant::adopted(
            "enc_boss",
            boss,
            EncounterRole::PrimaryTarget,
        )]),
        EncounterScript::new(vec![
            EncounterBeat::new(
                EncounterTrigger::Gate("go".into()),
                vec![EncounterEffect::Banner {
                    text: "BEAT 1".into(),
                    secs: 1.0,
                }],
            ),
            EncounterBeat::new(
                EncounterTrigger::Timer(0.1),
                vec![EncounterEffect::ForceKill(0)],
            ),
        ]),
    ));

    // Fire the first gate → beat 0 applies, cursor advances to beat 1.
    app.world_mut().write_message(EncounterGate::new("go"));
    app.update();
    {
        let mut q = app.world_mut().query::<&EncounterScript>();
        assert_eq!(q.single(app.world()).unwrap().cursor(), 1);
    }
    assert!(member_health(&app, boss).alive());

    // Tick past the 0.1s timer (1/60 per frame) → beat 1 fires the kill.
    for _ in 0..10 {
        app.update();
    }
    assert!(!member_health(&app, boss).alive());
    let mut q = app.world_mut().query::<&EncounterScript>();
    assert!(q.single(app.world()).unwrap().done());
}

fn boss_config() -> crate::features::ecs::boss_clusters::BossConfig {
    test_boss_config("b", "B", "mockingbird")
}

/// A `CommandedMove` steers the boss's control toward the target's x.
#[test]
fn commanded_move_steers_the_boss_toward_target() {
    let mut app = App::new();
    app.add_systems(Update, tick_commanded_moves);
    let boss = app
        .world_mut()
        .spawn((
            crate::features::BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::splat(40.0),
                facing: -1.0,
            },
            boss_config(),
            member(100),
            ambition_characters::brain::ActorControl::default(),
            ambition_characters::brain::BossAttackState::default(),
            ambition_characters::brain::BossAttackIntent::default(),
            CommandedMove {
                target: ae::Vec2::new(300.0, 0.0),
                speed: 150.0,
                arrive_tolerance: 10.0,
            },
        ))
        .id();

    app.update();

    let control = app
        .world()
        .entity(boss)
        .get::<ambition_characters::brain::ActorControl>()
        .unwrap();
    assert!(
        control.0.velocity_target.x > 0.0,
        "the boss is lured toward the +x target"
    );
    assert_eq!(control.0.facing, 1.0, "and faces the target");
}

/// An aligned `FallingHazard` falls onto its target and fires its impact gate.
#[test]
fn falling_hazard_drops_when_aligned_and_fires_impact_gate() {
    let mut app = App::new();
    app.insert_resource(WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    ambition_platformer_primitives::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_engine_core::RoomGeometry(ae::World::new(
            "t",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(50.0, 50.0),
            Vec::new(),
        )),
    );
    app.add_message::<EncounterGate>();
    app.add_systems(Update, tick_falling_hazards);

    // Target sits directly below the hazard anchor (aligned in x).
    let target = app
        .world_mut()
        .spawn(CenteredAabb::from_center_size(
            ae::Vec2::new(500.0, 500.0),
            ae::Vec2::splat(40.0),
        ))
        .id();
    app.world_mut().spawn((
        CenteredAabb::from_center_size(ae::Vec2::new(500.0, 100.0), ae::Vec2::splat(60.0)),
        FallingHazard {
            size: ae::Vec2::splat(60.0),
            gravity: 1400.0,
            terminal: 920.0,
            align_tolerance: 50.0,
            target,
            impact_gate: "boom".into(),
            vel_y: 0.0,
            dropping: false,
        },
    ));

    let mut fired = false;
    for _ in 0..180 {
        app.update();
        let msgs = app
            .world()
            .resource::<bevy::ecs::message::Messages<EncounterGate>>();
        if msgs
            .iter_current_update_messages()
            .any(|g| g.gate == "boom")
        {
            fired = true;
            break;
        }
    }
    assert!(
        fired,
        "an aligned hazard falls onto its target and fires its impact gate"
    );
    // The hazard despawns on impact.
    let mut q = app.world_mut().query::<&FallingHazard>();
    assert_eq!(q.iter(app.world()).count(), 0, "hazard retires on impact");
}
