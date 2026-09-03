//! Character materialization readiness checks.
//!
//! Every staged character must reach `Ready` or a named terminal `Failed` state
//! before reveal. `audit_character_capabilities` reports missing composition
//! authority instead of allowing a staged character to remain unsettled.

use bevy::prelude::*;

use super::{CharacterLoadStates, CharacterMaterializationService};
use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::load_demand::CharacterLoadDemand;

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

/// A disagreement between the prepared registry and assembled character catalog.
///
/// Both authorities are currently readable by different runtime paths, so shared
/// character ids must agree on identity, art, provider, and gameplay definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterAuthorityConflict {
    /// One display name identifies different characters across the two authorities.
    AmbiguousDisplayName {
        display_name: String,
        ids: Vec<String>,
    },
    /// One character id has different display names across the two authorities.
    DisplayNameDisagreement {
        character_id: String,
        registry_display_name: String,
        catalog_display_name: String,
    },
    /// One id declared by both authorities, with different art.
    ///
    /// both sheets are the CANONICAL sheet target, never the raw strings
    /// the two authorities happen to store. A registry names a target (`robot`);
    /// a catalog names files (`sprites/robot_spritesheet.png` +
    /// `sprites/robot_spritesheet.ron`). Comparing those as strings reported a
    /// conflict for every character both authorities declare — ten of them in
    /// the shipped cast, including `sanic`, `robot` and `mary_o` — so this
    /// audit's `error!` was permanently on and therefore unreadable
    /// .
    SheetDisagreement {
        character_id: String,
        registry_sheet: String,
        catalog_sheet: String,
    },
    /// One character ID resolving to two providers. Provider identity controls
    /// both presentation authorization and cue-bank selection, so disagreement is
    /// an authority split.
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
            Self::DisplayNameDisagreement {
                character_id,
                registry_display_name,
                catalog_display_name,
            } => write!(
                f,
                "`{character_id}` presents as `{registry_display_name}` according to the \
                 prepared registry and as `{catalog_display_name}` according to the \
                 catalog. Both are read: content addresses characters by name through \
                 the registry (`id_for_display_name`), while labels, barks and rosters \
                 read the catalog row — so this character answers to one name and is \
                 shown under the other. Author it once."
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
    let registry = world.get_resource::<ambition_characters::prepared::PreparedCharacterRegistry>();
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

    // And the NAME, for an id both authorities declare. The check above asks
    // "does this name belong to more than one character"; this one asks "does
    // this character have more than one name", which is a different question and
    // was never asked. Both authorities answer it to different consumers.
    if let (Some(registry), Some(catalog)) = (registry, catalog) {
        for (id, prepared) in registry.iter() {
            let Some(entry) = catalog.get(id) else {
                continue;
            };
            if entry.display_name != prepared.display_name {
                conflicts.push(CharacterAuthorityConflict::DisplayNameDisagreement {
                    character_id: id.to_string(),
                    registry_display_name: prepared.display_name.clone(),
                    catalog_display_name: entry.display_name.clone(),
                });
            }
        }
    }

    // And the art, for an id both authorities declare. Only when the registry
    // actually named a sheet: a registration that names none is deferring to the
    // catalog on purpose, which is agreement, not conflict.
    //
    // compared as sheet TARGETS, not as the strings each side stores. The
    // two authorities write the same logical asset in two different vocabularies
    // — a registry `with_sheet` names the baked manifest target (`robot`), a
    // catalog row names its files (`sprites/robot_spritesheet.png` and
    // `sprites/robot_spritesheet.ron`) — and `manifest_target()` is the existing
    // canonical projection between them. Comparing raw strings made this fire on
    // every character both authorities declare, which is to say it fired on
    // agreement. Ten shipped characters, an `error!` on
    // every boot, and nothing ever asserted on it — a guard that is always red is
    // a guard nobody reads.
    if let (Some(registry), Some(catalog)) = (registry, catalog) {
        for (id, prepared) in registry.iter() {
            let Some(registry_sheet) = prepared.sheet.as_deref() else {
                continue;
            };
            let Some(entry) = catalog.get(id) else {
                continue;
            };
            let catalog_target = entry.manifest_target();
            // The raw forms stay accepted as well. `manifest_target()` returns
            // `None` for a manifest that does not follow the `*_spritesheet.ron`
            // convention, and a row that stores the target verbatim is agreeing
            // in the plainest way there is — neither should be reported.
            if catalog_target == Some(registry_sheet)
                || entry.spritesheet == registry_sheet
                || entry.manifest == registry_sheet
            {
                continue;
            }
            conflicts.push(CharacterAuthorityConflict::SheetDisagreement {
                character_id: id.to_string(),
                registry_sheet: registry_sheet.to_string(),
                // The NORMALIZED form when there is one, so a real conflict reads
                // as two targets that differ rather than as a target next to a
                // path, which is the shape that made every agreement look like a
                // conflict in the first place.
                catalog_sheet: catalog_target
                    .map(str::to_owned)
                    .unwrap_or_else(|| entry.spritesheet.clone()),
            });
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
        bevy::log::error!(target: "ambition_platformer2d::character_runtime", "{conflict}");
    }
}

/// Report capability gaps once per frame in which staged characters cannot be
/// served. `error!` rather than `warn!`: a staged character that can never get
/// art is a broken composition, and the whole point is that this stopped being
/// silent.
pub fn report_character_capability_gaps(world: &mut World) {
    let gaps = audit_character_capabilities(world);
    for gap in gaps {
        bevy::log::error!(target: "ambition_platformer2d::character_runtime", "{gap}");
    }
}

#[cfg(test)]
mod authority_parity_tests {
    use super::*;
    use crate::character_runtime::CharacterDefinitionAppExt;
    use ambition_characters::actor::character_catalog::parse_catalog;
    use ambition_characters::actor::definition::CharacterDefinition;

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

    use ambition_platformer2d_shared_tangle::app_finalization::finalize;

    fn app_with_catalog() -> App {
        let mut app = App::new();
        app.insert_resource(
            ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
                parse_catalog(CATALOG),
            ),
        );
        app
    }

    /// A shared character id with different sheets is an authority conflict.
    #[test]
    fn a_character_declared_by_both_authorities_with_different_art_is_a_conflict() {
        let mut app = app_with_catalog();
        app.register_character(
            CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo")
                .with_sheet("registry_sheet"),
        );

        finalize(&mut app);
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
    #[test]
    fn deferring_to_the_catalog_for_art_is_not_a_conflict() {
        let mut app = app_with_catalog();
        app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
        finalize(&mut app);
        assert_eq!(audit_character_authority_parity(app.world()), Vec::new());
    }

    /// H3: two authorities assign different providers to one character.
    /// Provider disagreement can split construction ownership from presentation
    /// and audio ownership.
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

        finalize(&mut app);
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

    /// Duplicate declarations are allowed when their providers agree.
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
        finalize(&mut app);
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

        finalize(&mut app);
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

    /// The inverse question, which was never asked. (AF4b residue,
    /// )
    ///
    /// The test above asks whether one NAME belongs to several characters. This
    /// asks whether one CHARACTER has several names, and the audit could not
    /// answer it: both authorities author a display name, both are read — the
    /// registry answers `id_for_display_name`, which is how content addresses a
    /// character; the catalog answers the label on the pedestal — and nothing
    /// compared them. Rename in one place and the character answers to one name
    /// while being shown under the other.
    #[test]
    fn one_character_presenting_under_two_names_is_a_conflict() {
        let mut app = app_with_catalog();
        // The same id the catalog declares, renamed in the registry only —
        // exactly what editing one of two authorities looks like.
        app.register_character(CharacterDefinition::new("mary_o", "Mary O", "mary_o_demo"));

        finalize(&mut app);
        let conflicts = audit_character_authority_parity(app.world());
        assert!(
            conflicts.contains(&CharacterAuthorityConflict::DisplayNameDisagreement {
                character_id: "mary_o".to_string(),
                registry_display_name: "Mary O".to_string(),
                catalog_display_name: "Mary-O".to_string(),
            }),
            "one id with two display names must be reported: content addressing \
             resolves through one authority and every label reads the other: \
             {conflicts:?}"
        );
    }

    #[test]
    fn agreeing_display_names_are_not_a_conflict() {
        let mut app = app_with_catalog();
        app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
        finalize(&mut app);
        assert_eq!(audit_character_authority_parity(app.world()), Vec::new());
    }
}

// ── THE OPENING BELL IS NOT ALLOWED TO OUTRUN THE ART ───────────────────────

/// Match-critical characters whose art was still loading after the opening hold
/// came off.
///
/// ⭐ THE CONTRACT THIS NAMES: a fighter in the roster is known at select time,
/// so its sheets have the whole preparation window to arrive. If one is still
/// resolving once the match is LIVE, the decode lands on a gameplay frame — the
/// shape the first hardware profile measured as a 516ms frame.
///
/// ⛔ THIS REPORTS; IT MUST NEVER GATE. `release_the_opening_hold` releases on the
/// countdown alone, and that is correct: the countdown is replayed by rollback,
/// so making it wait on asset readiness would make the simulation depend on IO
/// and desync two peers whose disks disagree. The bell is deterministic and the
/// loading is not, which is exactly why this is an instrument.
#[derive(Resource, Default, Debug)]
pub struct LateMatchCriticalArt {
    late: std::collections::BTreeSet<String>,
    unready_frames: u32,
    live_frames: u32,
}

impl LateMatchCriticalArt {
    /// Roster characters observed unready after the bell, deterministically ordered.
    pub fn late_characters(&self) -> impl Iterator<Item = &str> {
        self.late.iter().map(String::as_str)
    }

    /// Frames on which at least one roster character was still resolving after the bell.
    pub fn unready_frames(&self) -> u32 {
        self.unready_frames
    }

    /// ⚠ THE POPULATION BESIDE THE FINDING. Zero late characters means nothing
    /// until this is non-zero: a run that never reached a live match cannot have
    /// observed a violation, and reporting that as "clean" is the instrument's
    /// silence wearing a number.
    pub fn live_frames_observed(&self) -> u32 {
        self.live_frames
    }
}

/// Name every match-critical character whose art was still resolving after the bell.
///
/// ⚠ Keyed on "has no terminal outcome yet", NOT on `is_ready`. A character whose
/// sheet FAILED is a content defect the art tests already own, and counting it
/// here would report a missing manifest as a performance violation every frame of
/// every match.
pub fn report_late_match_critical_art(
    roster: Option<Res<super::staging::MatchParticipantRoster>>,
    states: Option<Res<CharacterLoadStates>>,
    active: Option<Res<super::seating::ActiveMatch>>,
    prepared: Option<Res<super::prepared_match::PreparedMatch>>,
    tick: Option<Res<ambition_time::SimTick>>,
    late: Option<ResMut<LateMatchCriticalArt>>,
) {
    let (Some(roster), Some(states), Some(active), Some(prepared), Some(mut late)) =
        (roster, states, active, prepared, late)
    else {
        return;
    };
    // Before the bell there is nothing to violate: preparation is exactly when
    // this work is SUPPOSED to happen.
    let Some(elapsed) = tick
        .as_deref()
        .and_then(|now| active.ticks_since_activation(now.get()))
    else {
        return;
    };
    if prepared.rules().opening_phase(elapsed) != super::prepared_match::OpeningPhase::Live {
        return;
    }
    late.live_frames += 1;

    let mut any_unready = false;
    for participant in &roster.participants {
        let id = participant.character.as_str();
        if states.outcome(id).is_some() {
            continue;
        }
        any_unready = true;
        if late.late.insert(id.to_string()) {
            // `eprintln!` with a bare `[tag]`, not `warn!` — that is the census
            // convention in this repo and the profiler's extractors anchor on
            // `^\[`, so a tracing prefix would make this unparseable. One line,
            // every field on it: a qualifier on its own line is one `grep` away
            // from being separated from the number it qualifies.
            eprintln!(
                "[late-art] ticks_after_live={elapsed} character={id} \
                 (a rostered fighter still resolving after the bell decodes on \
                 gameplay frames; the roster named it at preparation time)"
            );
        }
    }
    if any_unready {
        late.unready_frames += 1;
    }
}
