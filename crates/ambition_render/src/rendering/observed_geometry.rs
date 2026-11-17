//! F1's observed-geometry layer (GOBS-1,
//! `docs/planning/engine/geometry-observability-and-f1.md`).
//!
//! It draws whatever the installed geometry observers report for each shown
//! room ([`ambition_geometry_observation::observe_all`], the same call
//! headless inspection makes), with each view's selected subject.
//! A new observer is drawn with no edit here. Nothing is asked for unless
//! the debug overlay, gizmos and `show_observed_geometry` are all on.
//!
//! Colour says what gameplay does with a piece (its first role); an inactive
//! piece (a broken platform's block, a gate open for the player) is drawn
//! faint with a cross through it. With one observer selected, each piece is
//! labelled too ([`observation_labels`]): its key, its roles, and why it is
//! inactive. With every observer drawn the labels would bury the room.

use ambition_dev_tools::DeveloperRuntimeState;
use ambition_dev_tools::dev_tools::{DeveloperTools, GeometrySubjectSelection};
use ambition_geometry_observation::{
    GeometryObservation, ObservationQuery, ObservedLayer, ObservedShape, ObservedState, ObserverAnswer,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;
use bevy::gizmos::{config::GizmoConfigGroup, gizmos::GizmoBuffer};
use bevy::prelude::*;

use super::debug_viz::{w2, with_alpha};

/// Install collection and drawing without the generic F1 input or body layers.
pub struct ObservedGeometryPlugin;

#[derive(Component, Default)]
struct ObservedGeometryDrawing;

impl Plugin for ObservedGeometryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ObservedGeometryFrame>();
        app.add_systems(Update, (
            collect_observed_geometry
                .after(ambition_sim_view::PresentedPoseSet)
                .after(ambition_sim_view::camera_snapshot::CameraObservationSet),
            draw_observed_geometry.run_if(bevy::ecs::schedule::common_conditions::resource_exists::<Assets<bevy::gizmos::GizmoAsset>>),
        ).chain());
    }
}

fn draw_observed_geometry(
    mut drawings: super::retained_drawings::RetainedDrawings<ObservedGeometryDrawing>,
    frame: Res<ObservedGeometryFrame>,
    worlds: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<ae::RoomGeometry>,
    state: Res<DeveloperRuntimeState>,
    tools: Res<DeveloperTools>,
) {
    if state.debug_enabled() && tools.gizmos_enabled && tools.show_observed_geometry {
        for shown in &frame.views {
            let (Some(world), Ok(report)) = (worlds.in_room(shown.room), &shown.report) else { continue };
            drawings.draw(super::retained_drawings::DrawingScope::View(shown.view), |gizmos| {
                draw_observed_layers(gizmos, &world.0, &report.layers);
            });
        }
    }
    drawings.finish();
}

/// One view's mechanical geometry and its query context.
#[derive(Debug)]
pub struct ObservedViewGeometry {
    pub view: Entity,
    pub room: LiveRoomInstance,
    pub report: Result<ambition_geometry_observation::GeometryReport, String>,
}

/// On-demand reports for the local views. No report survives a disabled frame.
#[derive(Resource, Default, Debug)]
pub struct ObservedGeometryFrame {
    pub views: Vec<ObservedViewGeometry>,
}

impl ObservedGeometryFrame {
    pub fn in_view(&self, view: Entity) -> Option<&ObservedViewGeometry> {
        self.views.iter().find(|shown| shown.view == view)
    }
}

/// Collect each view with its own subject. An explicit selection must be in
/// the shown room; a foreign or stale selection produces a refusal.
pub fn collect_observed_geometry(world: &mut World) {
    let on = world.get_resource::<DeveloperRuntimeState>().is_some_and(DeveloperRuntimeState::debug_enabled)
        && world.get_resource::<DeveloperTools>()
            .is_some_and(|tools| tools.gizmos_enabled && tools.show_observed_geometry);
    if !on {
        if let Some(mut frame) = world.get_resource_mut::<ObservedGeometryFrame>() {
            frame.views.clear();
        }
        return;
    }
    let shown: Vec<_> = world.query_filtered::<(
        Entity, &ambition_sim_view::camera_snapshot::ResolvedCameraSnapshot,
        Option<&ambition_sim_view::ResolvedViewSubject>,
    ), With<ambition_sim_view::LocalView>>()
        .iter(world).filter_map(|(view, camera, subject)| {
            camera.frame().map(|frame| (view, frame.room, subject.and_then(|subject| subject.0)))
        }).collect();
    let roots: Vec<_> = world.query_filtered::<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>()
        .iter(world).map(|(root, room)| (root, *room)).collect();
    let subjects = ambition_sim_view::geometry_inspection::geometry_subjects(world);
    let installed: Vec<String> = world.get_resource::<ambition_geometry_observation::GeometryObservers>()
        .map(|observers| observers.catalog().map(|info| info.name.to_string()).collect()).unwrap_or_default();
    let tools = world.resource::<DeveloperTools>().clone();
    {
        let mut live_tools = world.resource_mut::<DeveloperTools>();
        live_tools.observed_geometry_providers = installed;
        live_tools.observed_geometry_subjects = subjects.iter().map(|(id, _)| id.clone()).collect();
    }
    let default_subject = world.get_resource::<ambition_platformer2d_shared_tangle::markers::ControlledSubject>()
        .and_then(|subject| subject.0)
        .or_else(|| world.query_filtered::<Entity, With<PrimaryPlayer>>().single(world).ok());
    let mut views = Vec::new();
    for (view, room, framed_subject) in shown {
        let root = roots.iter().find(|(_, at)| *at == room).map(|(root, _)| *root);
        let subject = match &tools.observed_geometry_subject {
            GeometrySubjectSelection::Body(id) => subjects.iter().find(|(name, _)| name == id).map(|(_, entity)| *entity),
            GeometrySubjectSelection::Room => None,
            GeometrySubjectSelection::View => framed_subject.or(default_subject).filter(|entity| world.get::<InRoomInstance>(*entity).map(|at| at.0) == Some(room)),
        };
        let report = if matches!(tools.observed_geometry_subject, GeometrySubjectSelection::Body(_)) && subject.is_none() {
            Err("the selected subject is no longer live".into())
        } else {
            ambition_sim_view::geometry_inspection::inspect_geometry(world,
                ObservationQuery { room: root, subject }, |provider| tools.draws_observer(provider))
        };
        views.push(ObservedViewGeometry { view, room, report });
    }
    world.get_resource_or_insert_with(ObservedGeometryFrame::default).views = views;
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
        // A line of the world: a slope, a hill, a mount's back.
        "line" => Color::srgba(0.2, 1.0, 0.75, 0.95),
        "solid_line" => Color::srgba(0.85, 0.85, 0.85, 0.9),
        "water" => Color::srgba(0.25, 0.55, 1.0, 0.6),
        "climbable" => Color::srgba(0.7, 0.5, 0.25, 0.8),
        "rebound" => Color::srgba(1.0, 0.6, 0.2, 0.9),
        "hurt" => Color::srgba(1.0, 0.9, 0.2, 0.9),
        "hit" => Color::srgba(1.0, 0.2, 0.2, 0.95),
        // A body as the kernel swept it: its box, its path, and the edges it
        // reported touching.
        "body_material" | "projectile" | "body_box" => Color::srgba(0.75, 0.55, 1.0, 0.9),
        "sweep" => Color::srgba(0.75, 0.55, 1.0, 0.6),
        "contact_ground" | "contact_head" | "contact_wall" => Color::srgba(0.2, 1.0, 0.2, 1.0),
        // What an interact press reaches.
        "interact_touch" | "interact_talk" | "interact_front" => Color::srgba(1.0, 0.75, 0.3, 0.8),
        "talk" | "switch" | "chest" | "door" => Color::srgba(1.0, 0.95, 0.6, 0.9),
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

/// A display outline only. The report keeps circles and cuts analytic.
fn display_outline(shape: &ObservedShape) -> Vec<[f32; 2]> {
    match shape {
        ObservedShape::Aabb { min, max } => vec![*min, [max[0], min[1]], *max, [min[0], max[1]]],
        ObservedShape::Segment { a, b } => vec![*a, *b],
        ObservedShape::Polyline { points } | ObservedShape::Polygon { points } => points.clone(),
        ObservedShape::Circle { center, radius } => (0..128).map(|n| {
            let angle = n as f32 * std::f32::consts::TAU / 128.0;
            [center[0] + radius * angle.cos(), center[1] + radius * angle.sin()]
        }).collect(),
        ObservedShape::Clipped { primitive, planes, transform, .. } => {
            let mut points: Vec<[f64; 2]> = display_outline(primitive).into_iter().map(|p| p.map(f64::from)).collect();
            for plane in planes {
                let mut kept = Vec::new();
                for (i, a) in points.iter().enumerate() {
                    let b = points[(i + 1) % points.len()];
                    let side = |p: [f64; 2]| plane.inward[0] * p[0] + plane.inward[1] * p[1] - plane.offset;
                    let (sa, sb) = (side(*a), side(b));
                    if sa >= 0.0 { kept.push(*a); }
                    if (sa >= 0.0) != (sb >= 0.0) {
                        let t = sa / (sa - sb);
                        kept.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
                    }
                }
                points = kept;
            }
            points.into_iter().map(|p| transform.map(|row| (row[0] * p[0] + row[1] * p[1] + row[2]) as f32)).collect()
        }
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
        }
        ObservedShape::Segment { a, b } => {
            gizmos.line_2d(p(a), p(b), color);
        }
        ObservedShape::Circle { center, radius } => {
            gizmos.circle_2d(p(center), *radius, color);
        }
        ObservedShape::Polyline { points } => {
            for pair in points.windows(2) {
                gizmos.line_2d(p(&pair[0]), p(&pair[1]), color);
            }
            // Each vertex marked, so a slope's joins and a short segment show.
            for point in points {
                gizmos.circle_2d(p(point), 2.5, color);
            }
        }
        ObservedShape::Clipped { .. } => {
            let points = display_outline(shape);
            for (i, point) in points.iter().enumerate() {
                gizmos.line_2d(p(point), p(&points[(i + 1) % points.len()]), color);
            }
        }
        ObservedShape::Polygon { points } => {
            for (i, point) in points.iter().enumerate() {
                gizmos.line_2d(p(point), p(&points[(i + 1) % points.len()]), color);
            }
        }
    }
    if cross {
        let points = display_outline(shape);
        if let Some(first) = points.first() {
            let (mut min, mut max) = (*first, *first);
            for point in &points {
                for axis in 0..2 {
                    min[axis] = min[axis].min(point[axis]);
                    max[axis] = max[axis].max(point[axis]);
                }
            }
            gizmos.line_2d(p(&min), p(&max), color);
            gizmos.line_2d(p(&[min[0], max[1]]), p(&[max[0], min[1]]), color);
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
                ObservedShape::Clipped { .. } => {
                    let points = display_outline(&observation.shape);
                    let Some(first) = points.first() else { continue };
                    ae::Vec2::new(first[0], first[1])
                }
                ObservedShape::Polyline { points } | ObservedShape::Polygon { points } => {
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
    let mut diagnostics = Vec::new();
    for layer in layers {
        let geometry = match &layer.answer {
            ObserverAnswer::Observed { geometry } => {
                if geometry.is_empty() {
                    diagnostics.push((format!("{}: no geometry at this tick", layer.provider), Color::WHITE));
                }
                geometry
            }
            ObserverAnswer::NeedsSubject => {
                diagnostics.push((format!("{}: select a subject", layer.provider), Color::srgb(1.0, 0.7, 0.3)));
                continue;
            }
            ObserverAnswer::Unavailable { reason } => {
                diagnostics.push((format!("{} unavailable: {reason}", layer.provider), Color::srgb(1.0, 0.3, 0.3)));
                continue;
            }
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
    if layers.is_empty() {
        return vec![("no geometry observers selected".into(), Color::WHITE)];
    }
    let mut lines = vec![(format!("observed: {pieces} pieces, {inactive} inactive (faint, crossed)"), Color::WHITE)];
    lines.extend(roles);
    lines.extend(diagnostics);
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
        assert!(observation_legend(&[])[0].0.contains("no geometry observers"));
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
    #[test]
    fn the_legend_distinguishes_empty_missing_subject_and_unavailable() {
        let layers = vec![
            ObservedLayer { provider: "empty", answer: ObserverAnswer::Observed { geometry: vec![] } },
            ObservedLayer { provider: "subject", answer: ObserverAnswer::NeedsSubject },
            ObservedLayer { provider: "absent", answer: ObserverAnswer::Unavailable { reason: "no authority".into() } },
        ];
        let lines: Vec<_> = observation_legend(&layers).into_iter().map(|(line, _)| line).collect();
        assert!(lines.iter().any(|line| line == "empty: no geometry at this tick"));
        assert!(lines.iter().any(|line| line == "subject: select a subject"));
        assert!(lines.iter().any(|line| line == "absent unavailable: no authority"));
    }

    #[test]
    fn a_clipped_circle_draws_its_arc_instead_of_its_bounding_box() {
        let shape = ObservedShape::Clipped {
            primitive: Box::new(ObservedShape::Circle { center: [0.0, 0.0], radius: 20.0 }),
            planes: vec![ambition_geometry_observation::ObservedPlane { inward: [0.0, 1.0], offset: 0.0 }],
            transform: [[-0.5, 0.0, 500.0], [0.0, 0.5, -200.0]], path: vec![0],
        };
        let points = display_outline(&shape);
        assert!(points.len() > 32);
        assert!(points.iter().all(|p| p[1] >= -200.0 && (p[0] - 500.0).hypot(p[1] + 200.0) < 10.001));
        assert!(points.iter().any(|p| p[1] > -190.01));
    }

}
