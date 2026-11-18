//! Scripts receive domain results after native work has settled.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults};
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("action_probe", "results", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "action_probe", key: "commands",
       schemas: [#{ key: "results", fields: [
           #{ tag: 1, name: "sent", kind: "bool" },
           #{ tag: 2, name: "applied", kind: "u32" },
           #{ tag: 3, name: "rejected", kind: "u32" },
           #{ tag: 4, name: "wrong", kind: "u32" },
           #{ tag: 5, name: "admitted", kind: "u32" },
           #{ tag: 6, name: "submitted", kind: "u32" },
       ] }],
       entries: [
           #{ key: "tick", phase: "actor_think",
              trigger: #{ port: "story_tick", selector: "intro_wake_room" },
              writes: ["results"], requests: ["story_command"], limit: 4 },
           #{ key: "result", phase: "actor_think",
              trigger: #{ port: "action_result", selector: "result" },
              writes: ["results"] },
           #{ key: "other", phase: "actor_think",
              trigger: #{ port: "action_result", selector: "result" },
              writes: ["results"] },
       ],
    }
}
fn tick() {
    if !this.trigger.flags.contains("action_probe_begin") { return; }
    if this.state.results.sent { return; }
    this.state.results.sent = true;
    this.requests.push(request("story_command", #{ line: "world.set_flag action_probe_applied true",
        reply: #{ request: 0, selector: "action_probe::commands/result" } }));
    this.requests.push(request("story_command", #{ line: "world.set_flag",
        reply: #{ request: 1, selector: "action_probe::commands/result" } }));
    this.requests.push(request("story_command", #{ line: "world.set_flag action_probe_redirected true",
        reply: #{ request: 2, selector: "ambition::other/result" } }));
}
fn result() {
    let state = this.state.results;
    if this.trigger.stage == "applied" { state.applied += 1; }
    if this.trigger.stage == "rejected" { state.rejected += 1; }
    if this.trigger.stage == "admitted" { state.admitted += 1; }
    if this.trigger.stage == "submitted" { state.submitted += 1; }
    this.state.results = state;
}
fn other() { this.state.results.wrong += 1; }
"#;

#[derive(Resource)]
struct DelayedOccurrences {
    first: ambition_platformer2d::platformer::action_results::ActionReply,
    second: ambition_platformer2d::platformer::action_results::ActionReply,
    applications: usize,
}

fn complete_occurrences_on_the_replayed_timeline(world: &mut World) {
    use ambition_platformer2d::{platformer::{action_results::*, lifecycle::*, sim_id::SimId},
        extension::sdk::action::{ActionStage, ReplyTo}};
    let frame = world.resource::<ambition_platformer2d::rollback::RollbackFrameCount>().0;
    if ![5, 8, 10].contains(&frame) { return; }
    let (first, second) = {
        let pending = world.resource::<DelayedOccurrences>();
        (pending.first.clone(), pending.second.clone())
    };
    if frame == 5 {
        let body = first.recipient.resolve(world).unwrap();
        world.entity_mut(body).insert(InRoomInstance(LiveRoomInstance::from_ordinal(3)));
    }
    if frame == 8 {
        let body = second.recipient.resolve(world).unwrap();
        let owner = *world.get::<SessionScopedEntity>(body).unwrap();
        let room = *world.get::<InRoomInstance>(body).unwrap();
        world.despawn(body);
        let replacement = world.spawn((owner, room, SimId::placement("repeated_object"))).id();
        let reply = begin_action(world, replacement, "occurrence_probe::actions/ask", "test.delay",
            Some(&ReplyTo {request: 0, selector: "occurrence_probe::actions/result".into()}), 0).reply.unwrap();
        assert!(publish_action_result(world, Some(&reply), ActionStage::Applied, true, "replacement", "", Vec::new()));
    }
    if frame == 10 {
        let mut state = bevy::ecs::system::SystemState::<ActionReplies>::new(world);
        let mut replies = state.get_mut(world).unwrap();
        assert!(replies.publish(Some(&first), ActionStage::Applied, true, "traveled", "", Vec::new()));
        assert!(!replies.publish(Some(&second), ActionStage::Applied, true, "retired", "", Vec::new()));
        world.resource_mut::<DelayedOccurrences>().applications += 1;
    }
}

#[test]
fn equal_authored_object_ids_receive_independent_replies_after_travel_and_rollback() {
    use ambition_platformer2d::{platformer::{action_results::*, lifecycle::*, sim_id::SimId},
        extension::sdk::action::ReplyTo, engine_core::RoomGeometry, world::rooms::{LiveRoomDefinition, RoomSet}};
    const SCRIPT: &str = r#"
fn declare() {
    #{provider: "occurrence_probe", key: "actions", schemas: [#{key: "memory", fields: [
        #{tag: 1, name: "completed", kind: "u32"}, #{tag: 2, name: "code", kind: "text 32"}]}],
      entries: [
        #{key: "ask", phase: "actor_think", trigger: #{port: "story_tick", selector: "unused_occurrence_selector"}},
        #{key: "result", phase: "actor_think", trigger: #{port: "action_result", selector: "result"}, writes: ["memory"]}]}
}
fn ask() {}
fn result() {
    if !this.trigger.terminal { return; }
    this.state.memory.completed += 1;
    this.state.memory.code = this.trigger.code;
}
"#;
    let file = std::env::temp_dir().join(format!("ambition_occurrence_replies_{}.rhai", std::process::id()));
    std::fs::write(&file, SCRIPT).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("switch_lab").with_extension_module_files(vec![file.clone()])
            .with_sync_test_rollback_settings(4, 12)).unwrap();
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let root = world.query_filtered::<Entity, With<RoomInstanceRoot>>().single(world).unwrap();
    let owner = *world.get::<SessionScopedEntity>(root).unwrap();
    let geometry = world.get::<RoomGeometry>(root).unwrap().clone();
    let definition = *world.get::<LiveRoomDefinition>(root).unwrap();
    for n in 1..4 {
        let room = LiveRoomInstance::from_ordinal(n);
        assert!(session_world_component_mut::<RoomSet>(world).unwrap().mint_live_room(room));
        let extra = spawn_live_room(world, room, geometry.clone());
        world.entity_mut(extra).insert(definition);
    }
    let a = world.spawn((owner, InRoomInstance(LiveRoomInstance::ACTIVATION), SimId::placement("repeated_object"))).id();
    let b = world.spawn((owner, InRoomInstance(LiveRoomInstance::from_ordinal(1)), SimId::placement("repeated_object"))).id();
    let untracked = world.spawn((owner, InRoomInstance(LiveRoomInstance::from_ordinal(2)), SimId::placement("repeated_object"))).id();
    let ask = |world: &mut World, body| begin_action(world, body, "occurrence_probe::actions/ask", "test.delay",
        Some(&ReplyTo {request: 0, selector: "occurrence_probe::actions/result".into()}), 0).reply.unwrap();
    let first = ask(world, a);
    let second = ask(world, b);
    world.insert_resource(DelayedOccurrences {first: first.clone(), second: second.clone(), applications: 0});
    sim.app_mut().add_systems(ambition_platformer2d::rollback::GgrsSchedule,
        complete_occurrences_on_the_replayed_timeline.before(ambition_platformer2d::sim::Platformer2dSimulationPhase::CoreSimulation));
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 30);
    sim.rollback_health().unwrap();
    assert!(sim.world().resource::<DelayedOccurrences>().applications > 1);
    assert_eq!(second.recipient.resolve(sim.world()), None);
    assert!(sim.world().get::<ActionMailbox>(untracked).is_none());
    let world = sim.world_mut();
    let mut receivers = world.query::<(&ObjectOccurrence, &BodyRecords)>();
    let rows: Vec<_> = receivers.iter(world).filter_map(|(occurrence, records)| {
        let record = records.get(&SchemaKey::new("occurrence_probe", "memory", 1))?;
        assert_eq!(record.get(FieldRef(0)).unwrap(), &Value::U32(1));
        Some((*occurrence, record.get(FieldRef(1)).unwrap().as_text().unwrap().to_owned()))
    }).collect();
    assert_eq!(rows.len(), 2);
    assert!(rows.contains(&(first.recipient.occurrence, "traveled".into())));
    assert!(rows.iter().any(|(_, code)| code == "replacement"));
    assert_eq!(world.resource::<ExtensionFaults>().total, 0);
    let _ = std::fs::remove_file(file);
}

#[test]
fn scripts_receive_native_application_and_refusal_in_the_asking_scope_after_rewind() {
    let dir = std::env::temp_dir().join(format!("ambition_action_results_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("commands.rhai");
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
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::session::AmbitionGameSave>()
        .data_mut()
        .set_flag("action_probe_begin", true);
    sim.rebase_rollback_history().unwrap();
    for _ in 0..30 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .resource::<ambition_platformer2d::session::AmbitionGameSave>()
            .data()
            .flag("action_probe_applied")
        {
            break;
        }
    }
    assert_eq!(
        counter(sim.world_mut(), 1),
        Value::U32(0),
        "application does not deliver in the same read cut"
    );
    let world = sim.world_mut();
    use ambition_platformer2d::platformer::lifecycle::{RoomInstanceRoot, RoomPace};
    let until = world.resource::<ambition_platformer2d::time::SimTick>().0 + 4;
    let roots: Vec<_> = world
        .query_filtered::<Entity, With<RoomInstanceRoot>>()
        .iter(world)
        .collect();
    for root in roots {
        let pace = world.get::<RoomPace>(root).copied().unwrap_or_default();
        world.entity_mut(root).insert(RoomPace {
            held_until: until,
            ..pace
        });
    }
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 3);
    assert_eq!(
        counter(sim.world_mut(), 1),
        Value::U32(0),
        "a held room retains its pending replies"
    );
    sim.step_n(AgentAction::default(), 12);
    let world = sim.world_mut();
    let record = world
        .query::<&BodyRecords>()
        .iter(world)
        .find_map(|records| records.get(&RECORD))
        .expect("the room root owns the script state");
    assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1));
    assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U32(1));
    assert_eq!(
        record.get(FieldRef(3)).unwrap(),
        &Value::U32(0),
        "shared selectors do not broadcast directed replies"
    );
    assert_eq!(record.get(FieldRef(4)).unwrap(), &Value::U32(1));
    assert_eq!(record.get(FieldRef(5)).unwrap(), &Value::U32(2));
    let save = world.resource::<ambition_platformer2d::session::AmbitionGameSave>();
    assert!(save.data().flag("action_probe_applied"));
    assert!(!save.data().flag("action_probe_redirected"));
    assert_eq!(world.resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}

fn counter(world: &mut World, field: usize) -> Value {
    world
        .query::<&BodyRecords>()
        .iter(world)
        .find_map(|records| records.get(&RECORD))
        .unwrap()
        .get(FieldRef(field))
        .unwrap()
        .clone()
}
