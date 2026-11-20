//! A take and a release answer the script that asked with what the item owner
//! did: `applied`, or `rejected` with the refusal's code.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults};
use ambition_platformer2d::held_items::{GroundItem, ItemCustody};
use ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("custody_probe", "custody", 1);

/// Each request number has one expected answer; the record keeps a bit for
/// each one heard as expected, and a count of the rest.
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "custody_probe", key: "hands",
       schemas: [#{ key: "custody", fields: [
           #{ tag: 1, name: "step", kind: "u32" },
           #{ tag: 2, name: "heard", kind: "u32" },
           #{ tag: 3, name: "wrong", kind: "u32" },
           #{ tag: 4, name: "answers", kind: "u32" },
       ] }],
       entries: [
           #{ key: "tick", phase: "actor_think",
              trigger: #{ port: "story_tick", selector: "primeval_trailhead" },
              writes: ["custody"], requests: ["take", "release"], limit: 4 },
           #{ key: "answer", phase: "actor_think",
              trigger: #{ port: "action_result", selector: "answer" },
              writes: ["custody"] },
       ],
    }
}
fn reply(n) { #{ request: n, selector: "custody_probe::hands/answer" } }
fn tick() {
    if !this.trigger.flags.contains("custody_probe_begin") { return; }
    let step = this.state.custody.step;
    if step == 0 {
        this.requests.push(request("take", #{ body: "BRAND", item: "FAR", reply: reply(1) }));
        this.requests.push(request("take", #{ body: "BRAND", item: "NEAR", reply: reply(2) }));
        this.requests.push(request("take", #{ body: "KIWI", item: "NEAR", reply: reply(3) }));
        this.requests.push(request("take", #{ body: "placement:nobody", item: "NEAR", reply: reply(4) }));
    } else if step == 1 {
        this.requests.push(request("release", #{ body: "BRAND", thrown: false, reply: reply(5) }));
        this.requests.push(request("release", #{ body: "KIWI", thrown: false, reply: reply(6) }));
    } else { return; }
    this.state.custody.step = step + 1;
}
fn answer() {
    // `submitted` comes first; the item owner's answer is the terminal stage.
    if !this.trigger.terminal { return; }
    let expected = [(), "out_of_reach", "item_taken", "no_such_item", "no_such_body", "item_released", "nothing_held"];
    let n = this.trigger.ticket.request;
    let applied = n == 2 || n == 5;
    let stage = if applied { "applied" } else { "rejected" };
    let state = this.state.custody;
    state.answers += 1;
    if this.trigger.stage == stage && this.trigger.code == expected[n] {
        if n == 2 && this.trigger.subjects.len() != 2 { state.wrong += 1; }
        state.heard = state.heard | (1 << n);
    } else {
        state.wrong += 1;
    }
    this.state.custody = state;
}
"#;

/// The record's field at `index` (declaration order, from 0).
fn field(sim: &mut Platformer2dSimHarness, index: usize) -> Value {
    let world = sim.world_mut();
    let root = world.query_filtered::<Entity, With<RoomInstanceRoot>>().single(world).unwrap();
    world.get::<BodyRecords>(root).and_then(|records| records.get(&RECORD)).map_or(Value::U32(0), |record| record.get(FieldRef(index)).unwrap().clone())
}

#[test]
fn takes_and_releases_answer_with_the_item_owners_outcome_through_replay() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("primeval_trailhead").with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let named = |world: &mut World, form: &str| {
        world
            .query::<(Entity, &CharacterForm, &SimId)>()
            .iter(world)
            .filter(|(_, f, _)| f.id() == form)
            .map(|(entity, _, id)| (entity, id.as_str().to_owned()))
            .min_by(|a, b| a.1.cmp(&b.1))
            .unwrap_or_else(|| panic!("the trailhead has a {form}"))
    };
    let (ranger, ranger_id) = named(world, "npc_primeval_brandvold");
    let (_, kiwi_id) = named(world, "kiwi");
    // The lying coconuts, lowest first: the near one is on the ground.
    let mut coconuts: Vec<(f32, Vec2, String)> = world
        .query::<(&GroundItem, &BodyKinematics, &ItemCustody, &SimId)>()
        .iter(world)
        .filter(|(_, _, custody, _)| matches!(custody, ItemCustody::InWorld))
        .map(|(_, kin, _, id)| (kin.pos.y, kin.pos, id.as_str().to_owned()))
        .collect();
    coconuts.sort_by(|a, b| b.0.total_cmp(&a.0));
    assert!(coconuts.len() >= 2, "two coconuts lie at the trailhead: {coconuts:?}");
    let (near_at, near_id) = (coconuts[0].1, coconuts[0].2.clone());
    let far_id = coconuts.iter().find(|c| (c.1 - near_at).length() > 200.0).expect("a coconut far from the near one").2.clone();
    // The ranger stands on the near coconut.
    world.get_mut::<BodyKinematics>(ranger).unwrap().pos = near_at;
    let source = SOURCE
        .replace("BRAND", &ranger_id)
        .replace("KIWI", &kiwi_id)
        .replace("\"NEAR\"", &format!("{near_id:?}"))
        .replace("\"FAR\"", &format!("{far_id:?}"));
    let dir = std::env::temp_dir().join(format!("ambition_custody_outcomes_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("hands.rhai");
    std::fs::write(&file, source).unwrap();
    drop(sim);

    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("primeval_trailhead")
            .with_extension_module_files(vec![file])
            .with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let ranger = world
        .query::<(Entity, &SimId)>()
        .iter(world)
        .find(|(_, id)| id.as_str() == ranger_id)
        .unwrap()
        .0;
    world.get_mut::<BodyKinematics>(ranger).unwrap().pos = near_at;
    world
        .resource_mut::<ambition_platformer2d::session::AmbitionGameSave>()
        .data_mut()
        .set_flag("custody_probe_begin", true);
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 30);
    sim.rollback_health().unwrap();
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    assert_eq!(field(&mut sim, 2), Value::U32(0), "every answer is the expected stage and code");
    assert_eq!(field(&mut sim, 1), Value::U32(0b111_1110), "all six requests were answered");
    assert_eq!(field(&mut sim, 3), Value::U32(6), "each request was answered once through the replays");
    let _ = std::fs::remove_dir_all(dir);
}
