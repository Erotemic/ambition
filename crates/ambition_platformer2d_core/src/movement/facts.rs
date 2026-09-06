//! The published, model-independent movement-facts vocabulary (ADR 0024).
//!
//! Axis maneuver state is policy-PRIVATE (it lives inside
//! [`AxisSweptMotion`](super::AxisSweptMotion)); animation, combat gates,
//! affordances, HUD, traces, time-control, and RL observations consume the
//! SEMANTIC facts published here instead of inspecting a policy's internals.
//! The facts are a projection: the drivers rewrite [`BodyMotionFacts`] from the
//! body's model right after each movement step, so a body running a non-axis
//! policy can never expose stale axis maneuver state — the projection of a
//! non-axis model is simply the default (no maneuver in flight).

use bevy_ecs::component::Component;

use super::model::MotionModel;
use crate::ledge_grab::LedgeGetupKind;
use crate::Vec2;

/// Semantic ledge-engagement facts (presentation-facing; the anchor and climb
/// curves stay policy-private).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LedgeFacts {
    /// False while hanging; true once a getup (climb/roll/attack) committed.
    pub climbing: bool,
    pub getup_kind: LedgeGetupKind,
}

/// SOMETHING ELSE OWNS THIS BODY'S POSE.
///
/// ⭐⭐ THE ONE FACT A CONSTRAINED BODY OWES EVERY OTHER DOMAIN. A body welded
/// into a saddle, carried by a lift, held by a grab or posed by a capture is not
/// deciding where it is — and the systems that need to know that are scattered
/// across crates which cannot see each other: the movement kernel (do not
/// integrate its locomotion), combat (a move may forbid itself while the body is
/// held), presentation, brains. Every one of those asking its own domain-shaped
/// question is how a body ends up with two authorities that disagree.
///
/// ⛔⛔ IT LIVES IN `_core` BECAUSE OF THE DEPENDENCY GRAPH, and that is not an
/// accident of convenience. `ambition_mount` knows what a saddle is and
/// `ambition_combat` knows what a move is, and NEITHER depends on the other, so
/// there is no crate among them that could hold this. Both depend on this one.
/// A marker here is the only shape in which "held" is sayable to both.
///
/// ⚠ IT SAYS NOTHING ABOUT WHO. Deliberately: a consumer that needed the holder
/// would be reaching into a domain it does not depend on, which is the coupling
/// this marker exists to avoid. Ask the domain that owns the relationship.
///
/// # What a constrained body still advances
///
/// ⭐⭐ THE CONTRACT, AXIS BY AXIS, because for a long time this marker DID four
/// things and STATED none of them, and the difference between "guaranteed" and
/// "true by luck" was not written anywhere. Measured on the pirate's shark over
/// 299 mounted ticks with the stick jammed toward the stage edge, and pinned by
/// `a_saddled_body_advances_its_clocks_and_none_of_its_displacement`:
///
/// ```text
/// DISPLACEMENT        the constraint's, wholly. The kernel still integrates and
///                     the constraint still overwrites, so the pass is WASTED —
///                     but a mounted body ends every tick at velocity ZERO, so
///                     none of it survives into the next one.
/// GRAVITY             follows displacement: it is an input to an integration
///                     whose result the constraint discards.
/// GROUND / CONTACT    the constraint's. A saddled body reads airborne from the
///                     tick the ride actually has it. ⚠ NOT on the handoff tick,
///                     where it is genuinely still standing where it was.
/// LEDGES              unreachable. A rider is not travelling past anything
///                     under its own power, and cannot catch what carries it.
/// GAIT                unpublished. `running` and `dashing` describe a body
///                     driving itself.
/// VOLUNTARY VERBS     cleared before the kernel sees them, because a jump or a
///                     dodge spent here changes state the constraint CANNOT
///                     undo — a snap fixes a position, not a spent double jump.
/// RESOURCES           not spent, and clearing this tick's verbs was NOT enough
///                     to say so: an evade and a dash come out of the maneuver
///                     BUFFER, a press made earlier that stays spendable. A
///                     press made on the floor a moment before the constraint
///                     took the body was still spent inside it — measured, the
///                     air dodge went on mounted tick 2. The spend is refused in
///                     the control phase now. ⛔ REFUSED, NOT ERASED: the buffer
///                     is input memory, and dropping it swallows a press the
///                     player is entitled to have honoured the tick the
///                     constraint lets go. Not refreshed either, since the
///                     ground fact they refresh on is the constraint's.
/// EXTERNAL LAUNCHES   the hit lands NOW and its travel STAYS STAGED — see
///                     `MotionStepContext::pose_owned_externally`.
/// CLOCKS AND COMBAT   STILL RUNNING, and this is the half that separates a held
///                     body from a dead one. A rider steers its mount and swings
///                     from the saddle; hitstun has to decay or the flinch that
///                     put it there never ends.
/// ```
///
/// ⛔ THE KERNEL IS NOT SHORT-CIRCUITED FOR THIS, and that was a considered
/// decision rather than an omission. Skipping the motion step would also skip
/// the clock decay that lives inside it, which is the half a rider needs most;
/// the measured cost of running it is wasted arithmetic, not a wrong answer.
/// ⛔ And it is NOT routed through `out_of_play`, which halts velocity and zeroes
/// `dt` outright — correct for a body nobody is driving, and the exact opposite
/// of what a rider needs.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PoseOwnedExternally;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyMotionFacts {
    /// An active dash is in flight.
    ///
    /// this is the TRAVERSAL dash — Ambition's discrete, charge-gated
    /// burst. A platform fighter's kit switches it off, so nothing that means
    /// *"is this body running"* may read it. That is [`Self::running`].
    pub dashing: bool,
    /// This body is in a RUN: grounded, steering the way it travels, at or
    /// above [`crate::MovementTuning::run_commit_frac`] of its top speed. The
    /// gait the genre's running attack comes out of.
    pub running: bool,
    /// Is this body inside its INITIAL DASH — the window in which a direction
    /// change is still free ([`crate::LocomotionTuning::initial_dash_time`])?
    ///
    /// Published so animation and the rows built on this phase (foxtrot, dash
    /// dance, pivot grab) read ONE answer rather than each re-deriving it from
    /// speed, which cannot tell a dash from a run at the same speed.
    pub initial_dashing: bool,
    /// Is this body mid-TURNAROUND — committed to a run, asking to face the
    /// other way, and not there yet
    /// ([`crate::LocomotionTuning::turnaround_time`])?
    ///
    /// Published so the rows built on the phase (pivot grab, reverse aerial
    /// rush) read ONE answer rather than each comparing a stick against a
    /// facing and calling the disagreement a turnaround.
    pub turning_around: bool,
    /// Is this body standing on the BRINK — supported, but not if it leaned any
    /// further the way it faces
    /// ([`crate::LocomotionTuning::teeter_margin`])?
    ///
    /// Published for control and animation. ⛔ It changes no collision: a
    /// teetering body may still walk off, and nothing here refuses it.
    pub teetering: bool,
    /// HOW MUCH RISE THIS BODY'S OWN AIR JUMP PUT IN AND STILL HAS — the
    /// amount a double-jump cancel may take back. `0.0` for a body that has not
    /// air-jumped, has landed, or whose jump gravity has already eaten.
    ///
    /// ⛔⛔ AN AMOUNT, NOT A PREDICATE, and the difference is the whole fact.
    /// This was a bool meaning "rising, and no faster than my own jump could
    /// push me" — a magnitude test that a weak opponent launch also passes, so
    /// an aerial deleted knockback. A quantity that only ever shrinks cannot be
    /// confused with somebody else's velocity: see
    /// [`crate::movement::MotionState::air_jump_rise_owned`].
    ///
    /// ⭐ THE CONSUMER SHEDS `min(this, actual rise)` ALONG THE BODY'S OWN RISE
    /// AXIS, so no reader needs this body's jump tuning and none can form a
    /// second opinion about it.
    pub air_jump_rise_owned: f32,
    /// WHERE THE WIRE SHE IS HANGING FROM COMES DOWN FROM, in world space, or
    /// `None` for a body that is not on one.
    ///
    /// ⭐⭐ THE ANCHOR AND NOT A FLAG, because the only consumer is a renderer
    /// that has to draw a rope from a point in the sky to her — and a `bool`
    /// would force presentation to re-derive that point from tuning it has no
    /// business reading. It is the same reasoning `air_jump_rise_owned` above is
    /// a quantity: publish what the consumer needs, not a predicate it has to
    /// reconstruct the number behind.
    ///
    /// ⛔ AND ONE SENTENCE FOR BOTH ROADS. `BodyPoseView` is the session's
    /// exploration player and `FeatureView` is every ACTOR, which is what a
    /// Smash fighter is; a rule stated on only one of them is not stated. That
    /// split is what drew the Performer under the stage for a month.
    pub wire_anchor: Option<crate::Vec2>,
    pub jump_squatting: bool,
    /// Dodge-roll i-frames are active.
    pub dodge_rolling: bool,
    /// Inside the evade's INVULNERABLE window — the staled half. Shorter than
    /// [`Self::dodge_rolling`] for a body that has been evading a lot.
    pub evade_invulnerable: bool,
    /// LEDGE intangibility is active — the grab's earned window, a getup roll,
    /// or a getup attack.
    ///
    /// A sibling of [`Self::dodge_rolling`], not a refinement of it, and the
    /// separation is the point: it read as a dodge roll until this fact
    /// existed, so nothing downstream could tell a body hanging on an edge from
    /// one mid-evade. Everything that only asks *"is this body untouchable
    /// right now?"* reads [`Self::evading`], which takes both.
    pub ledge_intangible: bool,
    /// The grounded evade is a SPOT DODGE, not a roll. A refinement OF
    /// [`Self::dodge_rolling`] rather than a sibling: both are true together,
    /// because the i-frames are the same and only the pose differs. Everything
    /// asking *"is this body evading?"* keeps reading [`Self::evading`].
    pub spot_dodging: bool,
    /// Air-dodge i-frames are active — a separate fact from
    /// [`Self::dodge_rolling`] on purpose, so animation and debugging can tell
    /// the aerial evade from the grounded one. Everything that only asks *"is
    /// this body evading?"* should read [`Self::evading`] instead, which is the
    /// term the damage rule takes.
    pub air_dodging: bool,
    /// The air dodge's window has closed but its endlag has not: the body is
    /// committed and no longer invulnerable. Presentation and AI read this;
    /// [`Self::evading`] deliberately does NOT include it.
    pub air_dodge_endlag: bool,
    /// The GROUND ROLL's window has closed but its endlag has not — the same
    /// distinction as [`Self::air_dodge_endlag`], and the beat a defender is
    /// meant to read. Not part of [`Self::evading`]: the body is committed and
    /// no longer invulnerable, which is the whole point of the state.
    pub dodge_roll_endlag: bool,
    /// This body is COMMITTED to the evade it is in and may not start a move.
    ///
    /// ⭐⭐ RESOLVED IN THE KERNEL, because only the kernel holds both the
    /// evade's remaining time and the body's tuning. A consumer that had to
    /// subtract a timer from a constant would be re-deriving the rule at every
    /// call site — which is how the two halves come to disagree.
    ///
    /// `false` for every body in a game that declares no lockout, which is what
    /// every body did before it existed: an attack cancels an evade on its first
    /// frame.
    pub evade_committed: bool,
    /// Launched and helpless — see [`crate::movement::knockdown`].
    pub tumbling: bool,
    /// Prone on the floor with getup options open.
    pub knocked_down: bool,
    /// Tech/getup invulnerability is running.
    pub getup_invulnerable: bool,
    /// The blink telegraph is showing (precision aim or charge hold).
    pub blink_telegraph: bool,
    /// Precision blink aim specifically (drives the aim preview).
    pub blink_aiming: bool,
    /// The precision-blink aim offset (body-local; presentation preview data).
    pub blink_aim_offset: Vec2,
    /// Post-blink grace i-frames are active.
    pub blink_grace: bool,
    pub wall_clinging: bool,
    pub wall_climbing: bool,
    pub gliding: bool,
    pub fast_falling: bool,
    /// Grounded braking against travel — steering opposite the body's own
    /// tangential speed while riding. NOT part of the model projection (the
    /// model alone can't know the input): the surface-momentum integration
    /// republishes it right after [`Self::from_model`] each step, exactly like
    /// the ridden-surface fact. Axis walkers leave it false today.
    pub skidding: bool,
    /// Ledge engagement, if any.
    pub ledge: Option<LedgeFacts>,
    /// This body crawls along surfaces rather than walking a gravity axis —
    /// the `AdhesiveCrawler` policy, published as a FACT.
    ///
    /// ADR 0024 §8 forbids reading `ActorTuning::surface_walker` at
    /// runtime, because that boolean is spawn-time SELECTION: it chooses the
    /// motion model once and is then a stale copy of a decision the body already
    /// carries explicitly. One consumer still read it — the brain snapshot's
    /// `turns_at_walls`, where "does a wall mean turn around" is genuinely
    /// different for a body whose whole locomotion is walls. It reads this now.
    ///
    /// a FACT, not the model: consumers outside the kernel ask what is
    /// true of the body, never which enum variant produced it. That is the same
    /// rule the animation layer follows for `tumbling` and `knocked_down`.
    pub adhesive_crawling: bool,
}

impl BodyMotionFacts {
    /// Is this body inside an evade's invulnerable window? — the ONE term
    /// the damage rule takes, so a maneuver added later cannot grant i-frames at
    /// five emit sites and miss the sixth. Adding an evade means extending this
    /// method, not auditing every caller of `body_vulnerable`.
    pub fn evading(&self) -> bool {
        self.evade_invulnerable || self.getup_invulnerable || self.ledge_intangible
    }

    /// Project the active policy's semantic facts. Non-axis policies have no
    /// axis maneuvers by construction — their projection is the default.
    pub fn from_model(model: &MotionModel) -> Self {
        // the crawler is answered BEFORE the early return. Every fact below
        // is axis-swept state, so the old `else { return default() }` gave a
        // crawler a facts block claiming it was not crawling — which is the exact
        // reason its one consumer went on reading the spawn-time flag instead.
        if matches!(model, MotionModel::AdhesiveCrawler(_)) {
            return Self {
                adhesive_crawling: true,
                ..Self::default()
            };
        }
        let MotionModel::AxisSwept(axis) = model else {
            return Self::default();
        };
        let state = &axis.state;
        Self {
            // Answered above; an axis-swept body never crawls.
            adhesive_crawling: false,
            dashing: state.dash_timer > 0.0,
            running: state.running,
            initial_dashing: state.initial_dash_timer > 0.0,
            turning_around: state.turnaround_timer > 0.0,
            teetering: state.teetering,
            air_jump_rise_owned: state.air_jump_rise_owned,
            wire_anchor: state.wire.map(|w| w.anchor),
            jump_squatting: state.jump_squat_timer > 0.0,
            dodge_rolling: state.dodge_roll_timer > 0.0,
            // ⭐ THE SAFE HALF OF AN EVADE, WHICH IS SHORTER THAN THE MOVE WHEN
            // THE BODY HAS BEEN SPAMMING IT. `dodge_rolling` above is the move.
            evade_invulnerable: state.evade_invuln_timer > 0.0,
            // ⛔ HANGING IS NOT YET INTANGIBLE. The first frames of a ledge
            // catch are exposed on purpose — see `LEDGE_GRAB_VULNERABLE_TIME`.
            ledge_intangible: state.ledge_invuln_timer > 0.0 && state.ledge_vulnerable_timer <= 0.0,
            spot_dodging: state.dodge_roll_timer > 0.0 && state.spot_dodging,
            air_dodging: state.air_dodge_timer > 0.0,
            air_dodge_endlag: state.air_dodge_endlag_timer > 0.0,
            dodge_roll_endlag: state.dodge_roll_endlag_timer > 0.0,
            // ⭐ COMMITTED WHILE THE EVADE HAS MORE THAN THE TAIL LEFT. One
            // expression for all three evades: whichever timer is running is
            // the one being measured, and `0.0` disables the rule so a game
            // that declares nothing behaves exactly as it did.
            evade_committed: {
                let tail = axis.params.abilities.evade_cancel_tail;
                let remaining = state.dodge_roll_timer.max(state.air_dodge_timer);
                tail > 0.0 && remaining > tail
            },
            tumbling: state.tumble_until_landing,
            knocked_down: state.knockdown_timer > 0.0,
            getup_invulnerable: state.getup_invuln_timer > 0.0,
            blink_telegraph: state.blink_aiming || state.blink_hold_active,
            blink_aiming: state.blink_aiming,
            blink_aim_offset: state.blink_aim_offset,
            blink_grace: state.blink_grace_timer > 0.0,
            wall_clinging: state.wall_clinging,
            wall_climbing: state.wall_climbing,
            gliding: state.gliding,
            fast_falling: state.fast_falling,
            // Input-relative; the integration republishes it post-projection.
            skidding: false,
            ledge: state.ledge_grab.as_ref().map(|grab| LedgeFacts {
                climbing: grab.climbing,
                getup_kind: grab.getup_kind,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::{AxisSweptParams, MomentumParams};

    #[test]
    fn axis_maneuvers_project_to_semantic_facts() {
        let mut model = MotionModel::axis_swept(AxisSweptParams::default());
        let MotionModel::AxisSwept(axis) = &mut model else {
            unreachable!();
        };
        axis.state.dash_timer = 0.1;
        axis.state.blink_hold_active = true;
        axis.state.wall_clinging = true;
        let facts = BodyMotionFacts::from_model(&model);
        assert!(facts.dashing && facts.blink_telegraph && facts.wall_clinging);
        assert!(!facts.blink_aiming && !facts.gliding && facts.ledge.is_none());
    }

    #[test]
    fn a_non_axis_policy_can_never_expose_stale_axis_facts() {
        let model = MotionModel::surface_momentum(MomentumParams::default());
        assert_eq!(
            BodyMotionFacts::from_model(&model),
            BodyMotionFacts::default()
        );
    }
}
