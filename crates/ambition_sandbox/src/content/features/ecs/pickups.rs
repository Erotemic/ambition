//! Player → pickup collection on the ECS feature path.

use super::*;
use crate::features::SetFlagRequested;

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    player: Query<(Entity, &crate::player::PlayerKinematics), With<crate::player::PlayerEntity>>,
    pickups: Query<
        (
            Entity,
            &FeatureName,
            &FeatureAabb,
            &PickupFeature,
            Option<&Collected>,
        ),
        With<FeatureSimEntity>,
    >,
    mut heals: MessageWriter<crate::player::PlayerHealRequested>,
    mut wallets: Query<&mut crate::player::PlayerWallet>,
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
            crate::interaction::PickupKind::Health { amount } => {
                heals.write(crate::player::PlayerHealRequested::for_target(
                    *amount,
                    collector_entity,
                ));
            }
            crate::interaction::PickupKind::Currency { amount } => {
                // Credit the collecting player's wallet (HUD money meter).
                if let Ok(mut wallet) = wallets.get_mut(collector_entity) {
                    wallet.add(*amount);
                }
            }
            crate::interaction::PickupKind::Ability { ability_id } => {
                // Grant the ability into the player's catalog so it shows up in
                // the OoT inventory and can be equipped (wired abilities) — the
                // Metroidvania "learn a power from a boss" beat.
                if let Some(owned) = owned.as_deref_mut() {
                    if let Some(item) = crate::items::Item::from_dialog_id(ability_id) {
                        owned.grant(item, 1);
                    }
                }
            }
            crate::interaction::PickupKind::StoryFlag { flag } => {
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
            crate::interaction::PickupKind::Health { .. } => {
                ambition_sfx::ids::WORLD_HEALTH_COLLECT
            }
            crate::interaction::PickupKind::Currency { .. } => ambition_sfx::ids::WORLD_COIN_PICKUP,
            _ => ambition_sfx::ids::WORLD_PICKUP_GENERIC,
        };
        sfx.write(SfxMessage::Play { id, pos });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{PlayerEntity, PlayerHealRequested, PlayerKinematics};
    use bevy::prelude::{App, Update};

    fn player_at(app: &mut App, pos: ae::Vec2) -> bevy::prelude::Entity {
        app.world_mut()
            .spawn((
                PlayerEntity,
                PlayerKinematics {
                    pos,
                    size: ae::Vec2::new(28.0, 46.0),
                    base_size: ae::Vec2::new(28.0, 46.0),
                    facing: 1.0,
                    ..Default::default()
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
                FeatureAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
                PickupFeature::new(crate::interaction::Pickup::new(
                    id,
                    crate::interaction::PickupKind::Health { amount: 1 },
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
                crate::player::PlayerWallet::default(),
                PlayerKinematics {
                    pos: center,
                    size: ae::Vec2::new(28.0, 46.0),
                    base_size: ae::Vec2::new(28.0, 46.0),
                    facing: 1.0,
                    ..Default::default()
                },
            ))
            .id();
        app.world_mut().spawn((
            FeatureSimEntity,
            FeatureId::new("coin"),
            FeatureName::new("Coin"),
            FeatureAabb::from_center_size(center, ae::Vec2::new(12.0, 12.0)),
            PickupFeature::new(crate::interaction::Pickup::new(
                "coin",
                crate::interaction::PickupKind::Currency { amount: 25 },
            )),
        ));

        app.update();
        assert_eq!(
            app.world()
                .get::<crate::player::PlayerWallet>(player)
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
            crate::player::PlayerWallet::default(),
            PlayerKinematics {
                pos: center,
                size: ae::Vec2::new(28.0, 46.0),
                base_size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
        ));
        app.world_mut().spawn((
            FeatureSimEntity,
            FeatureId::new("ability_drop"),
            FeatureName::new("Blink"),
            FeatureAabb::from_center_size(center, ae::Vec2::new(16.0, 16.0)),
            PickupFeature::new(crate::interaction::Pickup::new(
                "ability_drop",
                crate::interaction::PickupKind::Ability {
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
}
