//! F1's observed-geometry layer (GOBS-1,
//! `docs/planning/engine/geometry-observability-and-f1.md`).
//!
//! It draws whatever the installed geometry observers report for each shown
//! room ([`ambition_geometry_observation::observe_all`], the same call
//! headless inspection makes), with the player in that room as the subject.
//! A new observer is drawn with no edit here. Nothing is asked for unless
//! the debug overlay, gizmos and `show_observed_geometry` are all on.
//!
//! Colour says what gameplay does with a piece (its first role); an inactive
//! piece (a broken platform's block, a gate open for the player) is drawn
//! faint with a cross through it.

use ambition_dev_tools::DeveloperRuntimeState;
use ambition_dev_tools::dev_tools::DeveloperTools;
use ambition_geometry_observation::{
    GeometryObservation, ObservationQuery, ObservedLayer, ObservedShape, ObservedState, ObserverAnswer, observe_all,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;
use bevy::gizmos::{config::GizmoConfigGroup, gizmos::GizmoBuffer};
use bevy::prelude::*;

use super::debug_viz::{w2, with_alpha};

/// What the observers reported this frame, per shown room.
#[derive(Resource, Default, Debug)]
pub struct ObservedGeometryFrame {
    pub rooms: Vec<(LiveRoomInstance, Vec<ObservedLayer>)>,
}

impl ObservedGeometryFrame {
    pub fn in_room(&self, room: LiveRoomInstance) -> &[ObservedLayer] {
        self.rooms.iter().find(|(shown, _)| *shown == room).map_or(&[], |(_, layers)| &layers[..])
    }
}

/// Ask every observer about each shown room, when the layer is on.
pub fn collect_observed_geometry(world: &mut World) {
    let on = world.get_resource::<DeveloperRuntimeState>().is_some_and(DeveloperRuntimeState::debug_enabled)
        && world
            .get_resource::<DeveloperTools>()
            .is_some_and(|tools| tools.gizmos_enabled && tools.show_observed_geometry);
    if !on {
        if let Some(mut frame) = world.get_resource_mut::<ObservedGeometryFrame>() {
            frame.rooms.clear();
        }
        return;
    }
    let shown: std::collections::BTreeSet<LiveRoomInstance> = world
        .query_filtered::<&ambition_sim_view::camera_snapshot::ResolvedCameraSnapshot, With<ambition_sim_view::LocalView>>()
        .iter(world)
        .filter_map(|view| view.frame().map(|frame| frame.room))
        .collect();
    let roots: Vec<(Entity, LiveRoomInstance)> = world
        .query_filtered::<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>()
        .iter(world)
        .map(|(root, room)| (root, *room))
        .filter(|(_, room)| shown.contains(room))
        .collect();
    let players: Vec<(Entity, LiveRoomInstance)> = world
        .query_filtered::<(Entity, &InRoomInstance), With<PrimaryPlayer>>()
        .iter(world)
        .map(|(body, room)| (body, room.0))
        .collect();
    // The installed observers' names, for the menu's cycle; written only when
    // they changed, so the tools are not marked changed every frame.
    let installed: Vec<String> = world
        .get_resource::<ambition_geometry_observation::GeometryObservers>()
        .map(|observers| observers.catalog().map(|info| info.name.to_string()).collect())
        .unwrap_or_default();
    let (only, stale) = {
        let tools = world.resource::<DeveloperTools>();
        (tools.observed_geometry_only.clone(), tools.observed_geometry_providers != installed)
    };
    if stale {
        world.resource_mut::<DeveloperTools>().observed_geometry_providers = installed;
    }
    let mut rooms = Vec::new();
    for (root, room) in roots {
        let subject = players.iter().find(|(_, at)| *at == room).map(|(body, _)| *body);
        rooms.push((
            room,
            observe_all(world, ObservationQuery { room: Some(root), subject }, |provider| {
                only.as_deref().is_none_or(|only| only == provider)
            }),
        ));
    }
    world.get_resource_or_insert_with(ObservedGeometryFrame::default).rooms = rooms;
}

fn role_color(observation: &GeometryObservation) -> Color {
    match observation.roles.first().map(|role| role.as_ref()).unwrap_or("") {
        "solid" | "barrier" => Color::srgba(0.85, 0.85, 0.85, 0.9),
        "one_way" | "bonk_only" => Color::srgba(0.35, 0.55, 1.0, 0.9),
        "blink_wall_soft" => Color::srgba(1.0, 0.35, 1.0, 0.9),
        "blink_wall_hard" | "hazard" => Color::srgba(1.0, 0.3, 0.3, 0.9),
        "pogo_orb" => Color::srgba(0.35, 1.0, 0.45, 0.9),
        "rebound" => Color::srgba(1.0, 0.6, 0.2, 0.9),
        "hurt" => Color::srgba(1.0, 0.9, 0.2, 0.9),
        "hit" => Color::srgba(1.0, 0.2, 0.2, 0.95),
        _ => Color::srgba(0.3, 1.0, 1.0, 0.9),
    }
}

fn outline(gizmos: &mut GizmoBuffer<impl GizmoConfigGroup, ()>, world: &ae::World, shape: &ObservedShape, color: Color, cross: bool) {
    let p = |xy: &[f32; 2]| w2(world, ae::Vec2::new(xy[0], xy[1]));
    match shape {
        ObservedShape::Aabb { min, max } => {
            let corners = [[min[0], min[1]], [max[0], min[1]], [max[0], max[1]], [min[0], max[1]]];
            for i in 0..4 {
                gizmos.line_2d(p(&corners[i]), p(&corners[(i + 1) % 4]), color);
            }
            if cross {
                gizmos.line_2d(p(&corners[0]), p(&corners[2]), color);
                gizmos.line_2d(p(&corners[1]), p(&corners[3]), color);
            }
        }
        ObservedShape::Segment { a, b } => {
            gizmos.line_2d(p(a), p(b), color);
        }
        ObservedShape::Circle { center, radius } => {
            gizmos.circle_2d(p(center), *radius, color);
        }
        ObservedShape::Polygon { points } => {
            for (i, point) in points.iter().enumerate() {
                gizmos.line_2d(p(point), p(&points[(i + 1) % points.len()]), color);
            }
        }
    }
}

/// Draw `layers` (one room's) in that room's coordinates.
pub fn draw_observed_layers(gizmos: &mut GizmoBuffer<impl GizmoConfigGroup, ()>, world: &ae::World, layers: &[ObservedLayer]) {
    for layer in layers {
        let ObserverAnswer::Observed { geometry } = &layer.answer else {
            continue;
        };
        for observation in geometry {
            let color = role_color(observation);
            match observation.state {
                ObservedState::Active => outline(gizmos, world, &observation.shape, color, false),
                ObservedState::Inactive { .. } => outline(gizmos, world, &observation.shape, with_alpha(color, 0.3), true),
            }
        }
    }
}
