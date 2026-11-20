//! A script equips the party's owned items through the same hand and worn
//! set as the menu, and hears what each did: `applied`, or `rejected` with
//! the refusal's code.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults};
use ambition_platformer2d::items::{Item, ItemCatalog, OwnedItems};
use ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("equip_probe", "kit", 1);

/// Each request number has one expected answer; the record keeps a bit for
/// each terminal answer heard as expected, and counts the rest.
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "equip_probe", key: "kit",
       schemas: [#{ key: "kit", fields: [
           #{ tag: 1, name: "step", kind: "u32" },
           #{ tag: 2, name: "heard", kind: "u32" },
           #{ tag: 3, name: "wrong", kind: "u32" },
           #{ tag: 4, name: "answers", kind: "u32" },
       ] }],
       entries: [
           #{ key: "tick", phase: "actor_think",
              trigger: #{ port: "story_tick", selector: "intro_wake_room" },
              writes: ["kit"], requests: ["equip", "stow"], limit: 6 },
           #{ key: "answer", phase: "actor_think",
              trigger: #{ port: "action_result", selector: "answer" },
              writes: ["kit"] },
       ],
    }
}
fn reply(n) { #{ request: n, selector: "equip_probe::kit/answer" } }
fn tick() {
    if !this.trigger.flags.contains("equip_probe_begin") { return; }
    let step = this.state.kit.step;
    if step == 0 {
        this.requests.push(request("equip", #{ body: "PLAYER", item: "HELD", reply: reply(1) }));
        this.requests.push(request("equip", #{ body: "PLAYER", item: "UNOWNED", reply: reply(2) }));
        this.requests.push(request("equip", #{ body: "TEACHER", item: "HELD", reply: reply(3) }));
        this.requests.push(request("equip", #{ body: "PLAYER", item: "not_a_catalog_item", reply: reply(4) }));
        this.requests.push(request("equip", #{ body: "PLAYER", item: "WORN", reply: reply(5) }));
        this.requests.push(request("equip", #{ body: "PLAYER", item: "PLAIN", reply: reply(6) }));
    } else if step == 1 {
        this.requests.push(request("stow", #{ body: "PLAYER", reply: reply(7) }));
        this.requests.push(request("stow", #{ body: "PLAYER", reply: reply(8) }));
    } else { return; }
    this.state.kit.step = step + 1;
}
fn answer() {
    // `submitted` comes first; the owner's answer is the terminal stage.
    if !this.trigger.terminal { return; }
    let expected = [(), "item_equipped", "not_owned", "not_a_party_body", "unknown_item",
        "item_worn", "not_equippable", "hand_stowed", "nothing_held"];
    let n = this.trigger.ticket.request;
    let applied = n == 1 || n == 5 || n == 7;
    let stage = if applied { "applied" } else { "rejected" };
    let state = this.state.kit;
    state.answers += 1;
    if this.trigger.stage == stage && this.trigger.code == expected[n] {
        if applied && this.trigger.subjects.len() != 1 { state.wrong += 1; }
        state.heard = state.heard | (1 << n);
    } else {
        state.wrong += 1;
    }
    this.state.kit = state;
}
"#;

/// The record's field at `index` (declaration order, from 0).
fn field(sim: &mut Platformer2dSimHarness, index: usize) -> Value {
    let world = sim.world_mut();
    let root = world.query_filtered::<Entity, With<RoomInstanceRoot>>().single(world).unwrap();
    world.get::<BodyRecords>(root).and_then(|records| records.get(&RECORD)).map_or(Value::U32(0), |record| record.get(FieldRef(index)).unwrap().clone())
}

fn options(files: Vec<std::path::PathBuf>) -> ambition_app::Platformer2dSimHarnessOptions {
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    crate::common::fixed_60hz_room_options("intro_wake_room")
        .with_save(save)
        .with_extension_module_files(files)
        .with_sync_test_rollback_settings(4, 12)
}

#[test]
fn equips_and_stows_answer_with_the_hand_and_worn_set_through_replay() {
    // Learn the identities the script names.
    let mut sim = Platformer2dSimHarness::new_with_options(options(Vec::new())).unwrap();
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let player_id = world.query_filtered::<&SimId, PrimaryPlayerOnly>().single(world).unwrap().as_str().to_owned();
    let teacher_id = world
        .query::<(&CharacterForm, &SimId)>()
        .iter(world)
        .find(|(form, _)| form.id() == "npc_creator")
        .map(|(_, id)| id.as_str().to_owned())
        .expect("the Creator is in the wake room");
    let catalog = world.resource::<ItemCatalog>().clone();
    let id = |item| catalog.dialog_id(item).unwrap().to_owned();
    let source = SOURCE
        .replace("PLAYER", &player_id)
        .replace("TEACHER", &teacher_id)
        .replace("\"HELD\"", &format!("{:?}", id(Item::GunSword)))
        .replace("\"UNOWNED\"", &format!("{:?}", id(Item::PortalGun)))
        .replace("\"WORN\"", &format!("{:?}", id(Item::Dash)))
        .replace("\"PLAIN\"", &format!("{:?}", id(Item::HealthCell)));
    drop(sim);
    let dir = std::env::temp_dir().join(format!("ambition_equipment_outcomes_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("kit.rhai");
    std::fs::write(&file, source).unwrap();

    let mut sim = Platformer2dSimHarness::new_with_options(options(vec![file])).unwrap();
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let mut bag = OwnedItems::default();
    for item in [Item::GunSword, Item::Dash, Item::HealthCell] {
        bag.grant(&catalog, item, 1);
    }
    world.insert_resource(bag);
    world
        .resource_mut::<ambition_platformer2d::session::AmbitionGameSave>()
        .data_mut()
        .set_flag("equip_probe_begin", true);
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 30);
    sim.rollback_health().unwrap();
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    assert_eq!(field(&mut sim, 2), Value::U32(0), "every answer is the expected stage and code");
    assert_eq!(field(&mut sim, 1), Value::U32(0b1_1111_1110), "all eight requests were answered");
    assert_eq!(field(&mut sim, 3), Value::U32(8), "each request was answered once through the replays");
    let world = sim.world_mut();
    let (worn, held) = world
        .query_filtered::<(Option<&WornEquipment>, Option<&ambition_platformer2d::combat::held_items::HeldItem>), PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    assert!(held.is_none(), "the stow emptied the hand");
    assert!(worn.is_some_and(|worn| worn.rows.iter().any(|row| row.id == "dash")), "the dash is worn");
    let _ = std::fs::remove_dir_all(dir);
}
