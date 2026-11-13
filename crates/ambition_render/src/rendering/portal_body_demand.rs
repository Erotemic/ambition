//! What a portal room asks of the composed bodies in it: each animated body
//! there has its picture made, so the carry can clip it.

use bevy::prelude::*;

/// Declare an image for every animated picture in a portal room.
/// A generated limb picture has the same image readers as its host.
/// Declare demand before drawing; a late declaration leaves no image for
/// the clipping pass on that frame.
pub fn declare_portal_body_demand(
    frames: ambition_portal2d_presentation::PortalFrames,
    portals: Query<Entity, With<ambition_portal2d_presentation::PlacedPortal>>,
    bodies: Query<
        Entity,
        (
            With<ambition_sprite_sheet::character::CharacterAnimator>,
            Without<bevy::prelude::ChildOf>,
        ),
    >,
    mut demand: ResMut<ambition_sprite_sheet::character::rigged::ComposedBodyDemand>,
) {
    let rooms: Vec<_> = portals.iter().filter_map(|portal| frames.room_of(portal)).collect();
    for body in &bodies {
        if frames.room_of(body).is_some_and(|room| rooms.contains(&room)) {
            demand.declare(body);
        }
    }
}
