//! Run the promotion input without trace serialization. One process measures
//! one backend so Linux memory observations have a separate process lifetime.

use std::path::{Path, PathBuf};
use std::time::Instant;
use ambition_app::{AgentAction, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_app::app::project::Project;
use ambition_extension_host::{ExtensionFaults, ExtensionProfile, ModuleImplementation};
use ambition_platformer2d::extension::sdk::ModuleKey;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::sim::SimTick;
use bevy::prelude::*;
use serde_json::{json, Value};

fn memory() -> Value {
    let text = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let kb = |key: &str| text.lines().find_map(|line| line.strip_prefix(key))
        .and_then(|value| value.split_whitespace().next()?.parse::<u64>().ok());
    json!({"rss_kib": kb("VmRSS:"), "peak_rss_kib": kb("VmHWM:")})
}

#[test]
#[ignore = "measurement: use ../profile.py to compare separate backend processes"]
fn profile_promotion_workload() {
    let backend = std::env::var("PROFILE_BACKEND").expect("PROFILE_BACKEND needs rhai, wasm, or linked");
    let cycles: u32 = std::env::var("PROFILE_CYCLES").unwrap_or_else(|_| "10".into()).parse().unwrap();
    assert!(cycles > 0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..").canonicalize().unwrap();
    let dir = root.join("examples/projects/pulse_forge");
    let wasm = dir.join("build/pulse_forge.wasm");
    assert!(backend != "wasm" || wasm.is_file(), "build the portable module first; see the example README");
    let started = Instant::now();
    let process_started = started;
    let mut project = Project::load(&dir).unwrap();
    let implementation = match backend.as_str() {
        "rhai" => ModuleImplementation::Artifact(dir.join("modules/pulse_forge.rhai").to_string_lossy().into_owned().into()),
        "wasm" => {
            project.module_files.push(wasm.clone());
            ModuleImplementation::Artifact(wasm.to_string_lossy().into_owned().into())
        }
        "linked" => ModuleImplementation::Linked,
        _ => panic!("PROFILE_BACKEND needs rhai, wasm, or linked"),
    };
    project.module_selections.0.insert(ModuleKey::new("pulse_forge", "emitter"), implementation);
    let options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    let mut sim = pulse_forge_host::sim(&project, options).unwrap();
    while sim.world().resource::<SimTick>().get() < 8 { sim.step(AgentAction::default()); }
    let startup_ms = started.elapsed().as_secs_f64() * 1000.0;
    let player = sim.world_mut().query_filtered::<Entity, PrimaryPlayerOnly>().single(sim.world()).unwrap();
    sim.world_mut().remove_resource::<ambition_platformer2d::abilities::mana::ManaRegen>();
    let memory_before = memory();
    let unprofiled = std::env::var_os("PROFILE_NO_COUNTERS").is_some();
    if !unprofiled { sim.world_mut().insert_resource(ExtensionProfile::default()); }
    let mut ticks_us = Vec::new();
    let expected_speed = std::env::var("PROFILE_EXPECT_SPEED").ok().map(|value| value.parse::<f32>().unwrap());
    let mut first_projectile_ms = None;
    for _ in 0..cycles {
        sim.world_mut().get_mut::<ambition_platformer2d::actor::ActorResources>(player).unwrap()
            .level_of_mut(&ambition_platformer2d::abilities::mana::MANA).unwrap().current = 72.0;
        for frame in 0..280 {
            let before_tick = sim.world().resource::<SimTick>().get();
            let started = Instant::now();
            sim.step(AgentAction {
                projectile: matches!(frame, 0 | 150),
                projectile_held: frame < 70 || (150..220).contains(&frame),
                projectile_released: matches!(frame, 70 | 220),
                ..Default::default()
            });
            ticks_us.push(started.elapsed().as_secs_f64() * 1e6);
            assert_eq!(sim.world().resource::<SimTick>().get(), before_tick + 1);
            if let Some(speed) = expected_speed.filter(|_| first_projectile_ms.is_none()) {
                let world = sim.world_mut();
                if world.query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics,
                    With<ambition_platformer2d::projectiles::entity::ProjectileOwner>>()
                    .iter(world).any(|body| body.vel.x.abs() == speed || body.vel.y.abs() == speed) {
                    first_projectile_ms = Some(process_started.elapsed().as_secs_f64() * 1000.0);
                    println!("OBSERVED_SPEED {speed}");
                }
            }
        }
        assert_eq!(sim.world().get::<ambition_platformer2d::actor::ActorResources>(player).unwrap()
            .level_of(&ambition_platformer2d::abilities::mana::MANA).unwrap().current, 12.0);
    }
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    assert!(expected_speed.is_none() || first_projectile_ms.is_some(), "the changed speed did not reach a projectile");
    let entries: serde_json::Map<String, Value> = sim.world().get_resource::<ExtensionProfile>()
        .into_iter().flat_map(|profile| profile.0.iter()).map(|(key, row)| (key.clone(), json!({
            "calls": row.calls, "faults": row.faults,
            "total_us": row.total.as_secs_f64() * 1e6,
            "backend_us": row.backend.as_secs_f64() * 1e6,
            "host_us": row.total.saturating_sub(row.backend).as_secs_f64() * 1e6,
        }))).collect();
    let total_us: f64 = ticks_us.iter().sum();
    ticks_us.sort_by(f64::total_cmp);
    let report = json!({"backend": backend, "cycles": cycles, "ticks": ticks_us.len(),
        "profile_counters": !unprofiled, "startup_ms": startup_ms, "first_projectile_ms": first_projectile_ms,
        "step_total_us": total_us, "step_mean_us": total_us / ticks_us.len() as f64,
        "step_median_us": ticks_us[ticks_us.len() / 2],
        "memory_before": memory_before, "memory_after": memory(), "entries": entries});
    let out = PathBuf::from(std::env::var_os("PROFILE_OUT").expect("PROFILE_OUT needs an output JSON path"));
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    std::fs::write(&out, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{}", report);
}
