//! The `smash_fighter` authored-content schema, owned by the platform-fighter
//! capability.
//!
//! one file per character, merged into one book. The aggregate exists
//! because the runtime looks a fighter up by id and must find exactly one
//! answer; without it, two packs (or one pack with a copy-pasted file) would
//! give a character two capture kits and the winner would be map iteration
//! order.
//!
//! this schema does NOT emit a reference to the character it names, and the
//! omission is deliberate. A `PendingRef` resolves against identities the same
//! pack defines, and the demo that authors the first facet registers its cast in
//! Rust rather than in a `character_catalog` source — so a reference would
//! refuse content that is entirely correct. The place "no such character" is
//! actually discoverable is the LOOKUP, where a fighter's registration asks for
//! its own facet by id and can say what the pack does define; see
//! `ambition_demo_smash::smash_pack`. do not add the reference until a pack
//! authors both halves — an unresolvable-by-construction reference trains
//! authors to ignore the diagnostic that is supposed to be rare.

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

/// What one facet file contributes before the merge: the facet and the path it
/// came from, so a collision names two files rather than two ids.
#[derive(Debug, Clone)]
struct Fragment {
    facet: SmashFighterFacet,
    declared_path: String,
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
        // reporting these here is a grab that plays and catches nobody, which
        // reads in a playtest as "the grab feels bad" rather than as a number.
        for problem in parsed.problems() {
            out.report(
                facet
                    .diagnostic(DiagnosticCode::MalformedProviderBinding, problem)
                    .about(id.clone())
                    .fix(
                        "a capture kit is a grab that is live for a positive window over a \
                         box with area, a pummel whose impact lands inside its own beat, and \
                         throws that release inside theirs",
                    ),
            );
        }

        if !out.failed() {
            out.lower(Fragment {
                facet: parsed,
                declared_path: facet.declared_path.to_string(),
            });
        }
    }

    fn aggregate(
        &self,
        fragments: &[LoweredFragment<'_>],
        out: &mut AggregateOutcome,
    ) -> Aggregation {
        let mut book: SmashFighterBook = BTreeMap::new();
        let mut source_of: BTreeMap<String, String> = BTreeMap::new();
        for fragment in fragments {
            let Some(Fragment {
                facet,
                declared_path,
            }) = fragment.get::<Fragment>()
            else {
                continue;
            };
            // not last-wins. Two files claiming one fighter is a question
            // with two answers, and picking one silently is how a character
            // quietly swings with the values somebody meant to delete.
            if let Some(first) = source_of.get(&facet.character) {
                out.report(
                    AggregateOutcome::refusal(
                        DiagnosticCode::ConflictingModuleContribution,
                        format!(
                            "character `{}` has a platform-fighter facet in `{first}` and in \
                             `{declared_path}`",
                            facet.character
                        ),
                    )
                    .in_source(declared_path)
                    .fix("one character, one facet file — the runtime looks the id up ONCE"),
                );
                continue;
            }
            source_of.insert(facet.character.clone(), declared_path.clone());
            book.insert(facet.character.clone(), facet.clone());
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

pub fn smash_fighter_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(SMASH_FIGHTER_SCHEMA),
        version: SMASH_FIGHTER_VERSION,
        capability: CapabilityId::new(SMASH_FIGHTER_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "One character's platform-fighter values. v1 carries the capture kit — the \
              grab's reach and timing, the pummel, and the throws — which the Smash \
              capability prepares into runtime moves. Every such file merges into one book \
              keyed by character id.",
        handler: Arc::new(SmashFighterSchema),
    }
}

#[cfg(test)]
mod tests;
