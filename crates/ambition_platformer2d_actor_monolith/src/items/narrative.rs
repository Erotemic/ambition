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
) {
    for request in requests.read() {
        owned.grant(request.item, request.count);
        info!(
            target: "ambition_platformer2d_actor_monolith::items::narrative",
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
) {
    let Ok(mut wallet) = wallets.single_mut() else {
        return;
    };
    for request in requests.read() {
        let outcome = request.apply(&mut wallet, &mut owned);
        info!(
            target: "ambition_platformer2d_actor_monolith::items::narrative",
            "{:?} {:?} @ {} -> {outcome:?} (balance now {})",
            request.side, request.item, request.price, wallet.balance,
        );
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
}
