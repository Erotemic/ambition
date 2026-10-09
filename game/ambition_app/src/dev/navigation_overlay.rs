//! The navigation overlay: the surface graph a navigating body is advised
//! from, drawn over the room.
//!
//! Presentation only. It reads the graphs in use
//! (`RoomNavigation::graphs_in_use`) and this tick's advice, and writes
//! nothing to the simulation. Off until [`NavigationOverlay::shown`] is set;
//! `capture_scene --nav-overlay` sets it.
//!
//! - green line: a standing surface (where the feet centre can be);
//! - yellow arrow: a hop, from its take-off to its landing;
//! - orange arrow: a drop;
//! - magenta cross: the place a body is going to; cyan cross: the place
//!   beside its target, when a route goes there.

use ambition_platformer2d::actors::features::ecs::navigation::{NavigationAdvice, RoomNavigation};
use ambition_platformer2d::engine_core::navigation::NavLegKind;
use ambition_platformer2d::render::rendering::debug_viz::{cyan, draw_arrow, green, magenta, orange, w2, yellow};
use bevy::prelude::*;

/// Whether the navigation overlay is drawn.
#[derive(Resource, Default)]
pub struct NavigationOverlay {
    pub shown: bool,
}

pub(crate) fn draw_navigation_overlay(
    overlay: Res<NavigationOverlay>,
    mut gizmos: Gizmos,
    collision: ambition_platformer2d::world::collision::CollisionWorld,
    navigation: Option<Res<RoomNavigation>>,
    advice: Option<Res<NavigationAdvice>>,
) {
    if !overlay.shown {
        return;
    }
    let (Some(world), Some(navigation)) = (collision.base(), navigation) else {
        return;
    };
    for graph in navigation.graphs_in_use() {
        for surface in &graph.surfaces {
            let (left, right) = (graph.frame.point(surface.left, surface.top), graph.frame.point(surface.right, surface.top));
            // Two lines: one gizmo line is thin in a shot of a whole room.
            for lift in [2.0, 4.0] {
                let up = graph.frame.down * -lift;
                gizmos.line_2d(w2(world, left + up), w2(world, right + up), green());
            }
        }
        for link in &graph.links {
            let color = if link.leg.kind == NavLegKind::Drop { orange() } else { yellow() };
            draw_arrow(&mut gizmos, w2(world, link.leg.takeoff), w2(world, link.leg.land), color);
        }
    }
    let cross = |gizmos: &mut Gizmos, at: Vec2, color: Color| {
        let at = w2(world, at);
        gizmos.line_2d(at + Vec2::new(-10.0, -10.0), at + Vec2::new(10.0, 10.0), color);
        gizmos.line_2d(at + Vec2::new(-10.0, 10.0), at + Vec2::new(10.0, -10.0), color);
    };
    for (_, advice) in advice.iter().flat_map(|advice| advice.iter()) {
        if let Some(goal) = advice.goal {
            cross(&mut gizmos, goal, magenta());
        }
        if let Some(place) = advice.target_place {
            cross(&mut gizmos, place, cyan());
        }
    }
}
