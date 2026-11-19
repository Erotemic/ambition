//! Loaded factories create native geometry for room systems and body behaviors.

#![cfg(feature = "rl_sim")]
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    breakables::BreakableFeature,
    characters::actor::{ActorFaction, BodyWallet, CharacterForm},
    combat::{
        components::{CenteredAabb, FeatureId},
        extension::status::BodyStatuses,
    },
    engine_core as ae,
    extension::{
        self,
        sdk::{FieldRef, SchemaKey, Value},
        BodyRecords, ExtensionFaults,
    },
    platformer::{
        construction::SpawnOrigin,
        lifecycle::{InRoomInstance, RoomInstanceRoot},
        markers::PrimaryPlayerOnly,
        sim_id::{SimId, SimIdCounter},
    },
    sim_view::{DynamicFeatureViews, FeatureSheet, FeatureViewIndex},
};
use bevy::{ecs::system::SystemState, prelude::*};

const LIBRARY: &str = r#"
const WIDTH = 96.0;
fn block(key, at, mode) {
    #{key: key, at: at, size: [global::WIDTH, 16.0], health: 2,
      collision: mode, look: "cracked_stone"}
}
fn memory(key) {
    #{key: key, fields: [
        #{tag: 1, name: "sent", kind: "bool"},
        #{tag: 2, name: "created", kind: "u32"},
        #{tag: 3, name: "refused", kind: "u32"},
        #{tag: 4, name: "objects", kind: #{seq: 16, of: "text 128"}},
        #{tag: 5, name: "consumed", kind: "bool"},
        #{tag: 6, name: "actor", kind: "opt body"},
    ]}
}
fn created(ctx) {
    if !ctx.trigger.terminal { return ctx; }
    if ctx.trigger.stage == "applied" {
        if ctx.trigger.ticket.domain == "world.create_actors" {
            ctx.state.memory.actor = ctx.trigger.subjects[0];
            return ctx;
        }
        if ctx.trigger.code != "breakables_created" { throw "false prop creation"; }
        ctx.state.memory.created += 1;
        ctx.state.memory.objects = ctx.trigger.subjects;
    } else if ctx.trigger.stage == "rejected" {
        if ctx.trigger.code != "breakable_creation_refused" { throw ctx.trigger.code; }
        ctx.state.memory.refused += 1;
    }
    ctx
}
"#;
const ROOM: &str = r#"
import "lib/blocks.rhai" as blocks;
fn declare() {
    #{provider: "prop_room_probe", key: "bridge", schemas: [blocks::memory("memory")], entries: [
        #{key: "tick", phase: "actor_think", trigger: #{port: "story_tick", selector: "intro_wake_room"},
          writes: ["memory"], requests: ["create_breakables", "create_actors"], limit: 4},
        #{key: "result", phase: "actor_think", trigger: #{port: "action_result", selector: "result"}, writes: ["memory"]},
    ]}
}
fn tick() {
    if !this.trigger.flags.contains("prop_probe_begin") || this.state.memory.sent { return; }
    this.state.memory.sent = true;
    let p = this.trigger.bodies[0].position;
    for body in this.trigger.bodies { if body.driven { p = body.position; } }
    let a = blocks::block("bridge", [p[0] + 180.0, p[1] - 100.0], "one_way");
    let b = blocks::block("wall", [p[0] + 380.0, p[1] - 100.0], "solid");
    let batch = #{reply: #{request: 1, selector: "prop_room_probe::bridge/result"}, objects: [a, b]};
    this.requests.push(request("create_breakables", batch));
    this.requests.push(request("create_breakables", batch));
    b.health = 0;
    this.requests.push(request("create_breakables", #{
        reply: #{request: 2, selector: "prop_room_probe::bridge/result"}, objects: [a, b]}));
    this.requests.push(request("create_actors", #{
        reply: #{request: 1, selector: "prop_room_probe::bridge/result"},
        actors: [#{key: "room_actor", character: "practice_sandbag", at: [p[0] + 600.0, p[1] - 200.0]}]}));
}
fn result() { this = blocks::created(this); }
"#;
const BODY: &str = r#"
import "lib/blocks.rhai" as blocks;
fn declare() {
    #{provider: "prop_body_probe", key: "bonk", schemas: [blocks::memory("memory")], entries: [
        #{key: "sense", phase: "body_contact", trigger: #{port: "body_contact", selector: "prop_body_probe.active"},
          writes: ["memory"], requests: ["create_breakables", "consume_contact"], limit: 1},
        #{key: "result", phase: "actor_think", trigger: #{port: "action_result", selector: "result"}, writes: ["memory"]},
    ]}
}
fn sense() {
    let state = this.state.memory;
    if !state.sent {
        this.state.memory.sent = true;
        let p = this.trigger.body.position;
        let ceiling = blocks::block("cache", [p[0], p[1] - this.trigger.body.size[1] * 0.5 - 18.0], "solid");
        ceiling.respawn_after = 0.25;
        this.requests.push(request("create_breakables", #{
            reply: #{request: 1, selector: "prop_body_probe::bonk/result"}, objects: [ceiling]}));
        return;
    }
    if state.objects.len() == 0 || state.consumed { return; }
    for index in 0..this.trigger.world.len() {
        let contact = this.trigger.world[index];
        if contact.role != "head" || contact.block == () { continue; }
        if contact.block.geometry.key != state.objects[0] { continue; }
        this.requests.push(request("consume_contact", #{
            reply: #{request: 1, selector: "prop_body_probe::bonk/result"}, contact: index,
            exchange: #{receive_coins: 3}}));
        return;
    }
}
fn result() {
    if this.trigger.ticket.domain == "world.consume_contact" {
        if !this.trigger.terminal { return; }
        if this.trigger.stage != "applied" || this.trigger.code != "contact_consumed" { throw this.trigger.code; }
        this.state.memory.consumed = true;
    } else { this = blocks::created(this); }
}
"#;

fn files(name: &str, root: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "ambition_prop_creation_{name}_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(dir.join("lib/blocks.rhai"), LIBRARY).unwrap();
    let file = dir.join("factory.rhai");
    std::fs::write(&file, root).unwrap();
    (dir, file)
}
fn options(file: std::path::PathBuf) -> ambition_app::rl_sim::Platformer2dSimHarnessOptions {
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    crate::common::fixed_60hz_room_options("intro_wake_room")
        .with_save(save)
        .with_extension_module_files(vec![file])
}
fn memory<'a>(
    world: &'a World,
    entity: Entity,
    provider: &'static str,
) -> &'a extension::sdk::Record {
    world
        .get::<BodyRecords>(entity)
        .unwrap()
        .get(&SchemaKey::new(provider, "memory", 1))
        .unwrap()
}
fn objects(world: &World, entity: Entity, provider: &'static str) -> Vec<String> {
    let Value::Seq(values) = memory(world, entity, provider).get(FieldRef(3)).unwrap() else {
        panic!("object refs")
    };
    values
        .iter()
        .map(|value| value.as_text().unwrap().to_owned())
        .collect()
}
fn object(world: &mut World, id: &str) -> Entity {
    world
        .query::<(Entity, &SimId, &BreakableFeature)>()
        .iter(world)
        .find(|(_, actual, _)| actual.as_str() == id)
        .unwrap()
        .0
}
fn collider_present(sim: &mut Platformer2dSimHarness, entity: Entity) -> bool {
    let id = sim
        .world()
        .get::<FeatureId>(entity)
        .unwrap()
        .as_str()
        .to_owned();
    let room = *sim.world().get::<InRoomInstance>(entity).unwrap();
    let mut query = SystemState::<ambition_platformer2d::world::collision::CollisionWorld>::new(
        sim.world_mut(),
    );
    query
        .get(sim.world())
        .unwrap()
        .room(Some(&room))
        .unwrap()
        .solids()
        .unwrap()
        .blocks
        .iter()
        .any(|block| block.id == ae::GeoId::placement(ae::PlacementId::new(&id), 0))
}

#[test]
fn room_factories_build_props_and_actors_with_one_rewound_source_counter() {
    let (dir, file) = files("room", ROOM);
    let mut sim = Platformer2dSimHarness::new_with_options(
        options(file).with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    sim.world_mut().init_resource::<extension::ExtensionProfile>();
    sim.step_n(AgentAction::default(), 4);
    let root = {
        let world = sim.world_mut();
        world
            .query_filtered::<Entity, With<RoomInstanceRoot>>()
            .single(world)
            .unwrap()
    };
    let start = sim.world().get::<SimIdCounter>(root).unwrap().0;
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::session::AmbitionGameSave>()
        .data_mut()
        .set_flag("prop_probe_begin", true);
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 24);
    let record = memory(sim.world(), root, "prop_room_probe");
    assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1));
    assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U32(1));
    let ids = objects(sim.world(), root, "prop_room_probe");
    assert_eq!(ids.len(), 2);
    assert_eq!(sim.world().get::<SimIdCounter>(root).unwrap().0, start + 3);
    let Value::Opt(Some(actor)) = record.get(FieldRef(5)).unwrap() else {
        panic!("room actor")
    };
    let actor_id = actor.as_ref_id().unwrap().to_owned();
    assert_ne!(actor_id, ids[0]);
    assert_ne!(actor_id, ids[1]);
    for (index, id) in ids.iter().enumerate() {
        let entity = object(sim.world_mut(), id);
        let feature = sim.world().get::<BreakableFeature>(entity).unwrap();
        assert_eq!(feature.state().health.current, 2);
        assert_eq!(feature.look(), Some("cracked_stone"));
        assert!(
            matches!(sim.world().get::<SpawnOrigin>(entity), Some(SpawnOrigin::Dynamic {sequence, ..}) if *sequence == start + index as u64 + 1)
        );
        assert!(collider_present(&mut sim, entity));
        let fact = sim
            .world()
            .resource::<DynamicFeatureViews>()
            .0
            .iter()
            .find(|row| row.id == *id)
            .unwrap();
        assert!(
            matches!(&fact.sheet, Some(FeatureSheet::Structure(look)) if look == "cracked_stone")
        );
    }
    // The bridge is a real native support surface for the controlled body.
    let bridge = object(sim.world_mut(), &ids[0]);
    let aabb = *sim.world().get::<CenteredAabb>(bridge).unwrap();
    let world = sim.world_mut();
    let player = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    let half = world
        .get::<ae::BodyKinematics>(player)
        .unwrap()
        .half_extent();
    sim.teleport_player((
        aabb.center.x,
        aabb.center.y - aabb.half_size.y - half.y - 1.0,
    ));
    sim.step_n(AgentAction::default(), 12);
    assert!(
        sim.world()
            .get::<ae::BodyGroundState>(player)
            .unwrap()
            .on_ground
    );
    assert_eq!(
        sim.world().resource::<ExtensionFaults>().total,
        0,
        "{:?}",
        sim.world().resource::<ExtensionFaults>().recent
    );
    sim.rollback_health().unwrap();
    for (path, cost) in &sim.world().resource::<extension::ExtensionProfile>().0 {
        if path.starts_with("prop_") && cost.calls > 0 {
            println!(
                "BREAKABLE_CREATION_COST {path} calls={} backend_us={} host_us={}",
                cost.calls,
                cost.backend.as_micros() as f64 / cost.calls as f64,
                cost.total.as_micros() as f64 / cost.calls as f64
            );
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn body_factories_create_contact_rewards_with_native_respawn_and_replay() {
    let (dir, file) = files("body", BODY);
    let mut sim = Platformer2dSimHarness::new_with_options(
        options(file).with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    sim.world_mut().init_resource::<extension::ExtensionProfile>();
    sim.step_n(AgentAction::default(), 30);
    let world = sim.world_mut();
    let player = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    let teacher_position = world
        .query::<(&CharacterForm, &ae::BodyKinematics)>()
        .iter(world)
        .find(|(form, _)| form.id() == "npc_creator")
        .unwrap()
        .1
        .pos;
    let npc = extension::create_actors(
        world,
        player,
        &[extension::RuntimeActor {
            initialization: (),
            key: "builder".into(),
            character: "practice_sandbag".into(),
            aabb: ae::Aabb::new(
                teacher_position - ae::Vec2::Y * 100.0,
                ae::Vec2::new(16.0, 24.0),
            ),
            faction: ActorFaction::Neutral,
            face_left: false,
            starting_wallet: Some(0),
            holds: Vec::new(),
        }],
        &[],
        Default::default(),
    )
    .unwrap()
    .remove(0)
    .entity;
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 60);
    for body in [player, npc] {
        sim.world_mut().get_mut::<BodyWallet>(body).unwrap().balance = 0;
        sim.world_mut()
            .entity_mut(body)
            .insert(BodyStatuses(vec!["prop_body_probe.active".into()]));
    }
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 4);
    let mut props = Vec::new();
    for body in [player, npc] {
        let ids = objects(sim.world(), body, "prop_body_probe");
        assert_eq!(ids.len(), 1);
        props.push(object(sim.world_mut(), &ids[0]));
        sim.world_mut()
            .get_mut::<ae::BodyKinematics>(body)
            .unwrap()
            .vel
            .y = -400.0;
    }
    sim.rebase_rollback_history().unwrap();
    let mut saw_broken = false;
    for _ in 0..12 {
        sim.step(AgentAction::default());
        if props.iter().all(|object| {
            sim.world()
                .get::<BreakableFeature>(*object)
                .unwrap()
                .broken()
        }) && [player, npc].iter().all(|body| {
            memory(sim.world(), *body, "prop_body_probe")
                .get(FieldRef(4))
                .unwrap()
                == &Value::Bool(true)
        }) {
            saw_broken = true;
            for object in &props {
                assert!(!collider_present(&mut sim, *object));
                let id = sim.world().get::<FeatureId>(*object).unwrap().as_str();
                let schedule = ambition_platformer2d::platformer::lifecycle::session_world_component::<
                    ambition_platformer2d::world_returns::WorldTimeSchedule>(sim.world()).unwrap();
                assert!(!schedule.holds("intro_wake_room", id), "a room prop has no durable reconstruction recipe");
            }
        }
    }
    assert!(saw_broken);
    for body in [player, npc] {
        assert_eq!(
            memory(sim.world(), body, "prop_body_probe")
                .get(FieldRef(4))
                .unwrap(),
            &Value::Bool(true)
        );
        assert_eq!(sim.world().get::<BodyWallet>(body).unwrap().balance, 3);
    }
    sim.step_n(AgentAction::default(), 24);
    for object in props {
        assert!(!sim
            .world()
            .get::<BreakableFeature>(object)
            .unwrap()
            .broken());
        assert!(collider_present(&mut sim, object));
        let id = sim.world().get::<FeatureId>(object).unwrap().as_str();
        assert!(
            sim.world()
                .resource::<FeatureViewIndex>()
                .get(id)
                .unwrap()
                .visible
        );
    }
    assert_eq!(
        sim.world().resource::<ExtensionFaults>().total,
        0,
        "{:?}",
        sim.world().resource::<ExtensionFaults>().recent
    );
    sim.rollback_health().unwrap();
    for (path, cost) in &sim.world().resource::<extension::ExtensionProfile>().0 {
        if path.starts_with("prop_") && cost.calls > 0 {
            println!(
                "BREAKABLE_CREATION_COST {path} calls={} backend_us={} host_us={}",
                cost.calls,
                cost.backend.as_micros() as f64 / cost.calls as f64,
                cost.total.as_micros() as f64 / cost.calls as f64
            );
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editing_the_factory_changes_new_geometry_without_rebuilding_existing_props() {
    let script = ROOM.replace("intro_wake_room", "switch_lab");
    let (dir, file) = files("edit", &script);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("switch_lab").with_extension_module_files(vec![file.clone()])).unwrap();
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::session::AmbitionGameSave>()
        .data_mut()
        .set_flag("prop_probe_begin", true);
    sim.step_n(AgentAction::default(), 24);
    let root = {
        let world = sim.world_mut();
        world
            .query_filtered::<Entity, With<RoomInstanceRoot>>()
            .single(world)
            .unwrap()
    };
    let before = objects(sim.world(), root, "prop_room_probe");
    assert_eq!(before.len(), 2);
    let generation = sim
        .world()
        .resource::<extension::ExtensionGeneration>()
        .clone();
    std::fs::write(
        dir.join("lib/blocks.rhai"),
        LIBRARY.replace("WIDTH = 96.0", "WIDTH = 160.0"),
    )
    .unwrap();
    std::fs::write(
        &file,
        script.replace("|| this.state.memory.sent", "|| !this.state.memory.sent")
            .replace(
                "this.state.memory.sent = true",
                "this.state.memory.sent = false",
            )
            .replace("request: 1,", "request: 3,")
            .replace("request: 2,", "request: 4,")
            .replace("p[0] + 180.0", "p[0] + 700.0")
            .replace("p[0] + 380.0", "p[0] + 900.0"),
    )
    .unwrap();
    sim.step_n(AgentAction::default(), 50);
    assert_ne!(
        *sim.world().resource::<extension::ExtensionGeneration>(),
        generation
    );
    let after = objects(sim.world(), root, "prop_room_probe");
    assert_eq!(after.len(), 2);
    assert!(after.iter().all(|id| !before.contains(id)));
    let all_ids: Vec<_> = before.iter().chain(&after).cloned().collect();
    for (ids, width) in [(before, 96.0), (after, 160.0)] {
        for id in ids {
            let entity = object(sim.world_mut(), &id);
            assert_eq!(
                sim.world().get::<CenteredAabb>(entity).unwrap().size().x,
                width
            );
            assert!(collider_present(&mut sim, entity));
        }
    }
    assert_eq!(
        memory(sim.world(), root, "prop_room_probe")
            .get(FieldRef(1))
            .unwrap(),
        &Value::U32(2)
    );
    assert_eq!(
        memory(sim.world(), root, "prop_room_probe")
            .get(FieldRef(2))
            .unwrap(),
        &Value::U32(2)
    );
    let old_room = *sim.world().get::<ambition_platformer2d::platformer::lifecycle::LiveRoomInstance>(root).unwrap();
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "central_hub_complex"), "central_hub_complex");
    sim.step_n(AgentAction::default(), 8);
    assert_ne!(sim.world().get::<ambition_platformer2d::platformer::lifecycle::LiveRoomInstance>(root), Some(&old_room));
    let world = sim.world_mut();
    assert!(!world
        .query::<&SimId>()
        .iter(world)
        .any(|id| all_ids.iter().any(|created| created == id.as_str())));
    assert!(!world
        .resource::<DynamicFeatureViews>()
        .0
        .iter()
        .any(|row| all_ids.contains(&row.id)));
    assert_eq!(
        sim.world().resource::<ExtensionFaults>().total,
        0,
        "{:?}",
        sim.world().resource::<ExtensionFaults>().recent
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn authored_batch_and_population_limits_refuse_without_partial_props() {
    use ambition_platformer2d::{
        breakables::creation::BreakableCreationLimits,
        world::scoped_rules::{DeclaredRules, RulesScope},
    };
    for population_limit in [false, true] {
        let (dir, file) = files(
            if population_limit {
                "population"
            } else {
                "batch"
            },
            ROOM,
        );
        let mut sim = Platformer2dSimHarness::new_with_options(options(file)).unwrap();
        sim.step_n(AgentAction::default(), 4);
        let world = sim.world_mut();
        let root = world
            .query_filtered::<Entity, With<RoomInstanceRoot>>()
            .single(world)
            .unwrap();
        let before = world.query::<&BreakableFeature>().iter(world).count();
        let counter = world.get::<SimIdCounter>(root).unwrap().0;
        let mut limits = BreakableCreationLimits::default();
        if population_limit {
            limits.max_room_breakables = before as u32 + 1;
        } else {
            limits.max_breakables = 1;
        }
        sim.world_mut().resource_mut::<DeclaredRules<BreakableCreationLimits>>()
            .restate(RulesScope::UntaggedRooms, Some(limits));
        sim.world_mut()
            .resource_mut::<ambition_platformer2d::session::AmbitionGameSave>()
            .data_mut()
            .set_flag("prop_probe_begin", true);
        sim.step_n(AgentAction::default(), 24);
        assert_eq!(
            memory(sim.world(), root, "prop_room_probe")
                .get(FieldRef(1))
                .unwrap(),
            &Value::U32(0)
        );
        assert_eq!(
            memory(sim.world(), root, "prop_room_probe")
                .get(FieldRef(2))
                .unwrap(),
            &Value::U32(2)
        );
        assert!(objects(sim.world(), root, "prop_room_probe").is_empty());
        assert_eq!(
            sim.world().get::<SimIdCounter>(root).unwrap().0,
            counter + 1,
            "only the independent actor request spends an identity"
        );
        let world = sim.world_mut();
        assert_eq!(
            world.query::<&BreakableFeature>().iter(world).count(),
            before
        );
        assert_eq!(
            world.resource::<ExtensionFaults>().total,
            0,
            "{:?}",
            world.resource::<ExtensionFaults>().recent
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}


#[test]
fn independent_room_creators_can_use_their_own_identity_ranges() {
    use ambition_platformer2d::{breakables::creation::{create_breakables, RuntimeBreakable},
        platformer::lifecycle::{spawn_live_room, LiveRoomInstance}, world::rooms::LiveRoomDefinition};
    let (dir, file) = files("rooms", ROOM);
    let mut sim = Platformer2dSimHarness::new_with_options(options(file)).unwrap();
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let first = world.query_filtered::<Entity, With<RoomInstanceRoot>>().single(world).unwrap();
    let geometry = world.get::<ae::RoomGeometry>(first).unwrap().clone();
    let definition = *world.get::<LiveRoomDefinition>(first).unwrap();
    let second = spawn_live_room(world, LiveRoomInstance::from_ordinal(27), geometry);
    world.entity_mut(second).insert(definition);
    let mut spec = ambition_platformer2d::world::rooms::BreakableSpec::new(2);
    spec.collision = ambition_platformer2d::world::rooms::BreakableCollisionSpec::Solid;
    let objects = [RuntimeBreakable {key: "shared".into(), label: "shared".into(),
        aabb: ae::Aabb::new(ae::Vec2::new(600.0, 100.0), ae::Vec2::new(20.0, 8.0)), spec}];
    let a = create_breakables(world, first, &objects, Default::default()).unwrap().remove(0);
    let b = create_breakables(world, second, &objects, Default::default()).unwrap().remove(0);
    assert_ne!(a.id, b.id, "room factories have separate creation namespaces");
    assert_ne!(a.entity, b.entity);
    assert_ne!(world.get::<InRoomInstance>(a.entity), world.get::<InRoomInstance>(b.entity));
    sim.step_n(AgentAction::default(), 2);
    for entity in [a.entity, b.entity] {
        assert!(collider_present(&mut sim, entity));
        let id = sim.world().get::<FeatureId>(entity).unwrap().as_str();
        let room = sim.world().get::<InRoomInstance>(entity).unwrap().0;
        assert!(sim.world().resource::<FeatureViewIndex>().get_in(id, Some(room)).is_some());
    }
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    let _ = std::fs::remove_dir_all(dir);
}
