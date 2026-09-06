//! Backend-agnostic item-confirmation effects for the unified menu.
//!
//! Pure decision helpers decide what confirming a slot means; the ECS helpers
//! apply the resolved equip/use/inspect effect for both grid and cube backends.

use bevy::prelude::*;

use ambition_platformer2d::actors::avatar::PlayerHealRequested;
use ambition_platformer2d::held_items::{equip_held_spec, held_spec_for_item, item_in_hand, unequip_held, StashedActionSet};
use ambition_platformer2d::characters::brain::ActionSet;
use ambition_platformer2d::combat::held_items::HeldItem;
use ambition_platformer2d::engine_core::BodyMana;
use ambition_platformer2d::items::{Inventory, Item, ItemCategory, OwnedItems};
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

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

/// Decide the action for confirming `item`, against the bag AND the hand.
pub fn decide(item: Item, owned: &Inventory<'_>) -> MenuAction {
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
/// ⚠ Spelled twice under `cfg` rather than once with a `cfg`'d tuple element:
/// an attribute cannot be applied to a type inside a tuple, and the extra
/// member only exists when the portal mechanic is compiled in.
#[cfg(feature = "portal")]
pub(crate) type MenuEffectPlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut ActionSet,
        Option<&'static StashedActionSet>,
        // Which portal pair this body owns, if it owns a gun at all. Read so a
        // menu re-equip hands back the gun the player actually has rather than
        // a fresh default one.
        Option<&'static ambition_platformer2d::portal::OwnedPortalGunPair>,
    ),
    (With<PlayerEntity>, With<PrimaryPlayer>),
>;

#[cfg(not(feature = "portal"))]
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

/// The primary player's HAND, read where it lives (I1): what the menu calls
/// "equipped". Every menu reader of that fact goes through [`Self::in_hand`];
/// there is no catalog slot mirroring it any more.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct PrimaryHand<'w, 's> {
    held: Query<'w, 's, Option<&'static HeldItem>, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    #[cfg(feature = "portal")]
    guns: Query<
        'w,
        's,
        Option<&'static ambition_platformer2d::portal::PortalGun>,
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
}

impl PrimaryHand<'_, '_> {
    /// The catalog item in the primary player's hand, or `None` for an empty
    /// hand, an item the catalog has no row for, or no primary player at all.
    pub(crate) fn in_hand(&self) -> Option<Item> {
        let held = self.held.single().ok().flatten();
        #[cfg(feature = "portal")]
        let gun = self.guns.single().ok().flatten();
        item_in_hand(
            held,
            #[cfg(feature = "portal")]
            gun,
        )
    }
}

/// The player-mana query shape shared by every menu-effect dispatch.
pub(crate) type MenuEffectManaQuery<'w, 's> =
    Query<'w, 's, &'static mut BodyMana, (With<PlayerEntity>, With<PrimaryPlayer>)>;

/// TEST SEAM: the primary player's hand, read from a bare `World` (the fact
/// tests used to read off `OwnedItems::equipped`).
#[cfg(test)]
pub(crate) fn hand_of_primary_player(world: &mut World) -> Option<Item> {
    let held = world
        .query_filtered::<Option<&HeldItem>, (With<PlayerEntity>, With<PrimaryPlayer>)>()
        .single(world)
        .ok()
        .flatten()
        .cloned();
    #[cfg(feature = "portal")]
    let gun = world
        .query_filtered::<
            Option<&ambition_platformer2d::portal::PortalGun>,
            (With<PlayerEntity>, With<PrimaryPlayer>),
        >()
        .single(world)
        .ok()
        .flatten()
        .cloned();
    item_in_hand(
        held.as_ref(),
        #[cfg(feature = "portal")]
        gun.as_ref(),
    )
}

/// Decide and apply the effect of confirming `item` (equip / unequip / use /
/// inspect). The ONE place both menu backends turn an item confirmation into ECS
/// side effects — neither duplicates the portal/equip/heal logic. Returns the
/// decided [`MenuAction`] so callers can surface its status.
pub(crate) fn dispatch_item_confirm(
    item: Item,
    owned: &mut OwnedItems,
    hand: &PrimaryHand<'_, '_>,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers<'_, '_>,
    mana_q: &mut MenuEffectManaQuery<'_, '_>,
    heals: &mut MessageWriter<PlayerHealRequested>,
) -> MenuAction {
    let action = decide(item, &Inventory::new(owned, hand.in_hand()));
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
            if let Ok(parts) = players.single_mut() {
                #[cfg(feature = "portal")]
                let (player, mut action_set, stashed, owned_pair) = parts;
                #[cfg(not(feature = "portal"))]
                let (player, mut action_set, stashed) = parts;
                // Clear whatever weapon is currently held (a held item OR the
                // portal gun) so we re-stash the true base, then equip the new one.
                // The hand is the only record of what is equipped (I1): there is
                // no catalog slot to keep in step any more.
                if stashed.is_some() {
                    unequip_held(commands, player, &mut action_set, stashed);
                    #[cfg(feature = "portal")]
                    commands
                        .entity(player)
                        .remove::<ambition_platformer2d::portal::PortalGun>();
                }
                #[cfg(feature = "portal")]
                if is_portal_gun {
                    ambition_platformer2d::held_items::equip_portal_gun(
                        commands,
                        player,
                        &mut action_set,
                        // The pair this body owns; 0 only if it never held one.
                        owned_pair.map_or(0, |owned| owned.0),
                    );
                } else if let Some(spec) = held_spec {
                    equip_held_spec(commands, player, &mut action_set, spec);
                }
                #[cfg(not(feature = "portal"))]
                if let Some(spec) = held_spec {
                    equip_held_spec(commands, player, &mut action_set, spec);
                }
            }
        }
        MenuAction::Unequip(_item) => {
            if let Ok(parts) = players.single_mut() {
                #[cfg(feature = "portal")]
                let (player, mut action_set, stashed, _owned_pair) = parts;
                #[cfg(not(feature = "portal"))]
                let (player, mut action_set, stashed) = parts;
                // Detach both possible weapon front-ends (held item + portal gun).
                unequip_held(commands, player, &mut action_set, stashed);
                #[cfg(feature = "portal")]
                commands
                    .entity(player)
                    .remove::<ambition_platformer2d::portal::PortalGun>();
            }
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
        assert_eq!(
            decide(Item::Axe, &Inventory::new(&owned, None)),
            MenuAction::NotOwned(Item::Axe)
        );
    }

    #[test]
    fn weapon_toggles_between_equip_and_unequip() {
        let mut owned = OwnedItems::default();
        owned.grant(Item::Axe, 1);
        assert_eq!(
            decide(Item::Axe, &Inventory::new(&owned, None)),
            MenuAction::Equip(Item::Axe)
        );
        assert_eq!(
            decide(Item::Axe, &Inventory::new(&owned, Some(Item::Axe))),
            MenuAction::Unequip(Item::Axe)
        );
    }

    /// A weapon picked up off the FLOOR has no stored copy; the hand is the
    /// record, and the menu must offer to stow it rather than call it not
    /// acquired (I1).
    /// THE DEFECT THE MIRROR HAD: a second seat picking up a gun-sword marked
    /// it equipped in the first seat's menu, because one process-global slot
    /// was written by every equip road. `PrimaryHand` reads the primary body
    /// and nobody else's.
    #[test]
    fn another_seats_weapon_is_not_the_primary_players_equipped_item() {
        use ambition_platformer2d::characters::brain::held_item_by_id;
        #[derive(Resource, Default)]
        struct Seen(Option<Option<Item>>);
        fn read(hand: PrimaryHand, mut seen: ResMut<Seen>) {
            seen.0 = Some(hand.in_hand());
        }
        let mut app = App::new();
        app.init_resource::<Seen>();
        app.add_systems(Update, read);
        let sword = held_item_by_id("gun_sword").unwrap();
        // Seat two, wielding.
        app.world_mut()
            .spawn((PlayerEntity, HeldItem::new(sword.clone())));
        // The primary player, empty-handed.
        let primary = app.world_mut().spawn((PlayerEntity, PrimaryPlayer)).id();
        app.update();
        assert_eq!(
            app.world().resource::<Seen>().0,
            Some(None),
            "seat two's gun-sword must not read as the primary player's equipped item"
        );
        // And the primary's own hand does.
        app.world_mut()
            .entity_mut(primary)
            .insert(HeldItem::new(sword));
        app.update();
        assert_eq!(app.world().resource::<Seen>().0, Some(Some(Item::GunSword)));
    }

    #[test]
    fn a_wielded_weapon_with_no_stored_copy_is_owned_and_stowable() {
        let owned = OwnedItems::default();
        assert_eq!(
            decide(
                Item::GunSword,
                &Inventory::new(&owned, Some(Item::GunSword))
            ),
            MenuAction::Unequip(Item::GunSword)
        );
        assert_eq!(
            decide(Item::GunSword, &Inventory::new(&owned, None)),
            MenuAction::NotOwned(Item::GunSword)
        );
    }

    #[test]
    fn usable_consumables_use_others_inspect() {
        let mut owned = OwnedItems::default();
        owned.grant(Item::HealthCell, 1);
        owned.grant(Item::ManaCell, 1);
        owned.grant(Item::DataChip, 1);
        assert_eq!(
            decide(Item::HealthCell, &Inventory::new(&owned, None)),
            MenuAction::UseConsumable(Item::HealthCell)
        );
        assert_eq!(
            decide(Item::ManaCell, &Inventory::new(&owned, None)),
            MenuAction::UseConsumable(Item::ManaCell)
        );
        // Owned but no effect → inspect.
        assert_eq!(
            decide(Item::DataChip, &Inventory::new(&owned, None)),
            MenuAction::Inspect(Item::DataChip)
        );
    }

    #[test]
    fn abilities_and_key_items_inspect_when_owned() {
        // Fly is still an unwired ability slot (no HeldItemSpec) → inspect-only.
        let mut owned = OwnedItems::default();
        owned.grant(Item::Fly, 1);
        owned.grant(Item::MapFragment, 1);
        assert_eq!(
            decide(Item::Fly, &Inventory::new(&owned, None)),
            MenuAction::Inspect(Item::Fly)
        );
        assert_eq!(
            decide(Item::MapFragment, &Inventory::new(&owned, None)),
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
            decide(Item::MarkRecall, &Inventory::new(&owned, None)),
            MenuAction::Equip(Item::MarkRecall)
        );
        assert_eq!(
            decide(
                Item::MarkRecall,
                &Inventory::new(&owned, Some(Item::MarkRecall))
            ),
            MenuAction::Unequip(Item::MarkRecall)
        );
    }
}
