//! The boss-pattern capability's authored-content SCHEMA registrations.
//!
//! Two families, both owned here because the types they parse into are owned
//! here: the SEED LIBRARY (`boss_seeds.ron`) and the fairness CALIBRATION
//! (`boss_validator_bands.ron`).
//!
//! ## Why these two and not `boss_profiles.ron`
//!
//! ⛔ `BossBehaviorProfile` lives in `ambition_platformer2d_actor_monolith`, and a
//! schema must be registered by the crate that owns its type — which means the
//! CLI has to LINK that crate to validate the family. Measured 2026-08-03: the
//! validator's dependency graph is 239 crates, the monolith's is 708, and the
//! monolith pulls `bevy_render`. Migrating a monolith-owned family would nearly
//! triple the validator and make it link a renderer, which contradicts the one
//! property that justifies it (`cargo build` in seconds, validate in
//! milliseconds).
//!
//! So `boss_profiles` waits on a placement decision, not on a handler. These two
//! parse into types `ambition_characters` already owns, and the CLI already
//! links this crate — they cost the tool nothing.

use std::sync::Arc;

use ambition_content_pack::{
    CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource,
    RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};

use super::seeds::SeedLibrary;
use super::validator::ValidatorBands;

/// The capability that owns both schemas here.
pub const BOSS_PATTERN_CAPABILITY: &str = "boss_pattern";

pub const BOSS_SEEDS_SCHEMA: &str = "boss_seed_library";
/// One attack archetype in the library.
pub const BOSS_SEED_SCHEMA: &str = "boss_seed";
pub const BOSS_VALIDATOR_BANDS_SCHEMA: &str = "boss_validator_bands";

pub const BOSS_SEEDS_VERSION: SchemaVersion = SchemaVersion(1);
pub const BOSS_VALIDATOR_BANDS_VERSION: SchemaVersion = SchemaVersion(1);

/// The canonical form an entry contributes to the pack fingerprint.
///
/// ⚠ `Debug`, not `ron::ser`, because these types are `Deserialize`-only — they
/// are read from authored RON and never written back. Debug is derived on all of
/// them, gives stable field order and materialised values, and moves when a
/// value moves. Adding `Serialize` to five types purely to hash them would be a
/// wider change for the same property.
fn canonical<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

fn code_for(error: &ron::error::SpannedError) -> DiagnosticCode {
    // Match the ron VARIANT, not the message text.
    match error.code {
        ron::error::Error::NoSuchStructField { .. } => DiagnosticCode::UnknownField,
        _ => DiagnosticCode::MalformedSource,
    }
}

// ── the seed library ─────────────────────────────────────────────────────────

struct BossSeedLibrarySchema;

impl ContentSchemaHandler for BossSeedLibrarySchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let library = match SeedLibrary::from_ron(facet.text) {
            Ok(library) => library,
            Err(error) => {
                out.report(facet.diagnostic(code_for(&error), format!("{error}")));
                return;
            }
        };

        if library.is_empty() {
            out.report(facet.diagnostic(
                DiagnosticCode::MalformedSource,
                "the seed library is empty, so every boss attack is unclassified",
            ));
        }

        // An attack key claimed by two seeds means the roster's classification is
        // ambiguous — `every_shipped_boss_attack_key_belongs_to_exactly_one_seed`
        // is the oracle for that, and this is the half of it that needs only ONE
        // source to see.
        let mut claimed: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();

        for (seed_id, seed) in library.iter() {
            let id = facet.content_id_in(BOSS_SEED_SCHEMA, seed_id);
            out.define(id.clone(), canonical(seed));

            // BD5 rule 2: a seed with no fair counter describes an attack the
            // player cannot answer, which is the definition of unfair.
            if seed.fair_counters.is_empty() {
                out.report(
                    facet
                        .diagnostic(
                            DiagnosticCode::MalformedProviderBinding,
                            format!("seed `{seed_id}` declares no fair counter"),
                        )
                        .about(id.clone())
                        .at_field("fair_counters")
                        .fix(
                            "name the movement verbs that ANSWER it — a seed with no counter is \
                             an attack the player has no move against",
                        ),
                );
            }

            if seed.intent.trim().is_empty() {
                out.report(
                    facet
                        .diagnostic(
                            DiagnosticCode::MalformedProviderBinding,
                            format!("seed `{seed_id}` has no written intent"),
                        )
                        .about(id.clone())
                        .at_field("intent"),
                );
            }

            // A band whose min exceeds its max matches NOTHING, so every
            // instance silently falls outside it.
            for (field, band) in [("telegraph", &seed.telegraph), ("active", &seed.active)] {
                if band.min_s > band.max_s {
                    out.report(
                        facet
                            .diagnostic(
                                DiagnosticCode::MalformedProviderBinding,
                                format!(
                                    "seed `{seed_id}`'s `{field}` band is inverted \
                                     ({} > {})",
                                    band.min_s, band.max_s
                                ),
                            )
                            .about(id.clone())
                            .at_field(field)
                            .fix("an inverted band matches nothing, so every instance falls outside it"),
                    );
                }
            }

            for instance in &seed.instances {
                if let Some(first) = claimed.insert(instance.as_str(), seed_id) {
                    out.report(
                        facet
                            .diagnostic(
                                DiagnosticCode::ConflictingModuleContribution,
                                format!(
                                    "attack `{instance}` is claimed by seeds `{first}` and \
                                     `{seed_id}`"
                                ),
                            )
                            .about(id.clone())
                            .at_field("instances")
                            .fix(
                                "an attack belongs to exactly one archetype — the classification \
                                 is what the seed MEANS, so two claims is a contradiction",
                            ),
                    );
                }
            }
        }

        if !out.failed() {
            out.lower(library);
        }
    }
}

// ── the fairness calibration ─────────────────────────────────────────────────

struct BossValidatorBandsSchema;

impl ContentSchemaHandler for BossValidatorBandsSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let bands = match ValidatorBands::from_ron(facet.text) {
            Ok(bands) => bands,
            Err(error) => {
                out.report(facet.diagnostic(code_for(&error), format!("{error}")));
                return;
            }
        };

        let id = facet.content_id_in(BOSS_VALIDATOR_BANDS_SCHEMA, "calibration");
        out.define(id.clone(), canonical(&bands));

        // The bands are expressed in TICKS and the authored fight data in
        // seconds, so `tick_hz` is the conversion. Zero or negative makes every
        // converted duration zero or inverted — and it converts silently.
        if bands.tick_hz <= 0.0 {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::MalformedProviderBinding,
                        format!("`tick_hz` is {}, which cannot convert seconds to ticks", bands.tick_hz),
                    )
                    .about(id)
                    .at_field("tick_hz")
                    .fix("the sim steps at 60 Hz; this is the number the bands are expressed against"),
            );
        }

        if !out.failed() {
            out.lower(bands);
        }
    }
}

// ── lowered accessors — the runtime's load path ──────────────────────────────

pub fn lowered_seed_library(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&SeedLibrary> {
    pack.lowered::<SeedLibrary>(&SchemaId::new(BOSS_SEEDS_SCHEMA))
}

pub fn lowered_validator_bands(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&ValidatorBands> {
    pack.lowered::<ValidatorBands>(&SchemaId::new(BOSS_VALIDATOR_BANDS_SCHEMA))
}

// ── registrations ────────────────────────────────────────────────────────────

pub fn boss_seed_library_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(BOSS_SEEDS_SCHEMA),
        version: BOSS_SEEDS_VERSION,
        capability: CapabilityId::new(BOSS_PATTERN_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "The boss attack archetypes, each with its design intent, fair counters and \
              measured duration bands. Defines `boss_seed` identities.",
        handler: Arc::new(BossSeedLibrarySchema),
    }
}

pub fn boss_validator_bands_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(BOSS_VALIDATOR_BANDS_SCHEMA),
        version: BOSS_VALIDATOR_BANDS_VERSION,
        capability: CapabilityId::new(BOSS_PATTERN_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "One game's boss-fairness calibration: the tick rate and the per-threat \
              telegraph/recovery bands the fight validator judges against.",
        handler: Arc::new(BossValidatorBandsSchema),
    }
}

#[cfg(test)]
mod tests;
