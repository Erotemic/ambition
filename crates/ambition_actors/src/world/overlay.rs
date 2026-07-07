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

/// Collision contribution from ECS-owned breakables. Rebuilt before the main
/// sandbox tick and consumed by `world_with_sandbox_solids` anywhere the engine
/// needs the augmented collision world.
#[derive(Resource, Default, Clone, Debug)]
pub struct FeatureEcsWorldOverlay {
    pub blocks: Vec<ae::Block>,
    /// Authored-equivalent static solids contributed by per-frame *gates*
    /// (encounter lock walls, intro flag gates) instead of being mutated into
    /// the authored [`ambition_engine_core::RoomGeometry`] base. Unlike `blocks` — which carry
    /// moving-platform semantics that projectiles pass through — gate solids are
    /// composited into EVERY collision read-path (player, projectile, traversal)
    /// AND surfaced to the render layer, so a lock wall collides and draws
    /// exactly as it did when it lived in the base. This is what keeps the base
    /// authored-immutable mid-room (the resolved RoomGeometry decision): a gate
    /// is a derived per-frame contribution, not a base edit. Cleared by
    /// [`rebuild_feature_ecs_world_overlay`] each frame, then re-extended by the
    /// gate contributor systems that run after it in `WorldPrep`. Also carries
    /// other per-frame additive statics that want full collision composition but
    /// no lock-wall sprite (falling-sand settled tiles); the render reconcile
    /// filters to `lockwall:` / `intro_lock:` names, so non-gate solids here are
    /// collision-only.
    pub gate_solids: Vec<ae::Block>,
    /// Portal apertures to carve OUT of the host surface: the floor / wall a
    /// portal sits on becomes non-solid inside the opening so a body can sink
    /// in (the "feet in, feet out" transit). Filled each frame by
    /// `portal::publish_portal_carves`; consumed by `world_with_sandbox_solids`.
    pub portal_carves: Vec<ae::Aabb>,
    /// Authored blocks (by `Block::name`) to REMOVE from the collision view this
    /// frame — a content gate *opening* an authored solid without mutating the
    /// immutable [`ambition_engine_core::RoomGeometry`] base. The inverse of `gate_solids`:
    /// `gate_solids` ADD derived statics, this SUBTRACTS authored ones (gnu_ton's
    /// `ladder_floor_gate` drops out on boss defeat so the player can climb out).
    /// Composited into every collision read-path (player/actor/item/traversal via
    /// `world_with_sandbox_solids`, projectiles via the gate view). Cleared each
    /// frame by [`rebuild_feature_ecs_world_overlay`], re-extended by the WorldPrep
    /// gate contributors — same clean-slate-per-frame contract as `gate_solids`.
    pub removed_block_names: Vec<String>,
    /// Authored climbable regions to SUPPRESS this frame: any base climbable
    /// region intersecting one of these AABBs is dropped from the view. Same
    /// immutable-base inversion as `removed_block_names` for ladders/vines (gnu_ton
    /// hides the arena retreat ladder while the boss is alive). Climbable-only —
    /// projectiles never read climbable — so it composites in
    /// `world_with_sandbox_solids` alone.
    pub climbable_carves: Vec<ae::Aabb>,
    /// Additive water/liquid regions contributed per-frame (falling-sand settled
    /// pools), composited into the player/actor view alongside the base water.
    /// Water-only — projectiles don't read water — so it composites in
    /// `world_with_sandbox_solids` alone. The additive counterpart to `gate_solids`
    /// for liquid, keeping the authored base immutable mid-room. Cleared + re-
    /// extended each frame like the other contributions.
    pub water_regions: Vec<ae::WaterRegion>,
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
