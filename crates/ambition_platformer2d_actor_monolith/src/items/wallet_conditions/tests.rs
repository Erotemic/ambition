use super::*;
use bevy::prelude::App;

fn ask(world: &World, price: f64) -> ConditionOutcome {
    can_afford(world, &[AuthoredArg::Number(price)])
}

/// A world whose primary player carries a wallet with `balance`.
fn world_with(balance: i32) -> App {
    let mut app = App::new();
    app.world_mut().spawn((PrimaryPlayer, BodyWallet { balance }));
    app
}

/// ⭐ THE BOUNDARY IS THE ASSERTION, because `>` and `>=` both pass a test that
/// only checks "rich enough" and "too poor".
///
/// A price the player can exactly meet must be affordable: the shop's `buy`
/// spends `balance -= price` and refuses only when short, so an off-by-one here
/// greys out a menu line the purchase itself would have allowed — a defect
/// visible only to a player holding exactly the asking price.
#[test]
fn exactly_enough_is_enough() {
    let app = world_with(25);
    assert_eq!(ask(app.world(), 25.0), ConditionOutcome::Satisfied);
    assert_eq!(ask(app.world(), 24.0), ConditionOutcome::Satisfied);
    assert!(matches!(
        ask(app.world(), 26.0),
        ConditionOutcome::NotSatisfied(_)
    ));
}

/// ⛔ THE WHY-NOT NAMES THE BALANCE, which is the whole point of the structured
/// answer: *"cannot afford"* sends a designer to the shop table, and *"holds 3"*
/// sends them to whatever was supposed to have paid the player.
#[test]
fn the_refusal_says_what_the_player_actually_has() {
    let app = world_with(3);
    let why = match ask(app.world(), 40.0) {
        ConditionOutcome::NotSatisfied(why) => why,
        other => panic!("3 coins does not buy a 40 coin item, got {other:?}"),
    };
    assert_eq!(why.term, "wallet.can_afford");
    assert_eq!(why.subject, "40");
    assert!(
        why.observed.contains('3'),
        "the why-not reports the balance: {}",
        why.observed
    );
}

/// ⛔⛔ NO WALLET IS `Unanswerable`, AND THAT IS A DIFFERENT ANSWER FROM `false`.
///
/// A composition with no currency has not recorded that the player is broke.
/// Answering "cannot afford" would be a confident claim about a world with no
/// money in it — and, worse, an authored gate written as
/// `<<if not can_afford(x)>>` would swing open in exactly the composition that
/// understands the question least.
#[test]
fn a_composition_with_no_wallet_cannot_answer() {
    let app = App::new();
    assert!(
        matches!(ask(app.world(), 10.0), ConditionOutcome::Unanswerable(_)),
        "with no wallet there is no notion of price to report on"
    );
}

/// ⚠ A BODY WITHOUT THE PRIMARY MARKER IS NOT THE PLAYER'S PURSE.
///
/// `BodyWallet` is a component any body may carry. Reading the first wallet in
/// the world rather than the player's would let a vendor's own float answer the
/// player's shop menu, and the two are indistinguishable once summed.
#[test]
fn somebody_elses_wallet_does_not_pay_for_the_player() {
    let mut app = App::new();
    app.world_mut().spawn(BodyWallet { balance: 9_000 });
    assert!(
        matches!(ask(app.world(), 10.0), ConditionOutcome::Unanswerable(_)),
        "a wallet on an unmarked body is not the primary player's"
    );
}

/// ⛔ A NEGATIVE PRICE IS A TYPO, NOT A GIFT.
///
/// `can_afford(-5)` reads as an authoring slip, and satisfying it would hide the
/// slip behind a door that always opens. Reported the way `body.fits` reports a
/// non-positive opening. ⚠ Zero is NOT an error — a free item is a real thing to
/// author — and the two are asserted together so the boundary cannot drift.
#[test]
fn a_negative_price_is_unanswerable_but_free_is_not() {
    let app = world_with(0);
    assert!(matches!(
        ask(app.world(), -5.0),
        ConditionOutcome::Unanswerable(_)
    ));
    assert_eq!(
        ask(app.world(), 0.0),
        ConditionOutcome::Satisfied,
        "a penniless player can afford a free item"
    );
}

/// ⚠ THE PARAMETER MUST BE A NUMBER, and a name that looks like one is not.
#[test]
fn a_price_that_is_not_a_number_is_unanswerable() {
    let app = world_with(100);
    assert!(matches!(
        can_afford(app.world(), &[AuthoredArg::Name("25".to_string())]),
        ConditionOutcome::Unanswerable(_)
    ));
}
