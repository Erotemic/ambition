//! Player → pickup collection on the ECS feature path.

use super::*;
use crate::features::events::GameplayEffect;

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
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
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
            crate::interaction::PickupKind::StoryFlag { flag } => {
                // PickupSpawn entities with `kind: "flag:<id>"` set
                // the named flag in the save layer and emit a
                // QuestAdvanceEvent::FlagSet via apply_flag_effects.
                // Mirrors the LockWall/Switch flag-setting pattern so
                // intro-v1 cartography pickups and similar narrative
                // story-flag drops just work without per-pickup wiring.
                gameplay_effects.write(GameplayEffect::SetFlag {
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
