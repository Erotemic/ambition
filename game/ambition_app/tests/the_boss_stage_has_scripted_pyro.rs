//! The encore's stage jets use the shared boss and projectile contracts.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::{HitEvent, HitSource};
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults, ExtensionGeneration, ExtensionProfile};
use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::projectiles::{ProjectileSpawnRequest, ProjectilePresentation};
use bevy::prelude::{Entity, Messages};

const BEAT: SchemaKey = SchemaKey::new("ambition", "boss_stage_pyro.beat", 1);
const SOURCE: &str = include_str!("../../ambition_content/rhai/boss_stage_pyro.rhai");

fn stage(check_distance: Option<usize>, files: Vec<std::path::PathBuf>) -> (Platformer2dSimHarness, Entity, Entity) {
    let mut options = crate::common::fixed_60hz_room_options("hall_bossofthegame_stage")
        .with_extension_module_files(files);
    if let Some(distance) = check_distance { options = options.with_sync_test_rollback_settings(distance, 12); }
    let mut sim = Platformer2dSimHarness::new_with_options(options).unwrap();
    sim.step_n(AgentAction::default(), 160);
    let (boss, player) = {
        let world = sim.world_mut();
        let boss = world.query::<(Entity, &BossConfig)>().iter(world)
            .find(|(_, config)| config.behavior.id == "bossofthegame").unwrap().0;
        let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
        (boss, player)
    };
    // The west middle riser is at y=864 and includes x=480 in both stages.
    sim.teleport_player((480.0, 840.0));
    sim.world_mut().get_mut::<BodyHealth>(boss).unwrap().health.current = 15;
    sim.rebase_rollback_history().unwrap();
    (sim, boss, player)
}

fn pulses(sim: &Platformer2dSimHarness, boss: Entity) -> u32 {
    match sim.world().get::<BodyRecords>(boss).and_then(|r| r.get(&BEAT)).and_then(|r| r.get(FieldRef(1)).ok()) {
        Some(Value::U32(value)) => *value,
        _ => 0,
    }
}

fn shots(sim: &Platformer2dSimHarness, boss: Entity) -> Vec<&ProjectileSpawnRequest> {
    sim.world().resource::<Messages<ProjectileSpawnRequest>>()
        .iter_current_update_messages().filter(|shot| shot.owner == boss
            && matches!(&shot.presentation, ProjectilePresentation::OpenVisual(id) if id == "stage_pyro")).collect()
}

fn trace(rollback: bool) -> Vec<(u32, i32)> {
    let (mut sim, boss, player) = stage(Some(if rollback { 4 } else { 0 }), vec![]);
    sim.world_mut().insert_resource(ExtensionProfile::default());
    let mut trace = Vec::new();
    let mut landed = 0;
    let mut warned = false;
    let mut placed = false;
    let started = std::time::Instant::now();
    for _ in 0..680 {
        sim.step(AgentAction::default());
        let count = pulses(&sim, boss);
        let bolts = shots(&sim, boss).len();
        if let Some(beat) = sim.world().get::<BodyRecords>(boss).and_then(|r| r.get(&BEAT)) {
            if matches!(beat.get(FieldRef(0)), Ok(Value::F32(value)) if *value > 0.0)
                && matches!(beat.get(FieldRef(2)), Ok(Value::Bool(false))) {
                warned = true;
                assert_eq!(bolts, 0, "the warning creates no damaging projectile");
            }
        }
        if warned && !placed {
            // Place the recipient in the east outer jet after the entrance beats.
            sim.teleport_player((2080.0, 840.0));
            sim.rebase_rollback_history().unwrap();
            placed = true;
        }
        if count > 0 {
            landed += sim.world().resource::<Messages<HitEvent>>().iter_current_update_messages()
                .filter(|hit| hit.attacker == Some(boss) && hit.source == HitSource::Projectile && hit.damage == 1).count();
        }
        trace.push((count, sim.world().get::<BodyHealth>(player).unwrap().health.current));
    }
    let row = &sim.world().resource::<ExtensionProfile>().0["ambition::boss_stage_pyro/jets"];
    println!("PYRO_COST rollback={rollback} ticks=680 calls={} faults={} total_us={} backend_us={} session_us={}",
        row.calls, row.faults, row.total.as_micros(), row.backend.as_micros(), started.elapsed().as_micros());
    assert!(trace.iter().any(|(p, _)| *p >= 3), "the encore fires several pairs");
    assert!(warned, "the pattern warns before the jets fire");
    assert!(landed > 0, "a stage jet reaches the player through the native contact path");
    assert!(trace.windows(2).any(|p| p[1].0 > 0 && p[1].1 < p[0].1), "the real combat path applies jet damage");
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    trace
}

#[test]
fn the_encore_fires_real_jets_and_replays_the_same_damage() {
    assert_eq!(trace(false), trace(true));
}

#[test]
fn a_dead_boss_cannot_finish_its_warned_attack() {
    let (mut sim, boss, _) = stage(None, vec![]);
    for _ in 0..650 {
        sim.step(AgentAction::default());
        if sim.world().get::<BodyRecords>(boss).and_then(|r| r.get(&BEAT)).is_some() {
            sim.world_mut().get_mut::<BodyHealth>(boss).unwrap().health.current = 0;
            for _ in 0..120 {
                sim.step(AgentAction::default());
                assert!(shots(&sim, boss).is_empty());
            }
            return;
        }
    }
    panic!("the encore did not warn of its jets");
}

#[test]
fn the_prebuilt_session_accepts_a_script_edit_and_keeps_bad_code_out() {
    let dir = std::env::temp_dir().join(format!("ambition_pyro_edit_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("boss_stage_pyro.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let (mut sim, boss, _) = stage(None, vec![file.clone()]);
    let before = sim.world().resource::<ExtensionGeneration>().clone();
    std::fs::write(&file, "fn declare() { bad source").unwrap();
    sim.step_n(AgentAction::default(), 40);
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), before);
    let started = std::time::Instant::now();
    std::fs::write(&file, SOURCE.replace("const JET_SPEED = 720.0;", "const JET_SPEED = 500.0;")).unwrap();
    sim.step_n(AgentAction::default(), 40);
    assert_ne!(*sim.world().resource::<ExtensionGeneration>(), before);
    for _ in 0..650 {
        sim.step(AgentAction::default());
        let bolts = shots(&sim, boss);
        if !bolts.is_empty() {
            assert_eq!(bolts.len(), 2);
            assert!(bolts.iter().all(|b| (b.projectile.body.kin.vel.length() - 500.0).abs() < 0.01));
            println!("PYRO_EDIT edited_source_to_live_jets_us={}", started.elapsed().as_micros());
            std::fs::remove_file(file).unwrap();
            return;
        }
    }
    panic!("the edited encore fired no jets");
}
