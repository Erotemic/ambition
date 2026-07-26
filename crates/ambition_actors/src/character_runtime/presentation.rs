//! **A staged cast authorizes its own presentation sources.** (§4.5, §4.7)
//!
//! §7.7 gave the engine the vocabulary for several providers emitting cues in one
//! session: a request carries a [`PresentationSourceId`], and
//! `ActiveAudioSelection::authorize_sfx_source` binds that source to a provider's
//! cue registry and bank allowlist. So the same logical cue id — `Dash` — can
//! resolve to two genuinely different sounds for two characters in one fight.
//!
//! Nothing in production called it. The only non-unit-test caller was a rendered
//! test, which means the vocabulary existed and the sentence was never spoken: a
//! secondary provider's correctly-tagged cue reached the audio authority, found no
//! authorization for its source, and was **denied**. A cue that is silently
//! dropped is worse than one that plays the wrong sound, because nothing reports
//! it — the request was well-formed and the refusal was invisible.
//!
//! This module is that sentence. Seating a cast is what authorizes its sources.

use bevy::prelude::*;
use std::collections::BTreeSet;

use ambition_characters::actor::character_catalog::CharacterCatalogOwners;

use super::{CharacterLoadStates, PreparedCharacterRegistry};

/// The provider that authored a character, from either declaration source.
///
/// The prepared registry is consulted FIRST and the assembled catalog second, for
/// the same reason `sheet_for_declared_character` prefers the registered sheet: a
/// character may exist only on the registration seam, and then the registry is the
/// only place its provider is written down.
pub fn provider_of_character<'a>(
    registry: Option<&'a PreparedCharacterRegistry>,
    owners: Option<&'a CharacterCatalogOwners>,
    character_id: &str,
) -> Option<&'a str> {
    registry
        .and_then(|registry| registry.get(character_id))
        .map(|prepared| prepared.provider.as_str())
        .or_else(|| owners.and_then(|owners| owners.provider_for(character_id)))
}

/// Authorize a presentation source for every provider in the staged cast.
///
/// Runs off [`CharacterLoadStates`] — the set of characters a session actually
/// staged — rather than off everything registered, because authorization is a
/// property of THIS session's cast. A game with fifty registered fighters and two
/// on stage authorizes two providers.
///
/// The source id is the provider id, which is exactly what `advance_move_playback`
/// stamps onto a `MoveEventMessage` when it resolves a body's character to its
/// author. Those two must agree or the tag is unauthorized; deriving both from
/// `provider_of_character` is what keeps them agreeing.
///
/// Idempotent by construction: `authorize_sfx_source` treats a repeat with an
/// identical definition as a no-op (and panics on a repeat with a DIFFERENT one,
/// which is the right behaviour — two disagreeing definitions of one source is a
/// composition bug, not a runtime condition to paper over).
/// Not gated on the `audio` feature: `ambition_audio` is an unconditional
/// dependency (only the Kira playback backend is optional), and gating the
/// AUTHORIZATION would mean a headless or art-free build silently denies cues it
/// would have allowed — a composition difference that changes behaviour, which is
/// the defect class this whole module exists to close.
pub fn authorize_staged_character_presentation_sources(
    states: Option<Res<CharacterLoadStates>>,
    registry: Option<Res<PreparedCharacterRegistry>>,
    owners: Option<Res<CharacterCatalogOwners>>,
    audio_catalog: Option<Res<ambition_audio::catalog::AudioCatalogRegistry>>,
    bank_ids: Option<Res<ambition_audio::catalog::SfxBankRegistry>>,
    selection: Option<ResMut<ambition_audio::selection::ActiveAudioSelection>>,
) {
    let (Some(states), Some(mut selection)) = (states, selection) else {
        return;
    };
    // No session owns the speakers yet (a frontend route, or startup before the
    // gameplay session commits). Authorizing into nothing is a silent no-op inside
    // `authorize_sfx_source` anyway; returning here says so out loud.
    if selection.current().is_none() {
        return;
    }
    let mut authorized: BTreeSet<String> = BTreeSet::new();
    for character_id in states.staged_characters() {
        let Some(provider) = provider_of_character(
            registry.as_deref(),
            owners.as_deref(),
            character_id,
        ) else {
            // No declaration names an author. The load ledger already reports
            // unknown characters; this is not a second place to complain about it.
            continue;
        };
        if !authorized.insert(provider.to_string()) {
            continue;
        }
        let sfx = audio_catalog
            .as_deref()
            .and_then(|catalog| catalog.sfx_for(provider))
            .cloned();
        let ids = bank_ids
            .as_deref()
            .map(|banks| banks.ids_for(provider))
            .unwrap_or_default();
        selection.authorize_sfx_source(provider.to_string(), provider.to_string(), sfx, ids);
    }
}

#[cfg(test)]
mod tests;
