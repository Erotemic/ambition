//! Presentation half of the follow camera.
//!
//! The resolve (zoom policy, camera zones, target easing, blink
//! interpolation, clamping; the `CameraEaseState` write) belongs to the
//! observation seam ([`ambition_sim_view::camera_snapshot::CameraObservationPlugin`]),
//! which publishes one [`ResolvedCameraSnapshot`] per rendered frame. This
//! module only (a) applies presentation-only deltas (portal camera
//! continuity, shake) to a copy of the snapshot, and (b) writes the Bevy
//! camera transform and projection. Render never mutates sim camera state.
//!
//! Per frame, not per tick: the simulation produces world facts, and where
//! the camera looks is presentation.
//!
//! The observer facts the resolver reads (`CameraViewport`,
//! `CameraScreenFraming`) are published by
//! `ambition_platformer2d_host::gameplay_presentation`: they describe the
//! physical display and the active presentation profile. Render does not
//! select policy.

#[cfg(feature = "portal_render")]
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::primitives::PlayerVisual;
use ambition_sim_view::camera_snapshot::{CameraPresentationInputs, ResolvedCameraSnapshot};
// Only the portal publisher creates a chart transit; without that feature
// the import is unused and `-D warnings` fails.
#[cfg(feature = "portal_render")]
use ambition_sim_view::camera_snapshot::CameraChartTransit;
use ambition_sim_view::LocalView;

// Re-exported from `ambition_sim_view`; see that module for why it is
// per-view state.
pub use ambition_sim_view::CameraViewState;

#[cfg(feature = "portal_render")]
#[derive(SystemParam)]
pub struct PortalCameraContinuityParams<'w> {
    selection: Option<Res<'w, ambition_portal2d_presentation::PortalCameraContinuitySelection>>,
    state: Option<ResMut<'w, ambition_portal2d_presentation::PortalCameraContinuityState>>,
    host_view: Option<ResMut<'w, ambition_portal2d_presentation::PortalCameraContinuityHostView>>,
}

/// Pass the portal-continuity facts the resolver needs (the clamp pad and
/// the chart rotation) into its generic inputs before this tick's resolve.
///
/// Same-frame: a post-resolve copy would lag the pad one frame and step the
/// camera when the transit clears. A rotation-aware clamp needs the roll as
/// an input, and the composition rule ([`presented_roll_radians`]) needs the
/// base observer roll, which is known here.
#[cfg(feature = "portal_render")]
pub fn publish_portal_camera_clamp(
    selection: Option<Res<ambition_portal2d_presentation::PortalCameraContinuitySelection>>,
    state: Option<Res<ambition_portal2d_presentation::PortalCameraContinuityState>>,
    // One row per local view. The portal facts describe the world, so every
    // view of that world gets them.
    mut views: Query<
        (
            Entity,
            &ResolvedCameraSnapshot,
            &mut CameraPresentationInputs,
        ),
        With<LocalView>,
    >,
) {
    let enabled = selection.as_deref().is_some_and(|selection| {
        selection.mode == ambition_portal2d_presentation::PortalCameraTransitMode::Continuous
    });
    let clamp_center = enabled
        .then(|| state.as_deref().and_then(|s| s.clamp_padding_center_world))
        .flatten();
    let crossing = enabled
        .then(|| {
            state
                .as_deref()
                .filter(|s| s.active_weight() > 0.0)
                .map(|s| s.roll_radians)
        })
        .flatten();
    for (_, resolved, mut presentation) in &mut views {
        presentation.extra_clamp_center_world = clamp_center;
        presentation.chart_transit = crossing.map(|chart_roll_radians| CameraChartTransit {
            chart_roll_radians,
            observer_roll_at_entry: match presentation.chart_transit {
                // Already crossing: keep the roll adopted when it began.
                Some(active) => active.observer_roll_at_entry,
                // Rising edge: this view's last resolved roll is its base roll.
                // An unframed view has no roll, so zero is used explicitly
                // (not through a `Default` frame).
                None => resolved
                    .frame()
                    .map_or(0.0, |frame| frame.snapshot.rotation_radians),
            },
        });
    }
}

/// Apply the sim-resolved camera snapshot to each main camera, through the
/// view it names, with the presentation-only deltas (portal camera
/// continuity, shake) applied to a copy.
pub fn camera_follow(
    // The views, read through each camera's own link. A camera names the
    // view it presents (`PresentsView`), so a second view or a second
    // camera is not a special case.
    mut views: Query<
        (
            Entity,
            &ResolvedCameraSnapshot,
            &mut CameraPresentationInputs,
            // The view this camera presents; its diagnostics go here.
            &mut CameraViewState,
        ),
        With<LocalView>,
    >,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    shake: Res<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeState>,
    // Required, not `Option<Res<..>>`. An `Option` would keep test fixtures
    // passing and silently disable the finish zoom in production if the
    // registration were dropped. `shake` above is already a required `Res`,
    // so a host missing this would already fail on `CameraShakeState`.
    finish_zoom: Res<ambition_platformer2d_shared_tangle::camera_ease::FinishZoomState>,
    finish_zoom_tuning: Res<ambition_platformer2d_shared_tangle::camera_ease::FinishZoomTuning>,
    #[cfg(feature = "portal_render")] mut portal_continuity: PortalCameraContinuityParams,
    // `With<MainCamera>`, not `With<Camera2d>`: the #31 cube pause menu has a
    // `Camera3d`, and the portal view-cone renderer spawns offscreen capture
    // `Camera2d`s.
    mut query: Query<
        (
            &mut Transform,
            &mut Projection,
            Option<&ambition_sim_view::PresentsView>,
        ),
        (
            With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>,
            Without<PlayerVisual>,
        ),
    >,
) {
    // The binding rule is in `ambition_sim_view::ViewsOnHand`, shared with
    // the viewport applier and the draw-side lookup, so all three agree on
    // which view a camera is for.
    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter().map(|(view, ..)| view));
    let shake_offset = shake.offset();
    // Resolved once per run: both are process-wide, and an idle zoom is 1.0,
    // so this is a no-op for hosts that never decide a match.
    let finish_zoom_factor = finish_zoom.scale_factor(*finish_zoom_tuning);

    for (mut transform, mut projection, link) in &mut query {
        let Some(view_entity) = on_hand.presented_by(link.copied()) else {
            continue;
        };
        let Ok((_, resolved, mut presentation, mut view_state)) = views.get_mut(view_entity) else {
            bevy::log::error_once!(
                "a camera presents view {view_entity:?}, which is not a local view"
            );
            continue;
        };
        // An unframed view is not presented; do not move the camera to a
        // default frame.
        let Some(frame) = resolved.frame() else {
            continue;
        };
        // Presentation deltas apply to a copy; the resolved snapshot is
        // read-only here.
        #[cfg_attr(not(feature = "portal_render"), allow(unused_mut))]
        let mut snapshot = frame.snapshot.clone();
        #[cfg(feature = "portal_render")]
        let follow_world = frame.follow_world;

        #[cfg(not(feature = "portal_render"))]
        {
            // Without portal continuity nothing writes these; keep them clear so a
            // stale pad or roll cannot remain across feature configs.
            *presentation = CameraPresentationInputs::default();
        }
        #[cfg(feature = "portal_render")]
        let _ = &mut presentation; // written pre-resolve by publish_portal_camera_clamp

        // Portal camera continuity is still one global for the process.
        // `PortalCameraContinuityState` and `HostView` are `Resource`s, so with
        // two cameras the last writer wins.
        #[cfg(feature = "portal_render")]
        {
            let portal_continuity_enabled =
                portal_continuity
                    .selection
                    .as_deref()
                    .is_some_and(|selection| {
                        selection.mode
                            == ambition_portal2d_presentation::PortalCameraTransitMode::Continuous
                    });
            let ordinary_center_world = snapshot.center_world;
            let portal_clamp_padding_still_needed =
                (ordinary_center_world - snapshot.unpadded_center_world).length() > 0.5;

            if let Some(portal_state) = portal_continuity.state.as_deref_mut() {
                if portal_continuity_enabled {
                    let weight = portal_state.active_weight();
                    if weight > 0.0 {
                        let screen_offset =
                            portal_state.body_screen_offset_world.unwrap_or(Vec2::ZERO);
                        snapshot.center_world = follow_world - screen_offset;
                        portal_state.target_camera_world = Some(snapshot.center_world);
                    } else if !portal_clamp_padding_still_needed {
                        portal_state.clear_clamp_padding();
                    }
                    // `publish_portal_camera_clamp` gives both facts to the resolve,
                    // so `snapshot.rotation_radians` already has the composed
                    // answer.
                } else {
                    portal_state.clear();
                }
            }
            if let Some(host_view) = portal_continuity.host_view.as_deref_mut() {
                host_view.capture(
                    snapshot.center_world,
                    ordinary_center_world,
                    snapshot.target_world,
                    snapshot.visible_view,
                    snapshot.active_camera_zones,
                    snapshot.active_camera_zone.clone(),
                );
            }
            if let Some(portal_state) = portal_continuity.state.as_deref_mut() {
                portal_state.last_host_camera_world = Some(snapshot.center_world);
            }
        }

        let x = snapshot.center_world.x - world.0.size.x * 0.5;
        let y = world.0.size.y * 0.5 - snapshot.center_world.y;

        *view_state = CameraViewState::from(&snapshot);

        if let Projection::Orthographic(orthographic) = &mut *projection {
            // The finishing zoom is applied here only. `snapshot.orthographic_scale`
            // comes from a policy that floors at 1.0 (`CameraZoneSpec::effective_zoom`
            // and `camera_snapshot`'s `target_scale`), because the design view is a
            // readability floor. A finishing zoom goes below it, so it is applied to
            // the presented projection, like shake to the presented transform.
            // `scale_factor` is 1.0 when idle.
            orthographic.scale = snapshot.orthographic_scale * finish_zoom_factor;
        }
        transform.translation.x = x + shake_offset.x;
        transform.translation.y = y + shake_offset.y;
        transform.rotation = Quat::from_rotation_z(snapshot.rotation_radians);
    }
}

#[cfg(test)]
mod two_views_one_simulation_tests {
    use super::*;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_shared_tangle::camera_ease::CameraShakeState;
    use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
    use ambition_sim_view::camera_snapshot::CameraSnapshot2d;
    use ambition_sim_view::{LocalView, LocalViewId, PresentsView};
    use bevy::ecs::system::RunSystemOnce as _;

    /// 800x600, so the world-to-Bevy flip below is easy to check by hand.
    fn room() -> ae::RoomGeometry {
        ae::RoomGeometry(ae::World::new(
            "two views",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(50.0, 50.0),
            Vec::new(),
        ))
    }

    /// What `camera_follow` must write on the Bevy transform for a view centred
    /// here: the same flip as production, so the expectation is derived.
    fn expected_translation(center: ae::Vec2) -> Vec2 {
        Vec2::new(center.x - 800.0 * 0.5, 600.0 * 0.5 - center.y)
    }

    fn spawn_view(world: &mut World, id: u8, center: ae::Vec2, ortho: f32) -> Entity {
        world
            .spawn((
                LocalView,
                LocalViewId(id),
                ResolvedCameraSnapshot(Some(
                    ambition_sim_view::camera_snapshot::ResolvedCameraFrame {
                        snapshot: CameraSnapshot2d {
                            center_world: center,
                            unpadded_center_world: center,
                            orthographic_scale: ortho,
                            ..Default::default()
                        },
                        follow_world: center,
                    },
                )),
                CameraPresentationInputs::default(),
                CameraViewState::default(),
            ))
            .id()
    }

    /// One world, one simulation, two views, two cameras. `first_presents_left`
    /// is the only difference between runs: it swaps which view each camera
    /// names, and keeps spawn order, entity ids, and every snapshot value.
    ///
    /// Returns, per camera in spawn order, the transform translation and
    /// orthographic scale `camera_follow` gave it, plus what each view's own
    /// `CameraViewState` ended up holding.
    fn present(first_presents_left: bool) -> ([(Vec2, f32); 2], [ae::Vec2; 2]) {
        let mut world = World::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            &mut world,
            room(),
        );
        world.init_resource::<CameraShakeState>();
        world.init_resource::<ambition_platformer2d_shared_tangle::camera_ease::FinishZoomState>();
        world.init_resource::<ambition_platformer2d_shared_tangle::camera_ease::FinishZoomTuning>();

        let left = spawn_view(&mut world, 0, ae::Vec2::new(100.0, 200.0), 2.0);
        let right = spawn_view(&mut world, 1, ae::Vec2::new(700.0, 500.0), 0.5);

        let (first, second) = if first_presents_left {
            (left, right)
        } else {
            (right, left)
        };
        let cameras: Vec<Entity> = [first, second]
            .into_iter()
            .map(|view| {
                world
                    .spawn((
                        MainCamera,
                        Transform::default(),
                        Projection::Orthographic(OrthographicProjection::default_2d()),
                        PresentsView(view),
                    ))
                    .id()
            })
            .collect();

        world
            .run_system_once(camera_follow)
            .expect("camera_follow should run: the fixture provides the session world it reads");

        let mut presented = [(Vec2::ZERO, 0.0); 2];
        for (slot, camera) in cameras.into_iter().enumerate() {
            let entity = world.entity(camera);
            let translation = entity
                .get::<Transform>()
                .expect("camera transform")
                .translation;
            let scale = match entity.get::<Projection>().expect("camera projection") {
                Projection::Orthographic(orthographic) => orthographic.scale,
                other => panic!("the fixture spawned an orthographic camera, found {other:?}"),
            };
            presented[slot] = (translation.truncate(), scale);
        }
        let view_states = [left, right].map(|view| {
            world
                .entity(view)
                .get::<CameraViewState>()
                .expect("the view carries its camera state")
                .center_world
        });
        (presented, view_states)
    }

    /// Each main camera presents the view it names, not the first camera's view.
    ///
    /// The test checks values, not only that the cameras differ: each camera is
    /// compared with the framing of the view it names.
    ///
    /// The second run swaps only the two `PresentsView` links, and the cameras
    /// must swap too. A version keyed on camera iteration order fails that run.
    #[test]
    fn each_main_camera_presents_the_view_it_names() {
        let left_expected = (expected_translation(ae::Vec2::new(100.0, 200.0)), 2.0);
        let right_expected = (expected_translation(ae::Vec2::new(700.0, 500.0)), 0.5);
        assert_ne!(
            left_expected, right_expected,
            "the fixture must give the two views genuinely different framings, or \
             nothing below can tell a per-camera resolve from a shared one"
        );

        let (presented, view_states) = present(true);
        assert_eq!(
            presented,
            [left_expected, right_expected],
            "each camera must take the framing of the view its `PresentsView` names; \
             getting {presented:?} means one view's snapshot was applied to both \
             cameras (or the wrong one was)"
        );
        assert_eq!(
            view_states,
            [ae::Vec2::new(100.0, 200.0), ae::Vec2::new(700.0, 500.0)],
            "each view's own `CameraViewState` must be written from ITS OWN \
             snapshot; a shared resolve writes one view's framing into whichever \
             view it resolved and leaves the other at `Default`"
        );

        let (swapped, _) = present(false);
        assert_eq!(
            swapped,
            [right_expected, left_expected],
            "swapping only the two links must swap the two cameras. It did not, so \
             the framing is following camera iteration order and the assertion above \
             was passing for the wrong reason"
        );
    }
}
