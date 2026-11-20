//! A script opens and shuts an authored gate through the gate's own owner: the
//! wall waits on a world value (`world.value_is`), the script writes the value
//! and hears the write's outcome, and `lock_walls` says whether the wall stands.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}};
use ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const SOURCE: &str = r#"
fn declare() {
    #{provider: "gate_probe", key: "keeper", schemas: [
        #{key: "gate", fields: [
            #{tag: 1, name: "step", kind: "u32"},
            #{tag: 2, name: "trail", kind: "text 128"},
            #{tag: 3, name: "answers", kind: "text 128"},
        ]},
    ], entries: [
        #{key: "tick", phase: "actor_think", trigger: #{port: "story_tick", selector: "blink_run"},
          reads: ["lock_walls"], writes: ["gate"], requests: ["set_world_value"], limit: 1},
        #{key: "answer", phase: "actor_think", trigger: #{port: "action_result", selector: "answer"}, writes: ["gate"]},
    ]}
}
fn standing(walls) {
    for wall in walls {
        if wall.id == "probe_gate" { return if wall.standing { "S" } else { "o" }; }
    }
    "?"
}
fn tick() {
    if !this.trigger.flags.contains("gate_probe_begin") { return; }
    let gate = this.state.gate;
    gate.step += 1;
    // Note the wall on steps 5, 15, 25, 35 and 45; open it at 10, shut it at 30.
    if gate.step % 10 == 5 { gate.trail += standing(this.observe.lock_walls.walls); }
    if gate.step == 10 {
        this.requests.push(request("set_world_value", #{key: "probe.gate", value: "open",
            reply: #{request: 1, selector: "gate_probe::keeper/answer"}}));
    }
    if gate.step == 30 {
        this.requests.push(request("set_world_value", #{key: "probe.gate", value: "shut",
            reply: #{request: 2, selector: "gate_probe::keeper/answer"}}));
    }
    this.state.gate = gate;
}
fn answer() {
    if this.trigger.terminal { this.state.gate.answers += this.trigger.code + ";"; }
}
"#;

/// ⭐ The wall stands until the world value opens it, stands again when the
/// value shuts it, and the script reads each state. Sync-test host.
#[test]
fn a_script_opens_and_shuts_a_gated_wall_through_a_world_value() {
    use ambition_platformer2d::world::rooms::{EncounterLockWallSpec, RoomSet};
    let dir = std::env::temp_dir().join(format!("ambition_lock_walls_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("keeper.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options("blink_run")
        .with_extension_module_files(vec![file])
        .with_sync_test_rollback_settings(4, 12)).unwrap();
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let at = world.query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, PrimaryPlayerOnly>()
        .single(world).unwrap().pos;
    {
        let mut rooms = ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<RoomSet>(world).unwrap();
        let index = rooms.room_index_by_id("blink_run").unwrap();
        rooms.rooms[index].lock_walls.push(EncounterLockWallSpec {
            id: "probe_gate".into(), gated_by: Some("world.value_is probe.gate open".into()), look: None,
            min: at + ambition_platformer2d::engine_core::Vec2::new(300.0, -400.0),
            size: ambition_platformer2d::engine_core::Vec2::new(16.0, 64.0),
        });
    }
    world.resource_mut::<ambition_platformer2d::session::AmbitionGameSave>().data_mut().set_flag("gate_probe_begin", true);
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 50);
    sim.rollback_health().unwrap();
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent.first());
    let world = sim.world_mut();
    let root = world.query_filtered::<Entity, With<RoomInstanceRoot>>().single(world).unwrap();
    let gate = world.get::<BodyRecords>(root).unwrap().get(&SchemaKey::new("gate_probe", "gate", 1)).cloned().unwrap();
    assert_eq!(gate.get(FieldRef(2)).unwrap(), &Value::Text("world_value_set;world_value_set;".into()), "both writes applied");
    assert_eq!(gate.get(FieldRef(1)).unwrap(), &Value::Text("SooSS".into()),
        "standing before, open after the first write, standing again after the second");
    let _ = std::fs::remove_dir_all(dir);
}
