//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::avatar::PlayerHealRequested;
use ambition_engine_core::BodyBaseSize;
use ambition_engine_core::BodyKinematics;
use ambition_platformer_primitives::markers::PlayerEntity;
use bevy::prelude::{App, Update};

fn player_at(app: &mut App, pos: ae::Vec2) -> bevy::prelude::Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            ambition_platformer_primitives::markers::PrimaryPlayer,
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
    app.add_message::<SfxMessage>();
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
    app.add_message::<SfxMessage>();
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
    app.add_message::<SfxMessage>();
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
    app.add_message::<SfxMessage>();
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
fn nearby_pickups_drift_toward_the_player() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 0.1,
        ..Default::default()
    });
    app.add_systems(Update, magnetize_pickups);
    player_at(&mut app, ae::Vec2::new(100.0, 100.0));
    // In range (dist 100 < 130) -> drifts toward the player (leftward).
    let near = health_pickup_at(&mut app, "near", ae::Vec2::new(200.0, 100.0));
    // Out of range (dist 400) -> unmoved.
    let far = health_pickup_at(&mut app, "far", ae::Vec2::new(500.0, 100.0));
    app.update();
    let near_x = app.world().get::<CenteredAabb>(near).unwrap().center.x;
    let far_x = app.world().get::<CenteredAabb>(far).unwrap().center.x;
    assert!(
        near_x < 200.0,
        "the nearby pickup drifted toward the player (x={near_x})"
    );
    assert_eq!(far_x, 500.0, "the far pickup is out of magnet range");
}
