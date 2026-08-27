//! Bevy `App` integration for character registration and finalization.
//!
//! Preparation lives in `ambition_characters::prepared`; this module owns the
//! application registration seam and the typed finalization barrier.
//! [`super::CharacterLoadStates`] reports the asset-loading side.


use ambition_characters::prepared::{CharacterBindings, CharacterRegistrationError};
use ambition_characters::actor::definition::CharacterDefinition;

// Test-only preparation seams used by the child `definition_tests` module.
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use ambition_characters::prepared::{
    prepare_and_finalize_against_for_test, prepare_and_finalize_for_test, FinalizedCharacter,
};

/// Fill missing engine sheet and portrait vocabularies at the registration seam.
///
/// Preparation remains a pure function of its inputs. This function lives above
/// `ambition_characters` because the concrete vocabularies come from
/// `ambition_sprite_sheet`, which already depends on `ambition_characters`.
pub fn with_engine_vocabularies(mut bindings: CharacterBindings) -> CharacterBindings {
    if !bindings.has_sheet_vocabulary() {
        bindings = bindings
            .with_available_sheets(ambition_sprite_sheet::character::sheets::available_sheet_keys());
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
        // The engine supplies its OWN vocabulary. A sheet target resolves against the baked
        // manifest index, which the engine always knows, so every registration gets its sheet
        // reference checked with a did-you-mean whether or not the provider thought to pass a
        // resolver.
        //
        // and enriching the BINDINGS is the whole of what this layer does
        // now. Staging, the barrier and the fold went down to
        // `ambition_characters:prepared`: while they were up here,
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
