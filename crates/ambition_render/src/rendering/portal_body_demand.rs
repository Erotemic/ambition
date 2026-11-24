//! Request composed pictures for bodies near a paired opening.

use ambition_portal2d_presentation::{
    PlacedPortal, PortalBodyView, PortalCarrySelection, PortalFrames, PortalTransitView,
};
use ambition_sprite_sheet::character::rigged::ComposedBodyDemand;
use bevy::prelude::*;

use super::actors::rigged::{LimbTwin, RiggedPresentedBy};

/// Prepare the picture before a body reaches the opening. Keep the picture
/// for the full transit, including a limb that carries its host's picture.
pub fn declare_portal_body_demand(
    frames: PortalFrames,
    portals: Query<(Entity, &PlacedPortal)>,
    selection: Option<Res<PortalCarrySelection>>,
    bodies: Query<
        (
            Entity,
            &Transform,
            &Sprite,
            Option<&PortalBodyView>,
            Has<PortalTransitView>,
            Has<LimbTwin>,
        ),
        (With<RiggedPresentedBy>, Without<ChildOf>),
    >,
    mut demand: ResMut<ComposedBodyDemand>,
) {
    if selection.is_some_and(|selection| !selection.enabled) {
        return;
    }
    let by_room = frames.portals_by_room(portals.iter());
    for (entity, transform, sprite, body, in_transit, limb_twin) in &bodies {
        let Some(placement) = frames.of(entity) else {
            continue;
        };
        let room_portals = by_room.in_room(Some(placement.room));
        let center = ambition_platformer2d_core::config::bevy_size_to_world(
            placement.frame.size,
            transform.translation.truncate(),
        );
        let art_reach = sprite.custom_size.unwrap_or(Vec2::ZERO).length()
            * transform.scale.truncate().abs().max_element();
        if room_portals.iter().any(|portal| {
            if !room_portals
                .iter()
                .any(|other| other.channel == portal.channel.partner())
            {
                return false;
            }
            // A limb twin can appear at an opening far from its host.
            in_transit
                || limb_twin
                || near_opening(portal, center, art_reach)
                || body.is_some_and(|body| near_opening(portal, body.pos, body.size.length() * 0.5))
        }) {
            demand.declare(entity);
        }
    }
}

fn near_opening(portal: &PlacedPortal, center: Vec2, reach: f32) -> bool {
    let mouth = portal.opening_face();
    let along = Vec2::new(-mouth.normal.y, mouth.normal.x);
    let from_mouth = center - mouth.center;
    let closest = along
        * from_mouth
            .dot(along)
            .clamp(-mouth.half_along, mouth.half_along);
    // The extra opening width gives the capture time to prepare during approach.
    let prepare_reach = reach + 2.0 * mouth.half_along + mouth.half_out;
    (from_mouth - closest).length_squared() <= prepare_reach * prepare_reach
}
