//! The boss-pattern capability's authored-content SCHEMA registrations.
//!
//! Three families, all owned here because the types they parse into are owned
//! here: the boss ROSTER (`boss_profiles.ron`), the SEED LIBRARY
//! (`boss_seeds.ron`) and the fairness CALIBRATION (`boss_validator_bands.ron`).
//!
//! ## The roster only became ownable when its type moved
//!
//! A schema must be registered by the crate owning its type, and the validator has to link that
//! crate — so a boss-profile schema meant the CLI linking the monolith: 708 crates against its
//! 239, and a renderer, destroying the property that justifies the compiler at all.
//!
//! It now lives in [`super::profile`], the actor crate re-exports it, and the `BossCatalog`
//! lookups became `BossBehaviorProfileExt` there because the orphan rule does not let an
//! inherent `impl` follow a type across a crate boundary.
//!
//! `PickupKind` moved DOWN to `ambition_entity_catalog` in the same change —
//! `BossRewardProfile` names it and `ambition_interaction` depends on THIS
//! crate, so it was a cycle.

use std::sync::Arc;

use ambition_content_pack::{
    AggregateOutcome, Aggregation, CapabilityId, ContentSchemaHandler, DiagnosticCode,
    FacetOutcome, FacetSource, LoweredFragment, PendingRef, RuntimeDisposition, SchemaId,
    SchemaRegistration, SchemaVersion,
};

use crate::pattern::profile::BossBehaviorProfile;
use crate::pattern::seeds::SeedLibrary;
use crate::pattern::validator::ValidatorBands;
use ambition_characters::boss_encounter::BossEncounterSpec;

/// The capability that owns both schemas here.
pub const BOSS_PATTERN_CAPABILITY: &str = "boss_pattern";

pub const BOSS_SEEDS_SCHEMA: &str = "boss_seed_library";
/// One attack archetype in the library.
pub const BOSS_SEED_SCHEMA: &str = "boss_seed";
pub const BOSS_VALIDATOR_BANDS_SCHEMA: &str = "boss_validator_bands";
pub const BOSS_PROFILES_SCHEMA: &str = "boss_profiles";
/// One authored boss.
pub const BOSS_SCHEMA: &str = "boss";
pub const BOSS_ENCOUNTER_SCHEMA: &str = "boss_encounter";

pub const BOSS_SEEDS_VERSION: SchemaVersion = SchemaVersion(1);
pub const BOSS_VALIDATOR_BANDS_VERSION: SchemaVersion = SchemaVersion(1);
pub const BOSS_PROFILES_VERSION: SchemaVersion = SchemaVersion(1);
pub const BOSS_ENCOUNTER_VERSION: SchemaVersion = SchemaVersion(1);

/// The canonical form an entry contributes to the pack fingerprint.
///
/// `Debug`, not `ron::ser`, because these types are `Deserialize`-only — they
/// are read from authored RON and never written back. Debug is derived on all of
/// them and moves when a value moves.
///
/// ⛔ **Debug gives stable FIELD order and follows ITERATION order, so every
/// container reaching this must be ordered.** A `HashMap` here randomises per
/// instance (measured: six constructions of one four-key map, six different
/// orders, same process), so two identical rosters fingerprint differently the
/// moment a boss authors a second strike override. `BTreeMap` everywhere;
/// `the_canonical_form_does_not_depend_on_map_construction_order` is the guard.
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

// ── one boss encounter ───────────────────────────────────────────────────────

/// One encounter file, of nine — the family the aggregation contract was built
/// for.
///
/// the schema now says how they combine ([`Self::aggregate`]): each file
/// lowers one [`BossEncounterSpec`], and the merge is the
/// `BTreeMap<String, BossEncounterSpec>` the boss catalog already holds. The
/// compiler's copy IS the runtime's copy.
struct BossEncounterSchema;

/// What the nine encounter files lower to, together: the catalog's own map.
pub type BossEncounterBook = std::collections::BTreeMap<String, BossEncounterSpec>;

impl ContentSchemaHandler for BossEncounterSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let spec: BossEncounterSpec = match ron::from_str(facet.text) {
            Ok(spec) => spec,
            Err(error) => {
                out.report(facet.diagnostic(code_for(&error), format!("{error}")));
                return;
            }
        };

        let id = facet.content_id_in(BOSS_ENCOUNTER_SCHEMA, &spec.id);
        out.define(id.clone(), canonical(&spec));

        // Every encounter needs the behaviour row it drives. The runtime says
        // so too (`MissingBehavior`) — after the game has started.
        out.refer(PendingRef::new(
            SchemaId::new(BOSS_SCHEMA),
            &spec.id,
            "boss profile",
            id.clone(),
            "id",
        ));

        // AND ITS MUSIC, because both sides are in the pack now. These four fields name
        // `music_registry` tracks. Empty means "no swap for this phase" and is not a reference.
        for (field, track) in [
            ("music_intro", &spec.music_intro),
            ("music_phase1", &spec.music_phase1),
            ("music_phase2", &spec.music_phase2),
            ("music_enrage", &spec.music_enrage),
        ] {
            // EXACTLY empty, not `trim().is_empty()`. `phase_music`
            // gates on `!track.is_empty()`, so `"   "` is a REAL music request
            // at runtime — one that matches no track and silently falls through
            // to another candidate. Skipping it here (and in the startup
            // validator) meant both validators accepted a value the runtime
            // acts on, which is the same compiler-vs-runtime rule mismatch as
            // the padded case one line below, in its emptiness predicate.
            // Whitespace-only now becomes an unresolved exact reference and is
            // refused.
            if track.is_empty() {
                continue;
            }
            // Resolve what the runtime will actually ask for.
            out.refer(PendingRef::new(
                SchemaId::new("music_track"),
                track.as_str(),
                "music track",
                id.clone(),
                field,
            ));
        }

        // The fragment: ONE encounter, which `aggregate` keys into the book.
        out.lower(spec);
    }

    fn aggregate(
        &self,
        fragments: &[LoweredFragment<'_>],
        out: &mut AggregateOutcome,
    ) -> Aggregation {
        let mut book = BossEncounterBook::new();
        let mut source_of: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        for fragment in fragments {
            let Some(spec) = fragment.get::<BossEncounterSpec>() else {
                continue;
            };
            // checked here even though `define` already makes two encounter
            // files with one id a `DuplicateIdentity`. The merge must not
            // depend on another stage having caught it: a silent `insert` that
            // returns `Some` is exactly the last-wins the compiler refuses, and
            // it would be one refactor of `check` away from being the only
            // reader of that fact. `BossCatalogFragment` makes the same check
            // for the same reason.
            if let Some(first) = source_of.get(&spec.id) {
                out.report(
                    AggregateOutcome::refusal(
                        DiagnosticCode::ConflictingModuleContribution,
                        format!(
                            "encounter `{}` is defined by `{first}` and by `{}`",
                            spec.id, fragment.declared_path
                        ),
                    )
                    .in_source(fragment.declared_path)
                    .fix("one encounter id, one file — the runtime looks the id up ONCE"),
                );
                continue;
            }
            source_of.insert(spec.id.clone(), fragment.declared_path.to_string());
            book.insert(spec.id.clone(), spec.clone());
        }
        if !out.failed() {
            out.lower(book);
        }
        Aggregation::Defined
    }
}

/// The encounter book a prepared pack lowered, if it carries one — the runtime's
/// load path, replacing nine `ron::from_str` calls in the catalog builder.
pub fn lowered_boss_encounters(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&BossEncounterBook> {
    pack.lowered::<BossEncounterBook>(&SchemaId::new(BOSS_ENCOUNTER_SCHEMA))
}

pub fn boss_encounter_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(BOSS_ENCOUNTER_SCHEMA),
        version: BOSS_ENCOUNTER_VERSION,
        capability: CapabilityId::new(BOSS_PATTERN_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "One boss encounter: phase progression and HP thresholds. Defines a \
              `boss_encounter` identity and requires the `boss` profile of the same id. \
              Every such file merges into one encounter book.",
        handler: Arc::new(BossEncounterSchema),
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

            // The other half of the correspondence the runtime enforces: a
            // profile with no encounter is `MissingEncounter` at startup, behind
            // an `.expect`. Resolved here, across sources, before the game runs.
            out.refer(PendingRef::new(
                SchemaId::new(BOSS_ENCOUNTER_SCHEMA),
                key,
                "boss encounter",
                id.clone(),
                "id",
            ));

            // the key IS the lookup, and the row states its own id. Every
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
