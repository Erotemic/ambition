//! **THE BEVY `App` LAYER OVER CHARACTER PREPARATION** — registration, the
//! finalization barrier, and the plugin that closes it.
//!
//! ⭐⭐ **the authored MODEL and the preparation PIPELINE are not here any more**
//! (campaign P1.7, 2026-08-12). They are `ambition_characters::prepared`, which
//! is where the reusable character domain belongs: Jon's brief is explicit that
//! a dependency obstacle must not be solved by leaving the authoritative
//! character model inside the actor monolith. What stays is the part that is
//! genuinely an App's — `try_register_character`, `StagedCharacterOverrides`,
//! `CharacterPreparationPlugin`, and the barrier it closes.
//!
//! ⭐ **the cut was clean, and it was MEASURED before it was made**: the moved
//! region held zero `crate::` reach-ins, zero `super::` uses, zero bevy, and
//! five import lines all naming crates `ambition_characters` already had.
//!
//! ⛔⛔ **and the thing that made it hard was not the imports — it was the
//! PRIVACY.** `prepare_character` and `finalize_character` were private module
//! functions, and that privacy IS the finalization barrier: it is what made an
//! early fold unreachable. Splitting the module would have meant publishing
//! them, putting the ordering hazard `CharacterPreparationPlugin::finish` exists
//! to remove back on the production surface. So the barrier became a TYPE
//! instead — `prepared::StagedCharacter` can only be minted by
//! `prepare_for_registration` and only consumed by `finalize_cast`, the
//! `Bound<N>` pattern this repository already runs. Folding early is not
//! prevented now; it is unspellable, and that survives a crate boundary where
//! privacy does not.
//!
//! [`super::CharacterLoadStates`] is where the art half reports.

// ⭐ **every moved name is re-exported**, so `character_runtime::{..}` paths
// across the workspace are unchanged by the relocation. The module a reader
// should EDIT is `ambition_characters::prepared`; this list is the door.
pub use ambition_characters::binding_namespaces::{
    MoveId, PortraitTarget, RangedPayload, SfxCueId, SheetTarget, VerbId, VfxTag,
};
pub use ambition_characters::prepared::{
    CharacterBindings, CharacterBodyBlueprint, CharacterCatalogGeneration,
    CharacterPreparationPlugin, CharacterRegistrationError, MissingCharacterFacts,
    PreparedCharacterDefinition, PreparedCharacterRegistry, PreparedKit,
};

use ambition_characters::actor::definition::CharacterDefinition;

// The barrier-bypassing fixture seams, re-exported so `definition_tests.rs` —
// which is `#[path]`-included as a CHILD of this module and reaches them through
// `use super::*` — is unchanged by the relocation. Behind `ambition_characters`'s
// `test-support` feature, which this crate enables as a DEV-dependency only.
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use ambition_characters::prepared::{
    prepare_and_finalize_against_for_test, prepare_and_finalize_for_test, FinalizedCharacter,
};

/// **The single registration seam.** (§4.1)
///
/// Prepares the definition and publishes it into the prepared authority. A
/// provider makes ONE call and does not have to know that sheets, cues, and
/// gameplay numbers are consumed by different subsystems.
///
/// # Registration is DECLARATIVE — it does not load anything
///
/// This used to end by calling `CharacterLoadDemand::request`, on the reasoning
/// that a provider should not need a second call. That reasoning was wrong, and
/// the mistake gets worse the better the plan works: as more characters migrate
/// onto this seam, merely *installing* a provider's plugin would demand every one
/// of its characters' art. That is precisely the startup decode storm §7.1
/// deleted — four privileged ids decoding at boot because someone decided in
/// advance which characters mattered — rebuilt from the other end.
///
/// Loading is driven by a PROJECTION of what a session actually stages: a room
/// plan, a match roster, a startup spec, or a body putting on an identity
/// (`StagesCharacters`). Registration says *what exists*; staging says *what is
/// needed now*. A game with fifty registered fighters and two on screen decodes
/// two sheets.
///
/// The binding report is logged rather than returned as an error: see
/// [`prepare_character`] for why an unresolved reference degrades loudly instead
/// of refusing.
/// **Fill in the engine's baked sheet + portrait vocabularies unless the caller
/// supplied them** — the registration seam's job, and it is a FREE FUNCTION for
/// a reason.
///
/// ⛔ these were inherent methods on `CharacterBindings`
/// (`with_engine_{sheet,portrait}_vocabulary`). The type is moving down into
/// `ambition_characters`, and the vocabularies come from
/// `ambition_sprite_sheet`, which DEPENDS ON `ambition_characters` — so keeping
/// them inherent would be a cycle the compiler finds at the worst moment. The
/// orphan rule adjudicating placement (P1.7 sub-case (a)).
///
/// ⭐ **and it belongs here anyway**, which is what makes this a repair rather
/// than a workaround. The original doc said so: *"kept OUT of
/// `prepare_character`, which stays a pure function of its arguments — reaching
/// into a baked global from inside preparation would make the same definition
/// prepare differently depending on the build. This is the registration seam's
/// job, because registration is where the engine is."*
///
/// ⛔⛔ **portrait targets were checked NOWHERE until the portrait half existed**
/// (ledger D106): `with_available_portraits` was the only road to the resolver
/// and nothing called it, so `portraits` was `None` in every composition,
/// `PortraitTarget::NAME` never joined a prepared character's `checked` list,
/// and preparation reported *"we did not look"* — permanently, correctly, and
/// therefore invisibly. Both halves apply at the ONE seam every registration
/// passes through, rather than at the three call sites, so the next provider
/// cannot forget one.
pub fn with_engine_vocabularies(mut bindings: CharacterBindings) -> CharacterBindings {
    if !bindings.has_sheet_vocabulary() {
        bindings = bindings
            .with_available_sheets(ambition_sprite_sheet::character::sheets::available_targets());
    }
    if !bindings.has_portrait_vocabulary() {
        bindings =
            bindings.with_available_portraits(ambition_sprite_sheet::available_portrait_targets());
    }
    bindings
}

pub trait CharacterDefinitionAppExt {
    fn try_register_character(
        &mut self,
        definition: CharacterDefinition,
        bindings: CharacterBindings,
    ) -> Result<&mut Self, CharacterRegistrationError>;

    fn register_character(&mut self, definition: CharacterDefinition) -> &mut Self {
        self.try_register_character(definition, CharacterBindings::default())
            .unwrap_or_else(|error| panic!("{error}"))
    }
}

impl CharacterDefinitionAppExt for bevy::prelude::App {
    fn try_register_character(
        &mut self,
        definition: CharacterDefinition,
        bindings: CharacterBindings,
    ) -> Result<&mut Self, CharacterRegistrationError> {
        // The engine supplies its OWN vocabulary. A sheet target resolves against
        // the baked manifest index, which the engine always knows, so every
        // registration gets its sheet reference checked with a did-you-mean whether
        // or not the provider thought to pass a resolver. A boundary that only
        // works when the caller opts in is a boundary most callers will not have.
        // ⭐ **BOTH vocabularies, at the ONE seam every registration passes
        // through.** Sheets have been checked here since the boundary landed;
        // portraits were not checked anywhere at all (D106), and adding it here
        // rather than at the three call sites is what stops the next provider
        // forgetting one of them.
        //
        // ⭐⭐ **and enriching the BINDINGS is the whole of what this layer does
        // now.** Staging, the barrier and the fold went down to
        // `ambition_characters::prepared` (2026-08-12): while they were up here,
        // the low crate had to publish a public mint and a public consumer for
        // this function to call, and two public ends make a public fold no matter
        // how opaque the value between them. What crosses the boundary is a
        // CONTRIBUTION, and what this crate contributes on top of the provider's
        // is the engine's own art vocabulary — which is the one fact
        // `ambition_characters` must not know, because `ambition_sprite_sheet`
        // depends on it.
        let bindings = with_engine_vocabularies(bindings);
        ambition_characters::prepared::stage_authored_character(self, definition, &bindings)?;
        Ok(self)
    }
}

// A CHILD of the preparation module, not a sibling. Its subject is the partial
// phase — `PreparedCharacterOverrides`, the fold, the barrier — and a sibling
// could not name any of them. Making the tests reach the same way runtime does
// would have meant widening the visibility that IS the design.
#[cfg(test)]
#[path = "definition_tests.rs"]
mod definition_tests;
