//! The catalog's row rules, stated once.
//!
//! [`findings`] walks a catalog and lists what each row must not do and which
//! named presets each row refers to. Two readers show the list:
//! [`validate`], for the RON reader (`CharacterCatalogFragment::from_ron`),
//! and the content schema, which turns each preset reference into a pack
//! reference and each problem into a diagnostic. A rule added here reaches both.

use std::collections::BTreeMap;

use super::entry::CharacterCatalogData;

/// A table of named values that a row can refer to by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PresetTable {
    /// The catalog's own rows.
    Character,
    Brain,
    ActionSet,
    AxisTuning,
    Locomotion,
    AutonomousProfile,
}

impl PresetTable {
    /// The catalog field that holds this table.
    pub(crate) fn field(self) -> &'static str {
        match self {
            Self::Character => "characters",
            Self::Brain => "brain_presets",
            Self::ActionSet => "action_set_presets",
            Self::AxisTuning => "axis_tuning_presets",
            Self::Locomotion => "locomotion_presets",
            Self::AutonomousProfile => "autonomous_profiles",
        }
    }

    pub(crate) fn contains(self, catalog: &CharacterCatalogData, name: &str) -> bool {
        match self {
            Self::Character => catalog.characters.contains_key(name),
            Self::Brain => catalog.brain_presets.contains_key(name),
            Self::ActionSet => catalog.action_set_presets.contains_key(name),
            Self::AxisTuning => catalog.axis_tuning_presets.contains_key(name),
            Self::Locomotion => catalog.locomotion_presets.contains_key(name),
            Self::AutonomousProfile => catalog.autonomous_profiles.contains_key(name),
        }
    }
}

/// One fact about one row: a problem, or a reference to check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Finding<'a> {
    pub character: &'a str,
    /// The authored field the finding is about, as a path (`portrait.image`).
    pub field: &'static str,
    pub kind: FindingKind<'a>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FindingKind<'a> {
    /// A name or path that the row must fill is empty.
    Empty,
    /// An earlier row owns the same display name.
    SharedDisplayName { first: &'a str, display: &'a str },
    /// The row names a preset in `field` and also states its own value in
    /// `inline_field`. `what` is the fact both state ("feel", "policy", "gait").
    BothStated {
        inline_field: &'static str,
        what: &'static str,
    },
    /// The row refers to `name` in `table`. This is not a problem until the
    /// name is absent from the table; the reader decides how to resolve it.
    Reference { table: PresetTable, name: &'a str },
    /// The row states `field`, which has a meaning only beside `needed`.
    /// `why` says what `field` takes from `needed`.
    Needs {
        needed: &'static str,
        why: &'static str,
    },
    /// The value in `field` cannot become what it describes. `rule` says what
    /// the value must be.
    OutOfRange { rule: &'static str },
}

/// Every problem and preset reference of `catalog`, row by row in id order.
pub(crate) fn findings<'a>(catalog: &'a CharacterCatalogData) -> Vec<Finding<'a>> {
    let mut out = Vec::new();
    let mut display_name_owners: BTreeMap<&str, &str> = BTreeMap::new();

    for (id, entry) in &catalog.characters {
        let mut push = |field: &'static str, kind: FindingKind<'a>| {
            out.push(Finding {
                character: id.as_str(),
                field,
                kind,
            });
        };
        if entry.display_name.trim().is_empty() {
            push("display_name", FindingKind::Empty);
        } else if let Some(first) =
            display_name_owners.insert(entry.display_name.trim(), id.as_str())
        {
            push(
                "display_name",
                FindingKind::SharedDisplayName {
                    first,
                    display: entry.display_name.trim(),
                },
            );
        }
        for (field, value) in [
            ("spritesheet", &entry.spritesheet),
            ("manifest", &entry.manifest),
        ] {
            if value.trim().is_empty() {
                push(field, FindingKind::Empty);
            }
        }
        // A portrait is optional, and a PARTIAL one is not: a half-authored
        // portrait shows as a missing texture.
        if let Some(portrait) = &entry.portrait {
            for (field, value) in [
                ("portrait.image", &portrait.image),
                ("portrait.manifest", &portrait.manifest),
                ("portrait.default_clip", &portrait.default_clip),
            ] {
                if value.trim().is_empty() {
                    push(field, FindingKind::Empty);
                }
            }
        }

        // An EMPTY `default_brain` names no preset on purpose (see the field's doc).
        if !entry.default_brain.is_empty() {
            push(
                "default_brain",
                FindingKind::Reference {
                    table: PresetTable::Brain,
                    name: &entry.default_brain,
                },
            );
        }
        push(
            "default_action_set",
            FindingKind::Reference {
                table: PresetTable::ActionSet,
                name: &entry.default_action_set,
            },
        );
        // A named preset and the row's own value for the same fact: the row
        // must state one of them.
        for (field, table, name, inline_field, inline_stated, what) in [
            (
                "axis_tuning_preset",
                PresetTable::AxisTuning,
                entry.axis_tuning_preset.as_deref(),
                "axis_tuning",
                entry.axis_tuning.is_some(),
                "feel",
            ),
            (
                "named_autonomous_profile",
                PresetTable::AutonomousProfile,
                entry.named_autonomous_profile.as_deref(),
                "autonomous_profile",
                entry.autonomous_profile.is_some(),
                "policy",
            ),
            (
                "locomotion_preset",
                PresetTable::Locomotion,
                entry.locomotion_preset.as_deref(),
                "locomotion",
                entry.locomotion.is_some(),
                "gait",
            ),
        ] {
            let Some(name) = name else { continue };
            push(field, FindingKind::Reference { table, name });
            if inline_stated {
                push(field, FindingKind::BothStated { inline_field, what });
            }
        }
        if let Some(insets) = entry.hurtbox_insets {
            if entry.hurtboxes.is_some() {
                push(
                    "hurtbox_insets",
                    FindingKind::BothStated {
                        inline_field: "hurtboxes",
                        what: "hurtbox",
                    },
                );
            }
            if entry.posed_body.is_none() {
                push(
                    "hurtbox_insets",
                    FindingKind::Needs {
                        needed: "posed_body",
                        why: "the insets are fractions of the body box its sheet authors at that scale",
                    },
                );
            }
            // The registration builds a box of (1 - left - right) by
            // (1 - top - bottom) of the body, and a hurtbox must have a finite,
            // positive size.
            let edges = [insets.left, insets.right, insets.top, insets.bottom];
            let leaves_a_box = edges.iter().all(|e| e.is_finite() && *e >= 0.0)
                && insets.left + insets.right < 1.0
                && insets.top + insets.bottom < 1.0;
            if !leaves_a_box {
                push(
                    "hurtbox_insets",
                    FindingKind::OutOfRange {
                        rule: "each inset is a finite fraction of at least 0, and left + right and top + bottom are each less than 1",
                    },
                );
            }
        }
        if let Some(name) = entry.derived_from.as_deref() {
            push(
                "derived_from",
                FindingKind::Reference {
                    table: PresetTable::Character,
                    name,
                },
            );
        }
        if let Some(name) = entry.provoked_profile.as_deref() {
            push(
                "provoked_profile",
                FindingKind::Reference {
                    table: PresetTable::AutonomousProfile,
                    name,
                },
            );
        }
    }
    out
}

/// Every problem in `catalog`, as one line each. An empty list means the
/// catalog is internally consistent.
pub fn validate(catalog: &CharacterCatalogData) -> Vec<String> {
    findings(catalog)
        .into_iter()
        .filter_map(|Finding { character, field, kind }| match kind {
            FindingKind::Empty => Some(format!("character '{character}' has an empty {field}")),
            FindingKind::SharedDisplayName { first, display } => Some(format!(
                "characters '{first}' and '{character}' share display_name '{display}'"
            )),
            FindingKind::BothStated { inline_field, what } => Some(format!(
                "character '{character}' states both {inline_field} and {field}; state one {what}"
            )),
            FindingKind::Reference { table, name } => (!table.contains(catalog, name)).then(|| {
                format!(
                    "character '{character}' {field} '{name}' not found in {}",
                    table.field()
                )
            }),
            FindingKind::Needs { needed, why } => Some(format!(
                "character '{character}' states {field} without {needed}: {why}"
            )),
            FindingKind::OutOfRange { rule } => {
                Some(format!("character '{character}' {field} is out of range: {rule}"))
            }
        })
        .collect()
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
            vec!["character 'alpha' has an empty portrait.image".to_string()]
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
