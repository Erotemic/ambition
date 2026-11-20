//! A script retires what it created through the creation owner, and a
//! subscriber hears the retirement once; what it did not create is refused.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    combat::extension::status::BodyStatuses,
    extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}},
    platformer::markers::PrimaryPlayerOnly,
};
use bevy::prelude::*;

const SOURCE: &str = r#"
fn declare() {
    #{provider: "life_probe", key: "keeper", schemas: [
        #{key: "life", fields: [
            #{tag: 1, name: "step", kind: "u32"},
            #{tag: 2, name: "me", kind: "opt object"},
            #{tag: 3, name: "other", kind: "opt object"},
            #{tag: 4, name: "children", kind: "seq 4 object"},
            #{tag: 5, name: "attached", kind: "bool"},
            #{tag: 6, name: "answers", kind: "text 256"},
        ]},
        #{key: "watch", fields: [
            #{tag: 1, name: "heard", kind: "u32"},
            #{tag: 2, name: "gone", kind: "opt object"},
        ]}
    ], entries: [
        #{key: "tick", phase: "technique_execution", trigger: #{port: "status_tick", selector: "life_probe.go"},
          writes: ["life"], requests: ["body_query", "create_actors", "retire_creation"], limit: 3},
        #{key: "found", phase: "actor_think", trigger: #{port: "action_result", selector: "found"},
          reads: ["body_query_result"], writes: ["life"]},
        #{key: "made", phase: "actor_think", trigger: #{port: "action_result", selector: "made"},
          writes: ["life"], requests: ["attach_behaviors"], limit: 1},
        #{key: "attached", phase: "actor_think", trigger: #{port: "action_result", selector: "attached"}, writes: ["life"]},
        #{key: "retired", phase: "actor_think", trigger: #{port: "action_result", selector: "retired"}, writes: ["life"]},
        #{key: "gone", phase: "actor_think", trigger: #{port: "object_retired", selector: "retired"}, writes: ["watch"]},
    ]}
}
fn bag(key, at) { #{key: key, character: "practice_sandbag", at: at, allegiance: "neutral"} }
fn retire(n, object) {
    request("retire_creation", #{object: object, reply: #{request: n, selector: "life_probe::keeper/retired"}})
}
fn tick() {
    let life = this.state.life;
    life.step += 1;
    let p = this.trigger.position;
    if life.step == 1 {
        this.requests.push(request("body_query", #{reply: #{request: 1, selector: "life_probe::keeper/found"},
            origin: p, radius: 4000.0, limit: 6, include_self: true}));
    }
    if life.step == 3 {
        this.requests.push(request("create_actors", #{reply: #{request: 1, selector: "life_probe::keeper/made"},
            actors: [bag("a", [p[0] + 200.0, p[1] - 300.0]), bag("b", [p[0] + 320.0, p[1] - 300.0])]}));
    }
    if life.step == 20 && life.attached {
        this.requests.push(retire(1, life.children[0]));
        this.requests.push(retire(2, life.other));
    }
    if life.step == 30 { this.requests.push(retire(3, life.children[0])); }
    this.state.life = life;
}
fn found() {
    if this.trigger.stage != "applied" { return; }
    for target in this.observe.body_query_result.targets {
        if target.driven { this.state.life.me = target.object; }
        else if this.state.life.other == () { this.state.life.other = target.object; }
    }
}
fn made() {
    if !this.trigger.terminal { return; }
    if this.trigger.code != "actors_created" { throw this.trigger.code; }
    let children = this.trigger.subjects.map(|subject| subject.object);
    this.state.life.children = children;
    this.requests.push(request("attach_behaviors", #{target: this.state.life.me, bindings: [#{key: "watch",
        references: #{a: children[0], b: children[1]}, methods: [#{name: "gone", entry: "life_probe::keeper/gone"}],
        subscriptions: [#{key: "a", source: "a", method: "gone"}, #{key: "b", source: "b", method: "gone"}]}],
        reply: #{request: 1, selector: "life_probe::keeper/attached"}}));
}
fn attached() {
    if this.trigger.stage == "rejected" { throw this.trigger.message; }
    if this.trigger.terminal { this.state.life.attached = true; }
}
fn retired() {
    if !this.trigger.terminal { return; }
    this.state.life.answers += `${this.trigger.ticket.request}:${this.trigger.code};`;
}
fn gone() {
    this.state.watch.heard += 1;
    this.state.watch.gone = this.trigger.object;
}
"#;

/// ⭐ The creator retires its own sandbag and its subscriber hears it once;
/// the other sandbag stays and is not heard; an authored body is not the
/// script's to retire, and a retired one is no longer there. Sync-test host,
/// so the retirement and its hearing survive the rewinds.
#[test]
fn a_creator_retires_its_creation_and_a_subscriber_hears_it_once() {
    let dir = std::env::temp_dir().join(format!("ambition_retirement_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("keeper.rhai");
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
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    world.entity_mut(player).insert(BodyStatuses(vec!["life_probe.go".into()]));
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 45);
    sim.rollback_health().unwrap();
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent.first());
    let life = sim.world().get::<BodyRecords>(player).unwrap().get(&SchemaKey::new("life_probe", "life", 1)).cloned().unwrap();
    assert_eq!(life.get(FieldRef(5)).unwrap(),
        &Value::Text("1:creation_retired;2:not_your_creation;3:unknown_object;".into()),
        "the creation is retired, an authored body is refused, and the retired one is gone");
    let Value::Seq(children) = life.get(FieldRef(3)).unwrap().clone() else { panic!("children") };
    let world = sim.world_mut();
    let heard: Vec<_> = world.query::<&BodyRecords>().iter(world)
        .filter_map(|records| records.get(&SchemaKey::new("life_probe", "watch", 1)).cloned()).collect();
    assert_eq!(heard.len(), 1, "one watch instance, which heard a retirement");
    let watch = &heard[0];
    assert_eq!(watch.get(FieldRef(0)).unwrap(), &Value::U32(1), "one retirement, heard once");
    assert_eq!(watch.get(FieldRef(1)).unwrap(), &Value::some(children[0].clone()), "the fact names the retired creation");
    use ambition_platformer2d::platformer::lifecycle::{ActiveSessionScope, resolve_object};
    let session = sim.world().resource::<ActiveSessionScope>().current().unwrap();
    assert!(resolve_object(sim.world(), session, children[0].as_object().unwrap()).is_err(), "the retired sandbag is gone");
    assert!(resolve_object(sim.world(), session, children[1].as_object().unwrap()).is_ok(), "the other sandbag stays");
    let _ = std::fs::remove_dir_all(dir);
}

/// ⭐ A ROOM'S RETIREMENT IS THE SAME FACT. The player subscribes to a sandbag
/// it made in `blink_run` and walks out; the room is taken down behind it,
/// and the player, now in the next room, hears that the sandbag is gone.
#[test]
fn leaving_a_room_retires_what_it_held_and_a_subscriber_outside_hears_it() {
    let source = SOURCE.replace("if life.step == 20 && life.attached", "if false")
        .replace("if life.step == 30 {", "if false {");
    let path = std::env::temp_dir().join(format!("ambition_room_retirement_{}.rhai", std::process::id()));
    std::fs::write(&path, source).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("blink_run").with_extension_module_files(vec![path.clone()])).unwrap();
    crate::common::doors_cross_at_once(&mut sim);
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    world.entity_mut(player).insert(BodyStatuses(vec!["life_probe.go".into()]));
    sim.step_n(AgentAction::default(), 20);
    let life = sim.world().get::<BodyRecords>(player).unwrap().get(&SchemaKey::new("life_probe", "life", 1)).cloned().unwrap();
    assert_eq!(life.get(FieldRef(4)).unwrap(), &Value::Bool(true), "the premise: the watch is attached");
    let Value::Seq(children) = life.get(FieldRef(3)).unwrap().clone() else { panic!("children") };
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "portal_bridge"), "portal_bridge");
    sim.step_n(AgentAction::default(), 20);
    let world = sim.world_mut();
    let heard: Vec<_> = world.query::<&BodyRecords>().iter(world)
        .filter_map(|records| records.get(&SchemaKey::new("life_probe", "watch", 1)).cloned()).collect();
    assert_eq!(heard.len(), 1, "one watch instance, carried with the player");
    assert_eq!(heard[0].get(FieldRef(0)).unwrap(), &Value::U32(2), "both sandbags left with their room, each heard once");
    assert!(children.iter().any(|child| heard[0].get(FieldRef(1)).unwrap() == &Value::some(child.clone())), "the fact names a sandbag");
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    let _ = std::fs::remove_file(path);
}
