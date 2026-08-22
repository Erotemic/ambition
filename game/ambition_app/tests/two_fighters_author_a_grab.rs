//! The capture vocabulary is not tied to one provider.
//!
//! the deliberate falsifier for stage J. George Booul's table lives in
//! `ambition_demo_smash` — the game that owns the stage — and the Pirate
//! Admiral's lives in `ambition_content`, the named-content crate. If the
//! capture authoring had quietly grown a dependency on one game's own helpers,
//! only one of these two could have been written.
//!
//! this reads the CONTRACTS rather than driving a match, on purpose. What is
//! under test is that two providers can SAY a grab; that a said grab is caught,
//! held, pummelled and thrown is pinned by the capture runtime's own guards.

use ambition_platformer2d::entity_catalog::{
    MoveSpec, MovesetContract, WindowTag, CAPTURE_PUMMEL_VERB, CAPTURE_THROW_FORWARD_VERB,
    GRAB_VERB,
};

fn george() -> MovesetContract {
    ambition_demo_smash::george_booul_moveset::george_booul_moveset()
}

fn admiral() -> MovesetContract {
    ambition_content::pirate_admiral_moveset::pirate_admiral_moveset()
}

fn move_for<'a>(contract: &'a MovesetContract, verb: &str) -> &'a MoveSpec {
    contract
        .move_for_verb(verb)
        .unwrap_or_else(|| panic!("no `{verb}` in this fighter's contract"))
}

/// BOTH FIGHTERS OFFER EVERY CAPTURE VERB — grab, pummel, and all four throws.
#[test]
fn two_providers_each_author_a_grab_a_pummel_and_a_throw() {
    for (who, contract) in [("george", george()), ("admiral", admiral())] {
        for verb in [GRAB_VERB, CAPTURE_PUMMEL_VERB, CAPTURE_THROW_FORWARD_VERB] {
            assert!(
                contract.verbs.contains_key(verb),
                "{who} does not answer `{verb}`, so its Grab slot would advertise \
                 a button with nothing behind it"
            );
        }
        // INVERTED. This asserted the other three throws were
        // ABSENT, which was true of both fighters when it was written and is
        // true of neither now: the admiral took the roster's back/up/down pass
        // and George authored his three as a modus tollens, a tautology and a
        // reductio. the claim it was making — an absent throw is absent, not
        // silently substituted — is NOT dropped, and was never this test's to
        // keep: `smash_capture`'s own tests build a forward-only kit and assert
        // `bound()` yields three verbs and not six.
        for verb in [
            "capture_throw_back",
            "capture_throw_up",
            "capture_throw_down",
        ] {
            assert!(
                contract.verbs.contains_key(verb),
                "{who} does not answer `{verb}`, so a grab press in that \
                 direction is a dead input"
            );
        }
    }
}

/// A GRAB'S ATTEMPT IS LIVE, AND ONLY DURING ITS ACTIVE WINDOW.
///
/// The authoring helper enforces this at construction; this checks it survived
/// the lowering into a contract, which is the part a fighter file cannot see.
#[test]
fn each_authored_grab_carries_a_live_capture_attempt() {
    for (who, contract) in [("george", george()), ("admiral", admiral())] {
        let grab = move_for(&contract, GRAB_VERB);
        let live: Vec<&WindowTag> = grab
            .windows
            .iter()
            .filter(|w| w.sustain_effect.is_some())
            .map(|w| &w.tag)
            .collect();
        assert_eq!(
            live.len(),
            1,
            "{who}'s grab sustains its attempt on {} windows",
            live.len()
        );
        assert!(
            matches!(live[0], WindowTag::Active),
            "{who}'s grab attempt is live outside its Active window, so it \
             catches during the tell or during recovery"
        );
        assert!(
            grab.windows.iter().all(|w| w.volumes.is_empty()),
            "{who}'s grab carries a hit volume — the same frames would both grab \
             and strike"
        );
    }
}

/// THE TWO FIGHTERS DID NOT CLONE ONE SPEC.
///
/// The whole reason two customers were required. If a later hand copies one
/// fighter's grab onto another and edits the id, this is what notices — and
/// "fighter-specific tuning lives in the fighter" stops being a claim the
/// architecture makes and starts being one it keeps.
#[test]
fn the_two_authored_grabs_are_genuinely_different_fighters() {
    let (g, a) = (george(), admiral());
    let (g_grab, a_grab) = (move_for(&g, GRAB_VERB), move_for(&a, GRAB_VERB));

    // The tell: the admiral boards fast, George reaches slowly.
    let startup = |m: &MoveSpec| {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .map(|w| w.start_s)
            .expect("a grab has an Active window")
    };
    assert!(
        startup(g_grab) > startup(a_grab) * 1.5,
        "the two grabs start within a hair of each other ({} vs {}) — that is one \
         spec wearing two names",
        startup(g_grab),
        startup(a_grab)
    );
    assert_ne!(
        g_grab.duration_s, a_grab.duration_s,
        "both grabs run for exactly the same time"
    );

    let (g_pummel, a_pummel) = (
        move_for(&g, CAPTURE_PUMMEL_VERB),
        move_for(&a, CAPTURE_PUMMEL_VERB),
    );
    assert!(
        g_pummel.duration_s > a_pummel.duration_s,
        "George's pummel is not the slower one, so the weight/rate trade the two \
         tables describe is not in the data"
    );

    let (g_throw, a_throw) = (
        move_for(&g, CAPTURE_THROW_FORWARD_VERB),
        move_for(&a, CAPTURE_THROW_FORWARD_VERB),
    );
    assert_ne!(
        g_throw.duration_s, a_throw.duration_s,
        "both forward throws run for exactly the same time"
    );
}
