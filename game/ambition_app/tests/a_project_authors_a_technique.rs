//! A project declares a technique, its equipment binding, and its emitter.
//! The host supplies the body, input, identity, requests, and rollback.

#![cfg(feature = "rl_sim")]

use ambition_app::app::project::Project;
use ambition_app::{
    AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::abilities::module_entity::ModuleEntity;
use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
use ambition_platformer2d::extension::{
    AdmittedExtensions, BodyRecords, ExtensionFaults, ExtensionGeneration,
};
use ambition_platformer2d::items::{Item, ItemEquipRequested};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::{Entity, Messages};
use std::path::{Path, PathBuf};

const CHARGE: SchemaKey = SchemaKey::new("pulse_forge", "emitter.charge", 1);
const CADENCE: SchemaKey = SchemaKey::new("pulse_forge", "emitter.cadence", 1);
const KIND: &str = "pulse_forge.emitter";

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/projects/pulse_forge")
}

fn options(rollback: bool) -> Platformer2dSimHarnessOptions {
    let options =
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if rollback {
        options.with_sync_test_rollback_settings(4, 10)
    } else {
        options
    }
}

fn equip(sim: &mut Platformer2dSimHarness) {
    ambition_platformer2d::platformer::host_intents::write_host_intent(
        sim.world_mut(),
        ItemEquipRequested {
            participant: ambition_platformer2d::characters::control::PlayerSlot(0),
            item: Item::EyeLaser,
        },
    );
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
}

fn charge(sim: &mut Platformer2dSimHarness, ticks: usize) {
    for tick in 0..ticks {
        sim.step(AgentAction {
            projectile: tick == 0,
            projectile_held: true,
            ..AgentAction::default()
        });
    }
}

fn charged_seconds(sim: &mut Platformer2dSimHarness) -> f32 {
    let world = sim.world_mut();
    let body = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    match world
        .get::<BodyRecords>(body)
        .and_then(|r| r.get(&CHARGE))
        .and_then(|r| r.get(FieldRef(0)).ok())
    {
        Some(Value::F32(seconds)) => *seconds,
        other => panic!("the body's charge record: {other:?}"),
    }
}

fn emitter(sim: &mut Platformer2dSimHarness) -> Option<Entity> {
    let world = sim.world_mut();
    let mut query = world.query::<(Entity, &ModuleEntity)>();
    let emitters: Vec<_> = query
        .iter(world)
        .filter(|(_, m)| m.kind == KIND)
        .map(|(e, _)| e)
        .collect();
    assert!(
        emitters.len() <= 1,
        "one release creates at most one emitter"
    );
    emitters.first().copied()
}

fn release(sim: &mut Platformer2dSimHarness) {
    sim.step(AgentAction {
        projectile_released: true,
        ..AgentAction::default()
    });
}

fn fired(sim: &Platformer2dSimHarness, body: Entity) -> u32 {
    match sim
        .world()
        .get::<BodyRecords>(body)
        .and_then(|r| r.get(&CADENCE))
        .and_then(|r| r.get(FieldRef(0)).ok())
    {
        Some(Value::U32(count)) => *count,
        other => panic!("the emitter's cadence record: {other:?}"),
    }
}

#[test]
fn a_project_declares_and_runs_a_charged_technique_through_rollback() {
    let project = Project::load(&example()).unwrap();
    let mut sim = project
        .sim(options(true))
        .expect("the authored project builds");
    let admitted = sim.world().resource::<AdmittedExtensions>();
    assert!(
        admitted
            .0
            .entries
            .iter()
            .any(|e| e.path == "pulse_forge::emitter/charge")
    );
    assert!(
        admitted
            .0
            .entries
            .iter()
            .any(|e| e.path == "pulse_forge::emitter/pulse")
    );
    equip(&mut sim);
    charge(&mut sim, 40);
    let seconds = charged_seconds(&mut sim);
    assert!(
        seconds > 0.0,
        "holding the authored slot populates its record"
    );
    assert!(
        emitter(&mut sim).is_none(),
        "holding does not release the charge"
    );
    charge(&mut sim, 10);
    assert!(
        charged_seconds(&mut sim) > seconds,
        "the populated charge advances through rewinds"
    );
    release(&mut sim);
    assert_eq!(charged_seconds(&mut sim), 0.0, "release spends the charge");
    assert!(
        emitter(&mut sim).is_some(),
        "release spawns the authored entity kind"
    );
    sim.step(AgentAction::default());
    let body = emitter(&mut sim).expect("the emitter survives its first pulse");
    assert_eq!(fired(&sim, body), 1, "the emitter keeps its own record");
    let mut saw_second_pulse = false;
    for _ in 0..90 {
        sim.step(AgentAction::default());
        if let Some(body) = emitter(&mut sim) {
            saw_second_pulse |= fired(&sim, body) >= 2;
        }
    }
    assert!(
        saw_second_pulse,
        "a charged emitter fires more than one pulse"
    );
    assert!(
        emitter(&mut sim).is_none(),
        "the script ends the emitter after its pulses"
    );
    assert_eq!(
        ambition_platformer2d::rollback::session_health(sim.world()),
        Ok(())
    );
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
}

struct ScratchProject(PathBuf);

impl ScratchProject {
    fn new(arm: &str, script: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("ambition_pulse_forge_{}_{arm}", std::process::id()));
        std::fs::create_dir_all(dir.join("modules")).unwrap();
        std::fs::write(dir.join("modules/pulse_forge.rhai"), script).unwrap();
        let content = example().join("content").canonicalize().unwrap();
        std::fs::write(dir.join("project.ron"), format!(
            "(schema: \"ambition.project.v1\", name: \"pulse test\", content: Some({:?}), modules: [\"modules\"], starting_items: Some([(\"eyelaser\", 1)]))",
            content.to_str().unwrap(),
        )).unwrap();
        Self(dir)
    }

    fn sim(&self, rollback: bool) -> Platformer2dSimHarness {
        Project::load(&self.0)
            .unwrap()
            .sim(options(rollback))
            .expect("the project builds")
    }

    fn edit(&self, text: &str) {
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(self.0.join("modules/pulse_forge.rhai"), text).unwrap();
    }
}

impl Drop for ScratchProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn source() -> String {
    std::fs::read_to_string(example().join("modules/pulse_forge.rhai")).unwrap()
}

#[test]
fn a_project_technique_fault_keeps_its_charge_and_spawns_nothing() {
    let throwing = source().replace("        c.seconds = 0.0;", "        c.seconds = 0.0;\n        this.state.charge = c;\n        throw \"release refused\";");
    assert!(throwing.contains("throw"));
    let project = ScratchProject::new("fault", &throwing);
    let mut sim = project.sim(true);
    equip(&mut sim);
    charge(&mut sim, 40);
    let before = charged_seconds(&mut sim);
    assert!(before > 0.0);
    release(&mut sim);
    assert_eq!(
        charged_seconds(&mut sim),
        before,
        "a fault commits no record writes"
    );
    assert!(
        emitter(&mut sim).is_none(),
        "a fault commits no spawn requests"
    );
    let faults = sim.world().resource::<ExtensionFaults>();
    let last = faults
        .recent
        .last()
        .expect("the release fault was reported");
    let fault = format!("{:?}", last.fault);
    assert!(
        fault.contains("pulse_forge.rhai")
            && fault.contains("generation")
            && fault.contains("release refused"),
        "{fault}"
    );
    assert_eq!(
        ambition_platformer2d::rollback::session_health(sim.world()),
        Ok(())
    );
}

#[test]
fn a_project_technique_keeps_last_good_code_then_reloads_and_leaves() {
    let original = source();
    let project = ScratchProject::new("reload", &original);
    let mut sim = project.sim(false);
    equip(&mut sim);
    let generation = sim.world().resource::<ExtensionGeneration>().clone();
    project.edit("fn declare() { syntax error");
    for _ in 0..50 {
        sim.step(AgentAction::default());
    }
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), generation);
    charge(&mut sim, 40);
    release(&mut sim);
    assert!(
        emitter(&mut sim).is_some(),
        "invalid source keeps the last good program"
    );
    for _ in 0..100 {
        sim.step(AgentAction::default());
    }

    let edited = original.replace("const BOLT_SPEED = 240.0;", "const BOLT_SPEED = 360.0;");
    assert_ne!(edited, original);
    project.edit(&edited);
    for _ in 0..50 {
        sim.step(AgentAction::default());
    }
    assert_ne!(*sim.world().resource::<ExtensionGeneration>(), generation);
    charge(&mut sim, 40);
    release(&mut sim);
    sim.step(AgentAction::default());
    let shots = sim
        .world()
        .resource::<Messages<ambition_platformer2d::projectiles::ProjectileSpawnRequest>>();
    let requests: Vec<_> = shots.iter_current_update_messages().collect();
    assert_eq!(requests.len(), 4, "the emitter requests four bolts");
    assert!(
        requests
            .iter()
            .all(|r| (r.projectile.body.kin.vel.length() - 360.0).abs() < 0.01),
        "the new speed reaches the projectile domain"
    );

    std::fs::remove_file(project.0.join("modules/pulse_forge.rhai")).unwrap();
    for _ in 0..50 {
        sim.step(AgentAction::default());
    }
    assert!(
        !sim.world()
            .resource::<AdmittedExtensions>()
            .0
            .entries
            .iter()
            .any(|e| e.path.starts_with("pulse_forge::"))
    );
    assert!(
        sim.world_mut()
            .query::<&BodyRecords>()
            .iter(sim.world())
            .all(|r| r.get(&CHARGE).is_none() && r.get(&CADENCE).is_none()),
        "removal retires the script's records"
    );
    project.edit(&original);
    for _ in 0..50 {
        sim.step(AgentAction::default());
    }
    assert!(
        sim.world()
            .resource::<AdmittedExtensions>()
            .0
            .entries
            .iter()
            .any(|e| e.path == "pulse_forge::emitter/charge"),
        "a script added to the project directory is admitted again"
    );
}
