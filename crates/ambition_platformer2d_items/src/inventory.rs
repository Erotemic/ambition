//! Script exchange admission and native inventory history publication.

use ambition_characters::{actor::BodyWallet, control::DrivingParticipant};
use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox, LowersIn};
use ambition_extension_sdk::{
    action::ActionStage,
    phases::*,
};
use ambition_items::{
    Item, ItemCatalog, OwnedItems,
    transaction::ExchangeRefusal,
};
use ambition_platformer2d_shared_tangle::{
    action_results::publish_action_result,
    lifecycle::{InRoomInstance, LiveRoomInstance, RoomWrite, SessionScopedEntity, live_room_of},
    sim_id::SimId,
};
use ambition_world_port::inventory::{
    InventoryExchangePort, InventoryItem, InventoryView, InventoryViewPort,
};
use bevy::prelude::*;

pub use ambition_items::transaction::{InventoryExchange, ExchangeApplied};

pub const EXCHANGE_DOMAIN: &str = "inventory.exchange";
/// The action domain of a script's equip.
pub const EQUIP_DOMAIN: &str = "inventory.equip";
/// The action domain of a script's stow.
pub const STOW_DOMAIN: &str = "inventory.stow";

/// The body a seat drives whose identity is `id`, in the invocation's room.
fn party_body(world: &mut World, scope: Entity, id: &str) -> Result<Entity, &'static str> {
    world.run_system_cached_with(find_party_body, (scope, id.to_owned())).map_err(|_| "inventory_unavailable")?
}

fn find_party_body(
    In((scope, id)): In<(Entity, String)>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    bodies: Query<(Entity, &ambition_platformer2d_shared_tangle::sim_id::SimId, Has<DrivingParticipant>)>,
) -> Result<Entity, &'static str> {
    let here = rooms.of(scope);
    let (body, _, driven) = bodies
        .iter()
        .find(|(entity, sim, _)| sim.as_str() == id && rooms.of(*entity) == here)
        .ok_or("no_such_body")?;
    if !driven {
        return Err("not_a_party_body");
    }
    Ok(body)
}

fn equip_one(world: &mut World, scope: Entity, request: &ambition_world_port::inventory::EquipItem)
    -> Result<(Entity, &'static str), &'static str> {
    let body = party_body(world, scope, &request.body)?;
    let catalog = world.get_resource::<ItemCatalog>().ok_or("inventory_unavailable")?;
    let item = catalog.item_by_id(&request.item).filter(|item| catalog.is_real(*item)).ok_or("unknown_item")?;
    if catalog.held_item_id(item).is_some() {
        world.run_system_cached_with(ambition_held_items::change_hand, (body, Some(item)))
            .map_err(|_| "inventory_unavailable")?
            .map_err(ambition_held_items::HandRefusal::code)?;
        return Ok((body, "item_equipped"));
    }
    if catalog.wears_at(item, 1).is_none() {
        return Err("not_equippable");
    }
    world.run_system_cached_with(crate::narrative::wear_item, (body, item)).map_err(|_| "inventory_unavailable")??;
    Ok((body, "item_worn"))
}

fn stow_one(world: &mut World, scope: Entity, request: &ambition_world_port::inventory::StowHand)
    -> Result<Entity, &'static str> {
    let body = party_body(world, scope, &request.body)?;
    match world.run_system_cached_with(ambition_held_items::change_hand, (body, None)).map_err(|_| "inventory_unavailable")? {
        Ok(ambition_held_items::HandChange::Changed) => Ok(body),
        Ok(ambition_held_items::HandChange::Unchanged) => Err("nothing_held"),
        Err(refusal) => Err(refusal.code()),
    }
}

/// A script's equips, in submission order, each answered with what the hand
/// or the worn set did.
fn lower_equips(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<ambition_world_port::inventory::EquipItemPort>(&ACTOR_THINK);
    for submitted in requests {
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<ambition_world_port::inventory::EquipItemPort>(
            world, submitted.state_scope, &submitted.entry, &submitted.value);
        if !dispatch.execute {
            continue;
        }
        let (stage, code, subjects) = match equip_one(world, submitted.scope, &submitted.value) {
            Ok((body, code)) => (ActionStage::Applied, code,
                ambition_platformer2d_shared_tangle::action_results::object_action_subject(world, body).into_iter().collect()),
            Err(code) => (ActionStage::Rejected, code, Vec::new()),
        };
        publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", subjects);
    }
}

/// A script's stows, answered as the equips are.
fn lower_stows(world: &mut World) {
    let requests = world.resource_mut::<ExtensionOutbox>().drain::<ambition_world_port::inventory::StowHandPort>(&ACTOR_THINK);
    for submitted in requests {
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<ambition_world_port::inventory::StowHandPort>(
            world, submitted.state_scope, &submitted.entry, &submitted.value);
        if !dispatch.execute {
            continue;
        }
        let (stage, code, subjects) = match stow_one(world, submitted.scope, &submitted.value) {
            Ok(body) => (ActionStage::Applied, "hand_stowed",
                ambition_platformer2d_shared_tangle::action_results::object_action_subject(world, body).into_iter().collect()),
            Err(code) => (ActionStage::Rejected, code, Vec::new()),
        };
        publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", subjects);
    }
}

pub fn install(app: &mut App) {
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, EXCHANGE_DOMAIN,
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    use ambition_extension_host::{
        InActorThink, InBodyContact, InBossConduct, InHitReaction, InModuleEntityTick,
        InTechniqueExecution, InWieldedUse,
    };
    macro_rules! phase {
        ($phase:expr, $scope:ty) => {
            app.install_extension_submitted_request::<InventoryExchangePort, _>(
                $phase,
                "ambition_platformer2d_items",
                lower::<$scope>,
            );
            app.install_extension_observation::<InventoryViewPort>(
                $phase,
                "ambition_platformer2d_items",
                view,
            );
        };
    }
    phase!(ACTOR_THINK, InActorThink);
    for domain in [EQUIP_DOMAIN, STOW_DOMAIN] {
        ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, domain,
            ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    }
    app.install_extension_submitted_request::<ambition_world_port::inventory::EquipItemPort, _>(
        ACTOR_THINK, "ambition_platformer2d_items", lower_equips);
    app.install_extension_submitted_request::<ambition_world_port::inventory::StowHandPort, _>(
        ACTOR_THINK, "ambition_platformer2d_items", lower_stows);
    phase!(BODY_CONTACT, InBodyContact);
    phase!(BOSS_CONDUCT, InBossConduct);
    phase!(HIT_REACTION, InHitReaction);
    phase!(MODULE_ENTITY_TICK, InModuleEntityTick);
    phase!(TECHNIQUE_EXECUTION, InTechniqueExecution);
    phase!(WIELDED_USE, InWieldedUse);
}

fn view(world: &World, scope: Entity) -> Option<InventoryView> {
    let (bag, catalog) = (
        world.get_resource::<OwnedItems>()?,
        world.get_resource::<ItemCatalog>()?,
    );
    Some(InventoryView {
        items: Item::ALL
            .into_iter()
            .filter(|item| catalog.is_real(*item))
            .map(|item| InventoryItem {
                id: catalog.dialog_id(item).unwrap().into(),
                count: bag.count(item),
                capacity: catalog.stack(item).unwrap(),
            })
            .collect(),
        wallet: world.get::<BodyWallet>(scope).map(|wallet| wallet.balance),
    })
}

/// Apply through the native item owner, then record its actual changes.
/// This service is also the inventory leg of a world-object transaction.
pub fn apply(
    world: &mut World,
    scope: Entity,
    exchange: &ambition_items::transaction::InventoryExchange,
) -> Result<ExchangeApplied, &'static str> {
    if world.query::<()>().get(world, scope).is_err() {
        return Err("inactive_scope");
    }
    let Some(session) = world.get::<SessionScopedEntity>(scope).map(|scope| scope.0) else {
        return Err("invalid_scope");
    };
    let Some(payer) = world.get::<SimId>(scope).cloned() else {
        return Err("unidentified_caller");
    };
    let Some(room) = live_room_of(
        world.get::<InRoomInstance>(scope),
        world.get::<LiveRoomInstance>(scope),
    ) else {
        return Err("unscoped_caller");
    };
    if !world.contains_resource::<ItemCatalog>() || !world.contains_resource::<OwnedItems>() {
        return Err("inventory_unavailable");
    }
    if !world.contains_resource::<crate::narrative::BagUsesInRooms>()
        || !world.contains_resource::<crate::pickup::BagChangesSinceCheckpoint>()
    {
        return Err("inventory_history_unavailable");
    }
    let mut bag = world.resource::<OwnedItems>().clone();
    let mut wallet = world.get::<BodyWallet>(scope).copied();
    let applied = exchange
        .apply(world.resource::<ItemCatalog>(), &mut bag, wallet.as_mut())
        .map_err(ExchangeRefusal::code)?;
    let tick = world
        .get_resource::<ambition_time::SimTick>()
        .map_or(0, |tick| tick.0);
    let written = Some(RoomWrite { room, tick });
    let mut owners: Vec<_> = world
        .query::<(&DrivingParticipant, &InRoomInstance, &SessionScopedEntity)>()
        .iter(world)
        .filter(|(_, stamp, owner)| stamp.0 == room && owner.0 == session)
        .map(|(driver, _, _)| driver.0)
        .collect();
    owners.sort_by_key(|owner| owner.0);
    owners.dedup();
    *world.resource_mut::<OwnedItems>() = bag;
    if let Some(wallet) = wallet {
        if let Some(mut current) = world.get_mut::<BodyWallet>(scope) {
            *current = wallet;
        }
    }
    use crate::pickup::minted_horizon::{BagChange, BagChangeRetention};
    let mut changes: Vec<_> = applied
        .items
        .iter()
        .map(|change| BagChange {
            item: Some(change.item),
            gained: change.added.saturating_sub(change.removed),
            gave: change.removed.saturating_sub(change.added),
            payer: Some(payer.clone()),
            coins: 0,
            written,
            retention: BagChangeRetention::RoomOwners(owners.clone()),
        })
        .collect();
    if applied.coins != 0 {
        changes.push(BagChange {
            item: None,
            gained: 0,
            gave: 0,
            payer: Some(payer),
            coins: applied.coins,
            written,
            retention: BagChangeRetention::RoomOwners(owners),
        });
    }
    for change in &applied.items {
        let mut log = world.resource_mut::<crate::narrative::BagUsesInRooms>();
        log.record_spend(
            written.unwrap(),
            change.item,
            change.removed.saturating_sub(change.added),
        );
        log.record_gain(
            written.unwrap(),
            change.item,
            change.added.saturating_sub(change.removed),
        );
    }
    let mut history = world.resource_mut::<crate::pickup::BagChangesSinceCheckpoint>();
    for change in changes {
        history.record(change);
    }
    Ok(applied)
}

fn lower<L: LowersIn>(world: &mut World) {
    let requests = world
        .resource_mut::<ExtensionOutbox>()
        .drain::<InventoryExchangePort>(&L::PHASE);
    for submitted in requests {
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<InventoryExchangePort>(
            world, submitted.state_scope, &submitted.entry, &submitted.value);
        if !dispatch.execute {
            continue;
        }
        let outcome = lower_one(world, submitted.scope, &submitted.value);
        let (stage, code, subjects) = match outcome {
            Ok(subjects) => (ActionStage::Applied, "inventory_exchanged", subjects),
            Err(code) => (ActionStage::Rejected, code, Vec::new()),
        };
        publish_action_result(
            world,
            dispatch.reply.as_ref(),
            stage,
            true,
            code,
            "",
            subjects.into_iter().map(ambition_extension_sdk::action::ActionSubject::named).collect(),
        );
    }
}
fn lower_one(
    world: &mut World,
    scope: Entity,
    request: &ambition_world_port::inventory::InventoryExchange,
) -> Result<Vec<String>, &'static str> {
    let catalog = world.get_resource::<ItemCatalog>().ok_or("inventory_unavailable")?;
    let exchange = resolve(catalog, &request.exchange)?;
    let subjects = exchange
        .costs
        .iter()
        .chain(&exchange.grants)
        .map(|line| catalog.dialog_id(line.item).unwrap().to_owned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    apply(world, scope, &exchange)?;
    Ok(subjects)
}

/// Resolve authored item IDs before a domain transaction mutates state.
pub fn resolve(catalog: &ItemCatalog, terms: &ambition_world_port::inventory::InventoryTerms)
    -> Result<InventoryExchange, &'static str> {
    let lines = |lines: &[ambition_world_port::inventory::ItemAmount]| {
        lines.iter().map(|line| Ok(ambition_items::transaction::ItemAmount {
            item: catalog.item_by_id(&line.id).filter(|item| catalog.is_real(*item)).ok_or("unknown_item")?,
            count: line.count,
        })).collect::<Result<Vec<_>, &'static str>>()
    };
    Ok(InventoryExchange {
        costs: lines(&terms.costs)?, grants: lines(&terms.grants)?,
        pay_coins: terms.pay_coins, receive_coins: terms.receive_coins,
    })
}
