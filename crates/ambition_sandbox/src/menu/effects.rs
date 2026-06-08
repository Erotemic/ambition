//! Backend-agnostic item-confirmation effects for the unified menu.
//!
//! Two halves, kept together because they are the ONE place an item
//! confirmation turns into ECS side effects (no portal / equip / heal logic is
//! duplicated by the Grid or Cube backend):
//!
//! * **decision** ([`MenuAction`] + [`decide`] + [`status_for`]) — pure,
//!   ECS-free, trivially unit-testable: "what does confirming this slot do?".
//! * **application** ([`MenuEffectPlayers`] / [`MenuEffectManaQuery`] +
//!   [`apply_menu_action`] / [`dispatch_item_confirm`]) — turns a decided
//!   [`MenuAction`] into the actual equip / use side effects.
//!
//! Relocated from the now-deleted `crate::bevy_ui_grid_menu` (Phase D1); the
//! text-only 6×4 OoT grid renderer it lived in is superseded by
//! [`crate::menu::grid_backend`], but these helpers are still shared by
//! [`crate::menu::dispatch`], [`crate::menu::grid_backend`], and the cube host
//! [`crate::lunex_kaleidoscope_app`].

use bevy::prelude::*;

use crate::brain::ActionSet;
use crate::item_pickup::{equip_held_spec, held_spec_for_item, unequip_held, StashedActionSet};
use crate::items::{Item, ItemCategory, OwnedItems};
use crate::player::{PlayerEntity, PlayerHealRequested, PlayerMana, PrimaryPlayer};

/// One health cell restores this much HP; one mana cell this much mana. Sandbox
/// values — a real balance pass is just a number change.
const HEALTH_CELL_HEAL: i32 = 4;
const MANA_CELL_RESTORE: f32 = 40.0;

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

/// The player query shape every menu-effect dispatch shares (grid + cube). The
/// lifetimes stay free so callers (systems with their own `'w`/`'s`) can pass
/// `&mut their_query` without the borrow escaping to `'static`.
pub(crate) type MenuEffectPlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut ActionSet,
        Option<&'static StashedActionSet>,
    ),
    (With<PlayerEntity>, With<PrimaryPlayer>),
>;

/// The player-mana query shape shared by every menu-effect dispatch.
pub(crate) type MenuEffectManaQuery<'w, 's> =
    Query<'w, 's, &'static mut PlayerMana, (With<PlayerEntity>, With<PrimaryPlayer>)>;

/// Decide and apply the effect of confirming `item` (equip / unequip / use /
/// inspect). The ONE place both menu backends turn an item confirmation into ECS
/// side effects — neither duplicates the portal/equip/heal logic. Returns the
/// decided [`MenuAction`] so callers can surface its status.
pub(crate) fn dispatch_item_confirm(
    item: Item,
    owned: &mut OwnedItems,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers<'_, '_>,
    mana_q: &mut MenuEffectManaQuery<'_, '_>,
    heals: &mut MessageWriter<PlayerHealRequested>,
) -> MenuAction {
    let action = decide(item, owned);
    apply_menu_action(action, owned, commands, players, mana_q, heals);
    action
}

/// Turn a decided [`MenuAction`] into its ECS side effects.
pub(crate) fn apply_menu_action(
    action: MenuAction,
    owned: &mut OwnedItems,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers<'_, '_>,
    mana_q: &mut MenuEffectManaQuery<'_, '_>,
    heals: &mut MessageWriter<PlayerHealRequested>,
) {
    match action {
        MenuAction::Equip(item) => {
            // The portal gun equips via its own component; other weapons via a
            // HeldItemSpec. Bail early if the item is neither. With the portal
            // mechanic compiled out, the Portal Gun roster slot still exists but
            // has no equip path, so it behaves like an unwired weapon.
            #[cfg(feature = "portal")]
            let is_portal_gun = item == Item::PortalGun;
            #[cfg(not(feature = "portal"))]
            let is_portal_gun = false;
            let held_spec = held_spec_for_item(item);
            if !is_portal_gun && held_spec.is_none() {
                return;
            }
            if let Ok((player, mut action_set, stashed)) = players.single_mut() {
                // Clear whatever weapon is currently held (a held item OR the
                // portal gun) so we re-stash the true base, then equip the new one.
                if stashed.is_some() {
                    unequip_held(commands, player, &mut action_set, stashed);
                    #[cfg(feature = "portal")]
                    commands.entity(player).remove::<crate::portal::PortalGun>();
                }
                #[cfg(feature = "portal")]
                if is_portal_gun {
                    crate::ambition_content::portal::equip_portal_gun(
                        commands,
                        player,
                        &mut action_set,
                    );
                } else if let Some(spec) = held_spec {
                    equip_held_spec(commands, player, &mut action_set, spec);
                }
                #[cfg(not(feature = "portal"))]
                if let Some(spec) = held_spec {
                    equip_held_spec(commands, player, &mut action_set, spec);
                }
                owned.set_equipped(Some(item));
            }
        }
        MenuAction::Unequip(_item) => {
            if let Ok((player, mut action_set, stashed)) = players.single_mut() {
                // Detach both possible weapon front-ends (held item + portal gun).
                unequip_held(commands, player, &mut action_set, stashed);
                #[cfg(feature = "portal")]
                commands.entity(player).remove::<crate::portal::PortalGun>();
            }
            owned.set_equipped(None);
        }
        MenuAction::UseConsumable(Item::HealthCell) => {
            if owned.take(Item::HealthCell, 1) > 0 {
                heals.write(PlayerHealRequested::new(HEALTH_CELL_HEAL));
            }
        }
        MenuAction::UseConsumable(Item::ManaCell) => {
            if owned.take(Item::ManaCell, 1) > 0 {
                if let Ok(mut mana) = mana_q.single_mut() {
                    mana.meter.refill(MANA_CELL_RESTORE);
                }
            }
        }
        MenuAction::UseConsumable(_) | MenuAction::Inspect(_) | MenuAction::NotOwned(_) => {}
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
