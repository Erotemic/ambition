//! Lightning uses the inventory, technique, projectile, and rollback paths.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::{BodyHealth, CharacterForm};
use ambition_platformer2d::engine_core::{BodyKinematics, resources::ActorResources};
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults, ExtensionGeneration, ExtensionProfile};
use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
use ambition_platformer2d::items::{Item, ItemEquipRequested};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::projectiles::ProjectileSpawnRequest;
use bevy::prelude::{Entity, Messages};

const CADENCE: SchemaKey = SchemaKey::new("ambition", "lightning.cadence", 1);
const SOURCE: &str = include_str!("../../ambition_content/rhai/lightning.rhai");

fn primary(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap()
}

fn equip(sim: &mut Platformer2dSimHarness, item: Item) {
    ambition_platformer2d::platformer::host_intents::write_host_intent(sim.world_mut(), ItemEquipRequested {
        participant: ambition_platformer2d::characters::control::PlayerSlot(0), item,
    });
    sim.step_n(AgentAction::default(), 3);
}

fn sandbox(files: Vec<std::path::PathBuf>, rollback: bool) -> Platformer2dSimHarness {
    let mut options = crate::common::sandbox_options()
        .with_timestep(ambition_app::TimestepMode::fixed_60hz())
        .with_extension_module_files(files);
    if rollback { options = options.with_sync_test_rollback_settings(4, 12); }
    let mut sim = Platformer2dSimHarness::new_with_options(options).unwrap();
    sim.step_n(AgentAction::default(), 30);
    equip(&mut sim, Item::Lightning);
    sim
}

fn held(pressed: bool) -> AgentAction {
    AgentAction { projectile: pressed, projectile_held: true, aim_x: 1.0, ..AgentAction::default() }
}

fn shots(sim: &Platformer2dSimHarness, owner: Entity) -> Vec<&ProjectileSpawnRequest> {
    sim.world().resource::<Messages<ProjectileSpawnRequest>>()
        .iter_current_update_messages().filter(|shot| shot.owner == owner).collect()
}

fn mana(sim: &Platformer2dSimHarness, body: Entity) -> f32 {
    sim.world().get::<ActorResources>(body).unwrap()
        .level_of(&ambition_platformer2d::abilities::mana::MANA).unwrap().current
}

fn cooldown(sim: &Platformer2dSimHarness, body: Entity) -> f32 {
    match sim.world().get::<BodyRecords>(body).and_then(|r| r.get(&CADENCE)).and_then(|r| r.get(FieldRef(0)).ok()) {
        Some(Value::F32(value)) => *value,
        None => 0.0,
        other => panic!("unexpected cadence: {other:?}"),
    }
}

#[test]
fn the_menu_equips_a_fan_that_repeats_and_returns_the_slot() {
    let mut sim = sandbox(vec![], false);
    let body = primary(&mut sim);
    let before = mana(&sim, body);
    sim.step(held(true));
    let bolts = shots(&sim, body);
    assert_eq!(bolts.len(), 3);
    assert!(bolts.iter().all(|b| b.projectile.body.kin.vel.x > 0.0));
    assert!(bolts.iter().any(|b| b.projectile.body.kin.vel.y < 0.0));
    assert!(bolts.iter().any(|b| b.projectile.body.kin.vel.y > 0.0));
    assert!(mana(&sim, body) < before - 8.0);
    assert!(cooldown(&sim, body) > 0.0);
    let mut repeated = false;
    for _ in 0..30 { sim.step(held(false)); repeated |= !shots(&sim, body).is_empty(); }
    assert!(repeated, "holding repeats the volley");
    sim.step(AgentAction { projectile_released: true, ..AgentAction::default() });
    let before = cooldown(&sim, body);
    sim.step(held(true));
    assert!(shots(&sim, body).is_empty(), "a new press cannot bypass the cadence");
    assert!(cooldown(&sim, body) < before);
    equip(&mut sim, Item::Fireball);
    sim.step(held(true));
    sim.step(AgentAction { projectile_released: true, ..AgentAction::default() });
    assert_eq!(shots(&sim, body).len(), 1, "the body fires its fireball again");
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
}

fn damage_trace(rollback: bool) -> Vec<i32> {
    let mut options = crate::common::fixed_60hz_room_options("scroll_lab");
    if rollback { options = options.with_sync_test_rollback_settings(4, 12); }
    let mut sim = Platformer2dSimHarness::new_with_options(options).unwrap();
    sim.step_n(AgentAction::default(), 120);
    equip(&mut sim, Item::Lightning);
    let (bag, at) = {
        let world = sim.world_mut();
        world.query::<(Entity, &CharacterForm, &BodyKinematics)>().iter(world)
            .find(|(_, form, _)| form.id() == "sandbag").map(|(e, _, kin)| (e, kin.pos)).unwrap()
    };
    sim.teleport_player((at.x - 95.0, at.y));
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 30);
    sim.world_mut().insert_resource(ExtensionProfile::default());
    let mut health = vec![sim.world().get::<BodyHealth>(bag).unwrap().health.current];
    let started = std::time::Instant::now();
    for tick in 0..180 {
        sim.step(held(tick == 0));
        health.push(sim.world().get::<BodyHealth>(bag).unwrap().health.current);
    }
    let elapsed = started.elapsed();
    let profile = sim.world().resource::<ExtensionProfile>();
    let row = &profile.0["ambition::lightning/volley"];
    println!("LIGHTNING_COST rollback={rollback} ticks=180 calls={} faults={} total_us={} backend_us={} session_us={}",
        row.calls, row.faults, row.total.as_micros(), row.backend.as_micros(), elapsed.as_micros());
    sim.rollback_health().unwrap();
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    if rollback { assert!(sim.rollback_execution_stats().unwrap().lifetime_load_runs > 100); }
    health
}

#[test]
fn lightning_hits_match_the_first_run_through_rewinds() {
    let first = damage_trace(false);
    assert!(first.windows(2).filter(|p| p[1] < p[0]).count() >= 3, "repeated volleys hit: {first:?}");
    assert_eq!(damage_trace(true), first);
}

struct ScriptFile(std::path::PathBuf);
impl ScriptFile {
    fn new(arm: &str, source: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("ambition_lightning_{}_{arm}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = Self(dir.join("lightning.rhai"));
        file.edit(source);
        file
    }
    fn edit(&self, source: &str) { std::fs::write(&self.0, source).unwrap(); }
}
impl Drop for ScriptFile {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

#[test]
fn a_fault_or_missing_mana_fires_nothing() {
    let mut sim = sandbox(vec![], false);
    let body = primary(&mut sim);
    sim.world_mut().get_mut::<ActorResources>(body).unwrap()
        .level_of_mut(&ambition_platformer2d::abilities::mana::MANA).unwrap().current = 0.0;
    sim.step(held(true));
    assert!(shots(&sim, body).is_empty());
    assert_eq!(cooldown(&sim, body), 0.0);

    let throwing = SOURCE.replace("    this.state.cadence = c;",
        "    this.state.cadence = c; if user.pressed { throw \"volley fault\"; }");
    let file = ScriptFile::new("fault", &throwing);
    let mut sim = sandbox(vec![file.0.clone()], true);
    let body = primary(&mut sim);
    let before = mana(&sim, body);
    sim.step(held(true));
    assert!(shots(&sim, body).is_empty());
    assert!(mana(&sim, body) >= before);
    assert_eq!(cooldown(&sim, body), 0.0);
    let faults = sim.world().resource::<ExtensionFaults>();
    assert!(faults.recent.iter().any(|f| format!("{:?}", f.fault).contains("volley fault")));
    sim.rollback_health().unwrap();

    let competing = SOURCE.replace("entries: [#{ key: \"volley\"", "entries: [#{ key: \"drain\", phase: \"technique_execution\", trigger: #{ port: \"technique_use\", selector: \"lightning\" }, requests: [\"spend_mana\"], limit: 1 }, #{ key: \"volley\"")
        + "\nfn drain() { if this.trigger.user.pressed { this.requests.push(request(\"spend_mana\", #{ amount: 99.0 })); } }\n";
    let file = ScriptFile::new("cost", &competing);
    let mut sim = sandbox(vec![file.0.clone()], true);
    let body = primary(&mut sim);
    sim.step(held(true));
    assert!(mana(&sim, body) < 2.0, "the earlier entry paid from the shared bank");
    assert!(shots(&sim, body).is_empty(), "the later volley cannot spend the same mana");
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
}

#[test]
fn an_edit_changes_live_bolts_without_cargo_and_bad_code_keeps_the_last_program() {
    let file = ScriptFile::new("edit", SOURCE);
    let mut sim = sandbox(vec![file.0.clone()], false);
    let body = primary(&mut sim);
    sim.step(held(true));
    let initial = shots(&sim, body)[0].projectile.body.kin.vel.length();
    let generation = sim.world().resource::<ExtensionGeneration>().clone();
    file.edit("fn declare() { bad source");
    sim.step_n(AgentAction::default(), 50);
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), generation);
    sim.step(held(true));
    assert_eq!(shots(&sim, body)[0].projectile.body.kin.vel.length(), initial);
    let started = std::time::Instant::now();
    file.edit(&SOURCE.replace("const BOLT_SPEED = 680.0;", "const BOLT_SPEED = 420.0;"));
    sim.step_n(AgentAction::default(), 50);
    assert_ne!(*sim.world().resource::<ExtensionGeneration>(), generation);
    sim.step(held(true));
    let bolts = shots(&sim, body);
    assert_eq!(bolts.len(), 3);
    assert!(bolts.iter().all(|b| (b.projectile.body.kin.vel.length() - 420.0).abs() < 0.01));
    println!("LIGHTNING_EDIT edited_source_to_live_bolts_us={}", started.elapsed().as_micros());
}
