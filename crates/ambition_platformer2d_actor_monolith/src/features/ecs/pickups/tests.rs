use super::*;
use crate::avatar::PlayerHealRequested;
use ambition_combat::components::FeatureId;
use ambition_platformer2d_core::BodyBaseSize;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use bevy::prelude::{App, Update};

fn player_at(app: &mut App, pos: ae::Vec2) -> bevy::prelude::Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
            BodyKinematics {
                pos,
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
            BodyBaseSize {
                base_size: ae::Vec2::new(28.0, 46.0),
            },
        ))
        .id()
}

fn health_pickup_at(app: &mut App, id: &str, pos: ae::Vec2) -> bevy::prelude::Entity {
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new(id),
            FeatureName::new("Health"),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
            PickupFeature::new(ambition_interaction::Pickup::new(
                id,
                ambition_interaction::PickupKind::Health { amount: 1 },
            )),
        ))
        .id()
}

#[test]
fn collect_marks_only_the_overlapping_pickup() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<SetFlagRequested>();
    app.add_systems(Update, collect_ecs_pickups);

    let center = ae::Vec2::new(64.0, 64.0);
    player_at(&mut app, center);
    let overlapping = health_pickup_at(&mut app, "hp_near", center);
    let distant = health_pickup_at(&mut app, "hp_far", ae::Vec2::new(1000.0, 1000.0));

    app.update();

    assert!(
        app.world().get::<Collected>(overlapping).is_some(),
        "a pickup the player overlaps should be Collected"
    );
    assert!(
        app.world().get::<Collected>(distant).is_none(),
        "a distant pickup should be left uncollected"
    );
}

#[test]
fn currency_pickup_credits_the_player_wallet() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<SetFlagRequested>();
    app.add_systems(Update, collect_ecs_pickups);

    let center = ae::Vec2::new(64.0, 64.0);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            ambition_characters::actor::BodyWallet::default(),
            BodyKinematics {
                pos: center,
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
            BodyBaseSize {
                base_size: ae::Vec2::new(28.0, 46.0),
            },
        ))
        .id();
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("coin"),
        FeatureName::new("Coin"),
        CenteredAabb::from_center_size(center, ae::Vec2::new(12.0, 12.0)),
        PickupFeature::new(ambition_interaction::Pickup::new(
            "coin",
            ambition_interaction::PickupKind::Currency { amount: 25 },
        )),
    ));

    app.update();
    assert_eq!(
        app.world()
            .get::<ambition_characters::actor::BodyWallet>(player)
            .unwrap()
            .balance,
        25,
        "collecting a currency pickup should credit the wallet"
    );
}

#[test]
fn collecting_an_ability_pickup_grants_it_to_the_catalog() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_items::OwnedItems::default());
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<SetFlagRequested>();
    app.add_systems(Update, collect_ecs_pickups);

    let center = ae::Vec2::new(64.0, 64.0);
    app.world_mut().spawn((
        PlayerEntity,
        ambition_characters::actor::BodyWallet::default(),
        BodyKinematics {
            pos: center,
            size: ae::Vec2::new(28.0, 46.0),
            facing: 1.0,
            ..Default::default()
        },
        BodyBaseSize {
            base_size: ae::Vec2::new(28.0, 46.0),
        },
    ));
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("ability_drop"),
        FeatureName::new("Blink"),
        CenteredAabb::from_center_size(center, ae::Vec2::new(16.0, 16.0)),
        PickupFeature::new(ambition_interaction::Pickup::new(
            "ability_drop",
            ambition_interaction::PickupKind::Ability {
                ability_id: "blink".to_string(),
            },
        )),
    ));

    app.update();
    assert!(
        app.world()
            .resource::<ambition_items::OwnedItems>()
            .has(ambition_items::Item::Blink),
        "collecting an ability pickup should grant it to the catalog",
    );
}

#[test]
fn collect_is_a_noop_with_no_player() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<SetFlagRequested>();
    app.add_systems(Update, collect_ecs_pickups);

    let pickup = health_pickup_at(&mut app, "hp", ae::Vec2::new(64.0, 64.0));
    app.update();
    assert!(
        app.world().get::<Collected>(pickup).is_none(),
        "with no player, nothing is collected"
    );
}

#[test]
fn a_pickup_that_declares_no_magnet_stays_where_it_landed() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 0.1,
        ..Default::default()
    });
    app.add_systems(Update, magnetize_pickups);
    player_at(&mut app, ae::Vec2::new(100.0, 100.0));
    // Well inside the CLASSIC range (dist 100 < 130), and carrying no magnet.
    let sitting = health_pickup_at(&mut app, "sitting", ae::Vec2::new(200.0, 100.0));
    app.update();
    assert_eq!(
        app.world().get::<CenteredAabb>(sitting).unwrap().center.x,
        200.0,
        "a pickup with no PickupMagnet must not drift — Mary-O's coins and \
         Sanic's rings are exactly this case",
    );
}

/// The magnet pulls toward the NEAREST collector, not toward "the player".
///
///  the old rule queried `With<PrimaryPlayer>` and `.single()`, so on a couch
/// every coin in the room flew at seat one — and with two players present it
/// would not have run at all.
#[test]
fn a_magnetized_pickup_goes_to_the_nearest_collector_of_several() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 0.1,
        ..Default::default()
    });
    app.add_systems(Update, magnetize_pickups);
    player_at(&mut app, ae::Vec2::new(0.0, 100.0));
    player_at(&mut app, ae::Vec2::new(260.0, 100.0));

    let pickup = health_pickup_at(&mut app, "contested", ae::Vec2::new(200.0, 100.0));
    app.world_mut()
        .entity_mut(pickup)
        .insert(super::PickupMagnet::classic());
    app.update();

    let x = app.world().get::<CenteredAabb>(pickup).unwrap().center.x;
    assert!(
        x > 200.0,
        "the pickup must move toward the NEARER body at x=260 (went to x={x})",
    );
}

/// ⛔⛔ TWO EQUIDISTANT COLLECTORS IS A COIN FLIP UNTIL SOMETHING BREAKS THE TIE.
///
/// `min_by` on distance alone keeps whichever candidate the query yields first,
/// which is archetype order — not a promise, and not what a rollback
/// resimulation reproduces. Which body a contested pickup flies to is
/// authoritative gameplay state, so deciding it by iteration order is
/// deterministically wrong.
///
/// ⭐ THE ARM THAT CATCHES IT IS SPAWN ORDER, exactly as the projectile-victim
/// tie-break's is: the same two bodies are spawned left-then-right and
/// right-then-left, and the pickup must go the SAME way both times. A single
/// arrangement agrees with the bug whenever the archetype happens to list the
/// winner first.
#[test]
fn a_pickup_between_two_equidistant_collectors_goes_the_same_way_whichever_spawned_first() {
    fn drift(left_first: bool) -> f32 {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime {
            scaled_dt: 0.1,
            ..Default::default()
        });
        app.add_systems(Update, magnetize_pickups);
        // EXACTLY equidistant, and both inside the classic 130px range.
        let left = ae::Vec2::new(100.0, 100.0);
        let right = ae::Vec2::new(300.0, 100.0);
        let (first, second) = if left_first {
            (left, right)
        } else {
            (right, left)
        };
        let a = player_at(&mut app, first);
        let b = player_at(&mut app, second);
        // ⭐ IDENTITIES, or the tie-break has nothing to break the tie WITH and
        // this test measures encounter order twice. The ids are fixed to the
        // POSITION, not to the spawn order, so "the same winner" means the same
        // body and not the same slot.
        for (entity, at) in [(a, first), (b, second)] {
            let id = if at.x < 200.0 { "left" } else { "right" };
            app.world_mut()
                .entity_mut(entity)
                .insert(ambition_platformer2d_shared_tangle::sim_id::SimId::placement(id));
        }

        let pickup = health_pickup_at(&mut app, "contested", ae::Vec2::new(200.0, 100.0));
        app.world_mut()
            .entity_mut(pickup)
            .insert(super::PickupMagnet::classic());
        app.update();
        app.world().get::<CenteredAabb>(pickup).unwrap().center.x
    }

    let a = drift(true);
    let b = drift(false);
    assert!(
        (a - 200.0).abs() > 1.0,
        "the pickup did not move at all, so this arm cannot tell one winner \
         from the other (x={a})"
    );
    assert_eq!(
        a, b,
        "the contested pickup went one way when the left collector was spawned \
         first and the other way when the right one was — the winner is \
         archetype order, which a resimulation does not reproduce"
    );
}

#[test]
fn nearby_pickups_drift_toward_the_player() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 0.1,
        ..Default::default()
    });
    app.add_systems(Update, magnetize_pickups);
    player_at(&mut app, ae::Vec2::new(100.0, 100.0));
    // In range (dist 100 < 130) -> drifts toward the collector (leftward).
    let near = health_pickup_at(&mut app, "near", ae::Vec2::new(200.0, 100.0));
    // Out of range (dist 400) -> unmoved.
    let far = health_pickup_at(&mut app, "far", ae::Vec2::new(500.0, 100.0));
    // Both DECLARE the classic magnet now — attraction is a pickup's policy.
    for pickup in [near, far] {
        app.world_mut()
            .entity_mut(pickup)
            .insert(super::PickupMagnet::classic());
    }
    app.update();
    let near_x = app.world().get::<CenteredAabb>(near).unwrap().center.x;
    let far_x = app.world().get::<CenteredAabb>(far).unwrap().center.x;
    assert!(
        near_x < 200.0,
        "the nearby pickup drifted toward the player (x={near_x})"
    );
    assert_eq!(far_x, 500.0, "the far pickup is out of magnet range");
}

/// ⛔⛔ WHO GETS THE RING, WHEN TWO PLAYERS ARE STANDING ON IT.
///
/// `collect_ecs_pickups` resolved this with `collectors.iter().find(..)`, and
/// the comment above it said "find the first overlapping collector" — which
/// reads like a rule and is not one. "First" is Bevy query order, i.e. archetype
/// order, and a resimulated tick can present the same two bodies in the other
/// one. Depending on the pickup that decides who heals, who banks the currency,
/// and who takes the flag.
mod who_gets_it {
    use super::*;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    fn identified_player_at(app: &mut App, slot: u8, pos: ae::Vec2) -> bevy::prelude::Entity {
        let entity = player_at(app, pos);
        app.world_mut()
            .entity_mut(entity)
            .insert(SimId::player_slot(slot));
        entity
    }

    /// Run one collection with the two collectors spawned in `order`, and report
    /// the winner's stable identity.
    fn winner_with_spawn_order(order: [u8; 2]) -> SimId {
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.add_message::<PlayerHealRequested>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<SetFlagRequested>();
        app.add_systems(Update, collect_ecs_pickups);

        let ring = ae::Vec2::new(64.0, 64.0);
        // EQUIDISTANT ON PURPOSE. The metric cannot separate them, so the answer
        // is entirely the tie-break — which is the half that was missing.
        let places = [
            ring + ae::Vec2::new(-6.0, 0.0),
            ring + ae::Vec2::new(6.0, 0.0),
        ];
        for (slot, place) in order.into_iter().zip(places) {
            identified_player_at(&mut app, slot, place);
        }
        let pickup = health_pickup_at(&mut app, "hp_contested", ring);

        app.update();

        assert!(
            app.world().get::<Collected>(pickup).is_some(),
            "nobody collected a pickup both bodies overlap, so this arm proves \
             nothing about who won"
        );
        let world = app.world_mut();
        let heals = world.resource_mut::<bevy::prelude::Messages<PlayerHealRequested>>();
        let mut cursor = heals.get_cursor();
        let target = cursor
            .read(&heals)
            .next()
            .and_then(|heal| heal.target)
            .expect("the heal is routed to the specific body that collected it");
        let world = app.world();
        world
            .get::<SimId>(target)
            .cloned()
            .expect("the winner carries the identity the tie-break used")
    }

    /// ⭐ THE PROPERTY. Reversing the order the two bodies were spawned in must
    /// not change who collects. Under `.iter().find(..)` it does — the winner
    /// follows archetype order, so this arm reads `slot:0` one way and `slot:1`
    /// the other.
    #[test]
    fn the_same_body_collects_whichever_order_the_two_were_spawned_in() {
        let forward = winner_with_spawn_order([0, 1]);
        let reversed = winner_with_spawn_order([1, 0]);
        assert_eq!(
            forward, reversed,
            "which of two equidistant bodies collected the pickup changed with \
             the order they were spawned in, so a resimulated tick can hand it \
             to the other player"
        );
        assert_eq!(
            forward,
            SimId::player_slot(0),
            "the tie-break is stable SimId, so the lower slot wins an exact tie"
        );
    }

    /// And the metric still comes first: a body that is genuinely nearer wins
    /// regardless of its identity, or the tie-break would have quietly become
    /// the whole rule.
    #[test]
    fn the_nearer_body_wins_even_with_the_higher_identity() {
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.add_message::<PlayerHealRequested>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<SetFlagRequested>();
        app.add_systems(Update, collect_ecs_pickups);

        let ring = ae::Vec2::new(64.0, 64.0);
        identified_player_at(&mut app, 0, ring + ae::Vec2::new(-12.0, 0.0));
        identified_player_at(&mut app, 1, ring);
        health_pickup_at(&mut app, "hp_contested", ring);

        app.update();

        let world = app.world_mut();
        let heals = world.resource::<bevy::prelude::Messages<PlayerHealRequested>>();
        let mut cursor = heals.get_cursor();
        let target = cursor
            .read(heals)
            .next()
            .and_then(|heal| heal.target)
            .expect("a heal was routed");
        assert_eq!(
            app.world().get::<SimId>(target).cloned(),
            Some(SimId::player_slot(1)),
            "the higher slot standing exactly on the pickup lost to a farther \
             body, so the identity tie-break is outranking the gameplay metric"
        );
    }
}
