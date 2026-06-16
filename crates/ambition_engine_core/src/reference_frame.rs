//! The gravity-relative reference frame and the transforms between Ambition's
//! three frames.
//!
//! Ambition reasons about three reference frames; the bugs in this area
//! (attack direction, pogo, sprite orientation, facing flip) were all "someone
//! forgot to transform between two of them". [`AccelerationFrame`] makes the frames
//! and the transforms explicit so being gravity-aware is "you hold a
//! `AccelerationFrame`", not "you remembered to multiply by `gravity_dir`".
//!
//! - **Input frame** — the controller: `axis_x` right-positive, `axis_y`
//!   screen-down-positive. Raw, never rotated.
//! - **Player frame** — relative to the player: `+x` is the run / side axis,
//!   `+y` is *toward the feet* (the player's own "down"). Combat geometry,
//!   impulses, and gates are authored here, in the upright (normal-gravity) pose.
//! - **World frame** — engine coordinates (`+y` screen-down).
//!
//! Under normal gravity the player frame *equals* the world frame, so every
//! transform below is the identity and play is byte-identical.

use crate::Vec2;

/// How the INPUT frame maps onto the player frame — "which way is right when
/// gravity is sideways or upside-down". A control preference, configurable per
/// player (see [`AccelerationFrame::control_frame`]).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize,
)]
pub enum InputFrameMode {
    /// Input is always SCREEN-aligned: right is screen-right regardless of
    /// gravity (the player mentally rotates).
    Screen,
    /// Input always follows the PLAYER frame: right is the player's own right,
    /// fully rotated with gravity (no accommodation).
    Player,
    /// Default HYBRID (Jon's gut-feel): follow the player frame up to ±90° from
    /// screen-down — gravity down / left / right, where a human tracks the
    /// rotation fine — then revert to screen-aligned past 90° (gravity up-ish),
    /// where the flip is hard to map. The vertical "descend" gate (pogo / crouch)
    /// is independent and always flips with the player frame ([`Self::descend`]).
    #[default]
    Hybrid,
}

/// The player's reference frame under a net "down"-defining acceleration.
///
/// The common source of `down` is gravity, but the frame is deliberately not
/// gravity-specific (hence the name): any net proper acceleration — a force
/// field, thrust, a spinning room — defines the player's local "down", and the
/// frame transforms the same way. The direction is also NOT snapped to a
/// cardinal, so an off-axis / rotating "down" works (the transforms are general
/// rotations); the gravity system happens to feed cardinal directions today.
///
/// `down` (toward the feet, a unit vector) and `side` (the perpendicular run
/// axis) are the player frame's basis expressed in world coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccelerationFrame {
    /// Toward the feet (unit) — the player's own "down". `(0,1)` under normal
    /// gravity.
    pub down: Vec2,
    /// The run / side axis (perpendicular to `down`). `(1,0)` under normal gravity.
    pub side: Vec2,
}

impl AccelerationFrame {
    /// Build the frame from the net "down"-defining acceleration (gravity is the
    /// usual source). The direction is normalized but NOT cardinal-snapped, so an
    /// arbitrary-angle `down` is supported. The side axis is `down` rotated −90°,
    /// so under normal gravity `down=(0,1)`, `side=(1,0)` and every transform is
    /// the identity. A zero acceleration defaults to normal-gravity down.
    pub fn new(acceleration: Vec2) -> Self {
        let down = acceleration.try_normalize().unwrap_or(Vec2::new(0.0, 1.0));
        Self {
            down,
            side: Vec2::new(down.y, -down.x),
        }
    }

    /// The frame the INPUT stick maps through, per the player's [`InputFrameMode`].
    /// `to_world(stick)` on the result turns raw `(axis_x, axis_y)` into a
    /// world-space movement direction. `Screen` → identity; `Player` → this frame;
    /// `Hybrid` → this frame up to ±90° (down.y ≥ 0), else screen-aligned. NOTE:
    /// this drives free MOVEMENT (run / flight); the toward-feet GATE uses
    /// [`Self::descend`] directly so pogo/crouch always flip with the player.
    pub fn control_frame(self, mode: InputFrameMode) -> AccelerationFrame {
        let screen = AccelerationFrame::new(Vec2::new(0.0, 1.0));
        match mode {
            InputFrameMode::Screen => screen,
            InputFrameMode::Player => self,
            InputFrameMode::Hybrid => {
                if self.down.y >= 0.0 {
                    self
                } else {
                    screen
                }
            }
        }
    }

    /// INPUT → PLAYER. Screen-vertical input (`axis_y`, +Y screen-down) → the
    /// "descend" (toward-feet) intent that gates crouch / pogo / drop-through /
    /// fast-fall. The accommodation: the gate stays on the up/down keys and only
    /// flips sign once gravity rotates PAST ±90° from screen-down (i.e. gravity
    /// points up-ish). Identity under normal gravity.
    pub fn descend(self, input_axis_y: f32) -> f32 {
        input_axis_y * if self.down.y < 0.0 { -1.0 } else { 1.0 }
    }

    /// PLAYER → WORLD. Rotate a player-frame vector (authored with `+y` toward the
    /// feet) into world coordinates. Identity under normal gravity.
    pub fn to_world(self, player: Vec2) -> Vec2 {
        self.side * player.x + self.down * player.y
    }

    /// PLAYER → WORLD for an axis-aligned half-extent. Returns the world-space
    /// AABB half-extent that BOUNDS the rotated box: exact for cardinal frames
    /// (90° just swaps width/height), an over-approximation for off-axis frames
    /// (the bound of the tilted rectangle). Identity under normal / inverted
    /// gravity.
    pub fn to_world_half(self, half: Vec2) -> Vec2 {
        Vec2::new(
            (self.side.x * half.x).abs() + (self.down.x * half.y).abs(),
            (self.side.y * half.x).abs() + (self.down.y * half.y).abs(),
        )
    }

    /// Set `vel` to a launch of `speed` AWAY from the feet (jump / pogo bounce),
    /// preserving the component perpendicular to gravity.
    pub fn launch(self, vel: &mut Vec2, speed: f32) {
        let perp = *vel - vel.dot(self.down) * self.down;
        *vel = perp - speed * self.down;
    }

    /// The component of `vel` directed toward the feet (its descent speed).
    pub fn descend_speed(self, vel: Vec2) -> f32 {
        vel.dot(self.down)
    }

    /// Force `vel`'s toward-feet component to at least `speed` (used to "commit"
    /// a down-attack), leaving the perpendicular component alone.
    pub fn ensure_descend_speed(self, vel: &mut Vec2, speed: f32) {
        let cur = vel.dot(self.down);
        if cur < speed {
            *vel += self.down * (speed - cur);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_gravity_is_identity() {
        let f = AccelerationFrame::new(Vec2::new(0.0, 1.0));
        assert_eq!(f.down, Vec2::new(0.0, 1.0));
        assert_eq!(f.side, Vec2::new(1.0, 0.0));
        assert_eq!(f.descend(0.7), 0.7);
        assert_eq!(f.to_world(Vec2::new(3.0, 5.0)), Vec2::new(3.0, 5.0));
        assert_eq!(f.to_world_half(Vec2::new(26.0, 34.0)), Vec2::new(26.0, 34.0));
    }

    #[test]
    fn inverted_gravity_flips_descend_and_vertical() {
        let f = AccelerationFrame::new(Vec2::new(0.0, -1.0));
        // Holding screen-up (axis_y = -1) is "descend" (toward the up-feet).
        assert_eq!(f.descend(-1.0), 1.0);
        // A player-frame "toward feet" offset (+y) maps to screen-up.
        assert_eq!(f.to_world(Vec2::new(0.0, 32.0)), Vec2::new(0.0, -32.0));
        // Vertical half-extent unchanged (still a 180° frame, no axis swap).
        assert_eq!(f.to_world_half(Vec2::new(26.0, 34.0)), Vec2::new(26.0, 34.0));
    }

    #[test]
    fn sideways_gravity_swaps_axes() {
        let f = AccelerationFrame::new(Vec2::new(1.0, 0.0)); // gravity points screen-right
        assert_eq!(f.down, Vec2::new(1.0, 0.0));
        // Toward-feet (+y player) maps to screen-right.
        assert_eq!(f.to_world(Vec2::new(0.0, 32.0)), Vec2::new(32.0, 0.0));
        // A wide-thin down-attack box becomes thin-wide in world.
        assert_eq!(f.to_world_half(Vec2::new(26.0, 34.0)), Vec2::new(34.0, 26.0));
    }

    #[test]
    fn off_axis_down_is_a_general_rotation() {
        // A 45° "down" (toward screen down-right) is not snapped — the frame is a
        // real rotation, so toward-feet maps along the diagonal.
        let f = AccelerationFrame::new(Vec2::new(1.0, 1.0));
        let inv_sqrt2 = 1.0 / 2.0_f32.sqrt();
        assert!((f.down - Vec2::new(inv_sqrt2, inv_sqrt2)).length() < 1e-6);
        let feet = f.to_world(Vec2::new(0.0, 10.0));
        assert!((feet - Vec2::new(10.0 * inv_sqrt2, 10.0 * inv_sqrt2)).length() < 1e-5);
    }

    #[test]
    fn hybrid_control_frame_rotates_to_90_then_reverts() {
        // Right gravity (≤90°): the control frame follows the player, so "right"
        // on the stick maps to screen-up (the player's right).
        let right = AccelerationFrame::new(Vec2::new(1.0, 0.0));
        let cf = right.control_frame(InputFrameMode::Hybrid);
        let world = cf.to_world(Vec2::new(1.0, 0.0));
        assert!((world - Vec2::new(0.0, -1.0)).length() < 1e-6, "{world:?}");
        // Up gravity (>90°): the control frame reverts to screen, so "right" maps
        // to screen-right (= the player's left — the accommodation).
        let up = AccelerationFrame::new(Vec2::new(0.0, -1.0));
        let cf = up.control_frame(InputFrameMode::Hybrid);
        assert_eq!(cf.to_world(Vec2::new(1.0, 0.0)), Vec2::new(1.0, 0.0));
        // Player mode never reverts; Screen mode never rotates.
        assert_eq!(up.control_frame(InputFrameMode::Player), up);
        assert_eq!(
            up.control_frame(InputFrameMode::Screen).down,
            Vec2::new(0.0, 1.0)
        );
    }

    #[test]
    fn launch_is_away_from_feet() {
        let f = AccelerationFrame::new(Vec2::new(0.0, -1.0)); // up gravity
        let mut v = Vec2::new(5.0, 0.0);
        f.launch(&mut v, 600.0);
        // Away from up-feet = screen-down (+y); perpendicular x preserved.
        assert_eq!(v, Vec2::new(5.0, 600.0));
    }
}
