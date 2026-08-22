//! The `encounter_waves` authored-content schema, owned by this capability.
//!
//! The content compiler is the single reader for this family: validation and
//! runtime lowering operate on the same parsed wave book.
//!
//! An encounter with NO waves parses perfectly and means something the author
//! cannot have intended: the loader falls back to marker-derived spawns, which is
//! exactly what omitting the key entirely does. An empty list is therefore a
//! wordier way of writing nothing, and it reads as "this encounter has no mobs".

use std::collections::HashMap;
use std::sync::Arc;

use ambition_content_pack::{
    CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource, PendingRef,
    RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};

use crate::spec::EncounterWaveSpec;

/// The capability that owns every schema in this module.
pub const ENCOUNTER_CAPABILITY: &str = "encounter";

/// The authored FILE kind: a book of wave timelines keyed by trigger id.
pub const ENCOUNTER_WAVES_SCHEMA: &str = "encounter_waves";

/// The schema version this handler reads.
pub const ENCOUNTER_WAVES_VERSION: SchemaVersion = SchemaVersion(1);

/// The identity kind a mob's `character` names — minted by `character_catalog`,
/// which lives in `ambition_characters`.
///
///  a cross-schema reference is by SCHEMA ID, so this crate needs no
/// dependency on the one that owns the family. `boss_encounter` names a
/// `music_track` across exactly the same kind of boundary. What resolves the
/// name is the pack: every identity it defines is matched by
/// `<namespace>:<schema>/<name>`, and a character the pack never defined is an
/// `UnresolvedReference` before the game starts.
const CHARACTER_SCHEMA: &str = "character";

/// What a prepared pack lowers a validated wave book to.
pub type EncounterWaveBook = HashMap<String, Vec<EncounterWaveSpec>>;

struct EncounterWavesSchema;

impl ContentSchemaHandler for EncounterWavesSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let book: EncounterWaveBook = match ron::from_str(facet.text) {
            Ok(book) => book,
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

        declare(facet, &book, out);

        // LOWER only when clean — a caller must never receive a runtime value
        // out of a pack that was refused.
        if !out.failed() {
            out.lower(book);
        }
    }
}

fn declare(facet: &FacetSource<'_>, book: &EncounterWaveBook, out: &mut FacetOutcome) {
    //  iterate SORTED. A `HashMap`'s order is not defined, and a diagnostic list
    // whose order changes between runs is one nobody can diff.
    let mut ids: Vec<&String> = book.keys().collect();
    ids.sort();

    for id in ids {
        let waves = &book[id];
        let trimmed = id.trim();
        if trimmed.is_empty() {
            out.report(facet.diagnostic(
                DiagnosticCode::MalformedSource,
                "an encounter trigger id is empty; the loader looks waves up BY id, so a \
                 nameless entry can never be found",
            ));
            continue;
        }
        if trimmed != id {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::MalformedSource,
                        format!("the trigger id {id:?} has surrounding whitespace"),
                    )
                    .fix(
                        "the loader matches the id VERBATIM against the level's trigger, so a \
                         padded key compiles and is unreachable",
                    ),
            );
        }

        // Define every encounter that contributes runtime content so wave edits
        // also change the pack fingerprint used for compatibility checks.
        //
        //  `Debug` is canonical for THIS vocabulary and it is worth saying why,
        // because it is not canonical in general: `EncounterWaveSpec` and
        // `EncounterMobSpec` are plain fields and ordered `Vec`s all the way
        // down, with no map to randomise its iteration. A map anywhere under
        // here would have to become a `BTreeMap` first — the trap
        // `BossBehaviorProfile::strike_geometry` already fell into.
        out.define(facet.content_id(trimmed), format!("{waves:?}"));

        //  the invariant a serde parse cannot see.
        if waves.is_empty() {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::MalformedSource,
                        format!("encounter {trimmed:?} authors ZERO waves"),
                    )
                    .fix(
                        "an encounter with no waves falls back to marker-derived spawns — \
                         exactly what omitting the key does. Either author the waves or delete \
                         the entry, so the file cannot claim a timeline it does not have",
                    ),
            );
        }

        for (index, wave) in waves.iter().enumerate() {
            //  the character a mob names must EXIST, and a compiler is the
            // only place that can say so. The runtime cannot: an unresolvable
            // character resolves to no sheet, and §4.10's ruling is that there
            // is no fallback sheet — the body draws the placeholder rectangle
            // and the game keeps running. A misspelling therefore looks exactly
            // like authoring nothing, right up until somebody walks into the
            // room. That is the same silence `favourite_snack: "worms"` bought,
            // one level down.
            for mob in &wave.mobs {
                let Some(character) = mob.character.as_deref() else {
                    continue;
                };
                out.refer(PendingRef::new(
                    SchemaId::new(CHARACTER_SCHEMA),
                    character,
                    "character",
                    facet.content_id(trimmed),
                    "character",
                ));
            }
            if wave.mobs.is_empty() {
                out.report(
                    facet
                        .diagnostic(
                            DiagnosticCode::MalformedSource,
                            format!(
                                "encounter {trimmed:?} wave {index} ({:?}) has no mobs",
                                wave.label
                            ),
                        )
                        .fix(
                            "a wave with no mobs is CLEARED the instant it starts, so it reads \
                             as a pause the encounter never actually takes",
                        ),
                );
            }
        }
    }
}

/// The wave book a prepared pack lowered to, if it carries one — the runtime's
/// load path, replacing `ron::from_str(include_str!(…))` at the call site.
pub fn lowered_encounter_waves(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&EncounterWaveBook> {
    pack.lowered::<EncounterWaveBook>(&SchemaId::new(ENCOUNTER_WAVES_SCHEMA))
}

/// The encounter capability's registration, for a composition to install.
pub fn encounter_waves_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(ENCOUNTER_WAVES_SCHEMA),
        version: ENCOUNTER_WAVES_VERSION,
        capability: CapabilityId::new(ENCOUNTER_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "Authored wave timelines keyed by encounter trigger id. An encounter absent from \
              the book falls back to one wave assembled from its level's spawn markers.",
        handler: Arc::new(EncounterWavesSchema),
    }
}

#[cfg(test)]
mod tests;
