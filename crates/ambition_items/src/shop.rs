//! Merchant economy primitives: buy/sell transactions over the player's
//! [`BodyWallet`] and the 24-item [`OwnedItems`] catalog.
//!
//! Kept ECS-free and pure so they're unit-testable; the Yarn `<<buy_item>>` /
//! `<<sell_item>>` commands ASK for one through [`ShopTransactionRequested`] and
//! a simulation system runs it, which is the design-intended shape for shops in
//! Ambition (the `merchant_seed` node + Vault Keeper dialogue call for "a
//! dialogue node with inventory, prices, requirements, consequences"). A bespoke
//! shop overlay UI can later ask for the same transaction.

use crate::transaction::{ExchangeApplied, ExchangeRefusal, InventoryExchange, ItemAmount};
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

/// The one reading of an authored price, shared by the question and the action.
///
/// `cmd_buy_item` and `wallet.can_afford` both use this. Two readings disagree
/// like this, and a guard that refuses what the action then does is worse
/// than no guard:
///
/// ```text
///   balance 25, authored 25.7   guard says NO,  buy_item charges 25 and SUCCEEDS
///   authored -5                 guard says NO,  buy_item charges  0 and SUCCEEDS
/// ```
///
/// Strict, not clamping. Every shipped price is a non-negative integer, so
/// clamping would only hide an authoring mistake such as `-5` becoming free.
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
    /// The seat whose conversation agreed to it (its input owner): its body's
    /// wallet pays or is paid, and the bag change is that body's room's act
    /// (ISL-2d). `None` is the primary seat's, a scripted conversation's.
    pub participant: Option<ambition_characters::control::PlayerSlot>,
}

impl ShopTransactionRequested {
    /// This transaction as exchange terms (A7): a purchase pays `price` and
    /// receives one `item`; a sale gives one `item` and receives `price`. The
    /// item owner's [`InventoryExchange`] is the whole law, so the offer's
    /// question ([`Self::check`]) and the accepted deal ([`Self::apply`]) are
    /// one calculation. A negative price is refused here, at the shared price
    /// boundary: coins do not go below zero.
    pub fn exchange(&self) -> Result<InventoryExchange, ExchangeRefusal> {
        shop_exchange(self.side, self.item, self.price)
    }

    /// Whether this transaction would succeed for `wallet` against `owned`,
    /// changing neither.
    pub fn check(
        &self,
        wallet: &BodyWallet,
        owned: &OwnedItems,
        catalog: &crate::ItemCatalog,
    ) -> Result<(), ExchangeRefusal> {
        self.exchange()?.check(catalog, owned, Some(wallet))
    }

    /// Run this transaction against a wallet and a bag: both change, or
    /// neither does.
    pub fn apply(
        &self,
        wallet: &mut BodyWallet,
        owned: &mut OwnedItems,
        catalog: &crate::ItemCatalog,
    ) -> Result<ExchangeApplied, ExchangeRefusal> {
        self.exchange()?.apply(catalog, owned, Some(wallet))
    }
}

/// The exchange terms of one shop deal: `side` of one `item` at `price`.
pub fn shop_exchange(side: ShopSide, item: Item, price: i32) -> Result<InventoryExchange, ExchangeRefusal> {
    let price = u32::try_from(price).map_err(|_| ExchangeRefusal::InvalidQuantity)?;
    let one = vec![ItemAmount { item, count: 1 }];
    Ok(match side {
        ShopSide::Buy => InventoryExchange { grants: one, pay_coins: price, ..Default::default() },
        ShopSide::Sell => InventoryExchange { costs: one, receive_coins: price, ..Default::default() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> &'static crate::ItemCatalog {
        crate::test_item_catalog()
    }

    fn deal(side: ShopSide, item: Item, price: i32) -> ShopTransactionRequested {
        ShopTransactionRequested { item, price, side, participant: None }
    }

    #[test]
    fn buying_an_affordable_item_debits_and_grants() {
        let mut wallet = BodyWallet { balance: 30 };
        let mut owned = OwnedItems::default();
        assert!(deal(ShopSide::Buy, Item::GunSword, 25).apply(&mut wallet, &mut owned, shipped()).is_ok());
        assert_eq!(wallet.balance, 5);
        assert!(owned.has(Item::GunSword));
    }

    #[test]
    fn buying_without_enough_money_changes_nothing() {
        let mut wallet = BodyWallet { balance: 10 };
        let mut owned = OwnedItems::default();
        assert_eq!(
            deal(ShopSide::Buy, Item::GunSword, 25).apply(&mut wallet, &mut owned, shipped()),
            Err(ExchangeRefusal::InsufficientFunds)
        );
        assert_eq!(wallet.balance, 10, "wallet untouched on a failed buy");
        assert!(!owned.has(Item::GunSword), "no item granted on a failed buy");
    }

    #[test]
    fn consumables_stack_when_bought_repeatedly() {
        let mut wallet = BodyWallet { balance: 100 };
        let mut owned = OwnedItems::default();
        let buy = deal(ShopSide::Buy, Item::HealthCell, 8);
        assert!(buy.apply(&mut wallet, &mut owned, shipped()).is_ok());
        assert!(buy.apply(&mut wallet, &mut owned, shipped()).is_ok());
        assert_eq!(owned.count(Item::HealthCell), 2);
        assert_eq!(wallet.balance, 84);
    }

    #[test]
    fn selling_an_owned_item_credits_and_removes() {
        let mut wallet = BodyWallet { balance: 0 };
        let mut owned = OwnedItems::default();
        owned.grant(shipped(), Item::HealthCell, 2);
        assert!(deal(ShopSide::Sell, Item::HealthCell, 4).apply(&mut wallet, &mut owned, shipped()).is_ok());
        assert_eq!(wallet.balance, 4);
        assert_eq!(owned.count(Item::HealthCell), 1);
    }

    #[test]
    fn selling_what_you_dont_have_is_rejected() {
        let mut wallet = BodyWallet { balance: 7 };
        let mut owned = OwnedItems::default();
        assert_eq!(
            deal(ShopSide::Sell, Item::GunSword, 12).apply(&mut wallet, &mut owned, shipped()),
            Err(ExchangeRefusal::InsufficientItems(Item::GunSword))
        );
        assert_eq!(wallet.balance, 7, "wallet untouched on a failed sell");
    }

    #[test]
    fn buy_then_sell_round_trips_ownership() {
        let mut wallet = BodyWallet { balance: 25 };
        let mut owned = OwnedItems::default();
        assert!(deal(ShopSide::Buy, Item::GunSword, 25).apply(&mut wallet, &mut owned, shipped()).is_ok());
        assert_eq!(wallet.balance, 0);
        assert!(deal(ShopSide::Sell, Item::GunSword, 12).apply(&mut wallet, &mut owned, shipped()).is_ok());
        assert_eq!(wallet.balance, 12);
        assert!(!owned.has(Item::GunSword));
    }

    #[test]
    fn re_buying_an_owned_unique_is_refused_without_spending() {
        let mut wallet = BodyWallet { balance: 100 };
        let mut owned = OwnedItems::default();
        owned.grant(shipped(), Item::Grapple, 1); // a held gauntlet — unique
        assert_eq!(
            deal(ShopSide::Buy, Item::Grapple, 45).apply(&mut wallet, &mut owned, shipped()),
            Err(ExchangeRefusal::StackFull(Item::Grapple)),
            "can't re-buy a unique you own"
        );
        assert_eq!(wallet.balance, 100, "wallet untouched");
        assert_eq!(owned.count(Item::Grapple), 1, "still just one");
    }

    /// ⭐ A7: the offer's question is the deal's own calculation. A full
    /// stack refuses the offer although the wallet could pay; a payout past
    /// the wallet refuses the sale whole (no partial credit, the cell kept);
    /// a negative price is refused, not clamped to a free deal. A check
    /// changes nothing, and an acceptance asks the state it meets again.
    #[test]
    fn the_offer_and_the_deal_are_one_exchange() {
        let catalog = shipped();
        let mut owned = OwnedItems::default();
        let stack = catalog.stack(Item::HealthCell).expect("a cell stacks to a limit");
        owned.grant(catalog, Item::HealthCell, stack);
        let mut wallet = BodyWallet { balance: 100 };
        let buy = deal(ShopSide::Buy, Item::HealthCell, 8);
        assert_eq!(buy.check(&wallet, &owned, catalog), Err(ExchangeRefusal::StackFull(Item::HealthCell)));
        assert_eq!(buy.apply(&mut wallet, &mut owned, catalog), Err(ExchangeRefusal::StackFull(Item::HealthCell)));
        assert_eq!((wallet.balance, owned.count(Item::HealthCell)), (100, stack));

        let mut rich = BodyWallet { balance: i32::MAX - 2 };
        let sell = deal(ShopSide::Sell, Item::HealthCell, 4);
        assert_eq!(sell.apply(&mut rich, &mut owned, catalog), Err(ExchangeRefusal::WalletOverflow));
        assert_eq!((rich.balance, owned.count(Item::HealthCell)), (i32::MAX - 2, stack));

        assert_eq!(deal(ShopSide::Sell, Item::HealthCell, -1).check(&wallet, &owned, catalog), Err(ExchangeRefusal::InvalidQuantity));

        // The preview reserves nothing: a cell sold between the offer and the
        // purchase makes room, and the purchase meets that state.
        assert!(sell.apply(&mut wallet, &mut owned, catalog).is_ok());
        assert!(buy.check(&wallet, &owned, catalog).is_ok());
        assert!(buy.apply(&mut wallet, &mut owned, catalog).is_ok());
        assert_eq!((wallet.balance, owned.count(Item::HealthCell)), (96, stack));
    }
}
