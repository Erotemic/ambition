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
    assert_eq!(
        decided.len(),
        4,
        "the four archetypes that author `is_aerial` are the fixture this test \
         rests on; got {decided:?}"
    );
    assert_eq!(
        flying.len(),
        2,
        "two of the four author flight; got {flying:?}"
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
