//! Pogo-target proximity query.
//!
//! Detects whether a pogo-able feature sits beneath the primary
//! player's feet within bounce range. Feeds
//! [`super::resolvers::WorldView::pogo_target_below`] so an aerial
//! aim-down Attack reads as `AttackVariant::Pogo` (instead of `DAir`)
//! only when bouncing would actually fire.
//!
//! "Pogo-able" today means any feature entity carrying the
//! [`crate::features::PogoTargetContributor`] marker — breakable
//! pogo orbs, spike-cap enemies the engine accepts as pogo targets,
//! etc. The detection is generous: any AABB overlap inside the
//! downward sweep volume counts. Tightening to the actual pogo arc
//! is the engine's job, not the HUD's; a false positive here just
//! means the HUD label upgrades to "Pogo" one frame before the
//! engine accepts the bounce, which is the right side of the
//! conservative trade.

use ambition_engine::AabbExt;
use bevy::prelude::*;

use crate::features::{FeatureAabb, FeatureSimEntity, PogoTargetContributor};

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
/// build per player + a linear scan over [`PogoTargetContributor`]
/// features (a few dozen at most per room).
pub fn update_pogo_target_below(
    player: Query<
        &crate::player::PlayerKinematics,
        (
            With<crate::player::PlayerEntity>,
            With<crate::player::PrimaryPlayer>,
        ),
    >,
    targets: Query<&FeatureAabb, (With<FeatureSimEntity>, With<PogoTargetContributor>)>,
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
    let center = ambition_engine::Vec2::new(kin.pos.x, feet_y + POGO_DETECTION_DEPTH * 0.5);
    let half_size =
        ambition_engine::Vec2::new(POGO_DETECTION_HALF_WIDTH, POGO_DETECTION_DEPTH * 0.5);
    let scan = ambition_engine::Aabb::new(center, half_size);

    let mut hit = false;
    for aabb in &targets {
        if aabb.aabb().strict_intersects(scan) {
            hit = true;
            break;
        }
    }
    if out.0 != hit {
        *out = PogoTargetBelow(hit);
    }
}
