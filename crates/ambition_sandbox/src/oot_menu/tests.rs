//! End-to-end ECS tests for the OoT menu input system: open/close, grid
//! navigation, equip/unequip (attaching a real `HeldItem` + swapping the
//! `ActionSet`), and consumable use — driven through `oot_menu_input` exactly
//! as the input chain drives it.

use bevy::prelude::*;

use super::input::oot_menu_input;
use super::state::OotMenuState;
use crate::brain::ActionSet;
use crate::features::HeldItem;
use crate::game_mode::GameMode;
use crate::input::MenuControlFrame;
use crate::inventory::InventoryUiState;
use crate::item_pickup::StashedActionSet;
use crate::items::{Item, OwnedItems};
use crate::player::{PlayerEntity, PlayerHealRequested, PlayerMana, PrimaryPlayer};

#[derive(Resource, Default)]
struct HealLog(usize);

fn record_heals(mut reader: MessageReader<PlayerHealRequested>, mut log: ResMut<HealLog>) {
    log.0 += reader.read().count();
}

fn test_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameMode>();
    app.init_resource::<OwnedItems>();
    app.init_resource::<OotMenuState>();
    app.init_resource::<InventoryUiState>();
    app.init_resource::<MenuControlFrame>();
    app.init_resource::<HealLog>();
    app.add_message::<PlayerHealRequested>();
    app.add_systems(Update, (oot_menu_input, record_heals).chain());
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActionSet::default(),
            PlayerMana::default(),
        ))
        .id();
    (app, player)
}

/// Set the menu frame for the next update (cleared each frame in real play; we
/// reset to default then set the one field under test).
fn press(app: &mut App, set: impl FnOnce(&mut MenuControlFrame)) {
    let mut frame = MenuControlFrame::default();
    set(&mut frame);
    app.insert_resource(frame);
}

#[test]
fn inventory_button_opens_and_closes_the_grid() {
    let (mut app, _player) = test_app();
    assert!(!app.world().resource::<InventoryUiState>().visible);

    press(&mut app, |f| f.inventory = true);
    app.update();
    assert!(
        app.world().resource::<InventoryUiState>().visible,
        "Inventory button opens the grid (shared visible flag)"
    );

    press(&mut app, |f| f.inventory = true);
    app.update();
    assert!(
        !app.world().resource::<InventoryUiState>().visible,
        "pressing Inventory again closes it"
    );
}

#[test]
fn equipping_a_weapon_attaches_a_held_item_and_swaps_the_action_set() {
    let (mut app, player) = test_app();
    app.world_mut().resource_mut::<OwnedItems>().grant(Item::Axe, 1);

    // Open.
    press(&mut app, |f| f.inventory = true);
    app.update();

    // Move cursor to the Axe slot and confirm.
    app.world_mut().resource_mut::<OotMenuState>().cursor = Item::Axe.index();
    press(&mut app, |f| f.select = true);
    app.update();

    assert!(
        app.world().get::<HeldItem>(player).is_some(),
        "equipping the axe attaches a HeldItem"
    );
    assert!(
        app.world().get::<StashedActionSet>(player).is_some(),
        "the base action set is stashed for restore on unequip"
    );
    assert!(
        app.world().resource::<OwnedItems>().is_equipped(Item::Axe),
        "ownership records the axe as equipped"
    );
    // The axe grants a melee verb.
    assert!(
        app.world().get::<ActionSet>(player).unwrap().melee.is_some(),
        "the axe's melee verb is overlaid onto the player's action set"
    );
}

#[test]
fn confirming_an_equipped_weapon_unequips_it() {
    let (mut app, player) = test_app();
    app.world_mut().resource_mut::<OwnedItems>().grant(Item::Axe, 1);
    press(&mut app, |f| f.inventory = true);
    app.update();
    app.world_mut().resource_mut::<OotMenuState>().cursor = Item::Axe.index();
    press(&mut app, |f| f.select = true);
    app.update(); // equip
    assert!(app.world().get::<HeldItem>(player).is_some());

    press(&mut app, |f| f.select = true);
    app.update(); // confirm again -> unequip

    assert!(
        app.world().get::<HeldItem>(player).is_none(),
        "confirming the equipped axe stows it (HeldItem removed)"
    );
    assert!(
        !app.world().resource::<OwnedItems>().is_equipped(Item::Axe),
        "ownership clears the equipped slot"
    );
}

#[test]
fn using_a_health_cell_spends_it_and_emits_a_heal() {
    let (mut app, _player) = test_app();
    app.world_mut()
        .resource_mut::<OwnedItems>()
        .grant(Item::HealthCell, 2);
    press(&mut app, |f| f.inventory = true);
    app.update();

    app.world_mut().resource_mut::<OotMenuState>().cursor = Item::HealthCell.index();
    press(&mut app, |f| f.select = true);
    app.update();

    assert_eq!(
        app.world().resource::<OwnedItems>().count(Item::HealthCell),
        1,
        "one health cell consumed"
    );
    assert_eq!(
        app.world().resource::<HealLog>().0,
        1,
        "a heal request was emitted"
    );
}

#[test]
fn grid_navigation_moves_the_cursor_while_open() {
    let (mut app, _player) = test_app();
    press(&mut app, |f| f.inventory = true);
    app.update();
    assert_eq!(app.world().resource::<OotMenuState>().cursor, 0);

    press(&mut app, |f| f.right = true);
    app.update();
    assert_eq!(
        app.world().resource::<OotMenuState>().cursor,
        1,
        "right moves one slot along the row"
    );

    press(&mut app, |f| f.down = true);
    app.update();
    assert_eq!(
        app.world().resource::<OotMenuState>().cursor,
        1 + crate::items::ITEM_GRID_COLS,
        "down moves one full row"
    );
}

#[test]
fn equipping_the_portal_gun_attaches_and_detaches_its_component() {
    let (mut app, player) = test_app();
    app.world_mut()
        .resource_mut::<OwnedItems>()
        .grant(Item::PortalGun, 1);
    press(&mut app, |f| f.inventory = true);
    app.update();

    // PortalGun is grid slot 0; confirm it.
    app.world_mut().resource_mut::<OotMenuState>().cursor = Item::PortalGun.index();
    press(&mut app, |f| f.select = true);
    app.update();
    assert!(
        app.world().get::<crate::portal::PortalGun>(player).is_some(),
        "equipping the portal gun attaches the PortalGun component"
    );
    assert!(app.world().resource::<OwnedItems>().is_equipped(Item::PortalGun));

    // Confirm again → unequip.
    press(&mut app, |f| f.select = true);
    app.update();
    assert!(
        app.world().get::<crate::portal::PortalGun>(player).is_none(),
        "confirming the equipped portal gun stows it"
    );
    assert!(!app.world().resource::<OwnedItems>().is_equipped(Item::PortalGun));
}

#[test]
fn confirming_an_unowned_slot_does_nothing() {
    let (mut app, player) = test_app();
    // Axe NOT granted.
    press(&mut app, |f| f.inventory = true);
    app.update();
    app.world_mut().resource_mut::<OotMenuState>().cursor = Item::Axe.index();
    press(&mut app, |f| f.select = true);
    app.update();
    assert!(
        app.world().get::<HeldItem>(player).is_none(),
        "an un-acquired weapon cannot be equipped"
    );
}
