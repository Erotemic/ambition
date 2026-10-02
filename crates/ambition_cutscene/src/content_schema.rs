//! The `cutscene_library` authored-content schema, owned by the cutscene
//! capability.
//!
//! A pack authors its cutscenes as RON lists of [`CutsceneScript`]s, in one
//! file or several. The content compiler checks each file, and merges the
//! files into one library in manifest order.
//!
//! The checks are the ones a serde parse cannot do. A script with no beats
//! parses and plays nothing. Two scripts with one id parse, and the library
//! keeps only the last (the compiler refuses one id defined twice, also across
//! files). An empty seen flag parses, and the runtime would mark
//! and test a flag with no name. A negative or non-finite duration parses,
//! and a beat's timer never ends or ends at once.

use std::sync::Arc;

use ambition_content_pack::{
    AggregateOutcome, Aggregation, CapabilityId, ContentSchemaHandler, DiagnosticCode,
    FacetOutcome, FacetSource, LoweredFragment, RuntimeDisposition, SchemaId, SchemaRegistration,
    SchemaVersion,
};

use crate::{CutsceneBeat, CutsceneScript};

/// The capability that owns this schema.
pub const CUTSCENE_CAPABILITY: &str = "cutscene";

/// The authored FILE kind: a list of cutscene scripts.
pub const CUTSCENE_LIBRARY_SCHEMA: &str = "cutscene_library";

/// The schema version this handler reads.
pub const CUTSCENE_LIBRARY_VERSION: SchemaVersion = SchemaVersion(1);

/// What a prepared pack lowers its cutscene files to: every script, the files
/// in manifest order and each file in authored order.
pub type AuthoredCutscenes = Vec<CutsceneScript>;

struct CutsceneLibrarySchema;

impl ContentSchemaHandler for CutsceneLibrarySchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let scripts: AuthoredCutscenes = match ron::from_str(facet.text) {
            Ok(scripts) => scripts,
            Err(error) => {
                let code = match error.code {
                    ron::error::Error::NoSuchStructField { .. } => DiagnosticCode::UnknownField,
                    _ => DiagnosticCode::MalformedSource,
                };
                out.report(facet.diagnostic(code, format!("{error}")));
                return;
            }
        };
        declare(facet, &scripts, out);
        if !out.failed() {
            out.lower(scripts);
        }
    }

    /// One library from every file, in manifest order. A script id in two
    /// files needs no check here: each file defines a content id per script,
    /// and the compiler refuses one id defined twice.
    fn aggregate(
        &self,
        fragments: &[LoweredFragment<'_>],
        out: &mut AggregateOutcome,
    ) -> Aggregation {
        let merged: AuthoredCutscenes = fragments
            .iter()
            .filter_map(|fragment| fragment.get::<AuthoredCutscenes>())
            .flatten()
            .cloned()
            .collect();
        out.lower(merged);
        Aggregation::Defined
    }
}

fn declare(facet: &FacetSource<'_>, scripts: &AuthoredCutscenes, out: &mut FacetOutcome) {
    let mut seen = std::collections::BTreeSet::new();
    for script in scripts {
        let id = script.id.as_str();
        if id.trim().is_empty() || id.trim() != id {
            out.report(facet.diagnostic(
                DiagnosticCode::MalformedSource,
                format!(
                    "the cutscene id {id:?} is empty or has surrounding whitespace; a room's \
                     `entry_cutscene` and a trigger name it verbatim"
                ),
            ));
            continue;
        }
        if !seen.insert(id) {
            out.report(facet.diagnostic(
                DiagnosticCode::DuplicateIdentity,
                format!("two cutscenes have the id {id:?}; the library keeps only one"),
            ));
            continue;
        }
        // `Debug` is canonical here: plain fields and ordered `Vec`s.
        out.define(facet.content_id(id), format!("{script:?}"));
        if script.beats.is_empty() {
            out.report(facet.diagnostic(
                DiagnosticCode::MalformedSource,
                format!("cutscene {id:?} has no beats, so it plays nothing"),
            ));
        }
        if script.seen_flag.as_deref().is_some_and(|flag| flag.trim().is_empty()) {
            out.report(facet.diagnostic(
                DiagnosticCode::MalformedSource,
                format!("cutscene {id:?} has an empty seen flag; omit the field instead"),
            ));
        }
        for (index, beat) in script.beats.iter().enumerate() {
            if let Some(problem) = beat_problem(beat) {
                out.report(facet.diagnostic(
                    DiagnosticCode::MalformedSource,
                    format!("cutscene {id:?} beat {index}: {problem}"),
                ));
            }
        }
    }
}

/// What is wrong with one beat that its type allows, if anything.
fn beat_problem(beat: &CutsceneBeat) -> Option<String> {
    let duration = |seconds: f32| {
        (!seconds.is_finite() || seconds < 0.0)
            .then(|| format!("the duration {seconds} is negative or not finite"))
    };
    match beat {
        CutsceneBeat::Wait { seconds }
        | CutsceneBeat::Banner { seconds, .. }
        | CutsceneBeat::CameraPan { seconds, .. } => duration(*seconds),
        CutsceneBeat::Fade {
            from_alpha,
            to_alpha,
            seconds,
        } => duration(*seconds).or_else(|| {
            [from_alpha, to_alpha]
                .iter()
                .any(|alpha| !(0.0..=1.0).contains(*alpha))
                .then(|| format!("the alphas {from_alpha} -> {to_alpha} are not in 0..=1"))
        }),
        CutsceneBeat::SetFlag { id, .. } => {
            id.trim().is_empty().then(|| "the flag id is empty".to_string())
        }
        CutsceneBeat::Dialogue { .. } => None,
    }
}

/// The cutscenes a prepared pack lowered, if it carries any.
pub fn lowered_cutscenes(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&AuthoredCutscenes> {
    pack.lowered::<AuthoredCutscenes>(&SchemaId::new(CUTSCENE_LIBRARY_SCHEMA))
}

/// The cutscene capability's registration, for a composition to install.
pub fn cutscene_library_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(CUTSCENE_LIBRARY_SCHEMA),
        version: CUTSCENE_LIBRARY_VERSION,
        capability: CapabilityId::new(CUTSCENE_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "Cutscene scripts: an id, ordered beats (Wait, Dialogue, CameraPan, Fade, SetFlag, \
              Banner) and an optional seen flag. Several files merge into one library.",
        handler: Arc::new(CutsceneLibrarySchema),
    }
}

#[cfg(test)]
mod tests;
