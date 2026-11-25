//! W2: one scripted request builds a mechanism from parts of two native
//! domains: a lift the platform domain moves and a switch that sends it. The
//! switch names its lift by local key, so two copies made with the same keys
//! bind each to its own lift.

#![cfg(feature = "rl_sim")]
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    encounter::switches::SwitchFeature,
    engine_core::geometry::CenteredAabb,
    extension::{
        sdk::{FieldRef, SchemaKey, Value},
        BodyRecords,
    },
    platformer::lifecycle::RoomInstanceRoot,
    world::{collision::MovingPlatformSet, platforms::LiftStage},
};
use bevy::prelude::*;

const ROOM: &str = r#"
fn declare() {
    #{provider: "mechanism_probe", key: "yard", schemas: [
        #{key: "memory", fields: [
            #{tag: 1, name: "sent", kind: "bool"},
            #{tag: 2, name: "created", kind: "u32"},
            #{tag: 3, name: "refused", kind: "u32"},
            #{tag: 4, name: "message", kind: "text 256"},
            #{tag: 5, name: "first", kind: "opt object"},
            #{tag: 6, name: "retired", kind: "bool"},
        ]},
    ], entries: [
        #{key: "tick", phase: "actor_think", trigger: #{port: "story_tick", selector: "intro_wake_room"},
          writes: ["memory"], requests: ["create_mechanism", "retire_creation"], limit: 4},
        #{key: "result", phase: "actor_think", trigger: #{port: "action_result", selector: "result"}, writes: ["memory"]},
    ]}
}
// One lift and the switch beside it that sends it, made at `x`.
fn lift_and_call(request, x, floor, sends) {
    #{reply: #{request: request, selector: "mechanism_probe::yard/result"},
      lifts: [#{key: "lift", at: [x, floor - 8.0], size: [96.0, 16.0], rise: -160.0, speed: 600.0}],
      switches: [#{key: "call", at: [x - 90.0, floor - 16.0], sends: #{local: sends}}]}
}
fn tick() {
    // The first copy's switch leaves the world; the second copy keeps working.
    if this.trigger.flags.contains("mechanism_retire") && !this.state.memory.retired {
        this.state.memory.retired = true;
        this.requests.push(request("retire_creation", #{object: this.state.memory.first,
            reply: #{request: 9, selector: "mechanism_probe::yard/result"}}));
        return;
    }
    if !this.trigger.flags.contains("mechanism_begin") || this.state.memory.sent { return; }
    this.state.memory.sent = true;
    let p = this.trigger.bodies[0].position;
    for body in this.trigger.bodies { if body.driven { p = body.position; } }
    this.requests.push(request("create_mechanism", lift_and_call(1, p[0] + 300.0, p[1] + 24.0, "lift")));
    this.requests.push(request("create_mechanism", lift_and_call(2, p[0] + 700.0, p[1] + 24.0, "lift")));
    this.requests.push(request("create_mechanism", lift_and_call(3, p[0] + 1100.0, p[1] + 24.0, "nowhere")));
}
fn result() {
    if !this.trigger.terminal { return; }
    if this.trigger.code == "mechanism_created" {
        this.state.memory.created += 1;
        if this.trigger.ticket.request == 1 { this.state.memory.first = this.trigger.subjects[0].object; }
    }
    if this.trigger.code == "mechanism_refused" {
        this.state.memory.refused += 1;
        this.state.memory.message = this.trigger.message;
    }
}
"#;

fn switches(sim: &mut Platformer2dSimHarness) -> Vec<(String, String, (f32, f32))> {
    let world = sim.world_mut();
    let mut q = world.query::<(&SwitchFeature, &CenteredAabb)>();
    let mut rows: Vec<_> = q
        .iter(world)
        .filter(|(feature, _)| feature.activation.action == "MoveLift")
        .map(|(feature, aabb)| (feature.activation.id.clone(), feature.activation.target_encounter.clone(), (aabb.center.x, aabb.center.y)))
        .collect();
    rows.sort_by(|a, b| a.2 .0.total_cmp(&b.2 .0));
    rows
}

fn lift(sim: &mut Platformer2dSimHarness, id: &str) -> Option<(f32, Option<LiftStage>)> {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&MovingPlatformSet, With<RoomInstanceRoot>>();
    q.iter(world)
        .flat_map(|set| set.0.iter())
        .find(|platform| platform.id == id)
        .map(|platform| (platform.pos.y, platform.lift_stage()))
}

/// ⭐ Two copies of one mechanism, made with the same local keys in one room,
/// each bind their switch to their own lift; a press on the first sends only
/// its lift. A third request whose switch names a key the request does not
/// make is refused whole, saying which switch, which field and which key.
/// It runs under sync-test rollback, so every step is resimulated and checked.
#[test]
fn a_scripted_mechanism_binds_each_switch_to_its_own_lift() {
    let dir = std::env::temp_dir().join(format!("ambition_mechanism_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("yard.rhai");
    std::fs::write(&file, ROOM).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room")
            .with_save(save)
            .with_extension_module_files(vec![file])
            .with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    sim.step_n(AgentAction::default(), 4);
    let platforms_before = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&MovingPlatformSet, With<RoomInstanceRoot>>();
        q.iter(world).map(|set| set.0.len()).sum::<usize>()
    };
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::session::AmbitionGameSave>()
        .data_mut()
        .set_flag("mechanism_begin", true);
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 12);

    let root = {
        let world = sim.world_mut();
        world.query_filtered::<Entity, With<RoomInstanceRoot>>().single(world).unwrap()
    };
    let memory = sim
        .world()
        .get::<BodyRecords>(root)
        .unwrap()
        .get(&SchemaKey::new("mechanism_probe", "memory", 1))
        .unwrap()
        .clone();
    assert_eq!(memory.get(FieldRef(1)).ok(), Some(&Value::U32(2)), "both good mechanisms were made");
    assert_eq!(memory.get(FieldRef(2)).ok(), Some(&Value::U32(1)), "the mechanism with a dangling key was refused");
    let message = memory.get(FieldRef(3)).ok().and_then(|value| value.as_text()).unwrap_or_default().to_owned();
    assert!(
        message.contains("\"call\"") && message.contains("sends") && message.contains("\"nowhere\""),
        "the refusal names the switch, the field and the key: {message:?}"
    );

    let made = switches(&mut sim);
    assert_eq!(made.len(), 2, "two switches, and none from the refused request: {made:?}");
    let platforms_after = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&MovingPlatformSet, With<RoomInstanceRoot>>();
        q.iter(world).map(|set| set.0.len()).sum::<usize>()
    };
    assert_eq!(platforms_after, platforms_before + 2, "two lifts, and none from the refused request");
    let (first, second) = (&made[0], &made[1]);
    assert_ne!(first.1, second.1, "both switches send the same lift: {made:?}");
    let first_rest = lift(&mut sim, &first.1).expect("the first switch's lift is in the room");
    let second_rest = lift(&mut sim, &second.1).expect("the second switch's lift is in the room");
    assert_eq!(first_rest.1, Some(LiftStage::Resting), "setup: the first lift waits");

    sim.teleport_player(first.2);
    sim.step(crate::common::base());
    sim.step(AgentAction { interact: true, ..crate::common::base() });
    for _ in 0..40 {
        sim.step(crate::common::base());
    }
    assert_eq!(
        lift(&mut sim, &first.1),
        Some((first_rest.0 - 160.0, Some(LiftStage::Parked))),
        "the pressed switch did not send its own lift to the far stop"
    );
    assert_eq!(lift(&mut sim, &second.1), Some(second_rest), "the press sent the other copy's lift");

    // Retire the first copy's switch (still under sync-test rollback): it
    // leaves, its lift stays parked, and the second copy's switch still sends
    // only its own lift.
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::session::AmbitionGameSave>()
        .data_mut()
        .set_flag("mechanism_retire", true);
    // A rewind restores the save, so the flag joins the history as a new base.
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 8);
    let left = switches(&mut sim);
    assert_eq!(left.iter().map(|row| &row.0).collect::<Vec<_>>(), vec![&second.0], "only the first copy's switch left");
    assert_eq!(lift(&mut sim, &first.1).map(|(_, stage)| stage), Some(Some(LiftStage::Parked)), "retiring the switch moved its lift");
    sim.teleport_player(second.2);
    sim.step(crate::common::base());
    sim.step(AgentAction { interact: true, ..crate::common::base() });
    for _ in 0..40 {
        sim.step(crate::common::base());
    }
    assert_eq!(
        lift(&mut sim, &second.1),
        Some((second_rest.0 - 160.0, Some(LiftStage::Parked))),
        "after the other copy's retirement, the second switch did not send its own lift"
    );
    let _ = std::fs::remove_dir_all(dir);
}
