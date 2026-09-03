use super::*;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_portal2d::arm_portal_pickups;

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
            // No PortalGun yet — the single pickup item grants it.
        ))
        .id();
    app.world_mut().spawn(PortalGunPickup {
        pos: Vec2::new(50.0, 50.0),
        half_extent: Vec2::splat(20.0),
        arm_timer: 0.0,
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

/// Holding it and the catalog saying so are ONE fact, so they move together.
///
/// `throw_held_item_system` cleared its slot on the equivalent release; this hand-written copy
/// of the same operation did not, and nothing could tell them apart because each caller kept
/// its own copy.
///
/// Both ends are asserted at BOTH moments deliberately: a release that cleared
/// the slot unconditionally would satisfy the drop half and fail the pickup half,
/// and the release that shipped (component only) fails the drop half. The third
/// assertion is the poison for over-correcting — releasing custody must not also
/// take the item away, because owning a gun and holding one are different facts.
#[test]
fn dropping_the_gun_clears_the_catalog_slot_that_picking_it_up_set() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<DropPortalGun>();
    app.add_message::<PickUpPortalGun>();
    app.add_message::<PortalGunEquipped>();
    // The catalog the two systems keep in step. Absent in the sibling tests, so
    // this is the only one that can see the slot at all.
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
            ambition_characters::control::ActorControl::default(),
            // No PortalGun yet — the world pickup is what grants it.
        ))
        .id();
    app.world_mut().spawn(PortalGunPickup {
        pos: Vec2::new(50.0, 50.0),
        half_extent: Vec2::splat(20.0),
        arm_timer: 0.0,
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

    // Immediately a pickup intent while overlapping — the freshly-dropped
    // pickup is still arming, so it must NOT be re-grabbed (the bug).
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

/// TWO SEATS ACT ON ONE TICK, and both are answered.
///
/// ⛔⛔ `read().next()` SERVED ONE INTENT AND LEFT THE REST. The intents already
/// name their bodies — that was fixed when a second seat's drop dropped the
/// FIRST seat's gun — but taking only the head of the queue meant two seats
/// acting on the same tick were SERIALIZED ACROSS UPDATES, the second landing in
/// a world the first had already changed. Found by the 2026-08-31 GPT review.
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

/// TWO SEATS REACH FOR ONE GUN, and exactly one gets it.
///
/// ⭐ THE WINNER IS MESSAGE ORDER, and it is definite: the first intent that
/// overlaps an ARMED pickup despawns it, so the second finds nothing. There is
/// one portal gun in the world and there is one after two people grab for it —
/// serving every intent must not mint a second.
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
