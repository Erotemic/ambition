//! Tumble, knockdown, tech, and getup body mechanics.
//!
//! Bodies opt in through authored traversal tuning. A body with zero
//! `tumble_speed` never enters this state machine, so fighter-specific recovery
//! behavior does not leak into unrelated actors.

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
/// A WALL TECH's push off the surface, px/s along the wall's own normal.
///
/// its own number rather than [`TECH_ROLL_SPEED`]'s, because the two are
/// different motions: a tech roll RUNS ALONG the floor it kept its feet on, and
/// a wall tech pushes PERPENDICULAR off a surface it must not stay against.
pub const WALL_TECH_SPEED: f32 = 300.0;

/// How hard a ceiling tech pushes a body back DOWN off the ceiling it caught
/// itself on.
///
/// Smaller than [`WALL_TECH_SPEED`]: gravity is already doing this direction's
/// work, and a ceiling tech is about keeping control of the fall rather than
/// travelling. What it really buys is the [`GETUP_INVULN`] and the end of the
/// tumble, which is what stops a ceiling being a free re-launch for whoever put
/// you there.
pub const CEILING_TECH_SPEED: f32 = 120.0;
/// Prone time before the body stands up on its own.
pub const KNOCKDOWN_TIME: f32 = 0.55;
/// A getup roll's travel speed.
pub const GETUP_ROLL_SPEED: f32 = 320.0;

/// Launch this body into tumble — the ONE entry point, called by whoever
/// resolved the knockback.
///
/// maneuver state is model-private (ADR 0024), so the combat side cannot
/// write the timer itself; it says *this body was launched at this speed* and
/// the kernel decides whether that is a tumble at all. `tumble_speed <= 0.0`
/// (the default) means this body does not tumble, and the call is a no-op.
/// THE JAB LOCK — does this launch PIN a body that is already prone, instead of
/// throwing it? Answers `true` when it pinned, and the caller must then not
/// apply the launch at all.
///
/// ⭐ ASKED AT THE ONE LAUNCH GATEWAY, beside `launch_into_tumble`, and for the
/// same reason: whether a body is prone is model-private maneuver state, and the
/// reaction that resolved the knockback does not hold it. A rule asked anywhere
/// else would be a follow-up call some caller forgets.
///
/// THREE THINGS HAVE TO BE TRUE and each is a different half of the mechanic:
/// the body is prone (there is nothing to lock otherwise), the hit is WEAK
/// enough (a smash launches whatever you are doing), and the lock has not been
/// spent (`jab_lock_limit`) — which is the reset that keeps this from being an
/// infinite.
pub fn jab_lock(state: &mut AxisManeuverState, tuning: AxisSweptParams, launch_speed: f32) -> bool {
    let speed = tuning.abilities.jab_lock_speed;
    if speed <= 0.0 || state.knockdown_timer <= 0.0 {
        return false;
    }
    if launch_speed > speed || state.jab_locks >= tuning.abilities.jab_lock_limit {
        return false;
    }
    state.jab_locks = state.jab_locks.saturating_add(1);
    // The pin RESTARTS the floor game rather than extending it, so a locked
    // body owes the same beat it owed the first time and the attacker's read is
    // the same read. `max` so a pin can never SHORTEN a knockdown already
    // running longer than the standard one.
    state.knockdown_timer = state.knockdown_timer.max(KNOCKDOWN_TIME);
    true
}

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
    enter_tumble(state, TUMBLE_TIME_PER_SPEED * over);
    // ⭐⭐ A HARD ENOUGH LAUNCH CANNOT BE TECHED, and this is the only place that
    // can decide it: the launch SPEED is an input here and is gone by the time
    // the body reaches a surface. Deciding it at the wall would need the impact
    // to reconstruct how hard the hit was, which nothing downstream knows.
    //
    // `0.0` — the default — means every launch is techable, which is what every
    // body did before the rule existed.
    let untechable = tuning.abilities.untechable_launch_speed;
    state.tumble_untechable = untechable > 0.0 && launch_speed >= untechable;
    true
}

/// A footstool put this body into tumble — the same helplessness, with no
/// launch behind it.
///
/// separate from [`launch_into_tumble`] because a footstool's tumble is not
/// proportional to anything. Ultimate's footstool does not produce real
/// knockback, so the duration is AUTHORED
/// ([`crate::FootstoolTuning::air_tumble_time`]) rather than derived from the
/// shove; feeding the shove speed to `launch_into_tumble` instead would put
/// every fighter's footstool below its own tumble threshold and tumble nobody.
///
/// the body's own `tumble_speed` still gates it: a body that never tumbles
/// does not start because somebody stood on it.
pub fn tumble_from_footstool(
    state: &mut AxisManeuverState,
    tuning: AxisSweptParams,
    seconds: f32,
) -> bool {
    if tuning.abilities.tumble_speed <= 0.0 || seconds <= 0.0 {
        return false;
    }
    enter_tumble(state, seconds);
    true
}

/// The floor-game half both tumble entry points share.
///
/// Entering tumble cancels the floor game the body was in: you cannot be prone
/// and airborne, and a getup's i-frames do not survive being put back in the
/// air.
fn enter_tumble(state: &mut AxisManeuverState, seconds: f32) {
    // ⛔ CLEARED ON EVERY ENTRY, so an untechable launch cannot leave the flag
    // latched on the NEXT tumble — a footstool is not a launch, and a body that
    // was once hit hard must not spend the rest of the stock untechable.
    state.tumble_untechable = false;
    state.tumble_timer = state.tumble_timer.max(seconds.min(MAX_TUMBLE_TIME));
    state.tumble_until_landing = true;
    state.knockdown_timer = 0.0;
    state.getup_invuln_timer = 0.0;
    state.tumble_unannounced = true;
}

/// Is the floor game holding the controller this tick? The simulation half
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
///   while tumbling that press means *tech* and nothing else. measured: with
///   the press passed through, a tech attempt late in a tumble came out as an
///   AIR DASH that zeroed the launch and stalled the body in mid-air.
/// * not in the floor game → the input, untouched. it does NOT short-circuit the step the way an active ledge grab
/// does: a knocked-down body still integrates, still resolves contacts, and
/// still falls if the floor is removed underneath it.
#[allow(clippy::too_many_arguments)]
pub(super) fn tick_knockdown(
    kinematics: &mut BodyKinematics,
    state: &mut AxisManeuverState,
    ground: &BodyGroundState,
    // The surface a WALL TECH is taken off, from the previous tick's
    // contact pass. A body slammed into a wall is against it for several ticks,
    // so the frame of latency costs the read nothing.
    wall: &crate::body_clusters::BodyWallState,
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
    // ⭐⭐ A LAUNCH TOO HARD TO TECH REFUSES THE PRESS AT THE SOURCE, not at each
    // surface. The wall, the ceiling and the floor each have their own tech arm,
    // and gating them one by one is how a fourth surface added later quietly
    // becomes techable again — so the PRESS is what an untechable tumble
    // refuses, and all three arms are covered by construction.
    //
    // ⛔ AND IT STILL SPENDS THE LOCKOUT BELOW: a player who mashes into an
    // untechable launch should not be free to keep mashing, which is the same
    // reason a missed tech has a lockout at all.
    // ⛔⛔ AND THE REFUSAL IS CHARGED HERE, WHICH IS WHAT THE SENTENCE ABOVE
    // CLAIMED AND THE CODE DID NOT DO. The lockout is only spent when
    // `tech_press_timer` EXPIRES — and an untechable press never armed that
    // timer, so mashing into a launch too hard to tech cost exactly nothing and
    // every press stayed live. "It still spends the lockout below" was true of
    // the missed-tech road and false of this one.
    //
    // ⭐ ONE GATE, THREE OUTCOMES, in the order a press is actually judged.
    if input.burst_pressed() && state.tech_lockout_timer <= 0.0 && state.tech_press_timer <= 0.0 {
        if state.tumble_untechable {
            // A guess into a launch nobody can tech is still a GUESS, and it
            // costs the option for the same reason a missed tech does.
            state.tech_lockout_timer = TECH_LOCKOUT;
        } else {
            state.tech_press_timer = TECH_WINDOW;
        }
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
        // THE WALL TECH — the floor is not the only thing you can catch
        // yourself on. A launch into a wall was a free continuation for the
        // attacker: the victim kept tumbling, hit the ground still helpless, and
        // the wall it slammed into was worth nothing to it. Ultimate lets that
        // press land on the surface.
        //
        // above the helpless gate on purpose. Being helpless is exactly
        // the state a tech exists to escape, and putting this below `!helpless`
        // would make the wall tech reachable only once the tumble had already
        // let go — which is the tick nobody needs it.
        if wall.on_wall && state.tech_press_timer > 0.0 {
            state.tech_press_timer = 0.0;
            state.tumble_timer = 0.0;
            state.tumble_until_landing = false;
            state.getup_invuln_timer = GETUP_INVULN;
            // OFF the wall along its own normal. Not a pushout: the body's
            // POSITION is untouched, and this is an impulse a timed press
            // earned.
            kinematics.vel = frame.side() * (wall.wall_normal_x * WALL_TECH_SPEED);
            events.op_clusters(combo_trace, MovementOp::Tech);
            return without_evade(input);
        }
        // ⭐ THE CEILING TECH — the last surface a launch can end on, and until
        // now the only one that could not be caught. A body thrown into a
        // ceiling kept its tumble, fell the whole way back down helpless, and
        // arrived on the floor as a knockdown it had no say in: one hit bought
        // the attacker the ceiling AND the landing.
        //
        // Beside the wall arm and above the helpless gate for the same reason
        // that one is: being helpless is the state a tech exists to escape.
        if ground.head_contact && state.tech_press_timer > 0.0 {
            state.tech_press_timer = 0.0;
            state.tumble_timer = 0.0;
            state.tumble_until_landing = false;
            state.getup_invuln_timer = GETUP_INVULN;
            // DOWN, off the surface it caught: the body pushes away from the
            // ceiling the way the wall tech pushes away from the wall, into a
            // fall it now controls.
            kinematics.vel = frame.down() * CEILING_TECH_SPEED;
            events.op_clusters(combo_trace, MovementOp::Tech);
            return without_evade(input);
        }
        // control comes back before the tumble does. Once the helpless
        // window has passed, a jump / attack / evade press ACTS OUT of the
        // tumble — the escape that makes a launch a situation rather than a
        // sentence — and the landing that follows is an ordinary one.
        if !helpless {
            // not the evade button. While tumbling that press already
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
    // and the input buffers do not survive it. Measured: a tech press
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
    // A FRESH knockdown, so the lock budget is fresh too: the count bounds one
    // trip through the floor game, not a whole stock.
    state.jab_locks = 0;
    kinematics.vel = crate::Vec2::ZERO;
    events.op_clusters(combo_trace, MovementOp::Knockdown);
    InputState::default()
}

/// The input with the evade press removed — the tech consumed it.
fn without_evade(mut input: InputState) -> InputState {
    input
        .movement
        .set_pressed(super::input::MovementAction::Burst, false);
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
