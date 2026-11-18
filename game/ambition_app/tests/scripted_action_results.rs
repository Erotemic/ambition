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
