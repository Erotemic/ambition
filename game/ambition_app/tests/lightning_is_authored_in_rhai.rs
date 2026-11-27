//! Lightning uses the inventory, technique, held ray, drawn row, and rollback
//! paths: a short cone of arcs from the outstretched hand.

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
    sim.world().resource::<Messages<ambition_platformer2d::projectiles::ProjectileEmission>>()
        .iter_current_update_messages().filter(|shot| shot.owner == owner).map(|emission| &emission.request).collect()
}

fn mana(sim: &Platformer2dSimHarness, body: Entity) -> f32 {
    sim.world().get::<ActorResources>(body).unwrap()
        .level_of(&ambition_platformer2d::abilities::mana::MANA).unwrap().current
}

/// The arcs the body holds now: the rays of its lightning entry.
fn arcs(sim: &mut Platformer2dSimHarness, body: Entity) -> Vec<ambition_platformer2d::combat::strike::RayStrike> {
    let records: Vec<Entity> = sim.world().get::<ambition_platformer2d::combat::extension::HeldDamageBoxes>(body)
        .map(|held| held.0.iter().filter(|r| r.entry.contains("lightning")).map(|r| r.entity).collect())
        .unwrap_or_default();
    records.into_iter().filter_map(|e| sim.world().get::<ambition_platformer2d::combat::strike::RayStrike>(e).cloned()).collect()
}

fn crackling(sim: &Platformer2dSimHarness, body: Entity) -> bool {
    match sim.world().get::<BodyRecords>(body).and_then(|r| r.get(&CADENCE)).and_then(|r| r.get(FieldRef(0)).ok()) {
        Some(Value::Bool(value)) => *value,
        None => false,
        other => panic!("unexpected cadence: {other:?}"),
    }
}

#[test]
fn the_menu_equips_a_cone_that_crackles_while_held_and_returns_the_slot() {
    let mut sim = sandbox(vec![], false);
    let body = primary(&mut sim);
    let before = mana(&sim, body);
    sim.step(held(true));
    let cone = arcs(&mut sim, body);
    assert_eq!(cone.len(), 5, "five arcs leave the hand");
    assert!(cone.iter().all(|arc| arc.dir.x > 0.0), "toward the aim: {cone:?}");
    assert!(cone.iter().any(|arc| arc.dir.y < -0.2) && cone.iter().any(|arc| arc.dir.y > 0.2), "a cone, not a line: {cone:?}");
    assert!(cone.iter().all(|arc| arc.length <= 130.0), "short range: {cone:?}");
    assert!(mana(&sim, body) < before, "it spends mana");
    assert!(crackling(&sim, body));
    for _ in 0..30 {
        sim.step(held(false));
        assert_eq!(arcs(&mut sim, body).len(), 5, "holding keeps it crackling");
    }
    for _ in 0..10 { sim.step(AgentAction { projectile_released: true, ..AgentAction::default() }); }
    assert!(arcs(&mut sim, body).is_empty(), "letting go stops it");
    assert!(!crackling(&sim, body));
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
    sim.teleport_player((at.x - 80.0, at.y));
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
    let row = &profile.0["ambition::lightning/zap"];
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
    assert!(first.windows(2).filter(|p| p[1] < p[0]).count() >= 3, "the held arcs hit again and again: {first:?}");
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
fn lightning_can_jump_to_one_enemy_outside_the_cone() {
    use ambition_platformer2d::{
        combat::components::FeatureId,
        damage::BodyHitResolved,
        entity_catalog::placements::CharacterBrain,
    };

    let cast = |chance: f32, rollback: bool| {
        let source = SOURCE.replace("const JUMP_CHANCE = 0.35;", &format!("const JUMP_CHANCE = {chance:.1};"));
        let file = ScriptFile::new("jump", &source);
        let mut options = crate::common::fixed_60hz_room_options("switch_lab")
            .with_extension_module_files(vec![file.0.clone()]);
        if rollback {
            options = options.with_sync_test_rollback_settings(4, 12);
        }
        let mut sim = Platformer2dSimHarness::new_with_options(options).unwrap();
        sim.step_n(AgentAction::default(), 30);
        let player = primary(&mut sim);
        let position = sim.world().get::<BodyKinematics>(player).unwrap().pos;
        let placements = [("first", 72.0), ("second", 160.0), ("third", 226.0)];
        for (name, offset) in placements {
            sim.spawn_enemy_character_at(name, name, (position.x + offset, position.y),
                (14.0, 23.0), CharacterBrain::Passive, "player_robot_v3");
        }
        sim.step_n(AgentAction::default(), 30);
        equip(&mut sim, Item::Lightning);
        let bodies = {
            let world = sim.world_mut();
            let mut query = world.query::<(Entity, &FeatureId)>();
            placements.map(|(name, _)| query.iter(world)
                .find(|(_, id)| id.as_str() == name).unwrap().0)
        };
        let health = |sim: &Platformer2dSimHarness| {
            bodies.map(|body| sim.world().get::<BodyHealth>(body).unwrap().current())
        };
        let before = health(&sim);
        let mut typed = [false; 3];
        let mut receipts = sim.world().resource::<Messages<BodyHitResolved>>().get_cursor_current();
        for tick in 0..9 {
            sim.step(held(tick == 0));
            for hit in receipts.read(sim.world().resource::<Messages<BodyHitResolved>>()) {
                if hit.attacker == Some(player) && hit.resolution.damage_applied() > 0 {
                    if let Some(index) = bodies.iter().position(|body| *body == hit.body) {
                        typed[index] |= hit.damage_types.contains("ambition.electric");
                    }
                }
            }
        }
        sim.rollback_health().unwrap();
        let after = health(&sim);
        let damage = std::array::from_fn::<_, 3, _>(|index| before[index] - after[index]);
        assert_eq!(typed, damage.map(|amount| amount > 0));
        damage
    };
    let off = cast(0.0, false);
    assert!(off[0] > 0);
    assert_eq!(&off[1..], &[0, 0]);
    let on = cast(1.0, false);
    assert_eq!(on[0], off[0]);
    assert!(on[1] > 0);
    assert_eq!(on[2], 0);
    assert_eq!(cast(1.0, true), on);
}

#[test]
fn a_fault_or_missing_mana_fires_nothing() {
    let mut sim = sandbox(vec![], false);
    let body = primary(&mut sim);
    sim.world_mut().get_mut::<ActorResources>(body).unwrap()
        .level_of_mut(&ambition_platformer2d::abilities::mana::MANA).unwrap().current = 0.0;
    sim.step(held(true));
    assert!(arcs(&mut sim, body).is_empty());
    assert!(!crackling(&sim, body));

    let throwing = SOURCE.replace("    this.state.cadence = c;\n}",
        "    this.state.cadence = c; if user.pressed { throw \"zap fault\"; }\n}");
    let file = ScriptFile::new("fault", &throwing);
    let mut sim = sandbox(vec![file.0.clone()], true);
    let body = primary(&mut sim);
    let before = mana(&sim, body);
    sim.step(held(true));
    assert!(arcs(&mut sim, body).is_empty());
    assert!(mana(&sim, body) >= before);
    assert!(!crackling(&sim, body));
    let faults = sim.world().resource::<ExtensionFaults>();
    assert!(faults.recent.iter().any(|f| format!("{:?}", f.fault).contains("zap fault")));
    sim.rollback_health().unwrap();

    let competing = SOURCE.replace("entries: [#{ key: \"zap\"", "entries: [#{ key: \"drain\", phase: \"technique_execution\", trigger: #{ port: \"technique_use\", selector: \"lightning\" }, requests: [\"spend_mana\"], limit: 1 }, #{ key: \"zap\"")
        + "\nfn drain() { if this.trigger.user.pressed { this.requests.push(request(\"spend_mana\", #{ amount: this.trigger.user.mana })); } }\n";
    let file = ScriptFile::new("cost", &competing);
    let mut sim = sandbox(vec![file.0.clone()], true);
    let body = primary(&mut sim);
    sim.step(held(true));
    let left = mana(&sim, body);
    assert!(left < 1.0, "the earlier entry paid from the shared bank: {left}");
    assert!(arcs(&mut sim, body).is_empty(), "the later zap cannot spend the same mana");
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
}

#[test]
fn an_edit_changes_live_arcs_without_cargo_and_bad_code_keeps_the_last_program() {
    let file = ScriptFile::new("edit", SOURCE);
    let mut sim = sandbox(vec![file.0.clone()], false);
    let body = primary(&mut sim);
    sim.step(held(true));
    let longest = |arcs: &[ambition_platformer2d::combat::strike::RayStrike]| arcs.iter().map(|a| a.length).fold(0.0, f32::max);
    // The arcs take new paths each pulse; the shipped reach is their cap.
    let initial = longest(&arcs(&mut sim, body));
    assert!(initial > 60.0 && initial <= 130.0, "the shipped arcs: {initial}");
    let generation = sim.world().resource::<ExtensionGeneration>().clone();
    file.edit("fn declare() { bad source");
    sim.step_n(AgentAction::default(), 50);
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), generation);
    sim.step(held(true));
    let kept = longest(&arcs(&mut sim, body));
    assert!(kept > 60.0 && kept <= 130.0, "bad code keeps the last program: {kept}");
    let started = std::time::Instant::now();
    file.edit(&SOURCE.replace("const REACH = 118.0;", "const REACH = 50.0;"));
    sim.step_n(AgentAction::default(), 50);
    assert_ne!(*sim.world().resource::<ExtensionGeneration>(), generation);
    sim.step(held(true));
    let cone = arcs(&mut sim, body);
    assert_eq!(cone.len(), 5);
    assert!(longest(&cone) <= 55.0, "the edited reach: {cone:?}");
    println!("LIGHTNING_EDIT edited_source_to_live_arcs_us={}", started.elapsed().as_micros());
}
