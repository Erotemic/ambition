use super::*;

/// The whole mechanic in one assertion: an opening is worth more charge than a
/// guess is.
#[test]
fn an_opening_buys_more_charge_than_neutral_does() {
    assert!(hold_ticks_for(Situation::Advantage) > hold_ticks_for(Situation::Neutral));
    assert!(hold_ticks_for(Situation::EdgeGuard) > hold_ticks_for(Situation::Neutral));
}

/// A charge is a commitment, and a body that is already losing the exchange has
/// nothing to commit. Holding here would be standing still while being hit.
#[test]
fn a_losing_body_does_not_charge() {
    assert_eq!(hold_ticks_for(Situation::Disadvantage), 0);
    assert_eq!(hold_ticks_for(Situation::Recovery), 0);
}

/// Neutral still pays for some of it — a brain that only ever tapped in neutral
/// would be the behaviour this module exists to remove.
#[test]
fn neutral_still_charges_a_little() {
    assert!(hold_ticks_for(Situation::Neutral) > 0);
}
