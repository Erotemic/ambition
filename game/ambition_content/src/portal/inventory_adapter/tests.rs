use bevy::prelude::*;

use ambition_characters::brain::HeldItemSpec;
use ambition_combat::held_items::HeldItem;
use ambition_held_items::GroundItem;
use ambition_portal2d::{OwnedPortalPair, PortalGenerator};

use super::reconcile_portal_generators;

fn gun() -> HeldItemSpec {
    ambition_characters::brain::held_item_by_id("portal_gun").expect("the portal gun is a held item")
}

fn app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_systems(Update, reconcile_portal_generators);
    app
}

/// A body is a generator exactly while the gun is in its hand, and it keeps
/// the pair after the gun leaves it. Another held item is the control.
#[test]
fn a_body_holding_the_gun_is_a_generator_and_keeps_its_pair_after() {
    let mut app = app();
    let body = app.world_mut().spawn((HeldItem::new(gun()), OwnedPortalPair(3))).id();
    let axe = app.world_mut().spawn(HeldItem::new(ambition_held_items::axe_spec())).id();
    app.update();
    assert_eq!(app.world().get::<PortalGenerator>(body).map(|g| g.pair()), Some(3), "it took up the pair it owned");
    assert!(app.world().get::<PortalGenerator>(axe).is_none(), "an axe opens no portals");

    app.world_mut().entity_mut(body).remove::<HeldItem>();
    app.update();
    assert!(app.world().get::<PortalGenerator>(body).is_none(), "the emptied hand still opened portals");
    assert_eq!(app.world().get::<OwnedPortalPair>(body), Some(&OwnedPortalPair(3)));
}

/// A gun lying in the world is a generator, so its pair's portals stay open
/// while it is on the floor (`despawn_orphaned_portals`).
#[test]
fn a_gun_on_the_floor_is_a_generator() {
    let mut app = app();
    let lying = app.world_mut().spawn(GroundItem::at_rest(gun(), Vec2::ZERO, Vec2::splat(10.0))).id();
    let axe = app
        .world_mut()
        .spawn(GroundItem::at_rest(ambition_held_items::axe_spec(), Vec2::ZERO, Vec2::splat(10.0)))
        .id();
    app.update();
    assert!(app.world().get::<PortalGenerator>(lying).is_some());
    assert!(app.world().get::<PortalGenerator>(axe).is_none());
}
