//! Backend-agnostic item-confirmation effects for the unified menu.
//!
//! Pure decision helpers decide what confirming a slot means; the ECS helpers
//! apply the resolved equip/use/inspect effect for both grid and cube backends.

use bevy::prelude::*;

use ambition_platformer2d::held_items::item_in_hand;
use ambition_platformer2d::combat::held_items::HeldItem;
use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::items::{EquippedItems, Inventory, Item, ItemCatalog, ItemCategory, OwnedItems};
#[cfg(test)]
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
/// body has equipped in its hand and worn abilities.
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

/// Read the body controlled by a menu's participant, without a controller-kind filter.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MenuBodySource<'w, 's> {
    items: ambition_platformer2d::items::ItemCatalogRead<'w>,
    drivers: Query<'w, 's, (Entity, &'static DrivingParticipant)>,
    held: Query<'w, 's, Option<&'static HeldItem>>,
    worn: Query<'w, 's, Option<&'static ambition_platformer2d::characters::equipment::WornEquipment>>,
}

fn body_for_slot(drivers: &Query<(Entity, &DrivingParticipant)>, participant: PlayerSlot) -> Option<Entity> {
    let mut candidates = drivers.iter().filter(|(_, driver)| driver.0 == participant);
    let body = candidates.next().map(|(entity, _)| entity);
    if candidates.next().is_none() { body } else { None }
}

/// The equipment and identity read from one controlled body at this boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MenuBodyFacts {
    pub entity: Entity,
    pub participant: PlayerSlot,
    pub in_hand: Option<Item>,
    pub equipped: EquippedItems,
}

impl MenuBodySource<'_, '_> {
    /// An absent or ambiguous binding has no body. It cannot select another seat.
    pub(crate) fn read(&self, owner: ambition_platformer2d::input::ParticipantId) -> Option<MenuBodyFacts> {
        let participant = ambition_platformer2d::actors::participant_seat::player_slot_of(owner);
        let entity = body_for_slot(&self.drivers, participant)?;
        let held = self.held.get(entity).ok().flatten();
        let in_hand = item_in_hand(self.items.get(), held);
        Some(MenuBodyFacts {
            entity, participant, in_hand,
            equipped: EquippedItems::of(self.items(), in_hand, self.worn.get(entity).ok().flatten()),
        })
    }

    pub(crate) fn items(&self) -> &ItemCatalog { self.items.get() }
    pub(crate) fn items_changed(&self) -> bool { self.items.is_changed() }
}

/// Where a menu asks the simulation to use a consumable, to equip an item or
/// to stow what a hand holds. The simulation applies each request on one
/// stamped tick (`apply_item_uses`, `apply_item_equips`,
/// `apply_hand_requests`), so the menu writes neither the bag nor the body.
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
    stows: ambition_platformer2d::platformer::host_intents::HostIntentWriter<
        'w,
        's,
        ambition_platformer2d::items::ItemStowRequested,
    >,
}

// In a test module, so the rollback-mutator guard, which strips test modules,
// does not read its `Last` registration as a shipped one.
#[cfg(test)]
mod hand_request_seam {
    use super::*;

    /// TEST SEAM: a menu fixture runs no simulation, so this stands in for its
    /// tick. At the end of each frame it hands each new equip and stow request in
    /// the host-intent ledgers to the simulation's own applier
    /// (`ambition_held_items::apply_hand_requests`), so a test reads the hand the
    /// request leads to.
    pub(crate) fn the_simulation_applies_hand_requests(app: &mut App) {
        use ambition_platformer2d::items::{ItemEquipRequested, ItemStowRequested};
        use ambition_platformer2d::platformer::host_intents::HostIntentLedger;
        fn forward(
            equips: Res<HostIntentLedger<ItemEquipRequested>>,
            stows: Res<HostIntentLedger<ItemStowRequested>>,
            mut seen: Local<(usize, usize)>,
            mut equip: MessageWriter<ItemEquipRequested>,
            mut stow: MessageWriter<ItemStowRequested>,
        ) {
            equip.write_batch(equips.payloads().skip(seen.0).copied());
            stow.write_batch(stows.payloads().skip(seen.1).copied());
            *seen = (equips.depth(), stows.depth());
        }
        app.add_message::<ItemEquipRequested>().add_message::<ItemStowRequested>();
        app.add_systems(Last, (forward, ambition_platformer2d::held_items::apply_hand_requests).chain());
        // A request is stamped for a live session; the one it would act on.
        app.world_mut().spawn(ambition_platformer2d::platformer::lifecycle::SessionRoot(
            ambition_platformer2d::platformer::lifecycle::SessionScopeId(1),
        ));
    }
}
#[cfg(test)]
pub(crate) use hand_request_seam::the_simulation_applies_hand_requests;

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
    item_in_hand(&items, held.as_ref())
}

/// Decide the effect of confirming `item` (equip / unequip / use / inspect)
/// and ask the simulation for it. The ONE place both menu backends turn an
/// item confirmation into a request; neither writes a hand, a bag or a worn
/// set. Returns the decided [`MenuAction`] so callers can surface its status.
pub(crate) fn dispatch_item_confirm(
    owner: ambition_platformer2d::input::ParticipantId,
    item: Item,
    owned: &OwnedItems,
    body_source: &MenuBodySource<'_, '_>,
    requests: &mut MenuItemRequests<'_, '_>,
) -> MenuAction {
    let Some(body) = body_source.read(owner) else { return MenuAction::Inspect(item); };
    let items = body_source.items();
    let action = decide(items, item, &Inventory::new(owned, body.in_hand), body.equipped);
    apply_menu_action(body.participant, action, requests);
    action
}

/// Turn a decided [`MenuAction`] into the request the simulation applies.
pub(crate) fn apply_menu_action(participant: PlayerSlot, action: MenuAction, requests: &mut MenuItemRequests<'_, '_>) {
    match action {
        // The simulation equips, on its own tick: an ability into the worn
        // set, a held item into the hand.
        MenuAction::Equip(item) => {
            requests.equips.write(ambition_platformer2d::items::ItemEquipRequested { participant, item });
        }
        MenuAction::Unequip(_item) => {
            requests.stows.write(ambition_platformer2d::items::ItemStowRequested { participant });
        }
        MenuAction::UseConsumable(item @ (Item::HealthCell | Item::ManaCell)) => {
            requests.uses.write(ambition_platformer2d::items::ItemUseRequested { participant, item });
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

    /// THE MENU ASKS; IT DOES NOT TAKE. An equip and a stow from the menu are
    /// stamped requests in the host-intent ledgers, for the participant that
    /// asked, and no body changes until the simulation applies them
    /// (`ambition_held_items::apply_hand_requests`, which owns the hand).
    #[test]
    fn a_menu_equip_or_stow_is_a_request_and_writes_no_hand() {
        use ambition_platformer2d::items::{ItemEquipRequested, ItemStowRequested, ItemUseRequested};
        use ambition_platformer2d::platformer::host_intents::HostIntentLedger;
        #[derive(Resource)]
        struct Asked(PlayerSlot, MenuAction);
        fn ask(asked: Res<Asked>, mut requests: MenuItemRequests) {
            apply_menu_action(asked.0, asked.1, &mut requests);
        }
        let mut app = App::new();
        app.init_resource::<HostIntentLedger<ItemUseRequested>>();
        app.init_resource::<HostIntentLedger<ItemEquipRequested>>();
        app.init_resource::<HostIntentLedger<ItemStowRequested>>();
        // One live session root: a request needs a session to be stamped for.
        app.world_mut().spawn(ambition_platformer2d::platformer::lifecycle::SessionRoot(
            ambition_platformer2d::platformer::lifecycle::SessionScopeId(1),
        ));
        let body = app.world_mut().spawn(DrivingParticipant(PlayerSlot(1))).id();
        app.insert_resource(Asked(PlayerSlot(1), MenuAction::Equip(Item::GunSword)));
        app.add_systems(Update, ask);
        app.update();
        assert_eq!(app.world().resource::<HostIntentLedger<ItemEquipRequested>>().depth(), 1);
        app.insert_resource(Asked(PlayerSlot(1), MenuAction::Unequip(Item::GunSword)));
        app.update();
        assert_eq!(app.world().resource::<HostIntentLedger<ItemStowRequested>>().depth(), 1);
        assert!(app.world().get::<HeldItem>(body).is_none(), "the menu put nothing in the hand");
    }

    #[test]
    fn unowned_item_is_a_noop_action() {
        let owned = OwnedItems::default();
        assert_eq!(decide_for(Item::GunSword, &owned, None, None), MenuAction::NotOwned(Item::GunSword));
    }

    #[test]
    fn weapon_toggles_between_equip_and_unequip() {
        let mut owned = OwnedItems::default();
        owned.grant(catalog(), Item::GunSword, 1);
        assert_eq!(decide_for(Item::GunSword, &owned, None, None), MenuAction::Equip(Item::GunSword));
        assert_eq!(decide_for(Item::GunSword, &owned, Some(Item::GunSword), None), MenuAction::Unequip(Item::GunSword));
    }

    /// THE DEFECT THE MIRROR HAD: a second seat picking up a gun-sword marked
    /// it equipped in the first seat's menu, because one process-global slot
    /// was written by every equip road. Each read names its participant.
    #[test]
    fn another_seats_weapon_is_not_the_primary_players_equipped_item() {
        use ambition_platformer2d::characters::brain::held_item_by_id;
        #[derive(Resource, Default)]
        struct Seen(Option<Option<Item>>);
        fn read(body_source: MenuBodySource, mut seen: ResMut<Seen>) {
            seen.0 = Some(body_source.read(ambition_platformer2d::input::ParticipantId::PRIMARY).and_then(|body| body.in_hand));
        }
        let mut app = App::new();
        app.init_resource::<Seen>();
        app.insert_resource(catalog().clone());
        app.add_systems(Update, read);
        let sword = held_item_by_id("gun_sword").unwrap();
        // Seat two, wielding.
        app.world_mut()
            .spawn((DrivingParticipant(PlayerSlot(1)), HeldItem::new(sword.clone())));
        // The primary player, empty-handed.
        let primary = app.world_mut().spawn((PlayerEntity, PrimaryPlayer, DrivingParticipant(PlayerSlot(0)))).id();
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
        owned.grant(catalog(), Item::FieldSurvey, 1);
        assert_eq!(decide_for(Item::AxeStance, &owned, None, None), MenuAction::Inspect(Item::AxeStance));
        assert_eq!(decide_for(Item::FieldSurvey, &owned, None, None), MenuAction::Inspect(Item::FieldSurvey));
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
