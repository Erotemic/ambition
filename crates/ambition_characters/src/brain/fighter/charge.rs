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

use super::situation::Situation;

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

/// How long to hold Attack for a smash whose leading startup is `startup_s`.
///
/// THE HOLD MUST OUTLAST THE STARTUP, and this is the whole reason the function
/// takes frame data at all. Charge does not begin at the press — it begins when
/// the timeline reaches the move's authored hold point at the end of its leading
/// Startup window. Measured in a real match: a brain that held for the charge
/// alone let go before the move ever got there, the freeze released on arrival
/// at zero, and 223 armed charges produced not one held frame. A hand does not
/// have this bug, because a hand holds from the press.
pub fn hold_ticks(situation: Situation, startup_s: f32, tick_hz: f32) -> u32 {
    let charge = charge_ticks_for(situation);
    if charge == 0 {
        return 0;
    }
    let startup_ticks = (startup_s.max(0.0) * tick_hz.max(1.0)).ceil() as u32;
    startup_ticks.saturating_add(charge)
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

#[cfg(test)]
mod tests;
