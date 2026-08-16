//! **Tumble → knockdown → tech → getup**: what happens to a body between the
//! hit that launched it and the moment it is standing again.
//!
//! ⛔ **the engine had none of this.** A launched body flew, landed, and was
//! instantly running again — so a big hit and a small one differed only in how
//! far you travelled, and there was no floor game at all. Every established
//! platform fighter builds its neutral out of exactly the states below, and the
//! reason is not decoration: without knockdown there is nothing to punish, and
//! without a tech there is no way to refuse the punish.
//!
//! ```text
//!   a hit launches you            ── tumble ──────────────────────────┐
//!   (only above the body's                     │ press the evade      │
//!    authored threshold)                       │ button near a        │
//!                                              ▼ surface             ▼
//!   you hit the floor still tumbling       ── tech ──          ── knockdown ──
//!                                          i-frames,           prone; getup by
//!                                          instant recovery    roll / attack /
//!                                                              stand / timeout
//!   a mistimed tech                        ── lockout ──  no tech for a while
//! ```
//!
//! ⭐ **body-generic and AUTHORED, exactly like the air dodge.** A body whose
//! tuning leaves [`crate::movement::TraversalAbilityTuning::tumble_speed`] at
//! `0.0` never tumbles, which is every body in the game until one says
//! otherwise — a wandering enemy that got knocked down and had to stand up would
//! be a different game for the exploration side, and that is not a decision the
//! fighter demo gets to make for it.

use super::input::InputState;
use super::model::AxisManeuverState;
use super::ops::MovementOp;
use super::tuning::AxisSweptParams;
use crate::body_clusters::{BodyComboTrace, BodyGroundState, BodyKinematics};
use crate::MotionFrame;

/// How long a body stays in tumble after a launch, per unit of launch speed
/// over its threshold — clamped to [`MAX_TUMBLE_TIME`]. A harder hit keeps you
/// helpless longer, which is what makes a big hit *feel* big beyond the
/// distance travelled.
pub const TUMBLE_TIME_PER_SPEED: f32 = 0.0016;
/// The ceiling on a tumble, so an enormous launch is still a finite loss of
/// control rather than a body watching itself fly.
pub const MAX_TUMBLE_TIME: f32 = 1.4;

/// The window a tech press stays live: press it this long before touching a
/// surface and the landing is teched. ~20 frames, the Ultimate window.
pub const TECH_WINDOW: f32 = 20.0 / 60.0;
/// A tech press that expires without touching anything locks the option out.
/// ~40 frames — long enough that mashing is worse than reading.
pub const TECH_LOCKOUT: f32 = 40.0 / 60.0;
/// Invulnerability granted by a successful tech, and by standing up.
pub const GETUP_INVULN: f32 = 0.30;
/// A teched landing keeps you moving if you were holding a direction.
pub const TECH_ROLL_SPEED: f32 = 360.0;
/// Prone time before the body stands up on its own.
pub const KNOCKDOWN_TIME: f32 = 0.55;
/// A getup roll's travel speed.
pub const GETUP_ROLL_SPEED: f32 = 320.0;

/// **Launch this body into tumble** — the ONE entry point, called by whoever
/// resolved the knockback.
///
/// ⚠ maneuver state is model-private (ADR 0024), so the combat side cannot
/// write the timer itself; it says *this body was launched at this speed* and
/// the kernel decides whether that is a tumble at all. `tumble_speed <= 0.0`
/// (the default) means this body does not tumble, and the call is a no-op.
pub fn launch_into_tumble(
    state: &mut AxisManeuverState,
    tuning: AxisSweptParams,
    launch_speed: f32,
) -> bool {
    let threshold = tuning.abilities.tumble_speed;
    if threshold <= 0.0 || launch_speed < threshold {
        return false;
    }
    let over = launch_speed - threshold;
    state.tumble_timer = state
        .tumble_timer
        .max((TUMBLE_TIME_PER_SPEED * over).min(MAX_TUMBLE_TIME));
    // A launch cancels the floor game the body was in: you cannot be prone and
    // airborne, and a getup's i-frames do not survive being hit again.
    state.tumble_until_landing = true;
    state.knockdown_timer = 0.0;
    state.getup_invuln_timer = 0.0;
    state.tumble_unannounced = true;
    true
}

/// **Is the floor game holding the controller this tick?** The simulation half
/// asks this so both phases neutralize the same input; the control half learns
/// it from [`tick_knockdown`]'s return, which is the same answer computed once.
pub(super) fn owns_control(state: &AxisManeuverState) -> bool {
    state.knockdown_timer > 0.0 || (state.tumble_until_landing && state.tumble_timer > 0.0)
}

/// Per-tick tumble/knockdown/tech/getup resolution.
///
/// Returns the input the REST of the step may act on. Three cases, and the
/// middle one is the reason this returns an input rather than a bool:
///
/// * helpless or prone → neutral; nothing else this body does is a choice.
/// * tumbling with control back → the same input MINUS the evade press, because
///   while tumbling that press means *tech* and nothing else. ⛔ measured: with
///   the press passed through, a tech attempt late in a tumble came out as an
///   AIR DASH that zeroed the launch and stalled the body in mid-air.
/// * not in the floor game → the input, untouched. ⚠ it does NOT short-circuit the step the way an active ledge grab
/// does: a knocked-down body still integrates, still resolves contacts, and
/// still falls if the floor is removed underneath it.
#[allow(clippy::too_many_arguments)]
pub(super) fn tick_knockdown(
    kinematics: &mut BodyKinematics,
    state: &mut AxisManeuverState,
    ground: &BodyGroundState,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
    events: &mut super::events::FrameEvents,
) -> InputState {
    let dec = |v: f32| (v - dt).max(0.0);
    state.getup_invuln_timer = dec(state.getup_invuln_timer);
    state.tech_lockout_timer = dec(state.tech_lockout_timer);

    if state.knockdown_timer > 0.0 {
        state.knockdown_timer = dec(state.knockdown_timer);
        // See the note at the landing below: a prone body owes nothing to a
        // press that happened while it was in the air.
        state.buffer_burst = 0.0;
        state.buffer_jump = 0.0;
        resolve_getup(kinematics, state, input, frame, combo_trace, events);
        return InputState::default();
    }

    if state.tumble_unannounced {
        state.tumble_unannounced = false;
        events.op_clusters(combo_trace, MovementOp::Tumble);
    }

    if !state.tumble_until_landing {
        // Not tumbling: a stale tech press is just an ordinary press.
        state.tech_press_timer = 0.0;
        return input;
    }
    let helpless = state.tumble_timer > 0.0;
    state.tumble_timer = dec(state.tumble_timer);

    // The tech input is the EVADE button — the same press that means dodge
    // everywhere else. Ultimate techs with shield; `InputState::shield_held`
    // carries no rising edge in this kernel, and inventing one here would be a
    // second source of truth about when a button went down. While tumbling
    // every other meaning of the press is gone anyway, so it is unambiguous.
    if input.dash_pressed() && state.tech_lockout_timer <= 0.0 && state.tech_press_timer <= 0.0 {
        state.tech_press_timer = TECH_WINDOW;
    }
    if state.tech_press_timer > 0.0 {
        state.tech_press_timer = dec(state.tech_press_timer);
        // Expired without touching anything: that was a guess, and a guess costs
        // the option. This is the half that makes teching a READ.
        if state.tech_press_timer <= 0.0 {
            state.tech_lockout_timer = TECH_LOCKOUT;
        }
    }

    if !ground.on_ground {
        // ⭐ **control comes back before the tumble does.** Once the helpless
        // window has passed, a jump / attack / evade press ACTS OUT of the
        // tumble — the escape that makes a launch a situation rather than a
        // sentence — and the landing that follows is an ordinary one.
        if !helpless {
            // ⛔ **not the evade button.** While tumbling that press already
            // MEANS tech, and letting it also mean "act out of tumble" would
            // make the tech unreachable: every timed press would cancel the
            // tumble it was trying to survive, so the landing it was aimed at
            // was never a knockdown in the first place. Jump and attack are the
            // act-out verbs; the evade stays the floor game's.
            if input.jump_pressed() || input.attack_pressed {
                state.tumble_until_landing = false;
                state.tech_press_timer = 0.0;
                return input;
            }
            return without_evade(input);
        }
        return InputState::default();
    }

    // Touched down while still tumbling — the moment the floor game is decided.
    //
    // ⛔ **and the input buffers do not survive it.** Measured: a tech press
    // that missed its window still sat in `buffer_burst`, so the body that hit
    // the floor emitted `[DodgeRoll, Knockdown]` on the same tick — it dodge
    // rolled out of a knockdown it was simultaneously entering. Neutralizing the
    // input for the rest of the step does not help: a buffer is input that
    // already happened.
    state.buffer_burst = 0.0;
    state.buffer_jump = 0.0;
    state.tumble_timer = 0.0;
    state.tumble_until_landing = false;
    let local_stick = input.local_axis();
    if state.tech_press_timer > 0.0 {
        state.tech_press_timer = 0.0;
        state.getup_invuln_timer = GETUP_INVULN;
        kinematics.vel = if local_stick.x.abs() > 0.5 {
            frame.side() * (local_stick.x.signum() * TECH_ROLL_SPEED)
        } else {
            crate::Vec2::ZERO
        };
        events.op_clusters(combo_trace, MovementOp::Tech);
        return without_evade(input);
    }
    state.knockdown_timer = KNOCKDOWN_TIME;
    kinematics.vel = crate::Vec2::ZERO;
    events.op_clusters(combo_trace, MovementOp::Knockdown);
    InputState::default()
}

/// The input with the evade press removed — the tech consumed it.
fn without_evade(mut input: InputState) -> InputState {
    input
        .movement
        .set_pressed(super::input::MovementAction::Dash, false);
    input
}

/// The prone body's options. Any of them ends the knockdown immediately; doing
/// nothing ends it when the timer runs out.
fn resolve_getup(
    kinematics: &mut BodyKinematics,
    state: &mut AxisManeuverState,
    input: InputState,
    frame: MotionFrame,
    combo_trace: &mut BodyComboTrace,
    events: &mut super::events::FrameEvents,
) {
    let local_stick = input.local_axis();
    let stand = |state: &mut AxisManeuverState| {
        state.knockdown_timer = 0.0;
        state.getup_invuln_timer = GETUP_INVULN;
    };
    if local_stick.x.abs() > 0.5 {
        kinematics.vel = frame.side() * (local_stick.x.signum() * GETUP_ROLL_SPEED);
        stand(state);
        events.op_clusters(combo_trace, MovementOp::GetupRoll);
        return;
    }
    if input.attack_pressed {
        stand(state);
        // The kernel does not swing — it publishes the option, and the combat
        // side answers it, exactly as it does for the ledge getup attack.
        events.op_clusters(combo_trace, MovementOp::GetupAttack);
        return;
    }
    if input.jump_pressed() || state.knockdown_timer <= 0.0 {
        stand(state);
        events.op_clusters(combo_trace, MovementOp::Getup);
    }
}
