//! Dialogue typewriter SFX selection and throttling.
//!
//! The reusable dialogue runtime owns reveal cadence and generic fallback cues.
//! A game provider contributes speaker identities through
//! [`DialogueVoiceCatalog`], so this crate never names a particular cast.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use ambition_sfx::{ids, SfxId};
use bevy::prelude::{App, Resource};

use crate::runtime::DialogSpeechStyle;

/// How many newly-visible alphanumeric characters pass between normal
/// typewriter blips. Spaces and punctuation act like visual pauses and do not
/// advance the counter.
const TALK_BLIP_CHAR_INTERVAL: usize = 5;

/// App-local, provider-authored dialogue voice mapping.
///
/// Exact keys are checked before substring aliases. The speaker label is tried
/// first, then the dialogue id, preserving the runtime behavior for lines whose
/// displayed speaker differs from the node that produced them.
#[derive(Resource, Clone, Debug, Default)]
pub struct DialogueVoiceCatalog {
    exact: BTreeMap<String, SfxId>,
    aliases: Vec<(String, SfxId)>,
}

impl DialogueVoiceCatalog {
    /// Register one provider-owned normal-speech voiceprint.
    ///
    /// Re-registering an identical key/cue pair is idempotent. Claiming the
    /// same normalized key for a different cue is rejected transactionally.
    pub fn register_voiceprint(
        &mut self,
        cue: SfxId,
        exact_keys: &[&str],
        aliases: &[&str],
    ) -> Result<(), DialogueVoiceCatalogError> {
        let exact_keys = normalize_registration_keys(exact_keys)?;
        let aliases = normalize_registration_keys(aliases)?;

        for key in &exact_keys {
            let alias_cue = self
                .aliases
                .iter()
                .find_map(|(alias, cue)| (alias == key).then_some(*cue));
            if let Some(existing) = self.exact.get(key).copied().or(alias_cue) {
                if existing != cue {
                    return Err(DialogueVoiceCatalogError::ConflictingExactKey {
                        key: key.clone(),
                        existing,
                        requested: cue,
                    });
                }
            }
        }
        for alias in &aliases {
            let registered_alias = self
                .aliases
                .iter()
                .find_map(|(existing, cue)| (existing == alias).then_some(*cue));
            if let Some(existing) = registered_alias.or_else(|| self.exact.get(alias).copied()) {
                if existing != cue {
                    return Err(DialogueVoiceCatalogError::ConflictingAlias {
                        alias: alias.clone(),
                        existing,
                        requested: cue,
                    });
                }
            }
        }

        for key in exact_keys {
            self.exact.entry(key).or_insert(cue);
        }
        for alias in aliases {
            if !self.aliases.iter().any(|(existing, _)| existing == &alias) {
                self.aliases.push((alias, cue));
            }
        }
        Ok(())
    }

    /// Resolve a provider-authored normal-speech cue without generic fallback.
    pub fn resolve(&self, speaker_label: &str, dialogue_id: &str) -> Option<SfxId> {
        let speaker = normalized_key(speaker_label);
        let dialogue = normalized_key(dialogue_id);

        self.resolve_key(&speaker)
            .or_else(|| self.resolve_key(&dialogue))
    }

    fn resolve_key(&self, key: &str) -> Option<SfxId> {
        if key.is_empty() {
            return None;
        }
        self.exact.get(key).copied().or_else(|| {
            // Alias precedence is provider-authored registration order. This
            // preserves the old fixed matcher order while keeping the runtime
            // open to additional providers.
            self.aliases
                .iter()
                .find_map(|(alias, cue)| key.contains(alias.as_str()).then_some(*cue))
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DialogueVoiceCatalogError {
    EmptyKey {
        raw: String,
    },
    ConflictingExactKey {
        key: String,
        existing: SfxId,
        requested: SfxId,
    },
    ConflictingAlias {
        alias: String,
        existing: SfxId,
        requested: SfxId,
    },
}

impl fmt::Display for DialogueVoiceCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyKey { raw } => write!(f, "dialogue voice key normalizes to empty: {raw:?}"),
            Self::ConflictingExactKey {
                key,
                existing,
                requested,
            } => write!(
                f,
                "dialogue voice exact key {key:?} already maps to {existing}, not {requested}",
            ),
            Self::ConflictingAlias {
                alias,
                existing,
                requested,
            } => write!(
                f,
                "dialogue voice alias {alias:?} already maps to {existing}, not {requested}",
            ),
        }
    }
}

impl Error for DialogueVoiceCatalogError {}

/// Composition-time registration sugar for provider-owned dialogue voices.
pub trait DialogueVoiceCatalogAppExt {
    fn register_dialogue_voiceprint(
        &mut self,
        cue: SfxId,
        exact_keys: &[&str],
        aliases: &[&str],
    ) -> &mut Self;
}

impl DialogueVoiceCatalogAppExt for App {
    fn register_dialogue_voiceprint(
        &mut self,
        cue: SfxId,
        exact_keys: &[&str],
        aliases: &[&str],
    ) -> &mut Self {
        self.init_resource::<DialogueVoiceCatalog>();
        self.world_mut()
            .resource_mut::<DialogueVoiceCatalog>()
            .register_voiceprint(cue, exact_keys, aliases)
            .unwrap_or_else(|error| panic!("invalid dialogue voice registration: {error}"));
        self
    }
}

/// Return true when a reveal tick should emit one dialogue blip.
///
/// `previous_visible_chars` and `visible_chars` are character counts into
/// `line`, not byte offsets. If a low framerate tick reveals many characters at
/// once, this still emits at most one blip for the frame.
pub(crate) fn should_play_talk_blip(
    line: &str,
    previous_visible_chars: usize,
    visible_chars: usize,
) -> bool {
    if visible_chars <= previous_visible_chars {
        return false;
    }
    let previous_voice_chars = voiced_char_count(line, previous_visible_chars);
    let visible_voice_chars = voiced_char_count(line, visible_chars);
    if visible_voice_chars <= previous_voice_chars {
        return false;
    }
    next_interval_index(previous_voice_chars) < next_interval_index(visible_voice_chars)
}

/// Resolve the talk-blip cue for a line.
///
/// Normal speech consults provider content and falls back to the generic blip.
/// Whisper and shout remain generic until a provider actually authors styled
/// voiceprints.
pub(crate) fn talk_blip_id_for_speaker(
    catalog: Option<&DialogueVoiceCatalog>,
    speaker_label: &str,
    dialogue_id: &str,
    style: DialogSpeechStyle,
) -> SfxId {
    match style {
        DialogSpeechStyle::Normal => catalog
            .and_then(|catalog| catalog.resolve(speaker_label, dialogue_id))
            .unwrap_or(ids::DIALOGUE_BLIP_GENERIC),
        DialogSpeechStyle::Whisper => ids::DIALOGUE_BLIP_WHISPER_GENERIC,
        DialogSpeechStyle::Shout => ids::DIALOGUE_BLIP_SHOUT_GENERIC,
    }
}

fn next_interval_index(count: usize) -> usize {
    count / TALK_BLIP_CHAR_INTERVAL
}

fn voiced_char_count(line: &str, visible_chars: usize) -> usize {
    line.chars()
        .take(visible_chars)
        .filter(|ch| ch.is_alphanumeric())
        .count()
}

fn normalize_registration_keys(
    raw_keys: &[&str],
) -> Result<Vec<String>, DialogueVoiceCatalogError> {
    let mut normalized = Vec::with_capacity(raw_keys.len());
    for raw in raw_keys {
        let key = normalized_key(raw);
        if key.is_empty() {
            return Err(DialogueVoiceCatalogError::EmptyKey {
                raw: (*raw).to_string(),
            });
        }
        if !normalized.contains(&key) {
            normalized.push(key);
        }
    }
    Ok(normalized)
}

fn normalized_key(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut last_was_sep = true;
    for ch in raw.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('_');
            last_was_sep = true;
        }
    }
    if out.ends_with('_') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALPHA: SfxId = SfxId::from_static("test.dialogue.alpha");
    const BETA: SfxId = SfxId::from_static("test.dialogue.beta");

    #[test]
    fn blip_throttle_ignores_spaces_and_punctuation() {
        assert!(!should_play_talk_blip("Hi, Sam!", 0, 4));
        assert!(should_play_talk_blip("Hello, Sam!", 0, 7));
        assert!(!should_play_talk_blip("Hello, Sam!", 7, 8));
    }

    #[test]
    fn provider_catalog_resolves_exact_alias_and_dialogue_fallback() {
        let mut catalog = DialogueVoiceCatalog::default();
        catalog
            .register_voiceprint(ALPHA, &["Speaker Alpha", "alpha_node"], &["alpha"])
            .unwrap();

        assert_eq!(catalog.resolve("Speaker Alpha", ""), Some(ALPHA));
        assert_eq!(catalog.resolve("The Alpha Captain", ""), Some(ALPHA));
        assert_eq!(catalog.resolve("", "alpha_node"), Some(ALPHA));
    }

    #[test]
    fn conflicting_registration_is_rejected_but_identical_is_idempotent() {
        let mut catalog = DialogueVoiceCatalog::default();
        catalog
            .register_voiceprint(ALPHA, &["alpha"], &["captain"])
            .unwrap();
        catalog
            .register_voiceprint(ALPHA, &["alpha"], &["captain"])
            .unwrap();

        assert!(matches!(
            catalog.register_voiceprint(BETA, &["alpha"], &[]),
            Err(DialogueVoiceCatalogError::ConflictingExactKey { .. })
        ));
        assert!(matches!(
            catalog.register_voiceprint(BETA, &[], &["captain"]),
            Err(DialogueVoiceCatalogError::ConflictingAlias { .. })
        ));
        assert!(matches!(
            catalog.register_voiceprint(BETA, &["captain"], &[]),
            Err(DialogueVoiceCatalogError::ConflictingExactKey { .. })
        ));
        assert!(matches!(
            catalog.register_voiceprint(BETA, &[], &["alpha"]),
            Err(DialogueVoiceCatalogError::ConflictingAlias { .. })
        ));
    }

    #[test]
    fn alias_precedence_follows_provider_registration_order() {
        let mut catalog = DialogueVoiceCatalog::default();
        catalog
            .register_voiceprint(ALPHA, &[], &["bob"])
            .unwrap();
        catalog
            .register_voiceprint(BETA, &[], &["tech"])
            .unwrap();

        assert_eq!(catalog.resolve("Bob Tech", ""), Some(ALPHA));
    }

    #[test]
    fn unknown_and_styled_speech_use_generic_variants() {
        let mut catalog = DialogueVoiceCatalog::default();
        catalog
            .register_voiceprint(ALPHA, &["alpha"], &[])
            .unwrap();

        assert_eq!(
            talk_blip_id_for_speaker(
                Some(&catalog),
                "Mystery",
                "",
                DialogSpeechStyle::Normal,
            ),
            ids::DIALOGUE_BLIP_GENERIC,
        );
        assert_eq!(
            talk_blip_id_for_speaker(
                Some(&catalog),
                "alpha",
                "",
                DialogSpeechStyle::Whisper,
            ),
            ids::DIALOGUE_BLIP_WHISPER_GENERIC,
        );
        assert_eq!(
            talk_blip_id_for_speaker(None, "alpha", "", DialogSpeechStyle::Shout),
            ids::DIALOGUE_BLIP_SHOUT_GENERIC,
        );
    }
}
