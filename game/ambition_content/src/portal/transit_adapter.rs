//! Ambition transit bindings for the portal mechanic.
//!
//! Portal core's transit ([`ambition_portal2d::portal_transit`] /
//! [`ambition_portal2d::warp_portal_input`] / [`ambition_portal2d::portal_teleport_ground_items`])
//! is content-agnostic: it reads/writes a [`PlayerMovementIntent`] resource and a
//! [`PortalTransitable`] body component instead of the Ambition [`ControlFrame`]
//! input type or the [`GroundItem`] body. These adapters own that glue:
//!
//! - **Movement intent:** the same-wall held-input warp + emergence guard rotate
//!   the player's held movement after a crossing so movement continues correctly.
//!   Portal core applies that to [`PlayerMovementIntent`]; this module mirrors the
//!   `ControlFrame` movement axes into the intent before the warp/transit runs and
//!   copies the (possibly warped) intent back to `ControlFrame` afterward, so the
//!   result is byte-identical to portal core mutating `ControlFrame` directly.
//! - **Ground-item transit:** thrown [`GroundItem`]s are teleported by portal core
//!   through the generic [`PortalTransitable`] body; this module attaches that
//!   marker to ground items and keeps it in sync with the `GroundItem` body around
//!   transit.
//!
//! [`ControlFrame`]: ambition_input::ControlFrame
//! [`GroundItem`]: ambition_platformer2d_actor_monolith::items::pickup::GroundItem
//! [`PlayerMovementIntent`]: ambition_portal2d::PlayerMovementIntent
//! [`PortalTransitable`]: ambition_portal2d::PortalTransitable

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::items::pickup::GroundItem;
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
