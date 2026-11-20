//! A project declares a character technique, its patrol, and its emitter.
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

fn ready(sim: &mut Platformer2dSimHarness) {
    assert!(
        ambition_platformer2d::world::rooms::sole_live_room_definition(sim.world()).is_some(),
        "the project session did not start: {:?}",
        sim.world().resource::<ambition_platformer2d::game_shell::ShellFailureLog>().reasons(),
    );
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    let world = sim.world_mut();
    let body = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    assert!(world.get::<ambition_platformer2d::characters::action_scheme::ActorTechniques>(body)
        .is_some_and(|techniques| techniques.all().iter().any(|action| action.id.as_str() == "pulse_forge")),
        "the character declares its technique without an inventory binding");
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
    let manifest = sim.world().resource::<ambition_platformer2d::world::world_manifest::WorldManifest>();
    assert_eq!(manifest.worlds.len(), 1);
    assert_eq!(manifest.primary().id.as_str(), "world.pulse_forge");
    assert_eq!(manifest.entry_room, "pulse_forge_drill");
    assert!(manifest.primary().embedded_text.is_none());
    assert!(manifest.primary().embedded_bevy_path.is_none());
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
    ready(&mut sim);
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
        std::fs::create_dir_all(dir.join("worlds")).unwrap();
        std::fs::copy(example().join("worlds/drill.ldtk"), dir.join("worlds/drill.ldtk")).unwrap();
        std::fs::write(dir.join("modules/pulse_forge.rhai"), script).unwrap();
        let content = example().join("content").canonicalize().unwrap();
        std::fs::write(dir.join("project.ron"), format!(
            "(schema: \"ambition.project.v1\", name: \"pulse test\", content: Some({:?}), worlds: Some(\"worlds\"), modules: [\"modules\"], starting_items: Some([]))",
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
fn a_project_world_loads_draws_and_keeps_the_live_room_when_reload_fails() {
    use ambition_app::app::VisibleRenderMode;
    use ambition_platformer2d::dev_tools::WorldSourceHotReload;
    use ambition_platformer2d::ldtk_map::ActiveLdtkProject;
    use ambition_platformer2d::platformer::lifecycle::{RoomVisual, session_world_component};
    use ambition_platformer2d::world::rooms::{RoomSet, sole_live_room_definition};

    let scratch = ScratchProject::new("world", &source());
    let path = scratch.0.join("worlds/drill.ldtk").canonicalize().unwrap();
    let original = std::fs::read_to_string(&path).unwrap();
    let project = Project::load(&scratch.0).unwrap();
    let mut app = project.app(VisibleRenderMode::NoWindow);
    bevy::tasks::block_on(async {
        let source = app.world().resource::<bevy::asset::AssetServer>().get_source("project").unwrap();
        let reader = source.reader();
        let mut world_file = reader.read(Path::new("worlds/drill.ldtk")).await.unwrap();
        let mut bytes = Vec::new();
        world_file.read_to_end(&mut bytes).await.unwrap();
        assert_eq!(bytes, original.as_bytes());
        // LDtk's relative image path uses the same source's library imports.
        let mut sprite = reader.read(Path::new("sprites/player_robot_v3_portraits.png")).await.unwrap();
        bytes.clear();
        sprite.read_to_end(&mut bytes).await.unwrap();
        assert!(!bytes.is_empty());
    });
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_nanos(16_666_667),
    ));
    let started = std::time::Instant::now();
    loop {
        app.update();
        let drawn = app.world_mut().query_filtered::<Entity, bevy::prelude::With<RoomVisual>>()
            .iter(app.world()).count();
        if sole_live_room_definition(app.world()).is_some() && drawn > 0 {
            break;
        }
        assert!(started.elapsed().as_secs() < 15, "the project world did not draw: {:?}",
            app.world().resource::<ambition_platformer2d::game_shell::ShellFailureLog>().reasons());
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let rooms = session_world_component::<RoomSet>(app.world()).unwrap();
    assert_eq!(rooms.rooms.len(), 1, "the host's rooms are not added");
    assert_eq!(rooms.rooms[0].id, "pulse_forge_drill");
    let before = sole_live_room_definition(app.world()).unwrap();
    let watch = app.world().resource::<WorldSourceHotReload>();
    assert_eq!(watch.watch_path.as_ref(), Some(&path));
    let applied = watch.applied_count;
    let levels = &app.world().resource::<ActiveLdtkProject>().0.levels;
    assert_eq!(levels.len(), 1);
    assert_eq!(levels[0].world_source, "world.pulse_forge");

    std::fs::write(&path, "{ invalid world").unwrap();
    crate::an_edit_reaches_the_shipped_game::press_apply_reload(&mut app);
    let watch = app.world().resource::<WorldSourceHotReload>();
    assert_eq!(watch.applied_count, applied);
    assert!(!watch.last_errors.is_empty());
    assert_eq!(sole_live_room_definition(app.world()).unwrap(), before);
    assert_eq!(app.world().resource::<ActiveLdtkProject>().0.levels.len(), 1);

    std::fs::write(&path, original).unwrap();
    crate::an_edit_reaches_the_shipped_game::press_apply_reload(&mut app);
    let watch = app.world().resource::<WorldSourceHotReload>();
    assert_eq!(watch.applied_count, applied + 1, "{}", watch.last_status);
    assert!(watch.last_errors.is_empty());
    for _ in 0..10 {
        app.update();
    }
    let rooms = session_world_component::<RoomSet>(app.world()).unwrap();
    assert_eq!(rooms.rooms.len(), 1);
    assert_eq!(rooms.rooms[0].id, "pulse_forge_drill");
}

#[test]
fn a_project_technique_fault_keeps_its_charge_and_spawns_nothing() {
    let throwing = source().replace("        c.seconds = 0.0;", "        c.seconds = 0.0;\n        this.state.charge = c;\n        throw \"release refused\";");
    assert!(throwing.contains("throw"));
    let project = ScratchProject::new("fault", &throwing);
    let mut sim = project.sim(true);
    ready(&mut sim);
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
    ready(&mut sim);
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
        .resource::<Messages<ambition_platformer2d::projectiles::ProjectileEmission>>();
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
            .any(|e| e.path.starts_with("pulse_forge::emitter/"))
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

#[test]
fn a_failed_file_keeps_the_whole_script_set_until_a_repaired_set_loads() {
    let original = source();
    let project = ScratchProject::new("atomic_set", &original);
    let mut sim = project.sim(false);
    ready(&mut sim);
    let generation = sim.world().resource::<ExtensionGeneration>().clone();
    project.edit(&original.replace("const BOLT_SPEED = 240.0;", "const BOLT_SPEED = 360.0;"));
    let second = project.0.join("modules/second_forge.rhai");
    std::fs::write(&second, "fn declare() { invalid source").unwrap();
    for _ in 0..50 { sim.step(AgentAction::default()); }
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), generation,
        "one bad file must not publish the other file's valid edit");
    std::fs::write(&second, original.replace("pulse_forge", "second_forge")).unwrap();
    for _ in 0..50 { sim.step(AgentAction::default()); }
    assert_ne!(*sim.world().resource::<ExtensionGeneration>(), generation);
    assert!(sim.world().resource::<AdmittedExtensions>().0.entries.iter()
        .any(|entry| entry.path.starts_with("second_forge::")));
    charge(&mut sim, 40);
    release(&mut sim);
    sim.step(AgentAction::default());
    let shots = sim.world().resource::<Messages<ambition_platformer2d::projectiles::ProjectileEmission>>();
    let requests: Vec<_> = shots.iter_current_update_messages().collect();
    assert_eq!(requests.len(), 4);
    assert!(requests.iter().all(|r| (r.projectile.body.kin.vel.length() - 360.0).abs() < 0.01),
        "repairing only the bad file must also publish the valid edit held back with it");
}
