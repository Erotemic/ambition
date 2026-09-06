//! Merchant economy primitives: buy/sell transactions over the player's
//! [`BodyWallet`] and the 24-item [`OwnedItems`] catalog.
//!
//! Kept ECS-free and pure so they're unit-testable; the Yarn `<<buy_item>>` /
//! `<<sell_item>>` commands ASK for one through [`ShopTransactionRequested`] and
//! a simulation system runs it, which is the design-intended shape for shops in
//! Ambition (the `merchant_seed` node + Vault Keeper dialogue call for "a
//! dialogue node with inventory, prices, requirements, consequences"). A bespoke
//! shop overlay UI can later ask for the same transaction.

use crate::{Item, OwnedItems};
use ambition_characters::actor::BodyWallet;
use bevy::prelude::Message;

/// Why an authored price is not a price.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoredPriceProblem {
    /// `NaN` or an infinity — Yarn arithmetic can produce both.
    NotFinite,
    /// Coins do not go below zero.
    Negative,
    /// `25.7` is not a number of coins.
    Fractional,
    /// Beyond what a wallet can hold.
    TooLarge,
}

impl AuthoredPriceProblem {
    /// One clause, for a warning or a `WhyNot`.
    pub fn observed(self) -> &'static str {
        match self {
            Self::NotFinite => "the authored price is not a finite number",
            Self::Negative => "the authored price is negative, and coins do not go below zero",
            Self::Fractional => "the authored price is fractional, and coins are whole",
            Self::TooLarge => "the authored price is larger than a wallet can hold",
        }
    }
}

/// The ONE reading of an authored price, shared by the question and the action.
///
/// ⛔⛔ THESE WERE TWO READINGS FOR HALF A DAY AND THEY DISAGREED. `cmd_buy_item`
/// built its request with `price.max(0.0) as i32`; when `wallet.can_afford` was
/// published (D-WALLET-PREDICATE) it compared the balance against the raw `f64`
/// instead. The migration removed one duplicated authority and created another,
/// on the other side of the same contract:
///
/// ```text
///   balance 25, authored 25.7   guard says NO,  buy_item charges 25 and SUCCEEDS
///   authored -5                 guard says NO,  buy_item charges  0 and SUCCEEDS
/// ```
///
/// ⇒ A guard that refuses what the action then performs is worse than no guard:
/// it reads as protection. Both roads take their price from here now.
///
/// ⭐ STRICT, NOT CLAMPING, and the shipped content pays nothing for it: every
/// authored `buy_item`/`sell_item`/`can_afford` price in the repository is a
/// non-negative integer (measured 2026-09-04 — 0, 4, 6, 8, 12, 17, 25, 30, 35,
/// 40, 45). ⇒ Clamping only ever silently rescued an authoring mistake, and
/// `-5` becoming a free purchase is the shape of mistake it rescued.
pub fn authored_price(price: f64) -> Result<i32, AuthoredPriceProblem> {
    if !price.is_finite() {
        return Err(AuthoredPriceProblem::NotFinite);
    }
    if price < 0.0 {
        return Err(AuthoredPriceProblem::Negative);
    }
    if price.fract() != 0.0 {
        return Err(AuthoredPriceProblem::Fractional);
    }
    if price > f64::from(i32::MAX) {
        return Err(AuthoredPriceProblem::TooLarge);
    }
    Ok(price as i32)
}

/// Which way the goods move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopSide {
    Buy,
    Sell,
}

/// Somebody asked for a transaction; the simulation runs it.
///
/// So the command records the REQUEST in the conversation's narrative ledger and
/// a simulation system applies it on the tick it was stamped for — every replay
/// of that tick included.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct ShopTransactionRequested {
    pub item: Item,
    /// What the merchant is charging (buy) or paying (sell).
    pub price: i32,
    pub side: ShopSide,
}

impl ShopTransactionRequested {
    /// Run this transaction against a wallet and a bag.
    ///
    /// Here rather than at the applier so the two sides cannot drift: a caller
    /// that had to remember which of [`buy`]/[`sell`] matches which
    /// [`ShopSide`] is one `match` away from paying the player for a purchase.
    pub fn apply(&self, wallet: &mut BodyWallet, owned: &mut OwnedItems) -> ShopTx {
        match self.side {
            ShopSide::Buy => buy(wallet, owned, self.item, self.price),
            ShopSide::Sell => sell(wallet, owned, self.item, self.price),
        }
    }
}

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
pub fn buy(wallet: &mut BodyWallet, owned: &mut OwnedItems, item: Item, price: i32) -> ShopTx {
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
pub fn sell(wallet: &mut BodyWallet, owned: &mut OwnedItems, item: Item, price: i32) -> ShopTx {
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
        let mut wallet = BodyWallet { balance: 30 };
        let mut owned = OwnedItems::default();
        assert_eq!(buy(&mut wallet, &mut owned, Item::Axe, 25), ShopTx::Bought);
        assert_eq!(wallet.balance, 5);
        assert!(owned.has(Item::Axe));
    }

    #[test]
    fn buying_without_enough_money_changes_nothing() {
        let mut wallet = BodyWallet { balance: 10 };
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
        let mut wallet = BodyWallet { balance: 100 };
        let mut owned = OwnedItems::default();
        assert!(buy(&mut wallet, &mut owned, Item::HealthCell, 8).succeeded());
        assert!(buy(&mut wallet, &mut owned, Item::HealthCell, 8).succeeded());
        assert_eq!(owned.count(Item::HealthCell), 2);
        assert_eq!(wallet.balance, 84);
    }

    #[test]
    fn selling_an_owned_item_credits_and_removes() {
        let mut wallet = BodyWallet { balance: 0 };
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
        let mut wallet = BodyWallet { balance: 7 };
        let mut owned = OwnedItems::default();
        assert_eq!(
            sell(&mut wallet, &mut owned, Item::Axe, 12),
            ShopTx::NotOwned
        );
        assert_eq!(wallet.balance, 7, "wallet untouched on a failed sell");
    }

    #[test]
    fn buy_then_sell_round_trips_ownership() {
        let mut wallet = BodyWallet { balance: 25 };
        let mut owned = OwnedItems::default();
        assert!(buy(&mut wallet, &mut owned, Item::Axe, 25).succeeded());
        assert_eq!(wallet.balance, 0);
        assert!(sell(&mut wallet, &mut owned, Item::Axe, 12).succeeded());
        assert_eq!(wallet.balance, 12);
        assert!(!owned.has(Item::Axe));
    }

    #[test]
    fn re_buying_an_owned_unique_is_refused_without_spending() {
        let mut wallet = BodyWallet { balance: 100 };
        let mut owned = OwnedItems::default();
        owned.grant(Item::Blink, 1); // an ability — unique
        let tx = buy(&mut wallet, &mut owned, Item::Blink, 45);
        assert_eq!(tx, ShopTx::AlreadyOwned, "can't re-buy a unique you own");
        assert_eq!(wallet.balance, 100, "wallet untouched");
        assert_eq!(owned.count(Item::Blink), 1, "still just one");
    }

    #[test]
    fn non_unique_consumables_still_stack_on_buy() {
        let mut wallet = BodyWallet { balance: 100 };
        let mut owned = OwnedItems::default();
        owned.grant(Item::HealthCell, 1); // consumable — stacks
        assert!(buy(&mut wallet, &mut owned, Item::HealthCell, 8).succeeded());
        assert_eq!(owned.count(Item::HealthCell), 2, "consumables stack");
    }
}
