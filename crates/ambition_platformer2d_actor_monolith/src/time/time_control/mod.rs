//! Ambition-specific clock requests and world-log reporting.
//!
//! Clock arbitration lives in `ambition_time::time_control`; this module emits
//! game-policy requests such as hitstop, bullet-time, and developer slow-motion.

use bevy::prelude::*;

use ambition_characters::actor::BodyCombat;
use ambition_combat::feel::Platformer2dFeelTuningMonolith;
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;
use ambition_time::time_control::{
    ClockRequester, ClockResetRequest, ClockScaleRequest, RequestedClockScale,
};
use ambition_time::{ClockDomain, ClockObserver, ClockState};

/// Read the primary player's state and the dev tools, decide what
/// SimClock scale should be in effect this frame, and fire one
/// [`ClockScaleRequest`].
///
/// Priority matches the historic [`crate::update_time_scale`]
/// ladder so behavior is preserved:
///
/// 1. hitstop active → 0.0   (Engine requester — the engine took
///    authority on the player's behalf)
/// 2. blink aiming → bullet_time_scale  (Player requester —
///    the "bullet_blink" verb)
/// 3. blink hold active → blink_hold_slow_scale  (Player —
///    "blink_hold_slow")
/// 4. otherwise → 1.0  (Engine — restoring real-time pace)
///
/// ⛔ THE DEVELOPER RUNG LEFT, 2026-08-31. It sat between 3 and 5 and read
/// `ambition_dev_tools::DeveloperRuntimeState` — a simulation package looking at
/// developer state, the last such read in this kernel. Slow-motion publishes its
/// own `ClockScaleRequest` now
/// (`ambition_dev_tools::request_developer_slow_motion`), and
/// `apply_clock_scale_requests` reduces every granted request by `min`.
///
/// ⚠ SO THIS IS NO LONGER A TOTAL ORDER over slow-downs, and one pair changed:
/// bullet-time (rung 2) used to outrank slow-motion, and now the stronger
/// slowdown wins. Inside this function the ladder is still a ladder.
///
/// ADR 0011 §"Two time-control operations" note: in SP, the
/// "slow sim" (Operation 1) and "boost player proper time"
/// (Operation 2) are observationally equivalent for one observer.
/// We implement Operation 1 here because it's the simpler write.
/// Step 3's per-entity `ProperTimeScale` component + `entity_dt`
/// accessor are the seam where future MP / RL regimes diverge.
pub fn emit_player_time_intent_system(
    // SLOT-0 BY DESIGN: bullet-time is a per-PLAYER feel-clock affordance (ADR
    // 0010/0011). Slot 0's blink slows slot 0's world; a second player would emit
    // its own intent against its own clock, not fight over this one.
    primary: Query<
        (&ambition_platformer2d_core::BodyMotionFacts, &BodyCombat),
        With<PrimaryPlayer>,
    >,
    feel: Res<Platformer2dFeelTuningMonolith>,
    // The MATCH's freeze, which belongs to nobody's seat. See rung zero below.
    impact: Option<Res<ambition_combat::impact_hitstop::ImpactHitstop>>,
    tick: Option<Res<ambition_time::SimTick>>,
    mut writer: MessageWriter<ClockScaleRequest>,
) {
    // ⭐⭐ RUNG ZERO: THE MATCH'S OWN IMPACT FREEZE, ABOVE EVERY SEAT.
    //
    // Every rung below reads slot zero, so a CPU-versus-CPU match produced local
    // hitlag on both bodies and NO screen freeze — the beat that sells a connect
    // was a player-shaped affordance in a game whose fights are frequently
    // between two CPUs. ⛔ and the fix is not a fake primary player: a match's
    // freeze is a fact about the MATCH.
    //
    // ⛔⛔ IT IS AN ABSOLUTE EXPIRY AGAINST `SimTick`, which is what makes this
    // safe to put above the ladder. The obvious version — freeze while any
    // body's `hitstop_timer` is live — deadlocks, because an actor's timer
    // decays on the very sim clock this stops. `SimTick` advances while
    // `sim_dt == 0`, so the hold cannot freeze its own expiry. See
    // `ambition_combat::impact_hitstop`.
    //
    // ⭐ AND THERE IS NO "REMEMBER TO SET IT BACK": this writes a request every
    // frame like every other rung, so an expired hold falls through to whatever
    // is true next — ultimately the `1.0` at the bottom.
    if let (Some(impact), Some(tick)) = (impact, tick) {
        if impact.is_freezing(*tick) {
            writer.write(ClockScaleRequest {
                domain: ClockDomain::SimClock,
                scale: 0.0,
                requester: ClockRequester::Engine,
                reason: "impact_hitstop",
            });
            return;
        }
    }
    // NO LOCAL BODY IS NOT "NOTHING TO SAY".
    //
    // There is no `PrimaryPlayer` in it, so nothing ever asked for the neutral pace back, and the
    // world ran at scale 0.0 forever. Every fighter built, seated, armed, framed, brains ticking,
    // `SimTick` counting up — and zero sim seconds per tick, so not one body moved a pixel. *"the
    // characters are just stuck in air"*, with a menu that still worked, because menus do not run
    // on sim time.
    //
    //  the ladder below is slot zero's; the LAST RUNG is the world's.
    // Hitstop and bullet-time are per-player feel affordances and are correctly
    // absent without a player. "Otherwise, run at normal speed" is not an
    // affordance — it is what a world does when nobody is bending time, and a
    // world with no bullet-time claimant is the clearest possible case of that.
    // Reading the two off one early return made the second one unreachable
    // exactly where it was the only thing left to say.
    let Ok((facts, combat)) = primary.single() else {
        //  the developer rung USED to survive here, because the inspector's
        // slow-motion is not slot zero's either — it belongs to whoever is
        // looking at the world. It still does not belong to slot zero, and that
        // is now expressed by the dev crate publishing its own request rather
        // than by this ladder carrying a copy of the rule:
        // `ambition_dev_tools::request_developer_slow_motion`. A world with no
        // primary player still slows for a developer, because nothing about that
        // ever went through here.
        writer.write(ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 1.0,
            requester: ClockRequester::Engine,
            reason: "default",
        });
        return;
    };
    let (scale, requester, reason) = if combat.hitstop_timer > 0.0 {
        (0.0, ClockRequester::Engine, "hitstop")
    } else if facts.blink_aiming {
        (
            feel.bullet_time_scale,
            ClockRequester::Player(ClockObserver::PRIMARY),
            "bullet_blink",
        )
    } else if facts.blink_telegraph {
        // The aiming arm above claimed precision aim, so a telegraph here is
        // the charge HOLD specifically.
        (
            feel.blink_hold_slow_scale,
            ClockRequester::Player(ClockObserver::PRIMARY),
            "blink_hold_slow",
        )
    } else {
        (1.0, ClockRequester::Engine, "default")
    };
    writer.write(ClockScaleRequest {
        domain: ClockDomain::SimClock,
        scale,
        requester,
        reason,
    });
}

/// Smooth [`ClockState::time_scale`] toward
/// [`RequestedClockScale::sim_clock`] at feel-tuned rates.
///
/// Replaces the imperative `crate::update_time_scale` helper. The
/// asymmetric ramp (`time_ramp_down_rate` when decelerating,
/// `time_ramp_up_rate` when accelerating) preserves the "snap into
/// bullet-time, breathe back to normal" feel the imperative version
/// shipped.
pub fn smooth_sim_clock_toward_target_system(
    target: Res<RequestedClockScale>,
    feel: Res<Platformer2dFeelTuningMonolith>,
    time: Res<Time>,
    mut clock: ResMut<ClockState>,
) {
    let frame_dt = time.delta_secs();
    let rate = if target.sim_clock < clock.time_scale {
        feel.time_ramp_down_rate
    } else {
        feel.time_ramp_up_rate
    };
    clock.time_scale = crate::move_toward(clock.time_scale, target.sim_clock, rate * frame_dt);
}

/// Report sim-clock target changes and frozen/running transitions.
///
/// Reports are edge-triggered rather than per-frame. Request metadata identifies
/// the requester and reason; changes without a request come from suspended-gameplay
/// clock control. Frozen detection uses an epsilon because the live scale is smoothed.
const SIM_CLOCK_FROZEN_EPS: f32 = 1e-4;

pub fn report_sim_clock_changes(
    clock: Res<ClockState>,
    target: Res<RequestedClockScale>,
    mut scale_requests: MessageReader<ClockScaleRequest>,
    mut reset_requests: MessageReader<ClockResetRequest>,
    mut last: Local<Option<(f32, f32)>>,
) {
    let live = clock.time_scale;
    let want = target.sim_clock;
    let previous = last.replace((live, want));

    let report = match previous {
        None => true,
        Some((previous_live, previous_want)) => {
            frozen_label(previous_live) != frozen_label(live)
                || (want - previous_want).abs() > SIM_CLOCK_FROZEN_EPS
        }
    };
    if !report {
        // The cursors still advance every frame. `emit_player_time_intent_system`
        // writes a request on EVERY playing frame (the ladder always resolves,
        // even to "default"), so a cursor left to lag would eventually staple a
        // stale reason onto an unrelated change.
        scale_requests.clear();
        reset_requests.clear();
        return;
    }

    let mut causes = String::new();
    for request in scale_requests.read() {
        if !causes.is_empty() {
            causes.push_str(", ");
        }
        causes.push_str(&format!(
            "{}@{:.3} by {:?}",
            request.reason, request.scale, request.requester
        ));
    }
    for request in reset_requests.read() {
        if !causes.is_empty() {
            causes.push_str(", ");
        }
        causes.push_str(&format!(
            "{} reset by {:?}",
            request.reason, request.requester
        ));
    }
    if causes.is_empty() {
        causes.push_str("no clock request this frame (see [game-mode])");
    }

    let Some((previous_live, previous_want)) = previous else {
        ambition_platformer2d_shared_tangle::world_log::sim_clock(format_args!(
            "initial {:<7} live={live:.3} target={want:.3}",
            frozen_label(live)
        ));
        return;
    };
    ambition_platformer2d_shared_tangle::world_log::sim_clock(format_args!(
        "{:<7} live {previous_live:.3} -> {live:.3}  target {previous_want:.3} -> {want:.3}  \
         cause: {causes}",
        frozen_label(live)
    ));
}

fn frozen_label(scale: f32) -> &'static str {
    if scale.abs() <= SIM_CLOCK_FROZEN_EPS {
        "FROZEN"
    } else {
        "running"
    }
}

#[cfg(test)]
mod tests;
