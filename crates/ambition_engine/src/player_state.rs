//! Reusable player-state vocabulary.
//!
//! Adds three high-leverage primitives identified in
//! `docs/mechanics_checklist.md` Tier 1:
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

use crate::movement::Player;
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

    /// Best-effort projection from the existing `Player` struct.
    /// Mechanics that own dedicated state can override by writing the
    /// resource directly; this only inspects fields that already exist.
    pub fn from_player(player: &Player) -> Self {
        if player.dash_timer > 0.0 {
            return LocomotionState::Dashing;
        }
        if player.blink_aiming {
            return LocomotionState::BlinkAiming;
        }
        if player.fly_enabled {
            return LocomotionState::Flying;
        }
        if player.wall_climbing {
            return LocomotionState::WallClimb;
        }
        if player.wall_clinging {
            return LocomotionState::WallCling;
        }
        if player.on_wall && !player.on_ground {
            return LocomotionState::WallSlide;
        }
        if player.fast_falling {
            return LocomotionState::FastFalling;
        }
        if player.on_ground {
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
}

impl BodyMode {
    pub fn label(self) -> &'static str {
        match self {
            BodyMode::Standing => "Standing",
            BodyMode::Crouching => "Crouching",
            BodyMode::Crawling => "Crawling",
            BodyMode::Sliding => "Sliding",
            BodyMode::MorphBall => "MorphBall",
        }
    }

    /// Read the player's authoritative `body_mode` field. Sandbox systems
    /// that drive crouch / morph / slide should write `player.body_mode`
    /// directly; gameplay reads (HUD, trace, AI) should call this so
    /// there is exactly one source of truth.
    pub fn from_player(player: &Player) -> Self {
        player.body_mode
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
            BodyMode::MorphBall => BodyShape {
                mode: self,
                // Symmetric, much smaller. Suitable for low tunnels.
                size: Vec2::new(base_size.x * 0.55, base_size.x * 0.55),
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
pub fn try_change_body_mode<F>(
    player: &mut crate::movement::Player,
    new_mode: BodyMode,
    world: &crate::world::World,
    predicate: F,
) -> bool
where
    F: FnMut(&crate::world::Block) -> bool,
{
    if player.body_mode == new_mode {
        return true;
    }
    let new_shape = new_mode.shape(player.base_size);
    let dy = (player.size.y - new_shape.size.y) * 0.5;
    let new_center = Vec2::new(player.pos.x, player.pos.y + dy);
    if !new_shape.fits_at(new_center, world, predicate) {
        return false;
    }
    player.pos = new_center;
    player.size = new_shape.size;
    player.body_mode = new_mode;
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
pub fn classify_player_safety<F>(
    player: &crate::movement::Player,
    world: &crate::world::World,
    margin: f32,
    mut solid_predicate: F,
) -> PlayerSafetyVerdict
where
    F: FnMut(&crate::world::Block) -> bool,
{
    if !player.pos.x.is_finite() || !player.pos.y.is_finite() {
        return PlayerSafetyVerdict::PositionNonFinite;
    }
    if !player.vel.x.is_finite() || !player.vel.y.is_finite() {
        return PlayerSafetyVerdict::VelocityNonFinite;
    }
    let aabb = player.aabb();
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
    use crate::world::{Block, World};

    #[test]
    fn locomotion_default_grounded_when_player_on_ground() {
        let mut p = Player::new(Vec2::new(0.0, 0.0));
        p.on_ground = true;
        assert_eq!(LocomotionState::from_player(&p), LocomotionState::Grounded);
    }

    #[test]
    fn locomotion_airborne_when_off_ground() {
        let p = Player::new(Vec2::new(0.0, 0.0));
        assert_eq!(LocomotionState::from_player(&p), LocomotionState::Airborne);
    }

    #[test]
    fn locomotion_dashing_overrides_other_states() {
        let mut p = Player::new(Vec2::new(0.0, 0.0));
        p.on_ground = true;
        p.dash_timer = 0.10;
        assert_eq!(LocomotionState::from_player(&p), LocomotionState::Dashing);
    }

    #[test]
    fn locomotion_blink_aiming_recognized() {
        let mut p = Player::new(Vec2::new(0.0, 0.0));
        p.blink_aiming = true;
        assert_eq!(
            LocomotionState::from_player(&p),
            LocomotionState::BlinkAiming
        );
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
        let mut player = Player::new(Vec2::new(100.0, 100.0));
        let original_size = player.size;
        let original_feet = player.pos.y + player.size.y * 0.5;

        let ok = try_change_body_mode(&mut player, BodyMode::Crouching, &world, |_| true);
        assert!(ok);
        assert_eq!(player.body_mode, BodyMode::Crouching);
        assert!(player.size.y < original_size.y);
        assert_eq!(player.size.x, original_size.x);
        let new_feet = player.pos.y + player.size.y * 0.5;
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
        let mut player = Player::new(Vec2::new(100.0, 100.0));
        let base = player.base_size;
        try_change_body_mode(&mut player, BodyMode::Crouching, &world, |_| true);
        try_change_body_mode(&mut player, BodyMode::Crouching, &world, |_| true);
        let ok = try_change_body_mode(&mut player, BodyMode::Standing, &world, |_| true);
        assert!(ok);
        assert_eq!(player.body_mode, BodyMode::Standing);
        assert_eq!(player.size, base);
    }

    #[test]
    fn try_change_body_mode_blocked_stand_up_under_low_ceiling() {
        // Authoring: ceiling block whose bottom is one crouch-height
        // above the player's feet so a crouching body fits but a
        // standing body would clip the ceiling on the way up.
        let player_spawn = Vec2::new(100.0, 100.0);
        let mut player = Player::new(player_spawn);
        let standing_top = player.pos.y - player.size.y * 0.5; // = 100 - 23 = 77

        // Place a ceiling whose bottom is at y == standing_top + 5,
        // i.e. just below where the standing body's top would be.
        let ceiling_bottom = standing_top + 5.0; // 82
        let ceiling_top = ceiling_bottom - 30.0; // 52
        let world = World::new(
            "test",
            Vec2::new(400.0, 400.0),
            Vec2::new(50.0, 50.0),
            vec![Block::solid(
                "ceiling",
                Vec2::new(player.pos.x - 50.0, ceiling_top),
                Vec2::new(100.0, 30.0),
            )],
        );

        // Crouching first must fit (ceiling is above the crouched body).
        let ok = try_change_body_mode(&mut player, BodyMode::Crouching, &world, |b| {
            matches!(b.kind, crate::world::BlockKind::Solid)
        });
        assert!(ok);

        // Stand-up must be rejected because the standing body would
        // overlap the ceiling.
        let stand_attempt = try_change_body_mode(&mut player, BodyMode::Standing, &world, |b| {
            matches!(b.kind, crate::world::BlockKind::Solid)
        });
        assert!(!stand_attempt);
        // State unchanged.
        assert_eq!(player.body_mode, BodyMode::Crouching);
    }

    #[test]
    fn try_change_body_mode_to_same_mode_is_no_op_success() {
        let world = World::new(
            "test",
            Vec2::new(400.0, 400.0),
            Vec2::new(50.0, 50.0),
            Vec::new(),
        );
        let mut player = Player::new(Vec2::new(100.0, 100.0));
        let pos_before = player.pos;
        let size_before = player.size;
        let ok = try_change_body_mode(&mut player, BodyMode::Standing, &world, |_| true);
        assert!(ok);
        assert_eq!(player.pos, pos_before);
        assert_eq!(player.size, size_before);
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
}
