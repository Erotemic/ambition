//! ⛔⛔ EVERY ARM STRADDLES ITS BOUNDARY. A price and a restore are one
//! operation with a sign, so each rule needs the case on both sides of it: a
//! payment that fits and one that does not, a heal below the cap and one at it.
//! A self-cost tested only where the fighter is healthy agrees with an
//! implementation that can kill her.

use ambition_characters::actor::body::BodyHealth;
use ambition_characters::actor::Health;

fn body(current: i32, max: i32) -> BodyHealth {
    let mut h = BodyHealth::new(Health::new(max));
    let spent = max - current;
    if spent > 0 {
        h.damage(spent);
    }
    h
}

#[test]
fn a_price_takes_what_it_asks_for_and_charges_the_meter() {
    let mut h = body(6, 6);
    assert_eq!(h.spend(2, 1), 2);
    assert_eq!(h.current(), 4);
    // ⭐⭐ THE METER IS THE POINT. In a platform fighter the accumulated meter
    // decides how far a body launches, so health bought with health has to
    // show up there or the cost is invisible at the only moment it matters.
    assert_eq!(h.damage_taken(), 2, "spending health advances the meter");
}

/// ⛔⛔ A MOVE THAT CAN KILL YOU BY BEING PRESSED IS NOT A COST. The arm on the
/// other side of the floor.
#[test]
fn a_price_never_takes_a_fighter_below_the_floor() {
    let mut h = body(2, 6);
    assert_eq!(h.spend(5, 1), 1, "she can afford exactly one point");
    assert_eq!(h.current(), 1);
    assert!(h.alive(), "a self-cost must never be a suicide button");
}

/// ⛔ AND AT THE FLOOR IT IS FREE, DELIBERATELY. The alternative is a special
/// that stops existing at low health, which is when she needs it.
#[test]
fn a_fighter_already_at_the_floor_pays_nothing_and_lives() {
    let mut h = body(1, 6);
    assert_eq!(h.spend(3, 1), 0);
    assert_eq!(h.current(), 1);
    assert_eq!(h.damage_taken(), 5, "and the meter does not move either");
}

/// ⛔⛔ A FLOOR OF ZERO IS CLAMPED UP. The field exists so a character can be
/// MORE cautious than "never self-KO", never less, and an author who writes `0`
/// (or forgets the field, whose default is `0`) must not thereby get a body that
/// can pay its way to death.
#[test]
fn a_floor_below_one_is_raised_to_one() {
    let mut h = body(3, 6);
    assert_eq!(h.spend(99, 0), 2);
    assert_eq!(h.current(), 1);
    assert!(h.alive());
}

/// ⛔ A COST IS NOT AN INJURY, and the sharpest difference is this one:
/// `damage` refuses a body holding invulnerability, so routing a price through
/// it would hand a fighter still under respawn protection a FREE special.
#[test]
fn a_price_is_charged_even_to_an_invulnerable_body() {
    let mut immune = body(6, 6);
    immune
        .health
        .invulnerable
        .set(ambition_characters::actor::Invulnerability::SCRIPTED, true);
    // ⭐ THE PREMISE, MEASURED. Without this the arm below passes against a
    // body that was never immune, and the rule it claims to defend is untested.
    assert!(
        !immune.damage(2),
        "premise: an invulnerable body refuses damage"
    );
    assert_eq!(immune.current(), 6);
    assert_eq!(
        immune.spend(2, 1),
        2,
        "a price is not an injury and does not consult immunity"
    );
    assert_eq!(immune.current(), 4);
}

#[test]
fn a_restore_refills_the_pool_and_repays_the_meter() {
    let mut h = body(2, 6);
    assert_eq!(h.damage_taken(), 4);
    h.heal(3);
    assert_eq!(h.current(), 5);
    assert_eq!(
        h.damage_taken(),
        1,
        "a heal that left the meter alone would be a heal you cannot feel"
    );
}

/// ⛔ THE PAIRED ARM: a restore stops at the pool's own maximum rather than
/// banking overflow.
#[test]
fn a_restore_stops_at_full() {
    let mut h = body(5, 6);
    h.heal(4);
    assert_eq!(h.current(), 6);
    assert_eq!(h.damage_taken(), 0);
}

/// ⛔ A ZERO OR NEGATIVE PRICE IS NOT A BACKWARDS HEAL.
#[test]
fn a_non_positive_price_takes_nothing() {
    let mut h = body(4, 6);
    assert_eq!(h.spend(0, 1), 0);
    assert_eq!(h.spend(-3, 1), 0);
    assert_eq!(h.current(), 4);
}
