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
//! - Player: sandbox `Brain::Player` translates per-player input into
//!   this frame, and the player control/simulation phases consume it.
//! - NPC/enemy: sandbox state-machine brains translate snapshots into
//!   desired motion and action edges; ActionSets resolve concrete effects.
//! - Boss: sandbox `BossPattern` brains and authored profiles emit
//!   movement/action frames for encounter-specific consumers.
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

use crate::engine_core::Vec2;

/// A request from a brain to fire a projectile this tick. Mirrors the
/// existing `ChoreographyAction::FireProjectile { dir, speed }` but
/// lives on the control-frame side of the seam so a future
/// non-choreography brain (RL policy, dialogue-scripted skirmish,
/// remote player) can emit one without going through the
/// choreography path.
///
/// **Current convention:** ActionSet-driven consumers read projectile
/// speed from the resolved `RangedActionSpec`; player/projectile legacy
/// paths may still carry `speed` here while migration is in progress.
/// New brain backends should prefer `speed = 0.0` and let the actor's
/// capability choose the launch speed.
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
    /// True when this actor's body should act as a contact hazard
    /// this tick. Default false so human-controlled bodies do not
    /// accidentally damage nearby enemies just by moving through
    /// them. Hostile AI can opt in explicitly when the body itself
    /// is supposed to be dangerous.
    pub body_contact_damage_enabled: bool,
    /// Sustain: shield / parry button is held. Brains that want a
    /// bubble shield up keep this true; release triggers shield-
    /// down behavior in the integration.
    pub shield_held: bool,
    /// Rising edge: brain wants to use its special / signature move.
    /// What this resolves to is per-entity (ActionSet), so the same
    /// `special_pressed=true` from a player brain and a possessed
    /// goblin yield different concrete effects.
    pub special_pressed: bool,
    /// Rising edge: brain wants to trigger a pogo bounce this tick.
    /// Today only the human player binds a verb here (the dedicated
    /// pogo input + attack+down combo); AI brains leave it false.
    /// Promoted onto the frame so the sandbox's player polarity flip
    /// can drop its raw `ControlFrame` dependency.
    pub pogo_pressed: bool,
    /// Rising edge: brain wants to enter / refresh fast-fall this
    /// tick (player-side dedicated input; AI brains ignore today).
    pub fast_fall_pressed: bool,
    /// Rising edge: brain wants to toggle fly mode (player-side
    /// dev/movement verb today).
    pub fly_toggle_pressed: bool,
    /// Rising edge: brain wants to start charging a projectile (player-
    /// side fireball/hadouken; the integration owns the charge state
    /// machine). When the charge releases, `fire = Some(...)` carries
    /// the resolved direction.
    pub projectile_pressed: bool,
    /// Sustain: charge button held this tick. Mirror of the player's
    /// projectile-held input; integration uses it to grow the charge
    /// preview.
    pub projectile_held: bool,
    /// Falling edge: charge button released — the integration spawns
    /// the projectile. `fire` carries the launch direction.
    pub projectile_released: bool,
    /// Rising edge: brain wants to initiate a blink/teleport
    /// (player-side signature ability; today translated from raw
    /// `blink_pressed`).
    pub blink_pressed: bool,
    /// Sustain: blink-aim input held — the player's precision-blink
    /// path uses this during aiming.
    pub blink_held: bool,
    /// Falling edge: blink released — commit the blink target.
    pub blink_released: bool,
    /// Aim direction for charged ranged attacks. `(0,0)` = use
    /// actor's facing; non-zero = explicit twin-stick / mouse aim
    /// vector. Mirror of the player's `(aim_x, aim_y)`.
    pub aim: Vec2,
}

impl ActorControlFrame {
    /// Empty / idle frame — no movement, no actions, hold current
    /// facing. Useful starting point for brains that conditionally
    /// fill fields, and for sandbox tests that want a known-stable
    /// baseline.
    pub fn neutral() -> Self {
        Self::default()
    }

    /// True iff any action verb (melee / fire / jump / dash /
    /// interact / shield / special) is requested this tick. Useful
    /// for debug HUD ("brain is asking for something"), perf
    /// counters, and trace recording predicates.
    pub fn wants_any_action(&self) -> bool {
        self.melee_pressed
            || self.fire.is_some()
            || self.jump_pressed
            || self.jump_held
            || self.dash_pressed
            || self.interact_pressed
            || self.shield_held
            || self.special_pressed
    }

    /// Clear all rising-edge flags (pressed / released) without
    /// touching sustains. Used by integrations that consume the
    /// frame in multiple stages and need to prevent a single edge
    /// from re-firing across stages.
    pub fn clear_edges(&mut self) {
        self.jump_pressed = false;
        self.jump_released = false;
        self.dash_pressed = false;
        self.interact_pressed = false;
        self.special_pressed = false;
        self.melee_pressed = false;
        self.fire = None;
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
        assert!(!frame.body_contact_damage_enabled);
        assert!(!frame.shield_held);
        assert!(!frame.special_pressed);
    }

    #[test]
    fn neutral_matches_default() {
        assert_eq!(ActorControlFrame::neutral(), ActorControlFrame::default());
    }

    #[test]
    fn wants_any_action_reports_false_for_neutral_frame() {
        let frame = ActorControlFrame::neutral();
        assert!(!frame.wants_any_action());
    }

    #[test]
    fn frames_differing_in_any_new_field_are_not_equal() {
        // PartialEq must cover every field. A future field added
        // to ActorControlFrame whose derive omits the field would
        // silently break frame equality checks. Pin that adding
        // each new field changes equality.
        let baseline = ActorControlFrame::neutral();
        let mut a = baseline;
        a.attack_axis = Vec2::new(1.0, 0.0);
        assert_ne!(baseline, a);
        let mut b = baseline;
        b.jump_pressed = true;
        assert_ne!(baseline, b);
        let mut c = baseline;
        c.dash_pressed = true;
        assert_ne!(baseline, c);
        let mut d = baseline;
        d.interact_pressed = true;
        assert_ne!(baseline, d);
        let mut e = baseline;
        e.shield_held = true;
        assert_ne!(baseline, e);
        let mut f = baseline;
        f.special_pressed = true;
        assert_ne!(baseline, f);
        let mut g = baseline;
        g.jump_held = true;
        assert_ne!(baseline, g, "jump_held should be in PartialEq");
        let mut h = baseline;
        h.jump_released = true;
        assert_ne!(baseline, h, "jump_released should be in PartialEq");
    }

    #[test]
    fn clear_edges_zeros_per_tick_edges_keeps_sustains() {
        let mut frame = ActorControlFrame::neutral();
        frame.jump_pressed = true;
        frame.jump_held = true;
        frame.jump_released = true;
        frame.dash_pressed = true;
        frame.interact_pressed = true;
        frame.special_pressed = true;
        frame.melee_pressed = true;
        frame.shield_held = true;
        frame.fire = Some(ActorFireRequest {
            dir: Vec2::new(1.0, 0.0),
            speed: 0.0,
        });
        // Also set a sustain that should NOT clear: jump_held + shield_held.
        frame.clear_edges();
        assert!(!frame.jump_pressed);
        assert!(!frame.jump_released);
        assert!(!frame.dash_pressed);
        assert!(!frame.interact_pressed);
        assert!(!frame.special_pressed);
        assert!(!frame.melee_pressed);
        assert!(frame.fire.is_none());
        // Sustains preserved.
        assert!(frame.jump_held);
        assert!(frame.shield_held);
    }

    #[test]
    fn wants_any_action_reports_true_when_any_verb_is_set() {
        let mut frame = ActorControlFrame::neutral();
        frame.melee_pressed = true;
        assert!(frame.wants_any_action());
        let mut frame = ActorControlFrame::neutral();
        frame.jump_pressed = true;
        assert!(frame.wants_any_action(), "jump_pressed should count");
        let mut frame = ActorControlFrame::neutral();
        frame.jump_held = true;
        assert!(frame.wants_any_action());
        let mut frame = ActorControlFrame::neutral();
        frame.fire = Some(ActorFireRequest {
            dir: Vec2::new(1.0, 0.0),
            speed: 0.0,
        });
        assert!(frame.wants_any_action());
        let mut frame = ActorControlFrame::neutral();
        frame.dash_pressed = true;
        assert!(frame.wants_any_action(), "dash_pressed should count");
        let mut frame = ActorControlFrame::neutral();
        frame.interact_pressed = true;
        assert!(frame.wants_any_action(), "interact_pressed should count");
        let mut frame = ActorControlFrame::neutral();
        frame.shield_held = true;
        assert!(frame.wants_any_action());
        let mut frame = ActorControlFrame::neutral();
        frame.special_pressed = true;
        assert!(frame.wants_any_action());
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
            body_contact_damage_enabled: frame.body_contact_damage_enabled,
            shield_held: frame.shield_held,
            special_pressed: frame.special_pressed,
            ..Default::default()
        };
        assert_eq!(frame, unchanged);
    }
}
