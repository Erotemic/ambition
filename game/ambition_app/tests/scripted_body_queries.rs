//! Body queries return native facts to scripts for every controller.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    characters::actor::CharacterForm,
    combat::{extension::status::BodyStatuses, targeting::MatchTeam},
    extension::{
        BodyRecords, ExtensionFaults,
        sdk::{FieldRef, SchemaKey, Value},
    },
    platformer::{markers::PrimaryPlayerOnly, sim_id::SimId},
};
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("query_probe", "query", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "query_probe", key: "targets",
       schemas: [#{ key: "query", fields: [
           #{ tag: 1, name: "step", kind: "u32" },
           #{ tag: 2, name: "target", kind: "opt body" },
           #{ tag: 3, name: "accepted", kind: "u32" },
           #{ tag: 4, name: "rejected", kind: "u32" },
       ] }],
       entries: [
           #{ key: "search", phase: "technique_execution",
              trigger: #{ port: "status_tick", selector: "query_probe.search" },
              writes: ["query"], requests: ["body_query"], limit: 1 },
           #{ key: "follow", phase: "technique_execution",
              trigger: #{ port: "status_tick", selector: "query_probe.follow" },
              writes: ["query"], requests: ["body_query"], limit: 1 },
           #{ key: "result", phase: "actor_think",
              trigger: #{ port: "action_result", selector: "result" },
              reads: ["body_query_result"], writes: ["query"] },
       ],
    }
}
fn search() {
    if this.state.query.step != 0 { return; }
    this.requests.push(request("body_query", #{
        reply: #{ request: 0, selector: "query_probe::targets/result" },
        origin: this.trigger.position, radius: 1000.0, limit: 1, relation: "foe",
    }));
    this.state.query.step = 1;
}
fn follow() {
    if this.state.query.step != 2 { return; }
    this.requests.push(request("body_query", #{
        reply: #{ request: 0, selector: "query_probe::targets/result" },
        origin: this.trigger.position, radius: 1000.0, limit: 1, relation: "foe",
        identities: [this.state.query.target], require_all: true,
    }));
    this.state.query.step = 3;
}
fn result() {
    if this.trigger.stage == "applied" {
        let found = this.observe.body_query_result;
        if found.ticket != this.trigger.ticket { throw "the reply names another query"; }
        if found.phase != "technique_execution" { throw "the query lost its read cut"; }
        if found.targets.len() != 1 { throw "the query did not honor its bound"; }
        if found.targets[0].relation != "foe" { throw "the query changed the relationship"; }
        this.state.query.target = found.targets[0].id;
        this.state.query.accepted += 1;
        this.state.query.step = 2;
    }
    if this.trigger.stage == "rejected" {
        if this.trigger.code != "target_missing" { throw this.trigger.code; }
        this.state.query.rejected += 1;
        this.state.query.step = 4;
    }
}
"#;

#[test]
fn controlled_and_autonomous_bodies_query_then_refuse_a_target_that_is_no_longer_a_foe() {
    let dir = std::env::temp_dir().join(format!("ambition_body_queries_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("targets.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room")
            .with_save(save)
            .with_extension_module_files(vec![file])
            .with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    let world = sim.world_mut();
    let player = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    let teacher = world
        .query::<(Entity, &CharacterForm)>()
        .iter(world)
        .find(|(_, form)| form.id() == "npc_creator")
        .unwrap()
        .0;
    for (body, team) in [(player, "red"), (teacher, "blue")] {
        world.entity_mut(body).insert((
            MatchTeam::new(team),
            BodyStatuses(vec!["query_probe.search".into()]),
        ));
    }
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 20);
    for (body, target) in [(player, teacher), (teacher, player)] {
        let world = sim.world();
        let record = world
            .get::<BodyRecords>(body)
            .unwrap()
            .get(&RECORD)
            .unwrap();
        assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U32(1));
        assert_eq!(
            record.get(FieldRef(1)).unwrap(),
            &Value::Opt(Some(Box::new(Value::Ref(
                world.get::<SimId>(target).unwrap().as_str().into()
            ))))
        );
    }
    let world = sim.world_mut();
    world.entity_mut(teacher).insert(MatchTeam::new("red"));
    for body in [player, teacher] {
        world.entity_mut(body).insert(BodyStatuses(vec![
            "query_probe.search".into(),
            "query_probe.follow".into(),
        ]));
    }
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 16);
    for body in [player, teacher] {
        let record = sim
            .world()
            .get::<BodyRecords>(body)
            .unwrap()
            .get(&RECORD)
            .unwrap();
        assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U32(1));
        assert_eq!(record.get(FieldRef(3)).unwrap(), &Value::U32(1));
    }
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}
