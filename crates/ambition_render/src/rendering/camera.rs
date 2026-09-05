//! Presentation half of the follow camera.
//!
//! The RESOLVE — zoom policy, camera zones, target easing, blink
//! interpolation, clamping (the `CameraEaseState` write) — belongs to the
//! observation seam
//! ([`ambition_sim_view::camera_snapshot::CameraObservationPlugin`], E4-17),
//! which publishes one [`ResolvedCameraSnapshot`] per rendered FRAME. This
//! module only (a) applies presentation-only deltas — portal camera
//! continuity, shake — to a COPY of the snapshot, and (b) writes the Bevy
//! camera transform/projection. Render never mutates sim camera state.
//!
//! Frame, not tick: the simulation produces authoritative world facts, and where the camera
//! looks at them is presentation.
//!
//! The observer facts the resolver consumes (`CameraViewport`,
//! `CameraScreenFraming`) are published by
//! `ambition_platformer2d_host::gameplay_presentation`: they are answers about the physical
//! display and the active presentation profile, and render does not select
//! policy.

#[cfg(feature = "portal_render")]
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::primitives::PlayerVisual;
use ambition_sim_view::camera_snapshot::{CameraPresentationInputs, ResolvedCameraSnapshot};
// Only the portal publisher mints a chart transit; without that feature the
// import is dead and `-D warnings` compositions say so.
#[cfg(feature = "portal_render")]
use ambition_sim_view::camera_snapshot::CameraChartTransit;
use ambition_sim_view::LocalView;

// It is re-exported from `ambition_sim_view` so the name still resolves here; see that module
// for why a process-global could not answer "whose view" once there are two.
pub use ambition_sim_view::CameraViewState;

#[cfg(feature = "portal_render")]
#[derive(SystemParam)]
pub struct PortalCameraContinuityParams<'w> {
    selection: Option<Res<'w, ambition_portal2d_presentation::PortalCameraContinuitySelection>>,
    state: Option<ResMut<'w, ambition_portal2d_presentation::PortalCameraContinuityState>>,
    host_view: Option<ResMut<'w, ambition_portal2d_presentation::PortalCameraContinuityHostView>>,
}

/// Bridge the portal-continuity facts the RESOLVER needs — the clamp pad and
/// the chart rotation — into its generic inputs BEFORE this tick's resolve.
///
/// Same-frame, like the old inline read: a post-resolve copy would lag the pad
/// one frame and visibly step the camera at transit clear.
///
/// A rotation-aware clamp is only expressible once the roll is an input, and the composition rule
/// ([`presented_roll_radians`]) can only be applied where the base observer roll is also known.
#[cfg(feature = "portal_render")]
pub fn publish_portal_camera_clamp(
    selection: Option<Res<ambition_portal2d_presentation::PortalCameraContinuitySelection>>,
    state: Option<Res<ambition_portal2d_presentation::PortalCameraContinuityState>>,
    // One row per local view. Ambition has one; the portal's facts are a fact
    // about the world's geometry, so every view presenting that world is told.
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
                // Rising edge: THIS view's last resolved roll is its base roll.
                // ⚠ An UNFRAMED view has no roll to adopt — zero is the honest
                // answer for a view nothing has framed yet, and it is stated
                // rather than reached through a `Default` frame.
                None => resolved
                    .frame()
                    .map_or(0.0, |frame| frame.snapshot.rotation_radians),
            },
        });
    }
}

/// Apply the sim-resolved camera snapshot to EACH main camera — every one of
/// them through the view it names — layering the presentation-only deltas
/// (portal camera continuity, shake) onto a COPY.
pub fn camera_follow(
    // THE VIEWS, read through each camera's own link. This was a
    // `Single<…, With<LocalView>>` beside a query for the main camera — two
    // uniqueness assumptions pretending to be a pairing, which held only because
    // there happened to be one of each. A camera now NAMES the view it presents
    // (`PresentsView`), so a second view does not turn this into a panic and a
    // second camera does not turn it into a fight over one snapshot.
    mut views: Query<
        (
            Entity,
            &ResolvedCameraSnapshot,
            &mut CameraPresentationInputs,
            // The resolve below already knows which view this camera presents; that is exactly
            // the entity whose diagnostics these are.
            &mut CameraViewState,
        ),
        With<LocalView>,
    >,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    shake: Res<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeState>,
    // ⭐ OPTIONAL, and deliberately not `Res` like the shake beside it. A host
    // that draws through `camera_follow` without installing the finishing
    // zoom's resources should present an unzoomed camera, not panic on a
    // missing resource — the same reasoning `shake_camera_on_landed_hits`
    // states for its own feel tuning: *"a missing one means no shake rather
    // than a panic"*. Making it required turned an existing render fixture
    // red the moment it landed, which is the cheap version of the same
    // report a downstream host would have made.
    finish_zoom: Option<Res<ambition_platformer2d_shared_tangle::camera_ease::FinishZoomState>>,
    finish_zoom_tuning: Option<Res<ambition_platformer2d_shared_tangle::camera_ease::FinishZoomTuning>>,
    #[cfg(feature = "portal_render")] mut portal_continuity: PortalCameraContinuityParams,
    // `With<MainCamera>` (not the broad `With<Camera2d>`): besides the #31 cube pause-menu
    // Camera3d, the portal view-cone renderer spawns offscreen capture `Camera2d`s.
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
    // Same singleton the component move deleted, restored as a loop-invariant.
    // `PresentedViewState::get()` already refused to choose between several main cameras; this had
    // no such protection, it just picked.
    //
    // the binding rule itself lives in `ambition_sim_view::ViewsOnHand` — one
    // statement, shared with the viewport applier and the draw-side lookup,
    // because three copies of "which view is this camera for" is three chances
    // to disagree silently.
    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter().map(|(view, ..)| view));
    let shake_offset = shake.offset();
    // Resolved once per run rather than per camera: both resources are
    // process-wide, and an absent pair is exactly the identity.
    let finish_zoom_factor = match (finish_zoom.as_deref(), finish_zoom_tuning.as_deref()) {
        (Some(zoom), Some(tuning)) => zoom.scale_factor(*tuning),
        _ => 1.0,
    };

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
        // ⛔ A VIEW THAT HAS NOT BEEN FRAMED IS NOT PRESENTED. Before the
        // `Option` (2026-09-04) this read a `Default` frame — a real-looking
        // 568x320 window on the world origin — and moved the camera there.
        let Some(frame) = resolved.frame() else {
            continue;
        };
        // Presentation deltas apply to a COPY — the sim's resolved snapshot is
        // read-only here.
        #[cfg_attr(not(feature = "portal_render"), allow(unused_mut))]
        let mut snapshot = frame.snapshot.clone();
        #[cfg(feature = "portal_render")]
        let follow_world = frame.follow_world;

        #[cfg(not(feature = "portal_render"))]
        {
            // Without portal continuity nothing writes these; keep them cleared
            // so a stale pad or roll can't linger across feature configs.
            *presentation = CameraPresentationInputs::default();
        }
        #[cfg(feature = "portal_render")]
        let _ = &mut presentation; // written pre-resolve by publish_portal_camera_clamp

        // portal camera continuity is still ONE global for the whole process.
        // `PortalCameraContinuityState`/`HostView` are `Resource`s, so the writes below are
        // last-camera-wins once a composition really has two.
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
                    // `publish_portal_camera_clamp` now hands both facts to the resolve, and
                    // `snapshot.rotation_radians` already carries the composed answer by the
                    // time this runs.
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
            // ⛔⛔ THE FINISHING ZOOM MULTIPLIES HERE AND NOWHERE UPSTREAM.
            // `snapshot.orthographic_scale` descends from a policy that floors
            // itself at 1.0 twice over — `CameraZoneSpec::effective_zoom` and
            // `camera_snapshot`'s own `target_scale` — because the design view
            // is a readability FLOOR the player is never given less than. A
            // finishing zoom goes the other way, so it is applied to the
            // PRESENTED projection instead of being allowed under that floor,
            // exactly as the shake is applied to the presented transform.
            // `scale_factor` returns 1.0 when idle, so this is a no-op for
            // every host that never decides a match.
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

    /// 800x600, so the world-to-Bevy flip below is arithmetic anyone can check
    /// by hand rather than a number copied out of a previous run.
    fn room() -> ae::RoomGeometry {
        ae::RoomGeometry(ae::World::new(
            "two views",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(50.0, 50.0),
            Vec::new(),
        ))
    }

    /// What `camera_follow` must put on the Bevy transform for a view centred
    /// here: the same flip the production line does, written once so the
    /// expectation is derived rather than pinned.
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
    /// is the only thing that differs between runs: it SWAPS which view each
    /// camera names while leaving spawn order, entity ids and every snapshot
    /// value untouched.
    ///
    /// Returns, per camera in spawn order, the transform translation and
    /// orthographic scale `camera_follow` gave it, plus what each VIEW's own
    /// `CameraViewState` ended up holding.
    fn present(first_presents_left: bool) -> ([(Vec2, f32); 2], [ae::Vec2; 2]) {
        let mut world = World::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            &mut world,
            room(),
        );
        world.init_resource::<CameraShakeState>();

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

    /// EACH MAIN CAMERA PRESENTS THE VIEW IT NAMES — NOT THE FIRST
    /// CAMERA'S VIEW.
    ///
    /// `camera_follow` read `query.iter.next`'s `PresentsView`, resolved that ONE view, and
    /// then wrote that one view's transform and projection onto EVERY main camera.
    ///
    /// the assertion is on the VALUES, not on inequality. "the two
    /// cameras differ" would pass for a pair that differ and are both wrong.
    /// Each camera is checked against the framing derived from the view it
    /// names.
    ///
    /// and the falsifier is inside the test. The second run swaps only
    /// the two `PresentsView` links — same spawn order, same entities, same
    /// snapshots — and the two cameras must swap with them. A `camera_follow`
    /// that keys off camera iteration order instead of the link passes the
    /// first run and fails this one.
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
