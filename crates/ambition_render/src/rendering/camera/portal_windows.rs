use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf;
use ambition_portal2d_presentation::{PortalObserverViews, PortalWindowView};
use bevy::prelude::{ResMut, Vec2};

use super::coverage::GameplayOutputs;

/// Publish the play-plane coverage after the view's final pose and zoom.
pub(in crate::rendering) fn publish_portal_window_views(
    mut windows: ResMut<PortalObserverViews>,
    outputs: GameplayOutputs,
    rooms: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
) {
    windows.set_windows(outputs.coverage().filter_map(|output| {
        let room = rooms.in_room(output.frame.room)?;
        let right = Vec2::from_angle(output.frame.roll_radians);
        let mut bounds = output
            .plane
            .iter()
            .chain(output.cards.iter())
            .map(|plane| plane.bounds(right));
        let first = bounds.next()?;
        let bounds = bounds.fold(first, |total, view| total.union(view));
        let center = right * bounds.center().x + right.perp() * bounds.center().y;
        let flip_y = Vec2::new(1.0, -1.0);
        Some((
            output.observer,
            PortalWindowView {
                center_world: room.0.size * 0.5 + center * flip_y,
                visible_view: bounds.size(),
                axis: right * flip_y,
                output_pixels: output.pixels.as_vec2(),
            },
        ))
    }));
}
