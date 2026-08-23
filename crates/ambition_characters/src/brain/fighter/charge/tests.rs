use super::*;

/// The whole mechanic in one assertion: an opening is worth more charge than a
/// guess is.
#[test]
fn an_opening_buys_more_charge_than_neutral_does() {
    assert!(charge_ticks_for(Situation::Advantage) > charge_ticks_for(Situation::Neutral));
    assert!(charge_ticks_for(Situation::EdgeGuard) > charge_ticks_for(Situation::Neutral));
}

/// A charge is a commitment, and a body that is already losing the exchange has
/// nothing to commit. Holding here would be standing still while being hit.
#[test]
fn a_losing_body_does_not_charge() {
    assert_eq!(charge_ticks_for(Situation::Disadvantage), 0);
    assert_eq!(charge_ticks_for(Situation::Recovery), 0);
}

/// Neutral still pays for some of it — a brain that only ever tapped in neutral
/// would be the behaviour this module exists to remove.
#[test]
fn neutral_still_charges_a_little() {
    assert!(charge_ticks_for(Situation::Neutral) > 0);
}

/// The hold has to survive the move's own startup, because the charge does not
/// begin until the timeline reaches the authored hold point. This is the
/// measurement that made the whole slice inert: a hold shorter than the startup
/// releases the freeze on arrival, at zero.
#[test]
fn the_hold_outlasts_the_startup_it_has_to_cross() {
    let slow = hold_ticks(Situation::Neutral, 0.5, 60.0);
    let fast = hold_ticks(Situation::Neutral, 0.05, 60.0);
    assert!(
        slow > fast,
        "a slower move did not buy a longer hold ({slow} vs {fast})"
    );
    assert!(
        slow >= 30,
        "half a second of startup at 60Hz is 30 ticks the charge never sees, and \
         the hold was {slow}"
    );
}

/// A situation that buys no charge buys no hold either, however long the startup
/// is — otherwise a body in hitstun would stand on the button.
#[test]
fn no_charge_means_no_hold_at_any_startup() {
    assert_eq!(hold_ticks(Situation::Disadvantage, 1.0, 60.0), 0);
}

/// ⭐ A STRING HOLD CROSSES ITS OWN STARTUP FOR THE SAME REASON A CHARGE DOES:
/// the first cancel window opens once the timeline has played the leading move,
/// not at the press. A hold that expires before the window arrives produces a
/// jab and nothing else — which is exactly what a 90-second census showed, `jab`
/// counted and `jab2` never once.
#[test]
fn a_string_hold_outlasts_the_startup_before_the_window_opens() {
    let slow = string_hold_ticks(Situation::Neutral, 0.5, 60.0);
    let fast = string_hold_ticks(Situation::Neutral, 0.05, 60.0);
    assert!(
        slow > fast,
        "a slower leading move did not buy a longer hold ({slow} vs {fast})"
    );
    assert!(
        slow >= 30,
        "half a second of startup at 60Hz is 30 ticks before any window opens, \
         and the hold was {slow}"
    );
}

/// The two gestures are the same button and must not become the same number.
/// A string is walked, a charge is leaned on, and neutral prices them
/// differently on purpose: a jab in neutral is a poke that takes its follow-up
/// if it is there, a smash in neutral is a guess paid for cheaply.
#[test]
fn walking_a_string_and_leaning_on_a_charge_are_priced_apart() {
    let carry = |f: fn(Situation, f32, f32) -> u32, s| f(s, 0.0, 60.0);
    assert_ne!(
        carry(string_hold_ticks, Situation::Advantage),
        carry(hold_ticks, Situation::Advantage),
        "a full string and a full charge collapsed to one number"
    );
    // ⛔ AND THE STRING DOES NOT LENGTHEN WITH THE OPENING, which is where the
    // first version of this got it backwards by copying the charge's shape. A
    // charge spends time the opponent cannot use; a string spends this body's
    // own next decision, and an opening is exactly when it should be spending
    // that on a punish instead.
    assert_eq!(
        carry(string_hold_ticks, Situation::Advantage),
        carry(string_hold_ticks, Situation::Neutral),
        "a string hold grew with the opening - that is the charge's rule, not this one"
    );
}

/// ⛔ Being hit is not the time to stand on the button, however fast the move
/// that would have led the string is. The `return 0` is before the startup term
/// on purpose; adding startup to a refused hold would put a body in hitstun back
/// on the button for the length of its own jab.
#[test]
fn a_refused_string_hold_stays_refused_at_any_startup() {
    assert_eq!(string_hold_ticks(Situation::Disadvantage, 1.0, 60.0), 0);
    assert_eq!(string_hold_ticks(Situation::Recovery, 1.0, 60.0), 0);
}
