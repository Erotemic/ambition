//! The committed world road between rollback islands (SCRIPT-COMPLETION; the
//! script-first plan's step 4): `post_world_signal`.
//!
//! On a host that rewinds guest rooms, Alice (a guest at seat 0) carries a
//! listener a script attached to her in `switch_lab`, subscribed to Bob, into
//! the hub. Bob (a guest at seat 1) stays in `switch_lab`, and his script posts
//! once he walks past a line. Each room rewinds alone, so the post must wait
//! until no late input can replay the tick that sent it, and a rewind of the
//! listener's room must not lose or repeat it.

use ambition_app::Platformer2dSimHarness;
use ambition_hosted_world::bevy_replicon::prelude::{ClientId, FromClient, RepliconPlugins, ServerPlugin};
use ambition_hosted_world::{GuestInput, GuestSeat, HostedRole, HostedWorldPlugin, RewindsGuestRooms, RoomRollbackStats};
use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::combat::extension::status::BodyStatuses;
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::runtime::world_posts::WorldPosts;
use ambition_platformer2d::time::SimTick;
use bevy::prelude::{Entity, PluginGroup as _, PostUpdate, With};

use crate::common::fixed_60hz_room_options;
use crate::hosted_world::{alice_leaves, alice_on_the_hub_door};
use crate::two_players_two_live_rooms::{bob_beside_alice, where_they_are, ROOM};

const SOURCE: &str = r#"
fn declare() {
    #{provider: "world_probe", key: "post", schemas: [
        #{key: "root", fields: [
            #{tag: 1, name: "started", kind: "bool"},
            #{tag: 2, name: "me", kind: "opt object"},
            #{tag: 3, name: "line", kind: "f32"},
            #{tag: 4, name: "asked", kind: "bool"},
            #{tag: 5, name: "armed", kind: "bool"},
            #{tag: 6, name: "posted", kind: "u32"},
            #{tag: 7, name: "code", kind: "text 32"},
        ]},
        #{key: "listener", fields: [
            #{tag: 1, name: "heard", kind: "u32"},
            #{tag: 2, name: "from", kind: "opt object"},
            #{tag: 3, name: "count", kind: "u32"},
        ]}
    ], entries: [
        #{key: "tick", phase: "technique_execution", trigger: #{port: "status_tick", selector: "world_probe.post"},
          writes: ["root"], requests: ["body_query", "post_world_signal"], limit: 1},
        #{key: "me", phase: "actor_think", trigger: #{port: "action_result", selector: "me"},
          reads: ["body_query_result"], writes: ["root"]},
        #{key: "found", phase: "actor_think", trigger: #{port: "action_result", selector: "found"},
          reads: ["body_query_result"], writes: ["root"], requests: ["attach_behaviors"], limit: 1},
        #{key: "attached", phase: "actor_think", trigger: #{port: "action_result", selector: "attached"}, writes: ["root"]},
        #{key: "sent", phase: "actor_think", trigger: #{port: "action_result", selector: "sent"}, writes: ["root"]},
        #{key: "hear", phase: "actor_think", trigger: #{port: "object_signal", selector: "door_opened"}, writes: ["listener"]},
    ]}
}
fn tick() {
    if !this.state.root.started {
        this.state.root.started = true;
        this.state.root.line = this.trigger.position[0] + 24.0;
        this.requests.push(request("body_query", #{reply: #{request: 0, selector: "world_probe::post/me"},
            origin: this.trigger.position, radius: 2.0, limit: 1, include_self: true}));
        return;
    }
    if this.state.root.me != () && !this.state.root.asked {
        this.state.root.asked = true;
        this.requests.push(request("body_query", #{reply: #{request: 1, selector: "world_probe::post/found"},
            origin: this.trigger.position, radius: 4000.0, limit: 8, include_self: false}));
        return;
    }
    if this.state.root.armed && this.state.root.posted == 0 && this.trigger.position[0] > this.state.root.line {
        this.state.root.posted += 1;
        this.requests.push(request("post_world_signal", #{topic: "door_opened", data: 7,
            reply: #{request: 2, selector: "world_probe::post/sent"}}));
    }
}
fn me() {
    if !this.trigger.terminal { return; }
    for target in this.observe.body_query_result.targets {
        if target.driven { this.state.root.me = target.object; }
    }
}
fn found() {
    if !this.trigger.terminal { return; }
    let other = ();
    for target in this.observe.body_query_result.targets {
        if target.driven { other = target.object; }
    }
    if other == () { throw "the query found no other driven body"; }
    this.requests.push(request("attach_behaviors", #{target: other, bindings: [#{key: "listener",
        references: #{sender: this.state.root.me}, methods: [#{name: "hear", entry: "world_probe::post/hear"}],
        subscriptions: [#{key: "door", source: "sender", method: "hear"}]}],
        reply: #{request: 3, selector: "world_probe::post/attached"}}));
}
fn attached() {
    if this.trigger.stage == "rejected" { throw this.trigger.message; }
    if this.trigger.terminal { this.state.root.armed = true; }
}
fn sent() {
    if this.trigger.terminal { this.state.root.code = this.trigger.code; }
}
fn hear() {
    this.state.listener.heard += 1;
    this.state.listener.from = this.trigger.from;
    this.state.listener.count = this.trigger.data;
}
"#;

pub(crate) fn boot(module: std::path::PathBuf) -> Platformer2dSimHarness {
    Platformer2dSimHarness::build(fixed_60hz_room_options(ROOM).with_extension_module_files(vec![module]), |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        app.add_plugins((RepliconPlugins.set(ServerPlugin::new(PostUpdate)), HostedWorldPlugin { role: HostedRole::Host }));
        app.finish();
        app.cleanup();
        Ok(())
    })
    .expect("the room boots")
}

fn bob(host: &mut Platformer2dSimHarness) -> Entity {
    let world = host.world_mut();
    world.query::<(Entity, &FeatureId)>().iter(world).find(|(_, id)| id.0 == "ow1_bob").expect("Bob is in the world").0
}

fn root(host: &mut Platformer2dSimHarness, bob: Entity) -> Option<ambition_platformer2d::extension::sdk::Record> {
    host.world().get::<BodyRecords>(bob)?.get(&SchemaKey::new("world_probe", "root", 1)).cloned()
}

/// Alice's listener: (heard, from, count).
fn heard(host: &mut Platformer2dSimHarness) -> (u32, Option<Value>, u32) {
    let listener = SchemaKey::new("world_probe", "listener", 1);
    let world = host.world_mut();
    let rows: Vec<_> = world.query::<&BodyRecords>().iter(world).filter_map(|records| records.get(&listener).cloned()).collect();
    assert!(rows.len() <= 1, "one listener");
    rows.first().map_or((0, None, 0), |row| {
        let Value::U32(heard) = row.get(FieldRef(0)).unwrap() else { panic!("heard is a count") };
        let Value::Opt(from) = row.get(FieldRef(1)).unwrap() else { panic!("from is optional") };
        let Value::U32(count) = row.get(FieldRef(2)).unwrap() else { panic!("count is a count") };
        (*heard, from.as_deref().cloned(), *count)
    })
}

fn posts(host: &Platformer2dSimHarness) -> (usize, usize) {
    let posts = host.world().resource::<WorldPosts>();
    (posts.held(), posts.admitted())
}

fn right() -> ControlFrame { ControlFrame { axis_x: 1.0, ..Default::default() } }

/// ⭐ A POST FROM ONE ROOM REACHES A LISTENER IN ANOTHER ONCE, AND ONLY WHEN
/// NO REPLAY CAN TAKE ITS SENDING TICK BACK. A post made on a predicted tick
/// that a late input then takes back never arrives; a post made on a tick that
/// only waits for the other guest's input is held until it comes, and arrives
/// though its sender is gone by then; and a rewind
/// of the listener's room across the admission keeps exactly one delivery.
#[test]
fn a_world_post_crosses_rooms_once_and_only_when_confirmed() {
    let dir = std::env::temp_dir().join(format!("ambition_world_post_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("world_probe.rhai");
    std::fs::write(&path, SOURCE).unwrap();
    let mut host = boot(path);
    bob_beside_alice(&mut host, ROOM, Some(PlayerSlot(1)));
    let bob = bob(&mut host);
    host.world_mut().entity_mut(bob).insert(BodyStatuses(vec!["world_probe.post".into()]));
    for _ in 0..20 {
        host.drive_seat(0, ControlFrame::default());
        host.drive_seat(1, ControlFrame::default());
        host.app_mut().update();
    }
    let record = root(&mut host, bob).expect("Bob's script started");
    assert_eq!(record.get(FieldRef(4)).unwrap(), &Value::Bool(true), "the listener is attached to Alice");
    let Value::Opt(Some(me)) = record.get(FieldRef(1)).unwrap().clone() else { panic!("Bob's script knows his object") };
    let Value::F32(line) = *record.get(FieldRef(2)).unwrap() else { panic!("the line is a number") };

    // Alice carries the listener into the hub; Bob stays.
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
    for _ in 0..5 {
        let tick = next(&host);
        send(&mut host, 0, tick, ControlFrame::default());
        send(&mut host, 1, tick, ControlFrame::default());
        host.app_mut().update();
    }

    // Bob steps right for one tick and stops. The stop is late: the host
    // predicts him walking on, past the line, and his script posts.
    let stepped = next(&host);
    send(&mut host, 1, stepped, right());
    let mut speculative = false;
    for _ in 0..40 {
        host.app_mut().update();
        speculative |= posts(&host).0 == 1;
        if speculative && bob_x(&mut host) > line + 8.0 {
            break;
        }
    }
    assert!(speculative, "the premise: the predicted walk crossed the line and posted");
    assert_eq!(posts(&host).1, 0, "a post on a tick the host may replay is not admitted");
    let rewinds = host.world().resource::<RoomRollbackStats>().rewinds;
    send(&mut host, 1, stepped + 1, ControlFrame::default());
    host.app_mut().update();
    assert!(host.world().resource::<RoomRollbackStats>().rewinds > rewinds, "the stop rewinds Bob's room");
    assert!(bob_x(&mut host) < line, "Bob never reached the line");
    assert_eq!(posts(&host), (0, 0), "the replay takes the post back");
    for _ in 0..10 {
        let tick = next(&host);
        send(&mut host, 0, tick, ControlFrame::default());
        send(&mut host, 1, tick, ControlFrame::default());
        host.app_mut().update();
    }
    assert_eq!(heard(&mut host).0, 0, "the post a replay took back never arrives");
    assert_eq!(root(&mut host, bob).unwrap().get(FieldRef(5)).unwrap(), &Value::U32(0));

    // Bob walks past the line, on time. Alice's input stops arriving, so the
    // host cannot yet know her room's past, and the post is held.
    let mut sent_at = None;
    for _ in 0..60 {
        let tick = next(&host);
        send(&mut host, 1, tick, right());
        host.app_mut().update();
        if posts(&host).0 == 1 {
            sent_at = Some(host.world().resource::<SimTick>().0);
            break;
        }
    }
    let sent_at = sent_at.expect("Bob walked past the line and posted");
    for _ in 0..3 {
        let tick = next(&host);
        send(&mut host, 1, tick, ControlFrame::default());
        host.app_mut().update();
    }
    assert_eq!(posts(&host), (1, 0), "held while Alice's input for the sending tick has not come");
    assert_eq!(heard(&mut host).0, 0);
    assert_eq!(root(&mut host, bob).unwrap().get(FieldRef(6)).unwrap(), &Value::Text("world_signal_held".into()));
    // The sender leaves play while its post is held: the post carries what
    // it needs, so it still arrives, named.
    host.world_mut().despawn(bob);

    // Alice's input arrives through the sending tick: it confirms, and the
    // next live tick admits the post.
    send(&mut host, 0, sent_at + 1, ControlFrame::default());
    host.app_mut().update();
    let admitted_at = host.world().resource::<SimTick>().0;
    assert_eq!(posts(&host), (0, 1), "confirmed and admitted");
    // A late input of Alice's before the admission rewinds her room across it.
    let rewinds = host.world().resource::<RoomRollbackStats>().rewinds;
    send(&mut host, 0, sent_at + 2, right());
    host.app_mut().update();
    let stats = *host.world().resource::<RoomRollbackStats>();
    assert!(stats.rewinds > rewinds, "Alice's late input rewinds the hub");
    assert!(stats.last_rewound_from.is_some_and(|from| from <= admitted_at), "the rewind spans the admission tick");
    for _ in 0..10 {
        let tick = next(&host);
        send(&mut host, 0, tick, ControlFrame::default());
        send(&mut host, 1, tick, ControlFrame::default());
        host.app_mut().update();
    }
    assert_eq!(heard(&mut host), (1, Some(Value::Object(me.as_object().unwrap())), 7),
        "heard once after the rewind, named by the runtime, with its data");
    let world = host.world_mut();
    let alice_room = world.query_filtered::<&ambition_platformer2d::platformer::lifecycle::InRoomInstance, With<PrimaryPlayer>>()
        .single(world).unwrap().0;
    assert_eq!(Some(alice_room), hub, "the listener heard it in the hub");
    let faults = host.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent);
    let _ = std::fs::remove_dir_all(dir);
}
