//! The `fighter_brain_ladder` authored-content schema, owned by this capability.
//!
//! This one had ZERO production readers. `fighter_brain_ladder.ron` was parsed by a content
//! TEST and by nothing else in the workspace: Ambition authored a nine-rung difficulty ladder
//! and the running game never read a row of it.
//!
//! the engine says outright that this is the case that must not happen.
//! [`FighterBrainProfile::for_level`](super::FighterBrainProfile::for_level)
//! describes itself as *"a FLOOR, not the ladder. A game that cares ships its own
//! nine rows (`FighterBrainLadder::from_ron`) and this is never consulted."* Both
//! production call sites consulted it anyway — because a rule about which of two
//! sources wins cannot be enforced by the source that loses. That rule is now
//! [`profile_for_level`](super::profile_for_level); this module is how the
//! winning source arrives.
//!
//! the invariant question was already answered, so this handler wires it up
//! rather than inventing it. The template asks what a family can say that its
//! own parser accepts and the runtime cannot use;
//! [`FighterBrainLadder::problems`] is exactly that list and predates this module
//! — nine rungs, labelled in order, monotone in reaction/APM/noise, and never a
//! zero reaction (*"a shipped difficulty never reacts instantly"*). What changes
//! is WHO asks: a content test asked, and now the pack does, which is the
//! difference between a ladder that is well-formed and a ladder that is loaded.

use std::sync::Arc;

use ambition_content_pack::{
    CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource,
    RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};

use ambition_characters::brain::fighter::FighterBrainLadder;

/// The capability that owns this schema.
pub const FIGHTER_BRAIN_CAPABILITY: &str = "fighter_brain";

/// The authored FILE kind: one game's nine-rung difficulty ladder.
pub const FIGHTER_BRAIN_LADDER_SCHEMA: &str = "fighter_brain_ladder";

/// The schema version this handler reads.
pub const FIGHTER_BRAIN_LADDER_VERSION: SchemaVersion = SchemaVersion(1);

fn canonical<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

struct FighterBrainLadderSchema;

impl ContentSchemaHandler for FighterBrainLadderSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let ladder = match FighterBrainLadder::from_ron(facet.text) {
            Ok(ladder) => ladder,
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

        let id = facet.content_id_in(FIGHTER_BRAIN_LADDER_SCHEMA, "ladder");
        out.define(id.clone(), canonical(&ladder));

        // every fault at once, at load, in one place. A ladder can be nonsense
        // while every individual row looks fine — non-monotone reaction is the
        // clearest case — and the alternative to reporting it here is noticing it
        // as "the levels do not order correctly" after hours of self-play.
        for problem in ladder.problems() {
            out.report(
                facet
                    .diagnostic(DiagnosticCode::MalformedProviderBinding, problem)
                    .about(id.clone())
                    .fix(
                        "a ladder is nine rows, level 1..9 in order, each faster and \
                         cleaner than the last, and none of them instant",
                    ),
            );
        }

        if !out.failed() {
            out.lower(ladder);
        }
    }
}

/// The runtime's load path: the validated ladder, or `None` when this game
/// shipped no rows and the engine floor applies.
pub fn lowered_fighter_brain_ladder(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&FighterBrainLadder> {
    pack.lowered::<FighterBrainLadder>(&SchemaId::new(FIGHTER_BRAIN_LADDER_SCHEMA))
}

pub fn fighter_brain_ladder_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(FIGHTER_BRAIN_LADDER_SCHEMA),
        version: FIGHTER_BRAIN_LADDER_VERSION,
        capability: CapabilityId::new(FIGHTER_BRAIN_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "One game's fighter difficulty ladder: nine rungs of reaction latency, \
              action cap, execution noise and what each rung's scoring NOTICES. \
              Absent means the engine's floor applies.",
        handler: Arc::new(FighterBrainLadderSchema),
    }
}

#[cfg(test)]
mod tests;
