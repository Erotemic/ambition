//! ⭐ THE NO-BUILD LOOP (scripting audit P3): the sentry's `turret` entry
//! from Rhai source, in the shipped composition. Nothing is compiled.
//!
//! The observable is the turret's cadence record: its first countdown starts
//! at the script's arming delay (the native module's is 0.25 s).

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::abilities::module_entity::ModuleEntity;
use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
use ambition_platformer2d::extension::{AdmittedExtensions, BodyRecords, EntryRunner, ExtensionFaults};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::Entity;

const CADENCE: SchemaKey = SchemaKey::new("ambition", "sentry.cadence", 1);

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The shipped script with arming delay `delay`.
fn sentry_script(delay: f32) -> String {
    let text = std::fs::read_to_string(root().join("game/ambition_content_modules/rhai/sentry.rhai")).unwrap();
    assert!(text.contains("const ARM_DELAY_S = 0.25;"), "the premise: the script states the native delay");
    text.replace("const ARM_DELAY_S = 0.25;", &format!("const ARM_DELAY_S = {delay:?};"))
}

/// A sandbox running `text` as `sentry.rhai` (a file of this test's own,
/// named by `arm`), with the player holding the sentry gauntlet.
fn sandbox(arm: &str, text: &str, rollback: bool) -> (Platformer2dSimHarness, std::path::PathBuf) {
    let dir = root().join(format!("target/extension-modules/script_test_{}_{arm}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("sentry.rhai");
    std::fs::write(&file, text).unwrap();
    let mut options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_extension_module_files(vec![file.clone()]);
    if rollback {
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds with the script");
    assert!(turret_is_scripted(&sim), "the turret runs from the script");
    {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
        let player = q.single(world).expect("primary player exists");
        let spec = ambition_platformer2d::characters::brain::held_item_by_id("sentry").expect("a known held item");
        world
            .entity_mut(player)
            .insert(ambition_platformer2d::combat::held_items::HeldItem::new(spec));
    }
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    (sim, file)
}

fn turret_is_scripted(sim: &Platformer2dSimHarness) -> bool {
    let admitted = sim.world().resource::<AdmittedExtensions>();
    let entry = admitted
        .0
        .entries
        .iter()
        .find(|e| e.path == "ambition::sentry/turret")
        .expect("the sentry's turret is admitted");
    matches!(entry.runner, EntryRunner::Loaded { .. })
}

/// Press Attack once, step `frames`, and give each tick's cadence records of
/// the live turrets.
fn deploy(sim: &mut Platformer2dSimHarness, frames: usize) -> Vec<(bool, f32)> {
    let mut seen = Vec::new();
    for frame in 0..frames {
        sim.step(AgentAction {
            attack: frame == 0,
            ..AgentAction::default()
        });
        let world = sim.world_mut();
        let mut q = world.query::<(&ModuleEntity, Option<&BodyRecords>)>();
        for (entity, records) in q.iter(world) {
            if entity.kind != "sentry" {
                continue;
            }
            let record = records.and_then(|r| r.get(&CADENCE));
            let armed = record.map(|r| r.get(FieldRef(0)).unwrap().clone()) == Some(Value::Bool(true));
            let cooldown = match record.map(|r| r.get(FieldRef(1)).unwrap().clone()) {
                Some(Value::F32(c)) => c,
                _ => 0.0,
            };
            seen.push((armed, cooldown));
        }
    }
    assert!(!seen.is_empty(), "a turret was deployed (its deploy entry is native)");
    seen
}

fn largest_countdown(seen: &[(bool, f32)]) -> f32 {
    seen.iter().map(|(_, c)| *c).fold(f32::MIN, f32::max)
}

/// The script's turret under a GGRS sync-test session, where every tick it
/// runs is rewound and replayed: the countdown is the script's, and the
/// session stays healthy.
#[test]
fn a_scripted_turret_keeps_its_cadence_through_rollback() {
    let (mut sim, file) = sandbox("rollback", &sentry_script(0.75), true);
    let largest = largest_countdown(&deploy(&mut sim, 60));
    assert!((0.70..0.75).contains(&largest), "the script's 0.75 s delay, less a tick: {largest}");
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    let _ = std::fs::remove_file(file);
}

/// The script is edited while the game runs; the reload is proposed and
/// published on the mechanical-edit road, the session's content becomes a new
/// generation, and the next turret runs the edit. (Under a sync-test session
/// the harness owns the timeline and the reload is refused, as for a `.wasm`
/// file: `a_module_file_that_changes_while_the_game_runs_is_reloaded`.)
#[test]
fn an_edited_sentry_script_runs_in_the_game_without_a_build() {
    let (mut sim, file) = sandbox("edit", &sentry_script(0.75), false);
    let first = largest_countdown(&deploy(&mut sim, 330));
    assert!((0.70..0.75).contains(&first), "the script's 0.75 s delay, less a tick: {first}");
    let generation = sim.world().resource::<ambition_platformer2d::extension::ExtensionGeneration>().clone();

    // A file system's clock can be coarse: the edit must look newer.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&file, sentry_script(1.5)).unwrap();
    let published = |sim: &Platformer2dSimHarness| {
        sim.world().resource::<ambition_platformer2d::extension::reload::StagedModuleReplacement>().published()
    };
    let mut frames = 0;
    while published(&sim) == 0 {
        sim.step(AgentAction::default());
        frames += 1;
        assert!(frames < 120, "the edited script was not reloaded within 120 frames");
    }
    assert!(turret_is_scripted(&sim), "the reloaded turret runs from the script");
    assert_ne!(
        *sim.world().resource::<ambition_platformer2d::extension::ExtensionGeneration>(),
        generation,
        "an edited script is a new generation of the session's modules"
    );
    let second = largest_countdown(&deploy(&mut sim, 60));
    assert!((1.45..1.5).contains(&second), "the edit's 1.5 s delay, less a tick: {second}");
    let _ = std::fs::remove_file(file);
}

/// ⭐ A FAULTING INVOCATION COMMITS NOTHING (under rewinds): this script
/// changes the record and stages a bolt, then throws. The turret keeps its
/// initial record, fires nothing, and the fault names the file and its
/// generation.
#[test]
fn a_throwing_script_commits_neither_its_record_nor_its_requests() {
    let throwing = sentry_script(0.75).replace(
        "    this.state.cadence = c;\n}",
        "    this.state.cadence = c;\n    this.requests.push(#{ port: \"projectile\", value: #{ origin: [0.0, 0.0], dir: [1.0, 0.0], speed: 1.0, damage: 1, max_lifetime: 1.0, half_extent: [1.0, 1.0] } });\n    throw \"the witness's fault\";\n}",
    );
    assert!(throwing.contains("throw"), "the premise: the edit applied");
    let (mut sim, file) = sandbox("fault", &throwing, true);
    let bolts = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world.query::<&ambition_platformer2d::projectiles::entity::ProjectileOwner>().iter(world).count()
    };
    let bolts_before = bolts(&mut sim);
    let faults_before = sim.world().resource::<ExtensionFaults>().total;
    let seen = deploy(&mut sim, 60);
    assert!(seen.iter().all(|(armed, _)| !armed), "a faulting invocation committed its record change: {seen:?}");
    assert_eq!(bolts(&mut sim), bolts_before, "a faulting invocation's staged bolt was fired");
    let faults = sim.world().resource::<ExtensionFaults>();
    assert!(faults.total > faults_before, "the throwing turret faulted");
    let last = faults.recent.last().expect("a recent fault");
    let message = format!("{:?}", last.fault);
    assert_eq!(&*last.entry, "ambition::sentry/turret");
    assert!(
        message.contains("sentry.rhai") && message.contains("generation") && message.contains("the witness's fault"),
        "the fault names the source and its generation: {message}"
    );
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    let _ = std::fs::remove_file(file);
}

/// ⭐ A MODULE ADDED WHILE THE GAME RUNS, AND REMOVED (P3: discovery from
/// the list). The game watches a directory with no script in it: the turret
/// is the linked one. A `sentry.rhai` put in the directory loads on the
/// mechanical-edit road; taken out, it leaves, and the linked turret runs
/// again.
#[test]
fn a_script_added_to_a_watched_directory_runs_and_removed_leaves() {
    let dir = root().join(format!("target/extension-modules/script_test_{}_discovery", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_extension_module_files(vec![dir.clone()]);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
    assert!(!turret_is_scripted(&sim), "an empty directory: the turret is the linked one");
    let published = |sim: &Platformer2dSimHarness| {
        sim.world().resource::<ambition_platformer2d::extension::reload::StagedModuleReplacement>().published()
    };
    let step_until = |sim: &mut Platformer2dSimHarness, count: u32, what: &str| {
        let mut frames = 0;
        while published(sim) < count {
            sim.step(AgentAction::default());
            frames += 1;
            assert!(frames < 120, "{what} was not published within 120 frames");
        }
    };
    let file = dir.join("sentry.rhai");
    std::fs::write(&file, sentry_script(1.5)).unwrap();
    step_until(&mut sim, 1, "the added script");
    assert!(turret_is_scripted(&sim), "the added script runs the turret");
    std::fs::remove_file(&file).unwrap();
    step_until(&mut sim, 2, "the removal");
    assert!(!turret_is_scripted(&sim), "the script left: the linked turret runs again");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The player of `sim` holds the sentry gauntlet; a few ticks to settle.
fn hold_the_sentry(sim: &mut Platformer2dSimHarness) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
    let player = q.single(world).expect("primary player exists");
    let spec = ambition_platformer2d::characters::brain::held_item_by_id("sentry").expect("a known held item");
    world.entity_mut(player).insert(ambition_platformer2d::combat::held_items::HeldItem::new(spec));
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
}

/// ⭐ P3'S COMPLETION, THROUGH A PROJECT: the example project
/// (`examples/projects/sentry_drill`, a manifest and one script) runs its
/// turret under a GGRS sync-test session, where every tick is rewound and
/// replayed; the populated record restores to the script's countdown and the
/// session stays healthy.
#[test]
fn a_projects_scripted_turret_keeps_its_cadence_through_rollback() {
    let project = ambition_app::app::project::Project::load(&root().join("examples/projects/sentry_drill"))
        .expect("the example project loads");
    let options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_sync_test_rollback_settings(4, 10);
    let mut sim = project.sim(options).expect("the project builds");
    assert!(turret_is_scripted(&sim), "the project's script runs the turret");
    hold_the_sentry(&mut sim);
    let largest = largest_countdown(&deploy(&mut sim, 60));
    assert!((0.20..0.25).contains(&largest), "the project's 0.25 s delay, less a tick: {largest}");
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
}

/// The invocation-fault witness, through a project: its script changes the
/// record and stages a bolt, then throws. Nothing commits, and the fault
/// names the project's file.
#[test]
fn a_projects_throwing_script_commits_nothing() {
    let dir = root().join(format!("target/extension-modules/project_test_{}_fault", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("modules")).unwrap();
    let throwing = sentry_script(0.75).replace(
        "    this.state.cadence = c;\n}",
        "    this.state.cadence = c;\n    this.requests.push(#{ port: \"projectile\", value: #{ origin: [0.0, 0.0], dir: [1.0, 0.0], speed: 1.0, damage: 1, max_lifetime: 1.0, half_extent: [1.0, 1.0] } });\n    throw \"the project's fault\";\n}",
    );
    assert!(throwing.contains("throw"), "the premise: the edit applied");
    std::fs::write(dir.join("modules/sentry.rhai"), throwing).unwrap();
    std::fs::write(
        dir.join("project.ron"),
        r#"(schema: "ambition.project.v1", name: "faulting", modules: ["modules/sentry.rhai"])"#,
    )
    .unwrap();
    let project = ambition_app::app::project::Project::load(&dir).expect("the project loads");
    let options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_sync_test_rollback_settings(4, 10);
    let mut sim = project.sim(options).expect("the project builds");
    hold_the_sentry(&mut sim);
    let bolts = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world.query::<&ambition_platformer2d::projectiles::entity::ProjectileOwner>().iter(world).count()
    };
    let bolts_before = bolts(&mut sim);
    let seen = deploy(&mut sim, 60);
    assert!(seen.iter().all(|(armed, _)| !armed), "a faulting invocation committed its record change: {seen:?}");
    assert_eq!(bolts(&mut sim), bolts_before, "a faulting invocation's staged bolt was fired");
    let faults = sim.world().resource::<ExtensionFaults>();
    let message = format!("{:?}", faults.recent.last().expect("a recent fault").fault);
    assert!(
        message.contains("modules/sentry.rhai") && message.contains("generation") && message.contains("the project's fault"),
        "the fault names the project's file and its generation: {message}"
    );
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A module the game does not link, declared by its script (P3 step 3:
/// `fn declare()`): its own schema, attached to each sentry turret, and an
/// entry on the turret's tick that counts it. No Rust names it.
const TALLY: &str = r#"
fn declare() {
    #{
        provider: "drill",
        key: "turret_tally",
        schemas: [
            #{ key: "turret_tally.count", attachment: "body",
               fields: [ #{ tag: 1, name: "ticks", kind: "u32" } ] },
        ],
        entries: [
            #{ key: "tally", phase: "module_entity_tick",
               trigger: #{ port: "module_entity_tick", selector: "sentry" },
               writes: ["turret_tally.count"], limit: 0 },
        ],
    }
}

fn tally() {
    let c = this.state.count;
    c.ticks += 1;
    this.state.count = c;
}
"#;

/// ⭐ A MODULE A SCRIPT DECLARES: admitted beside the game's own sentry, it
/// runs on each living turret's tick and keeps its own record there, which
/// rollback restores (sync-test session, every tick rewound and replayed).
#[test]
fn a_module_a_script_declares_runs_with_its_own_record() {
    const COUNT: SchemaKey = SchemaKey::new("drill", "turret_tally.count", 1);
    let dir = root().join(format!("target/extension-modules/script_test_{}_declared", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("turret_tally.rhai"), TALLY).unwrap();
    let options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_sync_test_rollback_settings(4, 10)
        .with_extension_module_files(vec![dir.join("turret_tally.rhai")]);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds with the declared module");
    {
        let admitted = sim.world().resource::<AdmittedExtensions>();
        let entry = admitted.0.entries.iter().find(|e| e.path == "drill::turret_tally/tally").expect("the declared entry is admitted");
        assert!(matches!(entry.runner, EntryRunner::Loaded { .. }));
    }
    assert!(!turret_is_scripted(&sim), "the game's own turret is still the linked one");
    hold_the_sentry(&mut sim);
    let counts = |sim: &mut Platformer2dSimHarness| -> Vec<u32> {
        let world = sim.world_mut();
        let mut q = world.query::<(&ModuleEntity, Option<&BodyRecords>)>();
        q.iter(world)
            .filter(|(entity, _)| entity.kind == "sentry")
            .filter_map(|(_, records)| match records?.get(&COUNT)?.get(FieldRef(0)) {
                Ok(Value::U32(ticks)) => Some(*ticks),
                _ => None,
            })
            .collect()
    };
    sim.step(AgentAction { attack: true, ..AgentAction::default() });
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let first = counts(&mut sim);
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let second = counts(&mut sim);
    assert_eq!(first.len(), 1, "one turret carries the declared record: {first:?}");
    assert!(first[0] > 20, "the tally ran on the turret's ticks: {first:?}");
    assert_eq!(second[0], first[0] + 30, "one count per tick, through every rewind: {first:?} then {second:?}");
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Initial conditions as prepared data (P3, the project entry's step 3): a
/// project lists its starting bag (`starting_items`), and its sessions begin
/// with that bag and not the game's own (Ambition's starter unlocks its
/// abilities, the Fireball among them). The control: the example project
/// lists none and begins with the game's.
#[test]
fn a_projects_starting_bag_is_its_data() {
    use ambition_platformer2d::item::{Item, OwnedItems};
    let dir = root().join(format!("target/extension-modules/project_test_{}_bag", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("project.ron"),
        r#"(schema: "ambition.project.v1", name: "a bag", starting_items: Some([("bomb", 3), ("lantern", 1)]))"#,
    )
    .unwrap();
    let options = || Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    let project = ambition_app::app::project::Project::load(&dir).expect("the project loads");
    let sim = project.sim(options()).expect("the project builds");
    let bag = sim.world().resource::<OwnedItems>();
    assert_eq!(bag.count(Item::Bomb), 3, "the listed bombs");
    assert!(bag.has(Item::Lantern), "the listed lantern");
    assert!(!bag.has(Item::Fireball), "the game's starter is not added to the listed bag");

    let example = ambition_app::app::project::Project::load(&root().join("examples/projects/sentry_drill"))
        .expect("the example project loads");
    let sim = example.sim(options()).expect("the example builds");
    let bag = sim.world().resource::<OwnedItems>();
    assert!(bag.has(Item::Fireball) && !bag.has(Item::Lantern), "control: no list, the game's starting bag");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A declared module that ends each sentry turret on its tenth tick, through
/// the `end_module_entity` port (described for scripts with `pull_bodies` and
/// `launch_toucher`, P2: "the other ports as scripts need them").
const SHORT_FUSE: &str = r#"
fn declare() {
    #{
        provider: "drill",
        key: "short_fuse",
        schemas: [
            #{ key: "short_fuse.count", attachment: "body",
               fields: [ #{ tag: 1, name: "ticks", kind: "u32" } ] },
        ],
        entries: [
            #{ key: "fuse", phase: "module_entity_tick",
               trigger: #{ port: "module_entity_tick", selector: "sentry" },
               writes: ["short_fuse.count"], requests: ["end_module_entity"], limit: 1 },
        ],
    }
}

fn fuse() {
    let c = this.state.count;
    c.ticks += 1;
    this.state.count = c;
    if c.ticks == 10 {
        this.requests.push(request("end_module_entity", #{}));
    }
}
"#;

/// ⭐ A SCRIPT ENDS ITS ENTITY THROUGH A DESCRIBED PORT: the turret is gone
/// a few ticks after its tenth, through every rewind of a sync-test session.
/// The control: the same deploy with no such module keeps its turret.
#[test]
fn a_script_ends_a_turret_through_a_described_port() {
    let turrets = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world.query::<&ModuleEntity>().iter(world).filter(|entity| entity.kind == "sentry").count()
    };
    let deployed = |files: Vec<std::path::PathBuf>| -> (usize, usize) {
        let options = Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10)
            .with_extension_module_files(files);
        let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
        hold_the_sentry(&mut sim);
        sim.step(AgentAction { attack: true, ..AgentAction::default() });
        for _ in 0..5 {
            sim.step(AgentAction::default());
        }
        let early = turrets(&mut sim);
        for _ in 0..25 {
            sim.step(AgentAction::default());
        }
        assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
        (early, turrets(&mut sim))
    };
    let dir = root().join(format!("target/extension-modules/script_test_{}_fuse", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("short_fuse.rhai"), SHORT_FUSE).unwrap();
    assert_eq!(deployed(vec![dir.join("short_fuse.rhai")]), (1, 0), "deployed, then ended by the script");
    assert_eq!(deployed(Vec::new()), (1, 1), "control: with no fuse the turret stays");
    let _ = std::fs::remove_dir_all(&dir);
}
