//! Loaded scripts receive shared physical contact facts for any controller.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    characters::actor::CharacterForm,
    combat::{
        extension::{contact::ContactFactsLimits, status::BodyStatuses},
        targeting::MatchTeam,
    },
    engine_core::BodyKinematics,
    extension::{
        BodyRecords, ExtensionFaults,
        sdk::{FieldRef, SchemaKey, Value},
    },
    platformer::{markers::PrimaryPlayerOnly, sim_id::SimId},
    world::scoped_rules::{DeclaredRules, RulesScope},
};
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("contact_probe", "contact", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "contact_probe", key: "contacts",
       schemas: [#{ key: "contact", fields: [
           #{ tag: 1, name: "samples", kind: "u32" },
           #{ tag: 2, name: "contacts", kind: "u32" },
           #{ tag: 3, name: "empty", kind: "u32" },
           #{ tag: 4, name: "target", kind: "opt body" },
       ] }],
       entries: [
           #{ key: "activate", phase: "technique_execution",
              trigger: #{ port: "status_tick", selector: "contact_probe.seed" },
              requests: ["hold_status"], limit: 1 },
           #{ key: "observe", phase: "body_contact",
           trigger: #{ port: "body_contact", selector: "contact_probe.observe" },
           writes: ["contact"], requests: ["contact_bounce"], limit: 1 }],
    }
}
fn activate() {
    this.requests.push(request("hold_status", #{ kind: "contact_probe.observe", held: true }));
}
fn observe() {
    let sample = this.trigger;
    if sample.incomplete { throw "the fixture has refused geometry"; }
    this.state.contact.samples += 1;
    let met = false;
    for index in 0..sample.touchers.len() {
        let toucher = sample.touchers[index];
        if toucher.relation != "foe" { continue; }
        met = true;
        if toucher.id == sample.body_id { throw "a body touches itself"; }
        if toucher.surfaces.len() != 1 { throw "the surface budget was ignored"; }
        if !toucher.surfaces_truncated { throw "the contact patch was not reported"; }
        this.state.contact.target = toucher.id;
        if this.state.contact.contacts == 0 {
            this.requests.push(request("contact_bounce", #{ toucher: index, speed: 250.0 }));
        }
        this.state.contact.contacts += 1;
    }
    if !met { this.state.contact.empty += 1; }
}
"#;

#[test]
fn scripts_read_bounded_native_contacts_and_bounce_either_controller_through_replay() {
    let dir = std::env::temp_dir().join(format!("ambition_body_contacts_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("contacts.rhai");
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
    let world = sim.world_mut();
    let player = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    let teacher = world
        .query::<(Entity, &CharacterForm)>()
        .iter(world)
        .find(|(_, form)| form.id() == "npc_creator")
        .unwrap()
        .0;
    let teacher_body = *world.get::<BodyKinematics>(teacher).unwrap();
    let player_body = *world.get::<BodyKinematics>(player).unwrap();
    for (body, team) in [(player, "red"), (teacher, "blue")] {
        world.entity_mut(body).insert((
            MatchTeam::new(team),
            BodyStatuses(vec!["contact_probe.seed".into()]),
        ));
    }
    world
        .get_resource_or_insert_with(DeclaredRules::<ContactFactsLimits>::default)
        .declare(
            RulesScope::EveryRoom,
            ContactFactsLimits {
                max_touchers: 1,
                max_surfaces: 1,
                ..Default::default()
            },
        );
    let x = teacher_body.pos.x + teacher_body.half_extent().x + player_body.half_extent().x - 5.0;
    sim.teleport_player((x, teacher_body.pos.y));
    sim.step_n(AgentAction::default(), 4);
    for (body, target) in [(player, teacher), (teacher, player)] {
        let world = sim.world();
        let record = world
            .get::<BodyRecords>(body)
            .unwrap()
            .get(&RECORD)
            .unwrap();
        assert!(matches!(record.get(FieldRef(1)).unwrap(), Value::U32(count) if *count > 0));
        assert_eq!(
            record.get(FieldRef(3)).unwrap(),
            &Value::Opt(Some(Box::new(Value::Ref(
                world.get::<SimId>(target).unwrap().as_str().into()
            ))))
        );
        assert!(world.get::<BodyKinematics>(body).unwrap().vel.y < 0.0);
    }
    sim.teleport_player((x + 300.0, teacher_body.pos.y));
    sim.step_n(AgentAction::default(), 8);
    for body in [player, teacher] {
        let record = sim
            .world()
            .get::<BodyRecords>(body)
            .unwrap()
            .get(&RECORD)
            .unwrap();
        assert!(matches!(record.get(FieldRef(2)).unwrap(), Value::U32(count) if *count > 0));
    }
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}

const WORLD_RECORD: SchemaKey = SchemaKey::new("world_contact_probe", "contact", 1);
const WORLD_SOURCE: &str = r#"
fn declare() {
    #{ provider: "world_contact_probe", key: "reactive_geometry", schemas: [
        #{key: "contact", fields: [
            #{tag: 1, name: "support", kind: "u32"},
            #{tag: 2, name: "head", kind: "u32"},
            #{tag: 3, name: "source", kind: "text 128"},
            #{tag: 4, name: "requested", kind: "bool"},
            #{tag: 5, name: "created", kind: "u32"},
            #{tag: 6, name: "child", kind: "opt body"},
        ]}
    ], entries: [
        #{key: "sense", phase: "body_contact",
          trigger: #{port: "body_contact", selector: "world_contact_probe.observe"},
          writes: ["contact"], requests: ["create_actors"], limit: 1},
        #{key: "created", phase: "actor_think",
          trigger: #{port: "action_result", selector: "created"}, writes: ["contact"]}
    ]}
}
fn sense() {
    let sample = this.trigger;
    if !sample.world_available { throw "no current movement output"; }
    if sample.world_refusal != () { throw sample.world_refusal; }
    if sample.world_truncated { throw "world contact budget exhausted"; }
    for hit in sample.world {
        if hit.role == "support" { this.state.contact.support += 1; }
        if hit.role != "head" || hit.block == () { continue; }
        if hit.block.geometry.origin != "placement" { continue; }
        this.state.contact.head += 1;
        this.state.contact.source = hit.block.geometry.key;
        if this.state.contact.requested { continue; }
        this.state.contact.requested = true;
        let p = sample.body.position;
        this.requests.push(request("create_actors", #{
            reply: #{request: 1, selector: "world_contact_probe::reactive_geometry/created"},
            actors: [#{key: "bonk_reward", character: "practice_sandbag",
                       at: [p[0] + 200.0, p[1] - 100.0]}]
        }));
    }
}
fn created() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage != "applied" { throw this.trigger.message; }
    this.state.contact.created += this.trigger.subjects.len();
    this.state.contact.child = this.trigger.subjects[0];
}
"#;

#[test]
fn native_world_contacts_start_scripted_creation_for_both_controllers_and_replay() {
    use ambition_platformer2d::{
        breakables::{BreakableFeature, StandTimer},
        combat::components::{CenteredAabb, FeatureId, FeatureName},
        engine_core as ae,
        interaction::{Breakable, BreakableCollision},
        platformer::lifecycle::{FeatureSimEntity, InRoomInstance, SessionScopedEntity},
    };
    let dir = std::env::temp_dir().join(format!("ambition_world_contacts_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("contacts.rhai");
    std::fs::write(&file, WORLD_SOURCE).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room")
            .with_save(save).with_extension_module_files(vec![file])
            .with_sync_test_rollback_settings(4, 12),
    ).unwrap();
    sim.step_n(AgentAction::default(), 30);
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    let teacher = world.query::<(Entity, &CharacterForm)>().iter(world)
        .find(|(_, form)| form.id() == "npc_creator").unwrap().0;
    for (index, body) in [player, teacher].into_iter().enumerate() {
        let kinematics = *world.get::<BodyKinematics>(body).unwrap();
        let room = *world.get::<InRoomInstance>(body).unwrap();
        let session = *world.get::<SessionScopedEntity>(body).unwrap();
        let id = format!("contact-probe-ceiling-{index}");
        let mut prop = Breakable::new(&id, 1);
        prop.collision = BreakableCollision::Solid;
        world.spawn((FeatureSimEntity, FeatureId::new(&id), FeatureName::new("Contact probe ceiling"),
            CenteredAabb::from_center_size(
                kinematics.pos - ae::Vec2::Y * (kinematics.half_extent().y + 18.0),
                ae::Vec2::new(100.0, 16.0)),
            BreakableFeature::new(prop), StandTimer(0.0), SimId::placement(&id), room, session));
        world.entity_mut(body).insert(BodyStatuses(vec!["world_contact_probe.observe".into()]));
    }
    world.init_resource::<ambition_platformer2d::extension::ExtensionProfile>();
    // The autonomous body enters its ceiling through the same movement solver.
    world.get_mut::<BodyKinematics>(teacher).unwrap().vel.y = -400.0;
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction { jump: true, jump_held: true, ..Default::default() }, 1);
    sim.step_n(AgentAction { jump_held: true, ..Default::default() }, 20);
    sim.step_n(AgentAction::default(), 30);
    for (index, body) in [player, teacher].into_iter().enumerate() {
        let record = sim.world().get::<BodyRecords>(body).unwrap().get(&WORLD_RECORD).unwrap();
        assert!(matches!(record.get(FieldRef(0)).unwrap(), Value::U32(n) if *n > 0));
        assert!(matches!(record.get(FieldRef(1)).unwrap(), Value::U32(n) if *n > 0));
        assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::Text(format!("contact-probe-ceiling-{index}")));
        assert_eq!(record.get(FieldRef(4)).unwrap(), &Value::U32(1));
        let Value::Opt(Some(child)) = record.get(FieldRef(5)).unwrap() else { panic!("actual child identity") };
        let id = child.as_ref_id().unwrap();
        let mut query = sim.world().try_query::<(&SimId, &CharacterForm, &InRoomInstance)>().unwrap();
        let children: Vec<_> = query.iter(sim.world()).filter(|(actual, _, _)| actual.as_str() == id).collect();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].1.id(), "practice_sandbag");
        assert_eq!(Some(children[0].2), sim.world().get::<InRoomInstance>(body));
    }
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    if let Some(profile) = sim.world().get_resource::<ambition_platformer2d::extension::ExtensionProfile>() {
        for (key, row) in &profile.0 {
            eprintln!("{key}: {} calls; backend {:.1} us/call; host {:.1} us/call", row.calls,
                row.backend.as_secs_f64() * 1e6 / row.calls.max(1) as f64,
                row.total.as_secs_f64() * 1e6 / row.calls.max(1) as f64);
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// Measure native collection separately from the complete headless step.
#[test]
#[ignore = "Local contact cost sample; run with --ignored --nocapture"]
fn crowded_contact_observer_cost() {
    use ambition_platformer2d::{
        characters::actor::{BodyHealth, Health},
        combat::extension::contact::queue_body_contacts,
        engine_core::{self as ae, RoomGeometry},
        extension::{ExtensionSet, sdk::phases::BODY_CONTACT},
        platformer::{lifecycle::*, schedule::SimScheduleExt},
        world::rooms::{LiveRoomDefinition, RoomSet},
    };
    use std::time::Instant;
    #[derive(Resource, Default)]
    struct CollectionTimes { started: Option<Instant>, samples: Vec<f64> }
    fn begin(mut times: ResMut<CollectionTimes>) { times.started = Some(Instant::now()); }
    fn end(mut times: ResMut<CollectionTimes>) {
        let elapsed = times.started.take().unwrap().elapsed().as_secs_f64() * 1e3;
        times.samples.push(elapsed);
    }
    const PROBE: &str = r#"
fn declare() {
    #{provider: "contact_cost", key: "sample", schemas: [#{key: "count", fields: [
        #{tag: 1, name: "samples", kind: "u32"}]}], entries: [
        #{key: "observe", phase: "body_contact", trigger: #{port: "body_contact", selector: "contact_cost.observe"},
          writes: ["count"]}]}
}
fn observe() {
    if this.trigger.incomplete { throw "incomplete probe material"; }
    this.state.count.samples += 1;
}
"#;
    let file = std::env::temp_dir().join(format!("ambition_contact_cost_{}.rhai", std::process::id()));
    std::fs::write(&file, PROBE).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("switch_lab")
            .with_extension_module_files(vec![file.clone()])).unwrap();
    sim.step_n(AgentAction::default(), 4);
    let schedule = sim.app_mut().sim_schedule();
    sim.app_mut().init_resource::<CollectionTimes>().add_systems(schedule,
        (begin.before(queue_body_contacts), end.after(queue_body_contacts))
            .in_set(ExtensionSet::Collect(BODY_CONTACT)));
    let world = sim.world_mut();
    let root = world.query_filtered::<Entity, With<RoomInstanceRoot>>().single(world).unwrap();
    let owner = *world.get::<SessionScopedEntity>(root).unwrap();
    let geometry = world.get::<RoomGeometry>(root).unwrap().clone();
    let definition = *world.get::<LiveRoomDefinition>(root).unwrap();
    let mut probes = Vec::new();
    for room_count in [1, 2] {
        if room_count == 2 {
            let room = LiveRoomInstance::from_ordinal(1);
            let world = sim.world_mut();
            assert!(session_world_component_mut::<RoomSet>(world).unwrap().mint_live_room(room));
            let extra = spawn_live_room(world, room, geometry.clone());
            world.entity_mut(extra).insert(definition);
        }
        let room = LiveRoomInstance::from_ordinal(room_count - 1);
        for index in 0..256 {
            let x = (index % 16) as f32 * 64.0;
            let y = (index / 16) as f32 * 96.0;
            let entity = sim.world_mut().spawn((owner, InRoomInstance(room),
                SimId::placement(&format!("contact-cost-{index:03}")),
                BodyKinematics {pos: ae::Vec2::new(x, y), canonical_size: ae::Vec2::new(64.0, 48.0), ..Default::default()},
                BodyHealth::new(Health::new(3)), BodyStatuses::default())).id();
            probes.push((entity, index));
        }
        for many in [false, true] {
            for (entity, index) in &probes {
                sim.world_mut().entity_mut(*entity).insert(BodyStatuses(if many || *index < 4 {
                    vec!["contact_cost.observe".into()]
                } else { vec![] }));
            }
            sim.step_n(AgentAction::default(), 4);
            sim.world_mut().resource_mut::<CollectionTimes>().samples.clear();
            let mut frames = Vec::new();
            for _ in 0..32 {
                let started = Instant::now();
                sim.step(AgentAction::default());
                frames.push(started.elapsed().as_secs_f64() * 1e3);
            }
            assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
            let observers = room_count as usize * if many { 256 } else { 4 };
            let record = SchemaKey::new("contact_cost", "count", 1);
            let observed = probes.iter().filter(|(entity, _)| sim.world().get::<BodyRecords>(*entity)
                .and_then(|records| records.get(&record)).is_some()).count();
            assert!(observed >= observers);
            let mut collection = std::mem::take(&mut sim.world_mut().resource_mut::<CollectionTimes>().samples);
            assert_eq!(collection.len(), frames.len());
            collection.sort_by(f64::total_cmp);
            frames.sort_by(f64::total_cmp);
            eprintln!("CONTACT_COST rooms={room_count} bodies={} observers={observers} collect_median_ms={:.3} collect_p95_ms={:.3} step_median_ms={:.3} step_p95_ms={:.3}",
                probes.len(), collection[16], collection[30], frames[16], frames[30]);
        }
    }
    std::fs::remove_file(file).unwrap();
}
