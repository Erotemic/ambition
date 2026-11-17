//! The same authored game and input through Rhai, WASM, and linked Rust.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use ambition_app::app::project::Project;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_combat_port::{EndModuleEntityPort, SpawnModuleEntityPort, SpendManaPort};
use ambition_extension_host::ExtensionOutbox;
use ambition_platformer2d::abilities::module_entity::ModuleEntity;
use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::extension::{AdmittedExtensions, BodyRecords, ExtensionFaults, ExtensionSet, ModuleImplementation};
use ambition_platformer2d::extension::sdk::{phases, ModuleKey, Phase};
use ambition_platformer2d::extension::sdk::script::{ScriptPort, ScriptValue, record_to_script};
use ambition_platformer2d::items::{Item, ItemEquipRequested};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::sim::{SimScheduleExt, SimTick};
use ambition_projectile_spec::ProjectileSpawnPort;
use bevy::prelude::*;
use serde_json::{json, Value};

#[derive(Resource, Default)]
struct Requests {
    first_tick: Option<u64>,
    rows: BTreeMap<String, Value>,
}

fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..").canonicalize().unwrap() }
fn project_dir() -> PathBuf { root().join("examples/projects/pulse_forge") }

fn json_value(value: ScriptValue) -> Value {
    match value {
        ScriptValue::Unit => Value::Null,
        ScriptValue::Bool(v) => json!(v), ScriptValue::Int(v) => json!(v), ScriptValue::Num(v) => json!(v),
        ScriptValue::Text(v) => json!(v),
        ScriptValue::List(v) => Value::Array(v.into_iter().map(json_value).collect()),
        ScriptValue::Map(v) => Value::Object(v.into_iter().map(|(k, v)| (k, json_value(v))).collect()),
    }
}

fn submitted<P: ScriptPort>(world: &World, phase: &Phase) -> Vec<Value> {
    world.resource::<ExtensionOutbox>().submitted::<P>(phase)
        .filter(|request| request.entry.starts_with("pulse_forge::"))
        .map(|request| json!({
            "scope": world.get::<SimId>(request.scope).expect("a game body has an identity").as_str(),
            "entry": request.entry.as_ref(), "occurrence": request.occurrence,
            "value": json_value(P::to_script(request.value)),
        })).collect()
}

fn observe(world: &mut World, phase: Phase, settled: bool) {
    let tick = world.resource::<SimTick>().get();
    let row = if phase == phases::TECHNIQUE_EXECUTION {
        json!({"spawn": submitted::<SpawnModuleEntityPort>(world, &phase), "cost": submitted::<SpendManaPort>(world, &phase)})
    } else {
        json!({"projectile": submitted::<ProjectileSpawnPort>(world, &phase), "end": submitted::<EndModuleEntityPort>(world, &phase)})
    };
    let mut observed = world.resource_mut::<Requests>();
    let origin = *observed.first_tick.get_or_insert(tick);
    let tick = tick.checked_sub(origin).expect("a replay stays inside the fixture input history");
    observed.rows.insert(format!("{tick:06}/{}/{settled}", phase.0), row);
}

fn add_observers(sim: &mut Platformer2dSimHarness) {
    let app = sim.app_mut();
    app.init_resource::<Requests>();
    let schedule = app.sim_schedule();
    for phase in [phases::TECHNIQUE_EXECUTION, phases::MODULE_ENTITY_TICK] {
        let before = phase.clone();
        let after = phase.clone();
        app.add_systems(schedule, (
            (move |world: &mut World| observe(world, before.clone(), false))
                .after(ExtensionSet::Invoke(phase.clone())).before(ExtensionSet::Settle(phase.clone())),
            (move |world: &mut World| observe(world, after.clone(), true))
                .after(ExtensionSet::Settle(phase.clone())).before(ExtensionSet::Lower(phase.clone())),
        ).in_set(ExtensionSet::Phase(phase)));
    }
}

fn facts(sim: &mut Platformer2dSimHarness) -> Value {
    let world = sim.world_mut();
    let schemas = world.resource::<AdmittedExtensions>().0.schemas.clone();
    let mut records = BTreeMap::new();
    for (id, stored) in world.query::<(&SimId, &BodyRecords)>().iter(world) {
        for row in stored.0.records().iter().filter(|row| row.key.provider == "pulse_forge") {
            records.insert(format!("{id}/{}", row.key), json_value(record_to_script(&schemas[&row.key].schema, &row.record)));
        }
    }
    let mut entities: Vec<_> = world.query::<(&SimId, &ModuleEntity)>().iter(world)
        .filter(|(_, body)| body.kind == pulse_forge_modules::KIND)
        .map(|(id, body)| json!({"id": id.as_str(), "position": body.pos, "remaining_s": body.remaining_s})).collect();
    entities.sort_by_key(Value::to_string);
    let mut projectiles: Vec<_> = world.query::<(&SimId, &ambition_platformer2d::projectiles::entity::ProjectileOwner,
        &ambition_platformer2d::engine_core::BodyKinematics)>()
        .iter(world).map(|(id, _, body)| json!({"id": id.as_str(), "position": body.pos, "velocity": body.vel})).collect();
    projectiles.sort_by_key(Value::to_string);
    let mut mana = BTreeMap::new();
    for (id, bank) in world.query::<(&SimId, &ambition_platformer2d::actor::ActorResources)>().iter(world) {
        if let Some(level) = bank.level_of(&ambition_platformer2d::abilities::mana::MANA) {
            mana.insert(id.as_str(), level.current);
        }
    }
    json!({"records": records, "entities": entities, "projectiles": projectiles, "mana": mana})
}

fn run(implementation: ModuleImplementation, rollback: bool, altered_source: Option<&Path>) -> Value {
    let backend = match &implementation {
        ModuleImplementation::Linked => "linked",
        ModuleImplementation::Artifact(path) if path.ends_with(".wasm") => "wasm",
        _ => "rhai",
    };
    let mut project = Project::load(&project_dir()).unwrap();
    if let Some(path) = altered_source {
        project.module_files = vec![path.to_path_buf(), project_dir().join("modules/sentinel.rhai")];
    }
    if let ModuleImplementation::Artifact(path) = &implementation {
        project.module_files.push(PathBuf::from(path.as_ref()));
    }
    project.module_selections.0.insert(ModuleKey::new("pulse_forge", "emitter"), implementation);
    let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if rollback { options = options.with_sync_test_rollback_settings(4, 10); }
    let mut sim = pulse_forge_host::sim(&project, options).unwrap();
    add_observers(&mut sim);
    ambition_platformer2d::platformer::host_intents::write_host_intent(sim.world_mut(), ItemEquipRequested {
        participant: PlayerSlot(0), item: Item::EyeLaser,
    });
    for _ in 0..5 { sim.step(AgentAction::default()); }
    let player = sim.world_mut().query_filtered::<Entity, PrimaryPlayerOnly>().single(sim.world()).unwrap();
    sim.world_mut().remove_resource::<ambition_platformer2d::abilities::mana::ManaRegen>();
    sim.world_mut().get_mut::<ambition_platformer2d::actor::ActorResources>(player).unwrap()
        .level_of_mut(&ambition_platformer2d::abilities::mana::MANA).unwrap().current = 72.0;
    sim.rebase_rollback_history().unwrap();
    *sim.world_mut().resource_mut::<Requests>() = Requests::default();
    let initial = facts(&mut sim);
    let mut frames = Vec::new();
    for frame in 0..280 {
        let pressed = matches!(frame, 0 | 150);
        let held = frame < 70 || (150..220).contains(&frame);
        let released = matches!(frame, 70 | 220);
        sim.step(AgentAction { projectile: pressed, projectile_held: held, projectile_released: released, ..Default::default() });
        frames.push(facts(&mut sim));
    }
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent);
    let requests = &sim.world().resource::<Requests>().rows;
    let count = |field: &str, settled: bool| requests.iter().filter(|(key, _)| key.ends_with(&format!("/{settled}")))
        .map(|(_, row)| row.get(field).and_then(Value::as_array).map_or(0, Vec::len)).sum::<usize>();
    assert_eq!(count("spawn", false), 2, "both releases submit a spawn");
    assert_eq!(count("spawn", true), 1, "the mana owner refuses the second release");
    assert_eq!(count("projectile", false), 20, "five accepted pulses emit four ordered requests each");
    assert!(frames.iter().any(|frame| !frame["entities"].as_array().unwrap().is_empty()));
    assert!(frames.iter().any(|frame| !frame["projectiles"].as_array().unwrap().is_empty()));
    assert!(frames.last().unwrap()["entities"].as_array().unwrap().is_empty());
    assert_eq!(sim.world().get::<ambition_platformer2d::actor::ActorResources>(player).unwrap()
        .level_of(&ambition_platformer2d::abilities::mana::MANA).unwrap().current, 12.0);
    let trace = json!({"initial": initial, "requests": requests, "frames": frames});
    let dir = root().join("target/pulse-forge-parity");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{backend}-rollback-{rollback}-altered-{}.json", altered_source.is_some())),
        serde_json::to_vec_pretty(&trace).unwrap()).unwrap();
    trace
}

fn differences(expected: &Value, actual: &Value, path: &str, out: &mut Vec<Value>) {
    match (expected, actual) {
        (Value::Object(a), Value::Object(b)) => {
            let keys: BTreeSet<_> = a.keys().chain(b.keys()).collect();
            for key in keys { differences(a.get(key).unwrap_or(&Value::Null), b.get(key).unwrap_or(&Value::Null), &format!("{path}.{key}"), out); }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (index, (a, b)) in a.iter().zip(b).enumerate() { differences(a, b, &format!("{path}[{index}]"), out); }
        }
        (a, b) if a != b => out.push(json!({"path": path, "expected": a, "actual": b})),
        _ => {},
    }
}

#[test]
fn the_project_has_the_same_contract_and_gameplay_through_all_backends_and_rollback() {
    let wasm = ambition_platformer2d::extension::build_module_crate(&root(), "pulse_forge_modules").unwrap();
    let source = project_dir().join("modules/pulse_forge.rhai");
    let linked = vec![pulse_forge_modules::module().into()];
    for file in [&source, &wasm] {
        let (_, modules) = ambition_platformer2d::extension::load_module_file(file, &linked).unwrap();
        assert_ne!(modules[0].code, linked[0].descriptor.code);
        assert_eq!(modules[0].semantic_differences(&linked[0].descriptor), vec![]);
    }
    let mut baseline = None;
    for rollback in [false, true] {
        let native = run(ModuleImplementation::Linked, rollback, None);
        for (name, file) in [("rhai", &source), ("wasm", &wasm)] {
            let candidate = run(ModuleImplementation::Artifact(file.display().to_string().into()), rollback, None);
            let mut diff = Vec::new();
            differences(&native, &candidate, "$", &mut diff);
            assert!(diff.is_empty(), "{name}, rollback={rollback}: {}", serde_json::to_string_pretty(&diff.iter().take(12).collect::<Vec<_>>()).unwrap());
        }
        if let Some(baseline) = &baseline {
            let mut diff = Vec::new();
            differences(baseline, &native, "$", &mut diff);
            assert!(diff.is_empty(), "rollback: {}", serde_json::to_string_pretty(&diff.iter().take(12).collect::<Vec<_>>()).unwrap());
        } else { baseline = Some(native); }
    }
}

#[test]
fn changed_backend_logic_produces_a_structured_gameplay_diff() {
    let dir = root().join("target/pulse-forge-parity-negative");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("pulse_forge.rhai");
    let source = std::fs::read_to_string(project_dir().join("modules/pulse_forge.rhai")).unwrap();
    std::fs::write(&file, source.replace("BOLT_SPEED = 240.0", "BOLT_SPEED = 360.0")).unwrap();
    let native = run(ModuleImplementation::Linked, false, None);
    let changed = run(ModuleImplementation::Artifact(file.display().to_string().into()), false, Some(&file));
    let mut diff = Vec::new();
    differences(&native, &changed, "$", &mut diff);
    assert!(diff.iter().any(|row| row["path"].as_str().unwrap().ends_with(".speed")), "{}", json!(diff));
    std::fs::write(dir.join("diff.json"), serde_json::to_vec_pretty(&diff).unwrap()).unwrap();
}
