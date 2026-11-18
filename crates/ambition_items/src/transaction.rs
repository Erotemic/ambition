//! Atomic exchanges over stored item quantities and one body's wallet.

use crate::{ITEM_COUNT, Item, ItemCatalog, OwnedItems};
use ambition_characters::actor::BodyWallet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemAmount {
    pub item: Item,
    pub count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InventoryExchange {
    pub costs: Vec<ItemAmount>,
    pub grants: Vec<ItemAmount>,
    pub pay_coins: u32,
    pub receive_coins: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExchangeRefusal {
    InvalidQuantity,
    UnknownItem(Item),
    InsufficientItems(Item),
    StackFull(Item),
    MissingWallet,
    InsufficientFunds,
    WalletOverflow,
}
impl ExchangeRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidQuantity => "invalid_quantity",
            Self::UnknownItem(_) => "unknown_item",
            Self::InsufficientItems(_) => "insufficient_items",
            Self::StackFull(_) => "stack_full",
            Self::MissingWallet => "missing_wallet",
            Self::InsufficientFunds => "insufficient_funds",
            Self::WalletOverflow => "wallet_overflow",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemChange {
    pub item: Item,
    pub removed: u32,
    pub added: u32,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExchangeApplied {
    pub items: Vec<ItemChange>,
    pub coins: i32,
}

fn totals(
    catalog: &ItemCatalog,
    lines: &[ItemAmount],
) -> Result<[u32; ITEM_COUNT], ExchangeRefusal> {
    let mut counts = [0u32; ITEM_COUNT];
    for line in lines {
        if !catalog.is_real(line.item) {
            return Err(ExchangeRefusal::UnknownItem(line.item));
        }
        if line.count == 0 {
            return Err(ExchangeRefusal::InvalidQuantity);
        }
        let count = &mut counts[line.item.index()];
        *count = count
            .checked_add(line.count)
            .ok_or(ExchangeRefusal::InvalidQuantity)?;
    }
    Ok(counts)
}

impl InventoryExchange {
    /// Costs must exist before grants. Every requested quantity must fit.
    /// Failure leaves both the bag and the wallet unchanged.
    pub fn apply(
        &self,
        catalog: &ItemCatalog,
        bag: &mut OwnedItems,
        mut wallet: Option<&mut BodyWallet>,
    ) -> Result<ExchangeApplied, ExchangeRefusal> {
        let costs = totals(catalog, &self.costs)?;
        let grants = totals(catalog, &self.grants)?;
        let pay = i32::try_from(self.pay_coins).map_err(|_| ExchangeRefusal::InvalidQuantity)?;
        let receive =
            i32::try_from(self.receive_coins).map_err(|_| ExchangeRefusal::InvalidQuantity)?;
        let mut purse = wallet.as_deref().copied();
        if pay > 0 || receive > 0 {
            let purse = purse.as_mut().ok_or(ExchangeRefusal::MissingWallet)?;
            if pay > 0 && !purse.try_spend(pay) {
                return Err(ExchangeRefusal::InsufficientFunds);
            }
            purse
                .balance
                .checked_add(receive)
                .ok_or(ExchangeRefusal::WalletOverflow)?;
            purse.add(receive);
        }
        let mut next = bag.clone();
        let mut applied = ExchangeApplied {
            items: Vec::new(),
            coins: receive - pay,
        };
        for item in Item::ALL {
            let cost = costs[item.index()];
            if next.count(item) < cost {
                return Err(ExchangeRefusal::InsufficientItems(item));
            }
            next.take(item, cost);
        }
        for item in Item::ALL {
            let (removed, added) = (costs[item.index()], grants[item.index()]);
            if added > 0 {
                let before = next.count(item);
                next.grant(catalog, item, added);
                if next.count(item) - before != added {
                    return Err(ExchangeRefusal::StackFull(item));
                }
            }
            if removed > 0 || added > 0 {
                applied.items.push(ItemChange {
                    item,
                    removed,
                    added,
                });
            }
        }
        *bag = next;
        if let (Some(wallet), Some(purse)) = (wallet.as_deref_mut(), purse) {
            *wallet = purse;
        }
        Ok(applied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn catalog() -> &'static ItemCatalog {
        crate::test_item_catalog()
    }
    #[test]
    fn exchange_checks_all_costs_and_capacity_before_changing_either_owner() {
        let mut bag = OwnedItems::default();
        bag.grant(catalog(), Item::HealthCell, 3);
        bag.grant(catalog(), Item::PortalGun, 1);
        let mut wallet = BodyWallet { balance: 9 };
        for refusal in [
            ExchangeRefusal::StackFull(Item::PortalGun),
            ExchangeRefusal::InsufficientItems(Item::HealthCell),
        ] {
            let before = bag.clone();
            let request = InventoryExchange {
                costs: vec![ItemAmount {
                    item: Item::HealthCell,
                    count: if matches!(refusal, ExchangeRefusal::InsufficientItems(_)) {
                        4
                    } else {
                        2
                    },
                }],
                grants: vec![ItemAmount {
                    item: Item::PortalGun,
                    count: 1,
                }],
                pay_coins: 5,
                ..Default::default()
            };
            assert_eq!(
                request.apply(catalog(), &mut bag, Some(&mut wallet)),
                Err(refusal)
            );
            assert_eq!(bag, before);
            assert_eq!(wallet.balance, 9);
        }
    }
    #[test]
    fn duplicate_costs_are_checked_together_and_grants_do_not_fund_costs() {
        let mut bag = OwnedItems::default();
        bag.grant(catalog(), Item::HealthCell, 3);
        let before = bag.clone();
        let line = ItemAmount {
            item: Item::HealthCell,
            count: 2,
        };
        let request = InventoryExchange {
            costs: vec![line, line],
            grants: vec![line],
            ..Default::default()
        };
        assert_eq!(
            request.apply(catalog(), &mut bag, None),
            Err(ExchangeRefusal::InsufficientItems(Item::HealthCell))
        );
        assert_eq!(bag, before);
    }
    #[test]
    fn exchange_returns_actual_changes_and_commits_both_owners() {
        let mut bag = OwnedItems::default();
        bag.grant(catalog(), Item::HealthCell, 3);
        let mut wallet = BodyWallet { balance: 10 };
        let result = InventoryExchange {
            costs: vec![ItemAmount {
                item: Item::HealthCell,
                count: 2,
            }],
            grants: vec![ItemAmount {
                item: Item::ManaCell,
                count: 1,
            }],
            pay_coins: 4,
            receive_coins: 1,
        }
        .apply(catalog(), &mut bag, Some(&mut wallet))
        .unwrap();
        assert_eq!(bag.count(Item::HealthCell), 1);
        assert_eq!(bag.count(Item::ManaCell), 1);
        assert_eq!(wallet.balance, 7);
        assert_eq!(result.coins, -3);
        assert_eq!(result.items.len(), 2);
    }
    #[test]
    fn overflow_and_missing_wallet_refuse_before_item_costs() {
        let mut bag = OwnedItems::default();
        bag.grant(catalog(), Item::HealthCell, 2);
        let before = bag.clone();
        let request = InventoryExchange {
            costs: vec![ItemAmount {
                item: Item::HealthCell,
                count: 1,
            }],
            receive_coins: 1,
            ..Default::default()
        };
        assert_eq!(
            request.apply(catalog(), &mut bag, None),
            Err(ExchangeRefusal::MissingWallet)
        );
        let mut wallet = BodyWallet { balance: i32::MAX };
        assert_eq!(
            request.apply(catalog(), &mut bag, Some(&mut wallet)),
            Err(ExchangeRefusal::WalletOverflow)
        );
        assert_eq!(bag, before);
        assert_eq!(wallet.balance, i32::MAX);
    }
}

#[cfg(test)]
mod authoring_tests {
    use super::*;
    #[test]
    fn authored_id_round_trips_through_the_catalog_and_save_without_dialogue_normalization() {
        let mut rows = crate::test_item_catalog().rows().to_vec();
        rows[Item::FieldSurvey.index()].dialog_id = "a_game.reactor_key".into();
        let catalog = ItemCatalog::from_rows(rows);
        assert_eq!(
            catalog.item_by_id("a_game.reactor_key"),
            Some(Item::FieldSurvey)
        );
        let mut bag = OwnedItems::default();
        bag.grant(&catalog, Item::FieldSurvey, 1);
        let saved = bag.to_persisted(&catalog);
        let mut restored = OwnedItems::default();
        restored.apply_persisted(&catalog, &saved);
        assert_eq!(restored, bag);
    }
}
