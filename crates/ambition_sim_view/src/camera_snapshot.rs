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
use ambition_platformer2d_actor_monolith::rooms::{
    apply_forward_only_x, CameraClampMode, CameraScrollPolicy, CameraZoneSpec,
};
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

/// **Which frame a view presents the world in.**
///
/// A presentation policy and nothing else: gravity, collision and body
/// integration are the same simulation facts whichever frame observes them. It
/// belongs to a VIEW, so when views become indexed this moves with them rather
/// than becoming a process-global mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraReferenceFrame {
    /// Screen orientation stays tied to the world frame even when the subject
    /// enters sideways or inverted gravity. Ordinary platformer readability, and
    /// the only behaviour that existed before this policy.
    #[default]
    WorldFixed,
    /// Screen orientation follows the view subject's resolved body frame, so a
    /// gravity change presents as the world rotating around an upright body.
    ///
    /// ⭐ **the subject is a view's subject, not a protagonist.** The resolver
    /// takes a direction, never an entity, so a spectator, a replay or a second
    /// local view can orient on whatever body it is watching.
    SubjectFrame,
}

/// The camera roll a view wants, given its frame policy and — for
/// [`CameraReferenceFrame::SubjectFrame`] — the subject's resolved down axis.
///
/// ⭐ **derived in RENDER space, which is world space with y flipped.** That is
/// the convention `portal_transit_roll` already establishes for the only other
/// producer of this value, and measuring the turn directly there is what keeps
/// the sign unambiguous. Screen-down is render `(0, -1)`; a world down of
/// `(dx, dy)` is render `(dx, -dy)`; the signed angle from screen-down to it is
/// `atan2(dx, dy)`. Rotating the CAMERA by that angle presents the world rotated
/// the other way, which puts the subject's feet at the bottom of the screen.
///
/// ⚠ **this is the BASE roll only** — what the view presents when nothing is
/// mapping the world through a chart rotation. [`presented_roll_radians`]
/// composes it with a transit.
pub fn observer_roll_radians(frame: CameraReferenceFrame, subject_down: Option<ae::Vec2>) -> f32 {
    match frame {
        CameraReferenceFrame::WorldFixed => 0.0,
        // No subject to orient on is not an error — a view may be framing a cast
        // or nothing at all. It reads as world-fixed, the readable default.
        CameraReferenceFrame::SubjectFrame => match subject_down {
            Some(down) if down.length_squared() > f32::EPSILON => {
                let down = down.normalize();
                down.x.atan2(down.y)
            }
            _ => 0.0,
        },
    }
}

/// **A chart rotation the view is presenting through, and the roll it had
/// adopted when that began.**
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
    /// **The roll the view had already adopted when the crossing began**, which
    /// is what stops the composition double-counting. See
    /// [`presented_roll_radians`].
    pub observer_roll_at_entry: f32,
}

/// **The roll a view actually presents**: its own frame, plus whatever part of
/// the chart rotation its frame has not already absorbed.
///
/// ⭐⭐ **the two rolls are NOT independent angles to add, and deriving that is
/// the whole of this function.** Take a floor↔wall portal, whose map rotation
/// `M` is ±π/2:
///
/// - **the destination's gravity matches the source's.** The body somersaults
///   but its down axis is unchanged, so the base roll is unchanged. The view
///   needs `M` on top of its base to keep the image continuous, and gives it
///   back when the crossing ends. `base + M`.
/// - **the portal also changes the body's frame** (it lands on a wall that is
///   now its floor). The subject's down rotates by `M` too, so a
///   [`CameraReferenceFrame::SubjectFrame`] view's base roll ALREADY moved by
///   `M` the instant the body crossed. Adding `M` again would spin the world a
///   full extra half-turn through the seam and then spin it back.
///
/// Both cases are one rule: **the view presents the roll it had adopted at
/// entry, turned by the chart rotation.** In the first case the adopted roll is
/// still the live base, so this is `base + M`; in the second the adopted roll is
/// the PRE-crossing base, so it is `base_before + M` — which is exactly the
/// post-crossing base, reached with no overshoot.
///
/// ⭐ and `WorldFixed` is the degenerate case rather than a separate path: its
/// base is identically 0, so this is `M`, decaying to 0 when the crossing ends —
/// byte-identical to the overwrite the renderer used to perform. The old code
/// was not wrong; it was one instance of this rule, written where the general
/// case could not be expressed.
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
    /// Which frame this view presents in. Defaults to the world-fixed behaviour
    /// that existed before the policy, so a caller that does not care is
    /// unaffected.
    pub reference_frame: CameraReferenceFrame,
    /// The view subject's resolved down axis, read by `SubjectFrame`. `None`
    /// when the view has no subject to orient on.
    pub subject_down: Option<ae::Vec2>,
    /// A chart rotation this view is presenting through — a portal crossing
    /// today. `None` whenever nothing is mapping the world, which is almost
    /// every frame and every capture.
    pub chart_transit: Option<CameraChartTransit>,
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
    // **THE CLAMP MEASURES THE VIEW'S WORLD FOOTPRINT, NOT ITS SIZE.** A rolled
    // view occupies a rotated rectangle; asking whether it fits inside a room is
    // a question about that rectangle's bound, and until this line it was asked
    // of an upright one.
    let rotation_radians = presented_roll_radians(
        input.reference_frame,
        input.subject_down,
        input.chart_transit,
    );
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

/// **The world-space half-extents a ROLLED view occupies.**
///
/// A camera clamp asks *does the view fit inside these bounds*, and the answer
/// depends on the view's ORIENTATION: at a quarter turn a 16:9 viewport is
/// taller than it is wide. The clamp read `half_view_w`/`half_view_h` — the
/// footprint of an upright rectangle — so a rolled view was clamped as though it
/// were upright, and portal transits roll ±π/2 TODAY for a floor↔wall pair. That
/// is how a transit could show outside the room.
///
/// This is the axis-aligned bound of the rotated rectangle, which is what a
/// clamp that must CONTAIN the view needs. A tighter policy would use the convex
/// footprint; nothing asks for one.
///
/// ⭐ **the render-space y flip does not reach this**, which is worth stating
/// rather than rediscovering: both terms take absolute values, and `cos(-t) ==
/// cos(t)`, `|sin(-t)| == |sin(t)|`. Rolling either way occupies one footprint.
///
/// At zero roll this is the identity, so every upright view — which is every
/// view that is not mid-transit — clamps exactly as it always did.
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
// of sim-side ease state. Owning it here makes the AJ13 "camera is an
// observer" boundary structural: ONE resolved snapshot, ONE writer, and
// presentation only consumes it (portal continuity applies its deltas to a
// COPY, never to this state).
//
// The invariant E4-17 was really about is the single writer — NOT which
// schedule advances it. The resolve is a visible-host observer: it takes the
// physical viewport and video settings, eases on the render clock, and no sim
// system reads what it publishes. So it runs once per rendered frame in
// `Update`, which is truthful for render-frame, fixed-tick and GGRS hosts
// alike; see `CameraObservationPlugin` for why the sim schedule was wrong on
// its own terms.
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

/// **What a presentation adapter tells the resolve, before it resolves.**
///
/// The generic seam a presentation adapter (portal camera continuity today) may
/// write: an extra clamp target the bounds should be padded toward, and the
/// chart rotation the view is presenting through. Both default to `None` every
/// frame they are not actively needed — the writer owns clearing them.
///
/// ⛔ **the chart rotation had to join it rather than be layered afterwards.**
/// The renderer used to overwrite `rotation_radians` AFTER the resolve, which
/// left the resolver clamping an axis-aligned footprint for a view it did not
/// know was rolled — at a quarter turn the world-space footprint swaps width for
/// height, so a floor↔wall transit could show outside the room. A rotation-aware
/// clamp is only expressible once the roll is an INPUT.
///
/// ⭐ **one resource, because it is one act.** Both fields are written by the
/// same adapter, in the same frame, for the same reason: *this is what the
/// presentation layer needs the resolve to know before it resolves.* This
/// REPLACED a single-field `CameraExtraClamp` rather than adding a second
/// resource beside it — which also keeps the resolve system under Bevy's
/// parameter ceiling without bundling unrelated things to get there.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default)]
pub struct CameraPresentationInputs {
    /// Optional extra center that should remain inside the clamp bounds.
    pub extra_clamp_center_world: Option<ae::Vec2>,
    /// The chart rotation this view is presenting through, if any.
    pub chart_transit: Option<CameraChartTransit>,
}

/// THE published observation: the follow-camera snapshot resolved once per
/// rendered FRAME, plus the raw follow point it framed. Presentation reads
/// this (applying shake/portal deltas to a copy); RL/headless readers may read
/// it too — it is plain data.
///
/// Per frame, not per tick: this is presentation state. Under fixed-tick the
/// sim may advance zero or several times between frames, and under GGRS it
/// resimulates arbitrarily many times per frame during rollback; the camera
/// must ease once per thing the participant actually sees, and camera state is
/// not rollback-registered. See [`CameraObservationPlugin`].
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct ResolvedCameraSnapshot {
    pub snapshot: CameraSnapshot2d,
    /// World-frame position of the followed body (the controlled subject)
    /// this tick — the un-eased follow point presentation adapters (portal
    /// continuity) key their offsets from.
    pub follow_world: ae::Vec2,
}

/// Resolve the follow camera for this frame (the ONE writer of
/// [`CameraEaseState`]).
///
/// An OBSERVER: it runs once the sim has finished advancing for this frame, so
/// it sees final body positions, and after any post-sim presentation adapter
/// (portal camera continuity) has had its same-frame say through the
/// observer-input resources. Presentation consumers order
/// `.after(`[`CameraObservationSet`]`)`.
///
/// It reads sim state and writes only presentation state; nothing in the
/// simulation reads what it publishes.
#[allow(clippy::too_many_arguments)]
/// **Where to point a camera that is watching a cast rather than driving one,
/// and how wide to open it.**
///
/// Returns `None` for an empty or unresolvable cast, which is the ordinary case
/// outside a match — a caller with no declared cast has nothing to frame and
/// must not fall back to the world origin, because "the camera is at 0,0" looks
/// exactly like "the camera is broken" and this repo has shipped that before.
struct CastFraming {
    centre: ae::Vec2,
    /// The cast's bounding box plus margin. Used both as the framing base size
    /// and as a FLOOR on the view, so a pair that separates stays on screen.
    view: ae::Vec2,
    /// The body the presented-pose sample is taken from — the first seat, so
    /// the choice is stable rather than whichever entity sorted first.
    anchor: bevy::prelude::Entity,
}

/// Half the extra room left around the cast's bounding box, in world units.
/// Small on purpose: the view is a FLOOR, so authored zoom still wins whenever
/// it is already wider.
const CAST_FRAMING_MARGIN: f32 = 48.0;

fn frame_the_cast(
    cast: &[bevy::prelude::Entity],
    bodies: &bevy::prelude::Query<&ambition_platformer2d_shared_tangle::body::BodyKinematics>,
) -> Option<CastFraming> {
    let mut anchor = None;
    let (mut min, mut max) = (
        ae::Vec2::new(f32::MAX, f32::MAX),
        ae::Vec2::new(f32::MIN, f32::MIN),
    );
    for entity in cast {
        let Ok(kin) = bodies.get(*entity) else {
            continue;
        };
        anchor.get_or_insert(*entity);
        let half = kin.size / 2.0;
        min.x = min.x.min(kin.pos.x - half.x);
        min.y = min.y.min(kin.pos.y - half.y);
        max.x = max.x.max(kin.pos.x + half.x);
        max.y = max.y.max(kin.pos.y + half.y);
    }
    let anchor = anchor?;
    Some(CastFraming {
        centre: (min + max) / 2.0,
        view: ae::Vec2::new(
            (max.x - min.x) + CAST_FRAMING_MARGIN * 2.0,
            (max.y - min.y) + CAST_FRAMING_MARGIN * 2.0,
        ),
        anchor,
    })
}

pub fn resolve_camera_observation(
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_actor_monolith::rooms::RoomSet,
    >,
    time: bevy::prelude::Res<bevy::prelude::Time>,
    developer_tools: bevy::prelude::Res<ambition_dev_tools::dev_tools::DeveloperTools>,
    encounter_view: bevy::prelude::Res<ambition_encounter::EncounterView>,
    user_settings: bevy::prelude::Res<ambition_persistence::settings::UserSettings>,
    viewport: bevy::prelude::Res<CameraViewport>,
    screen_framing: bevy::prelude::Res<CameraScreenFraming>,
    presentation: bevy::prelude::Res<CameraPresentationInputs>,
    ease_tuning: bevy::prelude::Res<
        ambition_platformer2d_shared_tangle::camera_ease::CameraEaseTuning,
    >,
    mut camera_state: bevy::prelude::ResMut<
        ambition_platformer2d_shared_tangle::camera_ease::CameraEaseState,
    >,
    mut resolved: bevy::prelude::ResMut<ResolvedCameraSnapshot>,
    mut last_camera_room: bevy::prelude::Local<Option<String>>,
    player: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            &ambition_platformer2d_shared_tangle::body::BodyKinematics,
            &ae::BodyBaseSize,
            &ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
        ),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    // ⚠ ONE param, two resources: this system sits at Bevy's 16-param ceiling,
    // which is also why `followed_body` below is a tuple. `framed` is what to
    // look at when nothing is driving a body — see the `None` arm below.
    subject: (
        bevy::prelude::Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>,
        bevy::prelude::Res<ambition_platformer2d_shared_tangle::markers::FramedCast>,
    ),
    // Both lookups for whichever body is being followed, grouped into ONE
    // system param because this resolve sits at Bevy's 16-param ceiling.
    //
    // The camera frames the PRESENTED subject, not the raw tick pose: this and
    // the sprite must sample the same frame-clock position, or they disagree by
    // up to a tick of travel and the subject shudders — see `presented_pose`.
    followed_body: (
        bevy::prelude::Query<&ambition_platformer2d_shared_tangle::body::BodyKinematics>,
        bevy::prelude::Query<&crate::presented_pose::PresentedPose>,
        // **The frame the followed body resolved this tick** (ADR 0024), for a
        // view that presents in its subject's frame rather than the world's. Read
        // off the SAME entity the framing follows, so orientation and framing
        // cannot disagree about whose view this is — and read as an
        // already-resolved fact, never by asking gravity anything.
        bevy::prelude::Query<
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        >,
    ),
) {
    let (body_kinematics, presented, subject_frames) = followed_body;
    let (controlled, framed) = subject;
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

    // ⛔ **A HOME AVATAR IS NOT REQUIRED ANY MORE, and this `single()` used to
    // make it one.** Without a primary player the whole system returned, so an
    // experience that legitimately has no session body — a MATCH, which realizes
    // its own cast — would have had no camera at all, silently. That is the
    // failure mode this repo has been bitten by repeatedly: presentation not
    // running looks exactly like presentation running badly.
    //
    // The home avatar remains the source of blink easing and the base size when
    // there IS one, because those are its presentation state. When there is not,
    // the CONTROLLED SUBJECT supplies the frame on its own.
    let home = player.single().ok().map(|(e, b, bs, bc)| (e, *b, *bs, *bc));
    let (mut player_body, player_base_size, blink_cam, mut followed) = match home {
        Some((entity, body, base_size, blink)) => (body, base_size, blink, entity),
        None => {
            // ⭐ **NO HOME AVATAR: the controlled body, else the DECLARED CAST.**
            // The second half used to be a bare `return`, which is why a
            // CPU-versus-CPU match drew nothing at all — Jon's own run said so
            // before any test did. What to frame is a presentation decision this
            // resolver is TOLD (`FramedCast`), never one it guesses: a scan for
            // bodies would have to decide which ones matter, and whoever
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
                    let Some(cast) = frame_the_cast(&framed.0, &body_kinematics) else {
                        return;
                    };
                    // The bounds decide the ZOOM as well as the centre: two
                    // fighters that run apart must both stay on screen, and a
                    // fixed view centred between them is how one of them walks
                    // off it. A FLOOR, so authored zoom still wins when wider.
                    base_view = base_view.max(cast.view);
                    (
                        ae::BodyKinematics {
                            pos: cast.centre,
                            ..Default::default()
                        },
                        ae::BodyBaseSize {
                            base_size: cast.view,
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
    if let Ok(presented) = presented.get(followed) {
        player_body.pos = presented.presented();
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
            extra_clamp_center_world: presentation.extra_clamp_center_world,
            ease_tuning: *ease_tuning,
            screen_framing: Some(*screen_framing),
            // ⚠ **the policy is still the world-fixed default, and that is the
            // whole behaviour of this line today.** What is wired is the DATA:
            // the subject's resolved down axis now reaches the resolver, so
            // selecting `SubjectFrame` is a policy change rather than a plumbing
            // change. Where the selection lives is deliberately still open — the
            // one thing it must not become is a process-global mode, because a
            // view is what owns it once views are indexed.
            reference_frame: Default::default(),
            subject_down: subject_frames.get(followed).ok().map(|frame| frame.down()),
            // Written pre-resolve by the portal adapter, exactly like the extra
            // clamp beside it — so the snapshot states the view's ACTUAL final
            // orientation instead of one the renderer overwrites afterwards.
            chart_transit: presentation.chart_transit,
        },
        Some(&mut *camera_state),
    );
    *resolved = ResolvedCameraSnapshot {
        snapshot,
        follow_world: player_body.pos,
    };
}

/// Ordering handle for the camera observation resolve.
///
/// Everything that FEEDS the resolve (the host's presentation layout) orders
/// `.before` this; everything that CONSUMES it (`camera_follow`, the physical
/// viewport application, the surround) orders `.after`. One handle, one
/// schedule, so the relationship is expressible rather than assumed.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct CameraObservationSet;

/// The observation seam's plugin: owns the observer-input resources + the
/// published snapshot, and schedules the ONE resolve per FRAME. Part of
/// [`PlatformerEnginePlugins`] — headless apps get a live snapshot too.
///
/// # Why `Update` and not the sim schedule
///
/// The resolve is an OBSERVER, and specifically a *visible-host* observer: its
/// inputs include the physical viewport and video settings, it integrates
/// [`CameraEaseState`] on the render clock, and its sole consumer is
/// `camera_follow` in `Update`. Nothing in the simulation reads
/// [`ResolvedCameraSnapshot`].
///
/// Registering it into `app.sim_schedule()` made that relationship
/// inexpressible, because Bevy ordering is SCHEDULE-LOCAL. `.before`/`.after`
/// edges between this system and the `Update`-side presentation cluster were
/// silently inert whenever the sim was not itself in `Update` — so fixed-tick
/// and GGRS hosts could apply a physical `Camera.viewport` from this frame's
/// layout while the snapshot still described last frame's. It was also wrong on
/// its own terms: on `FixedUpdate` the camera eased zero or two times per
/// rendered frame, and under GGRS it re-integrated the ease state on every
/// rollback resimulation step (no camera state is rollback-registered).
///
/// `Update` is truthful for all three hosts. `FixedUpdate` runs inside
/// `RunFixedMainLoop` and GGRS drives `GgrsSchedule` from `PreUpdate`; both
/// complete before `Update` in the same frame, so the sim is always finished
/// advancing before the camera observes it. This preserves the E4-17 invariant
/// that mattered — ONE writer of [`CameraEaseState`], render only consumes —
/// and changes only which clock it observes on.
pub struct CameraObservationPlugin;

impl bevy::prelude::Plugin for CameraObservationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;
        use bevy::prelude::IntoScheduleConfigs as _;
        app.init_resource::<CameraViewport>();
        app.init_resource::<CameraScreenFraming>();
        app.init_resource::<CameraPresentationInputs>();
        app.init_resource::<ResolvedCameraSnapshot>();

        // Declared ONLY when the sim shares this schedule. In fixed-tick and
        // GGRS hosts the sim is in another schedule, where this edge would be
        // an inert no-op that reads as a guarantee; the schedule boundary
        // already provides the ordering there.
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
            resolve_camera_observation.in_set(CameraObservationSet),
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
                ease_tuning: CameraEaseTuning::default(),
                screen_framing: None,
                reference_frame: Default::default(),
                subject_down: None,
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
    use ambition_platformer2d_shared_tangle::camera_ease::CameraEaseState;

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

    /// **World-fixed is the default and rolls for nothing.**
    ///
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

    /// **A subject-relative view puts the subject's feet at the bottom.**
    ///
    /// ⚠ the sign is the load-bearing part, and it is fixed by render space
    /// (world with y flipped) — the same convention `portal_transit_roll` uses,
    /// because they write the same field. Ordinary gravity must be the identity
    /// or every existing room would tilt the moment the mode is selected.
    #[test]
    fn subject_frame_orients_on_the_subjects_down_axis() {
        let roll = |x: f32, y: f32| {
            observer_roll_radians(CameraReferenceFrame::SubjectFrame, Some(ae::Vec2::new(x, y)))
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

    /// **No subject is not an error.** A view framing a cast, or nothing, reads
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

    /// **A world-fixed view presents the chart rotation and nothing else.**
    ///
    /// ⛔ this is the behaviour that shipped, and it must be preserved to the
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

    /// **A subject-frame view adds the chart rotation when the portal did not
    /// move its frame.**
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

    /// **⛔⛔ A CROSSING THAT ROTATES THE BODY'S OWN FRAME MUST NOT COUNT TWICE.**
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

    /// **An upright view's clamp footprint is its viewport, exactly.**
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

    /// **⛔ A QUARTER TURN SWAPS WIDTH AND HEIGHT** — the defect this exists for.
    ///
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

    /// **A diagonal roll grows BOTH extents, and rolling either way is the same
    /// footprint.**
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
