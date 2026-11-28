use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf;
use ambition_portal2d_presentation::{PortalObserverViews, PortalWindowView};
use bevy::prelude::{Entity, Local, Rect, ResMut, Vec2};

use super::coverage::GameplayOutputs;

pub(in crate::rendering) struct WindowDemand {
    room_center: Vec2,
    right: Vec2,
    bounds: Rect,
    density: Vec2,
}

impl WindowDemand {
    fn published(self) -> PortalWindowView {
        let along = self.bounds.center();
        let center = self.right * along.x + self.right.perp() * along.y;
        let flip_y = Vec2::new(1.0, -1.0);
        PortalWindowView {
            center_world: self.room_center + center * flip_y,
            visible_view: self.bounds.size(),
            axis: self.right * flip_y,
            texels_per_world: self.density,
        }
    }
}

/// Publish the play-plane coverage after the view's final pose and zoom.
pub(in crate::rendering) fn publish_portal_window_views(
    mut windows: ResMut<PortalObserverViews>,
    outputs: GameplayOutputs,
    rooms: LiveRoomOf<ambition_platformer2d_core::RoomGeometry>,
    mut demands: Local<Vec<(Entity, WindowDemand)>>,
) {
    for output in outputs.coverage() {
        let Some(room) = rooms.in_room(output.frame.room) else {
            continue;
        };
        let right = Vec2::from_angle(output.frame.roll_radians);
        for footprint in output.plane.iter().chain(output.cards.iter()) {
            let Some(density) = footprint.texels_per_world(right) else {
                continue;
            };
            let bounds = footprint.bounds(right);
            if let Some((_, demand)) = demands.iter_mut().find(|(view, _)| *view == output.observer)
            {
                demand.bounds = demand.bounds.union(bounds);
                demand.density = demand.density.max(density);
            } else {
                demands.push((
                    output.observer,
                    WindowDemand {
                        room_center: room.0.size * 0.5,
                        right,
                        bounds,
                        density,
                    },
                ));
            }
        }
    }
    windows.set_windows(demands.drain(..).map(|(view, demand)| (view, demand.published())));
}
