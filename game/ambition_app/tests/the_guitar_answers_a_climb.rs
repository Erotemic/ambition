//! Scripted feedback reads the mount's surfaces and enters ordinary combat.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{boss_encounter::BossConfig, characters::actor::{BodyHealth, CharacterForm}, engine_core as ae};
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults, ExtensionProfile};
use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
use ambition_platformer2d::{mount::BackLedge, platformer::markers::PrimaryPlayerOnly, sim_view::CombatGeometryView};
use bevy::prelude::*;

const CHARGE: SchemaKey = SchemaKey::new("ambition", "boss_stage_feedback.charge", 1);

fn phase(sim: &Platformer2dSimHarness, boss: Entity) -> u32 {
    match sim.world().get::<BodyRecords>(boss).and_then(|r| r.get(&CHARGE)).and_then(|r| r.get(FieldRef(0)).ok()) {
        Some(Value::U32(value)) => *value,
        _ => 0,
    }
}

fn stage(rollback: bool) -> (Platformer2dSimHarness, Entity, Entity, ae::Vec2) {
    let options = crate::common::fixed_60hz_room_options("hall_bossofthegame_stage")
        .with_sync_test_rollback_settings(if rollback { 4 } else { 0 }, 12);
    let mut sim = Platformer2dSimHarness::new_with_options(options).unwrap();
    sim.step_n(AgentAction::default(), 600);
    let world = sim.world_mut();
    let boss = world.query::<(Entity, &BossConfig)>().iter(world)
        .find(|(_, config)| config.behavior.id == "bossofthegame").unwrap().0;
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    let slope = world.query::<(&CharacterForm, &ae::BodyKinematics, &BackLedge, Has<ae::Unmirrored>)>()
        .iter(world).find(|(form, ..)| form.id() == "npc_full_stack")
        .and_then(|(_, kin, back, unmirrored)| back.slope_line(kin, unmirrored)).unwrap();
    let half = world.get::<ae::BodyKinematics>(player).unwrap().size().y * 0.5;
    let on_guitar = (slope[1] + slope[2]) * 0.5 - ae::Vec2::new(0.0, half);
    world.get_mut::<BodyHealth>(player).unwrap().health.current = 999;
    world.get_mut::<BodyHealth>(player).unwrap().health.max = 999;
    world.insert_resource(ExtensionProfile::default());
    sim.teleport_player(on_guitar.into());
    sim.rebase_rollback_history().unwrap();
    (sim, boss, player, on_guitar)
}

fn trace(rollback: bool) -> Vec<u32> {
    let (mut sim, boss, player, _) = stage(rollback);
    let mut trace = Vec::new();
    let mut warned = false;
    let mut struck = false;
    for _ in 0..180 {
        sim.step(AgentAction { move_x: -1.0, ..Default::default() });
        let phase = phase(&sim, boss);
        let geometry = sim.world().resource::<CombatGeometryView>();
        let rays: Vec<_> = geometry.strikes.iter().filter(|strike| strike.owner == boss
            && strike.ray.as_ref().is_some_and(|ray| ray.look.as_ref() == "force")).collect();
        if phase == 2 && trace.last() != Some(&2) {
            println!("feedback recipient {:?}, rays {:?}", sim.world().get::<ae::BodyKinematics>(player).unwrap().pos, rays);
        }
        if phase == 1 {
            warned = true;
            assert!(rays.is_empty(), "the warning carries no damaging ray");
        }
        struck |= rays.iter().any(|ray| ray.hit.contains(&player));
        trace.push(phase);
    }
    assert!(warned && trace.contains(&2) && trace.contains(&3), "the climb starts a warning, strike, and recovery: {trace:?}");
    assert!(struck, "the shared ray resolver confirms a real hit on the guitar");
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    let row = &sim.world().resource::<ExtensionProfile>().0["ambition::boss_stage_feedback/feedback"];
    println!("FEEDBACK_COST rollback={rollback} calls={} total_us={} backend_us={}", row.calls, row.total.as_micros(), row.backend.as_micros());
    trace
}

#[test]
fn a_climb_warns_then_strikes_through_shared_rays_and_replays() {
    assert_eq!(trace(false), trace(true));
}

#[test]
fn leaving_the_guitar_avoids_the_pulse_and_a_dead_boss_cancels_it() {
    let (mut sim, boss, player, on_guitar) = stage(false);
    for _ in 0..60 {
        sim.step(AgentAction::default());
        if phase(&sim, boss) == 1 { break; }
    }
    assert_eq!(phase(&sim, boss), 1);
    sim.teleport_player((176.0, 1064.0));
    sim.rebase_rollback_history().unwrap();
    for _ in 0..120 {
        sim.step(AgentAction::default());
        assert!(!sim.world().resource::<CombatGeometryView>().strikes.iter()
            .any(|strike| strike.ray.is_some() && strike.hit.contains(&player)));
    }
    sim.step_n(AgentAction::default(), 250);
    sim.teleport_player(on_guitar.into());
    sim.rebase_rollback_history().unwrap();
    for _ in 0..60 {
        sim.step(AgentAction::default());
        if phase(&sim, boss) == 1 { break; }
    }
    assert_eq!(phase(&sim, boss), 1);
    sim.world_mut().get_mut::<BodyHealth>(boss).unwrap().health.current = 0;
    sim.rebase_rollback_history().unwrap();
    for _ in 0..100 {
        sim.step(AgentAction::default());
        assert!(!sim.world().resource::<CombatGeometryView>().strikes.iter()
            .any(|strike| strike.owner == boss && strike.ray.is_some()));
    }
}
