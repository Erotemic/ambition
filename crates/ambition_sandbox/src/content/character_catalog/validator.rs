//! Load-time validator. Walks every catalog entry and confirms its
//! `default_brain` / `default_action_set` references resolve to
//! presets in the catalog. Returns a list of human-readable errors.
//!
//! Wired by [`super::CharacterCatalogPlugin`] as a Startup system
//! that panics with a single message listing every issue at once.
//! Pre-release stance: fail loud, fail early — better than a silent
//! mismatch that surfaces hours later as a spawn-time panic.

use super::entry::CharacterCatalogData;

/// Walk the catalog and collect every reference error. An empty
/// return means the catalog is internally consistent.
pub fn validate(catalog: &CharacterCatalogData) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();

    for (id, entry) in &catalog.characters {
        if entry.display_name.trim().is_empty() {
            errors.push(format!("character '{id}' has empty display_name"));
        }
        if entry.spritesheet.trim().is_empty() {
            errors.push(format!("character '{id}' has empty spritesheet path"));
        }
        if entry.manifest.trim().is_empty() {
            errors.push(format!("character '{id}' has empty manifest path"));
        }
        if !catalog.brain_presets.contains_key(&entry.default_brain) {
            errors.push(format!(
                "character '{id}' default_brain '{}' not found in brain_presets",
                entry.default_brain
            ));
        }
        if !catalog
            .action_set_presets
            .contains_key(&entry.default_action_set)
        {
            errors.push(format!(
                "character '{id}' default_action_set '{}' not found in action_set_presets",
                entry.default_action_set
            ));
        }
    }

    errors
}
