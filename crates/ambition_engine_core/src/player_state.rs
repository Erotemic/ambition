//! Reusable player-state vocabulary.
//!
//! Main primitives: explicit locomotion state, alternate body modes, and generic
//! resource meters. They are Bevy-free so the visible sandbox, headless sim, and
//! future pure-engine adapters can share the same vocabulary.

use crate::Vec2;
use serde::{Deserialize, Serialize};

/// Explicit movement / locomotion mode for the player.
///
/// Replaces the implicit "infer from on_ground / dash_timer / blink_aiming"
/// shape that older code uses. The variants intentionally cover both the
/// shipping sandbox's verbs (Grounded/Airborne/WallSlide/Dashing/Blinking)
/// and the Tier 1 mechanics-checklist verbs we expect to land soon
/// (Crouching/Crawling/Sliding/MorphBall/GrappleAiming/CurveRiding/Hitstun).
///
/// Adding a variant is a real architectural decision — keep this list
/// narrow and documented per the engine-vs-sandbox crate-boundary memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocomotionState {
    Grounded,
    Airborne,
    WallSlide,
    WallCling,
    WallClimb,
    LedgeHang,
    LedgeClimb,
    Dashing,
    Blinking,
    BlinkAiming,
    Flying,
    FastFalling,
    Crouching,
    Crawling,
    Sliding,
    MorphBall,
    GrappleAiming,
    GrapplePulling,
    CurveRiding,
    Hitstun,
}

impl LocomotionState {
    /// Static label used in HUD / trace dumps. Stable across renames so
    /// stored traces stay readable.
    pub fn label(self) -> &'static str {
        match self {
            LocomotionState::Grounded => "Grounded",
            LocomotionState::Airborne => "Airborne",
            LocomotionState::WallSlide => "WallSlide",
            LocomotionState::WallCling => "WallCling",
            LocomotionState::WallClimb => "WallClimb",
            LocomotionState::LedgeHang => "LedgeHang",
            LocomotionState::LedgeClimb => "LedgeClimb",
            LocomotionState::Dashing => "Dashing",
            LocomotionState::Blinking => "Blinking",
            LocomotionState::BlinkAiming => "BlinkAiming",
            LocomotionState::Flying => "Flying",
            LocomotionState::FastFalling => "FastFalling",
            LocomotionState::Crouching => "Crouching",
            LocomotionState::Crawling => "Crawling",
            LocomotionState::Sliding => "Sliding",
            LocomotionState::MorphBall => "MorphBall",
            LocomotionState::GrappleAiming => "GrappleAiming",
            LocomotionState::GrapplePulling => "GrapplePulling",
            LocomotionState::CurveRiding => "CurveRiding",
            LocomotionState::Hitstun => "Hitstun",
        }
    }

    /// Project `LocomotionState` from a body: the shared contact clusters plus
    /// the movement policy, whose model-private maneuver state (ADR 0024) owns
    /// the dash/blink/ledge/wall-engagement facts. Mirrors the same priority
    /// order callers used to drive off of `&Player`. Non-axis policies expose
    /// no maneuver verbs here and project to grounded/airborne from the shared
    /// support fact.
    pub fn from_body(
        model: &crate::movement::MotionModel,
        ground: &crate::body_clusters::BodyGroundState,
        wall: &crate::body_clusters::BodyWallState,
        flight: &crate::body_clusters::BodyFlightState,
    ) -> Self {
        let crate::movement::MotionModel::AxisSwept(axis) = model else {
            return if ground.on_ground {
                LocomotionState::Grounded
            } else {
                LocomotionState::Airborne
            };
        };
        let state = &axis.state;
        if state.dash_timer > 0.0 {
            return LocomotionState::Dashing;
        }
        if state.blink_aiming {
            return LocomotionState::BlinkAiming;
        }
        if flight.fly_enabled {
            return LocomotionState::Flying;
        }
        if let Some(grab) = state.ledge_grab {
            return if grab.climbing {
                LocomotionState::LedgeClimb
            } else {
                LocomotionState::LedgeHang
            };
        }
        if state.wall_climbing {
            return LocomotionState::WallClimb;
        }
        if state.wall_clinging {
            return LocomotionState::WallCling;
        }
        if wall.on_wall && !ground.on_ground {
            return LocomotionState::WallSlide;
        }
        if state.fast_falling {
            return LocomotionState::FastFalling;
        }
        if ground.on_ground {
            LocomotionState::Grounded
        } else {
            LocomotionState::Airborne
        }
    }
}

/// Alternate body-shape stance for the player.
///
/// The associated AABB size is returned by `body_shape`; mechanics that
/// want to swap modes should ask the world whether the new shape fits
/// at the player's position before committing (see
/// `BodyShape::fits_at`). MorphBall/Sliding/Crawling are listed even
/// though only Standing+Crouching are likely to be wired up in the
/// first sandbox station; the variants exist so the rest of the engine
/// can assume the enum is closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BodyMode {
    #[default]
    Standing,
    Crouching,
    Crawling,
    Sliding,
    MorphBall,
    /// Player is on a ladder / climbable surface. Body shape is
    /// identical to `Standing` (the player isn't compressed by
    /// climbing) but movement integration suspends gravity and
    /// converts vertical input to climb_speed motion. Set by the
    /// sandbox-side body-mode driver when `Player::climbable_contact`
    /// is `Some` and the player initiates a climb (Up/Down press).
    /// Jump while climbing and moving upward keeps this mode active
    /// and upgrades the climb to a short jump-speed boost; Down +
    /// Jump falls off the ladder with a short re-grab grace. Dash
    /// still clears the mode, as does losing contact.
    Climbing,
}

impl BodyMode {
    pub fn label(self) -> &'static str {
        match self {
            BodyMode::Standing => "Standing",
            BodyMode::Crouching => "Crouching",
            BodyMode::Crawling => "Crawling",
            BodyMode::Sliding => "Sliding",
            BodyMode::MorphBall => "MorphBall",
            BodyMode::Climbing => "Climbing",
        }
    }

    /// Read the player's authoritative body-mode field from cluster
    /// components.
    pub fn from_clusters(body_mode: &crate::body_clusters::BodyModeState) -> Self {
        body_mode.body_mode
    }

    pub fn shape(self, base_size: Vec2) -> BodyShape {
        match self {
            BodyMode::Standing => BodyShape {
                mode: self,
                size: base_size,
            },
            BodyMode::Crouching => BodyShape {
                mode: self,
                // Crouch is half-height; width unchanged.
                size: Vec2::new(base_size.x, base_size.y * 0.55),
            },
            BodyMode::Crawling => BodyShape {
                mode: self,
                // Crawl is much shorter and a touch narrower so the
                // player can fit through low tunnels but not arbitrary
                // gaps.
                size: Vec2::new(base_size.x * 0.85, base_size.y * 0.35),
            },
            BodyMode::Sliding => BodyShape {
                mode: self,
                size: Vec2::new(base_size.x * 1.05, base_size.y * 0.40),
            },
            BodyMode::MorphBall => {
                // Symmetric small ball, sized to fit a one-grid-cell
                // (16 px) tunnel with ~1 px clearance per side.
                //
                // DECOUPLED from `base_size` on purpose: the morph-
                // ball mechanic exists so the player can squeeze
                // through tunnels authored on the 16-px grid, not to
                // scale with the player's standing proportions. The
                // earlier `base_size.x * 0.55` math worked for the
                // old 28-px base (15.4 ≈ fits within rounding), but
                // when the standing base grew to 30 px the ball
                // became 16.5 and STOPPED fitting the 16-px morph_lab
                // tunnel — morph mode looked broken even though the
                // player had successfully transitioned into it.
                //
                // Holds the Crawl/Slide/Crouch/Stand discriminator:
                //   Crawl   = 25.5 × 16.8 → blocked
                //   Slide   = 31.5 × 19.2 → blocked
                //   Crouch  = 30   × 26.4 → blocked
                //   Stand   = 30   × 48   → blocked
                //   Morph   = 14   × 14   → fits with 1 px each side
                let _ = base_size; // retained for signature parity
                BodyShape {
                    mode: self,
                    size: Vec2::new(14.0, 14.0),
                }
            }
            BodyMode::Climbing => BodyShape {
                mode: self,
                // Climbing keeps the standing silhouette so the
                // climbable region's intersection check stays stable
                // across the transition. Future-proof: if we add a
                // "hugging the ladder" pose with reduced width, change
                // it here without touching call sites.
                size: base_size,
            },
        }
    }
}

/// AABB size + mode resolved from `BodyMode::shape`. The mode is kept
/// alongside the size so callers can short-circuit equality checks
/// without re-querying.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyShape {
    pub mode: BodyMode,
    pub size: Vec2,
}

impl BodyShape {
    /// Probe whether this shape fits at `center` without overlapping
    /// any block accepted by `predicate`. Callers can pass any block
    /// predicate but typically gate on `BlockKind::Solid` (cannot
    /// stand into a ceiling) and `BlockKind::OneWay` for stand-up
    /// inside a one-way ceiling.
    pub fn fits_at<F>(self, center: Vec2, world: &crate::world::World, predicate: F) -> bool
    where
        F: FnMut(&crate::world::Block) -> bool,
    {
        let aabb = crate::geometry::Aabb::new(center, self.size * 0.5);
        !world.body_overlaps_any(aabb, predicate)
    }
}

/// Attempt to change `player.body_mode` to `new_mode`.
///
/// Computes the new local-body shape via `BodyMode::shape(player.base_size)`,
/// adjusts the center along the acceleration frame so the body's feet stay
/// planted, then checks the oriented world-space AABB with the caller's
/// predicate. On success the player's `pos`, `size`, and `body_mode` are updated
/// and the function returns `true`. On failure all three are left untouched.
///
/// Sandbox crouch / morph wiring should call this every frame: each
/// transition is naturally idempotent because requesting the current
/// mode is a no-op success. Standing-back-up against a low ceiling
/// returns `false`, which the caller can surface as a "blocked stand-up"
/// trace event without re-deriving the geometry.
///
/// AABB convention: `pos` is the AABB center and Ambition uses +Y down.
/// The body's FEET are the AABB face in the acceleration direction. Under normal
/// gravity this is the bottom edge; under inverted gravity it is the top edge;
/// under sideways gravity it is the left/right edge. Shape changes therefore
/// shift the body along `AccelerationFrame::down`, not hard-coded world Y. This
/// keeps crouch/morph transitions grounded for wall-walking just like for
/// floor/ceiling walking.
/// Transition the body mode while keeping the feet planted. Mutates
/// only the kinematics (pos, size) and body-mode cluster components;
/// the rest of the player state is untouched. Returns `false` (and
/// leaves state unchanged) when the target shape doesn't fit in the
/// current world geometry — e.g. a low ceiling rejecting a stand-up.
pub fn try_change_body_mode_clusters<F>(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    base_size: &crate::body_clusters::BodyBaseSize,
    body_mode_state: &mut crate::body_clusters::BodyModeState,
    new_mode: BodyMode,
    world: &crate::world::World,
    gravity_dir: Vec2,
    predicate: F,
) -> bool
where
    F: FnMut(&crate::world::Block) -> bool,
{
    if body_mode_state.body_mode == new_mode {
        return true;
    }
    let frame = crate::AccelerationFrame::new(gravity_dir);
    let new_shape = new_mode.shape(base_size.base_size);

    let old_world_half = frame.to_world_half(kinematics.size * 0.5);
    let new_world_half = frame.to_world_half(new_shape.size * 0.5);
    let gravity_axis = frame.down;
    let old_feet_half =
        old_world_half.x * gravity_axis.x.abs() + old_world_half.y * gravity_axis.y.abs();
    let new_feet_half =
        new_world_half.x * gravity_axis.x.abs() + new_world_half.y * gravity_axis.y.abs();
    let new_center = kinematics.pos + gravity_axis * (old_feet_half - new_feet_half);

    let old_aabb = crate::geometry::Aabb::new(kinematics.pos, old_world_half);
    let new_aabb = crate::geometry::Aabb::new(new_center, new_world_half);
    // A feet-anchored shrink occupies a subset of the body's current space. It
    // cannot create a new collision, and must remain legal even when a surface
    // follower's contact tolerance leaves the standing AABB microscopically
    // embedded in its supporting floor. Re-testing that inherited overlap made
    // momentum bodies refuse to crouch on flat blocks while the same input
    // worked on SurfaceChains. Expansion still performs the full clearance test.
    let subset_eps = 1.0e-4;
    let is_spatial_subset = new_aabb.min.x >= old_aabb.min.x - subset_eps
        && new_aabb.min.y >= old_aabb.min.y - subset_eps
        && new_aabb.max.x <= old_aabb.max.x + subset_eps
        && new_aabb.max.y <= old_aabb.max.y + subset_eps;
    if !is_spatial_subset && world.body_overlaps_any(new_aabb, predicate) {
        return false;
    }
    kinematics.pos = new_center;
    kinematics.size = new_shape.size;
    body_mode_state.body_mode = new_mode;
    true
}

/// Result of [`classify_player_safety`]. The recorder, OOB detector,
/// and "remember safe spawn point" logic all consult this so a single
/// place defines what counts as a legal player position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerSafetyVerdict {
    /// Position and velocity are finite, AABB is inside the world
    /// envelope, and the player isn't overlapping any block matched by
    /// the caller's predicate (typically `BlockKind::Solid`).
    Safe,
    /// `pos.x` or `pos.y` is NaN/inf.
    PositionNonFinite,
    /// `vel.x` or `vel.y` is NaN/inf.
    VelocityNonFinite,
    /// AABB is outside the world envelope on the named axis (`'x'` or
    /// `'y'`). Callers can include the margin they tolerate.
    OutsideWorldEnvelope { axis: char },
    /// AABB strictly intersects a block accepted by the caller's
    /// predicate (typically a `Solid`).
    InsideSolid,
}

impl PlayerSafetyVerdict {
    pub fn is_safe(self) -> bool {
        matches!(self, PlayerSafetyVerdict::Safe)
    }
}

/// Single source of truth for "is this player position legal?". Used by
/// the trace recorder's OOB detector and by the sandbox runtime's
/// "remember last safe position" logic. Sharing the predicate prevents
/// the two definitions from drifting (concrete repro that motivated
/// this helper: the trace-recorded `last_safe_pos` was being set to
/// `(62, -23)`, an above-world position the OOB detector explicitly
/// rejected one frame later).
///
/// `margin` is the tolerance beyond the world envelope. Pass `0.0`
/// for a strict "inside the world" check; the trace recorder uses a
/// looser margin so the camera can briefly extend past the room
/// without auto-dumping.
/// AABB-only player safety classifier. Takes the
/// pos/vel/aabb directly so callers driving cluster components do not
/// need to materialize an `ae::Player`.
pub fn classify_safety_from_kinematics<F>(
    pos: crate::Vec2,
    vel: crate::Vec2,
    aabb: crate::Aabb,
    world: &crate::world::World,
    margin: f32,
    mut solid_predicate: F,
) -> PlayerSafetyVerdict
where
    F: FnMut(&crate::world::Block) -> bool,
{
    if !pos.x.is_finite() || !pos.y.is_finite() {
        return PlayerSafetyVerdict::PositionNonFinite;
    }
    if !vel.x.is_finite() || !vel.y.is_finite() {
        return PlayerSafetyVerdict::VelocityNonFinite;
    }
    use crate::geometry::AabbExt;
    if aabb.left() < -margin || aabb.right() > world.size.x + margin {
        return PlayerSafetyVerdict::OutsideWorldEnvelope { axis: 'x' };
    }
    if aabb.top() < -margin || aabb.bottom() > world.size.y + margin {
        return PlayerSafetyVerdict::OutsideWorldEnvelope { axis: 'y' };
    }
    if world.body_overlaps_any(aabb, |b| solid_predicate(b)) {
        return PlayerSafetyVerdict::InsideSolid;
    }
    PlayerSafetyVerdict::Safe
}

/// Generic resource meter (stamina / mana / ammo / charge / hover fuel).
///
/// `current` is clamped to `[0, max]`. `regen_rate` adds per second
/// during `tick`; `decay_rate` subtracts per second during `tick`.
/// Mechanics that should regen only when idle can call `tick_regen`
/// directly and skip `tick_decay`, or vice versa.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceMeter {
    pub current: f32,
    pub max: f32,
    pub regen_rate: f32,
    pub decay_rate: f32,
}

impl ResourceMeter {
    pub fn new(max: f32, regen_rate: f32, decay_rate: f32) -> Self {
        Self {
            current: max,
            max,
            regen_rate,
            decay_rate,
        }
    }

    /// Try to consume `cost`. Returns `true` and subtracts on success,
    /// `false` and leaves the meter unchanged on failure.
    pub fn try_spend(&mut self, cost: f32) -> bool {
        if cost < 0.0 {
            return false;
        }
        if self.current + 1e-6 < cost {
            return false;
        }
        self.current = (self.current - cost).max(0.0);
        true
    }

    pub fn refill(&mut self, amount: f32) {
        self.current = (self.current + amount).clamp(0.0, self.max);
    }

    pub fn refill_full(&mut self) {
        self.current = self.max;
    }

    pub fn tick_regen(&mut self, dt: f32) {
        if self.regen_rate > 0.0 && dt > 0.0 {
            self.refill(self.regen_rate * dt);
        }
    }

    pub fn tick_decay(&mut self, dt: f32) {
        if self.decay_rate > 0.0 && dt > 0.0 {
            self.current = (self.current - self.decay_rate * dt).max(0.0);
        }
    }

    /// Apply both regen and decay in one call. Sequence matters when
    /// rates are equal: regen first, then decay.
    pub fn tick(&mut self, dt: f32) {
        self.tick_regen(dt);
        self.tick_decay(dt);
    }

    pub fn fraction(self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }

    pub fn is_full(self) -> bool {
        self.current >= self.max - 1e-6
    }

    pub fn is_empty(self) -> bool {
        self.current <= 1e-6
    }
}

#[cfg(test)]
mod tests;
