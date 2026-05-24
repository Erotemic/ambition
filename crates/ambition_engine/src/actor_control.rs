//! Actor control-frame seam — the unified brain→sim contract.
//!
//! Every controllable entity in Ambition — players, enemies, bosses,
//! and (in the future) other NPC actors — funnels through a single
//! per-tick struct: an [`ActorControlFrame`]. A *brain* (the policy
//! choosing what the actor does this tick) writes into the frame; the
//! *simulation* half (gravity + [`crate::step_kinematic`] + cooldowns
//! + effects) reads from the frame.
//!
//! Brains today are hand-written:
//!
//! - Player: input → ControlFrame (still being unified — see project
//!   notes on the player-as-actor migration).
//! - Enemy: [`crate::CharacterAiSnapshot`] + [`crate::AttackChoreography`]
//!   → frame.
//! - Boss: [`crate::BossBrain`] + scripted pattern → frame.
//!
//! Brains tomorrow can be neural networks, replay buffers, remote
//! players, or scripted demos — same frame, same integration. The
//! ambition is that you can put any brain on any actor: the player
//! could control a goblin, a second player could control a boss, an
//! RL policy could drive a skitter, all without touching collision
//! code.
//!
//! Design pillars:
//!
//! 1. **Brains output desired motion in velocity-space**, not
//!    position-space. Anything that wrote `self.pos += vel * dt`
//!    bypassed wall collision; routing that desired velocity through
//!    [`crate::step_kinematic`] (with `gravity = 0` for fliers) makes
//!    every actor collide through the same code path.
//!
//! 2. **Brains are pure functions of a snapshot**. A future RL agent
//!    that wants to plug in here gets the same snapshot a scripted
//!    brain gets.
//!
//! 3. **The integration is brain-agnostic**. Velocity, attack intent,
//!    facing, and projectile fire all live on the control frame.
//!    Whether the brain came from a hand-authored choreography, a
//!    learned policy, or a remote player doesn't change the
//!    integration code.

use crate::Vec2;

/// A request from a brain to fire a projectile this tick. Mirrors the
/// existing `ChoreographyAction::FireProjectile { dir, speed }` but
/// lives on the control-frame side of the seam so a future
/// non-choreography brain (RL policy, dialogue-scripted skirmish,
/// remote player) can emit one without going through the
/// choreography path.
///
/// **Future narrowing:** The `speed` field is on track to disappear
/// once the EFFECTS-stage resolver reads the actor's
/// `ActionSet::ranged.speed()` instead — brains will only emit a
/// direction. New brain backends should set `speed = 0.0` as a
/// sentinel; callers that read speed today still consult this
/// field as the source of truth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorFireRequest {
    /// Launch direction (unit vector recommended; the sandbox
    /// projectile spawner normalizes anyway).
    pub dir: Vec2,
    /// Launch speed in px/s.
    pub speed: f32,
}

/// Per-tick movement + action intent from an actor brain. The same
/// role the player's input ControlFrame plays for the player
/// character: a flat struct of "what would you like to happen this
/// tick", where the simulation half decides what's actually possible
/// given collision, cooldowns, and world rules.
///
/// Construction goes through [`ActorControlFrame::neutral`] (or
/// `Default`) so adding a new field doesn't churn every caller.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActorControlFrame {
    /// Desired velocity in px/s. The simulation either uses this
    /// directly (aerial / kinematic / boss) or treats `desired_vel.x`
    /// as an axis intent and lets gravity own vy (grounded).
    pub desired_vel: Vec2,
    /// Suppress the OneWay vertical block this tick so the body
    /// falls through the platform it is standing on. Mirrors the
    /// player's `drop_through_pressed`.
    pub drop_through: bool,
    /// Desired facing this tick. `+1.0` = right, `-1.0` = left,
    /// `0.0` = leave the actor's existing facing alone.
    pub facing: f32,
    /// Brain wants to begin a melee attack windup this tick. The
    /// simulation half handles cooldown gating; the brain just
    /// signals intent.
    pub melee_pressed: bool,
    /// Brain wants to fire a projectile this tick. `Some` carries the
    /// launch direction + speed; `None` is "no shot".
    pub fire: Option<ActorFireRequest>,
    /// Direction-of-attack for melee. Zero = "use the actor's current
    /// facing". A non-zero vector lets the ActionSet pick between
    /// directional variants (up-tilt, down-air, back-air, …). Brains
    /// that don't care about directional melee leave this zero.
    pub attack_axis: Vec2,
    /// Rising edge: brain wants to jump this tick.
    pub jump_pressed: bool,
    /// Sustain: jump button is currently held. Used by variable-
    /// height jump integration to keep applying upward force while
    /// the button is held during the rising phase.
    pub jump_held: bool,
    /// Falling edge: jump button was released this tick. Some
    /// integrations cap upward velocity on release to make short
    /// taps feel responsive.
    pub jump_released: bool,
    /// Rising edge: brain wants to dash this tick. The simulation
    /// half handles cooldowns and direction selection.
    pub dash_pressed: bool,
    /// Rising edge: brain wants to interact with whatever is nearby
    /// (doors, NPCs, switches). E / F / RB on player binding; AI
    /// brains may toggle this for scripted door-opens or NPC chats.
    pub interact_pressed: bool,
    /// Sustain: shield / parry button is held. Brains that want a
    /// bubble shield up keep this true; release triggers shield-
    /// down behavior in the integration.
    pub shield_held: bool,
    /// Rising edge: brain wants to use its special / signature move.
    /// What this resolves to is per-entity (ActionSet), so the same
    /// `special_pressed=true` from a player brain and a possessed
    /// goblin yield different concrete effects.
    pub special_pressed: bool,
}

impl ActorControlFrame {
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
        let frame = ActorControlFrame::default();
        assert_eq!(frame.desired_vel, Vec2::ZERO);
        assert!(!frame.drop_through);
        assert_eq!(frame.facing, 0.0);
        assert!(!frame.melee_pressed);
        assert!(frame.fire.is_none());
        assert_eq!(frame.attack_axis, Vec2::ZERO);
        assert!(!frame.jump_pressed);
        assert!(!frame.jump_held);
        assert!(!frame.jump_released);
        assert!(!frame.dash_pressed);
        assert!(!frame.interact_pressed);
        assert!(!frame.shield_held);
        assert!(!frame.special_pressed);
    }

    #[test]
    fn neutral_matches_default() {
        assert_eq!(ActorControlFrame::neutral(), ActorControlFrame::default());
    }

    #[test]
    fn extended_frame_defaults_are_inert() {
        // Brain backends are free to set only the fields they care
        // about; every other field must default to a value the
        // integration treats as "no intent".
        let frame = ActorControlFrame::neutral();
        let unchanged = ActorControlFrame {
            attack_axis: frame.attack_axis,
            jump_pressed: frame.jump_pressed,
            jump_held: frame.jump_held,
            jump_released: frame.jump_released,
            dash_pressed: frame.dash_pressed,
            interact_pressed: frame.interact_pressed,
            shield_held: frame.shield_held,
            special_pressed: frame.special_pressed,
            ..Default::default()
        };
        assert_eq!(frame, unchanged);
    }
}
