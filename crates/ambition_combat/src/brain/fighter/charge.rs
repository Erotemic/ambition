//! How long a fighter leans on a smash before letting go.
//!
//! `smash_charge_mult` is authored per move and the runtime freezes the timeline
//! while Attack is held, so the multiplier a smash actually pays is decided by
//! whoever is holding the button. No brain held it, which made every CPU smash a
//! tap and quietly re-tuned every fighter's strongest option down to its floor.
//!
//! The decision is a READ, not a timer: charging is committing to a swing that
//! is not out yet, and what it costs is the window the opponent has to answer.
//! So the length of the hold is the length of the opening.

use ambition_characters::brain::fighter::situation::Situation;

/// Ticks of hold for an opening wide enough to finish the charge.
///
/// The move releases itself at its authored maximum, so "long" only has to
/// outlast the longest authored `max_hold_s`; it does not have to equal it, and
/// this brain deliberately does not read that number. A person charging a smash
/// at a body they just launched is not counting frames either.
const FULL_HOLD_TICKS: u32 = 90;

/// Ticks of hold for a swing thrown into a live opponent: enough to pay for
/// itself, short enough to still come out before the answer does.
const PARTIAL_HOLD_TICKS: u32 = 12;

/// How long to hold Attack for a smash that freezes at `hold_at_s` on its own
/// timeline.
///
/// THE HOLD MUST REACH THE HOLD POINT, and this is the whole reason the function
/// takes frame data at all. Charge does not begin at the press — it begins when
/// the move's clock arrives at the freeze. Measured in a real match: a brain
/// that held for the charge alone let go before the move ever got there, the
/// freeze released on arrival at zero, and 223 armed charges produced not one
/// held frame. A hand does not have this bug, because a hand holds from the
/// press.
///
/// ⛔⛔ IT IS THE HOLD POINT, NOT `startup_s`, AND THOSE ARE DIFFERENT NUMBERS.
/// This read the move's whole leading startup, which is only the same thing
/// while the freeze sits at the instant the strike comes out. The genre holds a
/// smash in its WINDUP pose — earlier — and the day the hold point moves, a
/// brain reading startup over-holds by the difference, keeping the button down
/// through a swing it already released.
pub fn hold_ticks(situation: Situation, hold_at_s: f32, tick_hz: f32) -> u32 {
    let charge = charge_ticks_for(situation);
    if charge == 0 {
        return 0;
    }
    let reach_ticks = (hold_at_s.max(0.0) * tick_hz.max(1.0)).ceil() as u32;
    reach_ticks.saturating_add(charge)
}

/// How long to keep holding once the charge has actually latched.
///
/// `0` is a tap, and a tap is the right answer often enough that it is the
/// default rather than a failure mode.
pub fn charge_ticks_for(situation: Situation) -> u32 {
    match situation {
        // The opponent is committed to something — hitstun, a whiffed swing, a
        // landing. This is the window the charge exists for.
        Situation::Advantage => FULL_HOLD_TICKS,
        // They are offstage and have to come back through this. Same window,
        // and the charge is what makes coming back expensive.
        Situation::EdgeGuard => FULL_HOLD_TICKS,
        // Neutral is a guess. Pay for a little of it and keep the option to be
        // wrong cheaply.
        Situation::Neutral => PARTIAL_HOLD_TICKS,
        // Being hit or cornered is not the time to stand still, and being
        // offstage is not the time to attack at all.
        Situation::Disadvantage | Situation::Recovery => 0,
    }
}

/// Ticks of hold for walking a STRING — jab into jab2 into jab3 — as opposed to
/// leaning on a single swing until it is fat.
///
/// ⭐ THE SAME BUTTON, A DIFFERENT GESTURE, AND THE ENGINE ALREADY DISTINGUISHES
/// THEM: `MovePlayback` treats a held Attack as a charge when the intent is
/// `Smash` and as a string continuation when it is not, and a continuation can
/// reach nothing but a successor the playing window already NAMES. So a hold on
/// a move that authors no chain does nothing at all, which is what makes it safe
/// to hold every neutral basic rather than asking the brain to know which moves
/// have chains.
///
/// ⛔ IT IS A DURATION OF INTENT, NOT A COUNT OF RUNGS. This brain deliberately
/// does not read how many links a string has or how long each one lasts — the
/// same reason [`hold_ticks`] does not read `max_hold_s`. A person leaning on
/// jab is not counting frames; they are holding for as long as the opponent
/// stays in front of them, and the string walks as far as it walks.
///
/// ⛔⛔ AND THE STARTUP TERM IS LOAD-BEARING FOR THE SAME REASON IT IS IN
/// [`hold_ticks`]: the first cancel window does not open at the press, it opens
/// once the timeline has played the leading move. A hold that expires before the
/// window arrives produces a jab and nothing else — which is precisely the state
/// this fixes, where a census over ninety seconds counted `jab` and never once
/// counted `jab2`.
pub fn string_hold_ticks(situation: Situation, startup_s: f32, tick_hz: f32) -> u32 {
    let carry = match situation {
        // ⛔⛔ THE SAME LENGTH IN EVERY ATTACKING SITUATION, AND THE FIRST
        // VERSION OF THIS FUNCTION WAS WRONG ABOUT THAT. It read like a charge —
        // a committed opponent buys a LONG hold, a guess buys a short one — and
        // that reasoning does not transfer, because the two gestures spend
        // different currency. A charge spends time the opponent cannot use. A
        // string spends the body's OWN next decision: every tick the button
        // stays down is a tick this fighter is walking a low-damage chain
        // instead of ending it and choosing a punish.
        //
        // MEASURED, on the shipped duel: a 60-tick hold in Advantage took seat
        // 0's damage over a full match from a passing 169% to 33%, and the
        // fighters stopped being knocked off the stage at all — a guard's
        // PREMISE went false, which is how it surfaced. An opening is exactly
        // when a jab string is the wrong thing to be doing.
        //
        // So the hold buys the FOLLOW-UP and never the flurry. A string that
        // wants to be walked to its end wants a brain that decided to walk it,
        // and this one decided to throw a jab.
        Situation::Advantage | Situation::EdgeGuard | Situation::Neutral => STRING_CARRY_TICKS,
        // Being hit is not the time to keep the button down, and being offstage
        // is not the time to be attacking.
        Situation::Disadvantage | Situation::Recovery => return 0,
    };
    let startup_ticks = (startup_s.max(0.0) * tick_hz.max(1.0)).ceil() as u32;
    startup_ticks.saturating_add(carry)
}

/// Long enough to reach the follow-up the window names, short enough that the
/// body is choosing again before the chain becomes a commitment.
const STRING_CARRY_TICKS: u32 = 20;

#[cfg(test)]
mod tests;
