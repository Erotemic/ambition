//! Ambition GROUND-ITEM bindings for the portal mechanic.
//!
//! Portal core teleports thrown items through its content-agnostic
//! [`PortalTransitable`] body rather than through Ambition's [`GroundItem`].
//! This module attaches that marker and keeps the two in sync around transit.
//!
//! - Movement intent: the same-wall held-input warp + emergence guard rotate
//!   the player's held movement after a crossing so movement continues correctly.
//!   Portal core applies that to [`PlayerMovementIntent`]; this module mirrors the
//!   `ControlFrame` movement axes into the intent before the warp/transit runs and
//!   copies the (possibly warped) intent back to `ControlFrame` afterward, so the
//!   result is byte-identical to portal core mutating `ControlFrame` directly.
//! - Ground-item transit: thrown [`GroundItem`]s are teleported by portal core
//!   through the generic [`PortalTransitable`] body; this module attaches that
//!   marker to ground items and keeps it in sync with the `GroundItem` body around
//!   transit.
//!
//! [`GroundItem`]: ambition_held_items::GroundItem
//! [`PortalTransitable`]: ambition_portal2d::PortalTransitable

use bevy::prelude::*;

use ambition_held_items::GroundItem;
use ambition_portal2d::PortalTransitable;

/// Attach the portal-core [`PortalTransitable`] marker to any [`GroundItem`] that
/// lacks it, and mirror the item's body into it. Resting items (`vel == ZERO`)
/// carry the marker too but never transit (portal core skips them). Runs before
/// [`ambition_portal2d::portal_teleport_ground_items`].
pub fn sync_ground_items_to_transitable(
    mut commands: Commands,
    mut items: Query<(Entity, &GroundItem, Option<&mut PortalTransitable>)>,
) {
    for (entity, item, transitable) in &mut items {
        match transitable {
            Some(mut t) => {
                t.pos = item.pos;
                t.vel = item.vel;
                t.half_extent = item.half_extent;
            }
            None => {
                commands.entity(entity).insert(PortalTransitable {
                    pos: item.pos,
                    vel: item.vel,
                    half_extent: item.half_extent,
                });
            }
        }
    }
}

/// Mirror the (possibly teleported) [`PortalTransitable`] body back into the
/// `GroundItem`. Runs immediately after
/// [`ambition_portal2d::portal_teleport_ground_items`].
pub fn sync_transitable_to_ground_items(mut items: Query<(&mut GroundItem, &PortalTransitable)>) {
    for (mut item, t) in &mut items {
        item.pos = t.pos;
        item.vel = t.vel;
    }
}
