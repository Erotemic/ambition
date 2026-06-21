//! Actor control-frame seam: the unified brain→simulation contract.
//!
//! Every controllable entity writes one [`ActorControlFrame`] per tick. Brains
//! choose desired velocity, facing, and action edges; the simulation decides what
//! is physically possible given collision, cooldowns, and world rules.
//!
//! The contract is intentionally brain-agnostic: hand-authored AI, player input,
//! replay, remote control, and future learned policies can all drive the same
//! velocity-space frame without touching collision code.
//!
//! Design rules:
//! - brains write desired motion, not direct position changes;
//! - brains are pure functions of a snapshot plus their local state;
//! - integration code reads only the frame, not the brain implementation.

use ambition_engine_core::{AccelerationFrame, GameplayFramePolicy, Vec2};

/// A request from a brain to fire a projectile this tick. Mirrors the
/// existing `ChoreographyAction::FireProjectile { dir, speed }` but
/// lives on the control-frame side of the seam so a future
/// non-choreography brain (RL policy, dialogue-scripted skirmish,
/// remote player) can emit one without going through the
/// choreography path.
///
/// The launch direction carries an explicit frame policy. Do not infer
/// "local" from `x/y` names or "world" from the current implementation's
/// cardinal gravity cases: callers should choose a constructor that says what
/// frame authored the vector, and the consumer should convert at its own
/// simulation seam. That keeps arbitrary-angle acceleration frames (and future
/// non-Euclidean/Lorentz-like motion policies) from inheriting a hidden
/// axis-aligned assumption.
///
/// **Current convention:** ActionSet-driven consumers read projectile
/// speed from the resolved `RangedActionSpec`; player/projectile legacy
/// paths may still carry `speed` here while migration is in progress.
/// New brain backends should prefer `speed = 0.0` and let the actor's
/// capability choose the launch speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorFireRequest {
    /// Launch direction in the frame named by [`Self::dir_policy`] (unit vector
    /// recommended; the sandbox projectile spawner normalizes anyway).
    pub dir: Vec2,
    /// Frame in which [`Self::dir`] was authored/interpreted.
    pub dir_policy: GameplayFramePolicy,
    /// Launch speed in px/s.
    pub speed: f32,
}

impl ActorFireRequest {
    /// Fire along a controlled-body-local direction (`+x` side/right,
    /// `+y` toward feet). Use for actor-combat verbs such as Smash-style
    /// ranged attacks where "forward/up/down" should follow the actor.
    pub fn controlled_body_local(dir: Vec2, speed: f32) -> Self {
        Self {
            dir,
            dir_policy: GameplayFramePolicy::ControlledBodyLocal,
            speed,
        }
    }

    /// Fire along a world/environment-space direction. Use for direct target
    /// vectors, arena hazards, and other effects that deliberately ignore the
    /// controlled body's local side/feet axes.
    pub fn world_space(dir: Vec2, speed: f32) -> Self {
        Self {
            dir,
            dir_policy: GameplayFramePolicy::WorldSpace,
            speed,
        }
    }

    /// Convert the request direction to world space at the consumer seam.
    ///
    /// `AccelerationFrame` and `ControlledBodyLocal` use the same basis today,
    /// but keeping both policies visible lets future motion/frame models split
    /// them without changing every call site.
    pub fn dir_to_world(self, frame: AccelerationFrame) -> Vec2 {
        match self.dir_policy {
            GameplayFramePolicy::ControlledBodyLocal | GameplayFramePolicy::AccelerationFrame => {
                frame.to_world(self.dir)
            }
            GameplayFramePolicy::WorldSpace => self.dir,
            GameplayFramePolicy::ScreenSpace => {
                debug_assert!(
                    false,
                    "screen-space fire directions must be resolved before gameplay"
                );
                self.dir
            }
        }
    }
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
    /// Normalized locomotion intent in the controlled body's local frame: `x` is
    /// local side/right, `y` is local down/toward-feet. Magnitude is a throttle in
    /// `[0, 1]` — "how hard, of what this body is capable" — *not* a velocity. Any
    /// per-actor variation (e.g. an enemy's per-spawn speed jitter) is baked into
    /// this throttle as intent; the body's px/s scale lives in its movement tuning
    /// (`max_run_speed`).
    ///
    /// One field, one meaning, for every self-locomoting actor — player input,
    /// possession, replay, hand-authored AI, learned policies. The integration
    /// half resolves velocity uniformly as `locomotion * max_run_speed`, with no
    /// per-actor-type branch. AI brains that reason in absolute speeds convert via
    /// [`crate::brain::BrainSnapshot::locomotion_for`]. Human input must resolve
    /// raw device axes before writing this.
    pub locomotion: Vec2,
    /// Exact world-space velocity command in px/s, for the *free-mover /
    /// choreography* modality: boss patterns that snap to a scripted velocity, and
    /// AI flyers that steer a 2D velocity directly. The free-mover integrator
    /// ([`crate`]'s `step_floating_body`) reads this; grounded integration reads
    /// [`Self::locomotion`] instead — each consumer picks the field for its
    /// movement mode, so the default `ZERO` simply means "no free-mover command".
    /// Deliberately distinct from locomotion (a different control modality, not a
    /// different actor type), so it does not reintroduce a player/enemy split.
    pub velocity_target: Vec2,
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
    /// Direction-of-attack for melee in the controlled actor's local frame.
    /// Zero = "use the actor's current facing". A non-zero vector lets the
    /// ActionSet pick between directional variants (up-tilt, down-air,
    /// back-air, …). Brains that don't care about directional melee leave this
    /// zero.
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
    /// Aim direction for charged ranged attacks in the controlled actor's local
    /// frame. `(0,0)` = use actor facing; non-zero = explicit twin-stick / mouse
    /// aim after crossing the input seam.
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
        assert_eq!(frame.locomotion, Vec2::ZERO);
        assert_eq!(frame.velocity_target, Vec2::ZERO);
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
        frame.fire = Some(ActorFireRequest::world_space(Vec2::new(1.0, 0.0), 0.0));
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
    fn fire_request_direction_policy_converts_through_arbitrary_acceleration_frame() {
        let frame = AccelerationFrame::new(Vec2::new(1.0, 1.0));
        let local = ActorFireRequest::controlled_body_local(Vec2::new(1.0, 0.0), 0.0);
        assert_eq!(local.dir_policy, GameplayFramePolicy::ControlledBodyLocal);
        assert_eq!(local.dir_to_world(frame), frame.side);

        let world_dir = Vec2::new(0.25, -0.75);
        let world = ActorFireRequest::world_space(world_dir, 0.0);
        assert_eq!(world.dir_policy, GameplayFramePolicy::WorldSpace);
        assert_eq!(world.dir_to_world(frame), world_dir);
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
        frame.fire = Some(ActorFireRequest::world_space(Vec2::new(1.0, 0.0), 0.0));
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
