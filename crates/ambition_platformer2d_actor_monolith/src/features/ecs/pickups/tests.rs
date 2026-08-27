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
    app.insert_resource(crate::items::OwnedItems::default());
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
            .resource::<crate::items::OwnedItems>()
            .has(crate::items::Item::Blink),
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
