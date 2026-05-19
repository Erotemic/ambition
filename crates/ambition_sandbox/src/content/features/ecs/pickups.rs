//! Player → pickup collection on the ECS feature path.

use super::*;

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    player: Query<&crate::player::PlayerBody, With<crate::player::PlayerEntity>>,
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
        // Iterate every player so the first player to touch the
        // pickup collects it. Single-player behavior preserved (one
        // entity in the iterator). For a future co-op build the heal
        // amount + banner / SFX / VFX would ideally target the
        // collector specifically (OVERNIGHT-TODO #17.6); today the
        // heal message is implicitly the primary player.
        if !player
            .iter()
            .any(|pb| aabb.aabb().strict_intersects(pb.aabb()))
        {
            continue;
        }
        commands.entity(entity).insert(Collected);
        banner.show(format!("picked up {}", name.0.as_str()), 2.6);
        if let ae::PickupKind::Health { amount } = &pickup.pickup.kind {
            heals.write(crate::player::PlayerHealRequested::new(*amount));
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
