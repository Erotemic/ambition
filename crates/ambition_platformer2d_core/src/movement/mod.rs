//! One trusted, frame-aware movement kernel with swappable physics policies.
//!
//! [`step_motion`] is the ONLY movement entry. Every movable body carries one
//! explicit [`MotionModel`], and every policy receives the same immutable
//! [`MotionFrame`] resolved once by the environment from a reference basis and
//! separately retained gravity and external acceleration contributions for that
//! body tick. The active frame
//! is environmental state: it is neither authored into model parameters nor
//! cached in model-private runtime state, and every directional quantity
//! crossing this boundary carries its frame in its type (see
//! [`InputState`]).
//!
//! Axis-swept action-platformer movement, [`surface momentum`](SurfaceMotion),
//! and the [`adhesive crawler`](CrawlerState) are sibling implementations. They
//! own different private state and contact logic, but they share one body-state
//! authority, one typed local-input contract, one world context, one frame, and
//! one deterministic dispatch seam. The phase-level axis functions below are
//! kernel-private implementation vocabulary — production integration calls
//! [`step_motion`], never an individual solver arm.

use crate::world::World;
use crate::MotionFrame;

mod abilities;
mod adhesive_crawler;
mod authority;
mod blink;
pub mod body_contact;
pub(crate) mod collision;
pub mod containment;
mod control;
mod events;
mod facts;
mod input;
mod integration;
mod kernel;
pub mod knockdown;
mod model;
mod ops;
mod player;
pub mod recovery;
mod simulation;
pub(crate) mod surface_momentum;
mod tuning;

pub use adhesive_crawler::{AdhesiveCrawlerMotion, CrawlAttachment, CrawlerParams, CrawlerState};
pub use surface_momentum::{
    DepthOcclusions, MomentumParams, OcclusionSpan, RouteDeparture, SurfaceMotion, SurfaceRef,
};

pub use abilities::{
    resolve_burst_maneuver, resolve_shield, spend_out_of_shield, BurstManeuver, OutOfShieldAction,
    OutOfShieldGate,
};
pub use blink::{blink_destination_clusters, blink_destination_to_point_clusters};
pub use body_contact::{constrain_motion, BodyContactBlocker, BodyContactField};
// The ONE hazard-touch rule, exported so external observers apply the SAME
// predicate the kernel applies — never a duplicated near-copy.
pub use authority::{
    arrive_body_in_room, carry_body, constrain_body_pose, halt_body, reconcile_transit,
    shift_frozen_body, transit_body, ArrivalMomentum, TransitVelocity,
};
pub use collision::{hazard_contact_on_path, touching_hazard_aabb, touching_rebound_aabb};
pub use events::{BlinkEvent, FrameEvents, GroundContactTransition, ResetCause};
pub use facts::{BodyMotionFacts, LedgeFacts, PoseOwnedExternally};
pub use input::{ActionEdges, ActionKey, Edge, InputState, MovementAction};
/// Screen-vertical input → gravity-relative "descend" intent (the vertical
/// sibling of the run-axis transform). Every crouch/pogo/drop-through/fast-fall
/// gate and gravity-relative vertical movement reads input through this so a
/// gravity flip moves them all together. See its doc for the convention.
pub use integration::gravity_descend;
/// The canonical "launch at `speed` opposite `gravity_dir`" velocity primitive
/// shared by jump, wall-kick, and pogo so a gravity flip moves them all. Any
/// pogo/jump impulse outside the engine (e.g. the sandbox attack path) MUST go
/// through this instead of a hardcoded `vel.y = -speed`.
pub use integration::set_jump_velocity;
/// The actor-generic normal-mode physics SPINE: gravity-relative gravity, run,
/// fast-fall/glide gates, and the fall cap. The player feeds it its rich ability
/// clusters; enemies/NPCs feed it [`NormalSpineCtx::bare`] + per-actor tuning, so
/// every actor falls + runs through the SAME core (the non-player-centric seam).
pub use integration::{integrate_normal_spine, NormalSpineCtx};
/// How much of a flyline's lift the winch reels at full rate before easing off.
/// Published so the technique that AUTHORS a lift can solve for the rate that
/// still travels the authored distance — one profile, one place.
pub use integration::{winch_rate_for, WIRE_CRUISE_FRAC};
pub use kernel::{step_motion, MotionStepContext, MotionStepResult, SupportFact};
pub use model::{
    catch_the_wire, cut_the_wire, footstool_victim, knock_off_ledge, switch_motion_model,
    AxisManeuverState, AxisSweptMotion, MotionModel, MotionModelKind, MotionModelSpec,
    PhasedJumpState, SurfaceMomentumMotion, WireState,
};
pub use ops::{ComboMark, MovementOp};
pub use player::{default_player_body_size, DEFAULT_PLAYER_BODY_HEIGHT, DEFAULT_PLAYER_BODY_WIDTH};
pub use tuning::{
    ActiveMovementTuning, AxisHorizontalLaw, AxisJumpLaw, AxisLocomotion, AxisSweptParams,
    FlightTuning, FootstoolTuning, LedgeMomentumTuning, MomentumHorizontalTuning, MovementTuning,
    OutOfShield, ParryTiming, PhasedGravityJumpTuning, ShieldTuning, TraversalAbilityTuning,
    AIR_ACCEL, AIR_DODGE_ENDLAG, AIR_DODGE_SPEED, AIR_DODGE_TIME, AIR_FRICTION, AIR_JUMPS,
    BLINK_COOLDOWN, BLINK_DISTANCE, BLINK_GRACE_TIME, BLINK_HOLD_THRESHOLD,
    BLINK_MAX_DOWNWARD_SPEED, COYOTE_TIME, DASH_BUFFER, DASH_COOLDOWN, DASH_SPEED, DASH_TIME,
    DEFAULT_AXIS_SWEPT_PARAMS, DEFAULT_GRAVITY_DIR, DEFAULT_TUNING, DODGE_ROLL_COOLDOWN,
    DODGE_ROLL_ENDLAG, DODGE_ROLL_SPEED, DODGE_ROLL_TIME, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL,
    FAST_FALL_SPEED, FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED,
    FLIGHT_TERMINAL_SPEED, GLIDE_AIR_ACCEL, GLIDE_FALL_SPEED, GRAVITY, GROUND_FRICTION,
    JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED, MAX_RUN_SPEED, ONE_WAY_DROP_THROUGH_GRACE,
    PARRY_WINDOW_TIME, POGO_SPEED, PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE,
    PRECISION_BLINK_MAX_DOWNWARD_SPEED, RUN_ACCEL, RUN_COMMIT_FRAC, SLASH_RECOIL, SPOT_DODGE_STICK,
    SPOT_DODGE_TIME, WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};

#[cfg(test)]
use collision::body_is_side_contact;

/// Frame-explicit axis-swept control phase — kernel-private. The current
/// acceleration frame is supplied by the environment, never read from or
/// written into model parameters. `state` is the axis policy's model-private
/// maneuver state ([`AxisManeuverState`]), threaded from the active
/// [`AxisSweptMotion`] variant.
pub(crate) fn update_body_control_in_frame(
    world: &World,
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    state: &mut AxisManeuverState,
    input: InputState,
    control_dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    // Somebody else owns this body's pose — see `PoseOwnedExternally`.
    pose_owned_externally: bool,
) -> FrameEvents {
    let mut events = FrameEvents::default();

    // Reset on edge press: the body only FLAGS the request; the body's owner
    // applies its reset policy (respawn for the home body, damage/ignore for
    // an actor).
    if input.reset_pressed && clusters.abilities.abilities.reset {
        events.reset = Some(ResetCause::Requested);
        return events;
    }

    // Resolve tumble/knockdown/tech/getup in the control phase before any
    // maneuver reads input. These states suppress control, not integration.
    let input = knockdown::tick_knockdown(
        clusters.kinematics,
        state,
        clusters.ground,
        clusters.wall,
        clusters.combo_trace,
        input,
        control_dt,
        frame,
        &mut events,
    );

    // Guard + down on a one-way surface is the PLATFORM DROP, so the evade has
    // to stand down and let it through — see `platform_drop_requested`, which
    // the simulation phase asks again to perform the drop itself.
    let platform_drop = integration::platform_drop_requested(
        world,
        clusters.kinematics,
        clusters.ground,
        input,
        frame,
        tuning,
    );

    abilities::apply_intent(
        clusters.kinematics,
        clusters.ground,
        clusters.flight,
        state,
        clusters.abilities,
        input,
        tuning,
        clusters.shield,
        clusters.dodge,
        platform_drop,
    );

    abilities::apply_fly_toggle(
        clusters.flight,
        state,
        clusters.abilities,
        clusters.combo_trace,
        input,
        &mut events,
    );

    // Blink hold / aim / release + melee + pogo dispatch.
    control::handle_blink_clusters(
        world,
        clusters.kinematics,
        clusters.abilities,
        clusters.blink,
        state,
        clusters.combo_trace,
        input,
        control_dt,
        frame,
        tuning,
        &mut events,
    );
    control::handle_attacks_clusters(
        clusters.kinematics,
        clusters.abilities,
        clusters.combo_trace,
        input,
        frame,
        tuning,
        &mut events,
    );

    // ⛔⛔ A CONSTRAINED BODY DOES NOT SPEND A BUFFERED MANEUVER, and the buffer
    // is why clearing this tick's verbs was not enough. `step_body` zeroes the
    // stick and the movement verbs before the kernel sees them — but an evade
    // and a dash are spent out of `AxisManeuverState::buffer_burst`, which is a
    // press made EARLIER that stays spendable, so a press made on the floor a
    // moment before the saddle took the body was still spent inside it.
    // Measured on the pirate's shark: guard and a direction held through the
    // ride, and the air dodge went on MOUNTED TICK 2, airborne, at the exact
    // cost `step_body`'s own comment says this rule exists to prevent — a snap
    // fixes a position, not a spent evade.
    //
    // ⛔ FORBIDDEN, NOT ERASED. The buffer is input memory; dropping it would
    // swallow a press the player made and is entitled to have honoured the tick
    // the constraint lets go. This refuses the SPEND and leaves the window to
    // expire on its own clock, which is what an unspendable press does
    // everywhere else.
    //
    // ⭐ THE GUARD AND THE JUMP RELEASE STILL RUN. Raising a shield spends
    // nothing, and a rider swinging from the saddle is the whole point of the
    // marker not being `out_of_play`.
    if !pose_owned_externally {
        abilities::apply_dodge(
            clusters.kinematics,
            clusters.dodge,
            state,
            clusters.ground,
            clusters.abilities,
            clusters.combo_trace,
            input,
            frame,
            tuning,
            &mut events,
        );

        abilities::apply_dash(
            clusters.kinematics,
            clusters.dash,
            state,
            clusters.abilities,
            clusters.combo_trace,
            input,
            frame,
            tuning,
            &mut events,
        );
    }

    abilities::apply_shield(
        clusters.shield,
        state,
        clusters.ground,
        clusters.abilities,
        clusters.combo_trace,
        input,
        tuning,
        &mut events,
    );

    abilities::apply_jump_release(
        clusters.kinematics,
        state,
        clusters.abilities,
        input,
        frame,
        tuning,
    );

    events
}

/// Frame-explicit axis-swept simulation phase — kernel-private. The same
/// immutable frame reaches every gravity-relative limb.
pub(crate) fn update_body_simulation_in_frame(
    world: &World,
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    state: &mut AxisManeuverState,
    input: InputState,
    raw_dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    contact: body_contact::BodyContactField<'_>,
    // See `MotionStepContext::recovery_commitment_outstanding`.
    recovery_commitment_outstanding: bool,
) -> FrameEvents {
    // §3.1 SweepSample: both endpoints are captured INSIDE the kernel —
    // `prev` at sim-phase entry, `curr` at exit — so any position change
    // outside this window (blink in the control phase, respawn policy after
    // this returns, portal/room/scripted teleports in other systems) is
    // excluded from the motion record BY CONSTRUCTION. Early returns still
    // pass through the write below (a zero-dt tick records a zero-length
    // segment, never a stale one).
    let entry_pos = clusters.kinematics.pos;
    let entry_vel = clusters.kinematics.vel;
    let (mut events, reach) = update_body_simulation_inner(
        world,
        clusters,
        state,
        input,
        raw_dt,
        frame,
        tuning,
        contact,
        recovery_commitment_outstanding,
    );
    if let Some(sweep) = clusters.sweep.as_deref_mut() {
        *sweep = crate::body_clusters::SweepSample {
            prev: entry_pos,
            curr: clusters.kinematics.pos,
            vel: entry_vel,
            half: clusters.kinematics.size * 0.5,
        };
    }

    // Hazard / out-of-bounds gate — body flags the cause; the owner applies its policy.
    //
    // ⛔ AFTER THE SAMPLE WRITE, AND THAT ORDER IS THE CONTRACT. The gate reads
    // the tick's travelled path, so running it inside the inner step — where it
    // used to live — read the PREVIOUS tick's segment, and on the first tick a
    // zero-length default. The other two policy arms already write their sample
    // immediately before calling the gate; this arm captures its endpoints out
    // here, so the gate has to be out here too.
    //
    // ⛔⛔ AND ONLY ON THE PATH THAT USED TO REACH IT. `SimPhaseReach` exists
    // because moving the gate out here would otherwise have ADDED three
    // populations the tail never judged — a zero-dt tick, a drowning, and a frame
    // an active ledge grab consumed. This is an ordering fix, not a widening.
    if reach == SimPhaseReach::Completed {
        kernel::apply_world_hazard_gate(world, clusters, frame, &mut events);
    }

    events
}

#[allow(clippy::too_many_arguments)]
/// A ROLL ENDS BY STOPPING — it takes back its own push and nothing else.
///
/// Jon, 2026-08-25: *"shield rolls have too much motion to them. They send the
/// character flying across the stage... they probably should stop at the end of
/// the roll."* Measured before the fix: a held guard+direction covered 1339px
/// in three seconds on a stage about 480px wide. The roll itself is only 124px
/// of that — the body then COASTED at the full roll speed for the rest of the
/// cooldown, because the roll set a velocity that nothing ever took back.
///
/// ⛔⛔ IT MAY NOT SIMPLY ZERO `vel`. That was tried, and it erased the
/// knockback of a body struck mid-roll. So the shed is bounded twice: never
/// against the body's current direction of travel, and never more than the
/// speed actually present. A body launched out of its own roll keeps every bit
/// of the launch; a body that merely finished one is left standing.
fn shed_dodge_roll_push(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    state: &mut AxisManeuverState,
    frame: MotionFrame,
) {
    let push = std::mem::replace(&mut state.dodge_roll_push, 0.0);
    if push == 0.0 {
        return;
    }
    let side = frame.side();
    let along = kinematics.vel.dot(side);
    // ⛔⛤ THE TEST IS EQUALITY, NOT A BOUND, and the difference is a real bug
    // that a bound version shipped: a body rolling right and then launched
    // right at LESS than roll speed passes "same sign, no faster", and shedding
    // would delete that launch. The roll's own push does not decay while the
    // roll runs — gravity acts along `down`, not `side`, and nothing applies
    // friction to it — so if the side speed is still exactly what the roll set,
    // nothing else has touched it. Any other value means something did.
    //
    // ⭐ THE ASYMMETRY IS DELIBERATE. Failing to shed leaves a body coasting for
    // a few frames; shedding wrongly deletes someone's knockback and reads as a
    // combo that does nothing. So every doubtful case does nothing.
    if (along - push).abs() > 1.0 {
        return;
    }
    kinematics.vel -= side * along;
}

/// The grounded per-tick refresh's answer to
/// [`crate::body_clusters::RecoveryRefresh`]: a body standing on the floor is
/// re-seated UNLESS it is in the middle of a recovery it has already paid for.
pub(super) fn recovery_refresh(
    commitment_outstanding: bool,
) -> crate::body_clusters::RecoveryRefresh {
    if commitment_outstanding {
        crate::body_clusters::RecoveryRefresh::Withheld
    } else {
        crate::body_clusters::RecoveryRefresh::Answered
    }
}

/// Did the simulation phase run to its END this tick, or short-circuit?
///
/// ⛔ THE HAZARD/OOB GATE RUNS ONLY ON `Completed`, AND THAT IS A POPULATION
/// DECISION, not a detail. `update_body_simulation_inner` has three early
/// returns — a `raw_dt <= 0.0` tick, a drowning, and a frame an active ledge
/// grab consumed — and none of them ever reached the gate while it sat in that
/// function's tail. Moving the gate to the caller without this flag silently
/// added all three: a body hanging on a ledge whose box overlaps a hazard
/// (spikes under a lip is an authored shape) would start dying, and a frozen
/// frame would judge a body nothing had stepped.
///
/// ⚠ The SAMPLE WRITE is deliberately NOT gated on this. It runs on every path,
/// because a zero-dt tick must record a zero-length segment rather than keep a
/// stale one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SimPhaseReach {
    /// The phase ran to its tail: the body was stepped and may be judged.
    Completed,
    /// The phase returned early; this tick did not step the body.
    ShortCircuited,
}

fn update_body_simulation_inner(
    world: &World,
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    state: &mut AxisManeuverState,
    input: InputState,
    raw_dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    contact: body_contact::BodyContactField<'_>,
    // See `MotionStepContext::recovery_commitment_outstanding`.
    recovery_commitment_outstanding: bool,
) -> (FrameEvents, SimPhaseReach) {
    let mut events = FrameEvents::default();
    if raw_dt <= 0.0 {
        return (events, SimPhaseReach::ShortCircuited);
    }
    let dt = raw_dt.min(1.0 / 30.0);

    // Ledge carry on moving geometry, BEFORE anything reads the body's pose.
    //
    // the same rule as the grounded ride, one step earlier in the frame. A body resting on
    // a moving solid is carried by `Block::velocity` down in `integrate_velocity_clusters`; a
    // body HANGING on one cannot be, because the active-hang tick below short-circuits the
    // whole simulation phase and `integrate_velocity_clusters` never runs.
    //
    // Nothing about the rule was player-specific; only its ingredients were, and reading the
    // solid's own velocity off the collision world removes them.
    if let Some(grab) = state.ledge_grab {
        match crate::ledge_grab::ledge_carry_for_frame(
            world,
            grab.contact,
            clusters.kinematics.aabb(),
            clusters.kinematics.size,
            frame.down(),
        ) {
            crate::ledge_grab::LedgeCarry::Stay => {}
            crate::ledge_grab::LedgeCarry::KnockOff => {
                state.ledge_grab = None;
                clusters.ledge.release_cooldown = clusters
                    .ledge
                    .release_cooldown
                    .max(crate::body_clusters::LEDGE_KNOCK_OFF_COOLDOWN);
            }
            crate::ledge_grab::LedgeCarry::Carry(delta) => {
                // Parent-frame carry (ADR 0024 external-constraint authority):
                // the solid moves the grabbed body, and the stored contact moves
                // with it or the next tick re-pins the body to where the ledge
                // used to be.
                authority::carry_body(clusters.kinematics, delta);
                if let Some(grab) = state.ledge_grab.as_mut() {
                    grab.contact.anchor += delta;
                    grab.contact.climb_target += delta;
                }
            }
        }
    }

    // Cache water + climbable contact once per tick so movement,
    // jump-buffer, and integration all see the same answer. Also
    // clear a stale ledge grab if the ability is no longer enabled.
    clusters.env_contact.water = world.water_at(clusters.kinematics.aabb());
    clusters.env_contact.climbable = world.climbable_at(clusters.kinematics.aabb());
    if !clusters.abilities.abilities.ledge_grab {
        state.ledge_grab = None;
    }

    // Drowning gate — body flags the cause; the owner applies its policy.
    if clusters.env_contact.water.is_some() && !clusters.abilities.abilities.swim {
        events.reset = Some(ResetCause::Drowned);
        return (events, SimPhaseReach::ShortCircuited);
    }

    // Age lifetime + timers + combo trace — cluster + maneuver-state inline.
    {
        clusters.lifetime.time_alive += dt;
        let speed = clusters.kinematics.vel.length();
        clusters.lifetime.max_speed = clusters.lifetime.max_speed.max(speed);
        for mark in clusters.combo_trace.combo.iter_mut() {
            mark.age += dt;
        }
        clusters
            .combo_trace
            .combo
            .retain(|m| m.age < 4.0 || m.op == ops::MovementOp::Reset);

        let dec = |v: f32| (v - dt).max(0.0);
        state.buffer_jump = dec(state.buffer_jump);
        state.buffer_burst = dec(state.buffer_burst);
        state.coyote_timer = dec(state.coyote_timer);
        state.drop_through_timer = dec(state.drop_through_timer);
        clusters.jump.ladder_jump_boost = dec(clusters.jump.ladder_jump_boost);
        clusters.jump.ladder_drop_through_timer = dec(clusters.jump.ladder_drop_through_timer);
        clusters.dash.cooldown = dec(clusters.dash.cooldown);
        clusters.blink.cooldown = dec(clusters.blink.cooldown);
        state.blink_grace_timer = dec(state.blink_grace_timer);
        state.rebound_cooldown = dec(state.rebound_cooldown);
        // ⭐⭐ THE ROLL HANDS OFF TO ITS OWN ENDLAG, exactly as the air dodge
        // below does: "invulnerable" and "committed" become separable states
        // rather than one fused timer, and the gap between them is what a
        // defender reads. Jon, 2026-08-24, asking for a roll that leaves the
        // character *"punishable for a frame or two."*
        //
        // ⛔⛔ AND IT DOES NOT CANCEL THE BODY'S VELOCITY, which this did for
        // about an hour. Two measurements killed that:
        //
        //   1. THE ROLL ALREADY STOPS. Ground friction (7600 px/s²) takes the
        //      530 px/s roll to zero in about four frames — a fifth of the
        //      0.22s window — so the roll travels ~14px and is motionless long
        //      before it ends. There was no persisting velocity to cancel, and
        //      the test written to prove otherwise stayed green with the cancel
        //      removed entirely.
        //   2. IT ATE LAUNCHES. `vel` here is the body's WHOLE velocity, not the
        //      roll's contribution to it, so a fighter struck during a roll had
        //      its knockback erased on the tick the window closed:
        //      `an_up_tilt_launches_much_further_at_a_high_percent` measured a
        //      victim rising 4.5px at 0% and 0.0px at 1427%.
        //
        // ⇒ a maneuver that ends must not reach into a shared velocity it does
        // not own. If a roll ever DOES need to shed its own push, the push has
        // to be tracked separately from whatever else moved the body.
        if state.dodge_roll_timer > 0.0 {
            state.dodge_roll_timer = dec(state.dodge_roll_timer);
            if state.dodge_roll_timer <= 0.0 && !state.spot_dodging {
                // ⛔ NOT AFTER A SPOT DODGE. The spot dodge runs on this SAME
                // timer — `spot_dodging` is a refinement of `dodge_rolling`, not
                // a sibling — so an endlag hung off the expiry alone is charged
                // to a maneuver that covers no distance and never asked for one.
                state.dodge_roll_endlag_timer = tuning.abilities.dodge_roll_endlag;
                shed_dodge_roll_push(clusters.kinematics, state, frame);
            }
        } else {
            state.dodge_roll_endlag_timer = dec(state.dodge_roll_endlag_timer);
        }
        // ⭐ THE SAFE HALF OF AN EVADE RUNS DOWN ON ITS OWN. It is armed shorter
        // than the maneuver for a body that has been spamming evades, so a stale
        // roll finishes its travel with the last of it already hittable — which
        // is the entire point of staling.
        state.evade_invuln_timer = dec(state.evade_invuln_timer);
        // THE TURNAROUND runs down here with every other maneuver clock. When
        // it reaches zero the next `apply_intent` flips the facing, because the
        // stick is still asking for it.
        state.turnaround_timer = dec(state.turnaround_timer);
        // ⛔ THE LEDGE'S EXPOSURE RUNS FIRST AND THE INVULN WAITS FOR IT. Ticking
        // both would quietly shorten every earned window by the vulnerability,
        // so a fighter who bought its edge with a long recovery would silently
        // get less than it earned.
        state.ledge_vulnerable_timer = dec(state.ledge_vulnerable_timer);
        if state.ledge_vulnerable_timer <= 0.0 {
            state.ledge_invuln_timer = dec(state.ledge_invuln_timer);
        }
        // The air dodge hands off to its own endlag the tick its window closes,
        // so "invulnerable" and "committed" are separable states rather than one
        // fused timer — the punish window is the half a defender reads.
        if state.air_dodge_timer > 0.0 {
            state.air_dodge_timer = dec(state.air_dodge_timer);
            if state.air_dodge_timer <= 0.0 {
                state.air_dodge_endlag_timer = tuning.abilities.air_dodge_endlag;
            }
        } else {
            state.air_dodge_endlag_timer = dec(state.air_dodge_endlag_timer);
        }
        clusters.dodge.cooldown = dec(clusters.dodge.cooldown);
        // ⭐ DODGE STALING BLEEDS OFF, one evade at a time. A fighter who stops
        // rolling gets the option back; one who keeps rolling never reaches the
        // delay's end, which is the whole mechanic.
        //
        // ⛔ ONE AT A TIME, not "forgive everything after a pause": a window
        // that cleared the whole count would let a fighter roll four times, wait
        // once, and be fully fresh — so recovering would cost the same whether
        // the option had been abused once or ten times.
        // ⛔⛔ AND IT DOES NOT BLEED WHILE THE EVADE IS STILL HAPPENING. The
        // contract in `spend_evade` says the count "only starts coming down once
        // the body actually stops", and this ticked every frame from the moment
        // the evade was ACCEPTED — so a 0.22s roll spent about 18% of Smash's
        // 1.2s forgiveness performing the very maneuver the delay exists to
        // charge for. The existing decay test seeds stale state on an IDLE body,
        // so it never ran an accepted evade through its own maneuver.
        //
        // ⭐ THE MANEUVER CLOCKS, which are the AUTHORED durations since the
        // staling split — not the i-frame clock, which is exactly what a spammed
        // evade shortens. Forgiveness must not speed up because the fighter has
        // been abusing the option.
        //
        // ⛔ ENDLAG IS NOT COUNTED: recovery is what the roll OWES, not part of
        // the evade, and a body in endlag has stopped evading.
        let still_evading = state.dodge_roll_timer > 0.0 || state.air_dodge_timer > 0.0;
        if !still_evading
            && clusters.dodge.evades_recent > 0
            && tuning.abilities.dodge_stale_recovery > 0.0
        {
            clusters.dodge.stale_decay = dec(clusters.dodge.stale_decay);
            if clusters.dodge.stale_decay <= 0.0 {
                clusters.dodge.evades_recent -= 1;
                clusters.dodge.stale_decay = if clusters.dodge.evades_recent > 0 {
                    tuning.abilities.dodge_stale_recovery
                } else {
                    0.0
                };
            }
        }
        clusters.shield.parry_window_timer = dec(clusters.shield.parry_window_timer);
        clusters.shield.absorb_window_timer = dec(clusters.shield.absorb_window_timer);
        clusters.shield.parry_caught_timer = dec(clusters.shield.parry_caught_timer);
        clusters.shield.drop_lag_timer = dec(clusters.shield.drop_lag_timer);
        if crate::body_clusters::tick_shield_resource(clusters.shield, tuning.abilities.shield, dt)
        {
            events.op_clusters(clusters.combo_trace, ops::MovementOp::ShieldBreak);
        }
        clusters.ledge.release_cooldown = dec(clusters.ledge.release_cooldown);
        if state.wall_clinging || clusters.ground.on_ground {
            state.pre_wall_vel_age += dt;
        }
        // Time NOT spent hanging, which is what the next grab's intangibility is bought
        // with.
        if state.ledge_grab.is_none() {
            state.time_off_ledge =
                (state.time_off_ledge + dt).min(crate::ledge_grab::LEDGE_INVULN_FULL_AIRTIME);
        }
        if clusters.ground.on_ground {
            state.coyote_timer = tuning.locomotion.coyote_time;
            // Landing ends an air dodge outright — window, endlag and budget.
            state.air_dodge_timer = 0.0;
            state.air_dodge_endlag_timer = 0.0;
            crate::body_clusters::refresh_movement_resources_clusters(
                clusters.abilities,
                clusters.dash,
                clusters.jump,
                clusters.dodge,
                tuning.locomotion.air_jumps,
                recovery_refresh(recovery_commitment_outstanding),
            );
        }
    }

    // The floor game already ran in the control phase; the simulation half only
    // has to agree about who is holding the controller.
    let input = if knockdown::owns_control(state) {
        InputState::default()
    } else {
        input
    };

    // Active ledge-grab tick. Returns true if it consumed the frame
    // (the rest of the simulation phase short-circuits).
    if crate::ledge_grab::tick_active_ledge_grab_clusters_in_frame(
        clusters,
        state,
        input,
        dt,
        frame,
        tuning,
        &mut events,
    ) {
        return (events, SimPhaseReach::ShortCircuited);
    }

    // Consume the buffered jump (or convert to swim stroke /
    // drop-through / wall-jump / double-jump).
    simulation::handle_jump_buffer_clusters(
        world,
        integration::platform_drop_requested(
            world,
            clusters.kinematics,
            clusters.ground,
            input,
            frame,
            tuning,
        ),
        state,
        clusters.env_contact,
        clusters.abilities,
        clusters.body_mode.body_mode,
        clusters.flight.fly_enabled,
        clusters.kinematics,
        clusters.ground,
        clusters.wall,
        clusters.jump,
        clusters.combo_trace,
        input,
        dt,
        frame,
        tuning,
        &mut events,
    );

    integration::integrate_velocity_clusters(
        world,
        clusters,
        state,
        input,
        dt,
        frame,
        tuning,
        contact,
        recovery_commitment_outstanding,
        &mut events,
    );

    // Probe for a fresh ledge grab now that the integration step
    // settled the new position. Required for the auto-snap-on-fall
    // recovery path (slow drifts ignore this; fast falls latch).
    crate::ledge_grab::try_start_ledge_grab_clusters_in_frame(
        world,
        clusters,
        state,
        input,
        frame,
        &tuning,
        &mut events,
    );

    (events, SimPhaseReach::Completed)
}

fn dec(value: f32, dt: f32) -> f32 {
    (value - dt).max(0.0)
}

/// Axis-swept implementation arm behind [`step_motion`] — kernel-private. All
/// frame-sensitive control and integration receives the exact same per-tick
/// frame value, and both phases share the SAME model-private maneuver state
/// borrowed from the active [`AxisSweptMotion`] variant.
/// `InputState::control_dt` overrides `raw_dt` for the control phase when
/// positive (so bullet-time slowing gravity does not slow input).
pub(crate) fn update_body_with_frame_clusters(
    world: &World,
    axis: &mut AxisSweptMotion,
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    input: InputState,
    frame: MotionFrame,
    raw_dt: f32,
    contact: body_contact::BodyContactField<'_>,
    // See `MotionStepContext::recovery_commitment_outstanding`: the grounded
    // refresh below may not answer for a recovery whose move is still running.
    recovery_commitment_outstanding: bool,
    // See `PoseOwnedExternally`: a constrained body spends no buffered maneuver.
    pose_owned_externally: bool,
) -> FrameEvents {
    let tuning = axis.params;
    let state = &mut axis.state;
    // A body may be freshly constructed, or the control phase may perform a discrete transit
    // (blink) that invalidates the departure contact.
    let entry_baseline = kernel::establish_axis_ground_contact_baseline(world, clusters, frame);
    let control_dt = if input.control_dt > 0.0 {
        input.control_dt
    } else {
        raw_dt
    };
    let mut events = update_body_control_in_frame(
        world,
        clusters,
        state,
        input,
        control_dt,
        frame,
        tuning,
        pose_owned_externally,
    );
    let baseline = if clusters.ground.contact_initialized {
        entry_baseline
    } else {
        kernel::establish_axis_ground_contact_baseline(world, clusters, frame)
    }
    .with_impact_velocity(clusters.kinematics.vel, frame)
    // BEFORE the simulation half, which is where `tick_knockdown` clears the
    // tumble it resolves. Read after, this is always false.
    .falling_out_of_a_launch(state.tumble_until_landing);
    // ONE READ, TWO SURFACES. The same fact the ground baseline just took is
    // what every contact this step produces is stamped with — a body thrown
    // into a wall and one that dashed into it are otherwise identical at the
    // contact, and only this separates a crash from a commute.
    //
    // ⛔ read HERE and not after the step: the floor game clears the tumble on
    // the touchdown it resolves, so a value taken afterwards is false exactly
    // when it matters.
    let arriving_out_of_a_launch = state.tumble_until_landing;
    let mut sim_events = update_body_simulation_in_frame(
        world,
        clusters,
        state,
        input,
        raw_dt,
        frame,
        tuning,
        contact,
        recovery_commitment_outstanding,
    );
    if arriving_out_of_a_launch {
        for contact in &mut sim_events.contacts {
            contact.involuntary = true;
        }
    }
    sim_events.ground_contact = baseline.transition_to(clusters.ground.on_ground);
    events.extend(sim_events);
    events
}

#[cfg(test)]
mod tests;
