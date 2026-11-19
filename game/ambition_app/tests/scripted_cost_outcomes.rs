//! An unpaid invocation reports correlated rejection without a native birth.

#![cfg(feature = "rl_sim")]
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{characters::actor::CharacterForm,
    combat::extension::status::BodyStatuses,
    extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}},
    platformer::{markers::PrimaryPlayerOnly, sim_id::SimIdCounter}};
use bevy::prelude::*;

const SOURCE: &str = r#"
fn declare() {
    #{provider: "cost_probe", key: "factory", schemas: [#{key: "memory", fields: [
        #{tag: 1, name: "sent", kind: "bool"},
        #{tag: 2, name: "rejected", kind: "u32"},
        #{tag: 3, name: "code", kind: "text 64"},
    ]}], entries: [
        #{key: "build", phase: "technique_execution", trigger: #{port: "status_tick", selector: "cost_probe.build"},
          writes: ["memory"], requests: ["spend_mana", "create_actors"], limit: 2},
        #{key: "result", phase: "actor_think", trigger: #{port: "action_result", selector: "result"}, writes: ["memory"]},
    ]}
}
fn build() {
    if this.state.memory.sent { return; }
    this.state.memory.sent = true;
    this.requests.push(request("spend_mana", #{amount: 1000000.0}));
    this.requests.push(request("create_actors", #{
        reply: #{request: 1, selector: "cost_probe::factory/result"},
        actors: [#{key: "unpaid", character: "practice_sandbag", at: this.trigger.position}]
    }));
}
fn result() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage != "rejected" { throw "unpaid action was accepted"; }
    this.state.memory.rejected += 1;
    this.state.memory.code = this.trigger.code;
}
"#;

#[test]
fn an_unaffordable_factory_reports_rejection_for_both_controllers_and_replays() {
    let path = std::env::temp_dir().join(format!("ambition_cost_outcome_{}.rhai", std::process::id()));
    std::fs::write(&path, SOURCE).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room").with_save(save)
            .with_extension_module_files(vec![path.clone()]).with_sync_test_rollback_settings(4, 12)).unwrap();
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    let teacher = world.query::<(Entity, &CharacterForm)>().iter(world)
        .find(|(_, form)| form.id() == "npc_creator").unwrap().0;
    let counters: Vec<_> = [player, teacher].iter().map(|body| world.get::<SimIdCounter>(*body).unwrap().0).collect();
    let sandbags = world.query::<&CharacterForm>().iter(world).filter(|form| form.id() == "practice_sandbag").count();
    for body in [player, teacher] { world.entity_mut(body).insert(BodyStatuses(vec!["cost_probe.build".into()])); }
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 24);
    for (index, body) in [player, teacher].into_iter().enumerate() {
        let record = sim.world().get::<BodyRecords>(body).unwrap().get(&SchemaKey::new("cost_probe", "memory", 1)).unwrap();
        assert_eq!(record.get(FieldRef(0)).unwrap(), &Value::Bool(true));
        assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1));
        assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::Text("invocation_cost_refused".into()));
        assert_eq!(sim.world().get::<SimIdCounter>(body).unwrap().0, counters[index]);
    }
    let world = sim.world_mut();
    assert_eq!(world.query::<&CharacterForm>().iter(world).filter(|form| form.id() == "practice_sandbag").count(), sandbags);
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_file(path);
}
