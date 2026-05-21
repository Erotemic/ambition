//! Enemy control-frame seam.
//!
//! Players read [`crate::ControlFrame`]-shaped inputs (axis_x, axis_y,
//! jump_pressed, attack_pressed, …) and the player simulation
//! integrates them through the kinematic + ability stack. Enemies
//! have grown an equivalent shape: a brain (today scripted, tomorrow
//! a neural net or replay) produces an [`EnemyControlFrame`] each
//! tick, and the enemy simulation half integrates that through the
//! shared [`crate::step_kinematic`] primitive so EVERY actor —
//! grounded, aerial, or path-driven — respects collision through the
//! same code path.
//!
//! Design pillars:
//!
//! 1. **Brains output desired motion in velocity-space**, not
//!    position-space. Aerial enemies that previously wrote
//!    `self.pos += vel * dt` directly bypassed wall collision; routing
//!    that desired velocity through [`crate::step_kinematic`]
//!    (with `gravity = 0` for fliers) makes them collide naturally.
//!
//! 2. **Brains are pure functions of a snapshot**, not stateful
//!    actors that read the world in odd places. A future RL agent
//!    that wants to plug in here gets the same snapshot a scripted
//!    brain gets.
//!
//! 3. **The integration is brain-agnostic**. Velocity, attack
//!    intent, and facing all live on the control frame. Whether the
//!    brain came from a hand-authored choreography or a learned
//!    policy doesn't change the integration code.

use crate::Vec2;

/// A request from a brain to fire a projectile this tick. Mirrors
/// the existing `ChoreographyAction::FireProjectile { dir, speed }`
/// but lives on the control-frame side of the seam so a future
/// non-choreography brain (RL policy, dialogue-scripted skirmish,
/// etc.) can emit one without going through the choreography path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyFireRequest {
    /// Launch direction (unit vector recommended; the sandbox
    /// projectile spawner normalizes anyway).
    pub dir: Vec2,
    /// Launch speed in px/s.
    pub speed: f32,
}

/// Per-tick movement + action intent from an enemy brain. Same role
/// the player's `ControlFrame` plays for the player character: a
/// flat struct of "what would you like to happen this tick", where
/// the simulation half decides what's actually possible given
/// collision, cooldowns, and world rules.
///
/// Construction goes through [`EnemyControlFrame::neutral`] (or
/// `Default`) so adding a new field doesn't churn every caller.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnemyControlFrame {
    /// Desired velocity in px/s. The simulation either uses this
    /// directly (aerial / kinematic) or treats `desired_vel.x` as
    /// an axis intent and lets gravity own vy (grounded).
    pub desired_vel: Vec2,
    /// Suppress the OneWay vertical block this tick so the body
    /// falls through the platform it is standing on. Mirrors the
    /// player's `drop_through_pressed`.
    pub drop_through: bool,
    /// Desired facing this tick. `+1.0` = right, `-1.0` = left,
    /// `0.0` = leave the actor's existing facing alone.
    pub facing: f32,
    /// Brain wants to begin a melee attack windup this tick.
    /// The simulation half handles cooldown gating; the brain just
    /// signals intent.
    pub melee_pressed: bool,
    /// Brain wants to fire a projectile this tick. `Some` carries
    /// the launch direction + speed; `None` is "no shot".
    pub fire: Option<EnemyFireRequest>,
}

impl EnemyControlFrame {
    /// Empty / idle frame — no movement, no actions, hold current
    /// facing. Useful starting point for brains that conditionally
    /// fill fields, and for sandbox tests that want a known-stable
    /// baseline.
    pub fn neutral() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_frame_is_neutral() {
        let frame = EnemyControlFrame::default();
        assert_eq!(frame.desired_vel, Vec2::ZERO);
        assert!(!frame.drop_through);
        assert_eq!(frame.facing, 0.0);
        assert!(!frame.melee_pressed);
        assert!(frame.fire.is_none());
    }

    #[test]
    fn neutral_matches_default() {
        assert_eq!(EnemyControlFrame::neutral(), EnemyControlFrame::default());
    }
}
