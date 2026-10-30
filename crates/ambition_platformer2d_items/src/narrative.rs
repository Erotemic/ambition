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
use ambition_items::{shop::ShopTransactionRequested, ItemGrantRequested, OwnedItems};

/// Grant what a conversation gave. (sim)
pub fn apply_item_grants(
    mut requests: MessageReader<ItemGrantRequested>,
    mut owned: ResMut<OwnedItems>,
    items: ambition_items::ItemCatalogRead,
) {
    for request in requests.read() {
        owned.grant(items.get(), request.item, request.count);
        info!(
            target: "crate::narrative",
            "give_item: granted {}x {:?}", request.count, request.item,
        );
    }
}

/// Run what a merchant conversation agreed to. (sim)
///
/// The wallet is the PRIMARY player's, which is what a merchant node means: a
/// price quoted in a text box is quoted to the person reading it.
pub fn apply_shop_transactions(
    mut requests: MessageReader<ShopTransactionRequested>,
    mut owned: ResMut<OwnedItems>,
    mut wallets: Query<
        &mut BodyWallet,
        With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
    >,
    items: ambition_items::ItemCatalogRead,
) {
    let Ok(mut wallet) = wallets.single_mut() else {
        return;
    };
    for request in requests.read() {
        let outcome = request.apply(&mut wallet, &mut owned, items.get());
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

/// Use what a menu asked to use. (sim)
///
/// The item is spent and its effect applied on one tick, so the bag and the
/// body cannot disagree after a rewind. A body that holds no Mana keeps its
/// Mana Cell: there is nothing for the cell to restore.
pub fn apply_item_uses(
    mut requests: MessageReader<ambition_items::ItemUseRequested>,
    mut owned: ResMut<OwnedItems>,
    mut heals: MessageWriter<ambition_damage::healing::PlayerHealRequested>,
    mut banks: Query<
        &mut ambition_platformer2d_core::resources::ActorResources,
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
) {
    use ambition_items::Item;
    for request in requests.read() {
        match request.item {
            Item::HealthCell => {
                if owned.take(Item::HealthCell, 1) > 0 {
                    heals.write(ambition_damage::healing::PlayerHealRequested::new(HEALTH_CELL_HEAL));
                }
            }
            Item::ManaCell => {
                let Ok(mut bank) = banks.single_mut() else {
                    continue;
                };
                if let Some(mana) = bank.level_of_mut(&ambition_abilities::mana::MANA) {
                    if owned.take(Item::ManaCell, 1) > 0 {
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
/// The primary player wears the ability's row. A row that names an exclusive
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
    mut bodies: Query<
        (Entity, Option<&mut ambition_characters::equipment::WornEquipment>),
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
) {
    let mut rows = Vec::new();
    for request in requests.read() {
        let Some(row) = items.get().wears(request.item) else {
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
        rows.push(row.clone());
    }
    let Ok((body, worn)) = bodies.single_mut() else {
        return;
    };
    match worn {
        Some(mut worn) => rows.into_iter().for_each(|row| worn.equip(row)),
        None if !rows.is_empty() => {
            let mut fresh = ambition_characters::equipment::WornEquipment::default();
            rows.into_iter().for_each(|row| fresh.equip(row));
            commands.entity(body).insert(fresh);
        }
        None => {}
    }
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

    /// The menu's equip reaches the body through the simulation. Equipping the
    /// laser takes the projectile slot from the fireball; equipping the
    /// fireball takes it back; a variant the bag does not hold is refused.
    #[test]
    fn an_equipped_ability_takes_its_slot_on_the_primary_body() {
        use ambition_characters::equipment::WornEquipment;
        use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
        let catalog = ambition_items::test_item_catalog();
        let mut app = App::new();
        app.insert_resource(catalog.clone());
        let mut owned = OwnedItems::default();
        owned.grant(catalog, Item::EyeLaser, 1);
        owned.grant(catalog, Item::Fireball, 1);
        app.insert_resource(owned);
        app.add_message::<ambition_items::ItemEquipRequested>();
        app.add_systems(Update, apply_item_equips);
        let body = app.world_mut().spawn((PlayerEntity, PrimaryPlayer)).id();
        // A second seat is not the menu's body.
        let other = app.world_mut().spawn(PlayerEntity).id();
        let worn_ids = |app: &App, entity| {
            app.world()
                .get::<WornEquipment>(entity)
                .map(|worn| worn.rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>())
        };

        app.world_mut().write_message(ambition_items::ItemEquipRequested { item: Item::EyeLaser });
        app.update();
        assert_eq!(worn_ids(&app, body), Some(vec!["eye_laser".to_string()]));
        assert_eq!(worn_ids(&app, other), None, "another seat wears nothing");

        app.world_mut().write_message(ambition_items::ItemEquipRequested { item: Item::Fireball });
        app.update();
        assert_eq!(worn_ids(&app, body), Some(vec!["fireball".to_string()]), "one variant per slot");

        app.world_mut().write_message(ambition_items::ItemEquipRequested { item: Item::SwordStance });
        app.update();
        assert_eq!(worn_ids(&app, body), Some(vec!["fireball".to_string()]), "an unowned variant is refused");
    }
}
