//! Load-time validator. Walks every catalog entry and confirms its
//! `default_brain` / `default_action_set` references resolve to
//! presets in the catalog. Returns a list of human-readable errors.
//!
//! Wired by [`super::CharacterCatalogPlugin`] as a Startup system
//! that panics with a single message listing every issue at once.
//! Pre-release stance: fail loud, fail early — better than a silent
//! mismatch that surfaces hours later as a spawn-time panic.

use std::collections::BTreeMap;

use super::entry::CharacterCatalogData;

/// Walk the catalog and collect every reference error. An empty
/// return means the catalog is internally consistent.
pub fn validate(catalog: &CharacterCatalogData) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    let mut display_name_owners: BTreeMap<&str, &str> = BTreeMap::new();

    for (id, entry) in &catalog.characters {
        if entry.display_name.trim().is_empty() {
            errors.push(format!("character '{id}' has empty display_name"));
        } else if let Some(first_id) =
            display_name_owners.insert(entry.display_name.as_str(), id.as_str())
        {
            errors.push(format!(
                "characters '{first_id}' and '{id}' share display_name '{}'",
                entry.display_name
            ));
        }
        if entry.spritesheet.trim().is_empty() {
            errors.push(format!("character '{id}' has empty spritesheet path"));
        }
        if entry.manifest.trim().is_empty() {
            errors.push(format!("character '{id}' has empty manifest path"));
        }
        if let Some(portrait) = &entry.portrait {
            if portrait.image.trim().is_empty() {
                errors.push(format!("character '{id}' has empty portrait image path"));
            }
            if portrait.manifest.trim().is_empty() {
                errors.push(format!("character '{id}' has empty portrait manifest path"));
            }
            if portrait.default_clip.trim().is_empty() {
                errors.push(format!("character '{id}' has empty portrait default_clip"));
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::character_catalog::parse_catalog;

    #[test]
    fn incomplete_portrait_references_are_rejected() {
        let catalog = parse_catalog(
            r#"(
                brain_presets: { "idle": StandStill },
                action_set_presets: { "peaceful": (move_style: Walk) },
                characters: {
                    "alpha": (
                        display_name: "Alpha",
                        spritesheet: "alpha.png",
                        manifest: "alpha.ron",
                        portrait: Some((
                            image: "",
                            manifest: "alpha_portraits.ron",
                            default_clip: "default",
                        )),
                        tier: MainHall,
                        body_kind: Standard,
                        composition: None,
                        default_brain: "idle",
                        default_action_set: "peaceful",
                        tags: [],
                    ),
                },
            )"#,
        );
        assert_eq!(
            validate(&catalog),
            vec!["character 'alpha' has empty portrait image path".to_string()]
        );
    }

    #[test]
    fn duplicate_display_names_are_rejected_deterministically() {
        let catalog = parse_catalog(
            r#"(
                brain_presets: { "idle": StandStill },
                action_set_presets: { "peaceful": (move_style: Walk) },
                characters: {
                    "alpha": (
                        display_name: "Shared", spritesheet: "alpha.png", manifest: "alpha.ron",
                        tier: MainHall, body_kind: Standard, composition: None,
                        default_brain: "idle", default_action_set: "peaceful", tags: [],
                    ),
                    "beta": (
                        display_name: "Shared", spritesheet: "beta.png", manifest: "beta.ron",
                        tier: MainHall, body_kind: Standard, composition: None,
                        default_brain: "idle", default_action_set: "peaceful", tags: [],
                    ),
                },
            )"#,
        );
        assert_eq!(
            validate(&catalog),
            vec!["characters 'alpha' and 'beta' share display_name 'Shared'".to_string()]
        );
    }
}
