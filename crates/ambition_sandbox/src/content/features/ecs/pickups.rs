//! Player → pickup collection on the ECS feature path.

use super::*;

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    player: Query<(Entity, &crate::player::PlayerBody), With<crate::player::PlayerEntity>>,
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
            .find(|(_, pb)| aabb.aabb().strict_intersects(pb.aabb()))
        else {
            continue;
        };
        commands.entity(entity).insert(Collected);
        banner.show(format!("picked up {}", name.0.as_str()), 2.6);
        if let ae::PickupKind::Health { amount } = &pickup.pickup.kind {
            heals.write(crate::player::PlayerHealRequested::for_target(
                *amount,
                collector_entity,
            ));
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
            ae::PickupKind::Health { .. } => ambition_sfx::ids::WORLD_HEALTH_COLLECT,
            ae::PickupKind::Currency { .. } => ambition_sfx::ids::WORLD_COIN_PICKUP,
            _ => ambition_sfx::ids::WORLD_PICKUP_GENERIC,
        };
        sfx.write(SfxMessage::Play { id, pos });
    }
}
