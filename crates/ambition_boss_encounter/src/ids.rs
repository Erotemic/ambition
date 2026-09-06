//! Boss encounter id helper: `encounter_id_from_name` slugs an authored boss
//! name into a stable id (`"Clockwork Warden"` -> `"clockwork_warden"`). The
//! engine names no boss — every boss's chest reward is authored data
//! (`BossRewardProfile::DropChest` in `boss_profiles.ron`), resolved through
//! the generic `encounter_chest_<id>` naming.

/// Sanitize an authored boss `name` into a stable encounter id. Lowercases,
/// strips non-alphanumeric characters, replaces spaces with underscores.
/// `"Clockwork Warden"` → `"clockwork_warden"`.
pub fn encounter_id_from_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_was_underscore = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_was_underscore = false;
        } else if !prev_was_underscore && !out.is_empty() {
            out.push('_');
            prev_was_underscore = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "boss".to_string()
    } else {
        out
    }
}

/// Encounter ids that were RENAMED, and what they became.
///
/// ⛔⛔ THIS FACT WAS WRITTEN TWICE. `gradient_sentinel -> clockwork_warden`
/// lived as a hardcoded arm in BOTH `profile.rs::for_encounter_id_or_name` and
/// `behavior.rs::for_authored_boss`, in two crates' worth of reading apart, and
/// nothing made them agree. A second rename would have had to be remembered in
/// both — and the failure of remembering is silent: the profile resolves and the
/// BEHAVIOUR falls through to `generic`, so an old save loads a boss that looks
/// right and fights like nothing in particular.
///
/// ⚠ NOT folded into [`encounter_id_from_name`], which is a pure SLUGGER
/// (`"Clockwork Warden"` -> `"clockwork_warden"`). Alias resolution is a
/// different job, and a caller that wants the literal slug — a sprite-sheet key,
/// say — must not have its id silently rewritten underneath it.
const RENAMED_ENCOUNTER_IDS: &[(&str, &str)] = &[("gradient_sentinel", "clockwork_warden")];

/// What a retired encounter id became, if it is one.
///
/// The one reading of the rename. Callers keep their own fallback SHAPE — the
/// profile road tries it after a miss, the behaviour road takes it before
/// consulting the catalog — because those are different control flow, but
/// neither spells the pair itself.
pub fn renamed_encounter_id(id: &str) -> Option<&'static str> {
    RENAMED_ENCOUNTER_IDS
        .iter()
        .find(|(retired, _)| *retired == id)
        .map(|(_, current)| *current)
}

#[cfg(test)]
mod renamed_id_tests {
    use super::*;

    #[test]
    fn a_retired_id_resolves_and_an_unknown_one_does_not() {
        assert_eq!(renamed_encounter_id("gradient_sentinel"), Some("clockwork_warden"));
        assert_eq!(renamed_encounter_id("clockwork_warden"), None);
        assert_eq!(renamed_encounter_id("cove_mockingbird"), None);
    }

    /// ⛔ A RENAME MUST NOT POINT AT ANOTHER RENAME, or the one lookup every
    /// caller makes resolves to an id that is itself retired — and no caller
    /// loops, so the chain would silently stop one hop short.
    #[test]
    fn no_rename_targets_an_id_that_is_itself_retired() {
        for (retired, current) in RENAMED_ENCOUNTER_IDS {
            assert_eq!(
                renamed_encounter_id(current),
                None,
                "`{retired}` renames to `{current}`, which is ITSELF retired; \
                 callers resolve one hop only"
            );
            assert_ne!(retired, current, "`{retired}` renames to itself");
        }
    }
}
