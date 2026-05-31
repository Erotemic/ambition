//! Pogo-target proximity query.
//!
//! Detects whether a pogo-able feature sits beneath the primary
//! player's feet within bounce range. Feeds
//! [`super::resolvers::WorldView::pogo_target_below`] so an aerial
//! aim-down Attack reads as `AttackVariant::Pogo` (instead of `DAir`)
//! only when bouncing would actually fire.
//!
//! Pogo-able features now publish [`crate::features::PogoTargetVolumes`].
//! The default feature rule derives those volumes from current
//! [`crate::features::DamageableVolumes`], so peaceful NPCs, hostile actors,
//! and boss hurtboxes all feed the same affordance path. The legacy
//! [`crate::features::PogoTargetContributor`] marker remains as a fallback for
//! authored stand-to-crumble surfaces that are pogoable but not damageable.

use crate::engine_core::AabbExt;
use bevy::prelude::*;

use crate::features::{FeatureAabb, FeatureSimEntity, PogoTargetContributor, PogoTargetVolumes};

/// Resource: true iff a pogo-able target is currently below the
/// primary player within [`POGO_DETECTION_DEPTH`] of their feet.
/// Default is `false`.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PogoTargetBelow(pub bool);

/// How far below the player's feet to scan for pogo-able features,
/// in world pixels. ~3× the player's height; close to engine-side
/// pogo acceptance range but deliberately generous so the HUD
/// upgrades to "Pogo" right before the bounce would fire, not
/// after.
pub const POGO_DETECTION_DEPTH: f32 = 96.0;

/// Half-width of the pogo scan box, in world pixels. Slightly wider
/// than the player's AABB so a slightly-mis-aligned approach still
/// previews "Pogo" rather than flickering between labels frame-to-
/// frame near the edge.
pub const POGO_DETECTION_HALF_WIDTH: f32 = 24.0;

/// Refresh [`PogoTargetBelow`] each frame. Cheap: a single AABB
/// build per player + a linear scan over published pogo volumes
/// (a few dozen at most per room).
pub fn update_pogo_target_below(
    player: Query<
        &crate::player::PlayerKinematics,
        (
            With<crate::player::PlayerEntity>,
            With<crate::player::PrimaryPlayer>,
        ),
    >,
    targets: Query<&PogoTargetVolumes, With<FeatureSimEntity>>,
    legacy_targets: Query<&FeatureAabb, (With<FeatureSimEntity>, With<PogoTargetContributor>)>,
    mut out: ResMut<PogoTargetBelow>,
) {
    let Ok(kin) = player.single() else {
        if out.0 {
            *out = PogoTargetBelow(false);
        }
        return;
    };
    // Build a downward scan box anchored at the player's feet
    // (player.pos.y is the center; +Y is down in sim coords, so feet
    // sit at `pos.y + size.y * 0.5`).
    let feet_y = kin.pos.y + kin.size.y * 0.5;
    let center = crate::engine_core::Vec2::new(kin.pos.x, feet_y + POGO_DETECTION_DEPTH * 0.5);
    let half_size =
        crate::engine_core::Vec2::new(POGO_DETECTION_HALF_WIDTH, POGO_DETECTION_DEPTH * 0.5);
    let scan = crate::engine_core::Aabb::new(center, half_size);

    let mut hit = false;
    'targets: for pogo in &targets {
        for aabb in pogo.volumes.iter().copied() {
            if aabb.strict_intersects(scan) {
                hit = true;
                break 'targets;
            }
        }
    }
    if !hit {
        for aabb in &legacy_targets {
            if aabb.aabb().strict_intersects(scan) {
                hit = true;
                break;
            }
        }
    }
    if out.0 != hit {
        *out = PogoTargetBelow(hit);
    }
}
