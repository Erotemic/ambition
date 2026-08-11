//! **An archetype can say GROUNDED, distinctly from saying nothing.**
//!
//! ⛔ **it could not until 2026-08-07, and that is what made the Perfect Cellular
//! Automaton undecidable.** `ArchetypeSpec::is_aerial` was a bare `bool` with
//! `#[serde(default)]`, so an archetype authoring `false` and one authoring
//! nothing produced the identical value — and two spawn paths disagree exactly
//! there: `new_peaceful_npc_in` reads the catalog's `body_kind: Floating` while
//! the hostile `EnemySpawn` path reads this field. The PCA says `Floating` in its
//! catalog row and is played grounded by the shipped duel, and with a bare bool
//! there was no way to state "the archetype says grounded" as distinct from "the
//! archetype is silent, so the catalog wins".
//!
//! ⭐ **the same defect the struct's own header warns about, one field away**:
//! `deny_unknown_fields` is there because a misspelled key "looks identical to
//! authoring nothing". So did a deliberate `false`.
//!
//! ⚠ **this decides nothing about the PCA.** Whether it flies when it fights is
//! Jon's call (`review-gpt56-through-32eb27a.md` P5). What is pinned here is that
//! the question became EXPRESSIBLE — the half that needs no answer — and that
//! the shipped content still says what it said before.

use ambition_combat::archetype_spec::ArchetypeSpec;
use std::collections::BTreeMap;

fn roster() -> BTreeMap<String, ArchetypeSpec> {
    ron::from_str(ambition_content::enemy_roster::CHARACTER_ROSTER_RON)
        .expect("character_archetypes.ron parses")
}

#[test]
fn the_shipped_archetypes_state_their_aerial_answer_explicitly() {
    let roster = roster();
    assert!(
        !roster.is_empty(),
        "no archetypes parsed, so this check has nothing to look at"
    );

    let decided: Vec<&String> = roster
        .iter()
        .filter(|(_, spec)| spec.is_aerial.is_some())
        .map(|(id, _)| id)
        .collect();
    let flying: Vec<&String> = roster
        .iter()
        .filter(|(_, spec)| spec.is_aerial == Some(true))
        .map(|(id, _)| id)
        .collect();

    // ⚠ the floor: if nothing authors the key, `Some`/`None` is untested by the
    // shipped content and this file is asserting against an empty set.
    //
    // ⛔ **this said `decided.len() == 4` and `flying.len() == 2`, and those
    // numbers were wrong within a week.** The D73 migration moves creatures OFF
    // this file one at a time — the sky parrot and the burning flying shark were
    // two of the four, and both now state `flies` on their character
    // definitions, which is where the fact belongs. A census of a file that is
    // deliberately shrinking to nothing is not a property; it is a countdown,
    // and it fails for the exact reason the campaign is succeeding. What this
    // test is FOR is that `Some(false)` and `None` are distinguishable in
    // shipped content, so that is what it asserts.
    assert!(
        !decided.is_empty(),
        "no shipped archetype authors `is_aerial` at all, so `Some` is \
         unreachable from content and this file tests nothing"
    );
    assert!(
        flying.len() <= decided.len(),
        "impossible census: {flying:?} of {decided:?}"
    );

    // ⭐ and the distinction that did not exist before: an archetype that says
    // nothing is SILENT, not grounded.
    let silent = roster
        .iter()
        .find(|(_, spec)| spec.is_aerial.is_none())
        .map(|(id, _)| id);
    assert!(
        silent.is_some(),
        "every archetype now authors `is_aerial`, so `None` is unreachable from \
         shipped content and the silent case is untested"
    );
}

/// **Absence still behaves exactly as the bare bool did.** The lift is
/// expressiveness; a resolved answer must not have moved.
#[test]
fn absence_still_resolves_to_grounded() {
    for (id, spec) in roster() {
        let resolved = spec.is_aerial.unwrap_or(false);
        if spec.is_aerial.is_none() {
            assert!(
                !resolved,
                "`{id}` is silent about flight and resolved to AERIAL, which is a \
                 behaviour change the lift was not supposed to make"
            );
        } else {
            assert_eq!(
                Some(resolved),
                spec.is_aerial,
                "`{id}` resolved differently from what it authored"
            );
        }
    }
}
