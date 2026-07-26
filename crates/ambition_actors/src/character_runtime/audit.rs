//! **The readiness invariant, and the backstop that names an omission.** (§4.9)
//!
//! ## Why not "compare two apps' resources"
//!
//! The obvious test for the defect this module exists to prevent — the host
//! materializes, a demo does not — is to boot both and diff what they installed.
//! That test would pass while being worthless: it asserts an implementation
//! detail (which resources exist) and goes red every time either app gains an
//! unrelated one. It also cannot fail for the RIGHT reason, because two apps that
//! install identical resources can still stage characters differently.
//!
//! The invariant that actually matters is about outcomes:
//!
//! > **Every staged character reaches `Ready` or a named terminal `Failed` state
//! > before the reveal barrier opens.**
//!
//! Note what it forbids: not failure — a missing sheet in an art-free build is
//! legitimate — but SILENCE. A character that is neither ready nor failed is one
//! nobody decided about, which is precisely the state Mary-O's rectangle lived in.
//!
//! ## And the backstop
//!
//! Making omission impossible by construction is the primary defence: the engine
//! plugin group installs the materializer unconditionally, so no application can
//! leave it out. [`audit_character_capabilities`] covers what construction cannot
//! — an unusual composition that assembled the pieces by hand — by NAMING the gap
//! instead of quietly drawing placeholders forever.

use bevy::prelude::*;

use super::{CharacterLoadDemand, CharacterLoadStates, CharacterMaterializationService};
use ambition_characters::actor::character_catalog::CharacterCatalog;

/// A staged character that has not reached any terminal state.
///
/// Returned rather than logged so a reveal barrier can BLOCK on it: this is the
/// thing that must be empty before the curtain opens.
pub fn unsettled_staged_characters(
    demand: &CharacterLoadDemand,
    states: &CharacterLoadStates,
) -> Vec<String> {
    demand
        .pending()
        .filter(|token| states.outcome(token).is_none())
        .map(str::to_string)
        .collect()
}

/// True when every staged character has a terminal answer.
pub fn character_reveal_ready(demand: &CharacterLoadDemand, states: &CharacterLoadStates) -> bool {
    unsettled_staged_characters(demand, states).is_empty()
}

/// A capability the composition needs and does not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterCapabilityGap {
    /// Characters were staged, but nothing can turn them into art.
    MaterializationServiceMissing { staged: Vec<String> },
    /// The materializer is installed but has no catalog to resolve against, so
    /// every character would resolve to "no sheet" for a reason invisible from
    /// the outside. Required authority, never an empty default
    /// (`engine.character-authority-is-app-local`).
    CharacterCatalogMissing { staged: Vec<String> },
}

impl std::fmt::Display for CharacterCapabilityGap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaterializationServiceMissing { staged } => write!(
                f,
                "character(s) {} were staged, but the CharacterMaterialization engine \
                 service is not installed — every one of them will draw the placeholder \
                 rectangle. Add the engine plugin group (it installs \
                 CharacterRuntimePlugin unconditionally) rather than the materializer \
                 by hand.",
                quoted(staged)
            ),
            Self::CharacterCatalogMissing { staged } => write!(
                f,
                "character(s) {} were staged and the materializer is installed, but no \
                 CharacterCatalog resource exists — nothing can resolve a sheet spec, so \
                 every character would silently have 'no art'. Assemble the App-local \
                 character catalog before staging.",
                quoted(staged)
            ),
        }
    }
}

fn quoted(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|t| format!("`{t}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Audit this world's ability to actually materialize what it has staged.
///
/// Reads the world rather than a registry of expectations, so it cannot go stale
/// against a composition it has never seen.
pub fn audit_character_capabilities(world: &World) -> Vec<CharacterCapabilityGap> {
    let staged: Vec<String> = world
        .get_resource::<CharacterLoadDemand>()
        .map(|demand| demand.pending().map(str::to_string).collect())
        .unwrap_or_default();
    // Nothing staged: there is nothing to be unable to do. An app that stages no
    // characters is not broken, it is a menu.
    if staged.is_empty() {
        return Vec::new();
    }
    let mut gaps = Vec::new();
    if world
        .get_resource::<CharacterMaterializationService>()
        .is_none()
    {
        gaps.push(CharacterCapabilityGap::MaterializationServiceMissing {
            staged: staged.clone(),
        });
    }
    if world.get_resource::<CharacterCatalog>().is_none() {
        gaps.push(CharacterCapabilityGap::CharacterCatalogMissing { staged });
    }
    gaps
}

/// **Two declaration authorities disagreeing about one character.**
///
/// The prepared registry and the assembled catalog are both real authorities during
/// the migration, and every lookup in the engine prefers one or the other by rule:
/// the sheet resolver takes the registry first, `provider_of_character` takes the
/// registry first, the sprite alias table takes whatever declared last. Those rules
/// only agree when the two authorities agree, and nothing checked that they did — so
/// a character could be spawned with the catalog's moveset, drawn from the registry's
/// sheet, and credited to the registry's provider (GPT 5.6, 2026-07-26).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterAuthorityConflict {
    /// One display name, two different characters, across the combined namespace.
    /// The registration seam rejects this WITHIN the registry; this is the case it
    /// cannot see, because the catalog is assembled separately and may arrive after.
    AmbiguousDisplayName {
        display_name: String,
        ids: Vec<String>,
    },
    /// One id declared by both authorities, with different art.
    SheetDisagreement {
        character_id: String,
        registry_sheet: String,
        catalog_sheet: String,
    },
    /// One id, two PROVIDERS. The sharpest form of the split, and the one the
    /// first version of this audit did not check (GPT 5.6, 2026-07-26): the
    /// provider is what authorizes a presentation source and what selects a cue
    /// bank, and `provider_of_character` prefers the registry while everything
    /// reading `CharacterCatalogOwners` gets the other answer. A character can end
    /// up constructed from catalog provider A and SOUNDING like registry provider B.
    ProviderDisagreement {
        character_id: String,
        registry_provider: String,
        catalog_provider: String,
    },
}

impl std::fmt::Display for CharacterAuthorityConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AmbiguousDisplayName { display_name, ids } => write!(
                f,
                "display name `{display_name}` is claimed by {} — content addresses \
                 characters by that label (a room's `enemy.name`, an interactable's \
                 `character_id`, a roster entry) and the registry, the catalog and the \
                 sprite alias table each resolve it their own way, so a demand for it can \
                 stage one character and decode another's art. Give one a distinct name.",
                quoted(ids)
            ),
            Self::SheetDisagreement {
                character_id,
                registry_sheet,
                catalog_sheet,
            } => write!(
                f,
                "`{character_id}` is declared by BOTH authorities with different art: the \
                 registry says `{registry_sheet}`, the catalog says `{catalog_sheet}`. \
                 Every resolver in the engine prefers the registry, so the catalog's \
                 sheet is dead content that still looks authoritative — and anything \
                 reading the catalog directly disagrees with what is drawn."
            ),
            Self::ProviderDisagreement {
                character_id,
                registry_provider,
                catalog_provider,
            } => write!(
                f,
                "`{character_id}` is authored by `{registry_provider}` according to the \
                 prepared registry and by `{catalog_provider}` according to the catalog \
                 owners map. The provider decides which presentation source is \
                 authorized and which cue bank answers, and `provider_of_character` \
                 prefers the registry — so this character is constructed as one \
                 provider's and sounds like the other's."
            ),
        }
    }
}

/// Compare the two declaration authorities. Empty when they agree, or when only
/// one of them exists — which is the ordinary case and not a conflict.
pub fn audit_character_authority_parity(world: &World) -> Vec<CharacterAuthorityConflict> {
    let registry = world.get_resource::<super::PreparedCharacterRegistry>();
    let catalog = world.get_resource::<CharacterCatalog>();
    let owners = world
        .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalogOwners>();
    let mut conflicts = Vec::new();

    // Display names across the COMBINED namespace, so a registry entry and a
    // catalog entry that present alike are caught even though neither authority
    // can see the collision on its own.
    let mut by_display: std::collections::BTreeMap<&str, std::collections::BTreeSet<&str>> =
        std::collections::BTreeMap::new();
    if let Some(registry) = registry {
        for (id, prepared) in registry.iter() {
            by_display
                .entry(prepared.display_name.as_str())
                .or_default()
                .insert(id);
        }
    }
    if let Some(catalog) = catalog {
        for (id, entry) in catalog.iter() {
            by_display
                .entry(entry.display_name.as_str())
                .or_default()
                .insert(id.as_str());
        }
    }
    for (display_name, ids) in by_display {
        if ids.len() > 1 {
            conflicts.push(CharacterAuthorityConflict::AmbiguousDisplayName {
                display_name: display_name.to_string(),
                ids: ids.into_iter().map(str::to_string).collect(),
            });
        }
    }

    // And the art, for an id both authorities declare. Only when the registry
    // actually named a sheet: a registration that names none is deferring to the
    // catalog on purpose, which is agreement, not conflict.
    if let (Some(registry), Some(catalog)) = (registry, catalog) {
        for (id, prepared) in registry.iter() {
            let Some(registry_sheet) = prepared.sheet.as_deref() else {
                continue;
            };
            let Some(entry) = catalog.get(id) else {
                continue;
            };
            if entry.spritesheet != registry_sheet && entry.manifest != registry_sheet {
                conflicts.push(CharacterAuthorityConflict::SheetDisagreement {
                    character_id: id.to_string(),
                    registry_sheet: registry_sheet.to_string(),
                    catalog_sheet: entry.spritesheet.clone(),
                });
            }
        }
    }

    // And the AUTHOR, which is the field with teeth: it picks the cue bank and the
    // authorized presentation source. Checked against `CharacterCatalogOwners` —
    // the catalog's own record of who contributed each id — rather than against the
    // catalog entry, which does not carry a provider at all.
    if let (Some(registry), Some(owners)) = (registry, owners) {
        for (id, prepared) in registry.iter() {
            let Some(catalog_provider) = owners.provider_for(id) else {
                continue;
            };
            if catalog_provider != prepared.provider {
                conflicts.push(CharacterAuthorityConflict::ProviderDisagreement {
                    character_id: id.to_string(),
                    registry_provider: prepared.provider.clone(),
                    catalog_provider: catalog_provider.to_string(),
                });
            }
        }
    }
    conflicts
}

/// Report authority conflicts. `error!` for the same reason capability gaps are:
/// a composition where two authorities disagree about who a character is will
/// produce a fighter assembled from both, and nothing else says so.
pub fn report_character_authority_conflicts(world: &mut World) {
    for conflict in audit_character_authority_parity(world) {
        bevy::log::error!(target: "ambition::character_runtime", "{conflict}");
    }
}

/// Report capability gaps once per frame in which staged characters cannot be
/// served. `error!` rather than `warn!`: a staged character that can never get
/// art is a broken composition, and the whole point is that this stopped being
/// silent.
pub fn report_character_capability_gaps(world: &mut World) {
    let gaps = audit_character_capabilities(world);
    for gap in gaps {
        bevy::log::error!(target: "ambition::character_runtime", "{gap}");
    }
}

#[cfg(test)]
mod authority_parity_tests {
    use super::*;
    use crate::character_runtime::{CharacterDefinition, CharacterDefinitionAppExt};
    use ambition_characters::actor::character_catalog::parse_catalog;

    /// One catalog character, so the two authorities have something to disagree
    /// about. `mary_o` is spelled the same way the registration below spells it.
    const CATALOG: &str = r#"(
    brain_presets: { "idle": StandStill },
    action_set_presets: { "peaceful": (move_style: Walk) },
    characters: {
        "mary_o": (
            display_name: "Mary-O",
            spritesheet: "catalog_sheet.png",
            manifest: "catalog_sheet.ron",
            tier: MainHall,
            body_kind: Standard,
            default_brain: "idle",
            default_action_set: "peaceful",
        ),
    },
)"#;

    fn app_with_catalog() -> App {
        let mut app = App::new();
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
                parse_catalog(CATALOG),
            ),
        );
        app
    }

    /// **The migration hazard, named.** Both authorities declare `mary_o`, with
    /// different art. Every resolver in the engine prefers the registry, so the
    /// catalog's sheet is dead content that still reads as authoritative — and
    /// anything consulting the catalog directly disagrees with what is drawn.
    #[test]
    fn a_character_declared_by_both_authorities_with_different_art_is_a_conflict() {
        let mut app = app_with_catalog();
        app.register_character(
            CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo")
                .with_sheet("registry_sheet"),
        );

        let conflicts = audit_character_authority_parity(app.world());
        assert!(
            conflicts.contains(&CharacterAuthorityConflict::SheetDisagreement {
                character_id: "mary_o".to_string(),
                registry_sheet: "registry_sheet".to_string(),
                catalog_sheet: "catalog_sheet.png".to_string(),
            }),
            "two authorities naming different art for one character must be \
             reported, not silently resolved by whichever lookup ran: {conflicts:?}"
        );
    }

    /// A registration that names NO sheet is deferring to the catalog on purpose.
    /// That is the ordinary migration state and must not be reported as a conflict,
    /// or the report becomes noise and stops being read.
    #[test]
    fn deferring_to_the_catalog_for_art_is_not_a_conflict() {
        let mut app = app_with_catalog();
        app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
        assert_eq!(audit_character_authority_parity(app.world()), Vec::new());
    }

    /// **H3: the field with teeth.** Two authorities, two different AUTHORS for
    /// one character.
    ///
    /// Sharper than the sheet disagreement, because the provider is what
    /// authorizes a presentation source and what selects a cue bank —
    /// `provider_of_character` prefers the registry while everything reading
    /// `CharacterCatalogOwners` gets the other answer, so the character is built as
    /// one provider's and sounds like the other's. The first version of this audit
    /// explained that exact hazard in its own doc comment and then checked only art
    /// and display names (GPT 5.6, 2026-07-26).
    #[test]
    fn a_character_authored_by_two_different_providers_is_a_conflict() {
        let mut app = app_with_catalog();
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalogOwners(
                [("mary_o".to_string(), "catalog_provider".to_string())]
                    .into_iter()
                    .collect(),
            ),
        );
        app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));

        let conflicts = audit_character_authority_parity(app.world());
        assert!(
            conflicts.contains(&CharacterAuthorityConflict::ProviderDisagreement {
                character_id: "mary_o".to_string(),
                registry_provider: "mary_o_demo".to_string(),
                catalog_provider: "catalog_provider".to_string(),
            }),
            "the provider picks the cue bank and the authorized source, so two \
             authorities naming different ones is the split that matters most: \
             {conflicts:?}"
        );
    }

    /// Agreeing providers are not a conflict — the ordinary state of a character
    /// that is declared twice during the migration.
    #[test]
    fn agreeing_providers_are_not_a_conflict() {
        let mut app = app_with_catalog();
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalogOwners(
                [("mary_o".to_string(), "mary_o_demo".to_string())]
                    .into_iter()
                    .collect(),
            ),
        );
        app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
        assert_eq!(audit_character_authority_parity(app.world()), Vec::new());
    }

    /// The collision the registration seam CANNOT see: the catalog is assembled
    /// separately, so a registry entry and a catalog entry presenting under one
    /// display name are two authorities that each look internally consistent.
    #[test]
    fn a_display_name_claimed_across_both_authorities_is_a_conflict() {
        let mut app = app_with_catalog();
        // A different id, the same label the catalog already uses.
        app.register_character(CharacterDefinition::new("mary_o_alt", "Mary-O", "other"));

        let conflicts = audit_character_authority_parity(app.world());
        assert!(
            conflicts.contains(&CharacterAuthorityConflict::AmbiguousDisplayName {
                display_name: "Mary-O".to_string(),
                ids: vec!["mary_o".to_string(), "mary_o_alt".to_string()],
            }),
            "registration rejects an ambiguous name within the registry, and this is \
             the half it cannot reach: {conflicts:?}"
        );
    }
}
