//! Ambition GROUND-ITEM bindings for the portal mechanic.
//!
//! Portal core teleports thrown items through its content-agnostic
//! [`PortalTransitable`] body rather than through Ambition's [`GroundItem`].
//! This module attaches that marker and keeps the two in sync around transit.
//!
//! ⚠ it used to own a second job — mirroring `ControlFrame` movement axes into a
//! `PlayerMovementIntent` resource and back — and neither the job nor the
//! resource exists: portal input warping now resolves the `DrivingParticipant`
//! and edits THAT seat's frame (`warp_portal_input`, `ability_adapter`).
//!
//! [`GroundItem`]: ambition_platformer2d_actor_monolith::items::pickup::GroundItem
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
