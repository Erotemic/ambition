//! Project publication and saved records through the normal host roads.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use ambition_app::app::project::Project;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_combat_port::SteerPort;
use ambition_extension_host::{AdmittedExtensions, BodyRecords, EntryRunner, ExtensionGeneration, ExtensionOutbox, ExtensionSet, ModuleBackend};
use ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer2d::extension::sdk::{phases, FieldRef, SchemaKey, Value};
use ambition_platformer2d::sim::SimScheduleExt;
use ambition_platformer2d::characters::actor::CharacterForm;
use bevy::prelude::*;

fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..").canonicalize().unwrap() }
fn example() -> PathBuf { root().join("examples/projects/pulse_forge") }
fn options() -> Platformer2dSimHarnessOptions {
    Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz())
}

struct Scratch(PathBuf);
impl Scratch {
    fn new(name: &str) -> Self {
        let at = root().join(format!("target/procedural-project-tests/{name}-{}", std::process::id()));
        if at.exists() { std::fs::remove_dir_all(&at).unwrap(); }
        fn copy(from: &Path, to: &Path) {
            std::fs::create_dir_all(to).unwrap();
            for entry in std::fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() { copy(&entry.path(), &to.join(entry.file_name())); }
                else { std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap(); }
            }
        }
        for folder in ["content", "modules", "worlds"] { copy(&example().join(folder), &at.join(folder)); }
        std::fs::copy(example().join("project.ron"), at.join("project.ron")).unwrap();
        Self(at)
    }
    fn edit(&self, file: &str, text: &str) {
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(self.0.join(file), text).unwrap();
    }
    fn enable_progress(&self) -> String {
        let source = std::fs::read_to_string(example().join("recipes/progress.rhai")).unwrap();
        self.edit("modules/progress.rhai", &source);
        let content = std::fs::read_to_string(self.0.join("content/characters.ron")).unwrap();
        let content = content.replace(r#"tags: ["pulse_forge.sentinels"],"#,
            r#"tags: ["pulse_forge.sentinels"], techniques: [(id: ("pulse_forge"), slot: Projectile, gate: Technique("pulse_forge"))],"#);
        self.edit("content/characters.ron", &content);
        source
    }
    fn project(&self) -> Project { Project::load(&self.0).unwrap() }
    fn sim(&self) -> Platformer2dSimHarness { self.project().sim(options()).unwrap() }
}
impl Drop for Scratch { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }

#[derive(Resource, Default)]
struct Pairs(Vec<(f32, f32, f32)>);

fn observe_pair(world: &mut World) {
    let speed = world.resource::<CharacterCatalog>().get("forge_sentinel").unwrap()
        .locomotion.as_ref().unwrap().run_speed;
    let body_speed = world.query::<(&CharacterForm, &ambition_platformer2d::combat::actor_tuning::ActorConfig)>()
        .iter(world).find(|(form, _)| form.id() == "forge_sentinel").unwrap().1.tuning.max_run_speed;
    let velocities: Vec<_> = world.resource::<ExtensionOutbox>().submitted::<SteerPort>(&phases::ACTOR_THINK)
        .filter(|request| request.entry.starts_with("pulse_forge::sentinel/"))
        .map(|request| request.value.velocity[0].abs()).collect();
    world.resource_mut::<Pairs>().0.extend(velocities.into_iter().map(|velocity| (speed, body_speed, velocity)));
}

fn backend(sim: &Platformer2dSimHarness, key: &str) -> Weak<dyn ModuleBackend> {
    let entry = sim.world().resource::<AdmittedExtensions>().0.entries.iter()
        .find(|entry| entry.path.starts_with(&format!("pulse_forge::{key}/"))).unwrap();
    let EntryRunner::Loaded { backend, .. } = &entry.runner else { panic!("a source module is loaded") };
    Arc::downgrade(backend)
}

#[test]
fn script_and_character_edits_publish_together_and_retire_old_code() {
    let files = Scratch::new("paired");
    files.enable_progress();
    let mut sim = files.sim();
    sim.app_mut().init_resource::<Pairs>();
    let schedule = sim.app_mut().sim_schedule();
    sim.app_mut().add_systems(schedule, observe_pair
        .after(ExtensionSet::Invoke(phases::ACTOR_THINK))
        .before(ExtensionSet::Settle(phases::ACTOR_THINK))
        .in_set(ExtensionSet::Phase(phases::ACTOR_THINK)));
    sim.step_n(AgentAction::default(), 10);
    let before = ticks(&mut sim);
    let generation = sim.world().resource::<ExtensionGeneration>().clone();
    let old = backend(&sim, "sentinel");
    let script = std::fs::read_to_string(files.0.join("modules/sentinel.rhai")).unwrap();
    let content = std::fs::read_to_string(files.0.join("content/characters.ron")).unwrap();
    let changed = script.replace("p.direction * 48.0", "p.direction * 96.0");
    assert_ne!(script, changed);
    files.edit("modules/sentinel.rhai", &changed);
    files.edit("content/characters.ron", "invalid character catalog");
    sim.step_n(AgentAction::default(), 60);
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), generation);
    assert!(old.upgrade().is_some());
    assert!(sim.world().resource::<Pairs>().0.iter().all(|pair| *pair == (120.0, 120.0, 48.0)));

    let changed = content.replace("run_speed: 120.0", "run_speed: 180.0");
    assert_ne!(content, changed);
    files.edit("content/characters.ron", &changed);
    for _ in 0..300 {
        sim.step(AgentAction::default());
        if sim.world().resource::<Pairs>().0.contains(&(180.0, 180.0, 96.0)) { break; }
    }
    let pairs = &sim.world().resource::<Pairs>().0;
    let out = root().join("target/procedural-project-state");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("paired-trace.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "fields": ["catalog_run_speed", "body_run_speed", "requested_speed"], "samples": pairs,
    })).unwrap()).unwrap();
    assert!(pairs.contains(&(180.0, 180.0, 96.0)), "the repaired content publishes the held script edit: {pairs:?}");
    assert!(pairs.iter().all(|pair| *pair == (120.0, 120.0, 48.0) || *pair == (180.0, 180.0, 96.0)), "mixed project generation: {pairs:?}");
    assert!(old.upgrade().is_none(), "the retired backend has no remaining owner");
    assert!(ticks(&mut sim) > before, "publication preserves progress while the save changes each tick");
}

const TICKS: SchemaKey = SchemaKey::new("pulse_forge", "progress.ticks", 1);
fn ticks(sim: &mut Platformer2dSimHarness) -> u32 {
    let world = sim.world_mut();
    let store = world.query::<(&CharacterForm, &BodyRecords)>().iter(world)
        .find(|(form, _)| form.id() == "forge_sentinel").unwrap().1;
    store.get(&TICKS).map_or(0, |record| match record.get(FieldRef(0)).unwrap() {
        Value::U32(n) => *n, other => panic!("{other:?}")
    })
}

#[test]
fn a_durable_project_record_loads_and_an_incompatible_replacement_keeps_it() {
    use ambition_platformer2d::persistence::save::{load_save, write_save};
    let files = Scratch::new("saved");
    let source = files.enable_progress();
    let mut sim = files.sim();
    sim.step_n(AgentAction::default(), 60);
    let before = ticks(&mut sim);
    assert!(before >= 60);
    let save = sim.world().resource::<ambition_platformer2d::session::AmbitionGameSave>().data().clone();
    assert_eq!(save.body_module_records().len(), 1);
    let file = files.0.join("save.ron");
    write_save(&file, &save).unwrap();
    let loaded = load_save(&file);
    assert_eq!(loaded.data, save);
    let mut sim = files.project().sim(options().with_save(loaded.data)).unwrap();
    sim.step_n(AgentAction::default(), 10);
    let after_load = ticks(&mut sim);
    assert!(after_load > before, "the authored body continues from its saved counter");

    let generation = sim.world().resource::<ExtensionGeneration>().clone();
    let path = files.0.join("modules/progress.rhai").canonicalize().unwrap();
    let mut refusals = Vec::new();
    for (incompatible, diagnostic) in [
        (source.replace("attachment: \"body\"", "attachment: \"session\""), "attachment"),
        (source.replace("kind: \"u32\"", "kind: \"f32\""), "field tag 1"),
    ] {
        files.edit("modules/progress.rhai", &incompatible);
        let linked = ambition_platformer2d::extension::linked_modules(sim.world());
        let (code, modules) = ambition_platformer2d::extension::load_module_file(&path, &linked).unwrap();
        let saved = ambition_platformer2d::runtime::module_save_road::current_saved_rows(sim.world());
        let reason = ambition_extension_host::reload::stage_loaded_replacement(sim.world_mut(), &path.to_string_lossy(), code, modules, &saved).unwrap_err();
        assert!(reason.contains(diagnostic), "{reason}");
        sim.step_n(AgentAction::default(), 60);
        assert_eq!(*sim.world().resource::<ExtensionGeneration>(), generation);
        refusals.push(reason);
    }
    let continued = ticks(&mut sim);
    sim.step(AgentAction::default());
    assert_eq!(ticks(&mut sim), continued + 1, "the saved record and its old code remain usable");
    let out = root().join("target/procedural-project-state");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::copy(&file, out.join("save.ron")).unwrap();
    std::fs::write(out.join("save-observation.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "body": save.body_module_records()[0].body.as_str(),
        "before_save": before, "after_load": after_load,
        "after_refused_reload": continued + 1, "refusals": refusals,
    })).unwrap()).unwrap();
    let saved_file = std::fs::read(&file).unwrap();
    let mut startup_refusals = Vec::new();
    for incompatible in [
        source.replace("attachment: \"body\"", "attachment: \"session\""),
        source.replace("kind: \"u32\"", "kind: \"f32\""),
    ] {
        files.edit("modules/progress.rhai", &incompatible);
        let mut refused = files.project().sim(options().with_save(save.clone())).unwrap();
        for _ in 0..10 { refused.app_mut().update(); }
        assert!(refused.world().resource::<ambition_platformer2d::game_shell::ActiveGameplaySession>().0.is_none(),
            "an incompatible saved schema must not activate a fresh session");
        let reason = ambition_platformer2d::runtime::module_save_road::validate_saved_module_records(
            refused.world(), &save).unwrap_err();
        assert!(reason.contains("saved") && reason.contains("progress.ticks"), "{reason}");
        startup_refusals.push(reason);
    }
    assert_eq!(std::fs::read(file).unwrap(), saved_file, "refused starts leave the save file intact");

    std::fs::remove_file(files.0.join("modules/progress.rhai")).unwrap();
    sim.step_n(AgentAction::default(), 60);
    let removed = sim.world().resource::<ExtensionGeneration>().clone();
    assert!(!sim.world().resource::<AdmittedExtensions>().0.schemas.contains_key(&TICKS));
    let saved = sim.world().resource::<ambition_platformer2d::session::AmbitionGameSave>()
        .data().body_module_records().iter().find(|row| row.key == TICKS.to_string()).unwrap().clone();
    files.edit("modules/progress.rhai", &source.replace("kind: \"u32\"", "kind: \"f32\""));
    sim.step_n(AgentAction::default(), 60);
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), removed,
        "a newly added module must honor the saved schema, even with no live schema to compare");
    assert_eq!(sim.world().resource::<ambition_platformer2d::session::AmbitionGameSave>()
        .data().body_module_records().iter().find(|row| row.key == TICKS.to_string()), Some(&saved));
    std::fs::write(out.join("save-admission-refusals.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "startup_refusals": startup_refusals,
        "incompatible_added_module_admitted": sim.world().resource::<AdmittedExtensions>().0.schemas.contains_key(&TICKS),
        "saved_body": saved.body, "saved_key": saved.key,
    })).unwrap()).unwrap();
}
