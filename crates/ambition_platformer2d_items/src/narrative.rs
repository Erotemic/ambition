//! What the narrative asked for, applied by the simulation.
//!
//! so the grant did not survive a rewind, and could not be replayed
//! either. A rollback restored the bag and the balance to before the purchase,
//! and nothing re-ran the command: the Yarn runner is not rewound (deliberately)
//! and it does not execute between resimulated ticks. The player watched an item
//! arrive and the authoritative world disagreed.
//!
//! The commands now record a REQUEST in the conversation's narrative ledger, and
//! these systems apply it on the tick it was stamped for — in the original run
//! and in every replay of that tick.
//!
//! The pure cores are unit-tested without a `World`: a shop deal is the item
//! owner's `InventoryExchange` (`ShopTransactionRequested::exchange`) and a
//! grant is `OwnedItems::grant`. These systems decide who calls them and when.

use bevy::prelude::*;
use ambition_platformer2d_core::replay_messages::ReplayMessageReader;

use ambition_characters::actor::BodyWallet;
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_items::{shop::ShopTransactionRequested, ItemGrantRequested, OwnedItems};

/// The room a narrative request is the act of, and the tick: its seat's
/// body's room (ISL-2d). `None` when no seat is named, or its body is in no
/// room, or there is no log to keep it in.
fn narrative_write(
    participant: Option<PlayerSlot>,
    drivers: &Query<(Entity, &DrivingParticipant)>,
    rooms: &Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
    tick: Option<&ambition_time::SimTick>,
) -> Option<ambition_platformer2d_shared_tangle::lifecycle::RoomWrite> {
    let body = driven_body(participant?, drivers)?;
    let room = rooms.get(body).ok()?.0;
    Some(ambition_platformer2d_shared_tangle::lifecycle::RoomWrite { room, tick: tick?.0 })
}

/// Grant what a conversation gave. (sim)
///
/// Under one rollback authority the grant enters the bag on its tick. A
/// conversation in a room that rewinds alone proposes it to the committed
/// bag instead ([`crate::committed_bag`]): no other room can spend it until
/// its tick is confirmed, and a rewind of the room drops the proposal. A
/// grant the bag refuses (no authority, no credits) is not given: it is
/// logged, never applied on this tick instead.
pub fn apply_item_grants(
    world: &mut World,
    requests: &mut bevy::ecs::system::SystemState<ReplayMessageReader<ItemGrantRequested>>,
) {
    let Ok(mut reader) = requests.get_mut(world) else { return; };
    let requests: Vec<ItemGrantRequested> = reader.read().copied().collect();
    for request in requests {
        let body = request.participant.and_then(|participant| driven_body_in(world, participant));
        let tick = world.get_resource::<ambition_time::SimTick>().map(|tick| tick.0);
        let written = body
            .and_then(|body| world.get::<ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>(body))
            .zip(tick)
            .map(|(room, tick)| ambition_platformer2d_shared_tangle::lifecycle::RoomWrite { room: room.0, tick });
        match crate::committed_bag::admit_grant(world, body, request.item, request.count) {
            crate::committed_bag::BagAdmission::Immediate => {}
            crate::committed_bag::BagAdmission::Held => continue,
            crate::committed_bag::BagAdmission::Refused(why) => {
                warn!(target: "crate::narrative", "give_item: {}x {:?} was refused ({why})", request.count, request.item);
                continue;
            }
        }
        let Some(before) = world.get_resource::<OwnedItems>().map(|bag| bag.count(request.item)) else { continue; };
        world.resource_scope(|world, mut owned: Mut<OwnedItems>| {
            owned.grant(ambition_items::item_catalog(world), request.item, request.count);
        });
        let gained = world.resource::<OwnedItems>().count(request.item) - before;
        if let Some(mut since) = world.get_resource_mut::<crate::pickup::BagChangesSinceCheckpoint>() {
            since.record(crate::pickup::BagChange {
                item: Some(request.item),
                gained,
                gave: 0,
                payer: None,
                coins: 0,
                written,
                retention: crate::pickup::minted_horizon::BagChangeRetention::NarrativeFacts,
            });
        }
        info!(
            target: "crate::narrative",
            "give_item: granted {}x {:?}", request.count, request.item,
        );
    }
}

/// Run what a merchant conversation agreed to. (sim)
///
/// The wallet is the one of the seat whose conversation it is (its input
/// owner): a price quoted in a text box is quoted to the person reading it.
/// A request that names no seat is the primary player's. What the bag gains
/// or gives up is dated in that body's room (ISL-2d); the wallet is on the
/// body, so a rewind of its room puts the wallet back with it.
pub fn apply_shop_transactions(
    mut requests: ReplayMessageReader<ShopTransactionRequested>,
    mut owned: ResMut<OwnedItems>,
    mut wallets: Query<&mut BodyWallet>,
    primary: Query<Entity, (With<BodyWallet>, With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>)>,
    items: ambition_items::ItemCatalogRead,
    (drivers, rooms, tick, mut log, mut since, ids): (
        Query<(Entity, &DrivingParticipant)>,
        Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        Option<Res<ambition_time::SimTick>>,
        Option<ResMut<BagUsesInRooms>>,
        Option<ResMut<crate::pickup::BagChangesSinceCheckpoint>>,
        Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    ),
) {
    for request in requests.read() {
        let payer = payer_among(
            request.participant,
            drivers.iter().map(|(body, driver)| (body, driver.0)),
            primary.iter(),
        );
        let Some(mut wallet) = payer.and_then(|payer| wallets.get_mut(payer).ok()) else {
            continue;
        };
        // The item owner's exchange is the whole law (A7), and its receipt
        // names what changed: no before/after reconstruction here.
        let outcome = request.apply(&mut wallet, &mut owned, items.get());
        info!(
            target: "crate::narrative",
            "{:?} {:?} @ {} -> {outcome:?} (balance now {})",
            request.side, request.item, request.price, wallet.balance,
        );
        let Ok(applied) = outcome else {
            continue;
        };
        let written = narrative_write(request.participant, &drivers, &rooms, tick.as_deref());
        for change in &applied.items {
            if let Some(since) = since.as_deref_mut() {
                since.record(crate::pickup::BagChange {
                    item: Some(change.item),
                    gained: change.added,
                    gave: change.removed,
                    payer: payer.and_then(|payer| ids.get(payer).ok()).cloned(),
                    coins: applied.coins,
                    written,
                    retention: crate::pickup::minted_horizon::BagChangeRetention::NarrativeFacts,
                });
            }
            if let (Some(written), Some(log)) = (written, log.as_deref_mut()) {
                if change.added > 0 {
                    log.record_gain(written, change.item, change.added);
                }
                for _ in 0..change.removed {
                    log.record(written, change.item);
                }
            }
        }
    }
}

/// How much one Health Cell heals.
pub const HEALTH_CELL_HEAL: i32 = 4;
/// How much Mana one Mana Cell restores.
pub const MANA_CELL_RESTORE: f32 = 40.0;

/// What a merchant's deal took from the shared bag, with the room and the
/// tick (ISL-2d): a rewind of that room puts it back, since the deal's other
/// side is in the room's snapshot, and its replay deals again. And what a
/// purchase added, which the rewind takes back. Cell grants and uses do not
/// write here: in a room that rewinds alone they take the committed road
/// ([`crate::committed_bag`]). Kept within the rewind horizon
/// ([`ambition_platformer2d_shared_tangle::lifecycle::ROOM_REWIND_HORIZON_TICKS`]).
///
/// Rollback state with a real value: a use writes it on a tick.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct BagUsesInRooms {
    uses: Vec<(ambition_platformer2d_shared_tangle::lifecycle::RoomWrite, ambition_items::Item, u32)>,
    gains: Vec<(ambition_platformer2d_shared_tangle::lifecycle::RoomWrite, ambition_items::Item, u32)>,
}

impl BagUsesInRooms {
    /// Remember a use of one `item` from a room.
    pub fn record(&mut self, at: ambition_platformer2d_shared_tangle::lifecycle::RoomWrite, item: ambition_items::Item) {
        self.record_spend(at, item, 1);
    }
    pub fn record_spend(&mut self, at: ambition_platformer2d_shared_tangle::lifecycle::RoomWrite, item: ambition_items::Item, count: u32) {
        self.uses.retain(|(written, ..)| !written.beyond_the_horizon(at.tick));
        if count > 0 { self.uses.push((at, item, count)); }
    }

    /// Remember `count` of `item` a room's conversation added to the bag.
    pub fn record_gain(
        &mut self,
        at: ambition_platformer2d_shared_tangle::lifecycle::RoomWrite,
        item: ambition_items::Item,
        count: u32,
    ) {
        self.gains.retain(|(written, ..)| !written.beyond_the_horizon(at.tick));
        if count > 0 {
            self.gains.push((at, item, count));
        }
    }

    /// `room` rewinds to `saved_at`: take out the uses it made after that tick
    /// and return their items, so they go back in the bag.
    pub fn retract_room_spends(
        &mut self,
        room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        saved_at: u64,
    ) -> Vec<(ambition_items::Item, u32)> {
        if !self.uses.iter().any(|(written, ..)| written.retracted_by(room, saved_at)) {
            return Vec::new();
        }
        let (taken, kept): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.uses).into_iter().partition(|(written, ..)| written.retracted_by(room, saved_at));
        self.uses = kept;
        taken.into_iter().map(|(_, item, count)| (item, count)).collect()
    }

    /// `room` rewinds to `saved_at`: take out what its conversations added
    /// after that tick, and return it, so it leaves the bag.
    pub fn retract_room_gains(
        &mut self,
        room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        saved_at: u64,
    ) -> Vec<(ambition_items::Item, u32)> {
        if !self.gains.iter().any(|(written, ..)| written.retracted_by(room, saved_at)) {
            return Vec::new();
        }
        let (taken, kept): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.gains).into_iter().partition(|(written, ..)| written.retracted_by(room, saved_at));
        self.gains = kept;
        taken.into_iter().map(|(_, item, count)| (item, count)).collect()
    }

    /// Entity-free value projection.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_u64};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.uses.len() as u64);
        for (written, item, count) in &self.uses {
            put_u64(&mut bytes, u64::from(written.room.ordinal()));
            put_u64(&mut bytes, written.tick);
            put_u64(&mut bytes, item.index() as u64);
            put_u64(&mut bytes, u64::from(*count));
        }
        put_u64(&mut bytes, self.gains.len() as u64);
        for (written, item, count) in &self.gains {
            put_u64(&mut bytes, u64::from(written.room.ordinal()));
            put_u64(&mut bytes, written.tick);
            put_u64(&mut bytes, item.index() as u64);
            put_u64(&mut bytes, u64::from(*count));
        }
        checksum_bytes(&bytes)
    }
}

/// Use what a menu asked to use. (sim)
///
/// The item is spent and its effect applied on one tick, so the bag and the
/// body cannot disagree after a rewind. A body that holds no Mana keeps its
/// Mana Cell: there is nothing for the cell to restore.
///
/// A body in a room that rewinds alone proposes the use to the committed bag
/// ([`crate::committed_bag`]): the cell and its effect come together on the
/// tick the bag allocates it, so the body never spends a cell another room's
/// rewind can take back. A refused use leaves the bag and the body as they
/// were.
pub fn apply_item_uses(
    world: &mut World,
    requests: &mut bevy::ecs::system::SystemState<ReplayMessageReader<ambition_items::ItemUseRequested>>,
) {
    use ambition_items::Item;
    let Ok(mut reader) = requests.get_mut(world) else { return; };
    let requests: Vec<ambition_items::ItemUseRequested> = reader.read().copied().collect();
    for request in requests {
        let Some(body) = driven_body_in(world, request.participant) else { continue; };
        let item = request.item;
        if !matches!(item, Item::HealthCell | Item::ManaCell) {
            warn!(target: "crate::narrative", "a use of {item:?} was asked for, and it is not a consumable");
            continue;
        }
        match crate::committed_bag::admit_use(world, body, item) {
            crate::committed_bag::BagAdmission::Immediate => {}
            crate::committed_bag::BagAdmission::Held => continue,
            crate::committed_bag::BagAdmission::Refused(why) => {
                warn!(target: "crate::narrative", "a use of {item:?} was refused ({why})");
                continue;
            }
        }
        if crate::committed_bag::cell_takes_effect(world, body, item)
            && world.get_resource_mut::<OwnedItems>().is_some_and(|mut bag| bag.take(item, 1) > 0)
        {
            crate::committed_bag::apply_cell_effect(world, body, item);
        }
    }
}

/// Equip what a menu asked to equip. (sim)
///
/// A body wearing a level of an ability wears the level the bag's count
/// says: a second pickup raises it, and a checkpoint that restores a smaller
/// count lowers it. The count is the one upgrade fact; the worn row follows
/// it, and only a body that wears some level of the ability is touched.
///
/// Every tick, with no change gate: a restore writes the bag and the worn set
/// back together, and a gate on the bag's change would also have to name the
/// worn set's (and a removal). The work is the leveled abilities times the
/// bodies that wear equipment.
pub fn level_worn_abilities(
    owned: Res<OwnedItems>,
    items: ambition_items::ItemCatalogRead,
    mut bodies: Query<&mut ambition_characters::equipment::WornEquipment>,
) {
    let catalog = items.get();
    for item in catalog.leveled() {
        let Some(due) = catalog.wears_at(item, owned.count(item)) else { continue; };
        for mut worn in &mut bodies {
            let wears_a_level = catalog.worn_rows(item).any(|row| worn.wears(&row.id));
            if wears_a_level && !worn.wears(&due.id) {
                worn.equip(due.clone());
            }
        }
    }
}

/// The participant's controlled body wears the ability row. An exclusive
/// slot replaces the variant that held the slot ([`WornEquipment::equip`]),
/// so one variant of a slot is worn at a time. The action scheme and the
/// repertoire read the worn set, so the slot changes with no other write.
///
/// A request for an item that is not owned, or that is no ability with a
/// row, is refused with a log and changes nothing.
///
/// [`WornEquipment::equip`]: ambition_characters::equipment::WornEquipment::equip
pub fn apply_item_equips(
    mut commands: Commands,
    mut requests: ReplayMessageReader<ambition_items::ItemEquipRequested>,
    owned: Res<OwnedItems>,
    items: ambition_items::ItemCatalogRead,
    drivers: Query<(Entity, &DrivingParticipant)>,
    mut bodies: Query<Option<&mut ambition_characters::equipment::WornEquipment>>,
) {
    let mut rows = std::collections::BTreeMap::<PlayerSlot, (Entity, Vec<_>)>::new();
    for request in requests.read() {
        // A held item goes into the hand, which is
        // `ambition_held_items::apply_hand_requests`'s.
        if items.get().held_item_id(request.item).is_some() {
            continue;
        }
        let Some(body) = driven_body(request.participant, &drivers) else { continue; };
        let row = match worn_row_for(items.get(), &owned, request.item) {
            Ok(row) => row,
            Err(code) => {
                warn!(target: "crate::narrative", "an equip of {:?} was refused: {code}", request.item);
                continue;
            }
        };
        rows.entry(request.participant).or_insert_with(|| (body, Vec::new())).1.push(row);
    }
    for (_, (body, rows)) in rows {
        let Ok(worn) = bodies.get_mut(body) else { continue; };
        match worn {
            Some(mut worn) => rows.into_iter().for_each(|row| worn.equip(row)),
            None => {
                let mut fresh = ambition_characters::equipment::WornEquipment::default();
                rows.into_iter().for_each(|row| fresh.equip(row));
                commands.entity(body).insert(fresh);
            }
        }
    }
}

/// The row an owned ability is worn as, at the bag's count (its level,
/// `ItemMeta::levels`), or why it is not: `not_worn` (no ability with a
/// row) or `not_owned`. The menu's stamped equip and a script's `equip` ask
/// this one rule.
pub fn worn_row_for(
    catalog: &ambition_items::ItemCatalog,
    owned: &OwnedItems,
    item: ambition_items::Item,
) -> Result<ambition_characters::equipment::EquipmentRow, &'static str> {
    let row = catalog.wears_at(item, owned.count(item)).ok_or("not_worn")?;
    if !owned.has(item) {
        return Err("not_owned");
    }
    Ok(row.clone())
}

/// `body` wears the owned ability `item`, as [`apply_item_equips`] wears a
/// stamped one: an exclusive slot replaces the variant that held it.
pub fn wear_item(
    In((body, item)): In<(Entity, ambition_items::Item)>,
    mut commands: Commands,
    owned: Res<OwnedItems>,
    items: ambition_items::ItemCatalogRead,
    mut bodies: Query<Option<&mut ambition_characters::equipment::WornEquipment>>,
) -> Result<(), &'static str> {
    let row = worn_row_for(items.get(), &owned, item)?;
    match bodies.get_mut(body).map_err(|_| "no_such_body")? {
        Some(mut worn) => worn.equip(row),
        None => {
            let mut fresh = ambition_characters::equipment::WornEquipment::default();
            fresh.equip(row);
            commands.entity(body).insert(fresh);
        }
    }
    Ok(())
}

fn driven_body_in(world: &mut World, participant: PlayerSlot) -> Option<Entity> {
    let mut drivers = world.query::<(Entity, &DrivingParticipant)>();
    let mut candidates = drivers.iter(world).filter(|(_, driver)| driver.0 == participant);
    let (body, _) = candidates.next()?;
    if candidates.next().is_some() {
        warn!("item request refused: multiple bodies drive participant {participant:?}");
        return None;
    }
    Some(body)
}

/// Who pays for a shop transaction: the body driving the conversation's
/// seat, or with no seat the one primary player. One rule for the purchase
/// (`apply_shop_transactions`) and every question asked before it
/// ([`shop_payer`]), so an offer and its purchase ask one purse. Two bodies
/// driving one seat, or two primaries, name no payer.
pub fn payer_among(
    participant: Option<PlayerSlot>,
    drivers: impl IntoIterator<Item = (Entity, PlayerSlot)>,
    primary: impl IntoIterator<Item = Entity>,
) -> Option<Entity> {
    let mut candidates: Box<dyn Iterator<Item = Entity>> = match participant {
        Some(participant) => Box::new(drivers.into_iter().filter(move |(_, seat)| *seat == participant).map(|(body, _)| body)),
        None => Box::new(primary.into_iter()),
    };
    let payer = candidates.next()?;
    if candidates.next().is_some() {
        warn!("shop payer refused: more than one body answers for {participant:?}");
        return None;
    }
    Some(payer)
}

/// [`payer_among`] read off a world, for a question asked before the purchase.
pub fn shop_payer(world: &World, participant: Option<PlayerSlot>) -> Option<Entity> {
    let drivers: Vec<(Entity, PlayerSlot)> = world
        .try_query::<(Entity, &DrivingParticipant)>()
        .map(|mut query| query.iter(world).map(|(body, driver)| (body, driver.0)).collect())
        .unwrap_or_default();
    let primary: Vec<Entity> = world
        .try_query_filtered::<Entity, (With<BodyWallet>, With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>)>()
        .map(|mut query| query.iter(world).collect())
        .unwrap_or_default();
    payer_among(participant, drivers, primary)
}

/// Would `payer` succeed at this shop deal now (A7)? The merchant's offer
/// asks the deal's own exchange against the payer's purse and the bag, so a
/// full stack, an item the bag does not hold and a payout the purse cannot
/// take refuse the offer exactly as they would refuse the deal. Reads only:
/// the purchase evaluates its own current state again when it runs.
pub fn shop_offer(
    world: &World,
    payer: Entity,
    side: ambition_items::shop::ShopSide,
    item: ambition_items::Item,
    price: i32,
) -> Result<(), ambition_items::transaction::ExchangeRefusal> {
    let wallet = world
        .get::<BodyWallet>(payer)
        .ok_or(ambition_items::transaction::ExchangeRefusal::MissingWallet)?;
    let deal = ShopTransactionRequested { item, price, side, participant: None };
    deal.check(wallet, world.resource::<OwnedItems>(), ambition_items::item_catalog(world))
}

fn driven_body(participant: PlayerSlot, drivers: &Query<(Entity, &DrivingParticipant)>) -> Option<Entity> {
    let mut candidates = drivers.iter().filter(|(_, driver)| driver.0 == participant);
    let (body, _) = candidates.next()?;
    if candidates.next().is_some() {
        warn!("item request refused: multiple bodies drive participant {participant:?}");
        return None;
    }
    Some(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_items::{shop::ShopSide, Item};

    /// A grant reaches the bag through the simulation, which is the property
    /// the direct mutation could not have: this system runs inside the sim
    /// schedule, so a resimulated tick that is handed the request again produces
    /// the same bag.
    #[test]
    fn a_granted_item_lands_in_the_bag() {
        let mut app = App::new();
        app.insert_resource(ambition_items::test_item_catalog().clone());
        app.init_resource::<OwnedItems>();
        app.add_message::<ItemGrantRequested>();
        app.add_systems(Update, apply_item_grants);

        app.world_mut().write_message(ItemGrantRequested {
            participant: None,
            item: Item::HealthCell,
            count: 2,
        });
        app.update();
        assert_eq!(
            app.world().resource::<OwnedItems>().count(Item::HealthCell),
            2
        );
    }

    /// A purchase debits and grants, and an unaffordable one does neither.
    /// The affordability rule is the shop's exchange and is tested there; what this
    /// pins is that the applier does not lose the answer.
    #[test]
    fn a_purchase_moves_money_and_goods_together() {
        let mut app = App::new();
        app.insert_resource(ambition_items::test_item_catalog().clone());
        app.init_resource::<OwnedItems>();
        app.add_message::<ShopTransactionRequested>();
        app.add_systems(Update, apply_shop_transactions);
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
            BodyWallet { balance: 30 },
        ));

        app.world_mut().write_message(ShopTransactionRequested {
            participant: None,
            item: Item::HealthCell,
            price: 10,
            side: ShopSide::Buy,
        });
        app.update();
        assert_eq!(
            app.world().resource::<OwnedItems>().count(Item::HealthCell),
            1
        );
        let balance = app
            .world_mut()
            .query::<&BodyWallet>()
            .iter(app.world())
            .next()
            .map(|wallet| wallet.balance);
        assert_eq!(balance, Some(20));

        // the poison: a price the player cannot meet must move neither.
        app.world_mut().write_message(ShopTransactionRequested {
            participant: None,
            item: Item::HealthCell,
            price: 5_000,
            side: ShopSide::Buy,
        });
        app.update();
        assert_eq!(
            app.world().resource::<OwnedItems>().count(Item::HealthCell),
            1,
            "an unaffordable purchase handed over the goods anyway"
        );
    }

    /// A sale in Bob's conversation pays Bob's wallet, not Player 1's; and
    /// with the rooms logged, the bag's changes are dated in his room, so a
    /// rewind of it takes back the purchase and puts back the sold item
    /// (ISL-2d).
    #[test]
    fn a_merchant_deals_with_the_seat_whose_conversation_it_is() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        let (alices, bobs) = (LiveRoomInstance::from_ordinal(1), LiveRoomInstance::from_ordinal(2));
        let mut app = App::new();
        app.insert_resource(ambition_items::test_item_catalog().clone());
        app.insert_resource(ambition_time::SimTick(7));
        let mut owned = OwnedItems::default();
        owned.grant(ambition_items::test_item_catalog(), Item::ManaCell, 1);
        app.insert_resource(owned);
        app.init_resource::<BagUsesInRooms>();
        app.add_message::<ShopTransactionRequested>();
        app.add_systems(Update, apply_shop_transactions);
        let alice = app
            .world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
                BodyWallet { balance: 30 },
                DrivingParticipant(PlayerSlot(0)),
                InRoomInstance(alices),
            ))
            .id();
        let bob = app
            .world_mut()
            .spawn((BodyWallet { balance: 30 }, DrivingParticipant(PlayerSlot(1)), InRoomInstance(bobs)))
            .id();
        // Two prices, so the two sides cannot cancel on the wrong wallet.
        for (item, price, side) in [(Item::HealthCell, 10, ShopSide::Buy), (Item::ManaCell, 4, ShopSide::Sell)] {
            app.world_mut().write_message(ShopTransactionRequested {
                participant: Some(PlayerSlot(1)),
                item,
                price,
                side,
            });
        }
        app.update();
        let balance = |app: &App, body| app.world().get::<BodyWallet>(body).map(|wallet| wallet.balance);
        assert_eq!(balance(&app, alice), Some(30), "Player 1 paid for Bob's purchase");
        assert_eq!(balance(&app, bob), Some(24), "Bob's wallet did not pay 10 and take 4");
        let bag = |app: &App| {
            let owned = app.world().resource::<OwnedItems>();
            (owned.count(Item::HealthCell), owned.count(Item::ManaCell))
        };
        assert_eq!(bag(&app), (1, 0), "premise: the purchase and the sale moved the bag");

        // A7: the offer asks the deal's own exchange of the same payer. The
        // sold cell is gone from the bag, so its sale is no longer offered,
        // whoever asks; Bob's 24 coins meet a 24-coin price and not 25.
        use ambition_items::transaction::ExchangeRefusal;
        let world = app.world();
        assert_eq!(shop_offer(world, bob, ShopSide::Sell, Item::ManaCell, 4), Err(ExchangeRefusal::InsufficientItems(Item::ManaCell)));
        assert_eq!(shop_offer(world, bob, ShopSide::Buy, Item::HealthCell, 24), Ok(()));
        assert_eq!(shop_offer(world, bob, ShopSide::Buy, Item::HealthCell, 25), Err(ExchangeRefusal::InsufficientFunds));
        assert_eq!(shop_offer(world, alice, ShopSide::Buy, Item::HealthCell, 25), Ok(()), "Alice's own purse answers her offer");
        assert_eq!(app.world().get::<BodyWallet>(bob).map(|wallet| wallet.balance), Some(24), "an offer changed the purse");

        let mut log = app.world_mut().resource_mut::<BagUsesInRooms>();
        assert!(log.retract_room_spends(alices, 6).is_empty() && log.retract_room_gains(alices, 6).is_empty(), "Alice's room took Bob's deal back");
        assert_eq!(log.retract_room_spends(bobs, 6), [(Item::ManaCell, 1)], "Bob's room did not put back the item he sold");
        assert_eq!(log.retract_room_gains(bobs, 6), [(Item::HealthCell, 1)], "Bob's room did not take back what he bought");
    }

    #[test]
    fn an_equipped_ability_reaches_either_participants_body() {
        use ambition_characters::equipment::WornEquipment;
        for participant in [PlayerSlot(0), PlayerSlot(1)] {
            let catalog = ambition_items::test_item_catalog();
            let mut app = App::new();
            app.insert_resource(catalog.clone());
            let mut owned = OwnedItems::default();
            owned.grant(catalog, Item::EyeLaser, 1);
            owned.grant(catalog, Item::Fireball, 1);
            app.insert_resource(owned);
            app.add_message::<ambition_items::ItemEquipRequested>();
            app.add_systems(Update, apply_item_equips);
            let body = app.world_mut().spawn(DrivingParticipant(participant)).id();
            let other = app.world_mut().spawn(DrivingParticipant(PlayerSlot(1 - participant.0))).id();
            let worn_ids = |app: &App, entity| {
                app.world().get::<WornEquipment>(entity)
                    .map(|worn| worn.rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>())
            };
            for item in [Item::EyeLaser, Item::Fireball] {
                app.world_mut().write_message(ambition_items::ItemEquipRequested { participant, item });
            }
            app.update();
            assert_eq!(worn_ids(&app, body), Some(vec!["fireball".into()]), "same-tick requests must retain their order");
            assert_eq!(worn_ids(&app, other), None);
            app.world_mut().write_message(ambition_items::ItemEquipRequested { participant, item: Item::SwordStance });
            app.update();
            assert_eq!(worn_ids(&app, body), Some(vec!["fireball".into()]), "an unowned variant must be refused");
        }
    }

    /// ABILITY-VARIANTS' upgrade fact: the bag's count of an ability is its
    /// level. Equipped at one, the body wears level 1; a second pickup raises
    /// the worn row to level 2 with no new equip; a restore of the count to
    /// one lowers it again. A body that wears no level of the ability is not
    /// touched.
    #[test]
    fn the_bags_count_of_an_ability_is_the_level_its_body_wears() {
        use ambition_characters::equipment::{EquipmentRow, WornEquipment};
        let shipped = ambition_items::test_item_catalog();
        let rows: Vec<_> = Item::ALL
            .into_iter()
            .map(|item| {
                let mut meta = shipped.meta(item).cloned().expect("the test catalog names every slot");
                if item == Item::PrecisionBlink {
                    let row = |id: &str| EquipmentRow {
                        exclusive_slot: Some("blink".into()),
                        id: id.into(),
                        modifiers: Vec::new(),
                        grants: Vec::new(),
                        on_hit: None,
                        withholds: Vec::new(),
                        lends: Vec::new(),
                        rebinds: Default::default(),
                    };
                    meta.wears = Some(row("blink_1"));
                    meta.levels = vec![row("blink_2"), row("blink_3")];
                    meta.stack = 3;
                }
                meta
            })
            .collect();
        let catalog = ambition_items::ItemCatalog::from_rows(rows);
        let mut app = App::new();
        app.insert_resource(catalog.clone());
        let mut owned = OwnedItems::default();
        owned.grant(&catalog, Item::PrecisionBlink, 1);
        app.insert_resource(owned);
        app.add_message::<ambition_items::ItemEquipRequested>();
        app.add_systems(Update, (apply_item_equips, level_worn_abilities).chain());
        let body = app.world_mut().spawn(DrivingParticipant(PlayerSlot(0))).id();
        let bystander = app.world_mut().spawn(WornEquipment::default()).id();
        let worn = |app: &App, entity| {
            app.world().get::<WornEquipment>(entity).map(|worn| worn.rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>())
        };

        app.world_mut().write_message(ambition_items::ItemEquipRequested { participant: PlayerSlot(0), item: Item::PrecisionBlink });
        app.update();
        assert_eq!(worn(&app, body), Some(vec!["blink_1".into()]), "one in the bag is level 1");

        app.world_mut().resource_mut::<OwnedItems>().grant(&catalog, Item::PrecisionBlink, 1);
        app.update();
        assert_eq!(worn(&app, body), Some(vec!["blink_2".into()]), "a second pickup raised the worn level");

        app.world_mut().resource_mut::<OwnedItems>().take(Item::PrecisionBlink, 1);
        app.update();
        assert_eq!(worn(&app, body), Some(vec!["blink_1".into()]), "a count restored to one lowered it");
        assert_eq!(worn(&app, bystander), Some(Vec::new()), "a body that wears no level is not touched");
    }

    #[test]
    fn cells_restore_only_the_requesting_participants_body() {
        use ambition_characters::actor::{BodyHealth, Health};
        use ambition_platformer2d_core::resources::ActorResources;
        use ambition_abilities::mana::{bank, MANA};
        for participant in [PlayerSlot(0), PlayerSlot(1)] {
            let catalog = ambition_items::test_item_catalog();
            let mut app = App::new();
            let mut owned = OwnedItems::default();
            owned.grant(catalog, Item::HealthCell, 2);
            owned.grant(catalog, Item::ManaCell, 2);
            app.insert_resource(owned);
            app.add_message::<ambition_items::ItemUseRequested>();
            app.add_message::<ambition_damage::healing::BodyHealRequested>();
            app.add_systems(Update, (apply_item_uses, ambition_damage::healing::apply_body_heal_requests).chain());
            let mut bodies = Vec::new();
            for slot in 0..2 {
                let mut mana = bank();
                mana.level_of_mut(&MANA).unwrap().drain(80.0);
                bodies.push(app.world_mut().spawn((DrivingParticipant(PlayerSlot(slot)),
                    BodyHealth::new(Health { current: 1, max: 10, invulnerable: Default::default() }), mana)).id());
            }
            for item in [Item::HealthCell, Item::ManaCell] {
                app.world_mut().write_message(ambition_items::ItemUseRequested { participant, item });
            }
            app.update();
            for (slot, &body) in bodies.iter().enumerate() {
                let requested = slot == participant.0 as usize;
                assert_eq!(app.world().get::<BodyHealth>(body).unwrap().current(), if requested { 5 } else { 1 });
                assert_eq!(app.world().get::<ActorResources>(body).unwrap().level_of(&MANA).unwrap().current,
                    if requested { 60.0 } else { 20.0 });
            }
            assert_eq!(app.world().resource::<OwnedItems>().count(Item::HealthCell), 1);
            assert_eq!(app.world().resource::<OwnedItems>().count(Item::ManaCell), 1);
            app.world_mut().entity_mut(bodies[participant.0 as usize]).remove::<DrivingParticipant>();
            for item in [Item::HealthCell, Item::ManaCell] {
                app.world_mut().write_message(ambition_items::ItemUseRequested { participant, item });
            }
            app.update();
            assert_eq!(app.world().resource::<OwnedItems>().count(Item::HealthCell), 1, "an absent body must not spend inventory");
            assert_eq!(app.world().resource::<OwnedItems>().count(Item::ManaCell), 1);
        }
    }
}
