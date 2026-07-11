//! Player → static-chest open path on the ECS feature side.

use super::*;
use ambition_sfx::SfxMessage;

/// Open ECS-owned static chests from the same interaction buffer used by doors
/// and legacy NPCs/switches.
pub fn open_ecs_chests(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    controlled: Option<Res<ambition_platformer_primitives::markers::ControlledSubject>>,
    // The local controller's buffered interact (published from the device onto its
    // slot); consumed for whatever body it currently drives.
    mut slot_gestures: ResMut<crate::control::SlotInteractionState>,
    // Interact-gesture pose + startup-frame fallback subject.
    mut input_surface: Query<
        (Entity, &mut crate::actor::BodyAnimFacts),
        (
            With<ambition_platformer_primitives::markers::PlayerEntity>,
            With<ambition_platformer_primitives::markers::PrimaryPlayer>,
        ),
    >,
    // The driven body's kinematics — reach is measured from the controlled subject.
    bodies: Query<&ambition_engine_core::BodyKinematics>,
    chests: Query<
        (
            Entity,
            &FeatureId,
            &FeatureName,
            &CenteredAabb,
            Option<&Opened>,
            Option<&FallingChest>,
        ),
        (With<FeatureSimEntity>, With<ChestFeature>),
    >,
    mut set_flag: MessageWriter<SetFlagRequested>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    // Iterate every player so each player's own buffered interact
    // can open a chest the player is overlapping. Per-player interact
    // state is independent (each player has their own
    // `PlayerInteractionState`); the chest is shared (a future co-op
    // build can still gate "first-come gets the open" by inserting
    // the `Opened` marker, which keeps subsequent attempts no-ops).
    // OVERNIGHT-TODO #17.6/#17.8 — preserve single-player behavior
    // because the iterator has one entity today.
    // Same hold time as the NPC / switch interact gesture. Kept in
    // sync with `interact_ecs_actors_and_switches::INTERACT_ANIM_HOLD_SECS`
    // so the player's reach-and-open animation feels uniform across
    // every interactable kind.
    const INTERACT_ANIM_HOLD_SECS: f32 = 0.28;
    let Ok((primary_entity, mut anim)) = input_surface.single_mut() else {
        return;
    };
    if !slot_gestures.primary().buffered() {
        return;
    }
    // Reach is measured from the controlled subject — a possessed actor opens the
    // chest IT is standing on, not one the vacated home avatar is next to.
    let subject = controlled
        .and_then(|subject| subject.0)
        .unwrap_or(primary_entity);
    let Ok(subject_kin) = bodies.get(subject) else {
        return;
    };
    let reach_aabb = subject_kin.aabb();
    {
        for (entity, id, name, aabb, opened, falling) in &chests {
            if falling.is_some() || opened.is_some() || !aabb.aabb().strict_intersects(reach_aabb) {
                continue;
            }
            commands.entity(entity).insert(Opened);
            slot_gestures.primary_mut().clear();
            anim.interact_anim_timer = INTERACT_ANIM_HOLD_SECS;
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
                set_flag.write(SetFlagRequested {
                    id: format!("encounter_{encounter_id}_reward_dropped"),
                    on: true,
                });
            }
            break;
        }
    }
}

#[cfg(test)]
mod chest_tests;
