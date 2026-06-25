//! Sandbox collision-world overlay rebuilt from ECS feature state.
//!
//! The overlay is the bridge between the static ECS world (loaded
//! from LDtk) and the dynamic feature state (broken breakables, live
//! pogo target volumes, boss hurtboxes). Engine code that needs the augmented
//! collision world calls `world_with_sandbox_solids` with this resource;
//! rebuilding it once per frame keeps the augment cheap.

use super::*;

/// Collision contribution from ECS-owned breakables. Rebuilt before the main
/// sandbox tick and consumed by `world_with_sandbox_solids` anywhere the engine
/// needs the augmented collision world.
#[derive(Resource, Default, Clone, Debug)]
pub struct FeatureEcsWorldOverlay {
    pub blocks: Vec<ae::Block>,
    /// Authored-equivalent static solids contributed by per-frame *gates*
    /// (encounter lock walls, intro flag gates) instead of being mutated into
    /// the authored [`crate::RoomGeometry`] base. Unlike `blocks` — which carry
    /// moving-platform semantics that projectiles pass through — gate solids are
    /// composited into EVERY collision read-path (player, projectile, traversal)
    /// AND surfaced to the render layer, so a lock wall collides and draws
    /// exactly as it did when it lived in the base. This is what keeps the base
    /// authored-immutable mid-room (the resolved RoomGeometry decision): a gate
    /// is a derived per-frame contribution, not a base edit. Cleared by
    /// [`rebuild_feature_ecs_world_overlay`] each frame, then re-extended by the
    /// gate contributor systems that run after it in `WorldPrep`.
    pub gate_solids: Vec<ae::Block>,
    /// Portal apertures to carve OUT of the host surface: the floor / wall a
    /// portal sits on becomes non-solid inside the opening so a body can sink
    /// in (the "feet in, feet out" transit). Filled each frame by
    /// `portal::publish_portal_carves`; consumed by `world_with_sandbox_solids`.
    pub portal_carves: Vec<ae::Aabb>,
}

/// Rebuild the transient collision blocks contributed by ECS-owned features.
pub fn rebuild_feature_ecs_world_overlay(
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
    breakables: Query<
        (&FeatureId, &FeatureName, &CenteredAabb, &BreakableFeature),
        With<FeatureSimEntity>,
    >,
    legacy_pogo_targets: Query<
        (&FeatureId, &CenteredAabb),
        (
            With<FeatureSimEntity>,
            With<PogoTargetContributor>,
            Without<PogoTargetVolumes>,
        ),
    >,
    pogo_targets: Query<(&FeatureId, &PogoTargetVolumes), With<FeatureSimEntity>>,
) {
    overlay.blocks.clear();
    // Gate contributors (encounter / intro lock walls) re-extend this after we
    // run; clearing it here gives them the same clean-slate-per-frame contract
    // the breakable blocks above have.
    overlay.gate_solids.clear();
    for (id, name, aabb, feature) in &breakables {
        if feature.broken() {
            continue;
        }
        // Pogo-refresh breakables are now contributed through
        // PogoTargetVolumes below. Preserve the old behavior where a pogo orb is
        // a non-solid ghost block even if its authored collision says otherwise.
        if feature.breakable.pogo_refresh {
            continue;
        }
        let kind = match feature.breakable.collision {
            ambition_interaction::BreakableCollision::None => continue,
            ambition_interaction::BreakableCollision::Solid => ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard,
            },
            ambition_interaction::BreakableCollision::OneWayUp => ae::BlockKind::OneWay,
        };
        overlay.blocks.push(ae::Block {
            name: format!("ecs-breakable {}", name.0.as_str()),
            aabb: aabb.aabb(),
            kind,
            velocity: ae::Vec2::ZERO,
        });
        if feature.breakable.collision.blocks_movement() && feature.breakable.trigger.allows_stand()
        {
            overlay.blocks.push(ae::Block {
                name: format!("ecs-breakable-pogo-target {}", id.as_str()),
                aabb: aabb.aabb(),
                kind: ae::BlockKind::PogoOrb,
                velocity: ae::Vec2::ZERO,
            });
        }
    }

    // Legacy stand-to-crumble contributors that do not have the new volume
    // components yet. Production breakables currently receive PogoTargetVolumes
    // at spawn, but this fallback keeps minimal tests and custom spawns working.
    for (id, aabb) in &legacy_pogo_targets {
        overlay.blocks.push(ae::Block {
            name: format!("ecs-legacy-pogo-target {}", id.as_str()),
            aabb: aabb.aabb(),
            kind: ae::BlockKind::PogoOrb,
            velocity: ae::Vec2::ZERO,
        });
    }

    // Generic ECS pogo target bridge. Actors, NPCs, bosses, and hit-reactive
    // breakables publish PogoTargetVolumes; the overlay does not need to know
    // which feature family produced them.
    for (id, pogo) in &pogo_targets {
        for (idx, aabb) in pogo.volumes.iter().copied().enumerate() {
            overlay.blocks.push(ae::Block {
                name: format!("ecs-pogo-target {} {}", id.as_str(), idx),
                aabb,
                kind: ae::BlockKind::PogoOrb,
                velocity: ae::Vec2::ZERO,
            });
        }
    }
}
