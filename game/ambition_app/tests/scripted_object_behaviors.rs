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
            #{tag: 4, name: "returned", kind: "seq 4 object"},
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
        #{key: "done", phase: "actor_think", trigger: #{port: "action_result", selector: "done"}, reads: ["object_call_result"], writes: ["root"]},
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
    let dormant = target;
    dormant.sequence = "18446744073709551614";
    let bindings = [];
    for pair in [["credit", 0, 2, false], ["debit", 99, 0, false], ["fault", 0, 99, true]] {
        bindings.push(#{key: pair[0], params: #{pay: pair[1], receive: pair[2], fail: pair[3]},
            references: #{owner: target, dormant: dormant}, methods: [#{name: "use", entry: "objects_probe::composition/work"}]});
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
    if this.observe.behavior.references.owner != this.observe.behavior.object { throw "a live binding names another owner"; }
    if this.state.worker.busy {
        this.state.worker.completions += 1;
        this.requests.push(request("complete_object_call", #{sequence: this.trigger.sequence,
            reply: #{request: this.state.worker.completions, selector: "objects_probe::composition/ack"},
            accepted: false, code: "busy", data: #{binding: this.observe.behavior.binding, object: this.observe.behavior.object}}));
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
        accepted: this.trigger.stage == "applied", code: this.trigger.code,
        data: #{binding: this.observe.behavior.binding, object: this.observe.behavior.object}}));
}
fn ack() { if this.trigger.stage == "rejected" { throw this.trigger.code; } }
fn done() {
    if !this.trigger.terminal {
        if this.observe.object_call_result != () { throw "return value preceded completion"; }
        return;
    }
    let result = this.observe.object_call_result;
    if result != () {
        if result.ticket != this.trigger.ticket { throw "return value crossed a ticket"; }
        this.state.root.returned.push(result.data.object);
    }
    if this.trigger.stage == "completed" { this.state.root.completed += 1; }
    else { this.state.root.refused += 1; }
}
"#;

fn prepared(source: &str, label: &str) -> (Platformer2dSimHarness, [Entity; 2], std::path::PathBuf) {
    prepared_with_modules(source, label, Vec::new())
}
fn prepared_with_modules(source: &str, label: &str, mut modules: Vec<std::path::PathBuf>)
    -> (Platformer2dSimHarness, [Entity; 2], std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("ambition_object_behaviors_{label}_{}.rhai", std::process::id()));
    std::fs::write(&path, source).unwrap();
    modules.push(path.clone());
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room").with_save(save)
            .with_extension_module_files(modules).with_sync_test_rollback_settings(4, 12)).unwrap();
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    let teacher = world.query::<(Entity, &CharacterForm)>().iter(world)
        .find(|(_, form)| form.id() == "npc_creator").unwrap().0;
    for body in [player, teacher] {
        world.entity_mut(body).insert((BodyWallet {balance: 3}, BodyStatuses(vec!["objects_probe.start".into()])));
    }
    let mut delivery = ambition_platformer2d::world::scoped_rules::DeclaredRules::default();
    delivery.declare(ambition_platformer2d::world::scoped_rules::RulesScope::EveryRoom,
        ambition_platformer2d::platformer::action_results::ActionDeliveryPolicy {
            max_per_tick: std::num::NonZeroU32::new(1).unwrap(), ..Default::default()});
    world.insert_resource(delivery);
    sim.rebase_rollback_history().unwrap();
    (sim, [player, teacher], path)
}

#[test]
fn reusable_instances_await_native_results_on_both_controllers_and_replay() { exercise_instances(false); }

#[test]
fn wasm_methods_keep_private_state_and_complete_calls_after_native_results_and_replay() { exercise_instances(true); }

fn exercise_instances(portable: bool) {
    let (mut sim, [player, teacher], path) = if portable {
        let guest = ambition_platformer2d::extension::build_module_crate(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap(),
            "ambition_extension_fixture_modules").unwrap();
        let source = SOURCE.replace("entry: \"objects_probe::composition/work\"", "entry: \"fixture::object_worker/work\"");
        prepared_with_modules(&source, "wasm_instances", vec![guest])
    } else { prepared(SOURCE, "instances") };
    sim.step_n(AgentAction::default(), 40);
    let root = SchemaKey::new("objects_probe", "root", 1);
    let worker = if portable { SchemaKey::new("fixture", "object_worker.memory", 1) }
        else { SchemaKey::new("objects_probe", "worker", 1) };
    for body in [player, teacher] {
        let record = sim.world().get::<BodyRecords>(body).unwrap().get(&root).unwrap();
        assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1));
        assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U32(4));
        assert_eq!(sim.world().get::<BodyWallet>(body).unwrap().balance, 5);
        let Value::Seq(returned) = record.get(FieldRef(3)).unwrap() else { panic!("return values"); };
        assert_eq!(returned.len(), 3);
        assert!(returned.iter().all(|value| value.as_object() ==
            ambition_platformer2d::platformer::lifecycle::object_reference(sim.world(), body)));
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
fn requests_and_native_outcomes_share_one_private_receiver_budget_through_replay() {
    let source = SOURCE.replace("[\"credit\", 4]", "[\"credit\", 4], [\"credit\", 5], [\"credit\", 6], [\"credit\", 7]")
        .replace("limit: 6}", "limit: 9}");
    let (mut sim, bodies, path) = prepared(&source, "shared_inbox");
    let worker = SchemaKey::new("objects_probe", "worker", 1);
    let mut previous = std::collections::BTreeMap::new();
    for _ in 0..80 {
        sim.step_n(AgentAction::default(), 1);
        let world = sim.world_mut();
        let mut records = world.query::<(Entity, &BodyRecords)>();
        for (entity, records) in records.iter(world) {
            let Some(record) = records.get(&worker) else { continue; };
            let Value::U32(requests) = record.get(FieldRef(0)).unwrap() else { panic!("request count"); };
            let Value::U32(completions) = record.get(FieldRef(5)).unwrap() else { panic!("completion count"); };
            let receiver = ambition_platformer2d::platformer::lifecycle::object_reference(world, entity).unwrap();
            let count = requests + completions;
            let before = previous.insert(receiver, count).unwrap_or(0);
            assert!(count - before <= 1, "a method and an outcome consumed separate delivery budgets");
        }
    }
    for body in bodies {
        assert_eq!(sim.world().get::<BodyWallet>(body).unwrap().balance, 5);
    }
    assert!(!previous.is_empty());
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
        sim.step_n(AgentAction::default(), 1);
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
        sim.step_n(AgentAction::default(), 1);
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

const BIRTH_SOURCE: &str = r#"
fn declare() {
    #{provider: "objects_probe", key: "birth", schemas: [
        #{key: "factory", fields: [
            #{tag: 1, name: "sent", kind: "bool"}, #{tag: 2, name: "created", kind: "u32"},
            #{tag: 3, name: "refused", kind: "u32"}, #{tag: 4, name: "completed", kind: "u32"},
            #{tag: 5, name: "objects", kind: "seq 2 object"}
        ]},
        #{key: "instance", fields: [
            #{tag: 1, name: "born", kind: "u32"}, #{tag: 2, name: "called", kind: "u32"},
            #{tag: 3, name: "seed", kind: "u32"}
        ]}
    ], entries: [
        #{key: "build", phase: "technique_execution", trigger: #{port: "status_tick", selector: "objects_probe.start"},
          writes: ["factory"], requests: ["create_actors", "create_breakables"], limit: 4},
        #{key: "created", phase: "actor_think", trigger: #{port: "action_result", selector: "created"},
          writes: ["factory"], requests: ["call_object"], limit: 2},
        #{key: "done", phase: "actor_think", trigger: #{port: "action_result", selector: "done"}, writes: ["factory"]},
        #{key: "work", phase: "actor_think", trigger: #{port: "object_method", selector: "use"},
          reads: ["behavior"], writes: ["instance"], requests: ["complete_object_call"], limit: 1},
        #{key: "ack", phase: "actor_think", trigger: #{port: "action_result", selector: "ack"}}
    ]}
}
fn bindings(owner) {
    let result = [];
    for pair in [["first", 17], ["second", 29]] {
        result.push(#{key: pair[0], references: #{owner: #{local: owner}}, methods: [#{name: "use", entry: "objects_probe::birth/work"}],
            state: [#{schema: "instance", value: #{born: 0, called: 0, seed: pair[1]}}]});
    }
    result
}
fn build() {
    if this.state.factory.sent { return; }
    this.state.factory.sent = true;
    let p = this.trigger.position;
    let actor = #{key: "actor", character: "practice_sandbag", at: [p[0] + 800.0, p[1] - 500.0],
        starting_wallet: 3, holds: ["birth_probe.child"], bindings: bindings("actor")};
    let prop = #{key: "prop", at: [p[0] + 900.0, p[1] - 500.0], size: [12.0, 12.0],
        health: 2, collision: "none", bindings: bindings("prop")};
    this.requests.push(request("create_actors", #{reply: #{request: 1, selector: "objects_probe::birth/created"}, actors: [actor]}));
    this.requests.push(request("create_breakables", #{reply: #{request: 1, selector: "objects_probe::birth/created"}, objects: [prop]}));
    let broken_actor = actor;
    broken_actor.key = "broken_actor";
    broken_actor.bindings[1].state[0].value.typo = 1;
    this.requests.push(request("create_actors", #{reply: #{request: 2, selector: "objects_probe::birth/created"}, actors: [actor, broken_actor]}));
    let broken_prop = prop;
    broken_prop.key = "broken_prop";
    broken_prop.bindings[1].methods[0].entry = "objects_probe::birth/missing";
    this.requests.push(request("create_breakables", #{reply: #{request: 2, selector: "objects_probe::birth/created"}, objects: [prop, broken_prop]}));
}
fn created() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage == "rejected" { this.state.factory.refused += 1; return; }
    if this.trigger.stage != "applied" { throw "unexpected creation outcome"; }
    this.state.factory.created += 1;
    let object = this.trigger.subjects[0].object;
    this.state.factory.objects.push(object);
    for pair in [["first", 0], ["second", 1]] {
        this.requests.push(request("call_object", #{reply: #{request: this.state.factory.created * 2 + pair[1], selector: "objects_probe::birth/done"},
            target: object, binding: pair[0], method: "use"}));
    }
}
fn work() {
    if this.observe.behavior.references.owner != this.observe.behavior.object { throw "unresolved local owner at birth"; }
    if this.state.instance.born != 1 { throw "publication missed initialized private state"; }
    this.state.instance.called += 1;
    this.requests.push(request("complete_object_call", #{reply: #{request: 1, selector: "objects_probe::birth/ack"},
        sequence: this.trigger.sequence, accepted: true, code: "used"}));
}
fn done() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage != "completed" { throw "published behavior could not complete"; }
    this.state.factory.completed += 1;
}
fn ack() { if this.trigger.terminal && this.trigger.stage != "applied" { throw "completion refused"; } }
"#;

fn observe_composed_birth(
    event: On<ambition_platformer2d::platformer::lifecycle::ObjectPublished>,
    mut states: Query<(&ambition_platformer2d::platformer::lifecycle::ObjectScopeOwner,
        &ambition_platformer2d::platformer::lifecycle::ObjectOccurrence, &mut BodyRecords)>,
    bodies: Query<(Option<&BodyWallet>, Option<&BodyStatuses>)>,
) {
    let reference = event.occurrence.reference(event.session);
    let key = SchemaKey::new("objects_probe", "instance", 1);
    let mut found = 0;
    for (owner, _, mut records) in &mut states {
        if owner.0 != reference { continue; }
        let Some(record) = records.get(&key) else { continue; };
        assert_eq!(record.get(FieldRef(0)).unwrap(), &Value::U32(0));
        let mut record = record.clone();
        record.set(FieldRef(0), Value::U32(1)).unwrap();
        let shape = records.get_stored(&key).unwrap().shape;
        records.put(key.clone(), shape, record);
        found += 1;
    }
    if found == 0 { return; }
    assert_eq!(found, 2, "both private instances must be visible at the first birth notice");
    let (wallet, statuses) = bodies.get(event.subject).unwrap();
    if let Some(wallet) = wallet {
        assert_eq!(wallet.balance, 3);
        assert!(statuses.unwrap().0.iter().any(|key| key == "birth_probe.child"));
    }
}

#[test]
fn actor_and_prop_births_publish_complete_private_instances_and_refuse_incomplete_batches_after_replay() {
    use ambition_platformer2d::platformer::{lifecycle::*, sim_id::SimIdCounter};
    let (mut sim, subjects, path) = prepared(BIRTH_SOURCE, "birth");
    sim.world_mut().add_observer(observe_composed_birth);
    let counters = subjects.map(|body| sim.world().get::<SimIdCounter>(body).unwrap().0);
    sim.step_n(AgentAction::default(), 48);
    let factory = SchemaKey::new("objects_probe", "factory", 1);
    let private = SchemaKey::new("objects_probe", "instance", 1);
    let mut references = Vec::new();
    for (index, body) in subjects.into_iter().enumerate() {
        let records = sim.world().get::<BodyRecords>(body).unwrap_or_else(||
            panic!("factory records were not committed: {:?}", sim.world().resource::<ExtensionFaults>()));
        let record = records.get(&factory).unwrap_or_else(||
            panic!("factory state was not committed: {:?}", sim.world().resource::<ExtensionFaults>()));
        for (field, expected) in [(1, 2), (2, 2), (3, 4)] {
            assert_eq!(record.get(FieldRef(field)).unwrap(), &Value::U32(expected),
                "field {field}: {record:?}; faults: {:?}; actions: {:?}",
                sim.world().resource::<ExtensionFaults>(),
                sim.world().get::<ambition_platformer2d::platformer::action_results::ActionMailbox>(body).map(|mailbox| mailbox.history()));
        }
        let Value::Seq(objects) = record.get(FieldRef(4)).unwrap() else { panic!("object references"); };
        for value in objects {
            let Value::Object(reference) = value else { panic!("typed reference"); };
            references.push(*reference);
        }
        assert_eq!(sim.world().get::<SimIdCounter>(body).unwrap().0, counters[index] + 2);
    }
    let world = sim.world_mut();
    let mut count = 0;
    for (owner, records) in world.query::<(&ObjectScopeOwner, &BodyRecords)>().iter(world) {
        if !references.contains(&owner.0) { continue; }
        let record = records.get(&private).unwrap();
        assert_eq!(record.get(FieldRef(0)).unwrap(), &Value::U32(1));
        assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1));
        assert!(matches!(record.get(FieldRef(2)).unwrap(), Value::U32(17 | 29)));
        count += 1;
    }
    assert_eq!(count, 8);
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_file(path);
}

const LOCAL_SOURCE: &str = r#"
fn declare() {
    #{provider: "objects_probe", key: "locals", schemas: [
        #{key: "factory", fields: [
            #{tag: 1, name: "sent", kind: "bool"}, #{tag: 2, name: "created", kind: "u32"},
            #{tag: 3, name: "invalid", kind: "u32"}, #{tag: 4, name: "accepted", kind: "u32"},
            #{tag: 5, name: "refused", kind: "u32"}, #{tag: 6, name: "objects", kind: "seq 4 object"}
        ]},
        #{key: "relay", fields: [#{tag: 1, name: "pending", kind: "u64"},
            #{tag: 2, name: "forwarded", kind: "u32"}, #{tag: 3, name: "completed", kind: "u32"}]},
        #{key: "worker", fields: [#{tag: 1, name: "requests", kind: "u32"}, #{tag: 2, name: "pending", kind: "u64"},
            #{tag: 3, name: "applied", kind: "u32"}, #{tag: 4, name: "refused", kind: "u32"},
            #{tag: 5, name: "busy", kind: "bool"}, #{tag: 6, name: "completions", kind: "u32"}]}
    ], entries: [
        #{key: "build", phase: "technique_execution", trigger: #{port: "status_tick", selector: "objects_probe.start"},
          writes: ["factory"], requests: ["create_actors"], limit: 2},
        #{key: "created", phase: "actor_think", trigger: #{port: "action_result", selector: "created"},
          writes: ["factory"], requests: ["call_object"], limit: 2},
        #{key: "done", phase: "actor_think", trigger: #{port: "action_result", selector: "done"},
          reads: ["object_call_result"], writes: ["factory"]},
        #{key: "relay", phase: "actor_think", trigger: #{port: "object_method", selector: "relay"},
          reads: ["behavior"], writes: ["relay"], requests: ["call_object"], limit: 1},
        #{key: "relayed", phase: "actor_think", trigger: #{port: "action_result", selector: "relayed"},
          reads: ["object_call_result"], writes: ["relay"], requests: ["complete_object_call"], limit: 1},
        #{key: "work", phase: "actor_think", trigger: #{port: "object_method", selector: "work"},
          reads: ["behavior"], writes: ["worker"], requests: ["inventory_exchange"], limit: 1},
        #{key: "trade", phase: "actor_think", trigger: #{port: "action_result", selector: "trade"},
          reads: ["behavior"], writes: ["worker"], requests: ["complete_object_call"], limit: 1},
        #{key: "ack", phase: "actor_think", trigger: #{port: "action_result", selector: "ack"}}
    ]}
}
fn node(key, peer, entry, pay, p) {
    #{key: key, character: "practice_sandbag", at: p, starting_wallet: 3, allegiance: "neutral",
        bindings: [#{key: "logic", params: #{pay: pay, receive: 2, fail: false},
            references: #{owner: #{local: key}, peer: #{local: peer}},
            methods: [#{name: "use", entry: entry}]}]}
}
fn build() {
    if this.state.factory.sent { return; }
    this.state.factory.sent = true;
    let p = [this.trigger.position[0] + 800.0, this.trigger.position[1] - 500.0];
    let actors = [
        node("free_relay", "free_worker", "objects_probe::locals/relay", 0, p),
        node("free_worker", "free_relay", "objects_probe::locals/work", 0, p),
        node("cost_relay", "cost_worker", "objects_probe::locals/relay", 99, p),
        node("cost_worker", "cost_relay", "objects_probe::locals/work", 99, p)
    ];
    actors[0].character = "npc_giant_gnu";
    let broken = actors;
    broken[3].bindings[0].references.peer = #{local: "absent"};
    this.requests.push(request("create_actors", #{reply: #{request: 0, selector: "objects_probe::locals/created"}, actors: broken}));
    this.requests.push(request("create_actors", #{reply: #{request: 1, selector: "objects_probe::locals/created"}, actors: actors}));
}
fn created() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage == "rejected" { this.state.factory.invalid += 1; return; }
    this.state.factory.created += 1;
    for subject in this.trigger.subjects { this.state.factory.objects.push(subject.object); }
    for i in 0..2 {
        this.requests.push(request("call_object", #{reply: #{request: i, selector: "objects_probe::locals/done"},
            target: this.trigger.subjects[i * 2].object, binding: "logic", method: "use"}));
    }
}
fn relay() {
    let context = this.observe.behavior;
    if context.references.owner != context.object { throw "self reference is not the published owner"; }
    this.state.relay.pending = this.trigger.sequence;
    this.state.relay.forwarded += 1;
    this.requests.push(request("call_object", #{reply: #{request: 1, selector: "objects_probe::locals/relayed"},
        target: context.references.peer, binding: "logic", method: "use"}));
}
fn relayed() {
    if !this.trigger.terminal { return; }
    let data = this.observe.object_call_result.data;
    this.state.relay.completed += 1;
    this.requests.push(request("complete_object_call", #{reply: #{request: 1, selector: "objects_probe::locals/ack"},
        sequence: this.state.relay.pending, accepted: this.trigger.stage == "completed", code: this.trigger.code, data: data}));
}
fn work() {
    let context = this.observe.behavior;
    if context.references.owner != context.object || context.references.peer != this.trigger.caller {
        throw "mutual references crossed a factory occurrence";
    }
    this.state.worker.pending = this.trigger.sequence;
    this.state.worker.requests += 1;
    this.requests.push(request("inventory_exchange", #{reply: #{request: 1, selector: "objects_probe::locals/trade"},
        exchange: #{pay_coins: context.params.pay, receive_coins: 2}}));
}
fn trade() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage == "applied" { this.state.worker.applied += 1; } else { this.state.worker.refused += 1; }
    this.state.worker.completions += 1;
    this.requests.push(request("complete_object_call", #{reply: #{request: 1, selector: "objects_probe::locals/ack"},
        sequence: this.state.worker.pending, accepted: this.trigger.stage == "applied", code: this.trigger.code,
        data: #{object: this.observe.behavior.object}}));
}
fn done() {
    if !this.trigger.terminal { return; }
    if this.observe.object_call_result.data.object != this.state.factory.objects[this.trigger.ticket.request * 2 + 1] {
        throw "the return value came from another worker occurrence";
    }
    if this.trigger.stage == "completed" { this.state.factory.accepted += 1; } else { this.state.factory.refused += 1; }
}
fn ack() { if this.trigger.stage == "rejected" { throw this.trigger.code; } }
"#;

#[test]
fn local_factory_references_connect_independent_objects_before_publication_and_replay() { exercise_local_references(false, false); }

#[test]
fn local_factory_references_reach_portable_workers_with_native_outcomes_and_replay() { exercise_local_references(true, false); }

#[test]
fn native_births_activate_named_relay_methods_and_faults_are_inspectable_through_replay() {
    exercise_local_references(false, true);
}
#[test]
fn native_birth_relays_call_portable_workers_and_replay() { exercise_local_references(true, true); }

fn native_birth_source() -> String {
    LOCAL_SOURCE.replace(
        r##"#{tag: 3, name: "completed", kind: "u32"}]}"##,
        r##"#{tag: 3, name: "completed", kind: "u32"}, #{tag: 4, name: "birth_done", kind: "u32"}]}"##)
        .replace("fn build() {", r##"fn born() {
    let context = this.observe.behavior;
    if context.references.peer != this.trigger.object || this.observe.inventory.wallet != 3 {
        throw "a native birth exposed incomplete or crossed state";
    }
    this.requests.push(request("call_object", #{reply: #{request: 0, selector: "objects_probe::locals/birth_done"},
        target: context.object, binding: "logic", method: "use"}));
}
fn birth_fault() {
    this.state.relay.forwarded += 99;
    this.requests.push(request("inventory_exchange", #{reply: #{request: 99, selector: "objects_probe::locals/ack"},
        exchange: #{receive_coins: 99}}));
    throw "native birth deliberate staged fault";
}
fn birth_done() {
    if !this.trigger.terminal { return; }
    let history = this.observe.behavior_events;
    let acknowledged = 0; let quarantined = 0;
    if history.dropped != "0" { throw "birth facts were dropped"; }
    for row in history.deliveries {
        if row.source != this.observe.behavior.references.peer { throw "native source identity crossed"; }
        if row.progress.status == "acknowledged" { acknowledged += 1; }
        if row.progress.status == "quarantined" && row.progress.attempts == 3 { quarantined += 1; }
    }
    if acknowledged != 1 || quarantined != 1 { throw "native delivery did not retain acknowledgement and bounded fault"; }
    this.state.relay.birth_done += 1;
}
fn build() {"##)
        .replace(r##"#{key: "build", phase:"##, r##"#{key: "born", phase: "actor_think", trigger: #{port: "object_born", selector: "born"},
          reads: ["behavior", "inventory"], requests: ["call_object"], limit: 1},
        #{key: "birth_fault", phase: "actor_think", trigger: #{port: "object_born", selector: "born"},
          writes: ["relay"], requests: ["inventory_exchange"], limit: 1},
        #{key: "birth_done", phase: "actor_think", trigger: #{port: "action_result", selector: "birth_done"},
          reads: ["behavior", "behavior_events"], writes: ["relay"]},
        #{key: "build", phase:"##)
        .replace("    actors[0].character =", r##"    for index in [0, 2] {
        actors[index].bindings[0].methods.push(#{name: "ready", entry: "objects_probe::locals/born"});
        actors[index].bindings[0].methods.push(#{name: "broken", entry: "objects_probe::locals/birth_fault"});
        actors[index].bindings[0].subscriptions = [
            #{key: "ready", source: "peer", method: "ready"},
            #{key: "broken", source: "peer", method: "broken"}];
    }
    actors[0].character ="##)
        .replace(r##"    broken[3].bindings[0].references.peer = #{local: "absent"};"##,
            r##"    broken[3].bindings[0].subscriptions = [#{key: "invalid", source: "owner", method: "use"}];"##)
        .replace(r##"    for i in 0..2 {
        this.requests.push(request("call_object", #{reply: #{request: i, selector: "objects_probe::locals/done"},
            target: this.trigger.subjects[i * 2].object, binding: "logic", method: "use"}));
    }"##, "")
}

fn exercise_local_references(portable: bool, birth: bool) {
    use ambition_platformer2d::platformer::{lifecycle::*, sim_id::SimIdCounter};
    let source = if birth { native_birth_source() } else { LOCAL_SOURCE.into() };
    let (mut sim, roots, path) = if portable {
        let guest = ambition_platformer2d::extension::build_module_crate(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap(),
            "ambition_extension_fixture_modules").unwrap();
        let mut source = source.replace("\"objects_probe::locals/work\",", "\"fixture::object_worker/work\",");
        if birth { source = source.replace("objects_probe::locals/born", "fixture::object_worker/born"); }
        prepared_with_modules(&source, if birth { "birth_wasm" } else { "local_wasm" }, vec![guest])
    } else { prepared(&source, if birth { "birth_relays" } else { "locals" }) };
    let counters = roots.map(|entity| sim.world().get::<SimIdCounter>(entity).unwrap().0);
    sim.step_n(AgentAction::default(), 80);
    let factory = SchemaKey::new("objects_probe", "factory", 1);
    for (index, root) in roots.into_iter().enumerate() {
        let record = sim.world().get::<BodyRecords>(root).unwrap().get(&factory).unwrap();
        for field in 1..=4 {
            assert_eq!(record.get(FieldRef(field)).unwrap(), &Value::U32(u32::from(!birth || field < 3)),
                "field {field}: {record:?}; faults: {:?}", sim.world().resource::<ExtensionFaults>());
        }
        assert_eq!(sim.world().get::<SimIdCounter>(root).unwrap().0, counters[index] + 4);
        assert_eq!(sim.world().get::<BodyWallet>(root).unwrap().balance, 3);
    }
    let worker = if portable { SchemaKey::new("fixture", "object_worker.memory", 1) }
        else { SchemaKey::new("objects_probe", "worker", 1) };
    let world = sim.world_mut();
    let rows = world.query::<(&ObjectScopeOwner, &BodyRecords)>().iter(world)
        .filter_map(|(owner, records)| records.get(&worker).map(|record| (owner.0, record.clone()))).collect::<Vec<_>>();
    assert_eq!(rows.len(), 4);
    for (owner, record) in rows {
        assert_eq!(record.get(FieldRef(0)).unwrap(), &Value::U32(1));
        assert_eq!(record.get(FieldRef(5)).unwrap(), &Value::U32(1));
        let applied = record.get(FieldRef(2)).unwrap() == &Value::U32(1);
        assert_eq!(record.get(FieldRef(3)).unwrap(), &Value::U32(u32::from(!applied)));
        let entity = resolve_object(world, SessionScopeId(owner.session), owner).unwrap();
        assert_eq!(world.get::<BodyWallet>(entity).unwrap().balance, if applied { 5 } else { 3 });
    }
    if birth {
        let world = sim.world_mut();
        let relay = if portable { SchemaKey::new("fixture", "object_worker.birth", 1) }
            else { SchemaKey::new("objects_probe", "relay", 1) };
        let records = world.query::<(&ObjectScopeOwner, &BodyRecords)>().iter(world)
            .filter_map(|(owner, records)| records.get(&relay).map(|record| (owner.0, record.clone()))).collect::<Vec<_>>();
        assert_eq!(records.len(), 4);
        for (owner, record) in records {
            if portable {
                assert_eq!(record.get(FieldRef(0)).unwrap(), &Value::U32(1));
                assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1));
                let Value::U32(accepted) = record.get(FieldRef(2)).unwrap() else { panic!("accepted count"); };
                let Value::U32(refused) = record.get(FieldRef(3)).unwrap() else { panic!("refused count"); };
                assert_eq!(accepted + refused, 1);
            } else { for field in 1..=3 { assert_eq!(record.get(FieldRef(field)).unwrap(), &Value::U32(1), "relay {record:?}"); } }
            let entity = resolve_object(world, SessionScopeId(owner.session), owner).unwrap();
            assert_eq!(world.get::<BodyWallet>(entity).unwrap().balance, 3);
        }
        assert!(sim.world().resource::<ExtensionFaults>().total >= 12);
    } else { assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0); }
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_file(path);
}

const DEFEAT_SOURCE: &str = r#"
fn declare() {
    #{provider: "defeat_probe", key: "watch", schemas: [
        #{key: "root", fields: [
            #{tag: 1, name: "started", kind: "bool"},
            #{tag: 2, name: "foes", kind: "seq 4 object"},
            #{tag: 3, name: "attached", kind: "u32"},
        ]},
        #{key: "watcher", fields: [
            #{tag: 1, name: "downs", kind: "u32"},
            #{tag: 2, name: "fallen", kind: "opt object"},
        ]}
    ], entries: [
        #{key: "start", phase: "technique_execution", trigger: #{port: "status_tick", selector: "defeat_probe.start"},
          writes: ["root"], requests: ["body_query"], limit: 1},
        #{key: "found", phase: "actor_think", trigger: #{port: "action_result", selector: "found"},
          reads: ["body_query_result"], writes: ["root"], requests: ["attach_behaviors"], limit: 1},
        #{key: "attached", phase: "actor_think", trigger: #{port: "action_result", selector: "attached"}, writes: ["root"]},
        #{key: "down", phase: "actor_think", trigger: #{port: "object_defeated", selector: "defeated"}, writes: ["watcher"]},
    ]}
}
fn start() {
    if this.state.root.started { return; }
    this.state.root.started = true;
    this.requests.push(request("body_query", #{reply: #{request: 0, selector: "defeat_probe::watch/found"},
        origin: this.trigger.position, radius: 4000.0, limit: 5, include_self: true}));
}
fn found() {
    if this.trigger.stage != "applied" { return; }
    let references = #{};
    let subscriptions = [];
    let n = 0;
    let subject = ();
    for target in this.observe.body_query_result.targets {
        if target.driven { subject = target.object; continue; }
        if n == 4 { continue; }
        let key = "foe" + n;
        references[key] = target.object;
        subscriptions.push(#{key: key, source: key, method: "down"});
        this.state.root.foes.push(target.object);
        n += 1;
    }
    if subject == () { throw "the query found no driven body"; }
    this.requests.push(request("attach_behaviors", #{target: subject, bindings: [#{key: "watch",
        references: references, methods: [#{name: "down", entry: "defeat_probe::watch/down"}],
        subscriptions: subscriptions}], reply: #{request: 0, selector: "defeat_probe::watch/attached"}}));
}
fn attached() {
    if this.trigger.stage == "rejected" { throw this.trigger.message; }
    if this.trigger.terminal { this.state.root.attached += 1; }
}
fn down() {
    this.state.watcher.downs += 1;
    this.state.watcher.fallen = this.trigger.object;
}
"#;

/// ⭐ A DEFEAT IS A FACT A SCRIPT CAN SUBSCRIBE TO. The damage owner commits
/// it on the killing hit (`DefeatCommitted`); the runtime routes it to each
/// method subscribed to that body, once, and only the struck body's watchers
/// hear it. Fixed host: a hit written from outside the tick does not survive a
/// sync-test re-simulation, and the mailbox's replay is the activation path's
/// (`a_scripted_lever_spends_its_barrier`).
#[test]
fn a_subscribed_method_hears_the_defeat_of_its_source_once() {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    let path = std::env::temp_dir().join(format!("ambition_defeat_probe_{}.rhai", std::process::id()));
    std::fs::write(&path, DEFEAT_SOURCE).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("basement_enemies")
            .with_extension_module_files(vec![path.clone()])).unwrap();
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    world.entity_mut(player).insert(BodyStatuses(vec!["defeat_probe.start".into()]));
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 20);

    let root = SchemaKey::new("defeat_probe", "root", 1);
    let watcher = SchemaKey::new("defeat_probe", "watcher", 1);
    let record = sim.world().get::<BodyRecords>(player).unwrap().get(&root).cloned().expect("the probe started");
    assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U32(1), "the watch is attached");
    let Value::Seq(foes) = record.get(FieldRef(1)).unwrap() else { panic!("the probe lists its foes") };
    assert!(!foes.is_empty(), "the premise: the gallery has bodies near the player");
    // Strike the first listed body: the gallery's bodies are hostile, so it dies.
    let mut struck = Vec::new();
    for foe in foes.iter().take(1).map(|value| value.as_object().unwrap()) {
        let session = ambition_platformer2d::platformer::lifecycle::SessionScopeId(foe.session);
        let body = ambition_platformer2d::platformer::lifecycle::resolve_object(sim.world(), session, foe).unwrap();
        let volume = sim.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(body).unwrap().aabb().into();
        sim.world_mut().write_message(HitEvent {
            origin: None, properties: Default::default(), contact: None, pieces: Vec::new(), strike_sfx: None,
            volume, damage: 999, source: HitSource::Contact, attacker: Some(player), room: None,
            target: HitTarget::Body(body), mode: HitMode::Knockback, knockback: None, ignored_targets: Vec::new(),
            attacker_move_instance: None,
        });
        struck.push((foe, body));
    }
    sim.step_n(AgentAction::default(), 2);
    let fallen: Vec<_> = struck.iter().filter(|(_, body)| sim.world().get::<ambition_platformer2d::characters::actor::BodyHealth>(*body)
        .is_some_and(|health| !health.alive())).map(|(foe, _)| *foe).collect();
    assert_eq!(fallen.len(), 1, "the premise: the struck body is defeated");
    let foe = fallen[0];
    sim.step_n(AgentAction::default(), 30);

    let world = sim.world_mut();
    let heard: Vec<_> = world.query::<&BodyRecords>().iter(world).filter_map(|records| records.get(&watcher).cloned()).collect();
    assert_eq!(heard.len(), 1, "one watch instance");
    assert_eq!(heard[0].get(FieldRef(0)).unwrap(), &Value::U32(1), "one defeat, heard once");
    assert_eq!(heard[0].get(FieldRef(1)).unwrap(), &Value::Opt(Some(Box::new(Value::Object(foe)))), "the fact names the defeated body");
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_file(path);
}

const SIGNAL_SOURCE: &str = r#"
fn declare() {
    #{provider: "bell_probe", key: "ring", schemas: [
        #{key: "root", fields: [
            #{tag: 1, name: "started", kind: "bool"},
            #{tag: 2, name: "listeners", kind: "seq 4 object"},
            #{tag: 3, name: "attached", kind: "u32"},
            #{tag: 4, name: "rung", kind: "u32"},
            #{tag: 5, name: "heard_by", kind: "u32"},
            #{tag: 6, name: "refused", kind: "text 64"},
        ]},
        #{key: "listener", fields: [
            #{tag: 1, name: "heard", kind: "u32"},
            #{tag: 2, name: "from", kind: "opt object"},
            #{tag: 3, name: "peal", kind: "u32"},
        ]}
    ], entries: [
        #{key: "start", phase: "technique_execution", trigger: #{port: "status_tick", selector: "bell_probe.start"},
          writes: ["root"], requests: ["body_query"], limit: 1},
        #{key: "found", phase: "actor_think", trigger: #{port: "action_result", selector: "found"},
          reads: ["body_query_result"], writes: ["root"], requests: ["attach_behaviors"], limit: 2},
        #{key: "attached", phase: "actor_think", trigger: #{port: "action_result", selector: "attached"},
          writes: ["root"], requests: ["emit_signal"], limit: 2},
        #{key: "rang", phase: "actor_think", trigger: #{port: "action_result", selector: "rang"}, writes: ["root"]},
        #{key: "forged", phase: "actor_think", trigger: #{port: "action_result", selector: "forged"}, writes: ["root"]},
        #{key: "hear", phase: "actor_think", trigger: #{port: "object_signal", selector: "rung"}, writes: ["listener"]},
    ]}
}
fn start() {
    if this.state.root.started { return; }
    this.state.root.started = true;
    this.requests.push(request("body_query", #{reply: #{request: 0, selector: "bell_probe::ring/found"},
        origin: this.trigger.position, radius: 4000.0, limit: 5, include_self: true}));
}
fn found() {
    if this.trigger.stage != "applied" { return; }
    let bell = ();
    let others = [];
    for target in this.observe.body_query_result.targets {
        if target.driven { bell = target.object; } else if others.len() < 2 { others.push(target.object); }
    }
    if bell == () { throw "the query found no driven body"; }
    // Each listener hears the bell: its subscription names the bell's object,
    // and its method's entry names the topic.
    let n = 0;
    for listener in others {
        this.state.root.listeners.push(listener);
        this.requests.push(request("attach_behaviors", #{target: listener, bindings: [#{key: "listener",
            references: #{bell: bell}, methods: [#{name: "hear", entry: "bell_probe::ring/hear"}],
            subscriptions: [#{key: "bell", source: "bell", method: "hear"}]}],
            reply: #{request: n, selector: "bell_probe::ring/attached"}}));
        n += 1;
    }
}
fn attached() {
    if this.trigger.stage == "rejected" { throw this.trigger.message; }
    if !this.trigger.terminal { return; }
    this.state.root.attached += 1;
    if this.state.root.attached != this.state.root.listeners.len() { return; }
    // The bell (the body this runs for) rings once; a topic no entry could
    // name is refused before anyone hears it.
    this.requests.push(request("emit_signal", #{topic: "rung", data: #{peal: 3},
        reply: #{request: 0, selector: "bell_probe::ring/rang"}}));
    this.requests.push(request("emit_signal", #{topic: "ambition.objects.defeated@1", data: (),
        reply: #{request: 1, selector: "bell_probe::ring/forged"}}));
}
fn rang() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage != "applied" { throw this.trigger.code; }
    this.state.root.rung += 1;
    this.state.root.heard_by = this.trigger.subjects.len();
}
fn forged() {
    if !this.trigger.terminal { return; }
    this.state.root.refused = this.trigger.code;
}
fn hear() {
    this.state.listener.heard += 1;
    this.state.listener.from = this.trigger.from;
    this.state.listener.peal = this.trigger.data.peal;
}
"#;

/// ⭐ AN AUTHORED SIGNAL REACHES EVERY SUBSCRIBER, OR NONE. A script emits a
/// named message from the object it runs for; the runtime admits it to each
/// method subscribed to that object whose entry names the topic, answers the
/// sender with who heard it, and refuses a topic no entry could name.
#[test]
fn an_emitted_signal_is_heard_once_by_each_subscriber_and_names_its_sender() {
    let dir = std::env::temp_dir().join(format!("ambition_signal_probe_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bell_probe.rhai");
    std::fs::write(&path, SIGNAL_SOURCE).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("basement_enemies")
            .with_extension_module_files(vec![path.clone()])).unwrap();
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    world.entity_mut(player).insert(BodyStatuses(vec!["bell_probe.start".into()]));
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 40);

    let root = SchemaKey::new("bell_probe", "root", 1);
    let listener = SchemaKey::new("bell_probe", "listener", 1);
    let record = sim.world().get::<BodyRecords>(player).unwrap().get(&root).cloned().expect("the probe started");
    let Value::Seq(listeners) = record.get(FieldRef(1)).unwrap() else { panic!("the probe lists its listeners") };
    assert_eq!(listeners.len(), 2, "the premise: the gallery has two bodies near the player");
    assert_eq!(record.get(FieldRef(3)).unwrap(), &Value::U32(1), "the signal was admitted");
    assert_eq!(record.get(FieldRef(4)).unwrap(), &Value::U32(2), "the sender is told both heard it");
    assert_eq!(record.get(FieldRef(5)).unwrap(), &Value::Text("invalid_signal_topic".into()),
        "a topic spelled like a native port is refused");
    let bell = sim.world().get::<ambition_platformer2d::platformer::lifecycle::ObjectOccurrence>(player).map(|occurrence| {
        let session = sim.world().get::<ambition_platformer2d::platformer::lifecycle::SessionScopedEntity>(player).unwrap().0;
        occurrence.reference(session)
    }).expect("the bell has an occurrence");
    let world = sim.world_mut();
    let heard: Vec<_> = world.query::<&BodyRecords>().iter(world).filter_map(|records| records.get(&listener).cloned()).collect();
    assert_eq!(heard.len(), 2, "two listeners");
    for row in heard {
        assert_eq!(row.get(FieldRef(0)).unwrap(), &Value::U32(1), "heard once");
        assert_eq!(row.get(FieldRef(1)).unwrap(), &Value::Opt(Some(Box::new(Value::Object(bell)))), "the runtime names the bell");
        assert_eq!(row.get(FieldRef(2)).unwrap(), &Value::U32(3), "the data arrives");
    }
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}

const BROKEN_SOURCE: &str = r#"
fn declare() {
    #{provider: "broken_probe", key: "watch", schemas: [
        #{key: "root", fields: [
            #{tag: 1, name: "started", kind: "bool"},
            #{tag: 2, name: "me", kind: "opt object"},
            #{tag: 3, name: "barrier", kind: "opt object"},
            #{tag: 4, name: "attached", kind: "u32"},
        ]},
        #{key: "watcher", fields: [
            #{tag: 1, name: "heard", kind: "u32"},
            #{tag: 2, name: "spent", kind: "opt object"},
        ]}
    ], entries: [
        #{key: "start", phase: "technique_execution", trigger: #{port: "status_tick", selector: "broken_probe.start"},
          writes: ["root"], requests: ["body_query"], limit: 1},
        #{key: "me", phase: "actor_think", trigger: #{port: "action_result", selector: "me"},
          reads: ["body_query_result"], writes: ["root"], requests: ["resolve_authored_objects"], limit: 1},
        #{key: "located", phase: "actor_think", trigger: #{port: "action_result", selector: "located"},
          writes: ["root"], requests: ["attach_behaviors"], limit: 1},
        #{key: "attached", phase: "actor_think", trigger: #{port: "action_result", selector: "attached"}, writes: ["root"]},
        #{key: "down", phase: "actor_think", trigger: #{port: "object_broken", selector: "broken"}, writes: ["watcher"]},
    ]}
}
fn start() {
    if this.state.root.started { return; }
    this.state.root.started = true;
    this.requests.push(request("body_query", #{reply: #{request: 0, selector: "broken_probe::watch/me"},
        origin: this.trigger.position, radius: 64.0, limit: 1, include_self: true}));
}
fn me() {
    if !this.trigger.terminal { return; }
    for target in this.observe.body_query_result.targets {
        if target.driven { this.state.root.me = target.object; }
    }
    this.requests.push(request("resolve_authored_objects", #{placements: ["switch_lab_barrier"],
        reply: #{request: 0, selector: "broken_probe::watch/located"}}));
}
fn located() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage != "applied" { throw this.trigger.code; }
    let barrier = this.trigger.subjects[0].object;
    this.state.root.barrier = barrier;
    this.requests.push(request("attach_behaviors", #{target: this.state.root.me, bindings: [#{key: "watch",
        references: #{barrier: barrier}, methods: [#{name: "down", entry: "broken_probe::watch/down"}],
        subscriptions: [#{key: "barrier_down", source: "barrier", method: "down"}]}],
        reply: #{request: 0, selector: "broken_probe::watch/attached"}}));
}
fn attached() {
    if this.trigger.stage == "rejected" { throw this.trigger.message; }
    if this.trigger.terminal { this.state.root.attached += 1; }
}
fn down() {
    this.state.watcher.heard += 1;
    this.state.watcher.spent = this.trigger.object;
}
"#;

/// ⭐ A BROKEN PROP IS A FACT A SCRIPT CAN SUBSCRIBE TO. The breakable owner
/// commits it on the tick the prop breaks, by whatever road (here a strike);
/// a method subscribed to that object hears it once, named.
#[test]
fn a_subscribed_method_hears_its_barrier_break_once() {
    let path = std::env::temp_dir().join(format!("ambition_broken_probe_{}.rhai", std::process::id()));
    std::fs::write(&path, BROKEN_SOURCE).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("switch_lab")
            .with_extension_module_files(vec![path.clone()])).unwrap();
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    world.entity_mut(player).insert(BodyStatuses(vec!["broken_probe.start".into()]));
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 30);

    let root = SchemaKey::new("broken_probe", "root", 1);
    let watcher = SchemaKey::new("broken_probe", "watcher", 1);
    let record = sim.world().get::<BodyRecords>(player).unwrap().get(&root).cloned().expect("the probe started");
    assert_eq!(record.get(FieldRef(3)).unwrap(), &Value::U32(1), "the watch is attached");
    let Value::Opt(Some(barrier)) = record.get(FieldRef(2)).unwrap() else { panic!("the probe found the barrier") };
    let barrier = barrier.as_object().unwrap();
    let session = ambition_platformer2d::platformer::lifecycle::SessionScopeId(barrier.session);
    let entity = ambition_platformer2d::platformer::lifecycle::resolve_object(sim.world(), session, barrier).unwrap();
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    // A strike that names the barrier: its volume is not read.
    let volume = sim.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(player).unwrap().aabb().into();
    sim.world_mut().write_message(HitEvent {
        origin: None, properties: Default::default(), contact: None, pieces: Vec::new(), strike_sfx: None,
        volume, damage: 999, source: HitSource::Contact, attacker: Some(player), room: None,
        target: HitTarget::Feature(entity), mode: HitMode::Knockback, knockback: None, ignored_targets: Vec::new(),
        attacker_move_instance: None,
    });
    sim.step_n(AgentAction::default(), 30);

    let world = sim.world_mut();
    let heard: Vec<_> = world.query::<&BodyRecords>().iter(world).filter_map(|records| records.get(&watcher).cloned()).collect();
    assert_eq!(heard.len(), 1, "one watch instance");
    assert_eq!(heard[0].get(FieldRef(0)).unwrap(), &Value::U32(1), "one break, heard once");
    assert_eq!(heard[0].get(FieldRef(1)).unwrap(), &Value::Opt(Some(Box::new(Value::Object(barrier)))), "the fact names the barrier");
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_file(path);
}

const REENTRY_SOURCE: &str = r#"
fn declare() {
    #{provider: "reentry_probe", key: "watch", schemas: [
        #{key: "root", fields: [
            #{tag: 1, name: "begun", kind: "u32"},
            #{tag: 2, name: "me", kind: "opt object"},
            #{tag: 3, name: "barriers", kind: "seq 4 object"},
            #{tag: 4, name: "attached", kind: "u32"},
        ]},
        #{key: "watcher", fields: [
            #{tag: 1, name: "heard", kind: "u32"},
            #{tag: 2, name: "spent", kind: "opt object"},
        ]}
    ], entries: [
        #{key: "start", phase: "technique_execution", trigger: #{port: "status_tick", selector: "reentry_probe.start"},
          writes: ["root"], requests: ["body_query"], limit: 1},
        #{key: "again", phase: "technique_execution", trigger: #{port: "status_tick", selector: "reentry_probe.again"},
          writes: ["root"], requests: ["body_query"], limit: 1},
        #{key: "me", phase: "actor_think", trigger: #{port: "action_result", selector: "me"},
          reads: ["body_query_result"], writes: ["root"], requests: ["resolve_authored_objects"], limit: 1},
        #{key: "located", phase: "actor_think", trigger: #{port: "action_result", selector: "located"},
          writes: ["root"], requests: ["attach_behaviors"], limit: 1},
        #{key: "attached", phase: "actor_think", trigger: #{port: "action_result", selector: "attached"}, writes: ["root"]},
        #{key: "down", phase: "actor_think", trigger: #{port: "object_broken", selector: "broken"}, writes: ["watcher"]},
    ]}
}
fn start() {
    if this.state.root.begun >= 1 { return; }
    this.state.root.begun = 1;
    this.requests.push(request("body_query", #{reply: #{request: 0, selector: "reentry_probe::watch/me"},
        origin: this.trigger.position, radius: 64.0, limit: 1, include_self: true}));
}
fn again() {
    if this.state.root.begun >= 2 { return; }
    this.state.root.begun = 2;
    this.requests.push(request("body_query", #{reply: #{request: 1, selector: "reentry_probe::watch/me"},
        origin: this.trigger.position, radius: 64.0, limit: 1, include_self: true}));
}
fn me() {
    if !this.trigger.terminal { return; }
    for target in this.observe.body_query_result.targets {
        if target.driven { this.state.root.me = target.object; }
    }
    // The placement is looked up in the live room the caller is in now.
    this.requests.push(request("resolve_authored_objects", #{placements: ["switch_lab_barrier"],
        reply: #{request: this.state.root.begun, selector: "reentry_probe::watch/located"}}));
}
fn located() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage != "applied" { throw this.trigger.code; }
    let barrier = this.trigger.subjects[0].object;
    this.state.root.barriers.push(barrier);
    this.requests.push(request("attach_behaviors", #{target: this.state.root.me, bindings: [#{key: "watch" + this.state.root.begun,
        references: #{barrier: barrier}, methods: [#{name: "down", entry: "reentry_probe::watch/down"}],
        subscriptions: [#{key: "barrier_down", source: "barrier", method: "down"}],
        state: [#{schema: "watcher", value: #{heard: 0, spent: ()}}]}],
        reply: #{request: this.state.root.begun, selector: "reentry_probe::watch/attached"}}));
}
fn attached() {
    if this.trigger.stage == "rejected" { throw this.trigger.message; }
    if this.trigger.terminal { this.state.root.attached += 1; }
}
fn down() {
    this.state.watcher.heard += 1;
    this.state.watcher.spent = this.trigger.object;
}
"#;

/// ⭐ A ROOM ENTERED AGAIN IS ANOTHER COPY, AND A SUBSCRIPTION NAMES ONE. The
/// same script subscribes to `switch_lab`'s barrier, the body leaves and comes
/// back (a new live room with equal placement ids), and subscribes again by
/// the same placement. The lookup names the new copy's barrier, not the old
/// one. Both watches travel with the body; when the new barrier breaks, the
/// watch made in the new copy hears it, once, named, and the one made in the
/// old copy, subscribed to an occurrence that left with its room, hears
/// nothing.
#[test]
fn a_room_entered_again_is_a_new_copy_and_only_its_own_watch_hears_its_barrier() {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    let path = std::env::temp_dir().join(format!("ambition_reentry_probe_{}.rhai", std::process::id()));
    std::fs::write(&path, REENTRY_SOURCE).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("switch_lab")
            .with_extension_module_files(vec![path.clone()])).unwrap();
    let player = {
        let world = sim.world_mut();
        world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap()
    };
    sim.world_mut().entity_mut(player).insert(BodyStatuses(vec!["reentry_probe.start".into()]));
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 30);
    let root = SchemaKey::new("reentry_probe", "root", 1);
    let watcher = SchemaKey::new("reentry_probe", "watcher", 1);
    let record = |sim: &Platformer2dSimHarness| sim.world().get::<BodyRecords>(player).unwrap().get(&root).cloned().expect("the probe started");
    assert_eq!(record(&sim).get(FieldRef(3)).unwrap(), &Value::U32(1), "the first watch is attached");

    let watches = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world.query::<&BodyRecords>().iter(world).filter_map(|records| records.get(&watcher).cloned()).collect::<Vec<_>>()
    };
    assert_eq!(watches(&mut sim).len(), 1, "the first watch is live in the first copy");
    sim.world_mut().entity_mut(player).insert(BodyStatuses(Vec::new()));
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "central_hub_complex"), "central_hub_complex");
    sim.step_n(AgentAction::default(), 10);
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "switch_lab"), "switch_lab");
    sim.step_n(AgentAction::default(), 10);
    assert_eq!(watches(&mut sim).len(), 1, "the first watch travels with the body");
    sim.world_mut().entity_mut(player).insert(BodyStatuses(vec!["reentry_probe.again".into()]));
    sim.step_n(AgentAction::default(), 30);

    let record = record(&sim);
    assert_eq!(record.get(FieldRef(3)).unwrap(), &Value::U32(2), "the second watch is attached");
    let Value::Seq(barriers) = record.get(FieldRef(2)).unwrap() else { panic!("the probe lists the barriers") };
    let (old, new) = (barriers[0].as_object().unwrap(), barriers[1].as_object().unwrap());
    assert_ne!(old, new, "the placement names another occurrence in the room entered again");
    let session = ambition_platformer2d::platformer::lifecycle::SessionScopeId(new.session);
    assert!(ambition_platformer2d::platformer::lifecycle::resolve_object(sim.world(), session, old).is_err(),
        "the old copy's barrier left with its room");
    let entity = ambition_platformer2d::platformer::lifecycle::resolve_object(sim.world(), session, new).unwrap();
    let volume = sim.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(player).unwrap().aabb().into();
    sim.world_mut().write_message(HitEvent {
        origin: None, properties: Default::default(), contact: None, pieces: Vec::new(), strike_sfx: None,
        volume, damage: 999, source: HitSource::Contact, attacker: Some(player), room: None,
        target: HitTarget::Feature(entity), mode: HitMode::Knockback, knockback: None, ignored_targets: Vec::new(),
        attacker_move_instance: None,
    });
    sim.step_n(AgentAction::default(), 30);

    let watches = watches(&mut sim);
    let heard: Vec<_> = watches.iter().map(|row| (row.get(FieldRef(0)).unwrap().clone(), row.get(FieldRef(1)).unwrap().clone())).collect();
    assert_eq!(watches.len(), 2, "both watches: {heard:?}");
    let new_heard = (Value::U32(1), Value::Opt(Some(Box::new(Value::Object(new)))));
    let old_silent = (Value::U32(0), Value::Opt(None));
    assert!(heard.contains(&new_heard) && heard.contains(&old_silent),
        "the new copy's watch heard its barrier once, and the old copy's heard nothing: {heard:?}");
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_file(path);
}
