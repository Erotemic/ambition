//! Shared script behavior consumes native props with one atomic exchange.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    breakables::BreakableFeature,
    characters::actor::{ActorFaction, BodyWallet},
    combat::{components::{CenteredAabb, FeatureId}, extension::status::BodyStatuses},
    engine_core::{self as ae, AabbExt},
    extension::{self, BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}},
    items::{Item, OwnedItems},
    platformer::{lifecycle::InRoomInstance, markers::PrimaryPlayerOnly, sim_id::SimId},
    sim_view::FeatureViewIndex,
};
use bevy::{ecs::system::SystemState, prelude::*};

const CLAIM: &str = "Breakable-0172";
const SHOP: &str = "Breakable-0173";
const LIBRARY: &str = r#"
fn schema(key) {
    #{key: key, fields: [
        #{tag: 1, name: "sequence", kind: "u32"},
        #{tag: 2, name: "pending", kind: "bool"},
        #{tag: 3, name: "applied", kind: "u32"},
        #{tag: 4, name: "refused", kind: "u32"},
        #{tag: 5, name: "done", kind: "bool"},
        #{tag: 6, name: "retried", kind: "bool"},
        #{tag: 7, name: "index", kind: "u32"},
        #{tag: 8, name: "object", kind: "text 128"},
        #{tag: 9, name: "code", kind: "text 64"},
    ]}
}
fn terms(cfg, sequence) {
    if cfg.slot == "claim" {
        if sequence == 1 { return #{pay_coins: 1}; }
        return #{receive_coins: 3};
    }
    #{pay_coins: cfg.price, grants: [#{id: "healthcell", count: 1}]}
}
fn submit(ctx, cfg, sequence, index) {
    ctx.requests.push(request("consume_contact", #{
        reply: #{request: sequence, selector: cfg.reply}, contact: index,
        exchange: terms(cfg, sequence),
    }));
    ctx
}
fn sense(ctx, cfg) {
    let state = ctx.state[cfg.slot];
    if state.pending { return ctx; }
    if state.done {
        if state.applied == 1 && !state.retried {
            ctx = submit(ctx, cfg, state.sequence, state.index);
            ctx.state[cfg.slot].retried = true;
        }
        return ctx;
    }
    if !ctx.trigger.world_available { throw "no current movement output"; }
    for index in 0..ctx.trigger.world.len() {
        let hit = ctx.trigger.world[index];
        if hit.role != cfg.role || hit.block == () { continue; }
        if hit.block.geometry.origin != "placement" || hit.block.geometry.key != cfg.target { continue; }
        state.sequence += 1;
        state.pending = true;
        state.index = index;
        ctx.state[cfg.slot] = state;
        return submit(ctx, cfg, state.sequence, index);
    }
    ctx
}
fn result(ctx, cfg) {
    if ctx.trigger.ticket.domain != "world.consume_contact" { throw "wrong domain result"; }
    if !ctx.trigger.terminal { return ctx; }
    let state = ctx.state[cfg.slot];
    if state.done { return ctx; }
    state.pending = false;
    state.code = ctx.trigger.code;
    if ctx.trigger.stage == "applied" {
        if ctx.trigger.code != "contact_consumed" || ctx.trigger.subjects.len() != 1 {
            throw "false consumption";
        }
        state.applied += 1;
        state.object = ctx.trigger.subjects[0];
        state.done = true;
    } else if ctx.trigger.stage == "rejected" {
        state.refused += 1;
        if ctx.trigger.code != "insufficient_funds" {
            if ctx.trigger.code != "object_already_consumed" { throw ctx.trigger.code; }
            state.done = true;
        }
    } else { throw "unexpected consumption stage"; }
    ctx.state[cfg.slot] = state;
    ctx
}
"#;
const ROOT: &str = r#"
import "lib/consume.rhai" as consume;
fn declare() {
    #{provider: "consume_probe", key: "objects", schemas: [consume::schema("claim"), consume::schema("purchase")],
      entries: [
        #{key: "claim", phase: "body_contact", trigger: #{port: "body_contact", selector: "consume_probe.claim"},
          writes: ["claim"], requests: ["consume_contact"], limit: 1},
        #{key: "purchase", phase: "body_contact", trigger: #{port: "body_contact", selector: "consume_probe.purchase"},
          writes: ["purchase"], requests: ["consume_contact"], limit: 1},
        #{key: "claimed", phase: "actor_think", trigger: #{port: "action_result", selector: "claimed"}, writes: ["claim"]},
        #{key: "purchased", phase: "actor_think", trigger: #{port: "action_result", selector: "purchased"}, writes: ["purchase"]},
      ]}
}
fn claim_config() {
    #{slot: "claim", target: "Breakable-0172", role: "support", reply: "consume_probe::objects/claimed"}
}
fn purchase_config() {
    #{slot: "purchase", target: "Breakable-0173", role: "head", price: 1, reply: "consume_probe::objects/purchased"}
}
fn claim() { this = consume::sense(this, claim_config()); }
fn purchase() { this = consume::sense(this, purchase_config()); }
fn claimed() { this = consume::result(this, claim_config()); }
fn purchased() { this = consume::result(this, purchase_config()); }
"#;

fn source_files(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("ambition_contact_consume_{name}_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(dir.join("lib/consume.rhai"), LIBRARY).unwrap();
    let root = dir.join("objects.rhai");
    std::fs::write(&root, ROOT).unwrap();
    (dir, root)
}

fn options(file: std::path::PathBuf) -> ambition_app::rl_sim::Platformer2dSimHarnessOptions {
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    crate::common::fixed_60hz_room_options("basement_breakables")
        .with_save(save).with_extension_module_files(vec![file])
}

fn setup(sim: &mut Platformer2dSimHarness) -> [Entity; 2] {
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    world.init_resource::<extension::ExtensionProfile>();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    let child = extension::create_actors(world, player, &[extension::RuntimeActor {
        key: "customer".into(), character: "practice_sandbag".into(),
        aabb: ae::Aabb::new(ae::Vec2::new(700.0, 500.0), ae::Vec2::new(12.0, 16.0)),
        faction: ActorFaction::Neutral, face_left: false, starting_wallet: Some(0), holds: Vec::new(),
    }], &[], Default::default()).unwrap().remove(0).entity;
    world.get_mut::<BodyWallet>(player).unwrap().balance = 0;
    assert_eq!(world.get::<BodyWallet>(child).unwrap().balance, 0);
    [player, child]
}

fn prop(world: &mut World, id: &str) -> Entity {
    world.query::<(Entity, &FeatureId, &BreakableFeature)>().iter(world)
        .find(|(_, feature, _)| feature.as_str() == id).unwrap_or_else(|| panic!("missing {id}")).0
}

fn persistent(world: &mut World, entity: Entity) {
    let mut state = world.get::<BreakableFeature>(entity).unwrap().state().clone();
    state.respawn = ambition_platformer2d::entity_catalog::placements::HazardRespawn::Persistent;
    world.entity_mut(entity).insert(BreakableFeature::new(state));
}

fn place(sim: &mut Platformer2dSimHarness, body: Entity, target: Entity, head: bool, offset: f32) {
    let world = sim.world_mut();
    let aabb = world.get::<CenteredAabb>(target).unwrap().aabb();
    let half = world.get::<ae::BodyKinematics>(body).unwrap().half_extent();
    let pos = ae::Vec2::new(aabb.center().x + offset,
        if head { aabb.bottom() + half.y + 12.0 } else { aabb.top() - half.y - 1.0 });
    let mut query = world.query::<(ae::BodyClusterQueryData, &mut ae::MotionModel)>();
    let (mut cluster, mut model) = query.get_mut(world, body).unwrap();
    ae::movement::transit_body(&mut model, &mut cluster.as_clusters_mut(), pos,
        ae::movement::TransitVelocity::Set(if head { -ae::Vec2::Y * 400.0 } else { ae::Vec2::ZERO }));
}

fn number(sim: &Platformer2dSimHarness, body: Entity, slot: &'static str, field: usize) -> u32 {
    let record = sim.world().get::<BodyRecords>(body).unwrap()
        .get(&SchemaKey::new("consume_probe", slot, 1)).unwrap();
    let Value::U32(value) = record.get(FieldRef(field)).unwrap() else { panic!("expected u32") };
    *value
}

fn assert_absent(sim: &mut Platformer2dSimHarness, target: Entity, id: &str) {
    assert!(sim.world().get::<BreakableFeature>(target).unwrap().broken());
    let view = sim.world().resource::<FeatureViewIndex>().get(id).unwrap();
    assert!(!view.visible);
    assert_eq!(view.breakable_state, Some(ambition_platformer2d::interaction::BreakableState::Broken));
    let room = *sim.world().get::<InRoomInstance>(target).unwrap();
    let mut collision = SystemState::<ambition_platformer2d::world::collision::CollisionWorld>::new(sim.world_mut());
    let collision = collision.get(sim.world()).unwrap();
    let solids = collision.room(Some(&room)).unwrap().solids().unwrap();
    assert!(!solids.blocks.iter().any(|block| block.id == ae::GeoId::placement(ae::PlacementId::new(id), 0)));
}

#[test]
fn competing_contacts_commit_one_reward_and_replay_and_save_the_spent_object() {
    let (dir, file) = source_files("claim");
    let mut sim = Platformer2dSimHarness::new_with_options(options(file.clone())
        .with_sync_test_rollback_settings(4, 12)).unwrap();
    let bodies = setup(&mut sim);
    let target = prop(sim.world_mut(), CLAIM);
    persistent(sim.world_mut(), target);
    for (index, body) in bodies.into_iter().enumerate() {
        place(&mut sim, body, target, false, index as f32 * 8.0 - 4.0);
        sim.world_mut().entity_mut(body).insert(BodyStatuses(vec!["consume_probe.claim".into()]));
    }
    sim.rebase_rollback_history().unwrap();
    let mut saw_failure = false;
    for _ in 0..24 {
        sim.step(AgentAction::default());
        if bodies.iter().all(|body| sim.world().get::<BodyRecords>(*body)
            .and_then(|records| records.get(&SchemaKey::new("consume_probe", "claim", 1)))
            .is_some_and(|record| record.get(FieldRef(0)).unwrap() == &Value::U32(1)))
            && !sim.world().get::<BreakableFeature>(target).unwrap().broken() {
            saw_failure = true;
            for body in bodies { assert_eq!(sim.world().get::<BodyWallet>(body).unwrap().balance, 0); }
        }
    }
    assert!(saw_failure, "failed costs must leave both the object and wallets unchanged");
    assert_eq!(bodies.iter().map(|body| number(&sim, *body, "claim", 2)).sum::<u32>(), 1);
    assert_eq!(bodies.iter().map(|body| number(&sim, *body, "claim", 3)).sum::<u32>(), 3);
    assert_eq!(bodies.iter().map(|body| sim.world().get::<BodyWallet>(*body).unwrap().balance).sum::<i32>(), 3);
    let winner = bodies.into_iter().find(|body| number(&sim, *body, "claim", 2) == 1).unwrap();
    let record = sim.world().get::<BodyRecords>(winner).unwrap().get(&SchemaKey::new("consume_probe", "claim", 1)).unwrap();
    assert_eq!(record.get(FieldRef(5)).unwrap(), &Value::Bool(true));
    assert_eq!(record.get(FieldRef(7)).unwrap(), &Value::Text(sim.world().get::<SimId>(target).unwrap().as_str().into()));
    assert_absent(&mut sim, target, CLAIM);
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0, "{:?}", sim.world().resource::<ExtensionFaults>().recent);
    sim.rollback_health().unwrap();
    for (path, cost) in &sim.world().resource::<extension::ExtensionProfile>().0 {
        if path.starts_with("consume_probe::") && cost.calls > 0 {
            println!("CONSUMPTION_COST {path} calls={} backend_us={} host_us={}", cost.calls,
                cost.backend.as_micros() as f64 / cost.calls as f64,
                cost.total.as_micros() as f64 / cost.calls as f64);
        }
    }
    let saved = sim.world().resource::<ambition_platformer2d::session::AmbitionGameSave>().data().clone();
    let mut restored = Platformer2dSimHarness::new_with_options(options(file).with_save(saved)).unwrap();
    restored.step_n(AgentAction::default(), 4);
    let target = prop(restored.world_mut(), CLAIM);
    assert_absent(&mut restored, target, CLAIM);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_autonomous_head_contact_buys_an_item_through_the_same_behavior_and_replays() {
    let (dir, file) = source_files("purchase");
    let mut sim = Platformer2dSimHarness::new_with_options(options(file)
        .with_sync_test_rollback_settings(4, 12)).unwrap();
    let [_, customer] = setup(&mut sim);
    let target = prop(sim.world_mut(), SHOP);
    sim.world_mut().resource_mut::<OwnedItems>().take(Item::HealthCell, u32::MAX);
    sim.world_mut().entity_mut(customer).insert((BodyWallet {balance: 2},
        BodyStatuses(vec!["consume_probe.purchase".into()])));
    place(&mut sim, customer, target, true, 0.0);
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 24);
    assert_eq!(number(&sim, customer, "purchase", 2), 1);
    assert_eq!(number(&sim, customer, "purchase", 3), 0);
    assert_eq!(sim.world().get::<BodyWallet>(customer).unwrap().balance, 1);
    assert_eq!(sim.world().resource::<OwnedItems>().count(Item::HealthCell), 1);
    assert_absent(&mut sim, target, SHOP);
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0, "{:?}", sim.world().resource::<ExtensionFaults>().recent);
    sim.rollback_health().unwrap();
    for (path, cost) in &sim.world().resource::<extension::ExtensionProfile>().0 {
        if path.starts_with("consume_probe::") && cost.calls > 0 {
            println!("CONSUMPTION_COST {path} calls={} backend_us={} host_us={}", cost.calls,
                cost.backend.as_micros() as f64 / cost.calls as f64,
                cost.total.as_micros() as f64 / cost.calls as f64);
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn native_contact_consumption_refuses_foreign_steps_protection_and_invalid_indices() {
    use ambition_platformer2d::{breakables::consumption::consume_contact,
        characters::actor::Invulnerability, items::transaction::InventoryExchange};
    let (dir, file) = source_files("refusals");
    let mut sim = Platformer2dSimHarness::new_with_options(options(file)).unwrap();
    let [_, customer] = setup(&mut sim);
    let target = prop(sim.world_mut(), SHOP);
    place(&mut sim, customer, target, true, 0.0);
    let mut contact = None;
    for _ in 0..12 {
        sim.step(AgentAction::default());
        let frame = sim.world().get::<ae::BodyFrameOutput>(customer).unwrap();
        contact = frame.events.contacts.iter().position(|hit| matches!(&hit.source,
            ae::collision_semantics::ContactSource::Block {id, ..}
                if *id == ae::GeoId::placement(ae::PlacementId::new(SHOP), 0)));
        if contact.is_some() { break; }
    }
    let contact = contact.expect("native head contact") as u32;
    let exchange = InventoryExchange {receive_coins: 3, ..Default::default()};
    let world = sim.world_mut();
    assert_eq!(consume_contact(world, customer, u32::MAX, &exchange).err(), Some("contact_outside_budget"));
    let stamp = world.get::<ae::BodyFrameOutput>(customer).unwrap().stamp;
    for foreign_room in [true, false] {
        let mut other = stamp.unwrap();
        if foreign_room { other.room = Some(other.room.unwrap().wrapping_add(1)); }
        else { other.session = Some(other.session.unwrap().wrapping_add(1)); }
        world.get_mut::<ae::BodyFrameOutput>(customer).unwrap().stamp = Some(other);
        assert_eq!(consume_contact(world, customer, contact, &exchange).err(), Some("stale_contact"));
    }
    world.get_mut::<ae::BodyFrameOutput>(customer).unwrap().stamp = stamp;
    let intact = world.get::<BreakableFeature>(target).unwrap().clone();
    let mut protected = intact.state().clone();
    protected.health.invulnerable.set(Invulnerability::SCRIPTED, true);
    world.entity_mut(target).insert(BreakableFeature::new(protected));
    assert_eq!(consume_contact(world, customer, contact, &exchange).err(), Some("object_protected"));
    assert!(!world.get::<BreakableFeature>(target).unwrap().broken());
    assert_eq!(world.get::<BodyWallet>(customer).unwrap().balance, 0);
    world.entity_mut(target).insert(intact);
    let receipt = consume_contact(world, customer, contact, &exchange).unwrap();
    assert_eq!(&receipt.object, world.get::<SimId>(target).unwrap());
    assert_eq!(receipt.exchange.coins, 3);
    assert_eq!(consume_contact(world, customer, contact, &exchange).err(), Some("object_already_consumed"));
    assert_eq!(world.get::<BodyWallet>(customer).unwrap().balance, 3);
    sim.step_n(AgentAction::default(), 2);
    assert_absent(&mut sim, target, SHOP);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_edited_price_changes_native_consumption_without_rebuilding_the_host() {
    let (dir, file) = source_files("edit");
    std::fs::write(&file, ROOT.replace("price: 1,", "price: 3,")).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(options(file.clone())).unwrap();
    let [_, customer] = setup(&mut sim);
    let target = prop(sim.world_mut(), SHOP);
    sim.world_mut().resource_mut::<OwnedItems>().take(Item::HealthCell, u32::MAX);
    sim.world_mut().entity_mut(customer).insert((BodyWallet {balance: 2},
        BodyStatuses(vec!["consume_probe.purchase".into()])));
    place(&mut sim, customer, target, true, 0.0);
    sim.step_n(AgentAction::default(), 12);
    assert_eq!(number(&sim, customer, "purchase", 3), 1);
    assert!(!sim.world().get::<BreakableFeature>(target).unwrap().broken());
    assert_eq!(sim.world().get::<BodyWallet>(customer).unwrap().balance, 2);
    assert_eq!(sim.world().resource::<OwnedItems>().count(Item::HealthCell), 0);
    let generation = sim.world().resource::<extension::ExtensionGeneration>().clone();
    std::fs::write(&file, ROOT).unwrap();
    sim.step_n(AgentAction::default(), 50);
    assert_ne!(*sim.world().resource::<extension::ExtensionGeneration>(), generation);
    assert_eq!(number(&sim, customer, "purchase", 3), 1);
    place(&mut sim, customer, target, true, 0.0);
    sim.step_n(AgentAction::default(), 12);
    assert_eq!(number(&sim, customer, "purchase", 2), 1);
    assert_eq!(sim.world().get::<BodyWallet>(customer).unwrap().balance, 1);
    assert_eq!(sim.world().resource::<OwnedItems>().count(Item::HealthCell), 1);
    assert_absent(&mut sim, target, SHOP);
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0, "{:?}", sim.world().resource::<ExtensionFaults>().recent);
    let _ = std::fs::remove_dir_all(dir);
}
