//! Backend-agnostic item-confirmation effects for the unified menu.
//!
//! Pure decision helpers decide what confirming a slot means; the ECS helpers
//! apply the resolved equip/use/inspect effect for both grid and cube backends.

use bevy::prelude::*;

use ambition_platformer2d::held_items::{empty_hand, equip_held_spec, held_spec_for_item, item_in_hand};
use ambition_platformer2d::combat::held_items::HeldItem;
use ambition_platformer2d::items::{EquippedItems, Inventory, Item, ItemCatalog, ItemCategory, OwnedItems};
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

/// What pressing confirm on a slot should do, given current ownership/equip state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    /// Equip this item: a held item goes in the hand; an ability's row is
    /// worn and takes its slot.
    Equip(Item),
    /// Stow the held item in the hand.
    Unequip(Item),
    /// Consume one of a usable consumable (health / mana cell today).
    UseConsumable(Item),
    /// Owned, but confirming does nothing: a key item, an unimplemented
    /// consumable, or an ability already in effect on its slot. The menu still
    /// shows its detail.
    Inspect(Item),
    /// Not in the player's possession — confirm is a no-op with feedback.
    NotOwned(Item),
}

/// Decide the action for confirming `item`, against the bag and what the
/// primary body has `equipped` (its hand and its worn abilities).
pub fn decide(items: &ItemCatalog, item: Item, owned: &Inventory<'_>, equipped: EquippedItems) -> MenuAction {
    if !owned.has(item) {
        return MenuAction::NotOwned(item);
    }
    // A slot the catalog does not name is no item; there is nothing to do.
    let Some(category) = items.category(item) else {
        return MenuAction::Inspect(item);
    };
    match category {
        ItemCategory::Held if equipped.contains(item) => MenuAction::Unequip(item),
        ItemCategory::Held => MenuAction::Equip(item),
        // An ability in effect stays: one variant of a slot is always the
        // body's, so to change it is to equip another one.
        ItemCategory::Ability if items.equips(item) && !equipped.contains(item) => MenuAction::Equip(item),
        ItemCategory::Ability => MenuAction::Inspect(item),
        ItemCategory::Consumable => match item {
            Item::HealthCell | Item::ManaCell => MenuAction::UseConsumable(item),
            // The battery has no in-menu use yet.
            _ => MenuAction::Inspect(item),
        },
        ItemCategory::KeyItem | ItemCategory::Reserved => MenuAction::Inspect(item),
    }
}

/// What the menu calls a slot its catalog does not name.
pub const UNNAMED_SLOT: &str = "—";

/// A short status line describing what just happened, for the menu footer.
pub fn status_for(items: &ItemCatalog, action: MenuAction) -> String {
    let name = |item| items.display_name(item).unwrap_or(UNNAMED_SLOT);
    match action {
        MenuAction::Equip(i) => format!("Equipped {}", name(i)),
        MenuAction::Unequip(i) => format!("Stowed {}", name(i)),
        MenuAction::UseConsumable(i) => format!("Used {}", name(i)),
        MenuAction::Inspect(i) => name(i).to_string(),
        MenuAction::NotOwned(i) => format!("{} — not acquired", name(i)),
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
        ambition_platformer2d::combat::hand::RepertoireQuery,
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
    (Entity, ambition_platformer2d::combat::hand::RepertoireQuery),
    (With<PlayerEntity>, With<PrimaryPlayer>),
>;

/// The primary player's HAND, read where it lives (I1): what the menu calls
/// "equipped". Every menu reader of that fact goes through [`Self::in_hand`];
/// there is no catalog slot mirroring it any more.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct PrimaryHand<'w, 's> {
    // The catalog that names the item in the hand.
    items: ambition_platformer2d::items::ItemCatalogRead<'w>,
    held: Query<'w, 's, Option<&'static HeldItem>, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    // The abilities it wears.
    worn: Query<
        'w,
        's,
        Option<&'static ambition_platformer2d::characters::equipment::WornEquipment>,
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
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
            self.items.get(),
            held,
            #[cfg(feature = "portal")]
            gun,
        )
    }

    /// What the primary player has equipped: the item in its hand and each
    /// ability in effect on its slot.
    pub(crate) fn equipped(&self) -> EquippedItems {
        EquippedItems::of(self.items(), self.in_hand(), self.worn.single().ok().flatten())
    }

    /// The item catalog the menu names items with: the same one
    /// [`Self::in_hand`] reads.
    pub(crate) fn items(&self) -> &ItemCatalog {
        self.items.get()
    }

    /// Did a content reload publish a new item catalog since this system last
    /// ran? A menu face that names items must then be built again.
    pub(crate) fn items_changed(&self) -> bool {
        self.items.is_changed()
    }
}

/// Where a menu asks the simulation to use a consumable or to equip an
/// ability. The simulation applies each request on one stamped tick
/// (`apply_item_uses`, `apply_item_equips`), so the menu writes neither the
/// bag nor the body.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MenuItemRequests<'w, 's> {
    uses: ambition_platformer2d::platformer::host_intents::HostIntentWriter<
        'w,
        's,
        ambition_platformer2d::items::ItemUseRequested,
    >,
    equips: ambition_platformer2d::platformer::host_intents::HostIntentWriter<
        'w,
        's,
        ambition_platformer2d::items::ItemEquipRequested,
    >,
}

/// TEST SEAM: the primary player's hand, read from a bare `World` (the fact
/// tests used to read off `OwnedItems::equipped`).
#[cfg(test)]
pub(crate) fn hand_of_primary_player(world: &mut World) -> Option<Item> {
    let items = ambition_platformer2d::items::item_catalog(world).clone();
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
        &items,
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
    owned: &OwnedItems,
    hand: &PrimaryHand<'_, '_>,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers<'_, '_>,
    requests: &mut MenuItemRequests<'_, '_>,
) -> MenuAction {
    let items = hand.items();
    let action = decide(items, item, &Inventory::new(owned, hand.in_hand()), hand.equipped());
    apply_menu_action(items, action, commands, players, requests);
    action
}

/// Turn a decided [`MenuAction`] into its ECS side effects.
pub(crate) fn apply_menu_action(
    items: &ItemCatalog,
    action: MenuAction,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers<'_, '_>,
    requests: &mut MenuItemRequests<'_, '_>,
) {
    match action {
        // An ability is worn by the simulation, on its own tick.
        MenuAction::Equip(item) if items.category(item) == Some(ItemCategory::Ability) => {
            requests.equips.write(ambition_platformer2d::items::ItemEquipRequested { item });
        }
        MenuAction::Equip(item) => {
            // The portal gun equips via its own component; other weapons via a
            // HeldItemSpec. Bail early if the item is neither. With the portal
            // mechanic compiled out, the Portal Gun roster slot still exists but
            // has no equip path, so it behaves like an unwired weapon.
            #[cfg(feature = "portal")]
            let is_portal_gun = item == Item::PortalGun;
            #[cfg(not(feature = "portal"))]
            let is_portal_gun = false;
            let held_spec = held_spec_for_item(items, item);
            if !is_portal_gun && held_spec.is_none() {
                return;
            }
            if let Ok(parts) = players.single_mut() {
                #[cfg(feature = "portal")]
                let (player, mut repertoire, owned_pair) = parts;
                #[cfg(not(feature = "portal"))]
                let (player, mut repertoire) = parts;
                // Clear whatever weapon is currently held (a held item OR the
                // portal gun), then equip the new one. The hand is the only
                // record of what is equipped (I1).
                empty_hand(commands, player, &mut repertoire);
                #[cfg(feature = "portal")]
                if is_portal_gun {
                    ambition_platformer2d::held_items::equip_portal_gun(
                        commands,
                        player,
                        &mut repertoire,
                        // The pair this body owns; 0 only if it never held one.
                        owned_pair.map_or(0, |owned| owned.0),
                    );
                } else if let Some(spec) = held_spec {
                    equip_held_spec(commands, player, &mut repertoire, spec);
                }
                #[cfg(not(feature = "portal"))]
                if let Some(spec) = held_spec {
                    equip_held_spec(commands, player, &mut repertoire, spec);
                }
            }
        }
        MenuAction::Unequip(_item) => {
            if let Ok(parts) = players.single_mut() {
                #[cfg(feature = "portal")]
                let (player, mut repertoire, _owned_pair) = parts;
                #[cfg(not(feature = "portal"))]
                let (player, mut repertoire) = parts;
                // Detach both possible weapon front-ends (held item + portal gun).
                empty_hand(commands, player, &mut repertoire);
            }
        }
        MenuAction::UseConsumable(item @ (Item::HealthCell | Item::ManaCell)) => {
            requests.uses.write(ambition_platformer2d::items::ItemUseRequested { item });
        }
        MenuAction::UseConsumable(_) | MenuAction::Inspect(_) | MenuAction::NotOwned(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::characters::equipment::WornEquipment;

    fn catalog() -> &'static ItemCatalog {
        ambition_platformer2d::items::test_item_catalog()
    }

    /// What confirming `item` does for a body with `hand` in its hand, wearing `worn`.
    fn decide_for(item: Item, owned: &OwnedItems, hand: Option<Item>, worn: Option<&WornEquipment>) -> MenuAction {
        decide(catalog(), item, &Inventory::new(owned, hand), EquippedItems::of(catalog(), hand, worn))
    }

    #[test]
    fn unowned_item_is_a_noop_action() {
        let owned = OwnedItems::default();
        assert_eq!(decide_for(Item::Axe, &owned, None, None), MenuAction::NotOwned(Item::Axe));
    }

    #[test]
    fn weapon_toggles_between_equip_and_unequip() {
        let mut owned = OwnedItems::default();
        owned.grant(catalog(), Item::Axe, 1);
        assert_eq!(decide_for(Item::Axe, &owned, None, None), MenuAction::Equip(Item::Axe));
        assert_eq!(decide_for(Item::Axe, &owned, Some(Item::Axe), None), MenuAction::Unequip(Item::Axe));
    }

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
        app.insert_resource(catalog().clone());
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

    /// A weapon picked up off the FLOOR has no stored copy; the hand is the
    /// record, and the menu must offer to stow it rather than call it not
    /// acquired (I1).
    #[test]
    fn a_wielded_weapon_with_no_stored_copy_is_owned_and_stowable() {
        let owned = OwnedItems::default();
        assert_eq!(
            decide_for(Item::GunSword, &owned, Some(Item::GunSword), None),
            MenuAction::Unequip(Item::GunSword)
        );
        assert_eq!(decide_for(Item::GunSword, &owned, None, None), MenuAction::NotOwned(Item::GunSword));
    }

    #[test]
    fn usable_consumables_use_others_inspect() {
        let mut owned = OwnedItems::default();
        owned.grant(catalog(), Item::HealthCell, 1);
        owned.grant(catalog(), Item::ManaCell, 1);
        owned.grant(catalog(), Item::SpareBattery, 1);
        assert_eq!(decide_for(Item::HealthCell, &owned, None, None), MenuAction::UseConsumable(Item::HealthCell));
        assert_eq!(decide_for(Item::ManaCell, &owned, None, None), MenuAction::UseConsumable(Item::ManaCell));
        // Owned but no effect → inspect.
        assert_eq!(decide_for(Item::SpareBattery, &owned, None, None), MenuAction::Inspect(Item::SpareBattery));
    }

    #[test]
    fn key_items_and_placeholder_abilities_inspect_when_owned() {
        // A placeholder ability has no row, so even a granted one equips nothing.
        let mut owned = OwnedItems::default();
        owned.grant(catalog(), Item::AxeStance, 1);
        owned.grant(catalog(), Item::MapFragment, 1);
        assert_eq!(decide_for(Item::AxeStance, &owned, None, None), MenuAction::Inspect(Item::AxeStance));
        assert_eq!(decide_for(Item::MapFragment, &owned, None, None), MenuAction::Inspect(Item::MapFragment));
    }

    #[test]
    fn a_held_gauntlet_equips_like_a_weapon() {
        let mut owned = OwnedItems::default();
        owned.grant(catalog(), Item::MarkRecall, 1);
        assert!(catalog().held_item_id(Item::MarkRecall).is_some(), "Mark/Recall is wired");
        assert_eq!(decide_for(Item::MarkRecall, &owned, None, None), MenuAction::Equip(Item::MarkRecall));
        assert_eq!(
            decide_for(Item::MarkRecall, &owned, Some(Item::MarkRecall), None),
            MenuAction::Unequip(Item::MarkRecall)
        );
    }

    /// An ability variant equips into its slot, and the variant in effect on
    /// the slot stays: to change it is to equip another one.
    #[test]
    fn an_ability_variant_equips_until_it_is_in_effect() {
        let owned = OwnedItems::starter(catalog());
        assert_eq!(decide_for(Item::EyeLaser, &owned, None, None), MenuAction::Equip(Item::EyeLaser));
        assert_eq!(
            decide_for(Item::Fireball, &owned, None, None),
            MenuAction::Inspect(Item::Fireball),
            "the body's own projectile is in effect while nothing holds the slot"
        );
        let mut worn = WornEquipment::default();
        worn.equip(catalog().wears(Item::EyeLaser).unwrap().clone());
        assert_eq!(decide_for(Item::EyeLaser, &owned, None, Some(&worn)), MenuAction::Inspect(Item::EyeLaser));
        assert_eq!(decide_for(Item::Fireball, &owned, None, Some(&worn)), MenuAction::Equip(Item::Fireball));
    }
}
