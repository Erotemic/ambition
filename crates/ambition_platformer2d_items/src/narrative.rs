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
//! the pure cores did not move and did not change. `shop::buy`,
//! `shop::sell` and `OwnedItems::grant` are still the whole rule and still
//! unit-tested without a `World`; what changed is who calls them and when.

use bevy::prelude::*;

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
/// Dated in the room of the seat whose conversation gave it, so a rewind of
/// that room takes the grant back (ISL-2d) and its replay gives it again.
pub fn apply_item_grants(
    mut requests: MessageReader<ItemGrantRequested>,
    mut owned: ResMut<OwnedItems>,
    items: ambition_items::ItemCatalogRead,
    (drivers, rooms, tick, mut log, mut since): (
        Query<(Entity, &DrivingParticipant)>,
        Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        Option<Res<ambition_time::SimTick>>,
        Option<ResMut<BagUsesInRooms>>,
        Option<ResMut<crate::pickup::ConversationBagSinceCheckpoint>>,
    ),
) {
    for request in requests.read() {
        let before = owned.count(request.item);
        owned.grant(items.get(), request.item, request.count);
        let gained = owned.count(request.item) - before;
        let written = narrative_write(request.participant, &drivers, &rooms, tick.as_deref());
        if let (Some(written), Some(log)) = (written, log.as_deref_mut()) {
            log.record_gain(written, request.item, gained);
        }
        if let Some(since) = since.as_deref_mut() {
            since.record(crate::pickup::ConversationBagMove {
                item: request.item,
                gained,
                gave: 0,
                payer: None,
                coins: 0,
                written,
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
    mut requests: MessageReader<ShopTransactionRequested>,
    mut owned: ResMut<OwnedItems>,
    mut wallets: Query<&mut BodyWallet>,
    primary: Query<Entity, (With<BodyWallet>, With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>)>,
    items: ambition_items::ItemCatalogRead,
    (drivers, rooms, tick, mut log, mut since, ids): (
        Query<(Entity, &DrivingParticipant)>,
        Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        Option<Res<ambition_time::SimTick>>,
        Option<ResMut<BagUsesInRooms>>,
        Option<ResMut<crate::pickup::ConversationBagSinceCheckpoint>>,
        Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    ),
) {
    for request in requests.read() {
        let payer = match request.participant {
            Some(participant) => driven_body(participant, &drivers),
            None => primary.single().ok(),
        };
        let Some(mut wallet) = payer.and_then(|payer| wallets.get_mut(payer).ok()) else {
            continue;
        };
        let (before, balance) = (owned.count(request.item), wallet.balance);
        let outcome = request.apply(&mut wallet, &mut owned, items.get());
        let after = owned.count(request.item);
        let written = narrative_write(request.participant, &drivers, &rooms, tick.as_deref());
        if let Some(since) = since.as_deref_mut() {
            since.record(crate::pickup::ConversationBagMove {
                item: request.item,
                gained: after.saturating_sub(before),
                gave: before.saturating_sub(after),
                payer: payer.and_then(|payer| ids.get(payer).ok()).cloned(),
                coins: wallet.balance - balance,
                written,
            });
        }
        if let (Some(written), Some(log)) = (written, log.as_deref_mut()) {
            if after > before {
                log.record_gain(written, request.item, after - before);
            }
            for _ in after..before {
                log.record(written, request.item);
            }
        }
        info!(
            target: "crate::narrative",
            "{:?} {:?} @ {} -> {outcome:?} (balance now {})",
            request.side, request.item, request.price, wallet.balance,
        );
    }
}

/// How much one Health Cell heals.
pub const HEALTH_CELL_HEAL: i32 = 4;
/// How much Mana one Mana Cell restores.
pub const MANA_CELL_RESTORE: f32 = 40.0;

/// The cells bodies used from the shared bag, with the room each body was in
/// and the tick (ISL-2d): a rewind of that room puts each back in the bag,
/// since the use's effect is on the body, which the room's snapshot rewound,
/// and its replay uses again what it uses again. And what a conversation in
/// a room added to the bag (a grant, a purchase), which the rewind takes
/// back; a sale's item is a use. Kept within the rewind horizon
/// ([`ambition_platformer2d_shared_tangle::lifecycle::ROOM_REWIND_HORIZON_TICKS`]).
///
/// Rollback state with a real value: a use writes it on a tick.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct BagUsesInRooms {
    uses: Vec<(ambition_platformer2d_shared_tangle::lifecycle::RoomWrite, ambition_items::Item)>,
    gains: Vec<(ambition_platformer2d_shared_tangle::lifecycle::RoomWrite, ambition_items::Item, u32)>,
}

impl BagUsesInRooms {
    /// Remember a use of one `item` from a room.
    pub fn record(&mut self, at: ambition_platformer2d_shared_tangle::lifecycle::RoomWrite, item: ambition_items::Item) {
        self.uses.retain(|(written, _)| !written.beyond_the_horizon(at.tick));
        self.uses.push((at, item));
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
    pub fn retract_room(
        &mut self,
        room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        saved_at: u64,
    ) -> Vec<ambition_items::Item> {
        if !self.uses.iter().any(|(written, _)| written.retracted_by(room, saved_at)) {
            return Vec::new();
        }
        let (taken, kept): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.uses).into_iter().partition(|(written, _)| written.retracted_by(room, saved_at));
        self.uses = kept;
        taken.into_iter().map(|(_, item)| item).collect()
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
        for (written, item) in &self.uses {
            put_u64(&mut bytes, u64::from(written.room.ordinal()));
            put_u64(&mut bytes, written.tick);
            put_u64(&mut bytes, item.index() as u64);
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
#[allow(clippy::too_many_arguments)]
pub fn apply_item_uses(
    mut requests: MessageReader<ambition_items::ItemUseRequested>,
    mut owned: ResMut<OwnedItems>,
    mut heals: MessageWriter<ambition_damage::healing::BodyHealRequested>,
    drivers: Query<(Entity, &DrivingParticipant)>,
    health: Query<(), With<ambition_characters::actor::BodyHealth>>,
    mut banks: Query<&mut ambition_platformer2d_core::resources::ActorResources>,
    // Each use, with the user's room and the tick: a rewind of that room
    // puts the item back (ISL-2d).
    (rooms, tick, mut uses): (
        Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        Option<Res<ambition_time::SimTick>>,
        Option<ResMut<BagUsesInRooms>>,
    ),
) {
    use ambition_items::Item;
    for request in requests.read() {
        let Some(body) = driven_body(request.participant, &drivers) else { continue; };
        let mut used = |item: Item| {
            if let (Ok(room), Some(tick), Some(uses)) = (rooms.get(body), tick.as_deref(), uses.as_deref_mut()) {
                uses.record(ambition_platformer2d_shared_tangle::lifecycle::RoomWrite { room: room.0, tick: tick.0 }, item);
            }
        };
        match request.item {
            Item::HealthCell => {
                if health.contains(body) && owned.take(Item::HealthCell, 1) > 0 {
                    used(Item::HealthCell);
                    heals.write(ambition_damage::healing::BodyHealRequested::new(HEALTH_CELL_HEAL, body));
                }
            }
            Item::ManaCell => {
                let Ok(mut bank) = banks.get_mut(body) else {
                    continue;
                };
                if let Some(mana) = bank.level_of_mut(&ambition_abilities::mana::MANA) {
                    if owned.take(Item::ManaCell, 1) > 0 {
                        used(Item::ManaCell);
                        mana.refill(MANA_CELL_RESTORE);
                    }
                }
            }
            other => {
                warn!(
                    target: "crate::narrative",
                    "a use of {other:?} was asked for, and it is not a consumable"
                );
            }
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
    mut requests: MessageReader<ambition_items::ItemEquipRequested>,
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
        // The level is the bag's count (`ItemMeta::levels`).
        let Some(row) = items.get().wears_at(request.item, owned.count(request.item)) else {
            warn!(
                target: "crate::narrative",
                "an equip of {:?} was asked for, and it is no ability with a row", request.item
            );
            continue;
        };
        if !owned.has(request.item) {
            warn!(target: "crate::narrative", "an equip of {:?} was asked for, and it is not owned", request.item);
            continue;
        }
        rows.entry(request.participant).or_insert_with(|| (body, Vec::new())).1.push(row.clone());
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
    /// The affordability rule is `shop::buy`'s and is tested there; what this
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

        let mut log = app.world_mut().resource_mut::<BagUsesInRooms>();
        assert!(log.retract_room(alices, 6).is_empty() && log.retract_room_gains(alices, 6).is_empty(), "Alice's room took Bob's deal back");
        assert_eq!(log.retract_room(bobs, 6), [Item::ManaCell], "Bob's room did not put back the item he sold");
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
