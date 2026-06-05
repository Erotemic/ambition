//! Pure decision logic for "what does confirming this slot do?".
//!
//! Kept ECS-free so it's trivially unit-testable; [`super::input`] turns a
//! [`MenuAction`] into the actual equip / use side effects.

use crate::items::{Item, ItemCategory, OwnedItems};

/// What pressing confirm on a slot should do, given current ownership/equip state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    /// Equip this weapon (attach its `HeldItem` + action set).
    Equip(Item),
    /// Unequip the currently-equipped weapon (restore the stashed action set).
    Unequip(Item),
    /// Consume one of a usable consumable (health / mana cell today).
    UseConsumable(Item),
    /// Owned, but confirming does nothing yet (ability/key item, or an
    /// unimplemented consumable). The menu still shows its detail.
    Inspect(Item),
    /// Not in the player's possession — confirm is a no-op with feedback.
    NotOwned(Item),
}

/// Decide the action for confirming `item`.
pub fn decide(item: Item, owned: &OwnedItems) -> MenuAction {
    if !owned.has(item) {
        return MenuAction::NotOwned(item);
    }
    match item.category() {
        ItemCategory::Weapon => {
            if owned.is_equipped(item) {
                MenuAction::Unequip(item)
            } else {
                MenuAction::Equip(item)
            }
        }
        ItemCategory::Consumable => match item {
            Item::HealthCell | Item::ManaCell => MenuAction::UseConsumable(item),
            // Bomb / gold pouch / battery / chip have no in-menu use yet.
            _ => MenuAction::Inspect(item),
        },
        ItemCategory::Ability => {
            // A "wired" ability — one backed by a HeldItemSpec, like Mark/Recall
            // — equips like a weapon (toggle equip/unequip). Ability slots with
            // no mechanic yet (Blink, Fly, …) stay inspect-only lore.
            if item.held_item_id().is_some() {
                if owned.is_equipped(item) {
                    MenuAction::Unequip(item)
                } else {
                    MenuAction::Equip(item)
                }
            } else {
                MenuAction::Inspect(item)
            }
        }
        ItemCategory::KeyItem | ItemCategory::Reserved => MenuAction::Inspect(item),
    }
}

/// A short status line describing what just happened, for the menu footer.
pub fn status_for(action: MenuAction) -> String {
    match action {
        MenuAction::Equip(i) => format!("Equipped {}", i.display_name()),
        MenuAction::Unequip(i) => format!("Stowed {}", i.display_name()),
        MenuAction::UseConsumable(i) => format!("Used {}", i.display_name()),
        MenuAction::Inspect(i) => i.display_name().to_string(),
        MenuAction::NotOwned(i) => format!("{} — not acquired", i.display_name()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unowned_item_is_a_noop_action() {
        let owned = OwnedItems::default();
        assert_eq!(decide(Item::Axe, &owned), MenuAction::NotOwned(Item::Axe));
    }

    #[test]
    fn weapon_toggles_between_equip_and_unequip() {
        let mut owned = OwnedItems::default();
        owned.grant(Item::Axe, 1);
        assert_eq!(decide(Item::Axe, &owned), MenuAction::Equip(Item::Axe));
        owned.set_equipped(Some(Item::Axe));
        assert_eq!(decide(Item::Axe, &owned), MenuAction::Unequip(Item::Axe));
    }

    #[test]
    fn usable_consumables_use_others_inspect() {
        let mut owned = OwnedItems::default();
        owned.grant(Item::HealthCell, 1);
        owned.grant(Item::ManaCell, 1);
        owned.grant(Item::DataChip, 1);
        assert_eq!(
            decide(Item::HealthCell, &owned),
            MenuAction::UseConsumable(Item::HealthCell)
        );
        assert_eq!(
            decide(Item::ManaCell, &owned),
            MenuAction::UseConsumable(Item::ManaCell)
        );
        // Owned but no effect → inspect.
        assert_eq!(
            decide(Item::DataChip, &owned),
            MenuAction::Inspect(Item::DataChip)
        );
    }

    #[test]
    fn abilities_and_key_items_inspect_when_owned() {
        // Fly is still an unwired ability slot (no HeldItemSpec) → inspect-only.
        let mut owned = OwnedItems::default();
        owned.grant(Item::Fly, 1);
        owned.grant(Item::MapFragment, 1);
        assert_eq!(decide(Item::Fly, &owned), MenuAction::Inspect(Item::Fly));
        assert_eq!(
            decide(Item::MapFragment, &owned),
            MenuAction::Inspect(Item::MapFragment)
        );
    }

    #[test]
    fn wired_ability_equips_like_a_weapon() {
        // Mark/Recall is an Ability backed by a HeldItemSpec, so the menu lets
        // you equip/unequip it (unlike Blink, a lore-only ability slot).
        let mut owned = OwnedItems::default();
        owned.grant(Item::MarkRecall, 1);
        assert!(
            Item::MarkRecall.held_item_id().is_some(),
            "Mark/Recall is wired"
        );
        assert_eq!(
            decide(Item::MarkRecall, &owned),
            MenuAction::Equip(Item::MarkRecall)
        );
        owned.set_equipped(Some(Item::MarkRecall));
        assert_eq!(
            decide(Item::MarkRecall, &owned),
            MenuAction::Unequip(Item::MarkRecall)
        );
    }
}
