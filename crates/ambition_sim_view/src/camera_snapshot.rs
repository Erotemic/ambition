//! Pure 2D camera-follow snapshot policy.
//!
//! This module is the non-rendering half of the camera system: given a room,
//! a focus point/body, and camera policy inputs, it resolves the camera that
//! should view that focus. The visible Bevy camera, future portal captures, and
//! no-GPU/headless PNG tools can all consume the same [`CameraSnapshot2d`]
//! without depending on each other.

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use bevy_math::UVec2;

use ambition_actors::rooms::{
    apply_forward_only_x, CameraClampMode, CameraScrollPolicy, CameraZoneSpec,
};
use ambition_persistence::settings::video::CameraFramingPreset;
use ambition_persistence::settings::CameraAspectPolicy;
use ambition_platformer_primitives::camera_ease::{CameraEaseState, CameraEaseTuning};
use ambition_platformer_primitives::gameplay_presentation::NormalizedScreenRegion;
use ambition_platformer_primitives::schedule::SimScheduleExt;

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

impl Default for CameraSnapshot2d {
    fn default() -> Self {
        Self {
            base_view: ae::Vec2::new(800.0, 450.0),
            requested_view: ae::Vec2::new(800.0, 450.0),
            visible_view: ae::Vec2::new(800.0, 450.0),
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
    /// Standing/baseline body size. Used to keep the camera from popping when a
    /// stance temporarily changes body height.
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
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq)]
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

    // **Soft subject framing.** A deadzone, not a second follow policy: while
    // the subject's protected bounds stay inside the safe region the camera
    // target does not move at all, and when they cross an edge only the
    // correction needed to return them is applied. Runs BEFORE easing (so the
    // ordinary smoothing carries the correction) and before clamping (so room
    // bounds remain the authoritative fallback).
    //
    // Bypassed while a camera zone has taken authorship (cinematic lock),
    // during blink arrival, and on any snap — a deadzone must never fight a
    // deliberate composition.
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
    let (normal_host_x, normal_host_y) = clamp_camera_target(
        input.world,
        target,
        half_view_w,
        half_view_h,
        bounds,
        active_zone,
        None,
    );
    let (host_x, host_y) = if let Some(padding_center) = input.extra_clamp_center_world {
        clamp_camera_target(
            input.world,
            target,
            half_view_w,
            half_view_h,
            bounds,
            active_zone,
            Some(padding_center),
        )
    } else {
        (normal_host_x, normal_host_y)
    };

    // **M2 — the one-way forward scroll.** Applied AFTER the bounds clamp, because
    // the watermark must record where the camera actually settled, not where it
    // wanted to be. `host_x` is centered-render x, which is monotone in world x.
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
        rotation_radians: 0.0,
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
/// `desired` contributes only its BIAS — everything the ordinary policy wanted
/// beyond centering (framing preset look-ahead, camera-zone offsets) — which
/// translates the admissible interval instead of overriding the deadzone.
fn apply_soft_subject_framing(
    desired: ae::Vec2,
    previous: ae::Vec2,
    focus: CameraFocus2d,
    visible_view: ae::Vec2,
    orthographic_scale: f32,
    framing: CameraScreenFraming,
) -> ae::Vec2 {
    let visible = visible_view.max(ae::Vec2::splat(f32::EPSILON));
    let anchor = focus.stable_center();
    let bias = desired - anchor;

    // Protected bounds: the standing body box (so a crouch does not shrink the
    // protection), padding in viewport pixels converted to world units, and the
    // look-ahead sweep.
    let half = focus.size.max(focus.base_size) * 0.5
        + framing.subject_padding_px.abs() * orthographic_scale.max(0.0);
    let lead = focus.velocity_world * framing.look_ahead_seconds.max(0.0);
    let swept_min = anchor.min(anchor + lead) - half;
    let swept_max = anchor.max(anchor + lead) + half;

    let region = framing.subject_safe_region;
    let low = swept_max + bias - visible * (region.max - ae::Vec2::splat(0.5));
    let high = swept_min + bias - visible * (region.min - ae::Vec2::splat(0.5));

    // Protected bounds wider than the region on an axis: no camera center can
    // satisfy it, so center the bounds in the region rather than snapping to an
    // arbitrary edge.
    let centered =
        (swept_min + swept_max) * 0.5 + bias - visible * (region.center() - ae::Vec2::splat(0.5));

    ae::Vec2::new(
        if low.x <= high.x {
            previous.x.clamp(low.x, high.x)
        } else {
            centered.x
        },
        if low.y <= high.y {
            previous.y.clamp(low.y, high.y)
        } else {
            centered.y
        },
    )
}

fn zone_area(zone: &CameraZoneSpec) -> f32 {
    let half = zone.aabb.half_size();
    (half.x * 2.0).max(0.0) * (half.y * 2.0).max(0.0)
}

fn world_to_centered_render(world: &ae::World, p: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new(p.x - world.size.x * 0.5, world.size.y * 0.5 - p.y)
}

fn clamp_camera_target(
    world: &ae::World,
    target: ae::Vec2,
    half_view_w: f32,
    half_view_h: f32,
    mode: CameraClampMode,
    zone: Option<&CameraZoneSpec>,
    extra_clamp_center_world: Option<ae::Vec2>,
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
            (
                clamp_or_center(target.x, min_x, max_x),
                clamp_or_center(target.y, min_y, max_y),
            )
        }
        CameraClampMode::RoomBounds => clamp_to_world_bounds(
            world,
            target,
            half_view_w,
            half_view_h,
            extra_clamp_center_world,
        ),
    }
}

fn clamp_to_world_bounds(
    world: &ae::World,
    target: ae::Vec2,
    half_view_w: f32,
    half_view_h: f32,
    extra_clamp_center_world: Option<ae::Vec2>,
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
    (
        clamp_or_center(target.x, min_x, max_x),
        clamp_or_center(target.y, min_y, max_y),
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

// ---------------------------------------------------------------------------
// The camera OBSERVATION seam (E4 slice 17 — the render→sim write inverted).
//
// The follow-camera resolve (which integrates `CameraEaseState`) used to run
// INSIDE the render crate's `camera_follow`, making presentation the writer
// of sim-side ease state. It is now a sim-scheduled system here — the
// AJ13 "camera is an observer" boundary made structural: the sim publishes
// ONE resolved snapshot per tick; presentation consumes it (portal
// continuity applies its deltas to a COPY, never to this state). This whole
// block relocates into [the observation boundary] (`ambition_sim_view`)
// with the E4 carve.
// ---------------------------------------------------------------------------

/// The observer's physical viewport in pixels — an OBSERVER FACT the
/// windowed host publishes each frame (`publish_camera_viewport` in the
/// render layer). Headless runs keep the default design-window size, so the
/// resolver (and any RL reader of [`ResolvedCameraSnapshot`]) works without
/// a window. Consumed ONLY by the observation resolve below — sim systems
/// never read it.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug)]
pub struct CameraViewport {
    /// Physical viewport size, pixels (world-frame-free — a screen fact).
    pub px: ae::Vec2,
}

impl Default for CameraViewport {
    fn default() -> Self {
        Self {
            px: ae::Vec2::new(ae::config::WINDOW_W as f32, ae::config::WINDOW_H as f32),
        }
    }
}

/// Optional extra clamp target for the resolve (world-frame center) — the
/// generic seam a presentation adapter (portal camera continuity today) may
/// write when it needs the clamp bounds padded toward a point. `None` every
/// frame it isn't actively needed (the writer owns clearing it).
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default)]
pub struct CameraExtraClamp(pub Option<ae::Vec2>);

/// THE published observation: the follow-camera snapshot resolved once per
/// sim tick, plus the raw follow point it framed. Presentation reads this
/// (applying shake/portal deltas to a copy); RL/headless readers may read it
/// too — it is plain data.
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct ResolvedCameraSnapshot {
    pub snapshot: CameraSnapshot2d,
    /// World-frame position of the followed body (the controlled subject)
    /// this tick — the un-eased follow point presentation adapters (portal
    /// continuity) key their offsets from.
    pub follow_world: ae::Vec2,
}

/// Resolve the follow camera for this tick (the ONE writer of
/// [`CameraEaseState`]). A TAIL OBSERVER: runs after the whole
/// `CoreSimulation` chain (like `Trace`) so it sees final body positions AND
/// any post-sim presentation adapters (portal camera continuity) have had
/// their same-frame say through the observer-input resources. Presentation
/// consumers order `.after(resolve_camera_observation)`.
#[allow(clippy::too_many_arguments)]
pub fn resolve_camera_observation(
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_actors::rooms::RoomSet,
    >,
    time: bevy::prelude::Res<bevy::prelude::Time>,
    developer_tools: bevy::prelude::Res<ambition_dev_tools::dev_tools::DeveloperTools>,
    encounter_view: bevy::prelude::Res<ambition_encounter::EncounterView>,
    user_settings: bevy::prelude::Res<ambition_persistence::settings::UserSettings>,
    viewport: bevy::prelude::Res<CameraViewport>,
    screen_framing: bevy::prelude::Res<CameraScreenFraming>,
    extra_clamp: bevy::prelude::Res<CameraExtraClamp>,
    ease_tuning: bevy::prelude::Res<ambition_platformer_primitives::camera_ease::CameraEaseTuning>,
    mut camera_state: bevy::prelude::ResMut<
        ambition_platformer_primitives::camera_ease::CameraEaseState,
    >,
    mut resolved: bevy::prelude::ResMut<ResolvedCameraSnapshot>,
    mut last_camera_room: bevy::prelude::Local<Option<String>>,
    player: bevy::prelude::Query<
        (
            &ambition_platformer_primitives::body::BodyKinematics,
            &ae::BodyBaseSize,
            &ambition_actors::avatar::PlayerBlinkCameraState,
        ),
        ambition_platformer_primitives::markers::PrimaryPlayerOnly,
    >,
    controlled: bevy::prelude::Res<ambition_platformer_primitives::markers::ControlledSubject>,
    body_kinematics: bevy::prelude::Query<&ambition_platformer_primitives::body::BodyKinematics>,
) {
    // Dev tools can temporarily replace the authored/default camera view.
    let (base_view_w, base_view_h) = if developer_tools.camera_view_override_enabled {
        (
            developer_tools.camera_view_w.max(64.0),
            developer_tools.camera_view_h.max(64.0),
        )
    } else {
        user_settings.video.camera_zoom.base_view()
    };
    let base_view = ae::Vec2::new(base_view_w, base_view_h);
    let overview_scale = developer_tools.overview_camera_scale.max(1.0);
    let encounter_scale = encounter_view.camera_zoom.max(1.0);

    let Ok((mut player_body, player_base_size, blink_cam)) =
        player.single().map(|(b, bs, bc)| (*b, *bs, *bc))
    else {
        return;
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
        }
    }

    let active_spec = room_set.active_spec();
    let room_changed = last_camera_room.as_deref() != Some(active_spec.id.as_str());
    if room_changed {
        *last_camera_room = Some(active_spec.id.clone());
        // Disjoint LDtk areas: reset target easing so it never interpolates
        // through unrelated world coordinates.
        camera_state.target_initialized = false;
    }
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
            blink: Some(blink),
            dt: time.delta_secs(),
            mode: CameraSnapshotResolveMode::Eased,
            extra_clamp_center_world: extra_clamp.0,
            ease_tuning: *ease_tuning,
            screen_framing: Some(*screen_framing),
        },
        Some(&mut *camera_state),
    );
    *resolved = ResolvedCameraSnapshot {
        snapshot,
        follow_world: player_body.pos,
    };
}

/// The observation seam's plugin: owns the observer-input resources + the
/// published snapshot, and schedules the ONE resolve per tick. Part of
/// [`PlatformerEnginePlugins`] — headless apps get a live snapshot too.
pub struct CameraObservationPlugin;

impl bevy::prelude::Plugin for CameraObservationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        use bevy::prelude::IntoScheduleConfigs as _;
        app.init_resource::<CameraViewport>();
        app.init_resource::<CameraScreenFraming>();
        app.init_resource::<CameraExtraClamp>();
        app.init_resource::<ResolvedCameraSnapshot>();
        app.add_systems(
            sim,
            resolve_camera_observation
                .after(ambition_platformer_primitives::schedule::SandboxSet::CoreSimulation),
        );
    }
}

#[cfg(test)]
mod m2_forward_scroll_tests {
    use super::*;
    use ambition_platformer_primitives::camera_ease::CameraEaseState;

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
                ease_tuning: CameraEaseTuning::default(),
                screen_framing: None,
            },
            Some(ease),
        );
        snap.center_world.x
    }

    /// **The wiring, not just the clamp.** A player who runs right and then walks
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

    /// A `Free` zone — every zone authored before M2 — follows the player both ways,
    /// and clears any watermark it inherited from a forward-only zone it just left.
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
mod soft_framing_tests {
    use super::*;
    use ambition_platformer_primitives::camera_ease::CameraEaseState;

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
                ease_tuning: CameraEaseTuning::default(),
                screen_framing,
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
        let mut snap = resolve(&w, &[], subject, ae::Vec2::ZERO, Some(framing(region)), &mut ease);
        for _ in 0..400 {
            snap = resolve(&w, &[], subject, ae::Vec2::ZERO, Some(framing(region)), &mut ease);
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

        let mut snap = resolve(&w, &[], subject, ae::Vec2::ZERO, Some(framing(region)), &mut ease);
        for _ in 0..400 {
            snap = resolve(&w, &[], subject, ae::Vec2::ZERO, Some(framing(region)), &mut ease);
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
