//! Reusable bindings retain private state while native actions use their subject.

#![cfg(feature = "rl_sim")]
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{characters::actor::{BodyWallet, CharacterForm},
    combat::extension::status::BodyStatuses,
    extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}},
    platformer::markers::PrimaryPlayerOnly};
use bevy::prelude::*;

const SOURCE: &str = r#"
fn declare() {
    #{provider: "objects_probe", key: "composition", schemas: [
        #{key: "root", fields: [
            #{tag: 1, name: "started", kind: "bool"},
            #{tag: 2, name: "completed", kind: "u32"},
            #{tag: 3, name: "refused", kind: "u32"},
        ]},
        #{key: "worker", fields: [
            #{tag: 1, name: "requests", kind: "u32"},
            #{tag: 2, name: "pending", kind: "u64"},
            #{tag: 3, name: "applied", kind: "u32"},
            #{tag: 4, name: "refused", kind: "u32"},
            #{tag: 5, name: "busy", kind: "bool"},
            #{tag: 6, name: "completions", kind: "u32"},
        ]}
    ], entries: [
        #{key: "start", phase: "technique_execution", trigger: #{port: "status_tick", selector: "objects_probe.start"},
          writes: ["root"], requests: ["body_query"], limit: 1},
        #{key: "found", phase: "actor_think", trigger: #{port: "action_result", selector: "found"},
          reads: ["body_query_result"], requests: ["attach_behaviors", "call_object"], limit: 6},
        #{key: "attached", phase: "actor_think", trigger: #{port: "action_result", selector: "attached"}},
        #{key: "done", phase: "actor_think", trigger: #{port: "action_result", selector: "done"}, writes: ["root"]},
        #{key: "work", phase: "actor_think", trigger: #{port: "object_method", selector: "work"},
          reads: ["behavior"], writes: ["worker"], requests: ["inventory_exchange", "complete_object_call"], limit: 1},
        #{key: "ack", phase: "actor_think", trigger: #{port: "action_result", selector: "ack"}},
        #{key: "trade", phase: "actor_think", trigger: #{port: "action_result", selector: "trade"},
          reads: ["behavior"], writes: ["worker"], requests: ["complete_object_call"], limit: 1},
    ]}
}
fn start() {
    if this.state.root.started { return; }
    this.state.root.started = true;
    this.requests.push(request("body_query", #{reply: #{request: 0, selector: "objects_probe::composition/found"},
        origin: this.trigger.position, radius: 1.0, limit: 1, include_self: true}));
}
fn found() {
    if this.trigger.stage != "applied" { return; }
    let target = this.observe.body_query_result.targets[0].object;
    let bindings = [];
    for pair in [["credit", 0, 2, false], ["debit", 99, 0, false], ["fault", 0, 99, true]] {
        bindings.push(#{key: pair[0], params: #{pay: pair[1], receive: pair[2], fail: pair[3]},
            methods: [#{name: "use", entry: "objects_probe::composition/work"}]});
    }
    this.requests.push(request("attach_behaviors", #{target: target, bindings: bindings,
        reply: #{request: 0, selector: "objects_probe::composition/attached"}}));
    for pair in [["credit", 0], ["debit", 1], ["absent", 2], ["fault", 3], ["credit", 4]] {
        this.requests.push(request("call_object", #{target: target, binding: pair[0], method: "use",
            reply: #{request: pair[1], selector: "objects_probe::composition/done"}}));
    }
}
fn attached() {
    if this.trigger.stage == "rejected" { throw this.trigger.message; }
}
fn work() {
    if this.state.worker.busy {
        this.state.worker.completions += 1;
        this.requests.push(request("complete_object_call", #{sequence: this.trigger.sequence,
            reply: #{request: this.state.worker.completions, selector: "objects_probe::composition/ack"},
            accepted: false, code: "busy"}));
        return;
    }
    this.state.worker.busy = true;
    this.state.worker.requests += 1;
    this.state.worker.pending = this.trigger.sequence;
    let params = this.observe.behavior.params;
    this.requests.push(request("inventory_exchange", #{
        reply: #{request: this.state.worker.requests, selector: "objects_probe::composition/trade"},
        exchange: #{pay_coins: params.pay, receive_coins: params.receive}}));
    if params.fail { throw "deliberate staged action fault"; }
}
fn trade() {
    if !this.trigger.terminal { return; }
    if this.observe.behavior.binding == "credit" && this.trigger.stage != "applied" {
        throw this.trigger.code;
    }
    if this.trigger.stage == "applied" { this.state.worker.applied += 1; }
    else { this.state.worker.refused += 1; }
    this.state.worker.busy = false;
    this.state.worker.completions += 1;
    this.requests.push(request("complete_object_call", #{sequence: this.state.worker.pending,
        reply: #{request: this.state.worker.completions, selector: "objects_probe::composition/ack"},
        accepted: this.trigger.stage == "applied", code: this.trigger.code}));
}
fn ack() { if this.trigger.stage == "rejected" { throw this.trigger.code; } }
fn done() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage == "completed" { this.state.root.completed += 1; }
    else { this.state.root.refused += 1; }
}
"#;

fn prepared(source: &str, label: &str) -> (Platformer2dSimHarness, [Entity; 2], std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("ambition_object_behaviors_{label}_{}.rhai", std::process::id()));
    std::fs::write(&path, source).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room").with_save(save)
            .with_extension_module_files(vec![path.clone()]).with_sync_test_rollback_settings(4, 12)).unwrap();
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    let teacher = world.query::<(Entity, &CharacterForm)>().iter(world)
        .find(|(_, form)| form.id() == "npc_creator").unwrap().0;
    for body in [player, teacher] {
        world.entity_mut(body).insert((BodyWallet {balance: 3}, BodyStatuses(vec!["objects_probe.start".into()])));
    }
    sim.rebase_rollback_history().unwrap();
    (sim, [player, teacher], path)
}

#[test]
fn reusable_instances_await_native_results_on_both_controllers_and_replay() {
    let (mut sim, [player, teacher], path) = prepared(SOURCE, "instances");
    sim.step_n(AgentAction::default(), 40);
    let root = SchemaKey::new("objects_probe", "root", 1);
    let worker = SchemaKey::new("objects_probe", "worker", 1);
    for body in [player, teacher] {
        let record = sim.world().get::<BodyRecords>(body).unwrap().get(&root).unwrap();
        assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1));
        assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U32(4));
        assert_eq!(sim.world().get::<BodyWallet>(body).unwrap().balance, 5);
        assert!(sim.world().get::<BodyRecords>(body).unwrap().get(&worker).is_none());
    }
    let world = sim.world_mut();
    let states: Vec<_> = world.query::<&BodyRecords>().iter(world).filter_map(|records| records.get(&worker)).collect();
    assert_eq!(states.len(), 4);
    assert!(states.iter().all(|record| record.get(FieldRef(0)).unwrap() == &Value::U32(1)));
    assert_eq!(states.iter().filter(|record| record.get(FieldRef(2)).unwrap() == &Value::U32(1)).count(), 2);
    assert_eq!(states.iter().filter(|record| record.get(FieldRef(3)).unwrap() == &Value::U32(1)).count(), 2);
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 6);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_file(path);
}

#[test]
fn canceling_a_queued_method_prevents_its_native_action_and_a_committed_method_refuses_cancel() {
    use ambition_platformer2d::{extension::sdk::action::{ActionStage, ActionTicket},
        platformer::action_results::{ActionMailbox, cancel_action, ActionCancellationRefusal}};
    let source = SOURCE.replace("[[\"credit\", 0], [\"debit\", 1], [\"absent\", 2], [\"fault\", 3], [\"credit\", 4]]", "[[\"credit\", 0]]");
    let (mut sim, [player, teacher], path) = prepared(&source, "cancel");
    let admitted = |sim: &Platformer2dSimHarness, body| -> Option<ActionTicket> {
        sim.world().get::<ActionMailbox>(body)?.history().entries.into_iter()
            .find(|row| row.result.ticket.domain == "objects.call" && row.result.stage == ActionStage::Admitted)
            .map(|row| row.result.ticket)
    };
    for _ in 0..20 {
        if admitted(&sim, player).is_some() && admitted(&sim, teacher).is_some() { break; }
        sim.step(AgentAction::default());
    }
    let player_ticket = admitted(&sim, player).unwrap();
    let teacher_ticket = admitted(&sim, teacher).unwrap();
    assert_eq!(sim.world().get::<BodyWallet>(player).unwrap().balance, 3);
    assert_eq!(cancel_action(sim.world_mut(), player, &player_ticket), Ok(()));
    sim.rebase_rollback_history().unwrap();
    sim.step(AgentAction::default());
    assert_eq!(sim.world().get::<BodyWallet>(teacher).unwrap().balance, 5);
    assert_eq!(cancel_action(sim.world_mut(), teacher, &teacher_ticket), Err(ActionCancellationRefusal::OwnerRefused));
    sim.step_n(AgentAction::default(), 24);
    assert_eq!(sim.world().get::<BodyWallet>(player).unwrap().balance, 3);
    assert_eq!(sim.world().get::<BodyWallet>(teacher).unwrap().balance, 5);
    let history = sim.world().get::<ActionMailbox>(player).unwrap().history();
    assert!(history.entries.iter().any(|row| row.result.ticket == player_ticket && row.result.stage == ActionStage::Canceled));
    assert!(!history.entries.iter().any(|row| row.result.ticket == player_ticket && row.result.stage == ActionStage::Completed));
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_file(path);
}

#[test]
fn room_snapshots_restore_private_callbacks_and_scope_follows_the_subject() {
    use ambition_platformer2d::{platformer::lifecycle::{ObjectScopeOwner, SessionScopedEntity,
        object_reference, object_scope_room, resolve_object, room_scope_members, RoomInstanceRoot,
        InRoomInstance}, rollback::RoomSnapshotRows};
    let (mut sim, [player, teacher], path) = prepared(SOURCE, "room_snapshot");
    for _ in 0..20 {
        if sim.world().get::<BodyWallet>(player).unwrap().balance == 5 { break; }
        sim.step(AgentAction::default());
    }
    assert_eq!(sim.world().get::<BodyWallet>(player).unwrap().balance, 5);
    let session_scope = |reference: ambition_platformer2d::extension::sdk::ObjectRef|
        ambition_platformer2d::platformer::lifecycle::SessionScopeId(reference.session);
    let reference = object_reference(sim.world(), player).unwrap();
    let room = object_scope_room(sim.world(), player).unwrap();
    let world = sim.world_mut();
    let owned: Vec<_> = world.query::<(Entity, &ObjectScopeOwner)>().iter(world)
        .filter(|(_, owner)| owner.0 == reference).map(|(entity, _)| entity).collect();
    assert_eq!(owned.len(), 3);
    let members = room_scope_members(world, room);
    assert!(owned.iter().all(|entity| members.contains(entity)));
    let private_reference = object_reference(world, owned[0]).unwrap();
    let rows = world.remove_resource::<RoomSnapshotRows>().unwrap();
    let mut snapshot = rows.save(world, &members);
    sim.step_n(AgentAction::default(), 12);
    let current = resolve_object(sim.world(), session_scope(reference), private_reference).unwrap();
    sim.world_mut().despawn(current);
    sim.world_mut().get_mut::<BodyWallet>(player).unwrap().balance = 999;
    let members = room_scope_members(sim.world_mut(), room);
    let report = rows.restore(sim.world_mut(), &mut snapshot, &members);
    assert!(report.recreated.iter().any(|(old, _)| *old == owned[0]));
    let again = resolve_object(sim.world(), ambition_platformer2d::platformer::lifecycle::SessionScopeId(reference.session), private_reference).unwrap();
    assert_ne!(again, owned[0]);
    let world = sim.world_mut();
    world.insert_resource(rows);
    let destination = room.next().next();
    let session = *world.get::<SessionScopedEntity>(player).unwrap();
    let root = world.spawn((RoomInstanceRoot, destination, session)).id();
    world.entity_mut(player).insert(InRoomInstance(destination));
    assert_eq!(object_scope_room(world, again), Some(destination));
    assert!(room_scope_members(world, destination).contains(&again));
    assert!(!room_scope_members(world, room).contains(&again));
    world.entity_mut(player).insert(InRoomInstance(room));
    world.despawn(root);
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 20);
    for body in [player, teacher] {
        assert_eq!(sim.world().get::<BodyWallet>(body).unwrap().balance, 5);
        let record = sim.world().get::<BodyRecords>(body).unwrap().get(&SchemaKey::new("objects_probe", "root", 1)).unwrap();
        assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1));
        assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U32(4));
    }
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_file(path);
}
