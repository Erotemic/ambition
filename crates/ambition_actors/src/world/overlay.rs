//! Sandbox collision-world overlay rebuilt from ECS feature state.
//!
//! The overlay is the bridge between the static ECS world (loaded
//! from LDtk) and the dynamic feature state (broken breakables, live
//! pogo target volumes, boss hurtboxes). Engine code that needs the augmented
//! collision world calls `world_with_sandbox_solids` with this resource;
//! rebuilding it once per frame keeps the augment cheap.

use ambition_engine_core as ae;
use bevy::prelude::*;

use crate::combat::*;

pub use ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay;

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
    // Gate contributors (encounter / intro lock walls, gnu_ton arena gate)
    // re-extend these after we run; clearing them here gives them the same
    // clean-slate-per-frame contract the breakable blocks above have. (Portal
    // carves are owned + cleared by the portal subsystem, so not touched here.)
    overlay.gate_solids.clear();
    overlay.removed_block_names.clear();
    overlay.climbable_carves.clear();
    overlay.water_regions.clear();
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
            id: ae::GeoId::anon(),
            name: format!("ecs-breakable {}", name.0.as_str()),
            aabb: aabb.aabb(),
            kind,
            velocity: ae::Vec2::ZERO,
        });
        if feature.breakable.collision.blocks_movement() && feature.breakable.trigger.allows_stand()
        {
            overlay.blocks.push(ae::Block {
                id: ae::GeoId::anon(),
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
            id: ae::GeoId::anon(),
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
                id: ae::GeoId::anon(),
                name: format!("ecs-pogo-target {} {}", id.as_str(), idx),
                aabb,
                kind: ae::BlockKind::PogoOrb,
                velocity: ae::Vec2::ZERO,
            });
        }
    }
}
