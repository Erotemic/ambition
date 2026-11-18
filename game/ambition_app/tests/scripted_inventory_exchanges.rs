//! Loaded rules exchange real quantities and currency through the native owner.

#![cfg(feature = "rl_sim")]
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    characters::actor::{BodyWallet, CharacterForm},
    combat::extension::status::BodyStatuses,
    extension::{
        BodyRecords, ExtensionFaults,
        sdk::{FieldRef, SchemaKey, Value},
    },
    items::{Item, ItemCatalog, OwnedItems},
    platformer::markers::PrimaryPlayerOnly,
};
use bevy::prelude::*;
const RECORD: SchemaKey = SchemaKey::new("exchange_probe", "trade", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "exchange_probe", key: "trade", schemas: [#{ key: "trade", fields: [
        #{ tag: 1, name: "sent", kind: "bool" },
        #{ tag: 2, name: "applied", kind: "u32" },
        #{ tag: 3, name: "refused", kind: "u32" },
        #{ tag: 4, name: "wallet", kind: "i32" },
    ] }], entries: [
        #{ key: "trade", phase: "technique_execution", trigger: #{ port: "status_tick", selector: "exchange_probe.trade" },
           writes: ["trade"], requests: ["inventory_exchange"], limit: 4 },
        #{ key: "result", phase: "actor_think", trigger: #{ port: "action_result", selector: "result" },
           reads: ["inventory"], writes: ["trade"] },
    ] }
}
fn trade() {
    if this.state.trade.sent { return; }
    this.state.trade.sent = true;
    let trade = #{ reply: #{ request: 1, selector: "exchange_probe::trade/result" },
        exchange: #{ costs: [#{ id: "healthcell", count: 1 }], grants: [#{ id: "manacell", count: 1 }], pay_coins: 5 } };
    this.requests.push(request("inventory_exchange", trade));
    this.requests.push(request("inventory_exchange", trade));
    trade.reply.request = 2;
    trade.exchange.grants = [#{ id: "portalgun", count: 1 }];
    this.requests.push(request("inventory_exchange", trade));
    trade.reply.request = 3;
    trade.exchange.grants = [#{ id: "not_a_catalog_item", count: 1 }];
    this.requests.push(request("inventory_exchange", trade));
}
fn result() {
    if this.trigger.stage == "applied" {
        if this.trigger.code != "inventory_exchanged" || !this.trigger.terminal { throw "false exchange completion"; }
        this.state.trade.applied += 1;
    } else if this.trigger.stage == "rejected" {
        if this.trigger.code != "stack_full" && this.trigger.code != "unknown_item" { throw "wrong refusal"; }
        this.state.trade.refused += 1;
    }
    this.state.trade.wallet = this.observe.inventory.wallet;
}
"#;

#[test]
fn either_controller_trades_once_with_actual_refusals_observations_and_replay() {
    let dir = std::env::temp_dir().join(format!(
        "ambition_inventory_exchange_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("trade.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room")
            .with_save(save)
            .with_extension_module_files(vec![file.clone()])
            .with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    sim.step_n(AgentAction::default(), 4);
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
    let catalog = world.resource::<ItemCatalog>();
    let mut bag = OwnedItems::default();
    bag.grant(catalog, Item::HealthCell, 4);
    bag.grant(catalog, Item::PortalGun, 1);
    world.insert_resource(bag);
    for body in [player, teacher] {
        world.entity_mut(body).insert((
            BodyWallet { balance: 100 },
            BodyStatuses(vec!["exchange_probe.trade".into()]),
        ));
    }
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 20);
    sim.rollback_health().unwrap();
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    let bag = sim.world().resource::<OwnedItems>();
    assert_eq!(bag.count(Item::HealthCell), 2);
    assert_eq!(bag.count(Item::ManaCell), 2);
    assert_eq!(bag.count(Item::PortalGun), 1);
    for body in [player, teacher] {
        assert_eq!(sim.world().get::<BodyWallet>(body).unwrap().balance, 95);
        let record = sim
            .world()
            .get::<BodyRecords>(body)
            .unwrap()
            .get(&RECORD)
            .unwrap();
        assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::U32(1));
        assert_eq!(record.get(FieldRef(2)).unwrap(), &Value::U32(2));
        assert_eq!(record.get(FieldRef(3)).unwrap(), &Value::I32(95));
    }
    let saved = sim
        .world()
        .resource::<ambition_platformer2d::session::AmbitionGameSave>()
        .data()
        .clone();
    let mut restored = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room")
            .with_save(saved)
            .with_extension_module_files(vec![file]),
    )
    .unwrap();
    restored.step_n(AgentAction::default(), 4);
    let bag = restored.world().resource::<OwnedItems>();
    assert_eq!(bag.count(Item::HealthCell), 2);
    assert_eq!(bag.count(Item::ManaCell), 2);
    let world = restored.world_mut();
    let wallet = world
        .query_filtered::<&BodyWallet, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    assert_eq!(wallet.balance, 95);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_exchange_is_retracted_with_its_source_room_by_a_real_checkpoint_restore() {
    use ambition_platformer2d::items::transaction::{
        InventoryExchange, ItemAmount, apply_in_world,
    };
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("central_hub_complex")
            .with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let player = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    let catalog = world.resource::<ItemCatalog>();
    let mut bag = OwnedItems::default();
    bag.grant(catalog, Item::HealthCell, 4);
    world.insert_resource(bag);
    world.entity_mut(player).insert(BodyWallet { balance: 100 });
    crate::death_restores_the_checkpoint::waive_the_death_fee(&mut sim);
    crate::death_restores_the_checkpoint::commit_a_hub_checkpoint(&mut sim);
    let world = sim.world_mut();
    let player = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    let applied = apply_in_world(
        world,
        player,
        &InventoryExchange {
            costs: vec![ItemAmount {
                item: Item::HealthCell,
                count: 1,
            }],
            grants: vec![ItemAmount {
                item: Item::ManaCell,
                count: 1,
            }],
            pay_coins: 5,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(applied.coins, -5);
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 8);
    assert_eq!(
        sim.world().resource::<OwnedItems>().count(Item::HealthCell),
        3
    );
    assert_eq!(
        sim.world().resource::<OwnedItems>().count(Item::ManaCell),
        1
    );
    let (x, floor) = {
        let world = sim.world_mut();
        let x = world.query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, PrimaryPlayerOnly>().single(world).unwrap().pos.x;
        let geometry = world
            .query::<&ambition_platformer2d::engine_core::RoomGeometry>()
            .single(world)
            .unwrap();
        (x, geometry.0.size.y + geometry.0.edges.fall)
    };
    sim.teleport_player((x, floor + 300.0));
    sim.rebase_rollback_history().unwrap();
    let mut committed = false;
    for _ in 0..240 {
        sim.step(AgentAction::default());
        let outcomes = ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::session::checkpoint::SessionCheckpointOutcomes,
        >(sim.world())
        .unwrap();
        if matches!(
            outcomes.latest(),
            Some(
                ambition_platformer2d::session::checkpoint::CheckpointRestoreOutcome::Committed { .. }
            )
        ) {
            committed = true;
            break;
        }
    }
    assert!(
        committed,
        "the real checkpoint restore must commit: latest={:?}, outstanding={:?}",
        ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::session::checkpoint::SessionCheckpointOutcomes,
        >(sim.world())
        .unwrap()
        .latest(),
        ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::session::checkpoint::OutstandingCheckpointRequest,
        >(sim.world())
        .unwrap()
        .0
    );
    sim.step_n(AgentAction::default(), 8);
    sim.rollback_health().unwrap();
    let bag = sim.world().resource::<OwnedItems>();
    assert_eq!(bag.count(Item::HealthCell), 4);
    assert_eq!(bag.count(Item::ManaCell), 0);
    let world = sim.world_mut();
    assert_eq!(
        world
            .query_filtered::<&BodyWallet, PrimaryPlayerOnly>()
            .single(world)
            .unwrap()
            .balance,
        100
    );
}
