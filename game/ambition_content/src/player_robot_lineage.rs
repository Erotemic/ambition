//! **The player robot's incarnations, emitted from one source.**
//!
//! The protagonist has been rebuilt twice, and Ambition's answer to that is not
//! a changelog. `npc_player_robot_v2`'s catalog row says it outright: an old
//! build is *"preserved as a CHARACTER rather than as history, [because]
//! Ambition wants old versions of yourself to be things you can meet, talk to,
//! and fight, so this keeps its own id, its own sheet, and its own pedestal
//! instead of living in a git object."*
//!
//! # These are separate characters, not variants of one
//!
//! That distinction is the whole design and it is easy to lose. A "player robot"
//! with a version *parameter* would be one character wearing three coats, and
//! every system downstream would have to learn what a version is in order to ask
//! anything useful. What exists instead is three characters that happen to share
//! a face: each has its own stable id, its own art, and its own kit — v0 is
//! peaceful, v2 swings the generic striker swipe the protagonist used to carry,
//! v3 carries the host-code kit — and nothing downstream knows they are related.
//!
//! # What the sharing is, and what it is NOT
//!
//! §4.3's rule, stated on [`Lineage`]: *"two independent, fully-resolved
//! products with distinct stable ids, emitted by one generator from shared
//! source. The engine **never learns what a mode is** — there is no patch layer
//! and no override precedence."*
//!
//! So the sharing lives HERE, in a generator, and stops at the door. What comes
//! out is three complete definitions. [`Incarnation`] is the part that differs;
//! [`definition`] is the part they have in common. Adding v4 is a struct literal
//! — which is the same shape `versus_fighters::DuelistNumbers` uses for the two
//! duelists, and for the same reason.
//!
//! ⚠ [`Lineage::derived_from`] is **provenance, not authority**. It records that
//! v3 replaced v2; nothing resolves through it, and no field of v3 is inherited
//! from v2. A reader who treats it as an inheritance edge has reintroduced the
//! patch layer this design exists to refuse.

use ambition_actors::character_runtime::{
    CharacterBindings, CharacterDefinition, CharacterDefinitionAppExt, Lineage,
};

/// One incarnation of the player robot: everything about it that is not shared.
///
/// Deliberately TWO fields, and it had four. **Jon ruled 2026-07-31 that each
/// robot version is a different character** — so a version's FACTS (its name,
/// its sheet, its physicals, its voice) belong to that character's content row,
/// and what Rust owns is the reusable lineage COMPOSITION: who exists, and which
/// one replaced which.
///
/// `display_name` and `sheet` lived here AND in `character_catalog.ron`, with
/// nothing deciding which won per field — the AF4b duplicate-authority row. The
/// `voice` field went the same way earlier the same day, and that one was worse
/// than duplication: the catalog outranks a definition's voice, so
/// `player_robot_v2`'s Rust lines could never be heard at all. Reading the row
/// is what makes "content owns the facts" structural instead of a convention.
pub struct Incarnation {
    /// Stable id. **Never reused and never repointed** — that is what makes an
    /// old build a thing you can meet rather than a thing you remember. A future
    /// v4 does not take v3's id; it takes its own, and v3 keeps standing.
    ///
    /// It is also the key into the catalog: everything else this character is
    /// comes from the row under this id.
    pub id: &'static str,
    /// The incarnation this one replaced. `None` for the original.
    ///
    /// Provenance only — see the module doc. It exists so the lineage is a fact
    /// the code owns rather than a sentence in an authoring description.
    pub replaces: Option<&'static str>,
}

// ⚠ **no `voice` field, and its removal is AF4b** (Jon ruled 2026-07-31: each
// version is a different CHARACTER, so version-specific facts belong to the
// per-character content row and Rust owns reusable lineage COMPOSITION).
//
// It was authored here AND in `character_catalog.ron`, with v0's two lines
// duplicated verbatim between them — and the duplicate was not symmetric. The
// catalog outranks a definition's voice (`npc_ambient_bark_line` asks
// `catalog.bark_line` first; the definition answers only when the catalog had
// nothing), and `CatalogEntry::bark` falls through `barks.pick` to
// `fallback_dialogue`. So `player_robot_v2`, which authored BOTH, could never
// reach its Rust voice at all — it was dead, and the test asserting every
// incarnation "says something" was green over it because it read the struct
// rather than the runtime.
//
// v0 and v3 authored only `barks.hall`, so their Rust lines DID speak — but only
// away from a pedestal, which is the one place they are usually seen. Both rows
// gained a `fallback_dialogue` carrying exactly those lines, so the voice they
// had is the voice they keep, from one authority.

/// **v0 — the original.** Its own bark: *"Version zero. Everything after me was
/// a patch note."*
pub const V0: Incarnation = Incarnation {
    id: "robot",
    replaces: None,
};

/// **v2 — the build that shipped before the SVG rig.**
///
/// There is no v1. v2's own dialogue handles the question (*"There is no v1. Ask
/// someone else why."*) and its row records the reason: it is a joke, not a gap.
pub const V2: Incarnation = Incarnation {
    id: "player_robot_v2",
    replaces: Some(V0.id),
};

/// **v3 — the body you are playing right now.**
///
/// Named for its version rather than for being current, so v4 costs a struct
/// literal instead of a rename. Until 2026-07-29 this was the one incarnation
/// whose id meant "whichever is latest", which would have made preserving it a
/// retroactive rename of every sheet, rig and reference it owns.
pub const V3: Incarnation = Incarnation {
    id: "player_robot_v3",
    replaces: Some(V2.id),
};

/// The whole lineage, oldest first.
pub const LINEAGE: &[&Incarnation] = &[&V0, &V2, &V3];

/// Build one incarnation's complete definition, reading its FACTS from the
/// catalog row under its id.
///
/// Everything the three have in common lives here and nowhere else. What it does
/// NOT do is inherit: no field is copied from `replaces`, and the definition that
/// comes out is complete on its own.
///
/// ⚠ **the row is the authority for the name and the art, and this used to
/// duplicate both.** `load_catalog` is a pure parse of an `include_str!`
/// constant — no `App`, no plugin order, no asset load — so there is no ordering
/// reason for the Rust side to carry its own copy, which is the objection that
/// kept AF4b open. The sheet comes through
/// [`CatalogEntry::manifest_target`](ambition_characters::actor::character_catalog::CatalogEntry::manifest_target),
/// the same canonical projection `audit_character_authority_parity` compares
/// with — a catalog row names FILES (`sprites/player_robot_v2_spritesheet.ron`)
/// and a definition names a TARGET (`player_robot_v2`).
///
/// A missing row is a panic rather than a fallback: an incarnation the catalog
/// does not describe cannot be registered as a character, and inventing a name
/// for it here would put the duplication back one `unwrap_or` at a time.
pub fn definition(incarnation: &Incarnation) -> CharacterDefinition {
    definition_from(&crate::character_catalog::load_catalog(), incarnation)
}

/// [`definition`] against an already-parsed catalog, so registering the whole
/// lineage parses the roster ONCE instead of once per incarnation.
fn definition_from(
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    incarnation: &Incarnation,
) -> CharacterDefinition {
    let row = catalog.get(incarnation.id).unwrap_or_else(|| {
        panic!(
            "player-robot incarnation `{}` has no row in character_catalog.ron — \
             the lineage names who exists; the row says what they are",
            incarnation.id
        )
    });
    let sheet = row.manifest_target().unwrap_or_else(|| {
        panic!(
            "`{}`'s catalog manifest `{}` does not follow the \
             `<target>_spritesheet.ron` convention, so no sheet target can be \
             derived from it",
            incarnation.id, row.manifest
        )
    });
    let mut definition = CharacterDefinition::new(
        incarnation.id,
        row.display_name.clone(),
        crate::AMBITION_CONTENT_PROVIDER,
    )
    .with_sheet(sheet);
    definition.lineage = Some(Lineage {
        derived_from: incarnation.replaces.map(str::to_string),
        // Left `None` deliberately. These are hand-authored incarnations, not
        // the output of a crossover generator, so there is no revision or source
        // fingerprint to state — and inventing one would make provenance that
        // cannot be traced look like provenance that can.
        generator_revision: None,
        source_fingerprint: None,
    });
    definition
}

/// Register every incarnation as a character in its own right.
///
/// The KIT is deliberately not authored here: each incarnation's catalog row
/// already states what it can do, and preparation folds that row in at the
/// finalization barrier. Authoring it a second time on the definition would be
/// two declarations of one fact — exactly the split the character-authority
/// campaign exists to remove.
pub fn register(app: &mut bevy::prelude::App) {
    // Parsed ONCE for the whole lineage. Three strings do not justify three
    // parses of the roster, and the cast is only going to grow.
    let catalog = crate::character_catalog::load_catalog();
    for incarnation in LINEAGE {
        app.try_register_character(
            definition_from(&catalog, incarnation),
            // The engine's sheet vocabulary, so a target that names nothing is
            // reported at load with a did-you-mean instead of silently drawing
            // the marked rectangle.
            CharacterBindings::default().with_engine_sheet_vocabulary(),
        )
        .unwrap_or_else(|error| panic!("player-robot incarnation rejected: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The chain is well-formed, and it is a chain.**
    ///
    /// Exactly one origin, every other link naming the incarnation before it,
    /// and no id repeated. A lineage that forked or looped would still compile
    /// and would quietly make "the version before this one" unanswerable.
    #[test]
    fn the_lineage_is_an_unbroken_chain_of_distinct_characters() {
        let ids: Vec<&str> = LINEAGE.iter().map(|inc| inc.id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            ids.len(),
            "two incarnations share an id, so one of them cannot be met: {ids:?}"
        );

        let mut previous: Option<&str> = None;
        for incarnation in LINEAGE {
            assert_eq!(
                incarnation.replaces, previous,
                "incarnation '{}' does not name the one before it — the lineage \
                 is a chain, and a break in it makes provenance a guess",
                incarnation.id
            );
            previous = Some(incarnation.id);
        }
    }

    /// Every incarnation's art resolves, and to a DIFFERENT sheet.
    ///
    /// ⚠ the second half is the one worth having. Eighteen shipped sheets
    /// declare `target: "robot"` — the name of the procedural generator, not of
    /// a character — so "the target resolves" is satisfied by all three
    /// resolving to the same robot. Distinctness is what says three incarnations
    /// actually look like three characters.
    ///
    /// ⚠ **it asks the DEFINITION now, not the struct.** The sheet used to be a
    /// `&'static str` on `Incarnation`, so this test read a Rust literal and
    /// would have stayed green while the catalog row — which is what the art
    /// pipeline actually resolves — said something else entirely. That is the
    /// same mistake `every_incarnation_says_something` had to be rewritten out
    /// of on the voice field the same day.
    #[test]
    fn every_incarnation_resolves_its_own_distinct_sheet() {
        use ambition_sprite_sheet::character::sheets;

        let mut seen: Vec<String> = Vec::new();
        for incarnation in LINEAGE {
            let sheet = definition(incarnation)
                .sheet
                .expect("the lineage always names a sheet target");
            assert!(
                sheets::record_for_target(&sheet).is_some(),
                "incarnation '{}' names sheet target '{sheet}', which resolves to \
                 nothing — it would draw the marked placeholder",
                incarnation.id,
            );
            assert!(
                !seen.contains(&sheet),
                "incarnation '{}' shares sheet '{sheet}' with an earlier one, so \
                 the lineage is one body wearing three names",
                incarnation.id,
            );
            seen.push(sheet);
        }
    }

    /// **The name comes from the row, and there is only one row.** (AF4b)
    ///
    /// The duplication this closes: `Incarnation` carried a `display_name` and
    /// so does the catalog, with nothing deciding which won per field. Now the
    /// definition IS the row's answer, so `DisplayNameDisagreement` cannot fire
    /// for these three by construction rather than by luck.
    #[test]
    fn every_incarnation_presents_under_its_catalog_name() {
        let catalog = crate::character_catalog::load_catalog();
        for incarnation in LINEAGE {
            let row = catalog
                .get(incarnation.id)
                .expect("every incarnation has a catalog row");
            assert_eq!(
                definition(incarnation).display_name,
                row.display_name,
                "incarnation '{}' presents under a name the catalog does not \
                 give it",
                incarnation.id,
            );
        }
    }

    /// **Nobody in the lineage stands mute — asked of the RUNTIME, not the
    /// struct.** (AF4b)
    ///
    /// This used to assert `!definition.voice.is_empty()`, which is a fact about
    /// a Rust literal and not about what anybody hears. It was green while
    /// `player_robot_v2`'s lines were unreachable: the catalog outranks a
    /// definition's voice, and v2's row authored both a `barks.hall` pool AND a
    /// `fallback_dialogue`, so `CatalogEntry::bark` always answered first.
    ///
    /// So ask the question the ticker asks. `bark` falls through the situation
    /// pool to `fallback_dialogue`, and a row with neither returns `None` — which
    /// is exactly the silence this test is named for.
    #[test]
    fn every_incarnation_says_something() {
        let catalog = crate::character_catalog::load_catalog();
        for incarnation in LINEAGE {
            for situation in [
                ambition_characters::actor::character_catalog::BarkSituation::Hall,
                ambition_characters::actor::character_catalog::BarkSituation::Idle,
            ] {
                assert!(
                    catalog.bark_line(incarnation.id, situation, 0).is_some(),
                    "incarnation '{}' has nothing to say in {situation:?}, so the \
                     ambient ticker skips it and it stands there silent",
                    incarnation.id,
                );
            }
        }
    }

    /// Every incarnation is in the playable cast.
    ///
    /// The point of the whole arrangement: "play as the build before this one"
    /// is a selection, not a content edit.
    #[test]
    fn every_incarnation_can_be_worn() {
        for incarnation in LINEAGE {
            assert!(
                crate::character_catalog::PLAYABLE_ROSTER.contains(&incarnation.id),
                "incarnation '{}' is a character you can meet and not one you \
                 can be — put it in PLAYABLE_ROSTER",
                incarnation.id,
            );
        }
    }
}
