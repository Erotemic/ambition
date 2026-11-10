//! `apply_hand_requests`: a menu's equip and stow reach the hand of the body
//! the requesting participant drives, in the simulation, and no other hand.

use super::*;
use ambition_characters::brain::{action_set::IdentityKit, ActionSet};
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_combat::moveset::ActorMoveset;
use ambition_items::{Item, ItemEquipRequested, ItemStowRequested, OwnedItems};

/// Two bodies, driven by participants 0 and 1, and an applier that owns a
/// gun-sword and a portal gun.
fn hands_app() -> (App, [Entity; 2]) {
    let mut app = App::new();
    let catalog = ambition_items::test_item_catalog().clone();
    let mut owned = OwnedItems::default();
    owned.grant(&catalog, Item::GunSword, 1);
    owned.grant(&catalog, Item::PortalGun, 1);
    app.insert_resource(catalog);
    app.insert_resource(owned);
    app.add_message::<ItemEquipRequested>();
    app.add_message::<ItemStowRequested>();
    app.add_systems(Update, apply_hand_requests);
    let bodies = [PlayerSlot(0), PlayerSlot(1)].map(|slot| {
        app.world_mut()
            .spawn((DrivingParticipant(slot), IdentityKit::default(), ActionSet::default(), ActorMoveset::default()))
            .id()
    });
    (app, bodies)
}

fn equip(app: &mut App, participant: PlayerSlot, item: Item) {
    app.world_mut().write_message(ItemEquipRequested { participant, item });
    app.update();
}

fn stow(app: &mut App, participant: PlayerSlot) {
    app.world_mut().write_message(ItemStowRequested { participant });
    app.update();
}

#[test]
fn an_equip_and_a_stow_change_the_named_participants_hand_only() {
    for participant in [PlayerSlot(0), PlayerSlot(1)] {
        let (mut app, bodies) = hands_app();
        let subject = bodies[participant.0 as usize];
        let other = bodies[1 - participant.0 as usize];
        equip(&mut app, participant, Item::GunSword);
        assert_eq!(app.world().get::<HeldItem>(subject).map(HeldItem::id), Some("gun_sword"));
        assert!(app.world().get::<HeldItem>(other).is_none());
        stow(&mut app, participant);
        assert!(app.world().get::<HeldItem>(subject).is_none());
        assert!(app.world().get::<HeldItem>(other).is_none());
    }
}

#[test]
fn a_missing_or_ambiguous_driver_changes_no_hand() {
    for ambiguous in [false, true] {
        let (mut app, bodies) = hands_app();
        app.world_mut().entity_mut(bodies[1]).remove::<DrivingParticipant>();
        if ambiguous {
            app.world_mut().entity_mut(bodies[0]).insert(DrivingParticipant(PlayerSlot(1)));
            app.world_mut().entity_mut(bodies[1]).insert(DrivingParticipant(PlayerSlot(1)));
        }
        equip(&mut app, PlayerSlot(1), Item::GunSword);
        for body in bodies {
            assert!(app.world().get::<HeldItem>(body).is_none());
        }
    }
}

#[test]
fn an_item_not_owned_and_an_ability_are_not_put_in_a_hand() {
    let (mut app, bodies) = hands_app();
    app.insert_resource(OwnedItems::default());
    equip(&mut app, PlayerSlot(0), Item::GunSword);
    assert!(app.world().get::<HeldItem>(bodies[0]).is_none(), "not owned");
    let mut owned = OwnedItems::default();
    owned.grant(ambition_items::test_item_catalog(), Item::SwordStance, 1);
    app.insert_resource(owned);
    equip(&mut app, PlayerSlot(0), Item::SwordStance);
    assert!(app.world().get::<HeldItem>(bodies[0]).is_none(), "an ability is worn, not held");
}

#[cfg(feature = "portal")]
#[test]
fn a_portal_gun_keeps_the_named_bodys_pair_when_equipped_again() {
    let (mut app, bodies) = hands_app();
    app.world_mut().entity_mut(bodies[0]).insert(OwnedPortalGunPair(2));
    app.world_mut().entity_mut(bodies[1]).insert(OwnedPortalGunPair(7));
    equip(&mut app, PlayerSlot(1), Item::PortalGun);
    assert!(app.world().get::<PortalGun>(bodies[0]).is_none());
    let gun = app.world().get::<PortalGun>(bodies[1]).expect("the gun is in the hand");
    assert!(gun.active);
    assert_eq!(gun.pair(), 7);
    stow(&mut app, PlayerSlot(1));
    assert!(app.world().get::<PortalGun>(bodies[1]).is_none());
    assert_eq!(app.world().get::<OwnedPortalGunPair>(bodies[1]).map(|pair| pair.0), Some(7));
}
