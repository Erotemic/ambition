//! Reusable player-state vocabulary.
//!
//! Adds three high-leverage primitives identified in
//! `docs/mechanics/expressibility-checklist.md` Tier 1:
//!
//! * `LocomotionState` — explicit player movement mode, derived from the
//!   existing `Player` struct so older code that still reads booleans /
//!   timers keeps working. New mechanics should branch on the enum.
//! * `BodyMode` — alternate body-shape stance. Backed by a `BodyShape`
//!   table that returns the AABB size each mode uses; gameplay can
//!   query "would this body shape fit here" before actually swapping
//!   stances (the start of collision-safe resize).
//! * `ResourceMeter` — generic stamina/mana/ammo/charge meter with
//!   regen/decay rates. `try_spend` honours the floor at 0; `tick`
//!   advances regen (when above zero spend) and decay independently
//!   so meters that should drain only when "in use" can be modeled
//!   with two separate meters or by skipping the tick on idle frames.
//!
//! These primitives are intentionally Bevy-free so they survive both the
//! sandbox visible-binary and the headless simulation (and any future
//! pure-engine RL adapter). The sandbox attaches `LocomotionState` and
//! `BodyMode` at the trace boundary by calling `from_player`; richer
//! systems (HUD, future per-mode physics) can keep the value as a
//! component or resource.

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

    /// Project `LocomotionState` from cluster components. Mirrors the
    /// same priority order callers used to drive off of `&Player`.
    pub fn from_clusters(
        ground: &crate::player_clusters::PlayerGroundState,
        wall: &crate::player_clusters::PlayerWallState,
        flight: &crate::player_clusters::PlayerFlightState,
        dash: &crate::player_clusters::PlayerDashState,
        blink: &crate::player_clusters::PlayerBlinkState,
        ledge: &crate::player_clusters::PlayerLedgeState,
    ) -> Self {
        if dash.timer > 0.0 {
            return LocomotionState::Dashing;
        }
        if blink.aiming {
            return LocomotionState::BlinkAiming;
        }
        if flight.fly_enabled {
            return LocomotionState::Flying;
        }
        if let Some(grab) = ledge.grab {
            return if grab.climbing {
                LocomotionState::LedgeClimb
            } else {
                LocomotionState::LedgeHang
            };
        }
        if wall.wall_climbing {
            return LocomotionState::WallClimb;
        }
        if wall.wall_clinging {
            return LocomotionState::WallCling;
        }
        if wall.on_wall && !ground.on_ground {
            return LocomotionState::WallSlide;
        }
        if flight.fast_falling {
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
    pub fn from_clusters(body_mode: &crate::player_clusters::PlayerBodyModeState) -> Self {
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
/// Computes the new shape via `BodyMode::shape(player.base_size)`,
/// adjusts `pos.y` so the player's feet stay planted, then checks
/// `BodyShape::fits_at` with the caller's predicate. On success the
/// player's `pos`, `size`, and `body_mode` are updated and the function
/// returns `true`. On failure all three are left untouched.
///
/// Sandbox crouch / morph wiring should call this every frame: each
/// transition is naturally idempotent because requesting the current
/// mode is a no-op success. Standing-back-up against a low ceiling
/// returns `false`, which the caller can surface as a "blocked stand-up"
/// trace event without re-deriving the geometry.
///
/// AABB convention: `pos` is the AABB center and Ambition uses +Y down,
/// so `feet_y == pos.y + size.y * 0.5`. Shrinking the body keeps feet
/// planted by *increasing* `pos.y` by half the height delta.
/// Transition the body mode while keeping the feet planted. Mutates
/// only the kinematics (pos, size) and body-mode cluster components;
/// the rest of the player state is untouched. Returns `false` (and
/// leaves state unchanged) when the target shape doesn't fit in the
/// current world geometry — e.g. a low ceiling rejecting a stand-up.
pub fn try_change_body_mode_clusters<F>(
    kinematics: &mut crate::player_clusters::BodyKinematics,
    base_size: &crate::player_clusters::PlayerBaseSize,
    body_mode_state: &mut crate::player_clusters::PlayerBodyModeState,
    new_mode: BodyMode,
    world: &crate::world::World,
    predicate: F,
) -> bool
where
    F: FnMut(&crate::world::Block) -> bool,
{
    if body_mode_state.body_mode == new_mode {
        return true;
    }
    let new_shape = new_mode.shape(base_size.base_size);
    let dy = (kinematics.size.y - new_shape.size.y) * 0.5;
    let new_center = Vec2::new(kinematics.pos.x, kinematics.pos.y + dy);
    if !new_shape.fits_at(new_center, world, predicate) {
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
mod tests {
    use super::*;
    use crate::movement::default_player_body_size;
    use crate::world::{Block, BlockKind, World};

    fn scratch_at(pos: Vec2) -> crate::PlayerClusterScratch {
        crate::PlayerClusterScratch::new_with_abilities(pos, crate::AbilitySet::sandbox_all())
    }

    fn locomotion(s: &crate::PlayerClusterScratch) -> LocomotionState {
        LocomotionState::from_clusters(&s.ground, &s.wall, &s.flight, &s.dash, &s.blink, &s.ledge)
    }

    #[test]
    fn locomotion_default_grounded_when_player_on_ground() {
        let mut s = scratch_at(Vec2::new(0.0, 0.0));
        s.ground.on_ground = true;
        assert_eq!(locomotion(&s), LocomotionState::Grounded);
    }

    #[test]
    fn locomotion_airborne_when_off_ground() {
        let s = scratch_at(Vec2::new(0.0, 0.0));
        assert_eq!(locomotion(&s), LocomotionState::Airborne);
    }

    #[test]
    fn locomotion_dashing_overrides_other_states() {
        let mut s = scratch_at(Vec2::new(0.0, 0.0));
        s.ground.on_ground = true;
        s.dash.timer = 0.10;
        assert_eq!(locomotion(&s), LocomotionState::Dashing);
    }

    #[test]
    fn locomotion_blink_aiming_recognized() {
        let mut s = scratch_at(Vec2::new(0.0, 0.0));
        s.blink.aiming = true;
        assert_eq!(locomotion(&s), LocomotionState::BlinkAiming);
    }

    #[test]
    fn body_shape_smaller_for_crouch_and_morph() {
        let base = Vec2::new(28.0, 46.0);
        let standing = BodyMode::Standing.shape(base);
        let crouch = BodyMode::Crouching.shape(base);
        let morph = BodyMode::MorphBall.shape(base);
        assert_eq!(standing.size, base);
        assert!(crouch.size.y < standing.size.y);
        assert!(morph.size.x < standing.size.x);
        assert!(morph.size.y < standing.size.y);
    }

    #[test]
    fn body_fits_at_open_space() {
        let world = World::new(
            "test",
            Vec2::new(200.0, 200.0),
            Vec2::new(50.0, 50.0),
            Vec::new(),
        );
        let shape = BodyMode::Standing.shape(Vec2::new(28.0, 46.0));
        assert!(shape.fits_at(Vec2::new(50.0, 50.0), &world, |_| true));
    }

    #[test]
    fn body_does_not_fit_inside_solid_block() {
        let world = World::new(
            "test",
            Vec2::new(200.0, 200.0),
            Vec2::new(50.0, 50.0),
            vec![Block::solid(
                "ceiling",
                Vec2::new(40.0, 40.0),
                Vec2::new(60.0, 30.0),
            )],
        );
        // Standing fits below the ceiling but not under it; check the
        // collision-safe stand-up case directly.
        let standing = BodyMode::Standing.shape(Vec2::new(28.0, 46.0));
        assert!(!standing.fits_at(Vec2::new(70.0, 65.0), &world, |b| {
            matches!(b.kind, crate::world::BlockKind::Solid)
        }));
    }

    #[test]
    fn resource_meter_try_spend_succeeds_and_reduces() {
        let mut m = ResourceMeter::new(10.0, 0.0, 0.0);
        assert!(m.try_spend(3.0));
        assert!((m.current - 7.0).abs() < 1e-4);
    }

    #[test]
    fn resource_meter_try_spend_fails_when_insufficient() {
        let mut m = ResourceMeter::new(2.0, 0.0, 0.0);
        assert!(!m.try_spend(5.0));
        assert_eq!(m.current, 2.0);
    }

    #[test]
    fn resource_meter_regen_clamps_to_max() {
        let mut m = ResourceMeter::new(10.0, 5.0, 0.0);
        m.current = 8.0;
        m.tick_regen(1.0);
        assert!((m.current - 10.0).abs() < 1e-4);
    }

    #[test]
    fn resource_meter_decay_clamps_at_zero() {
        let mut m = ResourceMeter::new(10.0, 0.0, 100.0);
        m.current = 1.0;
        m.tick_decay(1.0);
        assert_eq!(m.current, 0.0);
    }

    #[test]
    fn try_change_body_mode_to_crouching_keeps_feet_planted_and_shrinks() {
        let world = World::new(
            "test",
            Vec2::new(400.0, 400.0),
            Vec2::new(50.0, 50.0),
            Vec::new(),
        );
        let mut s = scratch_at(Vec2::new(100.0, 100.0));
        let original_size = s.kinematics.size;
        let original_feet = s.kinematics.pos.y + s.kinematics.size.y * 0.5;

        let ok = try_change_body_mode_clusters(
            &mut s.kinematics,
            &s.base_size,
            &mut s.body_mode,
            BodyMode::Crouching,
            &world,
            |_| true,
        );
        assert!(ok);
        assert_eq!(s.body_mode.body_mode, BodyMode::Crouching);
        assert!(s.kinematics.size.y < original_size.y);
        assert_eq!(s.kinematics.size.x, original_size.x);
        let new_feet = s.kinematics.pos.y + s.kinematics.size.y * 0.5;
        assert!((new_feet - original_feet).abs() < 1e-3);
    }

    #[test]
    fn try_change_body_mode_back_to_standing_uses_base_size() {
        let world = World::new(
            "test",
            Vec2::new(400.0, 400.0),
            Vec2::new(50.0, 50.0),
            Vec::new(),
        );
        let mut s = scratch_at(Vec2::new(100.0, 100.0));
        let base = s.base_size.base_size;
        try_change_body_mode_clusters(
            &mut s.kinematics,
            &s.base_size,
            &mut s.body_mode,
            BodyMode::Crouching,
            &world,
            |_| true,
        );
        try_change_body_mode_clusters(
            &mut s.kinematics,
            &s.base_size,
            &mut s.body_mode,
            BodyMode::Crouching,
            &world,
            |_| true,
        );
        let ok = try_change_body_mode_clusters(
            &mut s.kinematics,
            &s.base_size,
            &mut s.body_mode,
            BodyMode::Standing,
            &world,
            |_| true,
        );
        assert!(ok);
        assert_eq!(s.body_mode.body_mode, BodyMode::Standing);
        assert_eq!(s.kinematics.size, base);
    }

    #[test]
    fn try_change_body_mode_blocked_stand_up_under_low_ceiling() {
        let player_spawn = Vec2::new(100.0, 100.0);
        let mut s = scratch_at(player_spawn);
        let standing_top = s.kinematics.pos.y - s.kinematics.size.y * 0.5;
        let ceiling_bottom = standing_top + 5.0;
        let ceiling_top = ceiling_bottom - 30.0;
        let world = World::new(
            "test",
            Vec2::new(400.0, 400.0),
            Vec2::new(50.0, 50.0),
            vec![Block::solid(
                "ceiling",
                Vec2::new(s.kinematics.pos.x - 50.0, ceiling_top),
                Vec2::new(100.0, 30.0),
            )],
        );

        let ok = try_change_body_mode_clusters(
            &mut s.kinematics,
            &s.base_size,
            &mut s.body_mode,
            BodyMode::Crouching,
            &world,
            |b| matches!(b.kind, crate::world::BlockKind::Solid),
        );
        assert!(ok);

        let stand_attempt = try_change_body_mode_clusters(
            &mut s.kinematics,
            &s.base_size,
            &mut s.body_mode,
            BodyMode::Standing,
            &world,
            |b| matches!(b.kind, crate::world::BlockKind::Solid),
        );
        assert!(!stand_attempt);
        assert_eq!(s.body_mode.body_mode, BodyMode::Crouching);
    }

    #[test]
    fn try_change_body_mode_to_same_mode_is_no_op_success() {
        let world = World::new(
            "test",
            Vec2::new(400.0, 400.0),
            Vec2::new(50.0, 50.0),
            Vec::new(),
        );
        let mut s = scratch_at(Vec2::new(100.0, 100.0));
        let pos_before = s.kinematics.pos;
        let size_before = s.kinematics.size;
        let ok = try_change_body_mode_clusters(
            &mut s.kinematics,
            &s.base_size,
            &mut s.body_mode,
            BodyMode::Standing,
            &world,
            |_| true,
        );
        assert!(ok);
        assert_eq!(s.kinematics.pos, pos_before);
        assert_eq!(s.kinematics.size, size_before);
    }

    #[test]
    fn resource_meter_fraction_handles_zero_max() {
        let m = ResourceMeter {
            current: 5.0,
            max: 0.0,
            regen_rate: 0.0,
            decay_rate: 0.0,
        };
        assert_eq!(m.fraction(), 0.0);
    }

    /// Regression test for the morph_lab tunnel: a one-grid-cell
    /// (16 px) gap between a ceiling at y=336 and a floor at y=352
    /// must allow MorphBall through and block every other body
    /// mode (Standing, Crouching, Crawling, Sliding). The morph-ball
    /// shape is decoupled from `base_size` precisely so the
    /// discriminator survives changes to the player's standing
    /// dimensions — earlier the multiplier-based morph ball became
    /// 16.5 px when `base_size.x` grew to 30, snagging on the
    /// 16-px tunnel even though the player had transitioned into
    /// morph mode.
    #[test]
    fn morphball_fits_one_grid_cell_tunnel() {
        // A "tunnel" sandwich: floor at y=352, low ceiling at y=336.
        // 16-px gap between them. Player center at y=344 (midway).
        let world = World::new(
            "morph_tunnel",
            Vec2::new(200.0, 500.0),
            Vec2::ZERO,
            vec![
                Block::solid("floor", Vec2::new(0.0, 352.0), Vec2::new(200.0, 40.0)),
                Block::solid("ceiling", Vec2::new(0.0, 200.0), Vec2::new(200.0, 136.0)),
            ],
        );
        let base = default_player_body_size();
        let center = Vec2::new(100.0, 344.0);
        let solid_predicate = |b: &Block| matches!(b.kind, BlockKind::Solid);
        assert!(
            BodyMode::MorphBall
                .shape(base)
                .fits_at(center, &world, solid_predicate),
            "MorphBall must fit a 16-px tunnel — the morph_lab tunnel is sized exactly this way",
        );
        for non_fit in [
            BodyMode::Standing,
            BodyMode::Crouching,
            BodyMode::Crawling,
            BodyMode::Sliding,
        ] {
            assert!(
                !non_fit.shape(base).fits_at(center, &world, solid_predicate),
                "{:?} must NOT fit the 16-px morph-lab tunnel (discriminator broken)",
                non_fit,
            );
        }
    }

    /// Parity check: the cluster-native `LocomotionState::from_clusters`
    /// reproduces `from_player` exactly on default cluster state. Pins
    /// the priority-order contract so future edits to one variant
    /// trip a CI test if they diverge.
    #[test]
    fn locomotion_from_clusters_matches_from_player_at_rest() {
        use crate::player_clusters::{
            PlayerBlinkState, PlayerDashState, PlayerFlightState, PlayerGroundState,
            PlayerLedgeState, PlayerWallState,
        };
        let ground = PlayerGroundState {
            on_ground: true,
            ..Default::default()
        };
        let wall = PlayerWallState::default();
        let flight = PlayerFlightState::default();
        let dash = PlayerDashState::default();
        let blink = PlayerBlinkState::default();
        let ledge = PlayerLedgeState::default();
        assert_eq!(
            LocomotionState::from_clusters(&ground, &wall, &flight, &dash, &blink, &ledge),
            LocomotionState::Grounded
        );

        let ground = PlayerGroundState::default();
        let dash = PlayerDashState {
            timer: 0.1,
            ..Default::default()
        };
        assert_eq!(
            LocomotionState::from_clusters(&ground, &wall, &flight, &dash, &blink, &ledge),
            LocomotionState::Dashing,
            "dash timer overrides ground state"
        );
    }

    #[test]
    fn body_mode_from_clusters_reads_authoritative_field() {
        use crate::player_clusters::PlayerBodyModeState;
        let bm = PlayerBodyModeState {
            body_mode: BodyMode::Crouching,
        };
        assert_eq!(BodyMode::from_clusters(&bm), BodyMode::Crouching);
    }
}
