//! Deterministic attack-input interpretation for every controlled body.
//!
//! Controllers emit raw button edges, an aim axis, and an optional strong hint
//! through [`ActorControlFrame`](super::control::ActorControlFrame). This module
//! owns the authoritative multi-tick gesture history that turns those values into
//! a stable directional tilt/smash intent. The state belongs to the BODY, not to
//! a device adapter or a character: human, brain, replay, RL, and remote control
//! all traverse the same interpreter.

pub use ambition_entity_catalog::AttackDir;
use ambition_platformer2d_core as ae;
use bevy::prelude::Component;

/// Tilt versus smash classification. Characters map this semantic result to
/// their own authored move verbs; they never own flick thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackStrength {
    Tilt,
    Smash,
}

/// Ground/air posture sampled when the attack begins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackPosture {
    Grounded,
    Airborne,
}

/// Input edge represented by one semantic attack intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackInputPhase {
    Press,
    Hold,
    Release,
}

/// Direction/strength/posture of one attack, plus the input edge being emitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackGestureIntent {
    pub direction: AttackDir,
    pub strength: AttackStrength,
    pub posture: AttackPosture,
    pub phase: AttackInputPhase,
}

impl AttackGestureIntent {
    fn with_phase(self, phase: AttackInputPhase) -> Self {
        Self { phase, ..self }
    }
}

/// WHAT A SPECIAL PRESS MEANT WHEN IT HAPPENED.
///
/// ⛔⛔ A BUFFERED PRESS MUST BE REPLAYED VERBATIM, and the special slot was the
/// one that was not. `BodyActionBuffer::special` is a bare TIMER, so replay
/// re-read `attack_dir_from_axis` off the LIVE stick: press Up+Special during
/// endlag, let go, and the buffered press came out as a NEUTRAL special — the
/// wrong move. Out of shield it was worse, because the out-of-shield rule asks
/// whether the press RISES, so a buffered up-special replayed after the stick
/// centred no longer even qualified.
///
/// ⛔ POSTURE IS DECIDED AT THE PRESS, not read off live ECS state at replay.
/// A kit with `special_down` and `special_air_down` gets whichever the player
/// asked for, rather than whichever the body happens to be doing a few ticks
/// later — the same rule [`AttackGestureIntent::posture`] already states for
/// the attack family.
///
/// ⚠ it lives HERE and not on `BodyActionBuffer`: the generic timer is
/// body-core state and the semantic intent is combat's, exactly as the attack
/// slot is split today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecialGestureIntent {
    /// Facing-relative, resolved at the instant of the press.
    pub direction: AttackDir,
    /// Where the body was standing when it asked.
    pub posture: AttackPosture,
}

/// All semantic attack edges produced this tick. Press and release are separate
/// so a tap that begins and ends between simulation ticks remains lossless.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResolvedAttackGesture {
    pub pressed: Option<AttackGestureIntent>,
    pub held: Option<AttackGestureIntent>,
    pub released: Option<AttackGestureIntent>,
    /// The SPECIAL press, live or replayed from the buffer — ONE field
    /// downstream, so nothing has to learn that buffering exists. The same
    /// shape [`Self::pressed`] already has for the attack family.
    pub special: Option<SpecialGestureIntent>,
    /// The special button is DOWN — the sustain beside [`Self::special`]'s edge,
    /// exactly as [`Self::held`] sits beside [`Self::pressed`].
    ///
    /// A `bool` and not an intent, because the only question anyone asks of it
    /// is whether the hold continues: a chargeable neutral special resolved its
    /// direction and posture at the press, and re-deriving either from a button
    /// still being down is the mistake [`SpecialGestureIntent`] exists to
    /// prevent.
    pub special_held: bool,
}

/// Ruleset/player-owned interpretation thresholds. This component is required
/// by [`crate::control::ActorControl`]; a participant or ruleset may replace the
/// default on that body. Character definitions must not tune these values.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct AttackGestureTuning {
    /// Axis magnitude that begins a directional flick.
    pub flick_threshold: f32,
    /// Axis magnitude below which a future flick is re-armed.
    pub rearm_threshold: f32,
    /// Number of simulation ticks after a flick in which Attack counts as Smash.
    pub flick_window_ticks: u8,
    /// Directional deadzone used when reducing an axis to [`AttackDir`].
    pub directional_deadzone: f32,
    /// How long a COMBAT press stays spendable after its edge, in seconds of
    /// the owner's proper time.
    ///
    /// The one knob behind every combat verb's input leniency: the attack
    /// press buffered here, and the bare grab/pogo/special edges timed by
    /// `BodyActionBuffer`. It lives beside the flick thresholds because it is
    /// the same KIND of value — a ruleset's reading of a human's hands, never a
    /// character's property — and a fighter that bought itself a longer buffer
    /// than its opponent would be a different game.
    ///
    /// `0.0` disables buffering: a press is spendable only on the tick it
    /// arrives, which is the behaviour every body had before this existed.
    pub action_buffer_s: f32,
    /// How far the stick must lean, sideways, for the SPECIAL TURN to read a
    /// direction — the B-reverse edge.
    ///
    /// ⛔⛔ ITS OWN KNOB BECAUSE IT IS ITS OWN MECHANIC, and sharing the attack
    /// flick's was a quiet contradiction. The special turn reads a direction
    /// past this; the ordinary attack flick needs
    /// [`Self::flick_threshold`] (0.8) after re-arming below
    /// [`Self::rearm_threshold`] (0.35). So a 0.65 deflection — which
    /// [`TILT_DEFLECTION`] deliberately picks because it is a TILT and not a
    /// flick — was simultaneously "not a flick" to the attack gesture and "a
    /// flick" to the B-reverse recognizer, off one authored number.
    ///
    /// ⭐ AND THE SOFTER READING IS THE RIGHT ONE, which is why this is a second
    /// knob rather than a correction to one. The genre's B-reverse is a stick
    /// TAP backward during a special, not a smash-strength input: requiring a
    /// full flick would make the technique harder than the games it comes from.
    /// Default `0.5` is what the recognizer already used (it borrowed
    /// [`Self::directional_deadzone`]), so this names existing behaviour rather
    /// than changing it.
    pub special_turn_deflection: f32,
    /// SUBSEQUENT simulation ticks in which a lateral edge still turns the
    /// special this body just started.
    ///
    /// ⭐ SUBSEQUENT, and that word is the other half of the same
    /// contradiction. The ordinary attack flick keeps a recorded flick while
    /// `age_ticks <= flick_window_ticks`, counting ticks AFTER the flick; the
    /// special turn borrowed the same number and then spent one of it on the
    /// press tick itself, so `4` meant four for one mechanic and three for the
    /// other. Here it means four ticks after the press, and the acceptance tick
    /// is free — see `start_move`.
    pub special_turn_window_ticks: u8,
}

impl Default for AttackGestureTuning {
    fn default() -> Self {
        Self {
            flick_threshold: 0.8,
            rearm_threshold: 0.35,
            flick_window_ticks: 4,
            directional_deadzone: 0.5,
            // Six ticks at 60Hz — the platform-fighter house range. A knob,
            // not a measurement: tune it against play, do not scatter
            // per-move grace timers beside it.
            action_buffer_s: 0.1,
            // Both defaults preserve exactly what the recognizer did before it
            // had knobs of its own: it read `directional_deadzone` and spent
            // `flick_window_ticks`.
            special_turn_deflection: 0.5,
            special_turn_window_ticks: 4,
        }
    }
}

/// How far the brain pushes the stick for a move that is NOT a smash.
///
/// between the body's `directional_deadzone` (0.5) and its `flick_threshold`
/// (0.8), and both halves are load-bearing: below the deadzone the direction
/// does not register at all and the press falls back to the neutral move; at or
/// above the flick threshold [`crate::actor::attack_gesture::resolve_attack_gesture`]
/// records a FLICK, and a press inside the flick window is a smash whatever
/// the strength hint says (`strong_hint || recent_matches`).
///
/// the numbers it sits between are `AttackGestureTuning`'s DEFAULTS, and the
/// brain cannot see a body's tuning. A body that retunes them far enough to
/// swallow this deflection loses the CPU's tilts and keeps everything else; that
/// is a coupling worth stating rather than a fact worth threading, because the
/// same partial-deflection-means-tilt convention is what a human's stick obeys.
pub const TILT_DEFLECTION: f32 = 0.65;

/// The lateral stick sign the SPECIAL TURN reads, reduced to -1, 0 or 1.
///
/// ⛔⛔ THE STICK THE PLAYER IS HOLDING, NOT THE ONE THE BODY MAY MOVE BY. The
/// actor update publishes the DAMPED frame back onto the component after
/// integration, so locomotion reads zero for the whole of a rooted move — and a
/// special with a `motion_scale: 0.0` tail is how this repository authors a
/// commitment. Reading that would make the B-reverse impossible on exactly the
/// moves that most want it.
///
/// ⭐ ONE function because TWO sites need the identical answer: the accepted
/// special seeds `AttackGestureState::prev_lateral_sign` with it, and the
/// post-press recognizer compares against it. A second hand-written copy of this
/// expression is how a seed and a comparison drift into disagreeing about what
/// the player was holding.
///
/// ⛔⛔ IT WAS CALLED `lateral_flick_sign` AND READ `directional_deadzone`, AND
/// BOTH WERE WRONG IN THE SAME WAY. "Flick" is the attack gesture's word for a
/// deflection past `flick_threshold` (0.8) after re-arming under 0.35; this
/// reads a much softer lean and always did. One authored number therefore
/// answered two different questions, and a 0.65 deflection — the exact value
/// [`TILT_DEFLECTION`] picks BECAUSE it is a tilt — was a tilt to one consumer
/// and a flick to the other. The soft reading is correct for a B-reverse; what
/// was wrong was calling it the same mechanic.
pub fn special_turn_stick_sign(
    control: &crate::control::ActorControl,
    tuning: &AttackGestureTuning,
) -> f32 {
    let lateral = control.0.steer_axis().vec().x;
    if lateral.abs() > tuning.special_turn_deflection {
        lateral.signum()
    } else {
        0.0
    }
}

/// The direction the player is ASKING FOR this tick, or `None` for a stick at
/// rest.
///
/// ⭐⭐ THE TWIN OF [`special_turn_stick_sign`], reading the same axis for the
/// same reason: `update.rs` publishes the DAMPED frame back onto the component
/// after integration, so a body inside a move authored with `motion_scale: 0.0`
/// — which is how this repository writes a COMMITMENT — reports a neutral
/// `locomotion` for the move's whole duration. A technique that aimed itself by
/// `locomotion` therefore could not be aimed at all on exactly the moves that
/// most want it, which is every special worth aiming.
///
/// ⛔⛔ AND `None` MEANS NEUTRAL, NOT "USE FACING". A caller states its own
/// fallback, because the honest one differs: a teleport recovery's is straight
/// UP, and the facing fallback the held-item aim helper takes had every unaimed
/// recovery leaving sideways off the stage.
///
/// ⛔ THE BODY'S OWN DEADZONE decides, not a constant of this function's own.
/// "Has the stick been pushed far enough to mean a direction" is the question
/// [`AttackGestureTuning::directional_deadzone`] already answers for the attack
/// family, and a second number beside it would be two knobs for one feel.
pub fn aimed_stick_direction(
    control: &crate::control::ActorControl,
    tuning: &AttackGestureTuning,
) -> Option<ae::Vec2> {
    let stick = control.0.steer_axis().vec();
    (stick.length() > tuning.directional_deadzone).then_some(stick)
}

/// A recently detected directional flick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecentAttackFlick {
    pub direction: AttackDir,
    pub age_ticks: u8,
}

/// Authoritative per-body gesture history. This is rollback state: restoring in
/// the middle of a flick window must classify the replayed press identically.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct AttackGestureState {
    pub flick_armed: bool,
    pub recent_flick: Option<RecentAttackFlick>,
    pub active: Option<AttackGestureIntent>,
    /// The press the action authority has not accepted yet, replayed VERBATIM
    /// on every tick of its buffer window.
    ///
    /// It is the intent and not the raw input because a press is only
    /// classifiable at the instant it happens: the flick that made it a smash
    /// ages out within a few ticks, so re-resolving a buffered press from the
    /// live stick would quietly downgrade every buffered smash to a tilt.
    ///
    /// Its CLOCK is `BodyActionBuffer::attack` — the body-generic verb window —
    /// and the two are armed and cleared together. Meaningless (and forced back
    /// to `None`) whenever that window is zero.
    pub buffered_press: Option<AttackGestureIntent>,
    /// The SPECIAL press the action authority has not accepted yet.
    ///
    /// The twin of [`Self::buffered_press`] and armed by the same one system,
    /// with [`ambition_platformer2d_core::BodyActionBuffer::special`] for its
    /// clock — see [`SpecialGestureIntent`] for why a timer alone was not
    /// enough.
    pub buffered_special: Option<SpecialGestureIntent>,
    /// Simulation ticks left in which a lateral FLICK still turns the special
    /// this body just started — the B-reverse window.
    ///
    /// ⛔⛔ TICKS, NOT SECONDS, and that was a real defect rather than a style
    /// choice. It was `f32` seconds aged by `WorldTime::sim_dt()`, which is
    /// SCALED by pause, hitstop and bullet time — while the ordinary attack
    /// flick this window is authored from ([`AttackGestureTuning::flick_window_ticks`],
    /// and [`RecentAttackFlick::age_ticks`] beside it) counts integer ticks. One
    /// authored knob had two clock semantics, so a match at half time scale gave
    /// the player twice as many ticks of B-reverse opportunity as an ordinary
    /// smash-flick window from the same number.
    ///
    /// ⭐⭐ THE TECHNIQUES ARE TWO TOGGLES, NOT THREE NAMES. Each qualifying
    /// input flips the facing, and a flick AFTER the press also reverses the
    /// lateral drift:
    ///
    /// ```text
    /// back BEFORE the press       flip                   → turnaround-B
    /// back flick AFTER the press  flip + reverse drift   → B-reverse
    /// both                        flip twice (= no flip)
    ///                             + reverse drift        → WAVEBOUNCE
    /// ```
    ///
    /// ⛔ SO THE FOURTH OUTCOME NEEDS NO RECOGNITION OF ITS OWN, which is why
    /// this is one timer rather than a gesture vocabulary. The pre-press half is
    /// `special_turn`, committed where the move starts; this is the other half,
    /// and it must apply a few ticks INTO the move — which is what the genre
    /// does and why it cannot be decided at move-start.
    ///
    /// ⛔ ARMED WHERE THE MOVE IS ACCEPTED, never where the press is resolved: a
    /// press that starts nothing turns nobody.
    pub special_turn_ticks: u8,
    /// The lateral stick sign this body was holding last tick, for the flick
    /// edge above. Same shape and the same reason as the movement kernel's
    /// `prev_steer_dir`: a flick is an EDGE, and an edge needs a memory.
    ///
    /// ⛔⛔ AND THE ACCEPTED SPECIAL SEEDS IT, which is the whole of the
    /// same-frame bug. Production runs `CombatSet::Trigger` before
    /// `CombatSet::Playback`; a fresh Back+Special on one tick was accepted in
    /// Trigger — flipping the facing once as a turnaround-B — and then read
    /// again in Playback against a memory of NEUTRAL, so the very stick that
    /// bought the press counted a second time as a post-press flick. The facing
    /// flipped twice and the drift reversed: a wavebounce out of an ordinary
    /// fresh turnaround. Holding Back for one frame first hid it, because by
    /// then this field already said Back.
    ///
    /// ⇒ seeded at ACCEPTANCE from [`lateral_flick_sign`] — the same reading the
    /// recognizer makes — rather than carrying a second baseline field beside
    /// this one. There is only ever one memory, so the two cannot disagree.
    pub prev_lateral_sign: f32,
}

impl Default for AttackGestureState {
    fn default() -> Self {
        Self {
            flick_armed: true,
            recent_flick: None,
            active: None,
            buffered_press: None,
            buffered_special: None,
            special_turn_ticks: 0,
            prev_lateral_sign: 0.0,
        }
    }
}

/// Reduce an attack axis to a facing-relative direction. Vertical wins ties so
/// a clear up/down aim is not lost to slight horizontal drift.
/// takes [`ae::LocalAxes`] rather than a bare `Vec2`: `axis.x * facing` is a
/// body-LOCAL side product, and reading a world vector here would pick the wrong
/// tilt under any rotated gravity.
pub fn attack_dir_from_axis(axis: ae::LocalAxes, facing: f32, deadzone: f32) -> AttackDir {
    let forward = axis.x * facing;
    if axis.y.abs() >= axis.x.abs() && axis.y.abs() > deadzone {
        if axis.y < 0.0 {
            AttackDir::Up
        } else {
            AttackDir::Down
        }
    } else if forward > deadzone {
        AttackDir::Forward
    } else if forward < -deadzone {
        AttackDir::Back
    } else {
        AttackDir::Neutral
    }
}

fn posture(grounded: bool) -> AttackPosture {
    if grounded {
        AttackPosture::Grounded
    } else {
        AttackPosture::Airborne
    }
}

/// Advance one body's interpreter by one simulation tick.
///
/// `hint` is device-independent. A C-stick adapter, dedicated smash key,
/// replay, remote peer, or RL policy may set it; the accumulated flick history
/// remains body-local and is never streamed as resolved state.
pub fn resolve_attack_gesture(
    state: &mut AttackGestureState,
    tuning: AttackGestureTuning,
    axis: ae::LocalAxes,
    facing: f32,
    grounded: bool,
    pressed: bool,
    held: bool,
    released: bool,
    hint: ae::AttackStrengthHint,
) -> ResolvedAttackGesture {
    if let Some(mut flick) = state.recent_flick {
        flick.age_ticks = flick.age_ticks.saturating_add(1);
        state.recent_flick = (flick.age_ticks <= tuning.flick_window_ticks).then_some(flick);
    }

    let magnitude = axis.length();
    if magnitude <= tuning.rearm_threshold {
        state.flick_armed = true;
    } else if state.flick_armed && magnitude >= tuning.flick_threshold {
        let direction = attack_dir_from_axis(axis, facing, tuning.directional_deadzone);
        if direction != AttackDir::Neutral {
            state.recent_flick = Some(RecentAttackFlick {
                direction,
                age_ticks: 0,
            });
            state.flick_armed = false;
        }
    }

    let mut out = ResolvedAttackGesture::default();
    if pressed {
        let direction = attack_dir_from_axis(axis, facing, tuning.directional_deadzone);
        let recent_matches = state
            .recent_flick
            .is_some_and(|flick| flick.direction == direction);
        let base = AttackGestureIntent {
            direction,
            // ⭐⭐ AN EXPLICIT HINT OVERRULES THE STICK, IN BOTH DIRECTIONS.
            // This was `strong_hint || recent_matches`, which could only ever
            // ADD a smash: a tilt-stick pushed to full deflection armed a flick
            // on the way out, the flick matched the press direction, and the
            // interpreter returned Smash however the device asked. A full
            // deflection could not be a tilt.
            //
            // ⛔ AND THE WORKAROUND WOULD HAVE BEEN WORSE. Without this arm a
            // right-stick adapter has to spoof stick history — deflect just
            // enough to name a direction and not enough to arm the flick — which
            // throws away how hard the player actually pushed and makes the
            // adapter re-implement the interpreter's own thresholds.
            strength: match hint {
                ae::AttackStrengthHint::Tilt => AttackStrength::Tilt,
                ae::AttackStrengthHint::Smash => AttackStrength::Smash,
                ae::AttackStrengthHint::Auto => {
                    if recent_matches {
                        AttackStrength::Smash
                    } else {
                        AttackStrength::Tilt
                    }
                }
            },
            posture: posture(grounded),
            phase: AttackInputPhase::Press,
        };
        state.active = Some(base);
        out.pressed = Some(base);
    }

    if held {
        if let Some(active) = state.active {
            out.held = Some(active.with_phase(AttackInputPhase::Hold));
        }
    }

    if released {
        if let Some(active) = state.active {
            out.released = Some(active.with_phase(AttackInputPhase::Release));
        }
        state.active = None;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(
        state: &mut AttackGestureState,
        tuning: AttackGestureTuning,
        axis: ae::LocalAxes,
        facing: f32,
        pressed: bool,
        held: bool,
        released: bool,
        strong: bool,
    ) -> ResolvedAttackGesture {
        let hint = if strong {
            ae::AttackStrengthHint::Smash
        } else {
            ae::AttackStrengthHint::Auto
        };
        resolve_attack_gesture(
            state, tuning, axis, facing, true, pressed, held, released, hint,
        )
    }

    /// A FULL DEFLECTION CAN BE A TILT, and until 2026-08-31 it could not.
    ///
    /// ⭐⭐ THIS IS THE WHOLE POINT OF THE THREE-VALUED HINT. The old
    /// `strong_hint: bool` was one-way — `strong_hint || recent_matches` — so it
    /// could only ever ADD a smash. A right-stick tilt mode exists to throw
    /// tilts at full deflection, and the deflection itself armed a flick that
    /// matched the press direction, so the interpreter returned `Smash` however
    /// the device asked.
    ///
    /// ⛔ THE ARMS ARE THE POINT, not the values. Both presses here use the SAME
    /// hard deflection and the SAME flick history; only the hint differs. An
    /// arm that tilted by pushing less would be measuring the flick threshold
    /// and agreeing with the bug.
    #[test]
    fn an_explicit_hint_overrules_the_stick_in_both_directions() {
        let tuning = AttackGestureTuning::default();
        // Past `flick_threshold`: on `Auto` this is unambiguously a smash.
        let hard = ae::LocalAxes::new(0.95, 0.0);

        let mut press_with = |hint: ae::AttackStrengthHint| {
            let mut state = AttackGestureState::default();
            // Tick one arms the flick, tick two presses — the two-tick gesture a
            // person actually makes.
            resolve_attack_gesture(
                &mut state, tuning, hard, 1.0, true, false, false, false, hint,
            );
            resolve_attack_gesture(
                &mut state, tuning, hard, 1.0, true, true, false, false, hint,
            )
            .pressed
            .expect("a press was requested")
        };

        // ⛔ THE PREMISE. If the deflection did not read as a smash on its own,
        // the `Tilt` arm below would pass without overruling anything.
        assert_eq!(
            press_with(ae::AttackStrengthHint::Auto).strength,
            AttackStrength::Smash,
            "a full deflection no longer reads as a smash on its own, so this \
             fixture is not measuring an override"
        );
        assert_eq!(
            press_with(ae::AttackStrengthHint::Tilt).strength,
            AttackStrength::Tilt,
            "a TILT hint did not survive a full deflection — which is the defect \
             the one-way bool had: a tilt-stick could never throw a tilt"
        );

        // And the direction that always worked still does: no flick at all, and
        // the hint alone makes it a smash.
        let mut state = AttackGestureState::default();
        let soft = resolve_attack_gesture(
            &mut state,
            tuning,
            ae::LocalAxes::ZERO,
            1.0,
            true,
            true,
            false,
            false,
            ae::AttackStrengthHint::Smash,
        )
        .pressed
        .expect("a press was requested");
        assert_eq!(soft.strength, AttackStrength::Smash);
    }

    #[test]
    fn forward_and_back_are_facing_relative() {
        let tuning = AttackGestureTuning::default();
        assert_eq!(
            attack_dir_from_axis(ae::LocalAxes::X, 1.0, tuning.directional_deadzone),
            AttackDir::Forward
        );
        assert_eq!(
            attack_dir_from_axis(-ae::LocalAxes::X, -1.0, tuning.directional_deadzone),
            AttackDir::Forward
        );
        assert_eq!(
            attack_dir_from_axis(ae::LocalAxes::X, -1.0, tuning.directional_deadzone),
            AttackDir::Back
        );
    }

    #[test]
    fn recent_flick_makes_the_press_a_smash() {
        let tuning = AttackGestureTuning::default();
        let mut state = AttackGestureState::default();
        tick(
            &mut state,
            tuning,
            ae::LocalAxes::X,
            1.0,
            false,
            false,
            false,
            false,
        );
        let out = tick(
            &mut state,
            tuning,
            ae::LocalAxes::X,
            1.0,
            true,
            true,
            false,
            false,
        );
        assert_eq!(out.pressed.unwrap().strength, AttackStrength::Smash);
    }

    #[test]
    fn expired_flick_makes_the_press_a_tilt() {
        let tuning = AttackGestureTuning {
            flick_window_ticks: 1,
            ..Default::default()
        };
        let mut state = AttackGestureState::default();
        tick(
            &mut state,
            tuning,
            ae::LocalAxes::X,
            1.0,
            false,
            false,
            false,
            false,
        );
        tick(
            &mut state,
            tuning,
            ae::LocalAxes::X,
            1.0,
            false,
            false,
            false,
            false,
        );
        tick(
            &mut state,
            tuning,
            ae::LocalAxes::X,
            1.0,
            false,
            false,
            false,
            false,
        );
        let out = tick(
            &mut state,
            tuning,
            ae::LocalAxes::X,
            1.0,
            true,
            true,
            false,
            false,
        );
        assert_eq!(out.pressed.unwrap().strength, AttackStrength::Tilt);
    }

    #[test]
    fn strong_hint_does_not_require_a_flick() {
        let tuning = AttackGestureTuning::default();
        let mut state = AttackGestureState::default();
        let out = tick(
            &mut state,
            tuning,
            ae::LocalAxes::ZERO,
            1.0,
            true,
            true,
            false,
            true,
        );
        assert_eq!(out.pressed.unwrap().strength, AttackStrength::Smash);
        assert_eq!(out.pressed.unwrap().direction, AttackDir::Neutral);
    }

    #[test]
    fn press_hold_release_keep_the_initial_semantics() {
        let tuning = AttackGestureTuning::default();
        let mut state = AttackGestureState::default();
        let press = tick(
            &mut state,
            tuning,
            ae::LocalAxes::Y,
            1.0,
            true,
            true,
            false,
            true,
        );
        let hold = tick(
            &mut state,
            tuning,
            -ae::LocalAxes::X,
            -1.0,
            false,
            true,
            false,
            false,
        );
        let release = tick(
            &mut state,
            tuning,
            ae::LocalAxes::ZERO,
            1.0,
            false,
            false,
            true,
            false,
        );
        assert_eq!(press.pressed.unwrap().direction, AttackDir::Down);
        assert_eq!(hold.held.unwrap().direction, AttackDir::Down);
        assert_eq!(release.released.unwrap().direction, AttackDir::Down);
        assert_eq!(release.released.unwrap().phase, AttackInputPhase::Release);
        assert!(state.active.is_none());
    }

    #[test]
    fn sub_tick_tap_emits_press_and_release_together() {
        let tuning = AttackGestureTuning::default();
        let mut state = AttackGestureState::default();
        let out = tick(
            &mut state,
            tuning,
            ae::LocalAxes::ZERO,
            1.0,
            true,
            false,
            true,
            false,
        );
        assert!(out.pressed.is_some());
        assert!(out.released.is_some());
        assert!(state.active.is_none());
    }
}

#[cfg(test)]
mod special_turn_edge_tests {
    use super::*;
    use crate::actor::control::ActorControlFrame;
    use crate::control::ActorControl;

    fn holding(lateral: f32) -> ActorControl {
        let mut frame = ActorControlFrame::neutral();
        frame.locomotion = ae::LocalAxes::new(lateral, 0.0);
        ActorControl(frame)
    }

    /// ⛔⛔ ONE DEFLECTION, TWO MECHANICS, TWO ANSWERS — ON PURPOSE NOW.
    ///
    /// [`TILT_DEFLECTION`] is 0.65 and its own doc says why: above
    /// `directional_deadzone` (0.5) so the direction registers, below
    /// `flick_threshold` (0.8) so the press is a TILT and not a smash. The
    /// special turn reads a much softer lean than the attack flick — a
    /// B-reverse in this genre is a stick tap, not a smash input — so 0.65
    /// genuinely is an edge to one and not to the other.
    ///
    /// ⛔ WHAT WAS WRONG WAS THAT ONE NUMBER SAID BOTH. The recognizer borrowed
    /// `directional_deadzone` and the window borrowed `flick_window_ticks`, so
    /// this difference existed with nothing naming it — a knob tuned for the
    /// attack gesture silently moved the B-reverse, in the opposite direction to
    /// what its name suggested. The behaviour is unchanged; what is new is that
    /// the split is authored.
    ///
    /// ⭐ THE ARMS ARE A TABLE, not a single assertion: 0.65 must be an edge to
    /// the special turn AND not a flick to the attack gesture, and 0.85 must be
    /// both. Either half alone is satisfied by a threshold that swallowed the
    /// other mechanic entirely.
    #[test]
    fn a_tilt_strength_lean_turns_a_special_and_is_not_an_attack_flick() {
        let tuning = AttackGestureTuning::default();

        assert_eq!(
            special_turn_stick_sign(&holding(TILT_DEFLECTION), &tuning),
            1.0,
            "a tilt-strength lean did not register for the special turn, which \
             makes the B-reverse a smash-strength input the genre does not ask for"
        );
        let mut state = AttackGestureState::default();
        resolve_attack_gesture(
            &mut state,
            tuning,
            ae::LocalAxes::new(TILT_DEFLECTION, 0.0),
            1.0,
            true,
            false,
            false,
            false,
            ae::AttackStrengthHint::Auto,
        );
        assert!(
            state.recent_flick.is_none(),
            "the attack gesture recorded a FLICK at tilt strength, so \
             `TILT_DEFLECTION`'s whole reason for sitting below \
             `flick_threshold` is gone"
        );

        // …and a real flick is both.
        let hard = 0.85;
        assert_eq!(special_turn_stick_sign(&holding(hard), &tuning), 1.0);
        let mut state = AttackGestureState::default();
        resolve_attack_gesture(
            &mut state,
            tuning,
            ae::LocalAxes::new(hard, 0.0),
            1.0,
            true,
            false,
            false,
            false,
            ae::AttackStrengthHint::Auto,
        );
        assert!(
            state.recent_flick.is_some(),
            "a deflection past `flick_threshold` was not a flick, so the arm \
             above proves nothing about the two mechanics differing"
        );
    }
}
