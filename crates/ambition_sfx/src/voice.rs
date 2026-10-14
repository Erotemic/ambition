//! Which voice a speaker speaks with: the dialogue blip a speaker label or a
//! dialogue id resolves to.
//!
//! A game states its cast's voiceprints (as content: the `dialogue_voiceprints`
//! schema of `ambition_audio`); the dialogue runtime asks the catalog for the
//! cue of each line and falls back to the generic blip. The catalog is here,
//! beside [`SfxId`], so that the content compiler can check a voiceprint file
//! with the same rules the runtime resolves it by, and link no dialogue UI.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::SfxId;

/// App-local, provider-authored dialogue voice mapping.
///
/// Exact keys are checked before substring aliases. The speaker label is tried
/// first, then the dialogue id, preserving the runtime behavior for lines whose
/// displayed speaker differs from the node that produced them.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
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
            // Alias precedence is provider-authored registration order.
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

impl DialogueVoiceCatalog {
    /// Each exact key the catalog holds, normalized, with its cue, in key
    /// order.
    pub fn exact_keys(&self) -> impl Iterator<Item = (&str, SfxId)> {
        self.exact.iter().map(|(key, cue)| (key.as_str(), *cue))
    }
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
        catalog.register_voiceprint(ALPHA, &[], &["bob"]).unwrap();
        catalog.register_voiceprint(BETA, &[], &["tech"]).unwrap();

        assert_eq!(catalog.resolve("Bob Tech", ""), Some(ALPHA));
    }
}
