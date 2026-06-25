//! Ambition transit bindings for the portal mechanic.
//!
//! Portal core's transit ([`ambition_gameplay_core::portal::portal_transit`] /
//! [`ambition_gameplay_core::portal::warp_portal_input`] / [`ambition_gameplay_core::portal::portal_teleport_ground_items`])
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
//! [`GroundItem`]: ambition_gameplay_core::items::pickup::GroundItem
//! [`PlayerMovementIntent`]: ambition_gameplay_core::portal::PlayerMovementIntent
//! [`PortalTransitable`]: ambition_gameplay_core::portal::PortalTransitable

use bevy::prelude::*;

use ambition_input::ControlFrame;
use ambition_gameplay_core::items::pickup::GroundItem;
use ambition_gameplay_core::portal::{PlayerMovementIntent, PortalTransitable};

/// Copy this frame's `ControlFrame` movement axes into the portal-core
/// [`PlayerMovementIntent`]. Runs before [`ambition_gameplay_core::portal::warp_portal_input`] and
/// again before [`ambition_gameplay_core::portal::portal_transit`] so portal core always
/// reads the live held direction without naming `ControlFrame`.
pub fn sync_movement_intent_from_control(
    control: Option<Res<ControlFrame>>,
    mut intent: ResMut<PlayerMovementIntent>,
) {
    if let Some(control) = control {
        intent.dir = Vec2::new(control.axis_x, control.axis_y);
    }
}

/// Copy the (possibly warped) portal-core [`PlayerMovementIntent`] back into the
/// `ControlFrame` movement axes. Runs immediately after
/// [`ambition_gameplay_core::portal::warp_portal_input`], so the brain / movement see the adjusted
/// axes exactly as they did when portal core mutated `ControlFrame` directly.
pub fn apply_movement_intent_to_control(
    intent: Res<PlayerMovementIntent>,
    control: Option<ResMut<ControlFrame>>,
) {
    if let Some(mut control) = control {
        control.axis_x = intent.dir.x;
        control.axis_y = intent.dir.y;
    }
}

/// Attach the portal-core [`PortalTransitable`] marker to any [`GroundItem`] that
/// lacks it, and mirror the item's body into it. Resting items (`vel == ZERO`)
/// carry the marker too but never transit (portal core skips them). Runs before
/// [`ambition_gameplay_core::portal::portal_teleport_ground_items`].
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
/// [`ambition_gameplay_core::portal::portal_teleport_ground_items`].
pub fn sync_transitable_to_ground_items(mut items: Query<(&mut GroundItem, &PortalTransitable)>) {
    for (mut item, t) in &mut items {
        item.pos = t.pos;
        item.vel = t.vel;
    }
}
