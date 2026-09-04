//! Pure 2D camera-follow snapshot policy.
//!
//! This module is the non-rendering half of the camera system: given a room,
//! a focus point/body, and camera policy inputs, it resolves the camera that
//! should view that focus. The visible Bevy camera, future portal captures, and
//! no-GPU/headless PNG tools can all consume the same [`CameraSnapshot2d`]
//! without depending on each other.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use bevy_math::UVec2;

use ambition_persistence::settings::video::CameraFramingPreset;
use ambition_persistence::settings::CameraAspectPolicy;
use ambition_platformer2d_world::rooms::{apply_forward_only_x, CameraClampMode, CameraScrollPolicy, CameraZoneSpec};
use ambition_platformer2d_shared_tangle::camera_ease::{CameraEaseState, CameraEaseTuning};
use ambition_platformer2d_shared_tangle::gameplay_presentation::NormalizedScreenRegion;

/// Upper bound on `dt` for camera scale + target easing.
///
/// Smoothing is dt-correct in steady state, but a single render hitch is still
/// perceived as a large per-frame camera jump. Capping policy resolution to a
/// 30 FPS step keeps a one-frame hitch from visually overshooting.
pub const MAX_CAMERA_SMOOTH_DT: f32 = 1.0 / 30.0;

/// Concrete, renderer-agnostic 2D camera snapshot.
///
/// The normal Bevy camera path writes this data every frame; headless renderers
/// and future capture requests can ask for the same data for an arbitrary focus
/// point. Ambition world coordinates are used throughout: +Y points downward.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraSnapshot2d {
    /// Authored/default design view before encounter/camera-zone zoom.
    pub base_view: ae::Vec2,
    /// Requested gameplay view after zoom policy, before physical window aspect
    /// expansion.
    pub requested_view: ae::Vec2,
    /// Actual visible world-space rectangle after applying window aspect policy.
    pub visible_view: ae::Vec2,
    /// Live zoom multiplier applied to [`Self::base_view`].
    pub zoom_multiplier: f32,
    /// Bevy orthographic scale required to show [`Self::visible_view`] in the
    /// current physical viewport.
    pub orthographic_scale: f32,
    /// World-space focus/target after look-ahead, camera-zone offsets, blink
    /// interpolation, and optional target easing.
    pub target_world: ae::Vec2,
    /// Final world-space camera center before presentation-only shake.
    pub center_world: ae::Vec2,
    /// Camera center without optional clamp padding. Equal to
    /// [`Self::center_world`] for ordinary/headless captures.
    pub unpadded_center_world: ae::Vec2,
    /// Camera roll in radians. Ordinary 2D follow is zero; portal/capture
    /// adapters can apply a non-zero value after resolving the snapshot.
    pub rotation_radians: f32,
    /// Number of camera zones the focus overlaps this frame.
    pub active_camera_zones: usize,
    /// Highest-priority active camera-zone id, when any zone applies.
    pub active_camera_zone: Option<String>,
}

/// Which frame a view presents the world in, shared with input-frame policy.
/// [`ae::InputFrameMode::under_camera`] defines the corresponding input semantics.
pub use ae::CameraReferenceFrame;

/// Base camera roll for a view frame before any chart transit.
///
/// In subject-relative mode, render space flips world Y, so a normalized world-down `(dx, dy)`
/// maps to the camera angle `atan2(dx, dy)`. [`presented_roll_radians`] composes transit rotation.
pub fn observer_roll_radians(frame: CameraReferenceFrame, subject_down: Option<ae::Vec2>) -> f32 {
    match frame {
        CameraReferenceFrame::WorldFixed => 0.0,
        // No subject to orient on is not an error — a view may be framing a cast or nothing at
        // all.
        CameraReferenceFrame::SubjectFrame => match subject_down {
            Some(down) if down.length_squared() > f32::EPSILON => {
                let down = down.normalize();
                down.x.atan2(down.y)
            }
            _ => 0.0,
        },
    }
}

/// A chart rotation the view is presenting through, and the roll it had
/// adopted when that began.
///
/// A portal maps one part of the world onto another through a rotation; while a
/// subject is crossing, the view must present the DESTINATION chart or the image
/// tears at the seam. `chart_roll_radians` is that map's render-space rotation
/// (`portal_transit_roll`), which is a property of the portal PAIR — not of
/// gravity, not of the observer, and not of the body.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraChartTransit {
    /// The map's render-space rotation while the crossing is active.
    pub chart_roll_radians: f32,
    /// The roll the view had already adopted when the crossing began, which
    /// is what stops the composition double-counting. See
    /// [`presented_roll_radians`].
    pub observer_roll_at_entry: f32,
}

/// Roll actually presented by the view.
///
/// During a chart transit, compose the map rotation with the observer roll captured at entry rather
/// than the live base roll. This avoids double-counting a rotation already absorbed by a
/// [`CameraReferenceFrame::SubjectFrame`] subject.
pub fn presented_roll_radians(
    frame: CameraReferenceFrame,
    subject_down: Option<ae::Vec2>,
    transit: Option<CameraChartTransit>,
) -> f32 {
    match transit {
        None => observer_roll_radians(frame, subject_down),
        Some(transit) => transit.observer_roll_at_entry + transit.chart_roll_radians,
    }
}

/// Concrete scene-capture request: camera policy produces the snapshot, and
/// render backends consume this data to fill a target.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneCaptureRequest {
    pub snapshot: CameraSnapshot2d,
    pub target_size_px: UVec2,
    pub include_world: bool,
    pub include_backgrounds_or_parallax: bool,
    pub include_actors: bool,
    pub include_ui: bool,
    pub capture_depth: u32,
    pub debug_name: Option<String>,
}

impl SceneCaptureRequest {
    pub fn new(snapshot: CameraSnapshot2d, target_size_px: UVec2) -> Self {
        Self {
            snapshot,
            target_size_px,
            include_world: true,
            include_backgrounds_or_parallax: true,
            include_actors: true,
            include_ui: false,
            capture_depth: 0,
            debug_name: None,
        }
    }
}

/// The framing a snapshot carries before anything has resolved one, taken from
/// the shipped default preset so there is exactly one place that decides it.
fn default_base_view() -> ae::Vec2 {
    let (w, h) = ambition_persistence::settings::video::CameraZoomPreset::default().base_view();
    ae::Vec2::new(w, h)
}

impl Default for CameraSnapshot2d {
    fn default() -> Self {
        Self {
            // ⛔ DERIVED, NOT RESTATED. This used to hardcode 800x450 — the
            // `Combat` preset's view, and the default framing at the time. When
            // the default moved to `Duel` (568x320) on 2026-09-03 this became a
            // SECOND, disagreeing statement of "the default framing", which is
            // the one-authority failure in miniature: nothing reads a default
            // snapshot in production today, so the disagreement would have sat
            // here silently until something did.
            base_view: default_base_view(),
            requested_view: default_base_view(),
            visible_view: default_base_view(),
            zoom_multiplier: 1.0,
            orthographic_scale: 1.0,
            target_world: ae::Vec2::ZERO,
            center_world: ae::Vec2::ZERO,
            unpadded_center_world: ae::Vec2::ZERO,
            rotation_radians: 0.0,
            active_camera_zones: 0,
            active_camera_zone: None,
        }
    }
}

/// The body/focus that a follow camera should frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraFocus2d {
    /// Current body/focus center in world coordinates.
    pub center_world: ae::Vec2,
    /// Current body/focus size in world units.
    pub size: ae::Vec2,
    pub base_size: ae::Vec2,
    /// Horizontal facing sign used by camera-framing presets.
    pub facing: f32,
    /// Current body velocity in world units per second. Soft framing folds
    /// this into the protected bounds as look-ahead; every other policy stage
    /// ignores it.
    pub velocity_world: ae::Vec2,
}

impl CameraFocus2d {
    pub fn aabb(self) -> ae::Aabb {
        ae::Aabb::new(self.center_world, self.size * 0.5)
    }

    pub fn stable_center(self) -> ae::Vec2 {
        let resize_offset = (self.base_size.y - self.size.y) * 0.5;
        ae::Vec2::new(self.center_world.x, self.center_world.y - resize_offset)
    }
}

/// Optional blink-arrival interpolation input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraBlinkInput {
    pub blink_in_timer: f32,
    pub blink_in_duration: f32,
    pub blink_camera_from: ae::Vec2,
}

/// Where the controlled subject should preferably appear on screen.
///
/// The presentation layer resolves this from the active gameplay-presentation
/// profile (gameplay viewport ∩ device safe area − control occupancy) and
/// publishes it as an OBSERVER FACT, exactly like [`CameraViewport`]. The
/// resolver consumes it and nothing else in the sim reads it: mobile
/// conditions never enter actor simulation or collision.
///
/// Inactive by default, which is ordinary centering.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub struct CameraScreenFraming {
    /// Whether soft framing applies at all.
    pub active: bool,
    /// The subject-safe region, normalized within the gameplay viewport with a
    /// top-left origin. Ambition world space is also +Y down, so this needs no
    /// axis flip.
    pub subject_safe_region: NormalizedScreenRegion,
    /// Extra padding around the subject's protected bounds, in gameplay
    /// viewport pixels.
    pub subject_padding_px: ae::Vec2,
    /// Seconds of subject velocity folded into the protected bounds.
    pub look_ahead_seconds: f32,
}

impl Default for CameraScreenFraming {
    fn default() -> Self {
        Self {
            active: false,
            subject_safe_region: NormalizedScreenRegion::FULL,
            subject_padding_px: ae::Vec2::ZERO,
            look_ahead_seconds: 0.0,
        }
    }
}

/// Whether policy resolution should mutate/reuse live presentation easing state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraSnapshotResolveMode {
    /// Stateless resolution for capture tools and deterministic screenshots.
    #[default]
    Instant,
    /// Live presentation resolution: use and update [`CameraEaseState`].
    Eased,
}

/// Pure input bundle for resolving a follow-camera snapshot.
pub struct CameraSnapshotResolveInput<'a> {
    pub world: &'a ae::World,
    pub camera_zones: &'a [CameraZoneSpec],
    pub focus: CameraFocus2d,
    pub base_view: ae::Vec2,
    pub viewport_px: ae::Vec2,
    pub aspect_policy: CameraAspectPolicy,
    pub framing: CameraFramingPreset,
    pub overview_scale: f32,
    pub encounter_scale: f32,
    pub overview_camera: bool,
    pub snap_camera: bool,
    pub blink: Option<CameraBlinkInput>,
    pub dt: f32,
    pub mode: CameraSnapshotResolveMode,
    /// Optional extra center that should remain inside the clamp bounds. Live
    /// presentation adapters can use this to temporarily widen room clamps;
    /// ordinary captures pass `None`.
    pub extra_clamp_center_world: Option<ae::Vec2>,
    pub ease_tuning: CameraEaseTuning,
    /// Optional screen-framing fact from the presentation layer. `None` (and
    /// an inactive value) means ordinary centering — captures, headless runs,
    /// and games that declare no framing policy all pass nothing.
    pub screen_framing: Option<CameraScreenFraming>,
    /// Which frame this view presents in.
    pub reference_frame: CameraReferenceFrame,
    /// The view subject's resolved down axis, read by `SubjectFrame`. `None`
    /// when the view has no subject to orient on.
    pub subject_down: Option<ae::Vec2>,
    /// Chart rotation currently presented by this view, if any.
    pub chart_transit: Option<CameraChartTransit>,
    /// World box the room clamp must keep visible, such as a platform-fighter cast extending into
    /// blast margins outside the authored room bounds. [`hold_camera_target`] applies the minimum
    /// correction needed after the ordinary clamp.
    ///
    /// This is distinct from [`Self::extra_clamp_center_world`]: that input is a portal-padding
    /// point and is intentionally excluded from `unpadded_center_world` diagnostics.
    pub must_frame_world: Option<ae::Aabb>,
}

/// Resolve a camera snapshot for an arbitrary focus.
///
/// In [`CameraSnapshotResolveMode::Instant`] this is deterministic and does not
/// require live state, which makes it suitable for headless PNG tools and future
/// capture requests. In [`CameraSnapshotResolveMode::Eased`] pass the live
/// [`CameraEaseState`] to preserve the visible game's smoothing behavior.
pub fn resolve_follow_camera_snapshot(
    input: CameraSnapshotResolveInput<'_>,
    mut ease_state: Option<&mut CameraEaseState>,
) -> CameraSnapshot2d {
    let focus_aabb = input.focus.aabb();
    let mut active_camera_zones = 0usize;
    let active_zone = input
        .camera_zones
        .iter()
        .filter(|zone| focus_aabb.strict_intersects(zone.aabb))
        .inspect(|_| active_camera_zones += 1)
        .max_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| zone_area(a).total_cmp(&zone_area(b)))
        });

    let camera_zone_scale = active_zone
        .map(CameraZoneSpec::effective_zoom)
        .unwrap_or(1.0);
    let target_scale = if input.overview_camera {
        input.overview_scale
    } else {
        input.encounter_scale.max(camera_zone_scale)
    }
    .max(1.0);
    let dt = input.dt.clamp(0.0, MAX_CAMERA_SMOOTH_DT);
    let camera_scale = match input.mode {
        CameraSnapshotResolveMode::Instant => target_scale,
        CameraSnapshotResolveMode::Eased => {
            if let Some(state) = ease_state.as_deref_mut() {
                if input.overview_camera || input.snap_camera {
                    state.live_scale = target_scale;
                    target_scale
                } else {
                    let rate = if target_scale > state.live_scale {
                        input.ease_tuning.zoom_out_rate
                    } else {
                        input.ease_tuning.zoom_in_rate
                    };
                    let delta = (target_scale - state.live_scale).abs();
                    let step = (rate * dt).min(delta);
                    state.live_scale = if target_scale > state.live_scale {
                        state.live_scale + step
                    } else {
                        state.live_scale - step
                    };
                    if (state.live_scale - target_scale).abs() < input.ease_tuning.snap_epsilon {
                        state.live_scale = target_scale;
                    }
                    state.live_scale.max(1.0)
                }
            } else {
                target_scale
            }
        }
    };

    let target_view_w = input.base_view.x * camera_scale;
    let target_view_h = input.base_view.y * camera_scale;
    let viewport_w = input.viewport_px.x.max(1.0);
    let viewport_h = input.viewport_px.y.max(1.0);
    let scale_by_height = target_view_h / viewport_h;
    let scale_by_width = target_view_w / viewport_w;
    let orthographic_scale = match input.aspect_policy {
        CameraAspectPolicy::FitDesign => scale_by_height.max(scale_by_width),
        CameraAspectPolicy::FixedHeight => scale_by_height,
        CameraAspectPolicy::FixedWidth => scale_by_width,
    };
    let half_view_w = viewport_w * orthographic_scale * 0.5;
    let half_view_h = viewport_h * orthographic_scale * 0.5;
    let visible_view = ae::Vec2::new(half_view_w * 2.0, half_view_h * 2.0);

    let desired_target_world = if input.overview_camera {
        input.focus.stable_center()
    } else {
        let mut desired = input.focus.stable_center();
        let (bias_x, bias_y) =
            input
                .framing
                .target_offset(target_view_w, target_view_h, input.focus.facing);
        desired.x += bias_x;
        desired.y += bias_y;

        if let Some(zone) = active_zone {
            if zone.cinematic_lock {
                desired = zone.aabb.center();
            }
            desired += zone.target_offset;
        }

        if let Some(blink) = input.blink {
            if blink.blink_in_timer > 0.0 && blink.blink_in_duration > 0.0 {
                let raw_t = 1.0 - (blink.blink_in_timer / blink.blink_in_duration).clamp(0.0, 1.0);
                let t = raw_t * raw_t * (3.0 - 2.0 * raw_t);
                desired = blink.blink_camera_from + (desired - blink.blink_camera_from) * t;
            }
        }
        desired
    };

    // Soft framing is a deadzone applied before easing and clamping. Cinematic locks, blink arrival,
    // and snap-camera requests bypass it so it cannot compete with explicit composition.
    let target_roll = presented_roll_radians(
        input.reference_frame,
        input.subject_down,
        input.chart_transit,
    );
    // Ease ordinary subject-frame roll changes in the resolver. Chart transits adopt immediately so
    // the portal seam stays aligned; stateless captures use the raw target roll.
    let rotation_radians = match ease_state.as_deref_mut() {
        Some(state) if input.chart_transit.is_none() => {
            let eased = match state.live_observer_roll {
                // First resolve for this view: ADOPT.
                None => target_roll,
                Some(current) => {
                    ambition_platformer2d_shared_tangle::camera_ease::ease_roll_radians(
                        current,
                        target_roll,
                        input.dt,
                    )
                }
            };
            state.live_observer_roll = Some(eased);
            eased
        }
        Some(state) => {
            state.live_observer_roll = Some(target_roll);
            target_roll
        }
        None => target_roll,
    };

    let soft_framing = input
        .screen_framing
        .filter(|framing| framing.active)
        .filter(|_| !input.overview_camera && !input.snap_camera)
        .filter(|_| !active_zone.is_some_and(|zone| zone.cinematic_lock))
        .filter(|_| {
            !input
                .blink
                .is_some_and(|blink| blink.blink_in_timer > 0.0 && blink.blink_in_duration > 0.0)
        });
    let desired_target_world = match soft_framing {
        None => desired_target_world,
        Some(framing) => {
            let previous = ease_state
                .as_deref()
                .filter(|state| state.target_initialized)
                .map(|state| state.live_target_world)
                .unwrap_or(desired_target_world);
            apply_soft_subject_framing(
                desired_target_world,
                previous,
                input.focus,
                visible_view,
                orthographic_scale,
                framing,
                // The roll this view is presenting, resolved above — the screen
                // region is a fraction of what the participant SEES.
                rotation_radians,
            )
        }
    };

    let target_world = match input.mode {
        CameraSnapshotResolveMode::Instant => desired_target_world,
        CameraSnapshotResolveMode::Eased => {
            if let Some(state) = ease_state.as_deref_mut() {
                if input.overview_camera || input.snap_camera || !state.target_initialized {
                    state.target_initialized = true;
                    state.live_target_world = desired_target_world;
                    desired_target_world
                } else {
                    let target_ease_hz = active_zone
                        .and_then(|zone| zone.easing_hz)
                        .unwrap_or(8.0)
                        .max(0.0);
                    let alpha = (1.0 - (-target_ease_hz * dt).exp()).clamp(0.0, 1.0);
                    let previous_target_world = state.live_target_world;
                    let eased_target_world = previous_target_world
                        + (desired_target_world - previous_target_world) * alpha;
                    state.live_target_world = eased_target_world;
                    eased_target_world
                }
            } else {
                desired_target_world
            }
        }
    };

    let bounds = active_zone.map(|zone| zone.clamp_mode).unwrap_or_default();
    let target = world_to_centered_render(input.world, target_world);
    // Clamp the axis-aligned footprint of the rolled view, not the upright viewport dimensions.
    let (clamp_half_w, clamp_half_h) =
        rolled_view_half_extents(half_view_w, half_view_h, rotation_radians);
    let (normal_host_x, normal_host_y) = clamp_camera_target(
        input.world,
        target,
        clamp_half_w,
        clamp_half_h,
        bounds,
        active_zone,
        None,
        //  the must-frame relaxation applies to the UNPADDED diagnostic too:
        // `unpadded_center_world` reports the camera without the portal
        // adapter's PADDING, not without the framing the game asked for.
        input.must_frame_world,
    );
    let (host_x, host_y) = if let Some(padding_center) = input.extra_clamp_center_world {
        clamp_camera_target(
            input.world,
            target,
            clamp_half_w,
            clamp_half_h,
            bounds,
            active_zone,
            Some(padding_center),
            input.must_frame_world,
        )
    } else {
        (normal_host_x, normal_host_y)
    };

    // `host_x` is centered-render x, which is monotone in world x.
    //
    // Leaving the zone clears the watermark: the clamp is per-visit, not per-room.
    let forward_only =
        active_zone.is_some_and(|zone| zone.scroll_policy == CameraScrollPolicy::ForwardOnlyX);
    // `normal_host_x` is the UNPADDED diagnostic center; it is deliberately left
    // un-watermarked so a trace can still see where the camera wanted to be.
    let host_x = match ease_state.as_deref_mut() {
        Some(state) if forward_only => apply_forward_only_x(host_x, &mut state.scroll_watermark_x),
        Some(state) => {
            state.scroll_watermark_x = None;
            host_x
        }
        None => host_x,
    };

    let center_world = ae::Vec2::new(
        host_x + input.world.size.x * 0.5,
        input.world.size.y * 0.5 - host_y,
    );
    let unpadded_center_world = ae::Vec2::new(
        normal_host_x + input.world.size.x * 0.5,
        input.world.size.y * 0.5 - normal_host_y,
    );

    CameraSnapshot2d {
        base_view: input.base_view,
        requested_view: ae::Vec2::new(target_view_w, target_view_h),
        visible_view,
        zoom_multiplier: camera_scale,
        orthographic_scale,
        target_world,
        center_world,
        unpadded_center_world,
        rotation_radians,
        active_camera_zones,
        active_camera_zone: active_zone.map(|zone| zone.id.clone()),
    }
}

/// Return the camera target that keeps the subject's protected bounds inside
/// the safe region, moving `previous` as little as possible.
///
/// With camera center `C`, a world point `P` projects to the normalized screen
/// position `n = 0.5 + (P - C) / visible_view`. Requiring the whole protected
/// box to satisfy `region.min <= n <= region.max` yields a closed interval of
/// admissible camera centers per axis; the correction is then a plain clamp.
///
/// While a view is upright the two frames coincide and nothing could tell the difference; under a
/// rolled view — a gravity flip in `SubjectFrame` mode, a portal seam — they diverge by the roll,
/// and the deadzone protected the wrong screen edge. Rolling a quarter turn swapped which axis the
/// region constrained entirely.
///
///  the transform is a plain rotation by +roll, and that is worth deriving
/// rather than guessing. World is y-DOWN, render is y-UP, and screen space is
/// y-down again. World→render flips y; the camera's roll takes view-space to
/// render-space as `R(θ)`, so render→view is `R(-θ)`; view→screen flips y back.
/// Composing, `flip ∘ R(-θ) ∘ flip = R(θ)` — conjugating a rotation by a
/// reflection negates its angle — so a world delta becomes a screen delta under
/// `R(+θ)` with no sign surprises left over.
///
///  `visible_view` is deliberately the UNROTATED extent and that is exactly
/// what this needs: it describes the view's own width and height, which is what
/// a normalized screen region is a fraction OF. The rotated footprint is a
/// different question, asked by the room clamp.
fn apply_soft_subject_framing(
    desired: ae::Vec2,
    previous: ae::Vec2,
    focus: CameraFocus2d,
    visible_view: ae::Vec2,
    orthographic_scale: f32,
    framing: CameraScreenFraming,
    roll_radians: f32,
) -> ae::Vec2 {
    let visible = visible_view.max(ae::Vec2::splat(f32::EPSILON));
    let anchor = focus.stable_center();

    // World delta -> screen delta, and back. At zero roll both are the identity,
    // so every upright view resolves exactly as it did.
    let (sin, cos) = roll_radians.sin_cos();
    let to_screen = |v: ae::Vec2| ae::Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
    let to_world = |v: ae::Vec2| ae::Vec2::new(v.x * cos + v.y * sin, -v.x * sin + v.y * cos);

    let bias = to_screen(desired - anchor);

    // Protected bounds: the standing body box (so a crouch does not shrink the
    // protection), padding in viewport pixels converted to world units, and the
    // look-ahead sweep.
    //
    //  the body box is an EXTENT, not a point: rotated, it occupies a larger
    // axis-aligned rectangle on screen, which is the same footprint question the
    // room clamp asks — so it goes through the same helper rather than being
    // rotated as if it were a position.
    let half_world = focus.size.max(focus.base_size) * 0.5
        + framing.subject_padding_px.abs() * orthographic_scale.max(0.0);
    let (half_x, half_y) = rolled_view_half_extents(half_world.x, half_world.y, roll_radians);
    let half = ae::Vec2::new(half_x, half_y);
    let lead = to_screen(focus.velocity_world * framing.look_ahead_seconds.max(0.0));
    let origin = ae::Vec2::ZERO;
    let swept_min = origin.min(origin + lead) - half;
    let swept_max = origin.max(origin + lead) + half;

    let region = framing.subject_safe_region;
    let low = swept_max + bias - visible * (region.max - ae::Vec2::splat(0.5));
    let high = swept_min + bias - visible * (region.min - ae::Vec2::splat(0.5));

    // Protected bounds wider than the region on an axis: no camera center can
    // satisfy it, so center the bounds in the region rather than snapping to an
    // arbitrary edge.
    let centered =
        (swept_min + swept_max) * 0.5 + bias - visible * (region.center() - ae::Vec2::splat(0.5));

    let previous_screen = to_screen(previous - anchor);
    let resolved = ae::Vec2::new(
        if low.x <= high.x {
            previous_screen.x.clamp(low.x, high.x)
        } else {
            centered.x
        },
        if low.y <= high.y {
            previous_screen.y.clamp(low.y, high.y)
        } else {
            centered.y
        },
    );
    anchor + to_world(resolved)
}

fn zone_area(zone: &CameraZoneSpec) -> f32 {
    let half = zone.aabb.half_size();
    (half.x * 2.0).max(0.0) * (half.y * 2.0).max(0.0)
}

fn world_to_centered_render(world: &ae::World, p: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new(p.x - world.size.x * 0.5, world.size.y * 0.5 - p.y)
}

/// Axis-aligned world-space half-extents of a rolled rectangular view.
///
/// The clamp must contain the rotated footprint. Absolute sine/cosine terms make the result
/// independent of rotation sign, and zero roll is the identity.
fn rolled_view_half_extents(half_view_w: f32, half_view_h: f32, roll_radians: f32) -> (f32, f32) {
    if roll_radians == 0.0 {
        return (half_view_w, half_view_h);
    }
    let (sin, cos) = roll_radians.sin_cos();
    let (sin, cos) = (sin.abs(), cos.abs());
    (
        half_view_w * cos + half_view_h * sin,
        half_view_w * sin + half_view_h * cos,
    )
}

#[allow(clippy::too_many_arguments)]
fn clamp_camera_target(
    world: &ae::World,
    target: ae::Vec2,
    half_view_w: f32,
    half_view_h: f32,
    mode: CameraClampMode,
    zone: Option<&CameraZoneSpec>,
    extra_clamp_center_world: Option<ae::Vec2>,
    must_frame_world: Option<ae::Aabb>,
) -> (f32, f32) {
    match mode {
        CameraClampMode::None => (target.x, target.y),
        CameraClampMode::ZoneBounds => {
            let Some(zone) = zone else {
                return clamp_to_world_bounds(
                    world,
                    target,
                    half_view_w,
                    half_view_h,
                    extra_clamp_center_world,
                    must_frame_world,
                );
            };
            let min_x = zone.aabb.left() + half_view_w - world.size.x * 0.5;
            let max_x = zone.aabb.right() - half_view_w - world.size.x * 0.5;
            let min_y = world.size.y * 0.5 - (zone.aabb.bottom() - half_view_h);
            let max_y = world.size.y * 0.5 - (zone.aabb.top() + half_view_h);
            let (min_x, max_x, min_y, max_y) = expand_clamp_bounds_for_padding(
                world,
                min_x,
                max_x,
                min_y,
                max_y,
                extra_clamp_center_world,
            );
            hold_camera_target(
                world,
                (
                    clamp_or_center(target.x, min_x, max_x),
                    clamp_or_center(target.y, min_y, max_y),
                ),
                half_view_w,
                half_view_h,
                must_frame_world,
            )
        }
        CameraClampMode::RoomBounds => clamp_to_world_bounds(
            world,
            target,
            half_view_w,
            half_view_h,
            extra_clamp_center_world,
            must_frame_world,
        ),
    }
}

fn clamp_to_world_bounds(
    world: &ae::World,
    target: ae::Vec2,
    half_view_w: f32,
    half_view_h: f32,
    extra_clamp_center_world: Option<ae::Vec2>,
    must_frame_world: Option<ae::Aabb>,
) -> (f32, f32) {
    let min_x = -world.size.x * 0.5 + half_view_w;
    let max_x = world.size.x * 0.5 - half_view_w;
    let min_y = -world.size.y * 0.5 + half_view_h;
    let max_y = world.size.y * 0.5 - half_view_h;
    let (min_x, max_x, min_y, max_y) = expand_clamp_bounds_for_padding(
        world,
        min_x,
        max_x,
        min_y,
        max_y,
        extra_clamp_center_world,
    );
    hold_camera_target(
        world,
        (
            clamp_or_center(target.x, min_x, max_x),
            clamp_or_center(target.y, min_y, max_y),
        ),
        half_view_w,
        half_view_h,
        must_frame_world,
    )
}

/// Pull a clamped camera centre back until it HOLDS `must_frame_world`, moving
/// it as little as possible. See
/// [`CameraSnapshotResolveInput::must_frame_world`].
///
/// The centres that hold the box on one axis are exactly `[box_max - half_view,
/// box_min + half_view]` — the closed interval on which the whole box projects
/// inside the view — so the correction is a plain clamp into it, exactly the
/// shape [`apply_soft_subject_framing`] uses for a screen region.
///
/// it runs AFTER the room clamp and wins, and that ordering is the whole point rather than an
/// ordering detail: this is the one caller that is asking to look outside the room on purpose.
/// While the box sits comfortably inside the view the interval is wide, the eased target is already
/// inside it, and this changes nothing — so the ordinary smoothing is untouched and only the frames
/// where the cast would otherwise leave the screen are corrected.
///
///  a box wider than the view centres on it instead, which is the honest
/// answer to an impossible request: nothing holds it, so show the middle. In
/// practice the framing floor has already widened the view past the cast, so
/// this arm is the degenerate case rather than the working one.
fn hold_camera_target(
    world: &ae::World,
    clamped: (f32, f32),
    half_view_w: f32,
    half_view_h: f32,
    must_frame_world: Option<ae::Aabb>,
) -> (f32, f32) {
    let Some(box_world) = must_frame_world else {
        return clamped;
    };
    // World→centered-render flips y, so the box's world TOP becomes its render
    // maximum. Convert both corners and re-derive the extents rather than
    // assuming which one is which.
    let a = world_to_centered_render(world, box_world.min);
    let b = world_to_centered_render(world, box_world.max);
    let render_min = ae::Vec2::new(a.x.min(b.x), a.y.min(b.y));
    let render_max = ae::Vec2::new(a.x.max(b.x), a.y.max(b.y));

    let axis = |value: f32, low: f32, high: f32, half: f32| -> f32 {
        let lo = high - half;
        let hi = low + half;
        if lo <= hi {
            value.clamp(lo, hi)
        } else {
            (low + high) * 0.5
        }
    };
    (
        axis(clamped.0, render_min.x, render_max.x, half_view_w),
        axis(clamped.1, render_min.y, render_max.y, half_view_h),
    )
}

fn expand_clamp_bounds_for_padding(
    world: &ae::World,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    extra_clamp_center_world: Option<ae::Vec2>,
) -> (f32, f32, f32, f32) {
    let Some(center_world) = extra_clamp_center_world else {
        return (min_x, max_x, min_y, max_y);
    };
    let x = center_world.x - world.size.x * 0.5;
    let y = world.size.y * 0.5 - center_world.y;
    (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
}

fn clamp_or_center(value: f32, min: f32, max: f32) -> f32 {
    if min <= max {
        value.clamp(min, max)
    } else {
        (min + max) * 0.5
    }
}

// Camera observation has one writer. Presentation may transform copies; simulation never consumes
// the resolved presentation snapshot.

/// Screen rectangle occupied by this view, published by the windowed host.
///
/// Size and origin are in logical display pixels. The physical-pixel conversion belongs only at
/// `apply_gameplay_camera_viewport`; headless views use the default design-window rectangle.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub struct CameraViewport {
    /// Size of this view's rectangle, logical pixels (world-frame-free — a
    /// screen fact). Consumed by the observation resolve below for orthographic
    /// scale, visible-world extent and clamp half-extents.
    pub px: ae::Vec2,
    /// Top-left of this view's rectangle within the display, in logical pixels. Camera scale does
    /// not depend on origin; presentation uses it to place the view.
    pub origin_px: ae::Vec2,
}

impl Default for CameraViewport {
    fn default() -> Self {
        Self {
            px: ae::Vec2::new(ae::config::WINDOW_W as f32, ae::config::WINDOW_H as f32),
            origin_px: ae::Vec2::ZERO,
        }
    }
}

/// Presentation-owned inputs applied before camera resolution.
///
/// Writers clear inactive values each frame. Rotation is an input so the resolver can clamp the
/// actual rolled footprint.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Default)]
pub struct CameraPresentationInputs {
    /// Optional extra center that should remain inside the clamp bounds.
    pub extra_clamp_center_world: Option<ae::Vec2>,
    /// The chart rotation this view is presenting through, if any.
    pub chart_transit: Option<CameraChartTransit>,
}

/// Follow-camera observation resolved once per rendered frame.
///
/// Presentation transforms copies of this state; simulation does not consume it.
#[derive(bevy::prelude::Component, Clone, Debug, Default)]
pub struct ResolvedCameraSnapshot {
    pub snapshot: CameraSnapshot2d,
    /// World-frame position of the followed body (the controlled subject)
    /// this tick — the un-eased follow point presentation adapters (portal
    /// continuity) key their offsets from.
    pub follow_world: ae::Vec2,
}

/// Resolve the follow camera for this frame; this is the sole writer of [`CameraEaseState`].
///
/// It runs after simulation and presentation-input adapters. Consumers of the published snapshot
/// order after [`CameraObservationSet`].
#[allow(clippy::too_many_arguments)]
/// Framing bounds and stable anchor for a declared cast.
///
/// Empty or unresolvable casts return `None`; callers must not invent a world-origin fallback.
struct CastFraming {
    /// Cast bounds drive target center, minimum view size, and must-frame clamp relaxation.
    bounds: ae::Aabb,
    /// How many bodies the cast resolved to. A DROP in this is ONE of the two
    /// discontinuities the close-rate cap exists for — see
    /// [`CAST_FRAMING_CLOSE_MAX_UNITS_PER_S`].
    members: usize,
    /// A cast member did not TRAVEL to where it now is — it was put there.
    ///
    /// The other discontinuity, and the one that was missed: a fighter that
    /// loses a stock never leaves the cast, it is teleported from the blast
    /// zone back onto the respawn platform, so the population is unchanged and
    /// the box collapses inward by the width of the stage in a single tick.
    teleported: bool,
    /// The body the presented-pose sample is taken from — the first seat, so
    /// the choice is stable rather than whichever entity sorted first.
    anchor: bevy::prelude::Entity,
}

/// Half the extra room left around the cast's bounding box, in world units.
/// Small on purpose: the view is a FLOOR, so authored zoom still wins whenever
/// it is already wider.
const CAST_FRAMING_MARGIN: f32 = 48.0;

/// Exponential close rate for cast framing.
///
/// Outward edges are adopted immediately so a launched fighter stays visible;
/// inward edges ease at this rate.
///
/// An exponential chase settles at a lag of `closing speed / rate`, so this
/// number is what decides how far behind the fight the framing sits while
/// bodies are closing — and 5Hz left the view 36 units behind two CPUs that
/// were only 3 units apart horizontally. That was invisible until the fight
/// started launching bodies vertically, which is a good illustration of a
/// camera constant being a claim about the FIGHT rather than about the camera.
///
/// It could not be raised while the same easing was also responsible for
/// absorbing discontinuities — a faster rate makes a collapse jerkier in exact
/// proportion. Now that [`CAST_FRAMING_CLOSE_MAX_UNITS_PER_S`] owns the
/// discontinuity separately, the two are independent and this is free to track.
const CAST_FRAMING_CLOSE_HZ: f32 = 10.0;

/// Ceiling on how fast an inward cast edge may close WHILE A DISCONTINUITY IS
/// SETTLING, in world units per second.
///
/// Two facts make this a special case rather than a general speed limit.
///
/// Exponential easing moves a FRACTION of the remaining gap, so its first step
/// is proportional to the size of the discontinuity: a fighter leaving play
/// collapsed the cast box by 241 units and the "eased" close still moved the
/// framing further on that one frame than any ordinary frame of the match — the
/// cut the easing exists to prevent, and the bigger the jump the bigger the
/// jerk.
///
/// But a cap that is always on cannot tell a collapse from an APPROACH. Bodies
/// close the box under their own power all match long — a fall or a reversed
/// launch closes an edge as fast as a body travels — so a general cap makes the
/// view lag the fight continuously and never catch up. Shipping one at 240
/// units/s did exactly that.
///
/// So it is gated on the thing that actually IS discontinuous: the cast losing
/// a member. A cast that shrank did not move, it changed shape. During ordinary
/// play this never binds at all, and the rate alone governs.
///
/// Because it is gated, its VALUE is free: it trades only how long a collapse
/// takes to settle against how much of it lands on one frame. At 400 units/s a
/// 241-unit collapse contributes ~3 units to the framing centre on the frame it
/// happens — well inside ordinary per-frame motion — and is gone in about six
/// tenths of a second. An ungated cap could not be set here; it would have
/// throttled every approach in the match.
const CAST_FRAMING_CLOSE_MAX_UNITS_PER_S: f32 = 400.0;

/// How long the cap stays armed after the cast loses a member.
///
/// Long enough to spread the collapse the population change caused, short
/// enough that it is gone before ordinary play resumes. At the cap above, a
/// 241-unit collapse closes in about a fifth of a second.
const CAST_FRAMING_SETTLE_SECONDS: f32 = 0.5;

/// Does this frame's cast population mean a discontinuity is starting?
///
/// A DROP only. A cast that GAINED a member grew its box outward, and outward
/// edges are adopted immediately — there is nothing to smooth.
///
/// ⛔ This is not the whole question, and reading it as though it were is how
/// the respawn lurch survived: a fighter losing a stock is a member that
/// neither left nor joined. See [`CastFraming::teleported`].
fn a_cast_member_was_lost(previous: Option<usize>, now: usize) -> bool {
    previous.is_some_and(|previous| now < previous)
}

/// Which cast members arrived somewhere they could not have travelled to?
///
/// `last_seen` is this system's own record of where each member was on the
/// previous resolve, updated in place. A member with no record yet is new and
/// cannot have teleported.
///
/// ⭐ THE PREDICATE IS `presented_pose`'s, deliberately. That module already
/// refuses to EXTRAPOLATE across a teleport; this one must refuse to CHASE
/// one, and both are the same question about the same bodies. A second
/// implementation here would be a second opinion that drifts.
///
/// One tick is assumed rather than measured, which is the conservative
/// direction: a frame that advanced two ticks could read a very fast body as a
/// teleport, and the only consequence is that the cap arms for half a second
/// during play it would not otherwise touch.
fn placed_cast_members(
    last_seen: &mut Vec<(bevy::prelude::Entity, ae::Vec2)>,
    now: &[(bevy::prelude::Entity, ae::Vec2, ae::Vec2)],
) -> Vec<bevy::prelude::Entity> {
    let mut placed = Vec::new();
    for (entity, pos, vel) in now {
        if let Some((_, was)) = last_seen.iter().find(|(seen, _)| seen == entity) {
            if !crate::presented_pose::travelled_under_own_power(*was, *pos, *vel, 1) {
                placed.push(*entity);
            }
        }
    }
    last_seen.clear();
    last_seen.extend(now.iter().map(|(entity, pos, _)| (*entity, *pos)));
    placed
}

/// How far an inward edge may move this frame, in world units.
///
/// `f32::INFINITY` during ordinary play: the exponential rate alone governs an
/// approach, and anything finite here lags the fight. Finite only while a
/// population drop is settling.
fn cast_close_allowance(settling_seconds: f32, dt: f32) -> f32 {
    if settling_seconds > 0.0 {
        CAST_FRAMING_CLOSE_MAX_UNITS_PER_S * dt
    } else {
        f32::INFINITY
    }
}

/// Move one presented cast-box edge toward the current cast edge.
///
/// `outward` is `+1` for a maximum edge and `-1` for a minimum. Expansion is immediate;
/// contraction eases at [`CAST_FRAMING_CLOSE_HZ`] and is additionally capped at
/// `max_step` world units this frame. Easing the box keeps center, zoom, and clamp coherent.
///
/// ⛔ EXPANSION MUST STAY IMMEDIATE. Capping it while a discontinuity settles
/// looks like the obvious way to absorb a respawn — the teleport moves one edge
/// inward and the opposite edge outward in the same tick — and it took the
/// stock-loss lurch from 143 units to 6.7. It also put a live fighter outside
/// the frame on four body-frames of one match, which
/// `every_live_fighter_stays_inside_the_frame` caught: a body launched during
/// the settle window outruns a capped edge. The respawn is absorbed by taking
/// the PLACED body out of the box instead — see `frame_the_cast`.
fn ease_cast_edge(previous: f32, current: f32, alpha: f32, outward: f32, max_step: f32) -> f32 {
    if (current - previous) * outward >= 0.0 {
        return current;
    }
    let eased = (current - previous) * alpha;
    // Never overshoot the target, and never move faster than the cap.
    let step = eased.clamp(-max_step.abs(), max_step.abs());
    let step = if step.abs() > (current - previous).abs() {
        current - previous
    } else {
        step
    };
    previous + step
}

/// Framing bounds and stable anchor for the declared cast.
///
/// ⛔ EVERY MEMBER IS IN THE BOX, INCLUDING ONE THAT WAS JUST PLACED. Leaving a
/// respawning fighter out looks like the clean way to absorb the teleport — it
/// took the stock-loss lurch from 143 units to 6.7 — and
/// `every_live_fighter_stays_inside_the_frame` refused it on 8 body-frames at up
/// to 97 units, correctly: the excluded body IS the respawning fighter, it is
/// alive and drawn on the respawn platform, and a frame that does not contain it
/// is a frame with a live fighter outside it. The camera has to move.
fn frame_the_cast(
    cast: &[bevy::prelude::Entity],
    bodies: &bevy::prelude::Query<&ambition_platformer2d_shared_tangle::body::BodyKinematics>,
    last_seen: &mut Vec<(bevy::prelude::Entity, ae::Vec2)>,
) -> Option<CastFraming> {
    let mut anchor = None;
    let mut members = 0usize;
    let mut now: Vec<(bevy::prelude::Entity, ae::Vec2, ae::Vec2)> = Vec::new();
    let (mut min, mut max) = (
        ae::Vec2::new(f32::MAX, f32::MAX),
        ae::Vec2::new(f32::MIN, f32::MIN),
    );
    for entity in cast {
        let Ok(kin) = bodies.get(*entity) else {
            continue;
        };
        anchor.get_or_insert(*entity);
        members += 1;
        now.push((*entity, kin.pos, kin.vel));
        let half = kin.size / 2.0;
        min.x = min.x.min(kin.pos.x - half.x);
        min.y = min.y.min(kin.pos.y - half.y);
        max.x = max.x.max(kin.pos.x + half.x);
        max.y = max.y.max(kin.pos.y + half.y);
    }
    let teleported = !placed_cast_members(last_seen, &now).is_empty();
    let anchor = anchor?;
    Some(CastFraming {
        bounds: ae::Aabb {
            min: min.into(),
            max: max.into(),
        },
        members,
        teleported,
        anchor,
    })
}

pub fn resolve_camera_observation(
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
    time: bevy::prelude::Res<bevy::prelude::Time>,
    developer_tools: bevy::prelude::Res<ambition_dev_tools::dev_tools::DeveloperTools>,
    encounter_view: bevy::prelude::Res<ambition_encounter::EncounterView>,
    user_settings: bevy::prelude::Res<ambition_persistence::settings::UserSettings>,
    ease_tuning: bevy::prelude::Res<
        ambition_platformer2d_shared_tangle::camera_ease::CameraEaseTuning,
    >,
    // They are components on a view entity now, so reading one requires naming WHICH view, and a
    // second local view is an extra row rather than an architecture. See `local_view`.
    mut views: bevy::prelude::Query<
        (
            &CameraViewport,
            &CameraScreenFraming,
            &CameraPresentationInputs,
            &CameraReferenceFrame,
            // `None` — every single-view composition — frames the session's, resolved once
            // above the loop.
            &crate::local_view::ResolvedViewSubject,
            &mut ambition_platformer2d_shared_tangle::camera_ease::CameraEaseState,
            &mut ResolvedCameraSnapshot,
        ),
        bevy::prelude::With<crate::local_view::LocalView>,
    >,
    mut last_camera_room: bevy::prelude::Local<Option<String>>,
    // WHERE THE FOLLOWED SUBJECT WAS on the previous resolve, so a body that
    // was PUT somewhere can be told from one that travelled there.
    //
    // ⭐⭐ THE CAST CAMERA ALREADY HAD THIS TERM and the single-subject FOLLOW
    // had none — `snap_camera` was `blink || room_changed`, so a teleport
    // INSIDE one room was chased at the ease rate. Measured by Jon: a synthetic
    // teleport panned the view 440px over about 40 ticks.
    //
    // A `Local` for the same reason `last_camera_room` is one: it is this
    // system's own record of what it last presented, not simulation state.
    mut last_subject_placement: bevy::prelude::Local<Option<(bevy::prelude::Entity, ae::Vec2)>>,
    // THE CAST FRAMING'S PRESENTED STATE, as one param.
    //
    //  `Local`s rather than fields on `CameraEaseState`, and for the same
    // reason `last_camera_room` is one: what the CAST spans is a fact about the
    // world this system resolves ONCE, above the per-observer loop, while
    // `CameraEaseState` is per view. Presentation-only either way — nothing in
    // the simulation reads any of it.
    //
    //  grouped into a tuple because this system is at Bevy's parameter
    // ceiling, which is the same pressure that produced `followed_body` below.
    // They are one thing anyway: the box being presented, the population and
    // the positions it was last seen at, and how long the discontinuity cap
    // stays armed.
    cast_framing_state: (
        // The box eased toward the cast's real bounds on the way IN. See
        // `CAST_FRAMING_CLOSE_HZ`.
        bevy::prelude::Local<Option<ae::Aabb>>,
        // The population last seen — a DROP arms the cap.
        bevy::prelude::Local<Option<usize>>,
        // Where each member was last seen — a member that did not TRAVEL to
        // where it now is arms the cap too. See `placed_cast_members`.
        bevy::prelude::Local<Vec<(bevy::prelude::Entity, ae::Vec2)>>,
        // How long the cap stays armed.
        bevy::prelude::Local<f32>,
    ),
    player: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            &ambition_platformer2d_shared_tangle::body::BodyKinematics,
            &ae::BodyBaseSize,
            &ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
        ),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    // The session's default subject, for a view that names none of its own.
    controlled: bevy::prelude::Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>,
    // What to look at when nothing is driving a body — see the `None` arm below.
    framed: bevy::prelude::Res<ambition_platformer2d_shared_tangle::markers::FramedCast>,
    // Subject resolution moved to `resolve_view_subjects` and the headroom came back; the grouping
    // stayed because it was always the honest shape.
    //
    // The camera frames the PRESENTED subject, not the raw tick pose: this and
    // the sprite must sample the same frame-clock position, or they disagree by
    // up to a tick of travel and the subject shudders — see `presented_pose`.
    followed_body: (
        bevy::prelude::Query<&ambition_platformer2d_shared_tangle::body::BodyKinematics>,
        bevy::prelude::Query<&crate::presented_pose::PresentedPose>,
        // The frame the followed body resolved this tick (ADR 0024), for a
        // view that presents in its subject's frame rather than the world's. Read
        // off the SAME entity the framing follows, so orientation and framing
        // cannot disagree about whose view this is — and read as an
        // already-resolved fact, never by asking gravity anything.
        bevy::prelude::Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    ),
) {
    let (body_kinematics, presented, subject_frames) = followed_body;
    let (mut live_cast, mut live_members, mut cast_last_seen, mut settling) = cast_framing_state;
    // Dev tools can temporarily replace the authored/default camera view.
    let (base_view_w, base_view_h) = if developer_tools.camera_view_override_enabled {
        (
            developer_tools.camera_view_w.max(64.0),
            developer_tools.camera_view_h.max(64.0),
        )
    } else {
        user_settings.video.camera_zoom.base_view()
    };
    let mut base_view = ae::Vec2::new(base_view_w, base_view_h);
    let overview_scale = developer_tools.overview_camera_scale.max(1.0);
    let encounter_scale = encounter_view.camera_zoom.max(1.0);

    // That is the failure mode this repo has been bitten by repeatedly: presentation not
    // running looks exactly like presentation running badly.
    //
    // The home avatar remains the source of blink easing and the base size when
    // there IS one, because those are its presentation state. When there is not,
    // the CONTROLLED SUBJECT supplies the frame on its own.
    // Set only by the FRAMED-CAST arm below: every other follow has a single
    // subject the ordinary clamp already keeps on screen, and relaxing a room
    // clamp for one of those would just show the void beside the room.
    let mut must_frame_world: Option<ae::Aabb> = None;
    let home = player.single().ok().map(|(e, b, bs, bc)| (e, *b, *bs, *bc));
    let (mut player_body, player_base_size, blink_cam, mut followed) = match home {
        Some((entity, body, base_size, blink)) => (body, base_size, blink, entity),
        None => {
            // What to frame is a presentation decision this resolver is TOLD (`FramedCast`), never
            // one it guesses: a scan for bodies would have to decide which ones matter, and whoever
            // published the cast already knows.
            match controlled
                .0
                .and_then(|subject| body_kinematics.get(subject).ok().map(|kin| (*kin, subject)))
            {
                Some((kin, subject)) => (
                    kin,
                    ae::BodyBaseSize {
                        base_size: kin.size,
                    },
                    // No home avatar means no blink state to ease from, which is
                    // correct rather than a fallback: a fighter does not blink.
                    ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState::default(),
                    subject,
                ),
                None => {
                    let Some(cast) =
                        frame_the_cast(&framed.0, &body_kinematics, &mut cast_last_seen)
                    else {
                        //  forget the eased framing with the cast, or the next
                        // match opens by closing in from the last one's final
                        // spread instead of at its own true framing.
                        *live_cast = None;
                        *live_members = None;
                        cast_last_seen.clear();
                        *settling = 0.0;
                        return;
                    };
                    // A FLOOR, so authored zoom still wins when wider.
                    let dt = time.delta_secs().max(0.0);
                    let alpha = (1.0 - (-CAST_FRAMING_CLOSE_HZ * dt).exp()).clamp(0.0, 1.0);
                    // THE CAP IS FOR DISCONTINUITIES, and this resolve has
                    // TWO. A cast can lose a member, and a member can be put
                    // somewhere rather than travel there. Arm on either; leave
                    // ordinary closing to the rate alone, or the view lags
                    // every approach for the whole match.
                    if a_cast_member_was_lost(*live_members, cast.members) || cast.teleported {
                        *settling = CAST_FRAMING_SETTLE_SECONDS;
                    }
                    *live_members = Some(cast.members);
                    let max_step = cast_close_allowance(*settling, dt);
                    *settling = (*settling - dt).max(0.0);
                    let bounds = match *live_cast {
                        Some(previous) => ae::Aabb {
                            min: ae::Vec2::new(
                                ease_cast_edge(
                                    previous.min.x,
                                    cast.bounds.min.x,
                                    alpha,
                                    -1.0,
                                    max_step,
                                ),
                                ease_cast_edge(
                                    previous.min.y,
                                    cast.bounds.min.y,
                                    alpha,
                                    -1.0,
                                    max_step,
                                ),
                            )
                            .into(),
                            max: ae::Vec2::new(
                                ease_cast_edge(
                                    previous.max.x,
                                    cast.bounds.max.x,
                                    alpha,
                                    1.0,
                                    max_step,
                                ),
                                ease_cast_edge(
                                    previous.max.y,
                                    cast.bounds.max.y,
                                    alpha,
                                    1.0,
                                    max_step,
                                ),
                            )
                            .into(),
                        },
                        // The first frame ADOPTS rather than easing: a match must
                        // open already framed, not zoom in from nothing.
                        None => cast.bounds,
                    };
                    *live_cast = Some(bounds);
                    let centre: ae::Vec2 = bounds.center().into();
                    let span: ae::Vec2 = (bounds.max - bounds.min).into();
                    let presented = span + ae::Vec2::splat(CAST_FRAMING_MARGIN * 2.0);
                    base_view = base_view.max(presented);
                    // AND THE CLAMP IS TOLD WHAT IT MAY NOT HIDE. Framing the
                    // cast is worth nothing if the room clamp then throws the
                    // centre away, which is exactly what it did — see
                    // `CameraSnapshotResolveInput::must_frame_world`.
                    //
                    //  the PRESENTED box, not the cast's raw one: the presented
                    // box contains the raw one on every frame that matters (an
                    // edge moving outward is adopted), and holding the raw box
                    // would make the clamp cut back the instant a body left play
                    // while the view was still easing out there — the eased close
                    // and a hard clamp fighting over the same frame.
                    must_frame_world = Some(bounds);
                    (
                        ae::BodyKinematics {
                            pos: centre,
                            //  the framed CAST is not a body that crouches:
                            // its extent and its baseline extent are the same
                            // number by construction, so saying so once here
                            // makes the compensation vanish instead of being
                            // special-cased downstream.
                            size: presented,
                            ..Default::default()
                        },
                        // The PRESENTED framing, not the raw box — the two must
                        // be one number or the base size would close on the
                        // frame the view is still easing through.
                        ae::BodyBaseSize {
                            base_size: presented,
                        },
                        ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState::default(
                        ),
                        cast.anchor,
                    )
                }
            }
        }
    };
    // Follow the CONTROLLED SUBJECT's body. Zoom + blink easing stay on the
    // home avatar's presentation state; only the follow point tracks the
    // driven body.
    if let Some(subject) = controlled.0 {
        if let Ok(kin) = body_kinematics.get(subject) {
            player_body.pos = kin.pos;
            // Soft framing leads the DRIVEN body; leading the home avatar's
            // velocity while possessing something else would aim the camera at
            // where a body the participant is not controlling is going.
            player_body.vel = kin.vel;
            followed = subject;
        }
    }
    // Frame where the subject is DRAWN, not where its last tick left it. The
    // sprite is sampled on the same frame clock; if the camera framed the tick
    // pose instead, the two would disagree by up to a tick of travel and the
    // subject would shudder against the world at speed.
    //
    // For a FRAMED CAST they are not: `pos` is the pair's CENTRE, and assigning the anchor's
    // presented position throws that centre away and points the camera at seat 0. A framing
    // centre is rigidly attached to the cast exactly as a hitbox is to its owner, and both are
    // carried on the frame clock the same way.
    // The SIM position, kept before the presented offset is folded in — the
    // teleport predicate below is about where the BODY went.
    let subject_sim_pos = player_body.pos;
    if let Ok(presented) = presented.get(followed) {
        player_body.pos += presented.delta();
    }

    let active_spec = room_set.active_spec();
    let room_changed = last_camera_room.as_deref() != Some(active_spec.id.as_str());
    if room_changed {
        *last_camera_room = Some(active_spec.id.clone());
    }
    // ⭐⭐ AND THE SUBJECT WAS PUT THERE RATHER THAN TRAVELLING — the term the
    // FOLLOW path was missing. The cast camera has had it since the respawn
    // lurch (`CastFraming::teleported`); a single-subject view chased a teleport
    // inside one room at the ease rate, which Jon measured as a 440px pan over
    // about 40 ticks.
    //
    // ⭐ THE PREDICATE IS `presented_pose`'s, exactly as the cast path uses it.
    // That module already refuses to EXTRAPOLATE across a teleport and this
    // refuses to CHASE one; a second implementation here would be a second
    // opinion that drifts.
    //
    // ⛔ THE SIM POSITION, taken before the presented delta above is folded in:
    // the predicate is about where the BODY went, and the presented sample is a
    // sub-tick offset that would read as travel the velocity cannot explain.
    let subject_placed = match *last_subject_placement {
        // A DIFFERENT subject is not a teleport — possession and view changes
        // hand the camera a new body, and the room/blink terms cover the rest.
        Some((was, _)) if was != followed => false,
        Some((_, was_at)) => !crate::presented_pose::travelled_under_own_power(
            was_at,
            subject_sim_pos,
            player_body.vel,
            1,
        ),
        None => false,
    };
    *last_subject_placement = Some((followed, subject_sim_pos));
    let snap_camera = blink_cam.camera_snap_timer > 0.0 || room_changed;

    let focus = CameraFocus2d {
        center_world: player_body.pos,
        size: player_body.size,
        base_size: player_base_size.base_size,
        facing: player_body.facing,
        velocity_world: player_body.vel,
    };
    let blink = CameraBlinkInput {
        blink_in_timer: blink_cam.blink_in_timer,
        blink_in_duration: blink_cam.blink_in_duration,
        blink_camera_from: blink_cam.blink_camera_from,
    };
    let subject_down = subject_frames.get(followed).ok().map(|frame| frame.down());
    // THIS VIEW's own subject, framed on its own terms. Same three answers
    // the session resolve produces — where to look, which way is down there, and
    // what the snapshot should report as the follow point — for a body the
    // session is not following.
    //
    //  its own extent is its own baseline, which is the same answer the
    // no-home-avatar arm above gives: the crouch compensation in
    // `CameraFocus2d::stable_center` subtracts `(base_size.y - size.y) / 2`, and
    // there is no baseline pose to compare a spectated body against.
    let view_focus = |subject: bevy::prelude::Entity| {
        let kin = body_kinematics.get(subject).ok()?;
        // The PRESENTED position, on the frame clock, for the same reason the
        // session subject uses it: framing the tick pose while the sprite is
        // drawn on the frame clock makes the subject shudder at speed.
        let center = kin.pos + presented.get(subject).map_or(ae::Vec2::ZERO, |p| p.delta());
        Some((
            CameraFocus2d {
                center_world: center,
                size: kin.size,
                base_size: kin.size,
                facing: kin.facing,
                velocity_world: kin.vel,
            },
            subject_frames.get(subject).ok().map(|frame| frame.down()),
            center,
        ))
    };
    //  what to look at is a world question; how to present it is a VIEW
    // question. Everything above is resolved once — the followed body, the
    // room, the framing focus — and everything below is answered per observer.
    for (
        viewport,
        screen_framing,
        presentation,
        reference_frame,
        view_subject,
        mut camera_state,
        mut resolved,
    ) in &mut views
    {
        //  A VIEW MAY NAME ITS OWN SUBJECT, and everything that follows
        // from whose body it is moves with it: the framing focus, the down axis
        // orientation resolves against, and the follow point the snapshot
        // reports.
        //
        //  the two the override DROPS are as deliberate as the three it
        // replaces. `blink` is the HOME AVATAR's arrival easing — handing it to a
        // pane watching somebody else would yank that pane when the participant
        // blinks — and `must_frame_world` is the declared CAST's box, which is a
        // constraint on the view that is watching the cast and on no other.
        //  whether the view named a body or a seat was decided upstream, by
        // `resolve_view_subjects`. What is left here is what a camera resolve
        // should be doing with it: framing.
        let (focus, subject_down, follow_world, blink, must_frame_world) =
            match view_subject.0.and_then(view_focus) {
                Some((own_focus, own_down, own_center)) => {
                    (own_focus, own_down, own_center, None, None)
                }
                None => (
                    focus,
                    subject_down,
                    player_body.pos,
                    Some(blink),
                    must_frame_world,
                ),
            };
        if room_changed {
            // Disjoint LDtk areas: reset target easing so it never interpolates
            // through unrelated world coordinates. PER VIEW, because each view
            // eases toward its own target.
            camera_state.target_initialized = false;
        }
        // ⛔⛔ A TELEPORT A PORTAL IS ALREADY PRESENTING IS NOT OURS TO SNAP.
        // Portal continuity translates the body and holds it at the SAME SCREEN
        // POSITION by offsetting the view; snapping to the body on top of that
        // is two policies answering one discontinuity, and it showed up as a
        // 178px visible step through a floor-ceiling transit.
        //
        // ⭐ THE PORTAL'S OWN INPUTS SAY SO — written pre-resolve by the
        // presentation adapter, and already read three lines down for the
        // chart roll. No new channel, and no dependency on the portal crate
        // from here.
        //
        // ⛔ ONLY THE TELEPORT TERM IS SUPPRESSED: a room change or a blink
        // during a transit still snaps, exactly as before.
        let portal_presents_this_translation =
            presentation.extra_clamp_center_world.is_some() || presentation.chart_transit.is_some();
        let snap_camera = snap_camera || (subject_placed && !portal_presents_this_translation);
        let snapshot = resolve_follow_camera_snapshot(
            CameraSnapshotResolveInput {
                world: &world.0,
                camera_zones: &active_spec.camera_zones,
                focus,
                base_view,
                viewport_px: viewport.px,
                aspect_policy: user_settings.video.camera_aspect,
                framing: user_settings.video.camera_framing,
                overview_scale,
                encounter_scale,
                overview_camera: developer_tools.overview_camera,
                snap_camera,
                blink,
                dt: time.delta_secs(),
                mode: CameraSnapshotResolveMode::Eased,
                extra_clamp_center_world: presentation.extra_clamp_center_world,
                ease_tuning: *ease_tuning,
                screen_framing: Some(*screen_framing),
                // THIS VIEW's frame policy, read off the view that is being resolved.
                reference_frame: *reference_frame,
                subject_down,
                // Written pre-resolve by the portal adapter, exactly like the
                // extra clamp beside it — so the snapshot states the view's
                // ACTUAL final orientation instead of one the renderer
                // overwrites afterwards.
                chart_transit: presentation.chart_transit,
                must_frame_world,
            },
            Some(&mut *camera_state),
        );
        *resolved = ResolvedCameraSnapshot {
            snapshot,
            follow_world,
        };
    }
}

/// Ordering handle for the camera observation resolve.
///
/// Everything that FEEDS the resolve (the host's presentation layout) orders
/// `.before` this; everything that CONSUMES it (`camera_follow`, the physical
/// viewport application, the surround) orders `.after`. One handle, one
/// schedule, so the relationship is expressible rather than assumed.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct CameraObservationSet;

/// Camera observation runs once per rendered frame in `Update`.
///
/// [`CameraViewState`] stores diagnostics on each view rather than in a global
/// resource so multi-view readers always consume the state for their own view.
#[derive(bevy::prelude::Component, Clone, Debug)]
pub struct CameraViewState {
    pub base_view: ambition_platformer2d_core::Vec2,
    pub requested_view: ambition_platformer2d_core::Vec2,
    pub visible_view: ambition_platformer2d_core::Vec2,
    pub zoom_multiplier: f32,
    pub orthographic_scale: f32,
    pub target_world: ambition_platformer2d_core::Vec2,
    pub center_world: ambition_platformer2d_core::Vec2,
    pub active_camera_zones: usize,
    pub active_camera_zone: Option<String>,
}

impl Default for CameraViewState {
    fn default() -> Self {
        Self::from(&CameraSnapshot2d::default())
    }
}

impl From<&CameraSnapshot2d> for CameraViewState {
    fn from(snapshot: &CameraSnapshot2d) -> Self {
        Self {
            base_view: snapshot.base_view,
            requested_view: snapshot.requested_view,
            visible_view: snapshot.visible_view,
            zoom_multiplier: snapshot.zoom_multiplier,
            orthographic_scale: snapshot.orthographic_scale,
            target_world: snapshot.target_world,
            center_world: snapshot.center_world,
            active_camera_zones: snapshot.active_camera_zones,
            active_camera_zone: snapshot.active_camera_zone.clone(),
        }
    }
}

/// Resolve the single view presented by the main camera for diagnostics.
///
/// A camera with `PresentsView` selects that view. An unlinked single-camera,
/// single-view composition selects its only view. Multiple views without a link,
/// or multiple main cameras, are ambiguous and return no presented view rather
/// than choosing by iteration order. Draw systems that support multiview should
/// iterate views directly instead of using this single-view helper.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PresentedViewState<'w, 's> {
    cameras: bevy::prelude::Query<
        'w,
        's,
        Option<&'static crate::local_view::PresentsView>,
        bevy::prelude::With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>,
    >,
    views: bevy::prelude::Query<
        'w,
        's,
        (bevy::prelude::Entity, &'static CameraViewState),
        bevy::prelude::With<crate::local_view::LocalView>,
    >,
}

impl PresentedViewState<'_, '_> {
    /// The presented view's state, or `None` when there is no view to read (a
    /// headless or pre-composition host).
    pub fn get(&self) -> Option<&CameraViewState> {
        let mut presenting = self.cameras.iter();
        let first = presenting.next();
        if presenting.next().is_some() {
            bevy::log::error_once!(
                "a draw system asked for THE presented view state while several main \
                 cameras exist; refusing to guess. Each camera presents its own view, \
                 so this reader has to be keyed by view before split-screen can land."
            );
            return None;
        }
        // The binding rule is stated once, in `local_view` — this reader shares
        // it with `camera_follow` and the physical viewport applier rather than
        // keeping a third copy that can drift.
        let on_hand =
            crate::local_view::ViewsOnHand::survey(self.views.iter().map(|(view, _)| view));
        let view = on_hand.presented_by(first.flatten().copied())?;
        self.views.get(view).ok().map(|(_, state)| state)
    }
}

/// The player's camera-frame setting, applied to the local view.
///
///  the component is still the selection; this only gives it a player-facing source. left
/// the selection unbuilt on purpose so the policy could not become a process-global mode, and
/// that constraint is intact: this writes a per-view COMPONENT, so when views become indexed
/// each one can take its policy from wherever it likes and nothing here has to be removed.
///
///  writes only on a real change. An unconditional write would mark the
/// component changed every frame, and `is_changed()` ticks do not rewind — a
/// rollback resimulation would see a fresh "changed" on every replayed tick.
///
///  absent settings leave the component alone rather than forcing a default, so
/// a host that has no `UserSettings` (the unit tests, a headless probe) keeps the
/// "a game can just write the component" path that `local_view` pins.
fn apply_camera_reference_frame_setting(
    user_settings: Option<bevy::prelude::Res<ambition_persistence::settings::UserSettings>>,
    mut views: bevy::prelude::Query<&mut CameraReferenceFrame>,
) {
    let Some(settings) = user_settings.as_deref() else {
        return;
    };
    let wanted = settings.gameplay.camera_reference_frame;
    for mut frame in &mut views {
        if *frame != wanted {
            *frame = wanted;
        }
    }
}

/// EVERY FACT A LOCAL VIEW CARRIES, IN ONE PLACE.
///
///  it is a function, not a comment, because the alternative is a
/// hand-kept carry list. The single-view path
/// ([`CameraObservationPlugin`]) and the N-view composition helper
/// ([`crate::local_view::compose_local_views`]) both need a view that satisfies
/// the resolve's query, and a component present in one and missing in the other
/// is not a compile error, not a panic, and not a log line — it is a view that
/// silently does not match `resolve_camera_observation`'s query, which reads as
/// a camera frozen at the origin. With one definition the two paths cannot
/// differ; adding a fact here reaches both by construction.
///
///  the count is the contract `local_view:tests` pins component-by-component: the identity
/// ([`crate:local_view:LocalViewId`]) is passed separately, so what is here is exactly the six
/// facts moved off process-globals.
pub fn local_view_facts() -> impl bevy::prelude::Bundle {
    (
        CameraViewport::default(),
        CameraScreenFraming::default(),
        CameraPresentationInputs::default(),
        CameraReferenceFrame::default(),
        ambition_platformer2d_shared_tangle::camera_ease::CameraEaseState::default(),
        ResolvedCameraSnapshot::default(),
        // Where this view is looking, resolved from its own declaration before
        // anything frames it. Here rather than at the two spawn sites for the
        // reason this whole function exists.
        crate::local_view::ResolvedViewSubject::default(),
        // Carried here for the same reason as the others — a reader must never see a frame
        // where the view exists and its state does not.
        CameraViewState::default(),
    )
}

pub struct CameraObservationPlugin;

impl bevy::prelude::Plugin for CameraObservationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;
        use bevy::prelude::IntoScheduleConfigs as _;
        //  THE VIEW IS SPAWNED HERE, at plugin BUILD time, and that is not
        // a detail. From a startup system there would be one frame with no
        // view, so every reader would need `single()` + `else { return }` — the
        // shape that has produced four production defects in this repository,
        // because a system that silently does nothing looks exactly like one
        // that ran. A view that exists before any schedule runs cannot produce
        // that frame.
        crate::local_view::spawn_local_view(
            app.world_mut(),
            crate::local_view::LocalViewId::FIRST,
            local_view_facts(),
        );

        // Declared ONLY when the sim shares this schedule.
        if app.sim_is(bevy::prelude::Update) {
            app.configure_sets(
                bevy::prelude::Update,
                CameraObservationSet
                    .after(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::CoreSimulation),
            );
        }
        // The resolve frames the PRESENTED subject, so the frame-clock resample
        // must already have happened this frame.
        app.configure_sets(
            bevy::prelude::Update,
            CameraObservationSet.after(crate::presented_pose::PresentedPoseSet),
        );
        app.add_systems(
            bevy::prelude::Update,
            (
                apply_camera_reference_frame_setting,
                // WHO EACH VIEW IS WATCHING, before anything frames one.
                // Chained, so the resolve below reads a fact rather than
                // searching control authority for it.
                crate::local_view::resolve_view_subjects,
                resolve_camera_observation,
            )
                .chain()
                .in_set(CameraObservationSet),
        );
    }
}

#[cfg(test)]
mod m2_forward_scroll_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::camera_ease::CameraEaseState;

    fn world() -> ae::World {
        ae::World::new(
            "m2",
            ae::Vec2::new(4000.0, 600.0),
            ae::Vec2::ZERO,
            Vec::new(),
        )
    }

    fn zone(policy: CameraScrollPolicy) -> CameraZoneSpec {
        CameraZoneSpec {
            id: "scroll".into(),
            name: "scroll".into(),
            aabb: ae::Aabb::new(ae::Vec2::new(2000.0, 300.0), ae::Vec2::new(2000.0, 300.0)),
            priority: 0,
            zoom: Some(1.0),
            target_offset: ae::Vec2::ZERO,
            easing_hz: None,
            cinematic_lock: false,
            clamp_mode: CameraClampMode::None,
            scroll_policy: policy,
        }
    }

    fn resolve(
        world: &ae::World,
        zones: &[CameraZoneSpec],
        x: f32,
        ease: &mut CameraEaseState,
    ) -> f32 {
        let snap = resolve_follow_camera_snapshot(
            CameraSnapshotResolveInput {
                world,
                camera_zones: zones,
                focus: CameraFocus2d {
                    center_world: ae::Vec2::new(x, 300.0),
                    size: ae::Vec2::new(24.0, 40.0),
                    base_size: ae::Vec2::new(24.0, 40.0),
                    facing: 1.0,
                    velocity_world: ae::Vec2::ZERO,
                },
                base_view: ae::Vec2::new(480.0, 270.0),
                viewport_px: ae::Vec2::new(480.0, 270.0),
                aspect_policy: CameraAspectPolicy::FixedHeight,
                framing: CameraFramingPreset::default(),
                overview_scale: 1.0,
                encounter_scale: 1.0,
                overview_camera: false,
                snap_camera: true,
                blink: None,
                dt: 1.0 / 60.0,
                mode: CameraSnapshotResolveMode::Eased,
                extra_clamp_center_world: None,
                chart_transit: None,
                must_frame_world: None,
                ease_tuning: CameraEaseTuning::default(),
                screen_framing: None,
                reference_frame: Default::default(),
                subject_down: None,
            },
            Some(ease),
        );
        snap.center_world.x
    }

    /// The wiring, not just the clamp. A player who runs right and then walks
    /// back left leaves the camera where it was. This is the whole of Mary-O's
    /// scroll rule, resolved through the real snapshot path.
    #[test]
    fn a_forward_only_zone_refuses_to_scroll_back() {
        let w = world();
        let zones = [zone(CameraScrollPolicy::ForwardOnlyX)];
        let mut ease = CameraEaseState::default();

        let far = resolve(&w, &zones, 1800.0, &mut ease);
        let back = resolve(&w, &zones, 1400.0, &mut ease);
        assert!(
            (back - far).abs() < 0.5,
            "camera followed the player back: {far} -> {back}"
        );
        // ...and forward progress still works.
        let further = resolve(&w, &zones, 2200.0, &mut ease);
        assert!(further > far + 100.0, "{far} -> {further}");
    }

    /// A `Free` zone — every zone authored before — follows the player both ways, and clears
    /// any watermark it inherited from a forward-only zone it just left.
    #[test]
    fn a_free_zone_follows_both_ways_and_clears_the_watermark() {
        let w = world();
        let zones = [zone(CameraScrollPolicy::Free)];
        let mut ease = CameraEaseState {
            scroll_watermark_x: Some(9999.0),
            ..Default::default()
        };

        let far = resolve(&w, &zones, 1800.0, &mut ease);
        assert!(ease.scroll_watermark_x.is_none(), "leaving clears it");
        let back = resolve(&w, &zones, 1400.0, &mut ease);
        assert!(
            back < far - 100.0,
            "a free camera comes back: {far} -> {back}"
        );
    }
}

#[cfg(test)]
mod cast_framing_tests {
    use super::*;

    /// A fighter leaving play collapses the cast box, and the frame it happens
    /// on must look like every other frame.
    ///
    /// Exponential easing alone cannot promise that: it moves a FRACTION of the
    /// gap, so the step it takes scales with the size of the discontinuity —
    /// the bigger the cut, the bigger the jerk. This is the regression that
    /// produced a visible camera cut on the frame a stock was taken.
    #[test]
    fn a_collapse_of_any_size_closes_at_the_same_speed() {
        let dt = 1.0 / 60.0;
        let alpha = (1.0 - (-CAST_FRAMING_CLOSE_HZ * dt).exp()).clamp(0.0, 1.0);
        let max_step = CAST_FRAMING_CLOSE_MAX_UNITS_PER_S * dt;

        // A max edge collapsing inward by a little, and by a lot.
        let small = ease_cast_edge(100.0, 80.0, alpha, 1.0, max_step) - 100.0;
        let huge = ease_cast_edge(100.0, -400.0, alpha, 1.0, max_step) - 100.0;
        assert!(small < 0.0 && huge < 0.0, "both close inward");
        assert!(
            huge.abs() <= max_step + 1e-3,
            "a 500-unit collapse must not move {max_step} in one frame: moved {huge}"
        );
        // THE PROPERTY, stated as size-independence: once past the cap, how far
        // the framing moves this frame stops depending on how big the
        // discontinuity was. Uncapped, a 500-unit collapse steps 40 and a
        // 50,000-unit one steps 4,000 — the jerk scaling with the cut is the
        // whole defect.
        let enormous = ease_cast_edge(100.0, -50_000.0, alpha, 1.0, max_step) - 100.0;
        assert_eq!(
            huge, enormous,
            "a collapse 100x larger must move the framing exactly as far"
        );
        assert!(
            small.abs() < max_step,
            "a small collapse still eases rather than riding the cap: {small}"
        );
    }

    /// THE OTHER HALF of what the cap owes, and the one it originally broke.
    ///
    /// Size-independence says a big collapse and a small one move the framing
    /// equally. It says nothing about whether the framing KEEPS UP with a
    /// fight, and the first cap traded one property for the other: a general
    /// speed limit cannot tell a collapse from an APPROACH, so at 240 units/s
    /// it throttled ordinary play and the view lagged the fight forever.
    ///
    /// The cap is armed by a population DROP now, so during ordinary play the
    /// allowance is unbounded and the exponential rate alone governs. This is
    /// that rule, and it is the one that failed on the gate.
    #[test]
    fn ordinary_play_is_never_governed_by_the_cap() {
        let dt = 1.0 / 60.0;
        assert_eq!(
            cast_close_allowance(0.0, dt),
            f32::INFINITY,
            "an approach must be tracked by the rate alone"
        );
        assert!(
            cast_close_allowance(CAST_FRAMING_SETTLE_SECONDS, dt).is_finite(),
            "a settling collapse must be capped"
        );

        // And ONE of the two arming rules — the population half.
        assert!(a_cast_member_was_lost(Some(4), 3), "an elimination arms it");
        assert!(
            !a_cast_member_was_lost(Some(3), 3),
            "a steady cast does not"
        );
        assert!(
            !a_cast_member_was_lost(Some(3), 4),
            "a respawn grows the box OUTWARD, which is adopted immediately"
        );
        assert!(
            !a_cast_member_was_lost(None, 2),
            "the first frame adopts rather than settling"
        );
    }

    /// A FIGHTER THAT LOSES A STOCK NEVER LEAVES THE CAST, and that is why the
    /// population test alone could not see it.
    ///
    /// Measured before this existed: over one 5,400-tick CPU match the three
    /// largest single-tick camera steps were the three stock losses — 143.7,
    /// 86.3 and 82.1 world units against a p99 of 13.1 for every other tick in
    /// the match. The elimination, which DOES drop the population, was already
    /// smooth at 3.3. The cap was doing its job perfectly for the case it knew
    /// about and was blind to its sibling.
    ///
    /// The negative cases are the whole test: a body travelling fast under its
    /// own power must NOT arm the cap, or this becomes the general speed limit
    /// that was already tried and reverted for lagging every approach.
    #[test]
    fn a_respawned_fighter_is_a_discontinuity_even_though_the_cast_is_the_same_size() {
        use bevy::prelude::Entity;
        let body = Entity::from_raw_u32(1).unwrap();
        let other = Entity::from_raw_u32(2).unwrap();

        // First sight of a member cannot be a teleport.
        let mut seen = Vec::new();
        assert!(placed_cast_members(
            &mut seen,
            &[(body, ae::Vec2::new(100.0, 0.0), ae::Vec2::ZERO)]
        )
        .is_empty());

        // Ordinary travel, at a speed a launch really reaches.
        assert!(placed_cast_members(
            &mut seen,
            &[(body, ae::Vec2::new(125.0, 0.0), ae::Vec2::new(1500.0, 0.0))]
        )
        .is_empty());

        // THE RESPAWN: `reset_body_clusters` zeroes velocity and puts the body
        // over the platform, so the step is enormous and nothing was moving.
        assert_eq!(
            placed_cast_members(
                &mut seen,
                &[(body, ae::Vec2::new(900.0, -400.0), ae::Vec2::ZERO)]
            ),
            vec![body],
            "and it names WHICH body, so the caller can sit it out of the box"
        );

        // A second member joining is not a teleport — it has no history, and a
        // cast that GREW is the case the population rule already handles by
        // adopting the outward edge immediately.
        assert!(placed_cast_members(
            &mut seen,
            &[
                (body, ae::Vec2::new(900.0, -400.0), ae::Vec2::ZERO),
                (other, ae::Vec2::new(-900.0, -400.0), ae::Vec2::ZERO),
            ]
        )
        .is_empty());
    }

    /// The cap has to be small enough that a collapse lands as ordinary motion.
    ///
    /// It used to owe a second, contradictory thing — clearing the speed a body
    /// travels — because an ungated cap governed ordinary play too. Gating it on
    /// a population drop removed that constraint, which is what made a value
    /// this low possible: what it now trades is settle TIME against how much of
    /// the collapse lands on one frame, and nothing else.
    #[test]
    fn a_collapse_lands_as_ordinary_motion() {
        let dt = 1.0 / 60.0;
        // The elimination that started this: 241 units of collapse.
        const OBSERVED_COLLAPSE: f32 = 241.4;
        // The largest step the camera takes on an ordinary frame of a real
        // match, measured by `the_framing_centre_absorbs_an_elimination…`.
        const ORDINARY_CENTRE_STEP: f32 = 20.0;

        let edge_step = CAST_FRAMING_CLOSE_MAX_UNITS_PER_S * dt;
        // One edge collapsing moves the box CENTRE by half as much.
        let centre_step = edge_step / 2.0;
        assert!(
            centre_step < ORDINARY_CENTRE_STEP / 2.0,
            "a collapse contributes {centre_step} to the centre, which is not \
             comfortably inside an ordinary frame's {ORDINARY_CENTRE_STEP}"
        );

        // And it still settles promptly rather than crawling for seconds.
        let settle_seconds = OBSERVED_COLLAPSE / CAST_FRAMING_CLOSE_MAX_UNITS_PER_S;
        assert!(
            settle_seconds < CAST_FRAMING_SETTLE_SECONDS * 1.5,
            "a {OBSERVED_COLLAPSE}-unit collapse takes {settle_seconds}s, longer \
             than the window that arms the cap"
        );
    }

    /// Outward is still immediate: a launched fighter must never leave frame
    /// while the box catches up.
    #[test]
    fn an_edge_moving_outward_is_adopted_at_once() {
        let dt = 1.0 / 60.0;
        let alpha = (1.0 - (-CAST_FRAMING_CLOSE_HZ * dt).exp()).clamp(0.0, 1.0);
        let max_step = CAST_FRAMING_CLOSE_MAX_UNITS_PER_S * dt;
        // A max edge sprinting outward, far further than the cap.
        assert_eq!(
            ease_cast_edge(100.0, 5_000.0, alpha, 1.0, max_step),
            5_000.0
        );
        // And a min edge, whose outward direction is the other way.
        assert_eq!(
            ease_cast_edge(-100.0, -5_000.0, alpha, -1.0, max_step),
            -5_000.0
        );
    }

    /// The close never overshoots the edge it is chasing, however small the gap.
    #[test]
    fn closing_settles_exactly_on_the_target() {
        let dt = 1.0 / 60.0;
        let alpha = (1.0 - (-CAST_FRAMING_CLOSE_HZ * dt).exp()).clamp(0.0, 1.0);
        let max_step = CAST_FRAMING_CLOSE_MAX_UNITS_PER_S * dt;
        // A gap far smaller than one frame's cap.
        let settled = ease_cast_edge(100.0, 99.9, alpha, 1.0, max_step);
        assert!(
            (99.9..=100.0).contains(&settled),
            "closing overshot its target: {settled}"
        );
    }
}

#[cfg(test)]
mod soft_framing_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::camera_ease::CameraEaseState;

    /// ⛔⛔ ONE PLACE DECIDES THE DEFAULT FRAMING.
    ///
    /// `CameraSnapshot2d::default()` used to hardcode `800x450` — correct while
    /// `Combat` was the shipped default, and silently wrong the moment the
    /// default moved to `Duel` (568x320) for Smash-like readability. Nothing
    /// reads a default snapshot in production today, so the disagreement would
    /// have waited here until something did.
    ///
    /// ⇒ This test fails if the two ever diverge again, which is the only thing
    /// that makes "derived" true rather than merely currently-equal.
    #[test]
    fn the_default_snapshot_frames_itself_from_the_shipped_preset() {
        let (w, h) = ambition_persistence::settings::video::CameraZoomPreset::default().base_view();
        let snapshot = CameraSnapshot2d::default();

        assert_eq!(snapshot.base_view, ae::Vec2::new(w, h));
        assert_eq!(snapshot.requested_view, ae::Vec2::new(w, h));
        assert_eq!(snapshot.visible_view, ae::Vec2::new(w, h));
    }

    const VIEW: ae::Vec2 = ae::Vec2::new(800.0, 450.0);
    const BODY: ae::Vec2 = ae::Vec2::new(24.0, 40.0);

    fn world() -> ae::World {
        ae::World::new(
            "framing",
            ae::Vec2::new(40_000.0, 40_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        )
    }

    /// A generous centered region with no padding and no look-ahead, so the
    /// admissible interval is easy to state by hand.
    fn framing(region: NormalizedScreenRegion) -> CameraScreenFraming {
        CameraScreenFraming {
            active: true,
            subject_safe_region: region,
            subject_padding_px: ae::Vec2::ZERO,
            look_ahead_seconds: 0.0,
        }
    }

    fn zone(cinematic_lock: bool) -> CameraZoneSpec {
        CameraZoneSpec {
            id: "lock".into(),
            name: "lock".into(),
            aabb: ae::Aabb::new(ae::Vec2::splat(20_000.0), ae::Vec2::splat(10_000.0)),
            priority: 0,
            zoom: Some(1.0),
            target_offset: ae::Vec2::ZERO,
            easing_hz: None,
            cinematic_lock,
            clamp_mode: CameraClampMode::None,
            scroll_policy: CameraScrollPolicy::Free,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve(
        world: &ae::World,
        zones: &[CameraZoneSpec],
        pos: ae::Vec2,
        vel: ae::Vec2,
        screen_framing: Option<CameraScreenFraming>,
        ease: &mut CameraEaseState,
    ) -> CameraSnapshot2d {
        resolve_follow_camera_snapshot(
            CameraSnapshotResolveInput {
                world,
                camera_zones: zones,
                focus: CameraFocus2d {
                    center_world: pos,
                    size: BODY,
                    base_size: BODY,
                    facing: if vel.x < 0.0 { -1.0 } else { 1.0 },
                    velocity_world: vel,
                },
                base_view: VIEW,
                viewport_px: VIEW,
                aspect_policy: CameraAspectPolicy::FixedHeight,
                framing: CameraFramingPreset::default(),
                overview_scale: 1.0,
                encounter_scale: 1.0,
                overview_camera: false,
                snap_camera: false,
                blink: None,
                dt: 1.0 / 60.0,
                mode: CameraSnapshotResolveMode::Eased,
                extra_clamp_center_world: None,
                chart_transit: None,
                must_frame_world: None,
                ease_tuning: CameraEaseTuning::default(),
                screen_framing,
                reference_frame: Default::default(),
                subject_down: None,
            },
            Some(ease),
        )
    }

    /// Seed the ease state so `target_initialized` is true and the camera has a
    /// definite "where it is now" for the deadzone to hold.
    fn seeded(at: ae::Vec2) -> CameraEaseState {
        CameraEaseState {
            target_initialized: true,
            live_target_world: at,
            ..Default::default()
        }
    }

    /// The deadzone: while the subject stays inside the region the camera
    /// target does not move at all. This is the whole point — a camera that
    /// still crept toward center would not be "soft", just slow.
    #[test]
    fn the_camera_holds_still_while_the_subject_stays_inside_the_region() {
        let w = world();
        let start = ae::Vec2::new(20_000.0, 20_000.0);
        let mut ease = seeded(start);
        let region = NormalizedScreenRegion::centered_inset(0.25, 0.25);

        // Half the region is 200px wide in world units here, so ±100 is well
        // inside it.
        for dx in [0.0, 40.0, -60.0, 90.0] {
            let snap = resolve(
                &w,
                &[],
                start + ae::Vec2::new(dx, 0.0),
                ae::Vec2::ZERO,
                Some(framing(region)),
                &mut ease,
            );
            assert!(
                (snap.target_world - start).length() < 0.001,
                "camera drifted to {:?} for dx={dx}",
                snap.target_world,
            );
        }
    }

    /// Crossing an edge moves the camera by exactly the correction needed to
    /// put the protected bounds back on that edge — no more.
    #[test]
    fn crossing_an_edge_applies_only_the_needed_correction() {
        let w = world();
        let start = ae::Vec2::new(20_000.0, 20_000.0);
        let region = NormalizedScreenRegion::centered_inset(0.25, 0.25);
        let mut ease = seeded(start);

        // Region right edge sits at 0.75 of an 800-wide view => 200 world units
        // right of centre. The body half-width is 12, so the camera must start
        // moving once the subject centre passes +188.
        let overshoot = 60.0;
        let subject = start + ae::Vec2::new(200.0 - BODY.x * 0.5 + overshoot, 0.0);

        // The deadzone sets the TARGET; the existing 8 Hz ease carries the
        // camera there, so settle before measuring the correction.
        let mut snap = resolve(
            &w,
            &[],
            subject,
            ae::Vec2::ZERO,
            Some(framing(region)),
            &mut ease,
        );
        for _ in 0..400 {
            snap = resolve(
                &w,
                &[],
                subject,
                ae::Vec2::ZERO,
                Some(framing(region)),
                &mut ease,
            );
        }

        assert!(
            (snap.target_world.x - (start.x + overshoot)).abs() < 0.5,
            "expected a {overshoot} correction, camera settled at {:?}",
            snap.target_world,
        );
        assert!(
            (snap.target_world.y - start.y).abs() < 0.5,
            "an x-axis crossing must not move y",
        );
    }

    /// Look-ahead extends the protected bounds along the velocity, so a fast
    /// runner pushes the camera earlier than a stationary one at the same spot.
    #[test]
    fn look_ahead_pushes_the_camera_earlier_when_moving_fast() {
        let w = world();
        let start = ae::Vec2::new(20_000.0, 20_000.0);
        let region = NormalizedScreenRegion::centered_inset(0.25, 0.25);
        let subject = start + ae::Vec2::new(100.0, 0.0);

        let settle = |velocity, screen_framing, ease: &mut CameraEaseState| {
            let mut snap = resolve(&w, &[], subject, velocity, Some(screen_framing), ease);
            for _ in 0..400 {
                snap = resolve(&w, &[], subject, velocity, Some(screen_framing), ease);
            }
            snap
        };

        let mut still_ease = seeded(start);
        let still = settle(ae::Vec2::ZERO, framing(region), &mut still_ease);

        let mut fast_ease = seeded(start);
        let fast = settle(
            ae::Vec2::new(1200.0, 0.0),
            CameraScreenFraming {
                look_ahead_seconds: 0.25,
                ..framing(region)
            },
            &mut fast_ease,
        );

        assert!(
            (still.target_world.x - start.x).abs() < 0.5,
            "a standing subject at +100 is still inside the region",
        );
        assert!(
            fast.target_world.x > still.target_world.x + 50.0,
            "look-ahead should lead the runner: {} vs {}",
            fast.target_world.x,
            still.target_world.x,
        );
    }

    /// A cinematic camera zone has taken authorship of the composition; a
    /// deadzone must not fight it.
    #[test]
    fn a_cinematic_lock_bypasses_soft_framing() {
        let w = world();
        let start = ae::Vec2::new(20_000.0, 20_000.0);
        let region = NormalizedScreenRegion::centered_inset(0.25, 0.25);
        let zones = [zone(true)];
        let mut ease = seeded(start);

        let snap = resolve(
            &w,
            &zones,
            start + ae::Vec2::new(3000.0, 0.0),
            ae::Vec2::ZERO,
            Some(framing(region)),
            &mut ease,
        );
        // The locked zone's centre wins outright.
        assert!(
            (snap.target_world - zones[0].aabb.center()).length() < 1.0,
            "cinematic lock lost to the deadzone: {:?}",
            snap.target_world,
        );
    }

    /// Protected bounds wider than the region on an axis cannot be satisfied;
    /// centering them beats snapping to an arbitrary edge.
    #[test]
    fn bounds_larger_than_the_region_center_instead_of_snapping() {
        let w = world();
        let start = ae::Vec2::new(20_000.0, 20_000.0);
        let subject = start + ae::Vec2::new(500.0, 0.0);
        // A one-percent-wide region no body can fit inside.
        let region = NormalizedScreenRegion::centered_inset(0.495, 0.495);
        let mut ease = seeded(start);

        let mut snap = resolve(
            &w,
            &[],
            subject,
            ae::Vec2::ZERO,
            Some(framing(region)),
            &mut ease,
        );
        for _ in 0..400 {
            snap = resolve(
                &w,
                &[],
                subject,
                ae::Vec2::ZERO,
                Some(framing(region)),
                &mut ease,
            );
        }
        assert!(
            (snap.target_world.x - subject.x).abs() < 1.0,
            "an unsatisfiable axis should centre on the subject, got {:?}",
            snap.target_world,
        );
    }

    /// Oracle 9's sim-side half: an inactive framing fact resolves BIT-IDENTICALLY
    /// to passing nothing, so a game that declares no profile — and every
    /// headless run and capture — is untouched by this feature existing.
    #[test]
    fn inactive_framing_is_identical_to_no_framing() {
        let w = world();
        let start = ae::Vec2::new(20_000.0, 20_000.0);

        for offset in [0.0, 250.0, -900.0] {
            let subject = start + ae::Vec2::new(offset, 30.0);
            let velocity = ae::Vec2::new(offset, 0.0);

            let mut none_ease = seeded(start);
            let none = resolve(&w, &[], subject, velocity, None, &mut none_ease);

            let mut off_ease = seeded(start);
            let off = resolve(
                &w,
                &[],
                subject,
                velocity,
                Some(CameraScreenFraming::default()),
                &mut off_ease,
            );

            assert_eq!(none, off, "inactive framing changed the snapshot");
            assert_eq!(none_ease.live_target_world, off_ease.live_target_world);
        }
    }
}

#[cfg(test)]
mod reference_frame_tests {
    use super::*;

    const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;

    /// A view that never states a policy must present exactly as it did before
    /// the policy existed, whatever its subject is doing.
    #[test]
    fn world_fixed_never_rolls() {
        for down in [
            ae::Vec2::new(0.0, 1.0),
            ae::Vec2::new(1.0, 0.0),
            ae::Vec2::new(0.0, -1.0),
            ae::Vec2::new(-0.7, 0.7),
        ] {
            assert_eq!(
                observer_roll_radians(CameraReferenceFrame::default(), Some(down)),
                0.0,
                "the default policy rolled for a subject down of {down:?}"
            );
        }
    }

    /// A subject-relative view puts the subject's feet at the bottom.
    ///
    /// Ordinary gravity must be the identity or every existing room would tilt the moment the mode
    /// is selected.
    #[test]
    fn subject_frame_orients_on_the_subjects_down_axis() {
        let roll = |x: f32, y: f32| {
            observer_roll_radians(
                CameraReferenceFrame::SubjectFrame,
                Some(ae::Vec2::new(x, y)),
            )
        };
        assert_eq!(roll(0.0, 1.0), 0.0, "ordinary gravity must not tilt a view");
        assert!(
            (roll(0.0, -1.0).abs() - std::f32::consts::PI).abs() < 1e-5,
            "inverted gravity turns the view over, got {}",
            roll(0.0, -1.0)
        );
        assert!(
            (roll(1.0, 0.0) - HALF_PI).abs() < 1e-5,
            "gravity toward +x rolls one quarter turn, got {}",
            roll(1.0, 0.0)
        );
        assert!(
            (roll(-1.0, 0.0) + HALF_PI).abs() < 1e-5,
            "gravity toward -x rolls the OTHER quarter turn, got {}",
            roll(-1.0, 0.0)
        );
        // Not only the cardinals: the frame model permits any orientation, and a
        // policy that quantised to four would be a different feature.
        assert!(
            (roll(1.0, 1.0) - HALF_PI / 2.0).abs() < 1e-5,
            "a diagonal frame rolls by its own angle, got {}",
            roll(1.0, 1.0)
        );
        // Magnitude is irrelevant — this is a direction.
        assert!((roll(0.0, 42.0)).abs() < 1e-6);
        assert!((roll(9.0, 0.0) - HALF_PI).abs() < 1e-5);
    }

    /// No subject is not an error. A view framing a cast, or nothing, reads
    /// as world-fixed rather than as a missing value to unwrap.
    #[test]
    fn a_subject_frame_view_without_a_subject_stays_upright() {
        assert_eq!(
            observer_roll_radians(CameraReferenceFrame::SubjectFrame, None),
            0.0
        );
        assert_eq!(
            observer_roll_radians(CameraReferenceFrame::SubjectFrame, Some(ae::Vec2::ZERO)),
            0.0,
            "a degenerate down axis must not produce a NaN roll"
        );
    }

    ///  this is the behaviour that shipped, and it must be preserved to the
    /// bit: the renderer overwrote `rotation_radians` with the portal roll, and
    /// under `WorldFixed` the composed rule has to agree exactly — the base is
    /// identically zero, so the composition is the identity on the chart roll.
    #[test]
    fn a_world_fixed_view_presents_exactly_the_chart_rotation() {
        for chart in [0.0, HALF_PI, -HALF_PI, std::f32::consts::PI, 0.37] {
            assert_eq!(
                presented_roll_radians(
                    CameraReferenceFrame::WorldFixed,
                    Some(ae::Vec2::new(0.0, 1.0)),
                    Some(CameraChartTransit {
                        chart_roll_radians: chart,
                        observer_roll_at_entry: 0.0,
                    }),
                ),
                chart,
                "a world-fixed view must present the portal's roll unchanged"
            );
        }
        // And gives it back when the crossing ends.
        assert_eq!(
            presented_roll_radians(
                CameraReferenceFrame::WorldFixed,
                Some(ae::Vec2::new(0.0, 1.0)),
                None
            ),
            0.0
        );
    }

    /// A subject-frame view adds the chart rotation when the portal did not
    /// move its frame.
    ///
    /// Body somersaults, gravity is the same on both sides: the base roll never
    /// changes, so the crossing needs the map's rotation on top of it to keep
    /// the image continuous, and returns to base afterwards.
    #[test]
    fn a_crossing_that_leaves_the_frame_alone_adds_its_rotation() {
        let inverted = ae::Vec2::new(0.0, -1.0);
        let base = observer_roll_radians(CameraReferenceFrame::SubjectFrame, Some(inverted));
        let presented = presented_roll_radians(
            CameraReferenceFrame::SubjectFrame,
            Some(inverted),
            Some(CameraChartTransit {
                chart_roll_radians: HALF_PI,
                // The frame did not move, so the roll adopted at entry is still
                // the live base.
                observer_roll_at_entry: base,
            }),
        );
        assert!(
            (presented - (base + HALF_PI)).abs() < 1e-5,
            "expected {} + {HALF_PI}, got {presented}",
            base
        );
    }

    ///  A CROSSING THAT ROTATES THE BODY'S OWN FRAME MUST NOT COUNT TWICE.
    ///
    /// This is the case the naive rule gets wrong, and it is the reason the
    /// composition is not an addition of two independently-derived angles. A
    /// floor→wall portal rotates the map by a quarter turn AND leaves the body
    /// standing on what is now its floor — so a subject-frame view's base roll
    /// has already moved by that same quarter turn the instant the body crossed.
    /// Adding the chart rotation to the LIVE base would spin the world a full
    /// half turn through the seam and then spin it back.
    ///
    /// The rule composes against the roll adopted at ENTRY, so the presented
    /// roll during the crossing is exactly the destination's base roll: the view
    /// arrives where it was always going to arrive, with no overshoot.
    #[test]
    fn a_crossing_that_rotates_the_frame_does_not_double_count() {
        let before = ae::Vec2::new(0.0, 1.0);
        let after = ae::Vec2::new(1.0, 0.0);
        let base_before = observer_roll_radians(CameraReferenceFrame::SubjectFrame, Some(before));
        let base_after = observer_roll_radians(CameraReferenceFrame::SubjectFrame, Some(after));
        // The portal's map rotation IS the frame change here.
        let chart = base_after - base_before;
        let presented = presented_roll_radians(
            CameraReferenceFrame::SubjectFrame,
            // The body has crossed: its live down is already the destination's.
            Some(after),
            Some(CameraChartTransit {
                chart_roll_radians: chart,
                observer_roll_at_entry: base_before,
            }),
        );
        assert!(
            (presented - base_after).abs() < 1e-5,
            "a frame-rotating crossing must present the destination's own roll \
             ({base_after}), not it plus the map again; got {presented}"
        );
        // The poison: the rule this replaces.
        let naive = base_after + chart;
        assert!(
            (naive - presented).abs() > 1e-3,
            "this test proves nothing unless the naive rule actually differs"
        );
    }

    /// An upright view's clamp footprint is its viewport, exactly.
    ///
    /// The rotation-aware clamp must be the identity for every view that is not
    /// rolled, which is every view outside a transit. If this drifts, every room
    /// in the game re-frames.
    #[test]
    fn an_upright_view_clamps_by_its_own_extents() {
        for (w, h) in [(400.0, 225.0), (225.0, 400.0), (1.0, 1.0), (960.0, 540.0)] {
            assert_eq!(rolled_view_half_extents(w, h, 0.0), (w, h));
        }
    }

    /// `portal_transit_roll` returns ±π/2 for a floor↔wall pair, so this is not
    /// a future mode's problem: it is what a portal does today. A 400×225 view
    /// rolled a quarter turn is 225 wide and 400 tall in the world, and clamping
    /// it as 400×225 lets it show past the room's floor and ceiling.
    #[test]
    fn a_quarter_turn_swaps_the_footprint() {
        let (w, h) = (400.0f32, 225.0f32);
        for quarter in [HALF_PI, -HALF_PI] {
            let (rw, rh) = rolled_view_half_extents(w, h, quarter);
            assert!(
                (rw - h).abs() < 1e-3 && (rh - w).abs() < 1e-3,
                "a quarter turn must swap the footprint, got ({rw}, {rh})"
            );
        }
        // Half a turn is upright again.
        let (rw, rh) = rolled_view_half_extents(w, h, std::f32::consts::PI);
        assert!((rw - w).abs() < 1e-3 && (rh - h).abs() < 1e-3);
    }

    /// A diagonal roll grows BOTH extents, and rolling either way is the same
    /// footprint.
    ///
    /// The second half is the sign claim in the helper's own docs: the roll is a
    /// render-space angle and the clamp is world-space, a y flip apart, and this
    /// is why that does not matter here.
    #[test]
    fn a_diagonal_roll_grows_both_extents_symmetrically() {
        let (w, h) = (400.0f32, 225.0f32);
        let eighth = HALF_PI / 2.0;
        let (rw, rh) = rolled_view_half_extents(w, h, eighth);
        let expected = (w + h) / 2.0f32.sqrt();
        assert!((rw - expected).abs() < 1e-2, "got {rw}, want {expected}");
        assert!((rh - expected).abs() < 1e-2, "got {rh}, want {expected}");
        assert!(rw > w && rh > h, "a rolled view cannot occupy less");
        assert_eq!(
            rolled_view_half_extents(w, h, eighth),
            rolled_view_half_extents(w, h, -eighth),
            "the footprint must not depend on which way the view rolled"
        );
    }
}

#[cfg(test)]
mod observer_roll_continuity_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::camera_ease::{
        ease_roll_radians, CameraEaseState, OBSERVER_ROLL_EASE_RAD_PER_S,
    };

    fn world() -> ae::World {
        ae::World::new(
            "roll",
            ae::Vec2::new(4000.0, 2000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        )
    }

    /// One resolve at `down`, through the real snapshot path.
    fn resolve(
        w: &ae::World,
        ease: &mut CameraEaseState,
        down: ae::Vec2,
        transit: Option<CameraChartTransit>,
    ) -> f32 {
        let snap = resolve_follow_camera_snapshot(
            CameraSnapshotResolveInput {
                world: w,
                camera_zones: &[],
                focus: CameraFocus2d {
                    center_world: ae::Vec2::new(2000.0, 1000.0),
                    size: ae::Vec2::new(24.0, 40.0),
                    base_size: ae::Vec2::new(24.0, 40.0),
                    facing: 1.0,
                    velocity_world: ae::Vec2::ZERO,
                },
                base_view: ae::Vec2::new(800.0, 450.0),
                viewport_px: ae::Vec2::new(1600.0, 900.0),
                aspect_policy: Default::default(),
                framing: Default::default(),
                overview_scale: 1.0,
                encounter_scale: 1.0,
                overview_camera: false,
                snap_camera: false,
                blink: None,
                dt: 1.0 / 60.0,
                mode: CameraSnapshotResolveMode::Eased,
                extra_clamp_center_world: None,
                chart_transit: transit,
                must_frame_world: None,
                ease_tuning: Default::default(),
                screen_framing: None,
                reference_frame: CameraReferenceFrame::SubjectFrame,
                subject_down: Some(down),
            },
            Some(ease),
        );
        snap.rotation_radians
    }

    const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);
    const UP: ae::Vec2 = ae::Vec2::new(0.0, -1.0);

    /// A GRAVITY FLIP MUST NOT CUT.
    ///
    /// `presented_roll_radians` has no history, so before this the world rotated a half turn in
    /// ONE frame the instant the subject's down axis flipped — which is also what possessing a
    /// body on a different surface did.
    #[test]
    fn a_gravity_flip_turns_the_world_instead_of_cutting_it() {
        let w = world();
        let mut ease = CameraEaseState::default();

        // The view opens ADOPTED, not spun up from zero.
        let settled = resolve(&w, &mut ease, DOWN, None);
        assert_eq!(
            settled,
            observer_roll_radians(CameraReferenceFrame::SubjectFrame, Some(DOWN)),
            "a view must open already oriented"
        );

        // Flip. One frame may only move by one frame's worth.
        let after_one = resolve(&w, &mut ease, UP, None);
        let per_frame = OBSERVER_ROLL_EASE_RAD_PER_S / 60.0;
        assert!(
            (after_one - settled).abs() <= per_frame + 1.0e-4,
            "the world turned {:.3} rad in a single frame — that is the snap this \
             exists to remove (budget {per_frame:.3})",
            (after_one - settled).abs(),
        );

        //  the non-vacuity half: it must actually ARRIVE.
        let target = observer_roll_radians(CameraReferenceFrame::SubjectFrame, Some(UP));
        let mut last = after_one;
        for _ in 0..120 {
            last = resolve(&w, &mut ease, UP, None);
        }
        assert!(
            (last - target).abs() < 1.0e-3,
            "the roll never reached its target: {last} vs {target}"
        );
    }

    /// THE SEAM IS EXEMPT, and that is not an oversight.
    ///
    /// A portal transit's discontinuity is the point — the view presents the
    /// chart's rotation immediately so both sides line up across the seam — so it
    /// must ADOPT rather than ease. Smoothing through it reintroduces exactly the
    /// overshoot C4's composition rule was derived to avoid.
    #[test]
    fn a_chart_transit_is_adopted_whole_rather_than_eased_through() {
        let w = world();
        let mut ease = CameraEaseState::default();
        resolve(&w, &mut ease, DOWN, None);

        let transit = CameraChartTransit {
            observer_roll_at_entry: observer_roll_radians(
                CameraReferenceFrame::SubjectFrame,
                Some(DOWN),
            ),
            chart_roll_radians: std::f32::consts::FRAC_PI_2,
        };
        let presented = resolve(&w, &mut ease, DOWN, Some(transit));
        assert_eq!(
            presented,
            transit.observer_roll_at_entry + transit.chart_roll_radians,
            "the seam must be presented whole, on the frame it is asked for"
        );
        assert_eq!(
            ease.live_observer_roll,
            Some(presented),
            "and the adopted roll must be REMEMBERED, or leaving the transit \
             snaps back to where the view was before the seam"
        );
    }

    /// ±π are the same orientation, so a roll must never take the long way
    /// round to reach an angle it is already at.
    #[test]
    fn easing_takes_the_shortest_way_around_the_circle() {
        use std::f32::consts::PI;
        let dt = 1.0 / 60.0;
        let step = OBSERVER_ROLL_EASE_RAD_PER_S * dt;

        // Just below +π easing to just above -π: 0.02 radians of turn, not a
        // full rotation backwards.
        //
        // Assert angular distance rather than numeric ordering across the ±π wrap.
        let current = PI - 0.01;
        let target = -PI + 0.01;
        let next = ease_roll_radians(current, target, dt);
        let travelled = {
            let mut d = (next - current) % (2.0 * PI);
            if d > PI {
                d -= 2.0 * PI;
            } else if d < -PI {
                d += 2.0 * PI;
            }
            d
        };
        assert!(
            travelled > 0.0,
            "wrapped the wrong way: {current} -> {next} (target {target})"
        );
        assert!(
            travelled.abs() <= step + 1.0e-5,
            "turned {travelled} rad, more than one frame's budget {step}"
        );

        // A change smaller than one frame's step lands exactly, without
        // oscillating past it.
        assert_eq!(ease_roll_radians(0.0, 0.001, dt), 0.001);
    }
}

#[cfg(test)]
mod rolled_safe_area_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::gameplay_presentation::NormalizedScreenRegion;

    /// A region that pins the subject into the TOP band of the SCREEN, so the
    /// camera must move screen-DOWNWARD relative to it. Asymmetric on purpose: a
    /// centred region cannot tell +90° from -90°.
    fn top_band() -> CameraScreenFraming {
        CameraScreenFraming {
            active: true,
            subject_safe_region: NormalizedScreenRegion {
                min: ae::Vec2::new(0.0, 0.0),
                max: ae::Vec2::new(1.0, 0.25),
            },
            subject_padding_px: ae::Vec2::ZERO,
            look_ahead_seconds: 0.0,
        }
    }

    fn framed(roll: f32) -> ae::Vec2 {
        let focus = CameraFocus2d {
            center_world: ae::Vec2::ZERO,
            size: ae::Vec2::new(20.0, 20.0),
            base_size: ae::Vec2::new(20.0, 20.0),
            facing: 1.0,
            velocity_world: ae::Vec2::ZERO,
        };
        apply_soft_subject_framing(
            ae::Vec2::ZERO,
            ae::Vec2::ZERO,
            focus,
            ae::Vec2::new(800.0, 400.0),
            1.0,
            top_band(),
            roll,
        )
    }

    /// "THE LOWER THIRD" MEANS THE SCREEN'S, NOT THE WORLD'S.
    ///
    /// The safe region is a normalized SCREEN rectangle and was applied in world
    /// axes. Upright the two frames coincide and nothing could tell; rolled — a
    /// gravity flip in `SubjectFrame`, a portal seam — the deadzone protected the
    /// wrong screen edge, and a quarter turn swapped which axis it constrained.
    ///
    ///  the two quarter turns must DISAGREE, which is what makes this a
    /// sign test rather than a "rotation happens somewhere" test. Conjugating a
    /// rotation by the world's y-down/render's y-up reflection negates its angle,
    /// so getting that wrong lands the camera on the opposite side and every
    /// symmetric assertion still passes.
    #[test]
    fn the_safe_region_follows_the_screen_when_the_view_rolls() {
        use std::f32::consts::FRAC_PI_2;

        // Upright: screen-down is world +y, so the camera sits BELOW the subject
        // to push it up the screen.
        let upright = framed(0.0);
        assert!(
            upright.x.abs() < 1.0e-3 && upright.y > 1.0,
            "upright framing should push the camera along world +y, got {upright:?}"
        );
        let distance = upright.y;

        // A quarter turn one way sends the same screen-down along world +x...
        let cw = framed(FRAC_PI_2);
        assert!(
            (cw.x - distance).abs() < 1.0e-2 && cw.y.abs() < 1.0e-2,
            "at +90° the same screen constraint should act along world +x by \
             {distance}, got {cw:?}"
        );

        // ...and the other way along world -x. If these two agreed, the rotation
        // would be signless and the fix would be half-done.
        let ccw = framed(-FRAC_PI_2);
        assert!(
            (ccw.x + distance).abs() < 1.0e-2 && ccw.y.abs() < 1.0e-2,
            "at -90° it should act along world -x by {distance}, got {ccw:?}"
        );
    }

    ///  zero roll is byte-identical to the behaviour that shipped, which is
    /// what makes this safe for every upright view in the game — and what the 68
    /// existing resolve tests are implicitly asserting.
    #[test]
    fn an_upright_view_is_unchanged_by_the_rotation_path() {
        let focus = CameraFocus2d {
            center_world: ae::Vec2::new(100.0, 50.0),
            size: ae::Vec2::new(24.0, 40.0),
            base_size: ae::Vec2::new(24.0, 40.0),
            facing: 1.0,
            velocity_world: ae::Vec2::new(120.0, -30.0),
        };
        let framing = CameraScreenFraming {
            active: true,
            subject_safe_region: NormalizedScreenRegion {
                min: ae::Vec2::new(0.3, 0.55),
                max: ae::Vec2::new(0.7, 0.9),
            },
            subject_padding_px: ae::Vec2::new(8.0, 12.0),
            look_ahead_seconds: 0.4,
        };
        let desired = ae::Vec2::new(140.0, 20.0);
        let previous = ae::Vec2::new(90.0, 44.0);
        let visible = ae::Vec2::new(640.0, 360.0);

        let rolled_path =
            apply_soft_subject_framing(desired, previous, focus, visible, 1.5, framing, 0.0);

        // The pre-rotation arithmetic, inline, as the shipped version computed it.
        let anchor = focus.stable_center();
        let bias = desired - anchor;
        let half = focus.size.max(focus.base_size) * 0.5 + framing.subject_padding_px.abs() * 1.5;
        let lead = focus.velocity_world * framing.look_ahead_seconds;
        let swept_min = anchor.min(anchor + lead) - half;
        let swept_max = anchor.max(anchor + lead) + half;
        let region = framing.subject_safe_region;
        let low = swept_max + bias - visible * (region.max - ae::Vec2::splat(0.5));
        let high = swept_min + bias - visible * (region.min - ae::Vec2::splat(0.5));
        let expected = ae::Vec2::new(
            previous.x.clamp(low.x, high.x),
            previous.y.clamp(low.y, high.y),
        );

        assert!(
            (rolled_path - expected).length() < 1.0e-3,
            "zero roll changed an upright framing: {rolled_path:?} vs {expected:?}"
        );
    }
}
