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
