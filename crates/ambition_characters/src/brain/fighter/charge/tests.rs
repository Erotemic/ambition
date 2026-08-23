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
