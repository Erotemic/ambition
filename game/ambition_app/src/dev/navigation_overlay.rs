//! The navigation overlay: the surface graph a navigating body is advised
//! from, drawn over the room.
//!
//! It draws the graphs in use and the current navigation advice for each room.
//! Enable it with "Navigation Graph" in the developer menu or
//! `capture_scene --nav-overlay`.
//!
//! - green line: a standing surface (where the feet centre can be);
//! - yellow arrow: a hop, from its take-off to its landing;
//! - orange arrow: a drop;
//! - magenta cross: the place a body is going to; cyan cross: the place
//!   beside its target, when a route goes there.

use ambition_platformer2d::actors::body::navigation::NavigationAdvice;
use ambition_platformer2d::actors::body::navigation::RoomNavigation;
use ambition_platformer2d::engine_core::navigation::NavLegKind;
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, LiveRooms, RoomInstanceRoot};
use ambition_platformer2d::render::rendering::debug_viz::{cyan, draw_arrow, green, magenta, orange, w2, yellow};
use ambition_platformer2d::render::rendering::retained_drawings::{DrawingScope, RetainedDrawings};
use ambition_platformer2d::world::collision::CollisionWorld;
use ambition_platformer2d::world::navigation::geometry_stamp;
use bevy::gizmos::GizmoAsset;
use bevy::prelude::*;

/// Draw the navigation overlay, whatever the developer menu says. For a tool.
#[derive(Resource, Default)]
pub struct NavigationOverlay {
    pub shown: bool,
}

#[derive(Component, Default)]
pub(crate) struct NavigationDrawing;

pub(crate) fn navigation_overlay_is_shown_or_drawn(
    overlay: Res<NavigationOverlay>,
    developer: Option<Res<ambition_platformer2d::dev_tools::dev_tools::DeveloperTools>>,
    drawings: Query<(), With<NavigationDrawing>>,
) -> bool {
    overlay.shown || developer.is_some_and(|developer| developer.show_navigation) || !drawings.is_empty()
}

pub(crate) fn draw_navigation_overlay(
    overlay: Res<NavigationOverlay>,
    developer: Option<Res<ambition_platformer2d::dev_tools::dev_tools::DeveloperTools>>,
    mut drawings: RetainedDrawings<NavigationDrawing>,
    collision: CollisionWorld,
    roots: Query<&LiveRoomInstance, With<RoomInstanceRoot>>,
    room_of_body: LiveRooms,
    navigation: Option<Res<RoomNavigation>>,
    advice: Option<Res<NavigationAdvice>>,
) {
    if !(overlay.shown || developer.is_some_and(|developer| developer.show_navigation)) {
        drawings.finish();
        return;
    }
    let Some(navigation) = navigation else {
        drawings.finish();
        return;
    };

    for &room in &roots {
        let Some(walls) = collision.room(Some(&InRoomInstance(room))) else {
            continue;
        };
        let world = walls.base();
        let stamp = geometry_stamp(&walls.navigation_ground(), &[]);
        let mut graphs = navigation.graphs_in_use_stamped(stamp).peekable();
        let mut room_advice = advice.iter().flat_map(|advice| advice.iter())
            .filter(|(body, _)| room_of_body.of(*body) == Some(room)).peekable();
        if graphs.peek().is_none() && room_advice.peek().is_none() {
            continue;
        }
        drawings.draw(DrawingScope::Room(room), |gizmos| {
            for graph in graphs {
                for surface in &graph.ground.surfaces {
                    let left = graph.ground.frame.point(surface.left, surface.top);
                    let right = graph.ground.frame.point(surface.right, surface.top);
                    for lift in [2.0, 4.0] {
                        let up = graph.ground.frame.down * -lift;
                        gizmos.line_2d(w2(world, left + up), w2(world, right + up), green());
                    }
                }
                for link in &graph.links {
                    let color = if link.leg.kind == NavLegKind::Drop { orange() } else { yellow() };
                    draw_arrow(gizmos, w2(world, link.leg.takeoff), w2(world, link.leg.land), color);
                }
            }
            for (_, advice) in room_advice {
                if let Some(goal) = advice.goal {
                    cross(gizmos, w2(world, goal), magenta());
                }
                if let Some(place) = advice.target_place {
                    cross(gizmos, w2(world, place), cyan());
                }
            }
        });
    }
    drawings.finish();
}

fn cross(gizmos: &mut GizmoAsset, at: Vec2, color: Color) {
    let rising = Vec2::new(10.0, 10.0);
    let falling = Vec2::new(10.0, -10.0);
    gizmos.line_2d(at - rising, at + rising, color);
    gizmos.line_2d(at - falling, at + falling, color);
}
