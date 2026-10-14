//! The `dialogue_voiceprints` schema: which voice each speaker of a game's
//! cast speaks with, as data in its pack.
//!
//! Each row names a dialogue blip cue and the speaker labels (`exact`) and the
//! parts of labels (`aliases`) that resolve to it. Admission builds the
//! [`DialogueVoiceCatalog`] the runtime resolves by, so a key claimed by two
//! cues is refused, and so is an exact key that hides another cue's alias
//! (every speaker label shaped like it, but not declared, would take the
//! wrong voice).

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use ambition_sfx::voice::DialogueVoiceCatalog;
use ambition_sfx::SfxId;
use serde::Deserialize;

/// The authored FILE kind: a game's cast voiceprints.
pub const DIALOGUE_VOICEPRINTS_SCHEMA: &str = "dialogue_voiceprints";

/// One voiceprint, as a pack writes it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredVoiceprint {
    /// The blip cue, as the SFX registry names it (`dialogue.blip.alice`).
    pub cue: String,
    /// Speaker labels and dialogue ids that resolve to the cue. Checked before
    /// any alias.
    #[serde(default)]
    pub exact: Vec<String>,
    /// A label that contains one of these resolves to the cue. The first row
    /// whose alias matches wins, so the order of the rows is the precedence.
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// The catalog the rows state, or the first registration the catalog refuses.
fn catalog_of(rows: &[AuthoredVoiceprint], with_exact: bool) -> Result<DialogueVoiceCatalog, String> {
    let mut catalog = DialogueVoiceCatalog::default();
    for row in rows {
        let exact: Vec<&str> = if with_exact { row.exact.iter().map(String::as_str).collect() } else { Vec::new() };
        let aliases: Vec<&str> = row.aliases.iter().map(String::as_str).collect();
        catalog
            .register_voiceprint(SfxId::new(&row.cue), &exact, &aliases)
            .map_err(|error| format!("the voiceprint of `{}`: {error}", row.cue))?;
    }
    Ok(catalog)
}

/// Each row the runtime cannot use, and each exact key that hides an alias.
fn problems(rows: &Vec<AuthoredVoiceprint>) -> Vec<String> {
    let mut problems: Vec<String> = rows
        .iter()
        .filter(|row| row.cue.trim().is_empty())
        .map(|_| "a voiceprint has an empty `cue`".to_string())
        .collect();
    let catalog = match catalog_of(rows, true) {
        Ok(catalog) => catalog,
        Err(problem) => {
            problems.push(problem);
            return problems;
        }
    };
    // Exact keys are consulted before aliases, so an exact key can hide an
    // alias of another cue that matches it. The authored label still resolves;
    // every label shaped like it and not declared takes the other voice. So ask
    // each exact key again with the exact keys removed.
    let alias_only = catalog_of(rows, false).unwrap_or_default();
    // The cue as the file names it: an id is a hash.
    let name = |cue: SfxId| {
        rows.iter()
            .find(|row| SfxId::new(&row.cue) == cue)
            .map_or_else(|| cue.to_string(), |row| format!("`{}`", row.cue))
    };
    for (key, cue) in catalog.exact_keys() {
        if let Some(other) = alias_only.resolve(key, "").filter(|other| *other != cue) {
            let (cue, other) = (name(cue), name(other));
            problems.push(format!(
                "the exact key `{key}` is the only thing that gives it the voice {cue}: an alias of {other} \
                 matches it, so a label shaped like it and not declared takes {other}. Make that alias longer"
            ));
        }
    }
    problems
}

/// The schema. Owned by the audio capability, as the SFX registry is.
pub fn dialogue_voiceprints_schema() -> SchemaRegistration {
    rules_schema::<Vec<AuthoredVoiceprint>>(
        DIALOGUE_VOICEPRINTS_SCHEMA,
        1,
        super::AUDIO_CAPABILITY,
        "A game's cast voiceprints: the dialogue blip cue each speaker label and \
         dialogue id resolves to, exact keys before aliases.",
        problems,
    )
}

/// The voice catalog the pack states, or `None` when it has no
/// `dialogue_voiceprints` source.
pub fn lowered_voice_catalog(pack: &PreparedContentPack) -> Option<DialogueVoiceCatalog> {
    let rows = lowered_rules::<Vec<AuthoredVoiceprint>>(pack, DIALOGUE_VOICEPRINTS_SCHEMA)?;
    // Admission built this catalog from these rows and refused the pack if it
    // could not, so it builds here.
    Some(catalog_of(rows, true).expect("an admitted voiceprint file builds its catalog"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cue: &str, exact: &[&str], aliases: &[&str]) -> AuthoredVoiceprint {
        AuthoredVoiceprint {
            cue: cue.into(),
            exact: exact.iter().map(|key| key.to_string()).collect(),
            aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
        }
    }

    #[test]
    fn a_voiceprint_file_is_refused_for_a_shared_key_and_for_a_hidden_alias() {
        assert!(problems(&vec![row("blip.bob", &["bob"], &["bob"])]).is_empty());

        let shared = problems(&vec![row("blip.bob", &["bob"], &[]), row("blip.robot", &["bob"], &[])]);
        assert!(shared.iter().any(|problem| problem.contains("`blip.robot`")), "{shared:?}");

        // `goblin` is an alias of the chieftain; `goblin_merchant` resolves to
        // the merchant only because its exact key says so.
        let hidden = problems(&vec![
            row("blip.chieftain", &[], &["goblin"]),
            row("blip.merchant", &["goblin_merchant"], &["merchant"]),
        ]);
        assert!(hidden.iter().any(|problem| problem.contains("`goblin_merchant`")), "{hidden:?}");
    }
}
