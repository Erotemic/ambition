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
//! ⭐⭐ **AND THE PROPERTY MOVED TO THE CHARACTER, 2026-08-11 (ledger D89).** The
//! PCA was the only shipped archetype that authored `is_aerial` at all, and its
//! row is DELETED — so a census of `ArchetypeSpec` can no longer reach `Some`,
//! exactly as this file's own body predicted: *"a census of a file that is
//! deliberately shrinking to nothing is not a property; it is a countdown, and it
//! fails for the exact reason the campaign is succeeding."*
//!
//! ⇒ the three-state now lives on `CharacterLocomotion::flies`, which is where
//! the fact belongs, and shipped content authors all three cases. Jon's call is
//! also in: *"in smash PCA should not have the fly ability"* — so the automatons
//! say `Some(false)` while the parrot and the burning shark say `Some(true)`.
//! ⚠ what is pinned is unchanged: **stated-grounded and silent must be
//! distinguishable in shipped content.** Only the type carrying the distinction
//! moved.

use ambition_combat::archetype_spec::ArchetypeSpec;
use std::collections::BTreeMap;

fn roster() -> BTreeMap<String, ArchetypeSpec> {
    ron::from_str(ambition_content::enemy_roster::CHARACTER_ROSTER_RON)
        .expect("character_archetypes.ron parses")
}

#[test]
fn shipped_characters_state_their_flight_answer_explicitly() {
    use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

    let authored = |id: &str| {
        ambition_content::character_catalog::authored_intrinsics(
            id,
            CharacterDefinition::new(id, id, "ambition_content"),
        )
        .locomotion
        .and_then(|locomotion| locomotion.flies)
    };

    // ⭐ **a character that says it FLIES.** Without one, `Some(true)` is
    // unreachable from shipped content and the type is decorative.
    assert_eq!(
        authored("stochastic_parrot"),
        Some(true),
        "the parrot flies and must say so itself — its catalog row's \
         `body_kind: Floating` stopped deciding locomotion (D89)"
    );

    // ⭐ **a character that says it does NOT**, which is the case that could not
    // be expressed at all before: the PCA's row still says `Floating`, and that
    // is now a claim about its SILHOUETTE only.
    assert_eq!(
        authored("perfect_cellular_automaton"),
        Some(false),
        "the PCA is a grounded-base hybrid and its own definition has to say so, \
         or a presentation enum decides its locomotion again"
    );

    // ⭐ **and SILENCE, distinct from both.** A character that never mentions
    // flight leaves the question open at the source layer; preparation resolves
    // it once, so nothing downstream has to re-ask.
    assert_eq!(
        authored("npc_exploding_mite"),
        None,
        "a character that authors no flight answer must read as SILENT here — if \
         this becomes `Some(false)`, `None` is unreachable and the three-state is \
         a two-state wearing three names"
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

/// **THE PULSE SURVIVED ITS ARCHETYPE ROW.**
///
/// ⛔ "Cellular Pulse" was the repository's first data-driven move and it lived
/// inline on `character_archetypes.ron`'s `cellular_automaton_fighter` row, with
/// a monolith test proving the row deserialized. That row is deleted (ledger
/// D89); this is the same proof where the move now lives.
///
/// ⚠ the numbers are the row's verbatim — a migration that retuned on the way
/// would be a retune wearing a migration's commit.
#[test]
fn the_cellular_pulse_survived_its_archetype_row() {
    let moveset = ambition_content::cellular_automaton_moveset::cellular_pulse_moveset();
    assert_eq!(
        moveset.verbs.get("special").map(String::as_str),
        Some("cellular_pulse"),
        "the `special` verb must still resolve the pulse, or the PCA presses a \
         button and nothing happens"
    );
    let pulse = moveset
        .moves
        .iter()
        .find(|m| m.id == "cellular_pulse")
        .expect("the verb names a move that exists");
    let active = pulse
        .windows
        .iter()
        .find(|w| {
            matches!(
                w.tag,
                ambition_platformer2d::entity_catalog::WindowTag::Active
            )
        })
        .expect("a move with no ACTIVE window is a telegraph that never lands");
    assert!(
        !active.volumes.is_empty(),
        "the active window carries no hit volume, so the pulse cannot damage \
         anybody through the shared moveset runtime"
    );
    assert_eq!((active.start_s, active.end_s), (0.40, 0.54));
}
