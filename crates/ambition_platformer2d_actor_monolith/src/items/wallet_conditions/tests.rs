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

/// ⛔⛔ THE GUARD AND THE ACTION MUST AGREE ON EVERY AUTHORED PRICE — the
/// property a shared normalizer exists to make true, asserted by running BOTH.
///
/// For half a day they did not. `cmd_buy_item` built its request with
/// `price.max(0.0) as i32` while this condition compared the balance against the
/// raw `f64`, so a guard answering NO sat in front of an action that then
/// succeeded. ⭐ THIS TEST RUNS THE REAL TRANSACTION, not a restatement of the
/// rule: `ShopTransactionRequested::apply` is the code the simulation runs, and
/// a test that only compared two normalizations would agree with itself.
///
/// ⛔⛔ AND HERE IS WHAT IT DOES *NOT* CATCH, established by poisoning it rather
/// than by reasoning: re-introducing the exact original fork — this condition
/// reading the raw `f64` while the action takes the normalized coins — leaves
/// this test GREEN. Once `authored_price` REFUSES fractional and negative
/// values, the only prices it accepts are integral, and for those the two
/// readings are equal by arithmetic. ⇒ The fork can only show itself on prices
/// the normalizer now rejects, and this arm skips those by construction.
///
/// ⭐ So the defect is prevented by the SHAPE — one normalizer, consulted by
/// both roads — and what pins that shape is
/// `a_fractional_price_is_not_quietly_rounded_into_an_affordable_one`, which
/// asserts BOTH roads refuse `25.7` and `-5`. Clamping instead of refusing
/// reddens that test and this one stays green, which is the correct division of
/// labour and worth stating so neither is trusted past its reach.
#[test]
fn the_question_and_the_transaction_agree_on_every_authored_price() {
    use ambition_items::shop::{authored_price, ShopSide, ShopTransactionRequested, ShopTx};
    use ambition_items::{Item, OwnedItems};

    // The two the fork produced, the boundary, free, and a price nobody can meet.
    for price in [25.7_f64, -5.0, 25.0, 24.0, 26.0, 0.0, f64::NAN] {
        let app = world_with(25);
        let asked = ask(app.world(), price);

        let Ok(coins) = authored_price(price) else {
            assert!(
                matches!(asked, ConditionOutcome::Unanswerable(_)),
                "`{price}` is not a price, so the question cannot answer it — and the \
                 transaction refuses it, so a `NotSatisfied` here would be a guard \
                 disagreeing with an action that never runs"
            );
            continue;
        };

        let mut wallet = BodyWallet { balance: 25 };
        let mut owned = OwnedItems::default();
        let outcome = ShopTransactionRequested {
            item: Item::HealthCell,
            price: coins,
            side: ShopSide::Buy,
        }
        .apply(&mut wallet, &mut owned);

        let affordable = asked == ConditionOutcome::Satisfied;
        let bought = outcome != ShopTx::CantAfford;
        assert_eq!(
            affordable, bought,
            "`wallet.can_afford({price})` said {affordable} and the real transaction \
             said {bought} ({outcome:?}). A guard that refuses what the action performs \
             — or permits what it refuses — is worse than no guard, because it reads \
             as protection."
        );
    }
}

/// ⛔ A FRACTIONAL PRICE IS REFUSED BY BOTH ROADS, not rounded by one of them.
///
/// `25.7` was the sharp end of the fork: the guard compared `25 >= 25.7` and
/// said no, while `buy_item` truncated to a 25-coin charge the wallet could pay.
/// ⇒ The player was told they could not afford it and then charged for it.
#[test]
fn a_fractional_price_is_not_quietly_rounded_into_an_affordable_one() {
    use ambition_items::shop::{authored_price, AuthoredPriceProblem};

    let app = world_with(25);
    assert!(
        matches!(ask(app.world(), 25.7), ConditionOutcome::Unanswerable(_)),
        "coins are whole, so a fractional price is a question about nothing"
    );
    assert_eq!(
        authored_price(25.7),
        Err(AuthoredPriceProblem::Fractional),
        "and the ACTION must refuse it too — truncating here is what let the \
         charge happen after the refusal"
    );
    assert_eq!(
        authored_price(-5.0),
        Err(AuthoredPriceProblem::Negative),
        "a negative price became a FREE purchase under `price.max(0.0)`"
    );
    assert_eq!(authored_price(45.0), Ok(45), "an ordinary authored price still reads");
}

/// ⛔⛔ TWO PRIMARY WALLETS CANNOT PRODUCE A CONFIDENT ANSWER, because
/// `apply_shop_transactions` takes `wallets.single_mut()` and refuses that world.
///
/// Answering from the FIRST of several would let a malformed or mid-transition
/// world advertise affordability and then refuse every purchase — the question
/// and the action have to fail on the same worlds, not merely agree on good ones.
#[test]
fn a_world_with_two_primary_wallets_cannot_say_what_the_player_can_afford() {
    let mut app = App::new();
    app.world_mut().spawn((PrimaryPlayer, BodyWallet { balance: 9_000 }));
    app.world_mut().spawn((PrimaryPlayer, BodyWallet { balance: 0 }));
    assert!(
        matches!(ask(app.world(), 10.0), ConditionOutcome::Unanswerable(_)),
        "with two primary purses there is no single wallet the shop would charge, \
         and the transaction system declines such a world outright"
    );
}
