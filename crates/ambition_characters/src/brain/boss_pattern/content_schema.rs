//! The boss-pattern capability's authored-content SCHEMA registrations.
//!
//! Three families, all owned here because the types they parse into are owned
//! here: the boss ROSTER (`boss_profiles.ron`), the SEED LIBRARY
//! (`boss_seeds.ron`) and the fairness CALIBRATION (`boss_validator_bands.ron`).
//!
//! ## The roster only became ownable when its type moved
//!
//! ⛔ `BossBehaviorProfile` lived in `ambition_platformer2d_actor_monolith` until
//! 2026-08-03, and that BLOCKED this schema. A schema must be registered by the
//! crate owning its type, and the validator has to link that crate — so a
//! boss-profile schema meant the CLI linking the monolith: **708 crates against
//! its 239, and a renderer**, destroying the property that justifies the
//! compiler at all.
//!
//! The fix was not a workaround, it was the placement being wrong: nothing in
//! that vocabulary ever needed the actor crate (`cargo check -p
//! ambition_characters` passed the moment it moved, unchanged). It now lives in
//! [`super::profile`], the actor crate re-exports it, and the `BossCatalog`
//! lookups became `BossBehaviorProfileExt` there because the orphan rule does
//! not let an inherent `impl` follow a type across a crate boundary.
//!
//! ⚠ `PickupKind` moved DOWN to `ambition_entity_catalog` in the same change —
//! `BossRewardProfile` names it and `ambition_interaction` depends on THIS
//! crate, so it was a cycle.

use std::sync::Arc;

use ambition_content_pack::{
    CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource,
    RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};

use super::profile::BossBehaviorProfile;
use super::seeds::SeedLibrary;
use super::validator::ValidatorBands;

/// The capability that owns both schemas here.
pub const BOSS_PATTERN_CAPABILITY: &str = "boss_pattern";

pub const BOSS_SEEDS_SCHEMA: &str = "boss_seed_library";
/// One attack archetype in the library.
pub const BOSS_SEED_SCHEMA: &str = "boss_seed";
pub const BOSS_VALIDATOR_BANDS_SCHEMA: &str = "boss_validator_bands";
pub const BOSS_PROFILES_SCHEMA: &str = "boss_profiles";
/// One authored boss.
pub const BOSS_SCHEMA: &str = "boss";

pub const BOSS_SEEDS_VERSION: SchemaVersion = SchemaVersion(1);
pub const BOSS_VALIDATOR_BANDS_VERSION: SchemaVersion = SchemaVersion(1);
pub const BOSS_PROFILES_VERSION: SchemaVersion = SchemaVersion(1);

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

// ── the boss roster ──────────────────────────────────────────────────────────

/// The parsed boss roster: `{ "<boss_id>": BossBehaviorProfile }`.
pub type BossProfiles = std::collections::BTreeMap<String, BossBehaviorProfile>;

struct BossProfilesSchema;

impl ContentSchemaHandler for BossProfilesSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let profiles: BossProfiles = match ron::from_str(facet.text) {
            Ok(profiles) => profiles,
            Err(error) => {
                out.report(facet.diagnostic(code_for(&error), format!("{error}")));
                return;
            }
        };

        if profiles.is_empty() {
            out.report(facet.diagnostic(
                DiagnosticCode::MalformedSource,
                "the boss roster is empty, so every authored boss falls back to a generic clone",
            ));
        }

        for (key, profile) in &profiles {
            let id = facet.content_id_in(BOSS_SCHEMA, key);
            out.define(id.clone(), canonical(profile));

            // ⛔ **the key IS the lookup, and the row states its own id.** Every
            // runtime path resolves a boss by MAP KEY (`catalog.behavior(key)`)
            // and then reads `profile.id` for its sheet target, its music, its
            // bark pool. When the two disagree the boss is looked up under one
            // name and draws, sounds and speaks as another — with no error
            // anywhere, because each half is individually valid.
            if profile.id != *key {
                out.report(
                    facet
                        .diagnostic(
                            DiagnosticCode::ConflictingModuleContribution,
                            format!(
                                "boss row `{key}` declares `id: \"{}\"` — the key is what the \
                                 runtime looks up, the id is what it then draws and sounds as",
                                profile.id
                            ),
                        )
                        .about(id.clone())
                        .at_field("id")
                        .fix("make `id` equal the row's key, or move the row to the key it names"),
                );
            }

            // A non-positive strike scale freezes the boss mid-attack: the
            // moveset bake turns it into the Active window's motion_scale.
            if profile.strike_speed_scale <= 0.0 {
                out.report(
                    facet
                        .diagnostic(
                            DiagnosticCode::MalformedProviderBinding,
                            format!(
                                "boss `{key}` has `strike_speed_scale: {}` — at or below zero the \
                                 boss cannot move during a strike at all",
                                profile.strike_speed_scale
                            ),
                        )
                        .about(id.clone())
                        .at_field("strike_speed_scale")
                        .fix(
                            "`< 1.0` anchors the boss so its telegraph stays over its hitbox; \
                              `1.0` leaves steering untouched",
                        ),
                );
            }

            for (field, value) in [
                ("attack_cooldown", profile.attack_cooldown),
                ("attack_windup", profile.attack_windup),
                ("attack_active", profile.attack_active),
            ] {
                if value < 0.0 {
                    out.report(
                        facet
                            .diagnostic(
                                DiagnosticCode::MalformedProviderBinding,
                                format!("boss `{key}` has a negative `{field}` ({value})"),
                            )
                            .about(id.clone())
                            .at_field(field),
                    );
                }
            }
        }

        if !out.failed() {
            out.lower(profiles);
        }
    }
}

/// The boss roster a prepared pack lowered to — the runtime's load path.
pub fn lowered_boss_profiles(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&BossProfiles> {
    pack.lowered::<BossProfiles>(&SchemaId::new(BOSS_PROFILES_SCHEMA))
}

pub fn boss_profiles_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(BOSS_PROFILES_SCHEMA),
        version: BOSS_PROFILES_VERSION,
        capability: CapabilityId::new(BOSS_PATTERN_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "The boss roster: movement, attacks, damage, hitbox and reward tuning, one row \
              per boss. Defines `boss` identities.",
        handler: Arc::new(BossProfilesSchema),
    }
}

#[cfg(test)]
mod tests;
