//! Merchant economy primitives: buy/sell transactions over the player's
//! [`PlayerWallet`] and the 24-item [`OwnedItems`] catalog.
//!
//! Kept ECS-free and pure so they're unit-testable; the Yarn `<<buy_item>>` /
//! `<<sell_item>>` commands ([`crate::dialog::yarn_bindings`]) wrap these with a
//! player-wallet query, which is the design-intended shape for shops in Ambition
//! (the `merchant_seed` node + Vault Keeper dialogue call for "a dialogue node
//! with inventory, prices, requirements, consequences"). A bespoke shop overlay
//! UI can later wrap the same primitives.

use crate::items::{Item, OwnedItems};
use crate::player::PlayerWallet;

/// Outcome of a buy/sell attempt, for logging + (future) UI feedback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopTx {
    Bought,
    Sold,
    /// Not enough money for the purchase (wallet unchanged).
    CantAfford,
    /// Nothing of that item to sell (wallet unchanged).
    NotOwned,
    /// A unique item the player already owns — buying again would waste coins
    /// (the grant caps at one), so the purchase is refused (wallet unchanged).
    AlreadyOwned,
}

impl ShopTx {
    pub fn succeeded(self) -> bool {
        matches!(self, Self::Bought | Self::Sold)
    }
}

/// Attempt to buy one `item` for `price`: debit the wallet and grant the item
/// only if affordable. A negative price is rejected as unaffordable.
pub fn buy(wallet: &mut PlayerWallet, owned: &mut OwnedItems, item: Item, price: i32) -> ShopTx {
    if price < 0 {
        return ShopTx::CantAfford;
    }
    // A unique item (weapon / ability) the player already owns can't stack — the
    // grant caps at one — so refuse the buy instead of pocketing the coins.
    if item.category().is_unique() && owned.has(item) {
        return ShopTx::AlreadyOwned;
    }
    if wallet.try_spend(price) {
        owned.grant(item, 1);
        ShopTx::Bought
    } else {
        ShopTx::CantAfford
    }
}

/// Attempt to sell one `item` for `price`: remove one from the catalog and
/// credit the wallet only if the player owns at least one. Price is floored at 0.
pub fn sell(wallet: &mut PlayerWallet, owned: &mut OwnedItems, item: Item, price: i32) -> ShopTx {
    if owned.take(item, 1) > 0 {
        wallet.add(price.max(0));
        ShopTx::Sold
    } else {
        ShopTx::NotOwned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buying_an_affordable_item_debits_and_grants() {
        let mut wallet = PlayerWallet { balance: 30 };
        let mut owned = OwnedItems::default();
        assert_eq!(buy(&mut wallet, &mut owned, Item::Axe, 25), ShopTx::Bought);
        assert_eq!(wallet.balance, 5);
        assert!(owned.has(Item::Axe));
    }

    #[test]
    fn buying_without_enough_money_changes_nothing() {
        let mut wallet = PlayerWallet { balance: 10 };
        let mut owned = OwnedItems::default();
        assert_eq!(
            buy(&mut wallet, &mut owned, Item::Axe, 25),
            ShopTx::CantAfford
        );
        assert_eq!(wallet.balance, 10, "wallet untouched on a failed buy");
        assert!(!owned.has(Item::Axe), "no item granted on a failed buy");
    }

    #[test]
    fn consumables_stack_when_bought_repeatedly() {
        let mut wallet = PlayerWallet { balance: 100 };
        let mut owned = OwnedItems::default();
        assert!(buy(&mut wallet, &mut owned, Item::HealthCell, 8).succeeded());
        assert!(buy(&mut wallet, &mut owned, Item::HealthCell, 8).succeeded());
        assert_eq!(owned.count(Item::HealthCell), 2);
        assert_eq!(wallet.balance, 84);
    }

    #[test]
    fn selling_an_owned_item_credits_and_removes() {
        let mut wallet = PlayerWallet { balance: 0 };
        let mut owned = OwnedItems::default();
        owned.grant(Item::HealthCell, 2);
        assert_eq!(
            sell(&mut wallet, &mut owned, Item::HealthCell, 4),
            ShopTx::Sold
        );
        assert_eq!(wallet.balance, 4);
        assert_eq!(owned.count(Item::HealthCell), 1);
    }

    #[test]
    fn selling_what_you_dont_have_is_rejected() {
        let mut wallet = PlayerWallet { balance: 7 };
        let mut owned = OwnedItems::default();
        assert_eq!(
            sell(&mut wallet, &mut owned, Item::Axe, 12),
            ShopTx::NotOwned
        );
        assert_eq!(wallet.balance, 7, "wallet untouched on a failed sell");
    }

    #[test]
    fn buy_then_sell_round_trips_ownership() {
        let mut wallet = PlayerWallet { balance: 25 };
        let mut owned = OwnedItems::default();
        assert!(buy(&mut wallet, &mut owned, Item::Axe, 25).succeeded());
        assert_eq!(wallet.balance, 0);
        assert!(sell(&mut wallet, &mut owned, Item::Axe, 12).succeeded());
        assert_eq!(wallet.balance, 12);
        assert!(!owned.has(Item::Axe));
    }

    #[test]
    fn re_buying_an_owned_unique_is_refused_without_spending() {
        let mut wallet = PlayerWallet { balance: 100 };
        let mut owned = OwnedItems::default();
        owned.grant(Item::Blink, 1); // an ability — unique
        let tx = buy(&mut wallet, &mut owned, Item::Blink, 45);
        assert_eq!(tx, ShopTx::AlreadyOwned, "can't re-buy a unique you own");
        assert_eq!(wallet.balance, 100, "wallet untouched");
        assert_eq!(owned.count(Item::Blink), 1, "still just one");
    }

    #[test]
    fn non_unique_consumables_still_stack_on_buy() {
        let mut wallet = PlayerWallet { balance: 100 };
        let mut owned = OwnedItems::default();
        owned.grant(Item::HealthCell, 1); // consumable — stacks
        assert!(buy(&mut wallet, &mut owned, Item::HealthCell, 8).succeeded());
        assert_eq!(owned.count(Item::HealthCell), 2, "consumables stack");
    }
}
