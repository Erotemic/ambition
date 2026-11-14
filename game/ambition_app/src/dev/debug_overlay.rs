//! Debug drawing for the Bevy sandbox backend.
//!
//! These overlays intentionally live in the Bevy adapter layer. The movement
//! engine exposes simulation state; this module decides how to visualize that
//! state for tuning and feel work.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
use ambition_platformer2d::render::rendering::retained_drawings::{DrawingScope, RetainedDrawings};
use bevy::ecs::system::SystemParam;
use bevy::gizmos::{config::GizmoConfigGroup, gizmos::GizmoBuffer};
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use ambition_platformer2d::dev_tools::DeveloperRuntimeState;
use ambition_platformer2d::dev_tools::dev_tools::DeveloperTools;
use ambition_platformer2d::engine_core::RoomGeometry;
use ambition_platformer2d::engine_core::config::world_to_bevy;
#[cfg(feature = "input")]
use ambition_platformer2d::input::Platformer2dInputActionMonolith;
use ambition_platformer2d::input::read_gameplay_control_frame;
use ambition_platformer2d::platformer::schedule::GameMode;
use ambition_platformer2d::render::rendering::CameraViewState;
use ambition_platformer2d::world::rooms::{LoadingZone, LoadingZoneActivation};
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

mod gizmos;
mod prims;
pub use gizmos::*;
pub use prims::*;

// The engine-generic palette, primitives, and world layers live in the shared
// debug-viz module now (`DebugVizPlugin` gives them to any game); this richer
// overlay composes them with its game-specific layers below.
pub(crate) use ambition_platformer2d::render::rendering::debug_viz::{
    blue, cyan, draw_aabb, draw_aabb_styled, draw_arrow, draw_camera_frame,
    draw_combat_geometry_view, draw_combat_volume, draw_hitbox_volume, draw_world_geometry,
    engine_delta_to_bevy, gray, green, magenta, presentation_deltas, red, w2, with_alpha,
    yellow,
};

#[derive(Component, Default)]
pub(crate) struct DebugOverlayDrawing;

#[derive(Component, Default)]
pub(crate) struct DebugLabelDrawing;

/// Draw labels through their room's geometry and native render mask.
pub(crate) fn render_debug_overlay_labels(
    mut drawings: RetainedDrawings<DebugLabelDrawing>,
    worlds: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<RoomGeometry>,
    mut labels: ResMut<DebugOverlayLabels>,
) {
    let rooms: std::collections::BTreeSet<_> = labels.0.iter().map(|label| label.room).collect();
    for room in rooms {
        let Some(world) = worlds.in_room(room) else {
            continue;
        };
        drawings.draw(DrawingScope::Room(room), |picture| {
            for label in labels.0.iter().filter(|label| label.room == room) {
                picture.text_2d(
                    world_to_bevy(&world.0, label.world_pos, 0.0).truncate(),
                    &label.text,
                    DEBUG_LABEL_FONT_PX,
                    Vec2::ZERO,
                    label.color,
                );
            }
        });
    }
    labels.0.clear();
    drawings.finish();
}

/// Label the observed geometry (GOBS-1) when the Geometry Observer row has
/// one observer selected: with every observer drawn, the labels would bury
/// the room. Runs after the overlay, which clears the labels, and before
/// they are drawn.
pub(crate) fn label_observed_geometry(
    developer_tools: Res<DeveloperTools>,
    frame: Option<Res<ambition_platformer2d::render::rendering::observed_geometry::ObservedGeometryFrame>>,
    mut overlay_labels: ResMut<DebugOverlayLabels>,
) {
    let Some(frame) = frame else { return };
    if developer_tools.observed_geometry_only.is_none() {
        return;
    }
    for (room, layers) in &frame.rooms {
        for label in ambition_platformer2d::render::rendering::observed_geometry::observation_labels(layers) {
            overlay_labels.0.push(DebugLabel { room: *room, world_pos: label.at, text: label.text, color: label.color });
        }
    }
}

#[cfg(not(feature = "input"))]
pub(crate) fn draw_debug_overlay() {}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_debug_overlay(
    mut drawings: RetainedDrawings<DebugOverlayDrawing>,
    worlds: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<RoomGeometry>,
    dev_state: Res<DeveloperRuntimeState>,
    platform_sets: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<
        ambition_platformer2d::world::collision::MovingPlatformSet,
    >,
    collision: ambition_platformer2d::world::collision::CollisionWorld,
    developer_tools: Res<DeveloperTools>,
    room_specs: ambition_platformer2d::world::rooms::LiveRoomSpecs,
    ldtk_spine_index: Res<ambition_platformer2d::ldtk_map::LdtkRuntimeSpineIndex>,
    views: Query<
        (
            Entity,
            &ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot,
            &CameraViewState,
        ),
        With<ambition_platformer2d::sim_view::LocalView>,
    >,
    mode: Res<State<GameMode>>,
    mut overlay_labels: ResMut<DebugOverlayLabels>,
    action_query: Query<(
        &ambition_platformer2d::input::InputParticipant,
        &ActionState<Platformer2dInputActionMonolith>,
    )>,
    bodies: Query<(
        Entity,
        ae::body_clusters::BodyClusterQueryDataReadOnly,
        &ae::MotionModel,
        Option<&ambition_platformer2d::characters::actor::BodyHealth>,
        ambition_platformer2d::combat::moveset::MeleeSwingQuery,
        &ambition_platformer2d::characters::actor::CharacterForm,
        Option<&ambition_platformer2d::sim_view::PresentedPose>,
        Option<&ambition_platformer2d::combat::components::ActorRenderSize>,
        Option<&ambition_platformer2d::characters::control::DrivingParticipant>,
        &ambition_platformer2d::platformer::frame_env::ResolvedMotionFrame,
    )>,
    mut feature_q: FeatureDebugQueries,
    #[cfg(feature = "portal")] portals: Query<(
        Entity,
        &ambition_platformer2d::portal::PlacedPortal,
    )>,
) {
    overlay_labels.0.clear();
    if !dev_state.debug_enabled() || !developer_tools.gizmos_enabled {
        drawings.finish();
        return;
    }
    feature_q
        .bounce_trail
        .0
        .retain(|room, _| worlds.in_room(*room).is_some());
    let shown: std::collections::BTreeSet<_> = views
        .iter()
        .filter_map(|(_, view, _)| view.frame().map(|frame| frame.room))
        .collect();
    let gameplay_active = mode.get().allows_gameplay();
    for live in shown {
        let (Some(world), Some(definition)) =
            (worlds.in_room(live), room_specs.definition_in(live))
        else {
            continue;
        };
        let world = &world.0;
        let room_spec = room_specs.rooms().spec(definition);
        let mut labels = RoomLabels {
            room: live,
            labels: &mut overlay_labels.0,
        };
        drawings.draw(DrawingScope::Room(live), |gizmos| {
            draw_world_geometry(
                gizmos,
                world,
                &developer_tools,
                platform_sets
                    .in_room(live)
                    .map_or(&[][..], |platforms| &platforms.0[..]),
                feature_q
                    .base_gravity
                    .as_deref()
                    .map_or(ae::Vec2::Y, |gravity| gravity.dir_in(Some(live))),
            );
            if developer_tools.show_loading_zones {
                draw_loading_zones(gizmos, world, &room_spec.loading_zones);
                if ldtk_spine_index.active_area == room_spec.id {
                    draw_ldtk_runtime_spine(gizmos, world, &ldtk_spine_index);
                }
            }
            let combat = ambition_platformer2d::render::rendering::debug_viz::combat_rows_in(
                &feature_q.combat_geometry,
                live,
                |body| worlds.room_of(body),
            );
            draw_combat_geometry_view(
                gizmos,
                world,
                &combat,
                &developer_tools,
                &presentation_deltas(&combat, &feature_q.body_deltas),
            );
            for (
                entity,
                clusters,
                motion_model,
                health,
                attack,
                character,
                presented,
                drawn,
                driver,
                frame,
            ) in bodies
                .iter()
                .filter(|(entity, ..)| worlds.room_of(*entity) == Some(live))
            {
                let body_gameplay = gameplay_active
                    && driver.is_none_or(|driver| {
                        feature_q
                            .input_contexts
                            .as_deref()
                            .is_none_or(|contexts| contexts.gameplay_owned(driver.0.0))
                    });
                let actions = driver.filter(|_| body_gameplay).and_then(|driver| {
                    action_query
                        .iter()
                        .find(|(participant, _)| participant.id.0 == driver.0.0)
                        .map(|(_, actions)| actions)
                });
                let draw_pos =
                    presented.map_or(clusters.kinematics.pos, |presented| presented.presented());
                let solids = collision
                    .room(Some(&InRoomInstance(live)))
                    .and_then(|room| room.solids_for(entity));
                draw_body_debug(
                    gizmos,
                    world,
                    &feature_q.character_catalog,
                    &feature_q.authored_attack_volumes,
                    character.id(),
                    drawn.map(|quad| quad.0),
                    &clusters,
                    draw_pos,
                    motion_model,
                    solids.as_deref().unwrap_or(world),
                    attack.swing().as_ref(),
                    actions,
                    body_gameplay,
                    &developer_tools,
                    frame.down(),
                    &mut labels,
                );
                if developer_tools.show_health_bars {
                    let delta = presented.map_or(ae::Vec2::ZERO, |presented| presented.delta());
                    draw_health_bars(
                        gizmos,
                        world,
                        clusters
                            .kinematics
                            .aabb()
                            .translated(delta),
                        health,
                    );
                }
            }
            if developer_tools.show_feature_hitboxes {
                draw_feature_debug(gizmos, world, &feature_q, &developer_tools, &mut labels);
                #[cfg(feature = "portal")]
                draw_portals(
                    gizmos,
                    world,
                    portals
                        .iter()
                        .filter(|(entity, _)| worlds.room_of(*entity) == Some(live))
                        .map(|(_, portal)| portal),
                );
            }
            if developer_tools.show_feature_hitboxes
                || developer_tools.show_player_hitbox
                || developer_tools.show_combat_preview
            {
                draw_projectile_debug(gizmos, world, &mut feature_q, &developer_tools, &mut labels);
                label_combat_properties(&feature_q, &mut labels);
            }
            if developer_tools.show_feature_hitboxes || developer_tools.show_player_hitbox {
                draw_ground_item_debug(
                    gizmos,
                    world,
                    feature_q.ground_items.as_deref(),
                    live,
                    &developer_tools,
                );
            }
        });
    }
    if developer_tools.show_camera_frame {
        for (view, resolved, camera) in &views {
            if let Some(world) = resolved
                .frame()
                .and_then(|frame| worlds.in_room(frame.room))
            {
                drawings.draw(DrawingScope::View(view), |gizmos| {
                    draw_camera_frame(gizmos, &world.0, camera)
                });
            }
        }
    }
    drawings.finish();
}

#[cfg(all(test, feature = "input"))]
mod two_live_rooms_tests {
    use bevy::prelude::*;

    use ambition_platformer2d::dev_tools::DeveloperRuntimeState;
    use ambition_platformer2d::dev_tools::dev_tools::DeveloperTools;
    use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
    use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance};
    use ambition_platformer2d::world::rooms::{LiveRoomDefinition, RoomSet};

    /// Whether the overlay drew the player's label on the last frame, and how
    /// many labels the renderer left. The renderer drains them in the same
    /// frame, so one probe reads them between the two and one after the
    /// renderer. It holds no collection, so it is not a per-attempt resource.
    #[derive(Resource, Default)]
    struct DrawnLabels(bool, usize);

    fn record_drawn_labels(labels: Res<super::DebugOverlayLabels>, mut drawn: ResMut<DrawnLabels>) {
        drawn.0 = labels.0.iter().any(|label| label.text == "body");
    }

    fn record_undrawn_labels(
        labels: Res<super::DebugOverlayLabels>,
        mut drawn: ResMut<DrawnLabels>,
    ) {
        drawn.1 = labels.0.len();
    }

    /// Whether the overlay drew the player's label, how many labels the
    /// renderer did not draw, and the developer HUD's text.
    fn shown(app: &mut App) -> (bool, usize, String) {
        app.update();
        let hud = app
            .world_mut()
            .query_filtered::<&Text, With<ambition_platformer2d::render::rendering::HudText>>()
            .single(app.world())
            .expect("one developer HUD")
            .0
            .clone();
        let drawn = app.world().resource::<DrawnLabels>();
        (drawn.0, drawn.1, hud)
    }

    /// OW1: with two live rooms, the developer overlay and the developer HUD
    /// show the room of the primary body. Before, each read the sole live room
    /// and did not run while two rooms were live: the overlay drew no label
    /// and the HUD stood still on the room it showed last. The control is the
    /// same app with one live room.
    #[test]
    fn the_developer_overlay_and_hud_show_the_primary_bodys_room_while_two_rooms_are_live() {
        let mut app = crate::app::build_visible_app(crate::app::VisibleRenderMode::NoWindow, true);
        app.init_resource::<DrawnLabels>();
        app.add_systems(
            Update,
            record_drawn_labels
                .after(super::draw_debug_overlay)
                .before(super::render_debug_overlay_labels),
        );
        app.add_systems(
            Update,
            record_undrawn_labels.after(super::render_debug_overlay_labels),
        );
        app.finish();
        app.update();
        app.world_mut().write_message(ShellCommand::ReplaceWith {
            route: ShellRouteId::new("ambition_gameplay"),
            request: None,
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while app
            .world_mut()
            .query_filtered::<Entity, With<ambition_platformer2d::render::rendering::HudText>>()
            .iter(app.world())
            .next()
            .is_none()
        {
            if std::time::Instant::now() >= deadline {
                use ambition_platformer2d::{
                    load::LoadCoordinator, load_presentation::LoadPresentations,
                };
                let snapshots: Vec<_> = app.world().get_resource::<LoadPresentations>().into_iter()
                    .flat_map(|state| state.iter()).filter_map(|entry| {
                        app.world().get_resource::<LoadCoordinator>().and_then(|coordinator|
                            coordinator.snapshot(&entry.foreground.barrier.load_id, &entry.foreground.barrier.barrier_id))
                    }).collect();
                panic!("the session did not publish its HUD: {snapshots:#?}");
            }
            app.update();
            std::thread::yield_now();
        }
        app.world_mut()
            .resource_mut::<DeveloperRuntimeState>()
            .debug = true;
        {
            let mut tools = app.world_mut().resource_mut::<DeveloperTools>();
            tools.gizmos_enabled = true;
            tools.show_player_hitbox = true;
            tools.show_hud = true;
            tools.compact_hud = false;
        }
        app.world_mut()
            .resource_mut::<ambition_platformer2d::persistence::settings::UserSettings>()
            .gameplay
            .debug_hud_visible = true;

        let (player_label, undrawn, hud) = shown(&mut app);
        assert!(
            player_label,
            "control: one live room: the overlay drew no player label"
        );
        assert_eq!(
            undrawn, 0,
            "control: one live room: the renderer drew every label"
        );
        let (definition, geometry) = {
            let world = app.world_mut();
            let mut roots = world.query_filtered::<(
                &LiveRoomDefinition,
                &ambition_platformer2d::engine_core::RoomGeometry,
            ), With<ambition_platformer2d::session::RoomInstanceRoot>>(
            );
            let (definition, geometry) =
                roots.single(world).expect("the session has one live room");
            (*definition, geometry.clone())
        };
        assert!(
            hud.starts_with(&format!("P1 inspector: {}  ", geometry.0.name)),
            "control: the HUD names the one live room: {hud:?}"
        );

        // A second live room, of another definition and a geometry with its
        // own name. The primary body stands in it.
        let other = {
            let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
                RoomSet,
            >(app.world())
            .expect("the session has a room set");
            let id = &rooms
                .rooms
                .iter()
                .find(|spec| rooms.definition_by_id(&spec.id) != Some(definition))
                .expect("the shipped game has two rooms")
                .id;
            rooms.definition_by_id(id).unwrap()
        };
        let mut second_geometry = geometry.clone();
        second_geometry.0.name = "the second live room".to_string();
        let second_room = LiveRoomInstance::from_ordinal(1_000);
        let second = ambition_platformer2d::platformer::lifecycle::spawn_live_room(
            app.world_mut(),
            second_room,
            other,
        );
        app.world_mut().entity_mut(second).insert(second_geometry);
        let primary = app
            .world_mut()
            .query_filtered::<Entity, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(app.world())
            .expect("one primary player, whose bundle holds the primary body");
        app.world_mut()
            .entity_mut(primary)
            .insert(InRoomInstance(second_room));

        let (player_label, undrawn, hud) = shown(&mut app);
        assert!(
            player_label,
            "two live rooms: the overlay drew no player label"
        );
        assert_eq!(
            undrawn, 0,
            "two live rooms: the renderer did not draw the labels"
        );
        assert!(
            hud.starts_with("P1 inspector: the second live room  ")
                && hud.contains(&format!(" room {}/", other.index() + 1)),
            "two live rooms: the HUD names the primary body's room: {hud:?}"
        );
        app.world_mut()
            .resource_mut::<DeveloperRuntimeState>()
            .debug = false;
        app.update();
        assert!(
            app.world_mut()
                .query_filtered::<Entity, Or<(
                    With<super::DebugOverlayDrawing>,
                    With<super::DebugLabelDrawing>
                )>>()
                .iter(app.world())
                .next()
                .is_none(),
            "turning F1 off must remove shapes and labels"
        );
    }
}
