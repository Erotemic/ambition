//! Feature collision-world overlay rebuilt from ECS-owned world geometry.
//!
//! The overlay is the bridge between the static ECS world (loaded
//! from LDtk) and dynamic feature state that explicitly contributes world
//! collision geometry (broken breakables, moving rebound surfaces). Engine code
//! that needs the augmented collision world calls `world_with_sandbox_solids`
//! with this resource;
//! rebuilding it once per frame keeps the augment cheap.

use ambition_platformer2d_shared_tangle::feature_overlay::{FeatureEcsWorldOverlay};
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_combat::*;


/// Rebuild the transient collision blocks contributed by ECS-owned features.
/// The set `rebuild_feature_ecs_world_overlay` runs in, so a consumer can order
/// against a NAME instead of against this function.
///
/// six places order against that function today — four in `ambition_content` (`bosses`,
/// `falling_sand`, `falling_sand_sim`, `intro`), one in the monolith's own `encounter`, one more on
/// `update_ecs_hazards` beside it.
///
/// deliberately a ONE-MEMBER set. The obvious alternative — a set spanning
/// this system and `update_ecs_hazards` next to it in the chain — would make
/// `.after(set)` STRICTER than the `.after(rebuild_feature_ecs_world_overlay)` it
/// replaces, because consumers would newly wait for hazards too. One member
/// makes the swap exactly equivalent, which is what lets it be made without a
/// judgement call about scheduling in a rollback-critical chain.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FeatureWorldOverlaySet;

pub fn rebuild_feature_ecs_world_overlay(
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
    breakables: Query<(&FeatureName, &CenteredAabb, &BreakableFeature), With<FeatureSimEntity>>,
    // Only entities that explicitly contribute WORLD pogo geometry are lowered
    // into collision blocks. Combat bodies also publish `PogoTargetVolumes`, but
    // those are entity-side affordance geometry and must retain their identity.
    pogo_targets: Query<
        (&FeatureId, &CenteredAabb, Option<&PogoTargetVolumes>),
        (With<FeatureSimEntity>, With<PogoTargetContributor>),
    >,
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
    for (name, aabb, feature) in &breakables {
        if feature.broken() {
            continue;
        }
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
            art_color: None,
        });
    }

    // Explicit ECS WORLD-pogo bridge. `PogoTargetContributor` says this entity
    // contributes collision-world rebound geometry. A non-empty published
    // `PogoTargetVolumes` is authoritative; otherwise the contributor's own
    // centered envelope is the deliberate world-surface fallback. Ordinary
    // bodies intentionally lack the contributor and stay entity contacts, so
    // their identity is never flattened into anonymous blocks.
    for (id, centered, pogo) in &pogo_targets {
        let published = pogo.filter(|pogo| !pogo.volumes.is_empty());
        if let Some(pogo) = published {
            for (idx, aabb) in pogo.volumes.iter().copied().enumerate() {
                overlay.blocks.push(ae::Block {
                    id: ae::GeoId::anon(),
                    name: format!("ecs-pogo-target {} {}", id.as_str(), idx),
                    aabb,
                    kind: ae::BlockKind::PogoOrb,
                    velocity: ae::Vec2::ZERO,
                    art_color: None,
                });
            }
        } else {
            overlay.blocks.push(ae::Block {
                id: ae::GeoId::anon(),
                name: format!("ecs-pogo-target-fallback {}", id.as_str()),
                aabb: centered.aabb(),
                kind: ae::BlockKind::PogoOrb,
                velocity: ae::Vec2::ZERO,
                art_color: None,
            });
        }
    }
}
