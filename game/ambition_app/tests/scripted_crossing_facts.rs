//! A crossing is a fact a script can subscribe to: a method on the crossing
//! body hears each crossing once, naming the room left and the room entered.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{combat::extension::status::BodyStatuses,
    extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}},
    platformer::markers::PrimaryPlayerOnly};
use bevy::prelude::*;

const SOURCE: &str = r#"
fn declare() {
    #{provider: "cross_probe", key: "watch", schemas: [
        #{key: "root", fields: [
            #{tag: 1, name: "started", kind: "bool"},
            #{tag: 2, name: "attached", kind: "u32"},
        ]},
        #{key: "traveler", fields: [
            #{tag: 1, name: "crossings", kind: "u32"},
            #{tag: 2, name: "route", kind: "text 128"},
            #{tag: 3, name: "own", kind: "u32"},
        ]}
    ], entries: [
        #{key: "start", phase: "technique_execution", trigger: #{port: "status_tick", selector: "cross_probe.start"},
          writes: ["root"], requests: ["body_query"], limit: 1},
        #{key: "found", phase: "actor_think", trigger: #{port: "action_result", selector: "found"},
          reads: ["body_query_result"], writes: ["root"], requests: ["attach_behaviors"], limit: 1},
        #{key: "attached", phase: "actor_think", trigger: #{port: "action_result", selector: "attached"}, writes: ["root"]},
        #{key: "crossed", phase: "actor_think", trigger: #{port: "object_crossed", selector: "crossed"},
          reads: ["behavior"], writes: ["traveler"]},
        #{key: "refused", phase: "actor_think", trigger: #{port: "object_crossing_refused", selector: "crossing_refused"},
          writes: ["traveler"]},
    ]}
}
fn start() {
    if this.state.root.started { return; }
    this.state.root.started = true;
    this.requests.push(request("body_query", #{reply: #{request: 0, selector: "cross_probe::watch/found"},
        origin: this.trigger.position, radius: 64.0, limit: 4, include_self: true}));
}
fn found() {
    if this.trigger.stage != "applied" { return; }
    let me = ();
    for target in this.observe.body_query_result.targets {
        if target.driven { me = target.object; }
    }
    if me == () { throw "the query found no driven body"; }
    this.requests.push(request("attach_behaviors", #{target: me, bindings: [#{key: "travel",
        references: #{me: me}, methods: [#{name: "crossed", entry: "cross_probe::watch/crossed"},
            #{name: "refused", entry: "cross_probe::watch/refused"}],
        subscriptions: [#{key: "me", source: "me", method: "crossed"}, #{key: "me_refused", source: "me", method: "refused"}]}],
        reply: #{request: 0, selector: "cross_probe::watch/attached"}}));
}
fn attached() {
    if this.trigger.stage == "rejected" { throw this.trigger.message; }
    if this.trigger.terminal { this.state.root.attached += 1; }
}
fn crossed() {
    let state = this.state.traveler;
    state.crossings += 1;
    state.route += this.trigger.from + ">" + this.trigger.to + ";";
    if this.trigger.object == this.observe.behavior.object { state.own += 1; }
    this.state.traveler = state;
}
fn refused() {
    this.state.traveler.route += "refused:" + this.trigger.to + ":" + this.trigger.code + ";";
}
"#;

/// ⭐ The room owner commits the crossing once the destination published, on
/// either host: here the sync-test host, whose commit runs outside the rewound
/// schedule and rebases, so the row it queues is in the new baseline and every
/// re-simulation after it delivers it once.
#[test]
fn a_method_on_the_crossing_body_hears_each_crossing_once_through_replay() {
    hears_each_crossing_once(true);
}

/// The eager host commits inside the tick, in the room transition's apply set.
#[test]
fn a_method_on_the_crossing_body_hears_each_crossing_once_on_the_eager_host() {
    hears_each_crossing_once(false);
}

fn hears_each_crossing_once(rollback: bool) {
    let path = std::env::temp_dir().join(format!("ambition_cross_probe_{}_{rollback}.rhai", std::process::id()));
    std::fs::write(&path, SOURCE).unwrap();
    let options = crate::common::fixed_60hz_room_options("blink_run").with_extension_module_files(vec![path.clone()]);
    let options = if rollback { options.with_sync_test_rollback_settings(4, 12) } else { options };
    let mut sim = Platformer2dSimHarness::new_with_options(options).unwrap();
    crate::common::doors_cross_at_once(&mut sim);
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    world.entity_mut(player).insert(BodyStatuses(vec!["cross_probe.start".into()]));
    if rollback { sim.rebase_rollback_history().unwrap(); }
    sim.step_n(AgentAction::default(), 20);
    let root = SchemaKey::new("cross_probe", "root", 1);
    let record = sim.world().get::<BodyRecords>(player).unwrap().get(&root).cloned().expect("the probe started");
    assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1), "the behavior is attached to the player");

    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "portal_bridge"), "portal_bridge");
    sim.step_n(AgentAction::default(), 20);
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "blink_run"), "blink_run");
    sim.step_n(AgentAction::default(), 30);

    let traveler = SchemaKey::new("cross_probe", "traveler", 1);
    let world = sim.world_mut();
    let heard: Vec<_> = world.query::<&BodyRecords>().iter(world).filter_map(|records| records.get(&traveler).cloned()).collect();
    assert_eq!(heard.len(), 1, "one behavior instance, carried with the player");
    assert_eq!(heard[0].get(FieldRef(0)).unwrap(), &Value::U32(2), "two crossings, each heard once");
    assert_eq!(heard[0].get(FieldRef(1)).unwrap(), &Value::Text("blink_run>portal_bridge;portal_bridge>blink_run;".into()));
    assert_eq!(heard[0].get(FieldRef(2)).unwrap(), &Value::U32(2), "each fact names the body it subscribed to");
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    if rollback { sim.rollback_health().unwrap(); }
    let _ = std::fs::remove_file(path);
}

/// ⭐ A crossing the room owner refuses after it began is a fact too: the body
/// that could not cross hears where it was going and why, and is still where
/// it stood. Here the body lacks its combat state at the commit, so staging
/// refuses it; with the state back, the next crossing goes through.
#[test]
fn a_refused_crossing_is_heard_and_the_body_stays_where_it_was() {
    refused_crossing(false);
}

/// The confirmed host refuses in its own commit, outside the rewound schedule.
#[test]
fn a_refused_crossing_is_heard_once_through_replay() {
    refused_crossing(true);
}

fn refused_crossing(rollback: bool) {
    use ambition_platformer2d::characters::actor::BodyCombat;
    use ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit;
    let path = std::env::temp_dir().join(format!("ambition_cross_refused_{}_{rollback}.rhai", std::process::id()));
    std::fs::write(&path, SOURCE).unwrap();
    let options = crate::common::fixed_60hz_room_options("blink_run").with_extension_module_files(vec![path.clone()]);
    let options = if rollback { options.with_sync_test_rollback_settings(4, 12) } else { options };
    let mut sim = Platformer2dSimHarness::new_with_options(options).unwrap();
    crate::common::doors_cross_at_once(&mut sim);
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    world.entity_mut(player).insert(BodyStatuses(vec!["cross_probe.start".into()]));
    if rollback { sim.rebase_rollback_history().unwrap(); }
    sim.step_n(AgentAction::default(), 20);

    // Ask to cross, and take the combat state away while the crossing waits.
    let door = crate::common::door_to(&mut sim, "portal_bridge");
    use ambition_platformer2d::engine_core::AabbExt as _;
    let center = door.aabb.center();
    sim.teleport_player((center.x, center.y));
    let mut taken = None;
    for frame in 0..60 {
        sim.step(AgentAction { interact: frame % 2 == 0, interact_held: frame % 2 == 0, ..crate::common::base() });
        if sim.world().resource::<PendingLifecycleCommit>().pending.is_some() {
            taken = sim.world_mut().entity_mut(player).take::<BodyCombat>();
            break;
        }
    }
    let combat = taken.expect("the premise: the door admitted a crossing");
    // The confirmed host commits once the request's frame is confirmed.
    for _ in 0..120 {
        if sim.world().resource::<PendingLifecycleCommit>().pending.is_none() { break; }
        sim.step(AgentAction::default());
    }
    sim.step_n(AgentAction::default(), 10);
    assert!(sim.world().resource::<PendingLifecycleCommit>().pending.is_none(), "a refused crossing does not come back");
    assert_eq!(sim.observation().active_room, "blink_run", "the refused body stayed where it stood");
    sim.world_mut().entity_mut(player).insert(combat);
    if rollback { sim.rebase_rollback_history().unwrap(); }
    sim.step_n(AgentAction::default(), 10);
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "portal_bridge"), "portal_bridge");
    sim.step_n(AgentAction::default(), 20);

    let traveler = SchemaKey::new("cross_probe", "traveler", 1);
    let world = sim.world_mut();
    let heard: Vec<_> = world.query::<&BodyRecords>().iter(world).filter_map(|records| records.get(&traveler).cloned()).collect();
    assert_eq!(heard.len(), 1);
    assert_eq!(heard[0].get(FieldRef(1)).unwrap(),
        &Value::Text("refused:portal_bridge:body_cannot_cross;blink_run>portal_bridge;".into()));
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    if rollback { sim.rollback_health().unwrap(); }
    let _ = std::fs::remove_file(path);
}
