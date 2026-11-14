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
//! faint with a cross through it. With one observer selected, each piece is
//! labelled too ([`observation_labels`]): its key, its roles, and why it is
//! inactive. With every observer drawn the labels would bury the room.

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
    let (tools, stale) = {
        let tools = world.resource::<DeveloperTools>();
        (tools.clone(), tools.observed_geometry_providers != installed)
    };
    if stale {
        world.resource_mut::<DeveloperTools>().observed_geometry_providers = installed;
    }
    let mut rooms = Vec::new();
    for (root, room) in roots {
        let subject = players.iter().find(|(_, at)| *at == room).map(|(body, _)| *body);
        rooms.push((
            room,
            observe_all(world, ObservationQuery { room: Some(root), subject }, |provider| tools.draws_observer(provider)),
        ));
    }
    world.get_resource_or_insert_with(ObservedGeometryFrame::default).rooms = rooms;
}

fn role_color(observation: &GeometryObservation) -> Color {
    known_role_color(observation.roles.first().map(|role| role.as_ref()).unwrap_or(""))
        .unwrap_or(Color::srgba(0.3, 1.0, 1.0, 0.9))
}

/// The colour F1 draws a role in, if it has one of its own.
fn known_role_color(role: &str) -> Option<Color> {
    Some(match role {
        "solid" | "barrier" => Color::srgba(0.85, 0.85, 0.85, 0.9),
        "one_way" | "bonk_only" => Color::srgba(0.35, 0.55, 1.0, 0.9),
        "blink_wall_soft" => Color::srgba(1.0, 0.35, 1.0, 0.9),
        "blink_wall_hard" | "hazard" => Color::srgba(1.0, 0.3, 0.3, 0.9),
        "pogo_orb" => Color::srgba(0.35, 1.0, 0.45, 0.9),
        "rebound" => Color::srgba(1.0, 0.6, 0.2, 0.9),
        "hurt" => Color::srgba(1.0, 0.9, 0.2, 0.9),
        "hit" => Color::srgba(1.0, 0.2, 0.2, 0.95),
        // A body as the kernel swept it: its box, its path, and the edges it
        // reported touching.
        "body_box" => Color::srgba(0.75, 0.55, 1.0, 0.9),
        "sweep" => Color::srgba(0.75, 0.55, 1.0, 0.6),
        "contact_ground" | "contact_head" | "contact_wall" => Color::srgba(0.2, 1.0, 0.2, 1.0),
        // What an interact press reaches.
        "interact_touch" | "interact_talk" | "interact_front" => Color::srgba(1.0, 0.75, 0.3, 0.8),
        "talk" | "switch" | "chest" => Color::srgba(1.0, 0.95, 0.6, 0.9),
        // Navigation, in the overlay's colours.
        "nav_surface" => Color::srgba(0.2, 1.0, 0.2, 0.9),
        "nav_hop" => Color::srgba(1.0, 1.0, 0.2, 0.9),
        "nav_drop" => Color::srgba(1.0, 0.6, 0.1, 0.9),
        "nav_route" | "nav_goal" => Color::srgba(1.0, 0.2, 1.0, 0.9),
        "nav_target_place" => Color::srgba(0.2, 1.0, 1.0, 0.9),
        // The room's regions that are not walls.
        "loading_zone" => Color::srgba(0.3, 0.6, 1.0, 0.9),
        "camera_zone" => Color::srgba(0.6, 0.6, 0.6, 0.6),
        "gravity_zone" => Color::srgba(0.6, 0.3, 1.0, 0.8),
        "encounter_trigger" => Color::srgba(1.0, 0.45, 0.45, 0.8),
        "lock_wall" => Color::srgba(1.0, 0.7, 0.2, 0.9),
        // A portal's opening.
        "portal" => Color::srgba(0.85, 0.4, 1.0, 0.95),
        _ => return None,
    })
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

/// A label for one observed piece, in room pixels (y down).
#[derive(Clone, Debug, PartialEq)]
pub struct ObservedLabel {
    pub at: ae::Vec2,
    pub text: String,
    pub color: Color,
}

/// The labels of `layers` (one room's): for each piece, its key and roles,
/// and the owner's reason when it is inactive, at the piece's top-left
/// corner (a box), its middle (a segment) or its centre (a circle).
pub fn observation_labels(layers: &[ObservedLayer]) -> Vec<ObservedLabel> {
    const INSET: f32 = 3.0;
    let mut labels = Vec::new();
    for layer in layers {
        let ObserverAnswer::Observed { geometry } = &layer.answer else {
            continue;
        };
        for observation in geometry {
            let at = match &observation.shape {
                ObservedShape::Aabb { min, .. } => ae::Vec2::new(min[0] + INSET, min[1] + INSET),
                ObservedShape::Segment { a, b } => ae::Vec2::new((a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5),
                ObservedShape::Circle { center, .. } => ae::Vec2::new(center[0], center[1]),
                ObservedShape::Polygon { points } => {
                    let Some(first) = points.first() else { continue };
                    ae::Vec2::new(first[0], first[1])
                }
            };
            let roles: Vec<&str> = observation.roles.iter().map(|role| role.as_ref()).collect();
            let mut text = format!("{} [{}]", observation.key, roles.join(", "));
            let mut color = role_color(observation);
            if let ObservedState::Inactive { reason } = &observation.state {
                text.push_str(&format!(" off: {reason}"));
                color = with_alpha(color, 0.6);
            }
            labels.push(ObservedLabel { at, text, color });
        }
    }
    labels
}

/// The legend of `layers` (one room's), a line each, top first: how many
/// pieces are drawn and how many of them are inactive, then each role that
/// colours a piece, in its colour (GOBS-4). Empty when nothing was observed.
pub fn observation_legend(layers: &[ObservedLayer]) -> Vec<(String, Color)> {
    let mut pieces = 0;
    let mut inactive = 0;
    let mut roles: Vec<(String, Color)> = Vec::new();
    for layer in layers {
        let ObserverAnswer::Observed { geometry } = &layer.answer else {
            continue;
        };
        for observation in geometry {
            pieces += 1;
            if matches!(observation.state, ObservedState::Inactive { .. }) {
                inactive += 1;
            }
            // A role F1 has no colour for is drawn in the shared fallback
            // colour; the legend says so, so it is not mistaken for another.
            let role = observation.roles.first().map_or("", |role| role.as_ref());
            let line = match known_role_color(role) {
                Some(_) => role.to_string(),
                None => format!("{role} (no colour of its own)"),
            };
            if !roles.iter().any(|(seen, _)| *seen == line) {
                roles.push((line, role_color(observation)));
            }
        }
    }
    if pieces == 0 {
        return Vec::new();
    }
    let mut lines = vec![(format!("observed: {pieces} pieces, {inactive} inactive (faint, crossed)"), Color::WHITE)];
    lines.extend(roles);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_geometry_observation::GeometryObservation;

    /// The legend counts the pieces and the inactive ones, then names each
    /// role that colours a piece once, in its colour.
    #[test]
    fn the_legend_names_each_drawn_role_in_its_colour() {
        let piece = |key: &str, role: &'static str, state| GeometryObservation {
            key: key.into(),
            roles: vec![role.into()],
            shape: ObservedShape::aabb([0.0, 0.0], [1.0, 1.0]),
            state,
        };
        let layers = vec![ObservedLayer {
            provider: "test",
            answer: ObserverAnswer::Observed {
                geometry: vec![
                    piece("a", "solid", ObservedState::Active),
                    piece("b", "solid", ObservedState::Inactive { reason: "broken".into() }),
                    piece("c", "portal", ObservedState::Active),
                    piece("d", "a_new_domains_role", ObservedState::Active),
                ],
            },
        }];
        let legend = observation_legend(&layers);
        let words: Vec<&str> = legend.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(
            words,
            vec![
                "observed: 4 pieces, 1 inactive (faint, crossed)",
                "solid",
                "portal",
                "a_new_domains_role (no colour of its own)",
            ]
        );
        assert_eq!(legend[2].1, role_color(&layers_piece(&layers, 2)), "a role is written in the colour it is drawn in");
        assert!(observation_legend(&[]).is_empty());
    }

    fn layers_piece(layers: &[ObservedLayer], n: usize) -> GeometryObservation {
        let ObserverAnswer::Observed { geometry } = &layers[0].answer else { unreachable!() };
        geometry[n].clone()
    }

    /// Each piece is labelled with its key and roles at its anchor, and an
    /// inactive one says why; a provider that observed nothing adds none.
    #[test]
    fn each_observed_piece_is_labelled_with_its_key_roles_and_reason() {
        let layers = vec![
            ObservedLayer {
                provider: "collision.composed",
                answer: ObserverAnswer::Observed {
                    geometry: vec![
                        GeometryObservation {
                            key: "floor".into(),
                            roles: vec!["solid".into()],
                            shape: ObservedShape::aabb([10.0, 20.0], [50.0, 30.0]),
                            state: ObservedState::Active,
                        },
                        GeometryObservation {
                            key: "gate".into(),
                            roles: vec!["barrier".into(), "one_way".into()],
                            shape: ObservedShape::Segment { a: [0.0, 0.0], b: [10.0, 4.0] },
                            state: ObservedState::Inactive { reason: "open".into() },
                        },
                    ],
                },
            },
            ObservedLayer { provider: "combat.volumes", answer: ObserverAnswer::NeedsSubject },
        ];
        let labels = observation_labels(&layers);
        let read: Vec<(ae::Vec2, &str)> = labels.iter().map(|label| (label.at, label.text.as_str())).collect();
        assert_eq!(
            read,
            vec![
                (ae::Vec2::new(13.0, 23.0), "floor [solid]"),
                (ae::Vec2::new(5.0, 2.0), "gate [barrier, one_way] off: open"),
            ]
        );
    }
}
