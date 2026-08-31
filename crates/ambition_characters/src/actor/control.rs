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

use ambition_platformer2d_core::{
    AccelerationFrame, GameplayFramePolicy, InputState, LocalAxes, Vec2, WorldVec2,
};

/// The body→controller half of the intent-in seam.
///
/// A controller only *attempts* inputs; the body enforces its own physics
/// (cooldown / stun / resource / which abilities exist) and reports, per intent,
/// whether the attempt was accepted (and applied) or blocked, naming the reason.
/// This is the floor that makes a spam controller and a human produce the *same*
/// physical output on the same body (invariant I3): the controller cannot beat
/// the body's rate by attempting more often.
///
/// Built once per migrated intent as the resolver grows (fire first); routed back
/// to the controller through the world-view so a brain can react to a blocked
/// attempt (e.g. reposition instead of firing into a cooldown).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IntentOutcome {
    /// The body accepted the attempt and applied its effect this tick.
    #[default]
    Accepted,
    /// The body refused the attempt; the body's current physical state forbids
    /// it. The reason is informational — a controller may use it to choose a
    /// different intent next tick, but the body's decision is authoritative.
    Blocked(BlockReason),
}

impl IntentOutcome {
    /// True iff the body applied the attempted effect this tick.
    pub fn accepted(self) -> bool {
        matches!(self, IntentOutcome::Accepted)
    }
}

/// Why a body blocked an attempted intent. Open to extension as more intents
/// migrate into the resolver; each variant names a physical-state reason a human
/// would also be subject to (never "you are not the player").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockReason {
    /// The weapon / ability is still on its refire cooldown.
    Cooldown,
    /// The body is in hitstun / staggered and cannot act.
    Stunned,
    /// The body lacks the resource (meter / charge) the intent costs.
    OutOfResource,
    /// The body has no capability for this intent (it isn't in its kit).
    NoCapability,
    /// The intent is incompatible with the body's current locomotion mode.
    WrongMode,
    /// The body is dead.
    Dead,
}

/// Projectile-fire intent emitted by a brain. The direction carries an explicit
/// [`GameplayFramePolicy`] so consumers can resolve it in the active movement frame.
///
/// TODO(compat-remove): migrate remaining callers that author `speed` here to the resolved
/// `RangedActionSpec`, then remove the redundant speed authority from this request.
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
                // Under any rotated gravity that is a shot going the wrong way, with nothing to
                // grep for — and the `debug_assert!` that stood here alone said so only in a debug
                // build, which is not where a player fires it.
                //
                // Still returns `self.dir`: there is no better answer available
                // here, and the two policies share a basis today, so the value is
                // usually right. What changes is that a wrong one leaves a trace.
                debug_assert!(
                    false,
                    "screen-space fire directions must be resolved before gameplay"
                );
                bevy::log::error!(
                    target: "ambition_platformer2d::control",
                    "a screen-space fire direction reached gameplay unresolved; \
                     it is being used as a WORLD direction, which is wrong under \
                     any rotated gravity"
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
    ///
    /// the type is the contract, because the prose was not enough. This
    /// field was a bare `Vec2` and two shipped brains got its frame wrong in
    /// opposite directions (S48): the fighter brain wrote raw world `x` into it,
    /// and `brain/player.rs` wrote this body-local vector into the WORLD-space
    /// [`Self::velocity_target`] beside it. Both compiled, because a `Vec2` in a
    /// struct field records no frame — and the doc comment above, which has said
    /// "controlled body's local frame" the whole time, is not something the
    /// compiler reads.
    pub locomotion: LocalAxes,
    /// [`Self::locomotion`] as it stood BEFORE a playing move damped it. `None`
    /// when nothing damped it — then `locomotion` IS the stick.
    ///
    /// ⛔ EDGE DETECTION ONLY. It travels to [`InputState::undamped_axes`] so
    /// the kernel can remember what the player was HOLDING while still acting
    /// on what the body is ALLOWED to do. See
    /// [`Self::damped_by_move_motion`].
    pub undamped_locomotion: Option<LocalAxes>,
    /// Exact world-space velocity command in px/s, for the *free-mover /
    /// choreography* modality: boss patterns that snap to a scripted velocity, and
    /// AI flyers that steer a 2D velocity directly. The free-mover integrator
    /// ([`crate`]'s `step_floating_body`) reads this; grounded integration reads
    /// [`Self::locomotion`] instead — each consumer picks the field for its
    /// movement mode, so the default `ZERO` simply means "no free-mover command".
    /// Deliberately distinct from locomotion (a different control modality, not a
    /// different actor type), so it does not reintroduce a player/enemy split.
    ///
    /// [`WorldVec2`] because "world-space" was already written here and a
    /// brain still wrote a body-local vector into it. A possessed flyer flew
    /// world-UP when the player pushed right; see the note on
    /// [`Self::locomotion`]. The wrapper is what makes the two fields refuse to
    /// be assigned to each other.
    pub velocity_target: WorldVec2,
    // THERE IS NO `drop_through` FIELD, AND ITS ABSENCE IS THE DESIGN
    // . It sat here for months documented as
    // "suppress the OneWay vertical block this tick", and every part of that
    // sentence was true of the ENGINE and false of this struct: no brain ever
    // set it, `to_input_state` never mapped it, and the mount hand-off
    // dutifully copied `false` from rider to mount every tick.
    //
    // the wire was REFUSED, not forgotten, and the refusal is older than the
    // field's doc. Drop-through is a DERIVED GESTURE — `descend > 0.35 &&
    // jump_pressed`, owned by `movement::integration::wants_drop_through` and
    // computed at the consumer so it is gravity- and input-mode-relative. There
    // is deliberately no boolean for it on `InputState` either, so this field
    // had nowhere to be mapped TO. A brain asks for a drop-through exactly the
    // way a human does: `locomotion.y` toward the feet plus `jump_pressed`.
    // Adding the boolean back would fork the one gesture rule into two
    // spellings that agree only by hand.
    /// Desired facing this tick. `+1.0` = right, `-1.0` = left,
    /// `0.0` = leave the actor's existing facing alone.
    pub facing: f32,
    /// Brain wants to begin a melee attack windup this tick. The
    /// simulation half handles cooldown gating; the brain just
    /// signals intent.
    pub melee_pressed: bool,
    /// Sustain: the melee/attack button is currently held.
    pub melee_held: bool,
    /// Falling edge: the melee/attack button was released this tick.
    pub melee_released: bool,
    /// Device-independent strong-attack hint for the sim-side gesture
    /// interpreter. Brains/replays/RL may set this directly; characters never
    /// own its timing thresholds.
    pub melee_strong_hint: bool,
    /// Brain wants to fire a projectile this tick. `Some` carries the
    /// launch direction + speed; `None` is "no shot".
    pub fire: Option<ActorFireRequest>,
    /// Direction-of-attack for melee in the controlled actor's local frame.
    /// Zero = "use the actor's current facing". A non-zero vector lets the
    /// ActionSet pick between directional variants (up-tilt, down-air,
    /// back-air, …). Brains that don't care about directional melee leave this
    /// zero.
    ///
    /// typed with [`Self::locomotion`] rather than left bare: it is the third
    /// frame-carrying field on this struct, it says "local frame" in prose only,
    /// and it had exactly the property that let the other two go wrong.
    pub attack_axis: LocalAxes,
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
    /// Rising edge on the SHARED burst press — brain wants its dodge-or-dash
    /// this tick. WHICH of the two it gets is the body's answer
    /// (`resolve_burst_maneuver`), not the brain's; the simulation half handles
    /// cooldowns and direction selection.
    pub burst_pressed: bool,
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
    /// Rising edge: this body wants to CAPTURE somebody this tick.
    ///
    /// the human's Grab button and a CPU's decision write this SAME field.
    /// There is deliberately no `cpu_wants_grab` beside it — the whole point of
    /// the semantic control surface is that a brain asks for a grab the way a
    /// person does, and everything downstream reads one answer.
    pub grab_pressed: bool,
    /// Rising edge: this body wants to TAUNT this tick. The human's Taunt
    /// button and a brain's decision write this SAME field, for the reason
    /// stated above.
    pub taunt_pressed: bool,
    /// Rising edge: brain wants to use its special / signature move.
    /// What this resolves to is per-entity (ActionSet), so the same
    /// `special_pressed=true` from a player brain and a possessed
    /// goblin yield different concrete effects.
    pub special_pressed: bool,
    /// SUSTAIN: the special button is down. The twin of [`Self::melee_held`],
    /// and it exists for the mechanic the edge cannot express — a chargeable
    /// neutral special freezes its timeline while this is true and fires on the
    /// release, so by the time the charge is accruing the press is long gone.
    ///
    /// A brain that charges holds this the way a person does; one that taps
    /// leaves it false and gets the minimum payoff, which is the same rule the
    /// smash charge already plays by.
    pub special_held: bool,
    /// Rising edge: brain wants to trigger a pogo bounce this tick.
    /// Today only the human player binds a verb here (the dedicated
    /// pogo input + attack+down combo); AI brains leave it false.
    /// Promoted onto the frame so the sandbox's player polarity flip
    /// can drop its raw `ControlFrame` dependency.
    pub pogo_pressed: bool,
    /// Rising edge: brain wants to enter / refresh fast-fall this
    /// tick (player-side dedicated input; AI brains ignore today).
    pub fast_fall_pressed: bool,
    /// Rising edge on the MODE-SWITCH verb — [`ControlSlot::Utility`]'s device
    /// edge on this frame.
    ///
    /// not "the fly button", despite the name. Flight was the first thing
    /// to claim the slot, so the field wears its verb; what the press MEANS is
    /// whatever the body's action scheme puts there. A body that declares a
    /// technique on Utility (Sanic's transformation) has this routed to its
    /// sanctioned edge and cleared by `resolve_control_slots`, exactly like the
    /// combat slots — so a mode switch cannot leak into generic flight, and the
    /// on-screen control is named by the body rather than by the engine.
    ///
    /// [`ControlSlot::Utility`]: ambition_entity_catalog::action_scheme::ControlSlot::Utility
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
    pub blink_quick_dir: WorldVec2,
    /// WORLD-space precision-blink steer vector, resolved at the brain seam
    /// through the AIM frame mode (screen-directed by default). Decoupled from
    /// [`Self::blink_quick_dir`] so the two blink forms can use different frame
    /// policies on the same stick. `ZERO` → no precision steer this tick.
    pub blink_aim_step: WorldVec2,
    /// Aim direction for charged ranged attacks in the controlled actor's local
    /// frame. `(0,0)` = use actor facing; non-zero = explicit twin-stick / mouse
    /// aim after crossing the input seam.
    pub aim: LocalAxes,
    /// Sustain: the modifier slot is held this tick.
    ///
    /// The engine attaches NO meaning to it. It is carried here — in the
    /// body-local intent vocabulary — so a body's OWN rules can read a sustained
    /// control slot and interpret it as a technique (a locomotion mode, a stance,
    /// a guard) without reaching back to a device or to the global control frame.
    /// Because it rides `ActorControlFrame`, any controller can express it: a
    /// human holding the button, an AI brain deciding to sustain, a replay, a
    /// learned policy. A rule that reads this runs identically for every body.
    pub modifier_held: bool,
    /// Rising edge of the same slot whose sustain is [`Self::modifier_held`], so a
    /// body's rules can bind a momentary action to the press while the hold drives
    /// a sustained technique.
    pub modifier_pressed: bool,
}

impl ActorControlFrame {
    /// Empty / idle frame — no movement, no actions, hold current
    /// facing. Useful starting point for brains that conditionally
    /// fill fields, and for sandbox tests that want a known-stable
    /// baseline.
    pub fn neutral() -> Self {
        Self::default()
    }

    /// Did this frame carry an ACTION press — something the body did on purpose
    /// that is not a way of MOVING?
    ///
    /// ⭐⭐ ONE PLACE, because the alternative is a hard-coded verb list per
    /// customer and the lists then disagree. `ChargeSustain::UntilPressedAgain`
    /// is the first customer: a Performer under the trapdoor ends the beat "by
    /// pressing a non-move action", and the charge tick was checking Attack and
    /// Special ALONE — the two verbs the resolved ATTACK gesture happens to
    /// carry — while its own comment claimed every action but movement. The
    /// component it was reading could not express the rule it was asserting.
    ///
    /// ⛔ MOVEMENT IS EXCLUDED, AND TRAVERSAL IS MOVEMENT. The stick, the jump,
    /// the dash, the fast-fall, the blink, the pogo and the flight toggle are
    /// all ways of GOING somewhere, and a player steering under the stage must
    /// not end the beat by steering.
    ///
    /// ⚠ SHIELD IS A HOLD, NOT A PRESS. This frame carries `shield_held` and no
    /// shield edge, so a shield cannot contribute a press here. That is a
    /// vocabulary gap rather than a policy decision — whoever adds
    /// `shield_pressed` should add it to this list in the same change.
    ///
    /// ⚠ AND `modifier_pressed` IS NOT AN ACTION. It qualifies another verb; on
    /// its own the body did nothing.
    pub fn action_press_that_is_not_movement(&self) -> bool {
        self.melee_pressed
            || self.special_pressed
            || self.projectile_pressed
            || self.grab_pressed
            || self.taunt_pressed
            || self.interact_pressed
    }

    /// What the PLAYER is HOLDING, as opposed to what this body is ALLOWED to
    /// move by.
    ///
    /// ⭐⭐ THE TWIN OF [`InputState::steer_axis`], and it exists for the same
    /// reason on this side of the seam: `update.rs` PUBLISHES THE DAMPED FRAME
    /// back onto the component after integration, so a consumer reading
    /// `locomotion` off an actor's `ActorControl` sees zero for the whole of a
    /// rooted move.
    ///
    /// ⛔ THAT IS NOT A DETAIL. The B-reverse flick is read off this component,
    /// and a special with a `motion_scale: 0.0` tail — which is how this
    /// repository authors a commitment — would have made the technique
    /// impossible on exactly the moves that most want it.
    pub fn steer_axis(&self) -> LocalAxes {
        self.undamped_locomotion.unwrap_or(self.locomotion)
    }

    /// Damp this frame's STEERING INTENT by a live move's authored motion lock
    /// (`MoveSpec::motion_scale_at`, or zero while a charge roots the body).
    ///
    /// ⛔⛔ ONE PLACE, BECAUSE IT WAS ONLY EVER APPLIED ON ONE ROAD. The actor
    /// integrator scaled its brain frame inline and its doc claimed the lock held
    /// "for every controller alike"; the HOME/player integrator never received the
    /// scale at all, so a human fighter kept full steering through a committed
    /// swing and walked while charging a smash — the very rule Jon asked for, live
    /// for autonomous bodies and absent for the one he was driving.
    ///
    /// ⭐ INTENT ONLY. Action edges (melee, jump, burst, shield) pass through
    /// untouched: a move restricts where a body may GO, never whether it may act.
    /// A rooted body still releases its charge.
    /// ⭐⭐ AND IT RECORDS WHAT IT DAMPED. Forbidding movement must not erase the
    /// state that recognises the next input: the initial dash remembers
    /// direction by comparing this tick's stick against last tick's, so a player
    /// who simply HELD a direction through a rooted move read as neutral for its
    /// whole duration and was handed a free full-speed dash the frame it ended.
    /// [`Self::undamped_locomotion`] travels to `InputState` so edge detection
    /// can ask what was HELD while the body still acts on what it is ALLOWED.
    ///
    /// ⛔ RECORDED HERE rather than threaded through each call site, because
    /// this is the one function that knows the value is about to be lost.
    pub fn damped_by_move_motion(mut self, scale: f32) -> Self {
        let scale = scale.clamp(0.0, 1.0);
        if scale < 1.0 {
            self.undamped_locomotion = Some(self.locomotion);
            self.locomotion *= scale;
            self.velocity_target *= scale;
        }
        self
    }

    /// Map this controller intent into the engine's [`InputState`] — the input
    /// representation the shared player movement pipeline consumes.
    ///
    /// This is the bridge that lets ANY controller (a Brain, a possessing human, a
    /// future RL policy) drive the rich player limb pipeline, not just raw device
    /// input: the actor-unification routes every body through
    /// `update_player_*_clusters`, and this is the single place an
    /// `ActorControlFrame` becomes the `InputState` those limbs read. The mapping
    /// is near-identity — both are the body-local intent vocabulary — modulo names
    /// (`locomotion` → `axis_*`, `melee_pressed` → `attack_pressed`). `reset` is
    /// player-device-only (an actor never resets the room), and `control_dt` is a
    /// presentation concern, so both are left at their defaults.
    pub fn to_input_state(&self) -> InputState {
        InputState {
            movement: ambition_platformer2d_core::ActionEdges::EMPTY
                .with(
                    ambition_platformer2d_core::MovementAction::Jump,
                    ambition_platformer2d_core::Edge {
                        pressed: self.jump_pressed,
                        held: self.jump_held,
                        released: self.jump_released,
                    },
                )
                .with(
                    ambition_platformer2d_core::MovementAction::Burst,
                    ambition_platformer2d_core::Edge {
                        pressed: self.burst_pressed,
                        held: false,
                        released: false,
                    },
                )
                .with(
                    ambition_platformer2d_core::MovementAction::Blink,
                    ambition_platformer2d_core::Edge {
                        pressed: self.blink_pressed,
                        held: self.blink_held,
                        released: self.blink_released,
                    },
                )
                .with(
                    ambition_platformer2d_core::MovementAction::FlyToggle,
                    ambition_platformer2d_core::Edge {
                        pressed: self.fly_toggle_pressed,
                        held: false,
                        released: false,
                    },
                )
                .with(
                    ambition_platformer2d_core::MovementAction::FastFall,
                    ambition_platformer2d_core::Edge {
                        pressed: self.fast_fall_pressed,
                        held: false,
                        released: false,
                    },
                ),
            axes: self.locomotion,
            // What the player was HOLDING before a rooted move took it away.
            // `None` on every undamped road, which is every body not inside a
            // motion-scaled window.
            undamped_axes: self.undamped_locomotion,
            blink_quick_dir: self.blink_quick_dir,
            blink_aim_step: self.blink_aim_step,
            attack_pressed: self.melee_pressed,
            pogo_pressed: self.pogo_pressed,
            interact_pressed: self.interact_pressed,
            reset_pressed: false,
            shield_held: self.shield_held,
            control_dt: 0.0,
        }
    }

    /// True iff any action verb (melee / pogo / fire / jump / burst /
    /// interact / shield / special) is requested this tick. Useful
    /// for debug HUD ("brain is asking for something"), perf
    /// counters, and trace recording predicates.
    pub fn wants_any_action(&self) -> bool {
        self.melee_pressed
            || self.melee_held
            || self.melee_released
            || self.melee_strong_hint
            // The dedicated pogo button is a melee-swing trigger (the air-down
            // variant), so a pogo-only frame genuinely wants an action — omitting it
            // made any `resolve()`/processing gated on this drop the pogo swing.
            || self.pogo_pressed
            || self.fire.is_some()
            || self.jump_pressed
            || self.jump_held
            || self.burst_pressed
            || self.interact_pressed
            || self.shield_held
            || self.special_pressed
            || self.special_held
            || self.grab_pressed
            || self.taunt_pressed
    }

    /// Clear all rising- and falling-edge flags without touching sustains. Used
    /// by integrations that consume the frame in multiple stages, and by any
    /// brain that CARRIES a frame between decisions — an edge is a thing that
    /// happened once, and a frame cloned forward re-fires every edge on it.
    ///
    /// A helper that omits fields while claiming completeness is worse than no
    /// helper: the fighter's own tick had open-coded three of these and was
    /// RIGHT to distrust it. The rule for anything added to this frame is the
    /// one this doc now states — an edge belongs here, a sustain does not.
    ///
    /// Sustains deliberately untouched: `jump_held`, `melee_held`,
    /// `shield_held`, `special_held`, `projectile_held`, `blink_held`,
    /// `modifier_held`, and every continuous vector (`locomotion`, `attack_axis`, aim/steer).
    pub fn clear_edges(&mut self) {
        self.jump_pressed = false;
        self.jump_released = false;
        self.burst_pressed = false;
        self.interact_pressed = false;
        self.special_pressed = false;
        self.melee_pressed = false;
        self.melee_released = false;
        self.melee_strong_hint = false;
        self.fire = None;
        self.pogo_pressed = false;
        self.fast_fall_pressed = false;
        self.fly_toggle_pressed = false;
        self.projectile_pressed = false;
        self.projectile_released = false;
        self.blink_pressed = false;
        self.blink_released = false;
        self.modifier_pressed = false;
        self.grab_pressed = false;
        self.taunt_pressed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The input bridge maps the body-local control vocabulary onto `InputState`
    /// (the representation the shared player pipeline consumes): `locomotion` →
    /// `axis_*`, `melee` → `attack`, the rest near-identity. A neutral frame maps
    /// to neutral input; reset stays device-only.
    #[test]
    fn to_input_state_maps_the_control_vocabulary() {
        let neutral = ActorControlFrame::neutral().to_input_state();
        assert_eq!(neutral.axes.x, 0.0);
        assert!(!neutral.jump_pressed() && !neutral.attack_pressed);
        assert!(!neutral.reset_pressed, "an actor never resets the room");

        let mut frame = ActorControlFrame::neutral();
        frame.locomotion = LocalAxes::new(0.6, -0.2);
        frame.jump_pressed = true;
        frame.burst_pressed = true;
        frame.melee_pressed = true;
        frame.melee_strong_hint = true;
        frame.shield_held = true;
        let input = frame.to_input_state();
        assert_eq!(input.axes.x, 0.6, "locomotion.x → local x");
        assert_eq!(input.axes.y, -0.2, "locomotion.y → local y");
        assert!(input.jump_pressed() && input.burst_pressed() && input.shield_held);
        assert!(input.attack_pressed, "melee_pressed → attack_pressed");
    }

    /// A brain CAN ask for a drop-through, and this is how — the two ingredients of the
    /// engine's derived gesture (`wants_drop_through` = descend toward the feet + jump) both
    /// survive the brain→engine bridge in ONE frame.
    ///
    /// deliberately does not restate the 0.35 threshold: that number is the
    /// kernel's, `wants_drop_through` is kernel-private, and a copy here would
    /// be a second spelling of the rule this test exists to keep single.
    #[test]
    fn a_brain_expresses_drop_through_as_descend_plus_jump() {
        let mut frame = ActorControlFrame::neutral();
        // Local `y` points toward the FEET, so a full-magnitude descend is +1.
        frame.locomotion = LocalAxes::new(0.0, 1.0);
        frame.jump_pressed = true;
        let input = frame.to_input_state();
        assert_eq!(
            input.axes.y, 1.0,
            "the descend half of the gesture did not reach the kernel"
        );
        assert!(
            input.jump_pressed(),
            "the jump half of the gesture did not reach the kernel"
        );

        // Poison: neither half alone is the gesture, so neither may be
        // manufactured by the bridge out of the other.
        let mut descend_only = ActorControlFrame::neutral();
        descend_only.locomotion = LocalAxes::new(0.0, 1.0);
        assert!(!descend_only.to_input_state().jump_pressed());
        let mut jump_only = ActorControlFrame::neutral();
        jump_only.jump_pressed = true;
        assert_eq!(jump_only.to_input_state().axes.y, 0.0);
    }

    #[test]
    fn default_frame_is_neutral() {
        let frame = ActorControlFrame::default();
        assert_eq!(frame.locomotion, LocalAxes::ZERO);
        assert_eq!(frame.velocity_target, WorldVec2::ZERO);
        assert_eq!(frame.facing, 0.0);
        assert!(!frame.melee_pressed);
        assert!(!frame.melee_held);
        assert!(!frame.melee_released);
        assert!(!frame.melee_strong_hint);
        assert!(frame.fire.is_none());
        assert_eq!(frame.attack_axis, LocalAxes::ZERO);
        assert!(!frame.jump_pressed);
        assert!(!frame.jump_held);
        assert!(!frame.jump_released);
        assert!(!frame.burst_pressed);
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
        a.attack_axis = LocalAxes::new(1.0, 0.0);
        assert_ne!(baseline, a);
        let mut b = baseline;
        b.jump_pressed = true;
        assert_ne!(baseline, b);
        let mut c = baseline;
        c.burst_pressed = true;
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
        let mut i = baseline;
        i.melee_strong_hint = true;
        assert_ne!(baseline, i, "melee_strong_hint should be in PartialEq");
    }

    #[test]
    fn clear_edges_zeros_per_tick_edges_keeps_sustains() {
        let mut frame = ActorControlFrame::neutral();
        frame.jump_pressed = true;
        frame.jump_held = true;
        frame.jump_released = true;
        frame.burst_pressed = true;
        frame.interact_pressed = true;
        frame.special_pressed = true;
        frame.melee_pressed = true;
        frame.shield_held = true;
        frame.fire = Some(ActorFireRequest::world_space(Vec2::new(1.0, 0.0), 0.0));
        // Also set a sustain that should NOT clear: jump_held + shield_held.
        frame.clear_edges();
        assert!(!frame.jump_pressed);
        assert!(!frame.jump_released);
        assert!(!frame.burst_pressed);
        assert!(!frame.interact_pressed);
        assert!(!frame.special_pressed);
        assert!(!frame.melee_pressed);
        assert!(!frame.melee_held);
        assert!(!frame.melee_released);
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
        frame.melee_strong_hint = true;
        assert!(frame.wants_any_action(), "strong hint should count");
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
        frame.burst_pressed = true;
        assert!(frame.wants_any_action(), "burst_pressed should count");
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
    fn clear_edges_consumes_attack_edges_but_keeps_the_level() {
        let mut frame = ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.melee_held = true;
        frame.melee_released = true;
        frame.melee_strong_hint = true;
        frame.clear_edges();
        assert!(!frame.melee_pressed);
        assert!(frame.melee_held);
        assert!(!frame.melee_released);
        assert!(!frame.melee_strong_hint);
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
            burst_pressed: frame.burst_pressed,
            interact_pressed: frame.interact_pressed,
            body_contact_damage_enabled: frame.body_contact_damage_enabled,
            shield_held: frame.shield_held,
            special_pressed: frame.special_pressed,
            ..Default::default()
        };
        assert_eq!(frame, unchanged);
    }

    /// ⭐⭐ THE DAMPING RECORDS WHAT IT TOOK, AND IT REACHES THE KERNEL.
    ///
    /// The kernel's fix is only worth anything if the producer actually supplies
    /// the undamped stick — and it is supplied HERE, at the one function that
    /// knows the value is about to be lost, rather than threaded through each
    /// integrator. This walks the whole chain a rooted body walks.
    #[test]
    fn a_damped_frame_carries_the_stick_it_damped_all_the_way_to_the_kernel() {
        let mut held = ActorControlFrame::neutral();
        held.locomotion = LocalAxes::new(1.0, 0.0);

        // Nothing damping: the kernel is told there is no second answer.
        let plain = held.to_input_state();
        assert_eq!(plain.undamped_axes, None);
        assert_eq!(plain.steer_axis().x, 1.0);

        // ROOTED. The body may not move, and the player is still holding right.
        let rooted = held.damped_by_move_motion(0.0).to_input_state();
        assert_eq!(
            rooted.local_axis().x,
            0.0,
            "a rooted body was still allowed to steer"
        );
        assert_eq!(
            rooted.steer_axis().x,
            1.0,
            "the stick the player was HOLDING did not survive the damping, so \
             the kernel still cannot tell a rooted move from a release"
        );
    }
}
