use super::*;
use ambition_characters::brain::action_set::{ActionSet, IdentityKit};
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal2d::{arm_portal_pickups, PortalGunColor};

fn spawn_player(app: &mut App, pos: Vec2, facing: f32) -> Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos,
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            PortalGun::default(),
            ActionSet::default(),
            IdentityKit::default(),
            ambition_combat::moveset::ActorMoveset::default(),
            // Production bodies carry an intent frame; the drop spends the
            // Attack press on it when it commits.
            ambition_characters::control::ActorControl::default(),
        ))
        .id()
}

#[test]
fn picking_up_the_portal_gun_activates_it() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<PickUpPortalGun>();
    app.add_message::<PortalGunEquipped>();
    app.add_systems(Update, pickup_portal_gun_system);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: Vec2::new(50.0, 50.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            ActionSet::default(),
            IdentityKit::default(),
            ambition_combat::moveset::ActorMoveset::default(),
            // No PortalGun yet — the single pickup item grants it.
        ))
        .id();
    app.world_mut().spawn(PortalGunPickup {
        pos: Vec2::new(50.0, 50.0),
        half_extent: Vec2::splat(20.0),
        arm_timer: 0.0,
        pair: 0,
    });
    assert!(app.world().get::<PortalGun>(player).is_none());

    app.world_mut()
        .write_message(PickUpPortalGun { body: player });
    app.update();
    assert!(
        app.world()
            .get::<PortalGun>(player)
            .is_some_and(|g| g.active),
        "a pickup intent while overlapping grants the active gun"
    );
    let remaining = {
        let mut q = app.world_mut().query::<&PortalGunPickup>();
        q.iter(app.world()).count()
    };
    assert_eq!(remaining, 0, "the pickup is consumed");
}

/// `PortalGunEquipped` has no reader in this tree, so its emission is
/// asserted here.
///
/// It is a published notification ("for anyone who wants to react to it",
/// like `PulseFired`) with a rollback schema row
/// (`message.portal_gun_equipped`), so it has a cost. With no in-tree reader,
/// only this test stops the write in `inventory_adapter.rs` from being removed
/// silently.
///
/// The `player` field is asserted, not only the count: a notification that
/// names the wrong body is worse than none.
#[test]
fn picking_up_the_gun_announces_who_equipped_it() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<PickUpPortalGun>();
    app.add_message::<PortalGunEquipped>();
    app.add_systems(Update, pickup_portal_gun_system);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: Vec2::new(50.0, 50.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            ActionSet::default(),
            IdentityKit::default(),
            ambition_combat::moveset::ActorMoveset::default(),
        ))
        .id();
    app.world_mut().spawn(PortalGunPickup {
        pos: Vec2::new(50.0, 50.0),
        half_extent: Vec2::splat(20.0),
        arm_timer: 0.0,
        pair: 0,
    });

    // A quiet frame first: the announcement is caused by the pickup, not by the
    // system merely running.
    app.update();
    let quiet = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<PortalGunEquipped>>()
        .drain()
        .count();
    assert_eq!(
        quiet, 0,
        "no pickup intent, so nothing was equipped"
    );

    app.world_mut()
        .write_message(PickUpPortalGun { body: player });
    app.update();
    // Compared by field: `PortalGunEquipped` derives no `PartialEq`, and a test
    // must not change a rollback-registered wire type to suit itself.
    let announced: Vec<Entity> = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<PortalGunEquipped>>()
        .drain()
        .map(|message| message.player)
        .collect();
    assert_eq!(
        announced,
        vec![player],
        "equipping the gun announces the body that got it"
    );
}

/// Holding it and the catalog saying so are one fact, so they move together,
/// as in `throw_held_item_system`.
///
/// Both ends are asserted at both moments. A release that cleared the slot
/// unconditionally would pass the drop half and fail the pickup half; a
/// release that removed only the component fails the drop half. The third
/// assertion catches over-correction: releasing custody must not remove the
/// item, because owning a gun and holding one are different facts.
#[test]
fn dropping_the_gun_clears_the_catalog_slot_that_picking_it_up_set() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<DropPortalGun>();
    app.add_message::<PickUpPortalGun>();
    app.add_message::<PortalGunEquipped>();
    // The catalog the two systems keep in step. The sibling tests omit it, so
    // only this one can see the slot.
    app.insert_resource(OwnedItems::default());
    app.add_systems(
        Update,
        (drop_portal_gun_system, pickup_portal_gun_system).chain(),
    );
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos: Vec2::new(50.0, 50.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            ActionSet::default(),
            IdentityKit::default(),
            ambition_combat::moveset::ActorMoveset::default(),
            ambition_characters::control::ActorControl::default(),
            // No PortalGun yet — the world pickup is what grants it.
        ))
        .id();
    app.world_mut().spawn(PortalGunPickup {
        pos: Vec2::new(50.0, 50.0),
        half_extent: Vec2::splat(20.0),
        arm_timer: 0.0,
        pair: 0,
    });

    // TAKE custody.
    app.world_mut()
        .write_message(PickUpPortalGun { body: player });
    app.update();
    assert!(
        app.world().get::<PortalGun>(player).is_some(),
        "the pickup grants the gun"
    );
    // The hand IS the catalog's view of it (I1): no slot to name any more.
    assert_eq!(
        ambition_held_items::item_in_hand(
            None,
            app.world().get::<PortalGun>(player),
        ),
        Some(Item::PortalGun),
        "an active gun in the hand reads as the equipped PortalGun"
    );

    // RELEASE custody: both ends move, or the transfer is not one.
    app.world_mut()
        .write_message(DropPortalGun { body: player });
    app.update();
    assert!(
        app.world().get::<PortalGun>(player).is_none(),
        "the drop detaches the gun"
    );
    assert_eq!(
        ambition_held_items::item_in_hand(
            None,
            app.world().get::<PortalGun>(player),
        ),
        None,
        "and an empty hand reads as nothing equipped — a gun on the floor is not an equipped gun"
    );
    assert!(
        app.world().resource::<OwnedItems>().has(Item::PortalGun),
        "but OWNING it survives the drop: entitlement is not custody, and the \
         release must not quietly take the item away"
    );
}

#[test]
fn dropped_portal_gun_arms_before_it_can_be_regrabbed() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<DropPortalGun>();
    app.add_message::<PickUpPortalGun>();
    app.add_message::<PortalGunEquipped>();
    app.insert_resource(ambition_platformer2d_shared_tangle::time::SimDt { dt: 1.0 / 60.0 });
    app.add_systems(
        Update,
        (
            drop_portal_gun_system,
            arm_portal_pickups,
            pickup_portal_gun_system,
        )
            .chain(),
    );
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0), 1.0);

    // Drop intent drops the gun.
    app.world_mut()
        .write_message(DropPortalGun { body: player });
    app.update();
    assert!(
        app.world().get::<PortalGun>(player).is_none(),
        "a drop intent should drop the portal gun"
    );

    // Move the player directly onto the dropped pickup so only the arm
    // timer (not distance) guards against a re-grab.
    let pickup_pos = {
        let mut q = app.world_mut().query::<&PortalGunPickup>();
        q.iter(app.world())
            .next()
            .expect("a pickup was dropped")
            .pos
    };
    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .pos = pickup_pos;

    // A pickup intent right away while overlapping: the fresh pickup is still
    // arming, so it must not be re-grabbed.
    app.world_mut()
        .write_message(PickUpPortalGun { body: player });
    app.update();
    assert!(
        app.world().get::<PortalGun>(player).is_none(),
        "an armed (just-dropped) pickup can't be re-grabbed on the next intent"
    );

    // Let it disarm, then a pickup intent picks it back up.
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .write_message(PickUpPortalGun { body: player });
    app.update();
    assert!(
        app.world().get::<PortalGun>(player).is_some(),
        "once disarmed, a pickup intent while overlapping re-grabs the gun"
    );
}

/// A gun's colour pair survives the floor.
///
/// The pair makes one gun orange/blue and another red/yellow, and a gun spends
/// part of its life as a world pickup. If the pickup lost the pair, every drop
/// would reset a non-default gun to the classic one.
#[test]
fn a_dropped_gun_keeps_its_own_pair_when_picked_back_up() {
    const PAIR: u8 = 5;
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<DropPortalGun>();
    app.add_message::<PickUpPortalGun>();
    app.add_message::<PortalGunEquipped>();
    app.insert_resource(ambition_platformer2d_shared_tangle::time::SimDt { dt: 1.0 / 60.0 });
    app.add_systems(
        Update,
        (
            drop_portal_gun_system,
            arm_portal_pickups,
            pickup_portal_gun_system,
        )
            .chain(),
    );
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0), 1.0);
    // Not the default gun: this body holds pair 5.
    app.world_mut()
        .entity_mut(player)
        .insert(PortalGun::for_pair(PAIR));
    // Toggle it to the B end first, so the test also pins that the pickup
    // carries the pair, not the end the holder was on.
    app.world_mut().get_mut::<PortalGun>(player).unwrap().next_color =
        PortalGunColor::for_pair(PAIR).other();

    app.world_mut()
        .write_message(DropPortalGun { body: player });
    app.update();

    let dropped = {
        let mut q = app.world_mut().query::<&PortalGunPickup>();
        *q.iter(app.world()).next().expect("a pickup was dropped")
    };
    assert_eq!(
        dropped.pair, PAIR,
        "the dropped pickup forgot which gun it was"
    );

    app.world_mut()
        .get_mut::<BodyKinematics>(player)
        .unwrap()
        .pos = dropped.pos;
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .write_message(PickUpPortalGun { body: player });
    app.update();

    let regrabbed = app
        .world()
        .get::<PortalGun>(player)
        .expect("the gun came back");
    assert_eq!(
        regrabbed.pair(),
        PAIR,
        "picking the gun back up handed over a different gun"
    );
    // And it is still a two-colour gun, on ITS pair.
    assert_eq!(regrabbed.next_color.other().pair(), PAIR);
}

/// Two seats act on one tick, and both are answered.
///
/// Taking only the head of the queue (`read().next()`) would serialize two
/// seats across updates, so the second would land in a world the first had
/// already changed.
#[test]
fn two_seats_dropping_on_one_tick_both_drop() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<DropPortalGun>();
    app.add_systems(Update, drop_portal_gun_system);
    let one = spawn_player(&mut app, Vec2::new(0.0, 0.0), 1.0);
    let two = spawn_player(&mut app, Vec2::new(300.0, 0.0), -1.0);

    app.world_mut().write_message(DropPortalGun { body: one });
    app.world_mut().write_message(DropPortalGun { body: two });
    app.update();

    assert!(
        app.world().get::<PortalGun>(one).is_none(),
        "the first seat kept its gun"
    );
    assert!(
        app.world().get::<PortalGun>(two).is_none(),
        "the SECOND seat's drop was left in the queue — one tick, two intents, \
         and only the head of the queue was served"
    );
    // …and both left a pickup behind, rather than one drop overwriting the other.
    let mut pickups = app.world_mut().query::<&PortalGunPickup>();
    let world = app.world();
    assert_eq!(pickups.iter(world).count(), 2);
}

/// Two seats reach for one gun, and exactly one gets it.
///
/// Message order decides: the first intent that overlaps an armed pickup
/// despawns it, so the second finds nothing. Serving every intent must not
/// mint a second gun.
#[test]
fn two_seats_grabbing_one_gun_produce_exactly_one_gun() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<PickUpPortalGun>();
    app.add_message::<PortalGunEquipped>();
    app.add_systems(Update, pickup_portal_gun_system);
    let bare = |app: &mut App, pos: Vec2| {
        app.world_mut()
            .spawn((
                PlayerEntity,
                BodyKinematics {
                    pos,
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                BodyBaseSize {
                    base_size: Vec2::new(24.0, 40.0),
                },
                ActionSet::default(),
                IdentityKit::default(),
                ambition_combat::moveset::ActorMoveset::default(),
            ))
            .id()
    };
    // Both bodies overlap the same pickup.
    let one = bare(&mut app, Vec2::new(50.0, 50.0));
    let two = bare(&mut app, Vec2::new(52.0, 50.0));
    app.world_mut().spawn(PortalGunPickup {
        pos: Vec2::new(50.0, 50.0),
        half_extent: Vec2::splat(20.0),
        arm_timer: 0.0,
        pair: 0,
    });

    app.world_mut().write_message(PickUpPortalGun { body: one });
    app.world_mut().write_message(PickUpPortalGun { body: two });
    app.update();

    let armed = [one, two]
        .iter()
        .filter(|body| app.world().get::<PortalGun>(**body).is_some())
        .count();
    assert_eq!(
        armed, 1,
        "two bodies reached for one gun and {armed} came away armed"
    );
    assert!(
        app.world().get::<PortalGun>(one).is_some(),
        "the winner should be the first INTENT, not whichever body the query \
         yielded first"
    );
    let mut pickups = app.world_mut().query::<&PortalGunPickup>();
    let world = app.world();
    assert_eq!(pickups.iter(world).count(), 0, "the world item survived");
}
