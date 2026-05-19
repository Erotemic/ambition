//! Player → static-chest open path on the ECS feature side.

use super::*;

/// Open ECS-owned static chests from the same interaction buffer used by doors
/// and legacy NPCs/switches.
pub fn open_ecs_chests(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    mut player: Query<
        (
            &crate::player::PlayerBody,
            &mut crate::player::PlayerInteractionState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    chests: Query<
        (
            Entity,
            &FeatureId,
            &FeatureName,
            &FeatureAabb,
            Option<&Opened>,
            Option<&FallingChest>,
        ),
        (With<FeatureSimEntity>, With<ChestFeature>),
    >,
    mut gameplay_effects: MessageWriter<GameplayEffect>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    let Ok((player_body, mut interaction)) = player.single_mut() else {
        return;
    };
    if !interaction.buffered() {
        return;
    }
    let player_body = player_body.aabb();
    for (entity, id, name, aabb, opened, falling) in &chests {
        if falling.is_some() || opened.is_some() || !aabb.aabb().strict_intersects(player_body) {
            continue;
        }
        commands.entity(entity).insert(Opened);
        interaction.clear();
        banner.show(format!("opened {}", name.0.as_str()), 2.6);
        let pos = aabb.center;
        vfx.write(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
        sfx.write(SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_TREASURE_CHEST_OPEN,
            pos,
        });
        if let Some(encounter_id) = id.as_str().strip_prefix("encounter_chest_") {
            gameplay_effects.write(GameplayEffect::SetFlag {
                id: format!("encounter_{encounter_id}_reward_dropped"),
                on: true,
            });
        }
        break;
    }
}
