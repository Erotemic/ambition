//! Player → pickup collection on the ECS feature path.

use super::*;
use crate::features::SetFlagRequested;
use ambition_sfx::SfxMessage;

/// Range within which dropped loot drifts toward the player — a coin/loot magnet,
/// so the coins + hearts this run drops come to you instead of needing a pixel-
/// perfect walk-over.
const PICKUP_MAGNET_RANGE: f32 = 130.0;
/// How fast magnetized pickups close on the player (px/s).
const PICKUP_MAGNET_SPEED: f32 = 340.0;

/// Pull nearby uncollected pickups toward the player. Runs before
/// [`collect_ecs_pickups`], which still does the actual overlap grant — a pickup
/// pulled into overlap is collected the same frame.
pub fn magnetize_pickups(
    time: Res<ambition_time::WorldTime>,
    players: Query<
        &ambition_engine_core::BodyKinematics,
        With<ambition_platformer_primitives::markers::PrimaryPlayer>,
    >,
    mut pickups: Query<&mut CenteredAabb, (With<PickupFeature>, Without<Collected>)>,
) {
    let dt = time.scaled_dt;
    let Ok(player) = players.single() else {
        return;
    };
    for mut aabb in &mut pickups {
        let to_player = player.pos - aabb.center;
        let dist = to_player.length();
        if dist > 1.0 && dist < PICKUP_MAGNET_RANGE {
            aabb.center += to_player.normalize() * (PICKUP_MAGNET_SPEED * dt).min(dist);
        }
    }
}

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    player: Query<
        (Entity, &ambition_engine_core::BodyKinematics),
        With<ambition_platformer_primitives::markers::PlayerEntity>,
    >,
    pickups: Query<
        (
            Entity,
            &FeatureName,
            &CenteredAabb,
            &PickupFeature,
            Option<&Collected>,
        ),
        With<FeatureSimEntity>,
    >,
    mut heals: MessageWriter<crate::player::PlayerHealRequested>,
    mut wallets: Query<&mut ambition_characters::actor::BodyWallet>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut set_flag: MessageWriter<SetFlagRequested>,
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
) {
    if player.is_empty() {
        return;
    }
    for (entity, name, aabb, pickup, collected) in &pickups {
        if collected.is_some() {
            continue;
        }
        // Find the first overlapping player. The heal is then routed
        // to that specific player via `PlayerHealRequested::target` so
        // a non-primary collector still actually heals themselves
        // (OVERNIGHT-TODO #17.6 bridge). Single-player behavior is
        // unchanged: the iterator has one entity, and the target ==
        // primary fallback path lands the heal on the same player.
        let Some((collector_entity, _)) = player
            .iter()
            .find(|(_, kin)| aabb.aabb().strict_intersects(kin.aabb()))
        else {
            continue;
        };
        commands.entity(entity).insert(Collected);
        banner.show(format!("picked up {}", name.0.as_str()), 2.6);
        match &pickup.pickup.kind {
            ambition_interaction::PickupKind::Health { amount } => {
                heals.write(crate::player::PlayerHealRequested::for_target(
                    *amount,
                    collector_entity,
                ));
            }
            ambition_interaction::PickupKind::Currency { amount } => {
                // Credit the collecting player's wallet (HUD money meter).
                if let Ok(mut wallet) = wallets.get_mut(collector_entity) {
                    wallet.add(*amount);
                }
            }
            ambition_interaction::PickupKind::Ability { ability_id } => {
                // Grant the ability into the player's catalog so it shows up in
                // the OoT inventory and can be equipped (wired abilities) — the
                // Metroidvania "learn a power from a boss" beat.
                if let Some(owned) = owned.as_deref_mut() {
                    if let Some(item) = crate::items::Item::from_dialog_id(ability_id) {
                        owned.grant(item, 1);
                    }
                }
            }
            ambition_interaction::PickupKind::StoryFlag { flag } => {
                // PickupSpawn entities with `kind: "flag:<id>"` set
                // the named flag in the save layer and emit a
                // QuestAdvanceEvent::FlagSet via apply_flag_effects.
                // Mirrors the LockWall/Switch flag-setting pattern so
                // intro-v1 cartography pickups and similar narrative
                // story-flag drops just work without per-pickup wiring.
                set_flag.write(SetFlagRequested {
                    id: flag.clone(),
                    on: true,
                });
            }
            _ => {}
        }
        let pos = aabb.center;
        vfx.write(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
        let id = match &pickup.pickup.kind {
            ambition_interaction::PickupKind::Health { .. } => {
                ambition_sfx::ids::WORLD_HEALTH_COLLECT
            }
            ambition_interaction::PickupKind::Currency { .. } => {
                ambition_sfx::ids::WORLD_COIN_PICKUP
            }
            _ => ambition_sfx::ids::WORLD_PICKUP_GENERIC,
        };
        sfx.write(SfxMessage::Play { id, pos });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::PlayerHealRequested;
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
}
