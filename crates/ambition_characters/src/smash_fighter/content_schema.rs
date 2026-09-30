//! The `smash_fighter` authored-content schema, owned by the platform-fighter
//! capability.
//!
//! one file per character, merged into one book. The aggregate exists
//! because the runtime looks a fighter up by id and must find exactly one
//! answer; without it, two packs (or one pack with a copy-pasted file) would
//! give a character two weights and the winner would be map iteration
//! order.
//!
//! this schema does NOT emit a reference to the character it names. A
//! `PendingRef` resolves against identities the same pack defines, and a facet
//! was first authored in a pack with no `character_catalog` source. The Smash
//! pack now authors both halves, so the reference can be added; queue row AP78
//! records it.
//!
//! This module is the one reader of the facet: [`fold_into_definition`] folds
//! its character facts, and [`fighter_body`] gives its match fact to a
//! composition.

use std::collections::BTreeMap;
use std::sync::Arc;

use ambition_content_pack::{
    AggregateOutcome, Aggregation, CapabilityId, ContentSchemaHandler, DiagnosticCode,
    FacetOutcome, FacetSource, LoweredFragment, RuntimeDisposition, SchemaId, SchemaRegistration,
    SchemaVersion,
};

use super::{SmashFighterBook, SmashFighterFacet, SMASH_FIGHTER_CAPABILITY, SMASH_FIGHTER_SCHEMA};

/// The schema version this handler reads.
pub const SMASH_FIGHTER_VERSION: SchemaVersion = SchemaVersion(1);

/// What one facet file contributes before the merge.
///
/// ⚠ JUST THE FACET. It carried its `declared_path` too, for a collision message
/// the compiler turned out to write better and two stages earlier — see
/// [`SmashFighterSchema::aggregate`].
#[derive(Debug, Clone)]
struct Fragment {
    facet: SmashFighterFacet,
}

struct SmashFighterSchema;

impl ContentSchemaHandler for SmashFighterSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let parsed: SmashFighterFacet = match ron::from_str(facet.text) {
            Ok(parsed) => parsed,
            Err(error) => {
                // Match the ron VARIANT, not the message text — the message is a
                // rendering detail and pinning it makes the diagnostic depend on
                // ron's release notes.
                let code = match error.code {
                    ron::error::Error::NoSuchStructField { .. } => DiagnosticCode::UnknownField,
                    _ => DiagnosticCode::MalformedSource,
                };
                out.report(facet.diagnostic(code, format!("{error}")));
                return;
            }
        };

        let id = facet.content_id_in(SMASH_FIGHTER_SCHEMA, parsed.character.clone());
        out.define(id.clone(), format!("{parsed:?}"));

        // every fault at once, at load, naming the file. The alternative to
        // reporting these here is a fighter whose launch divides by zero, which
        // reads in a playtest as a broken launch rather than as a number.
        for problem in parsed.problems() {
            out.report(
                facet
                    .diagnostic(DiagnosticCode::MalformedProviderBinding, problem)
                    .about(id.clone())
                    .fix(
                        "a fighter body states positive magnitudes, and a knockback weight \
                         is positive",
                    ),
            );
        }

        if !out.failed() {
            out.lower(Fragment { facet: parsed });
        }
    }

    /// ⭐⭐ **A STRAIGHT MERGE, AND ITS COLLISION REFUSAL IS DELETED AS
    /// UNREACHABLE — MEASURED 2026-09-11 BY POISON.** This arm said *"not
    /// last-wins. Two files claiming one fighter is a question with two answers"*
    /// and removing it left all 19 of this schema's tests GREEN, because
    /// [`Self::check`] `define`s a content id per character and the compiler's
    /// own CONFLICT DETECTION stage refuses the second claim before aggregation
    /// runs — naming both source paths, which this arm did not:
    ///
    /// ```text
    /// [duplicate-identity] `ambition:smash_fighter/george` is defined twice:
    ///     in `fighters/george.ron` and in `fighters/george_copy.ron` <!-- cite-ok: an illustrative path, never a real one -->
    /// ```
    ///
    /// ⛔ A HANDLER THAT DEFINES A CONTENT ID PER ENTITY GETS THE COLLISION
    /// REFUSAL FOR FREE. A second one is unreachable code that reads like the
    /// thing enforcing the rule — so the rule looks guarded here and is actually
    /// guarded two stages earlier, and deleting `out.define` would remove it
    /// with nothing going red at this seam.
    fn aggregate(
        &self,
        fragments: &[LoweredFragment<'_>],
        out: &mut AggregateOutcome,
    ) -> Aggregation {
        let mut book: SmashFighterBook = BTreeMap::new();
        for fragment in fragments {
            if let Some(Fragment { facet }) = fragment.get::<Fragment>() {
                book.insert(facet.character.clone(), facet.clone());
            }
        }
        if !out.failed() {
            out.lower(book);
        }
        Aggregation::Defined
    }
}

/// The runtime's load path: every character's platform-fighter facet this pack
/// carries, or `None` when it authored no fighters.
pub fn lowered_smash_fighters(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&SmashFighterBook> {
    pack.lowered::<SmashFighterBook>(&SchemaId::new(SMASH_FIGHTER_SCHEMA))
}

/// `character`'s facet in `pack`, or `None` when the pack authors none.
pub fn facet<'a>(
    pack: &'a ambition_content_pack::PreparedContentPack,
    character: &str,
) -> Option<&'a SmashFighterFacet> {
    lowered_smash_fighters(pack)?.get(character)
}

/// Fold the facet into `definition`: its knockback weight, which applies
/// wherever the character appears, and its move damage, which the definition
/// only carries. A match that declares
/// [`MoveDamageSource::SmashFighterFacet`](super::MoveDamageSource) applies it
/// (`ambition_combat::worn_kit::WornKit::resolve`). One of the folds in
/// [`crate::pack_facets`]. The fighter body is a match fact; see
/// [`fighter_body`].
pub fn fold_into_definition(
    pack: &ambition_content_pack::PreparedContentPack,
    mut definition: crate::actor::definition::CharacterDefinition,
) -> crate::actor::definition::CharacterDefinition {
    let Some(facet) = facet(pack, definition.id.as_str()) else {
        return definition;
    };
    if let Some(weight) = facet.knockback_weight {
        definition.vitals.knockback_weight = Some(weight);
    }
    definition.fighter_move_damage = facet.move_damage.clone();
    definition
}

/// The body `character` plays on as a fighter: its facet's body layered over
/// the player-grade body. A composition gives it to the seat as
/// `MatchParticipant::body`.
///
/// The base is `DEFAULT_TUNING`, not the actor baseline
/// (`BodyMovementTuning::BASELINE`, the wandering-enemy body with an eighth of
/// the player's ground acceleration): an authored body states its differences
/// from a fighter, so they layer onto a fighter.
///
/// `None` when the pack has no facet for the character or the facet states no
/// body: keep the current body.
pub fn fighter_body(
    pack: &ambition_content_pack::PreparedContentPack,
    character: &str,
) -> Option<ambition_platformer2d_core::MovementTuning> {
    facet(pack, character)?
        .body
        .as_ref()
        .map(|body| body.over(ambition_platformer2d_core::DEFAULT_TUNING))
}

pub fn smash_fighter_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(SMASH_FIGHTER_SCHEMA),
        version: SMASH_FIGHTER_VERSION,
        capability: CapabilityId::new(SMASH_FIGHTER_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "One character's platform-fighter values apart from its moves: its fighter \
              body, its knockback weight, and the damage its moves deal on a \
              platform-fighter stage. The moves, grab included, are a `moveset` file. \
              Every such file merges into one book keyed by character id.",
        handler: Arc::new(SmashFighterSchema),
    }
}

#[cfg(test)]
mod tests;
