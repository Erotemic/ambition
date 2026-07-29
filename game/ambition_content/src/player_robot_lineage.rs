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
/// Deliberately four fields. The point of a generator is that the shape is
/// common; a knob only one incarnation ever sets belongs on that incarnation's
/// catalog row, not here.
pub struct Incarnation {
    /// Stable id. **Never reused and never repointed** — that is what makes an
    /// old build a thing you can meet rather than a thing you remember. A future
    /// v4 does not take v3's id; it takes its own, and v3 keeps standing.
    pub id: &'static str,
    /// What it is called. Includes the version, because the version is the
    /// character: two of these are on pedestals in the Hall introducing
    /// themselves.
    pub display_name: &'static str,
    /// The sheet manifest target its art resolves through.
    ///
    /// ⚠ the sheet's own `target:` field is NOT this. Eighteen shipped sheets
    /// declare `target: "robot"` because that names the procedural GENERATOR
    /// they came out of, not the character; the sheet index re-keys a
    /// single-record sheet by its FILE ROOT, which is what distinguishes
    /// `robot` from `player_robot_v2`. Registration resolves this against the
    /// engine's sheet vocabulary, so a wrong one is named at load instead of
    /// drawing a placeholder in silence.
    pub sheet: &'static str,
    /// The incarnation this one replaced. `None` for the original.
    ///
    /// Provenance only — see the module doc. It exists so the lineage is a fact
    /// the code owns rather than a sentence in an authoring description.
    pub replaces: Option<&'static str>,
}

/// **v0 — the original.** Its own bark: *"Version zero. Everything after me was
/// a patch note."*
pub const V0: Incarnation = Incarnation {
    id: "robot",
    display_name: "Robot",
    sheet: "robot",
    replaces: None,
};

/// **v2 — the build that shipped before the SVG rig.**
///
/// There is no v1. v2's own dialogue handles the question (*"There is no v1. Ask
/// someone else why."*) and its row records the reason: it is a joke, not a gap.
pub const V2: Incarnation = Incarnation {
    id: "npc_player_robot_v2",
    display_name: "Player Robot v2",
    sheet: "player_robot_v2",
    replaces: Some(V0.id),
};

/// **v3 — the body you are playing right now.**
///
/// Named for its version rather than for being current, so v4 costs a struct
/// literal instead of a rename. Until 2026-07-29 this was the one incarnation
/// whose id meant "whichever is latest", which would have made preserving it a
/// retroactive rename of every sheet, rig and reference it owns.
pub const V3: Incarnation = Incarnation {
    id: "player",
    display_name: "Player",
    sheet: "player_robot_v3",
    replaces: Some(V2.id),
};

/// The whole lineage, oldest first.
pub const LINEAGE: &[&Incarnation] = &[&V0, &V2, &V3];

/// Build one incarnation's complete definition.
///
/// Everything the three have in common lives here and nowhere else. What it does
/// NOT do is inherit: no field is copied from `replaces`, and the definition that
/// comes out is complete on its own.
pub fn definition(incarnation: &Incarnation) -> CharacterDefinition {
    let mut definition = CharacterDefinition::new(
        incarnation.id,
        incarnation.display_name,
        crate::AMBITION_CONTENT_PROVIDER,
    )
    .with_sheet(incarnation.sheet);
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
    for incarnation in LINEAGE {
        app.try_register_character(
            definition(incarnation),
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
    #[test]
    fn every_incarnation_resolves_its_own_distinct_sheet() {
        use ambition_sprite_sheet::character::sheets;

        let mut seen: Vec<&str> = Vec::new();
        for incarnation in LINEAGE {
            assert!(
                sheets::record_for_target(incarnation.sheet).is_some(),
                "incarnation '{}' names sheet target '{}', which resolves to \
                 nothing — it would draw the marked placeholder",
                incarnation.id,
                incarnation.sheet,
            );
            assert!(
                !seen.contains(&incarnation.sheet),
                "incarnation '{}' shares sheet '{}' with an earlier one, so the \
                 lineage is one body wearing three names",
                incarnation.id,
                incarnation.sheet,
            );
            seen.push(incarnation.sheet);
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
