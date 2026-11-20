//! World values: a script in one room writes state the save keeps, a script
//! in another room reads it, and a session entry hears each change once.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}};
use ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot;
use bevy::prelude::*;

const SOURCE: &str = r#"
fn declare() {
    #{provider: "world_values_probe", key: "ledger", schemas: [
        #{key: "writer", fields: [
            #{tag: 1, name: "step", kind: "u32"},
            #{tag: 2, name: "heard", kind: "u32"},
            #{tag: 3, name: "wrong", kind: "u32"},
        ]},
        #{key: "tally", attachment: "session", fields: [
            #{tag: 1, name: "changes", kind: "u32"},
            #{tag: 2, name: "last", kind: "u32"},
            #{tag: 3, name: "revision", kind: "u32"},
        ]},
        #{key: "seen", fields: [
            #{tag: 1, name: "goblins", kind: "u32"},
            #{tag: 2, name: "bridge", kind: "text 32"},
        ]},
    ], entries: [
        #{key: "write", phase: "actor_think", trigger: #{port: "story_tick", selector: "blink_run"},
          writes: ["writer"], requests: ["set_world_value", "add_world_count"], limit: 6},
        #{key: "answer", phase: "actor_think", trigger: #{port: "action_result", selector: "answer"}, writes: ["writer"]},
        #{key: "tally", phase: "actor_think", trigger: #{port: "world_value_changed", selector: "goblins"}, writes: ["tally"]},
        #{key: "read", phase: "actor_think", trigger: #{port: "story_tick", selector: "portal_bridge"},
          reads: ["world_values"], writes: ["seen"]},
    ]}
}
fn reply(n) { #{request: n, selector: "world_values_probe::ledger/answer"} }
fn write() {
    if !this.trigger.flags.contains("world_values_begin") || this.state.writer.step != 0 { return; }
    this.state.writer.step = 1;
    this.requests.push(request("set_world_value", #{key: "bridge.state", value: "lowered", expect: 0, reply: reply(1)}));
    this.requests.push(request("add_world_count", #{key: "goblins", delta: 2, reply: reply(2)}));
    this.requests.push(request("add_world_count", #{key: "goblins", delta: 3, reply: reply(3)}));
    this.requests.push(request("set_world_value", #{key: "bridge.state", value: "raised", expect: 0, reply: reply(4)}));
    this.requests.push(request("add_world_count", #{key: "bridge.state", delta: 1, reply: reply(5)}));
    this.requests.push(request("set_world_value", #{key: "Bad Key", value: 1, reply: reply(6)}));
}
fn answer() {
    if !this.trigger.terminal { return; }
    let expected = [(), "world_value_set", "world_count_added", "world_count_added",
        "revision_changed", "not_a_count", "invalid_world_key"];
    let n = this.trigger.ticket.request;
    let stage = if n <= 3 { "applied" } else { "rejected" };
    if this.trigger.stage == stage && this.trigger.code == expected[n] {
        this.state.writer.heard = this.state.writer.heard | (1 << n);
    } else {
        this.state.writer.wrong += 1;
    }
}
fn tally() {
    this.state.tally.changes += 1;
    this.state.tally.last = this.trigger.value;
    this.state.tally.revision = this.trigger.revision;
}
fn read() {
    for row in this.observe.world_values.values {
        if row.key == "goblins" { this.state.seen.goblins = row.value; }
        if row.key == "bridge.state" { this.state.seen.bridge = row.value; }
    }
}
"#;

fn room_record(sim: &mut Platformer2dSimHarness, key: &'static str) -> Option<ambition_platformer2d::extension::sdk::Record> {
    let schema = SchemaKey::new("world_values_probe", key, 1);
    let world = sim.world_mut();
    let root = world.query_filtered::<Entity, With<RoomInstanceRoot>>().single(world).unwrap();
    world.get::<BodyRecords>(root)?.get(&schema).cloned()
}

fn session_record(sim: &mut Platformer2dSimHarness) -> Option<ambition_platformer2d::extension::sdk::Record> {
    let schema = SchemaKey::new("world_values_probe", "tally", 1);
    let world = sim.world_mut();
    let records: Vec<_> = world.query::<&ambition_platformer2d::extension::SessionRecords>().iter(world)
        .filter_map(|records| records.get(&schema).cloned()).collect();
    records.into_iter().next()
}

fn boot(room: &str, file: &std::path::Path, save: ambition_platformer2d::session::AmbitionGameSaveData) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options(room)
        .with_save(save)
        .with_extension_module_files(vec![file.to_path_buf()])
        .with_sync_test_rollback_settings(4, 12)).unwrap();
    crate::common::doors_cross_at_once(&mut sim);
    sim
}

/// ⭐ One room writes, another reads, the session hears each change once, and
/// a session started from the save has the values. Sync-test host: each write
/// applies on its own tick, and every re-simulation agrees.
#[test]
fn a_world_value_written_in_one_room_is_read_in_another_and_saved() {
    let dir = std::env::temp_dir().join(format!("ambition_world_values_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("ledger.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("world_values_begin", true);
    let mut sim = boot("blink_run", &file, save);
    sim.step_n(AgentAction::default(), 30);
    let writer = room_record(&mut sim, "writer").expect("the writer ran");
    assert_eq!(writer.get(FieldRef(2)).unwrap(), &Value::U32(0), "every answer is the expected stage and code");
    assert_eq!(writer.get(FieldRef(1)).unwrap(), &Value::U32(0b111_1110), "all six writes were answered");
    let tally = session_record(&mut sim).expect("the session heard the count change");
    assert_eq!(tally.get(FieldRef(0)).unwrap(), &Value::U32(2), "two changes to the count, each heard once");
    assert_eq!(tally.get(FieldRef(1)).unwrap(), &Value::U32(5));
    assert_eq!(tally.get(FieldRef(2)).unwrap(), &Value::U32(2));

    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "portal_bridge"), "portal_bridge");
    sim.step_n(AgentAction::default(), 10);
    let seen = room_record(&mut sim, "seen").expect("the reader ran in the other room");
    assert_eq!(seen.get(FieldRef(0)).unwrap(), &Value::U32(5));
    assert_eq!(seen.get(FieldRef(1)).unwrap(), &Value::Text("lowered".into()));
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    let saved = sim.world().resource::<ambition_platformer2d::session::AmbitionGameSave>().data().clone();
    let keys: Vec<_> = saved.world_values().iter().map(|row| (row.key.as_str(), row.revision)).collect();
    assert_eq!(keys, [("bridge.state", 1), ("goblins", 2)], "the save keeps each value once, at its revision");
    drop(sim);

    // A session started from that save: the reader sees the values at once.
    let mut sim = boot("portal_bridge", &file, saved);
    sim.step_n(AgentAction::default(), 10);
    let seen = room_record(&mut sim, "seen").expect("the reader ran after the load");
    assert_eq!(seen.get(FieldRef(0)).unwrap(), &Value::U32(5));
    assert_eq!(seen.get(FieldRef(1)).unwrap(), &Value::Text("lowered".into()));
    assert!(session_record(&mut sim).is_none(), "a load is not a change");
    let _ = std::fs::remove_dir_all(dir);
}

const ISLAND_SOURCE: &str = r#"
fn declare() {
    #{provider: "world_island_probe", key: "count", schemas: [
        #{key: "root", fields: [
            #{tag: 1, name: "started", kind: "bool"},
            #{tag: 2, name: "line", kind: "f32"},
            #{tag: 3, name: "added", kind: "u32"},
            #{tag: 4, name: "code", kind: "text 32"},
        ]},
        #{key: "tally", attachment: "session", fields: [
            #{tag: 1, name: "changes", kind: "u32"},
            #{tag: 2, name: "last", kind: "u32"},
        ]},
    ], entries: [
        #{key: "tick", phase: "technique_execution", trigger: #{port: "status_tick", selector: "world_island_probe.go"},
          writes: ["root"], requests: ["add_world_count"], limit: 1},
        #{key: "sent", phase: "actor_think", trigger: #{port: "action_result", selector: "sent"}, writes: ["root"]},
        #{key: "tally", phase: "actor_think", trigger: #{port: "world_value_changed", selector: "crossings"}, writes: ["tally"]},
    ]}
}
fn tick() {
    if !this.state.root.started {
        this.state.root.started = true;
        this.state.root.line = this.trigger.position[0] + 24.0;
        return;
    }
    if this.state.root.added == 0 && this.trigger.position[0] > this.state.root.line {
        this.state.root.added += 1;
        this.requests.push(request("add_world_count", #{key: "crossings", delta: 1,
            reply: #{request: 1, selector: "world_island_probe::count/sent"}}));
    }
}
fn sent() {
    if this.trigger.terminal { this.state.root.code = this.trigger.code; }
}
fn tally() {
    this.state.tally.changes += 1;
    this.state.tally.last = this.trigger.value;
}
"#;

/// ⭐ A WRITE FROM A ROOM THAT REWINDS ALONE APPLIES ONCE, AND ONLY WHEN NO
/// REPLAY CAN TAKE ITS TICK BACK. On a host that rewinds guest rooms, Bob's
/// script counts his crossing of a line. A crossing a late input takes back
/// never counts; an on-time one is held until the other guest's input confirms
/// its tick, then counted once, saved, and heard once by the session.
#[test]
fn a_world_count_from_a_rewinding_room_applies_once_when_confirmed() {
    use ambition_hosted_world::bevy_replicon::prelude::{ClientId, FromClient};
    use ambition_hosted_world::{GuestInput, GuestSeat, RewindsGuestRooms, RoomRollbackStats};
    use ambition_platformer2d::characters::control::PlayerSlot;
    use ambition_platformer2d::combat::components::FeatureId;
    use ambition_platformer2d::combat::extension::status::BodyStatuses;
    use ambition_platformer2d::engine_core::ControlFrame;
    use ambition_platformer2d::runtime::world_values::{world_values, HeldWorldWrites};
    use ambition_platformer2d::time::SimTick;
    use crate::hosted_world::{alice_leaves, alice_on_the_hub_door};
    use crate::two_players_two_live_rooms::{bob_beside_alice, where_they_are, ROOM};

    let dir = std::env::temp_dir().join(format!("ambition_world_island_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("count.rhai");
    std::fs::write(&path, ISLAND_SOURCE).unwrap();
    let mut host = crate::a_world_post_crosses_rooms_once::boot(path);
    bob_beside_alice(&mut host, ROOM, Some(PlayerSlot(1)));
    let bob = {
        let world = host.world_mut();
        world.query::<(Entity, &FeatureId)>().iter(world).find(|(_, id)| id.0 == "ow1_bob").expect("Bob is in the world").0
    };
    host.world_mut().entity_mut(bob).insert(BodyStatuses(vec!["world_island_probe.go".into()]));
    for _ in 0..20 {
        host.drive_seat(0, ControlFrame::default());
        host.drive_seat(1, ControlFrame::default());
        host.app_mut().update();
    }
    let root = |host: &Platformer2dSimHarness| host.world().get::<BodyRecords>(bob)
        .and_then(|records| records.get(&SchemaKey::new("world_island_probe", "root", 1)).cloned());
    let Value::F32(line) = *root(&host).expect("Bob's script started").get(FieldRef(1)).unwrap() else { panic!("the line is a number") };

    alice_on_the_hub_door(&mut host);
    for frame in 1..=60 {
        host.drive_seat(0, alice_leaves(frame));
        host.drive_seat(1, ControlFrame::default());
        host.app_mut().update();
    }
    let (hub, lab) = where_they_are(&mut host);
    assert!(hub.is_some() && lab.flatten().is_some() && hub != lab.flatten(), "the premise: two rooms");

    host.world_mut().insert_resource(RewindsGuestRooms);
    let clients = [0, 1].map(|seat| host.world_mut().spawn(GuestSeat(PlayerSlot(seat))).id());
    let send = |host: &mut Platformer2dSimHarness, seat: usize, tick: u64, frame: ControlFrame| {
        host.world_mut().write_message(FromClient {
            client_id: ClientId::Client(clients[seat]),
            message: GuestInput { sequence: tick, tick, frame },
        });
    };
    let next = |host: &Platformer2dSimHarness| host.world().resource::<SimTick>().0 + 1;
    let bob_x = |host: &mut Platformer2dSimHarness| {
        host.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(bob).unwrap().pos.x
    };
    let held = |host: &Platformer2dSimHarness| host.world().resource::<HeldWorldWrites>().len();
    let count = |host: &mut Platformer2dSimHarness| world_values(host.world_mut()).values.into_iter()
        .find(|row| row.key == "crossings").map(|row| (row.value, row.revision));
    let right = || ControlFrame { axis_x: 1.0, ..Default::default() };
    for _ in 0..5 {
        let tick = next(&host);
        send(&mut host, 0, tick, ControlFrame::default());
        send(&mut host, 1, tick, ControlFrame::default());
        host.app_mut().update();
    }

    // Bob steps right for one tick and stops; the stop is late, so the host
    // predicts him walking on past the line, and his script counts.
    let stepped = next(&host);
    send(&mut host, 1, stepped, right());
    let mut speculative = false;
    for _ in 0..40 {
        host.app_mut().update();
        speculative |= held(&host) == 1;
        if speculative && bob_x(&mut host) > line + 8.0 { break; }
    }
    assert!(speculative, "the premise: the predicted walk crossed the line and wrote");
    assert_eq!(count(&mut host), None, "a write on a tick the host may replay is not applied");
    let rewinds = host.world().resource::<RoomRollbackStats>().rewinds;
    send(&mut host, 1, stepped + 1, ControlFrame::default());
    host.app_mut().update();
    assert!(host.world().resource::<RoomRollbackStats>().rewinds > rewinds, "the stop rewinds Bob's room");
    assert!(bob_x(&mut host) < line, "Bob never reached the line");
    assert_eq!(held(&host), 0, "the replay takes the write back");
    for _ in 0..10 {
        let tick = next(&host);
        send(&mut host, 0, tick, ControlFrame::default());
        send(&mut host, 1, tick, ControlFrame::default());
        host.app_mut().update();
    }
    assert_eq!(count(&mut host), None, "the write a replay took back never applies");

    // Bob walks past the line on time; Alice's input stops arriving, so the
    // write is held.
    let mut sent_at = None;
    for _ in 0..60 {
        let tick = next(&host);
        send(&mut host, 1, tick, right());
        host.app_mut().update();
        if held(&host) == 1 { sent_at = Some(host.world().resource::<SimTick>().0); break; }
    }
    let sent_at = sent_at.expect("Bob walked past the line and wrote");
    for _ in 0..3 {
        let tick = next(&host);
        send(&mut host, 1, tick, ControlFrame::default());
        host.app_mut().update();
    }
    assert_eq!(held(&host), 1, "held while Alice's input for the writing tick has not come");
    assert_eq!(count(&mut host), None);
    assert_eq!(root(&host).unwrap().get(FieldRef(3)).unwrap(), &Value::Text("world_value_held".into()));

    // Alice's input confirms the tick, and the next live tick applies it.
    send(&mut host, 0, sent_at + 1, ControlFrame::default());
    host.app_mut().update();
    assert_eq!(held(&host), 0);
    for _ in 0..10 {
        let tick = next(&host);
        send(&mut host, 0, tick, ControlFrame::default());
        send(&mut host, 1, tick, ControlFrame::default());
        host.app_mut().update();
    }
    assert_eq!(count(&mut host), Some((ambition_platformer2d::extension::sdk::script::ScriptValue::Int(1), 1)),
        "counted once, at revision 1");
    let tally = SchemaKey::new("world_island_probe", "tally", 1);
    let world = host.world_mut();
    let heard: Vec<_> = world.query::<&ambition_platformer2d::extension::SessionRecords>().iter(world)
        .filter_map(|records| records.get(&tally).cloned()).collect();
    assert_eq!(heard.len(), 1);
    assert_eq!(heard[0].get(FieldRef(0)).unwrap(), &Value::U32(1), "the session heard the change once");
    assert_eq!(heard[0].get(FieldRef(1)).unwrap(), &Value::U32(1));
    let faults = host.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent);
    let _ = std::fs::remove_dir_all(dir);
}
