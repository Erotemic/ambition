//! The combat capability's authored-content SCHEMA registration.
//!
//! `character_archetypes.ron` — the enemy roster — is a `{ "<brain_key>":
//! CharacterArchetypeSpec }` table, and this crate owns that type
//! ([`crate::archetype_spec`]), so it owns the schema.
//!
//! ⛔ **the type moved here on 2026-08-03 for exactly this reason.** It was
//! `pub(crate)` inside `ambition_platformer2d_actor_monolith`, which meant the
//! content validator would have had to link 708 crates and a renderer to
//! validate the family. Every field is combat/movement tuning; the coupling was
//! locational.
//!
//! ## The key IS the spawn brain key
//!
//! Each top-level key is what a `LoadingZone` or an encounter authors as
//! `Brain::Custom("…")`. That makes the key the identity, and it is why the
//! reserved `"combatant"` fallback row matters: an unknown brain key resolves to
//! it rather than failing, so a typo'd key is a SILENT downgrade to the generic
//! archetype. The schema cannot see the LDtk side, but it can insist the
//! fallback exists — without it the downgrade has nothing to land on.

use std::sync::Arc;

use ambition_content_pack::{
    CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource,
    RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};

use crate::archetype_spec::CharacterArchetypeSpec;

/// The capability that owns this schema.
pub const COMBAT_CAPABILITY: &str = "combat";

pub const CHARACTER_ARCHETYPES_SCHEMA: &str = "character_archetypes";
/// One authored archetype.
pub const ARCHETYPE_SCHEMA: &str = "character_archetype";

pub const CHARACTER_ARCHETYPES_VERSION: SchemaVersion = SchemaVersion(1);

/// The reserved fallback row every unknown brain key resolves to.
const FALLBACK_BRAIN_KEY: &str = "combatant";

/// The parsed roster: `{ "<brain_key>": CharacterArchetypeSpec }`.
pub type CharacterArchetypes = std::collections::BTreeMap<String, CharacterArchetypeSpec>;

struct CharacterArchetypesSchema;

impl ContentSchemaHandler for CharacterArchetypesSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let archetypes: CharacterArchetypes = match ron::from_str(facet.text) {
            Ok(archetypes) => archetypes,
            Err(error) => {
                let code = match error.code {
                    ron::error::Error::NoSuchStructField { .. } => DiagnosticCode::UnknownField,
                    _ => DiagnosticCode::MalformedSource,
                };
                out.report(facet.diagnostic(code, format!("{error}")));
                return;
            }
        };

        // ⚠ the fallback is what makes an unknown brain key a downgrade rather
        // than a crash. Without it there is nothing to downgrade TO.
        if !archetypes.contains_key(FALLBACK_BRAIN_KEY) {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::UnresolvedReference,
                        format!(
                            "the roster has no `{FALLBACK_BRAIN_KEY}` row — it is the reserved \
                             fallback every unknown spawn brain key resolves to"
                        ),
                    )
                    .fix("add a `combatant` row, or point the provider's fallback at another key"),
            );
        }

        for (key, spec) in &archetypes {
            let id = facet.content_id_in(ARCHETYPE_SCHEMA, key);
            out.define(id.clone(), canonical(spec));

            // ── the inheritance edge, as a real reference ────────────────────
            // Provider-local by design: an unqualified parent must be owned by
            // the same provider. A dangling parent used to fall back to the
            // baseline silently, which reads as "my inheritance did nothing".
            if let Some(parent) = &spec.inherits {
                if !archetypes.contains_key(parent) {
                    out.report(
                        facet
                            .diagnostic(
                                DiagnosticCode::UnresolvedReference,
                                format!("archetype `{key}` inherits `{parent}`, which this roster does not define"),
                            )
                            .about(id.clone())
                            .at_field("inherits")
                            .fix("inheritance is provider-local: name a row in THIS file, or drop the field to inherit the baseline"),
                    );
                }
                if parent == key {
                    out.report(
                        facet
                            .diagnostic(
                                DiagnosticCode::ConflictingModuleContribution,
                                format!("archetype `{key}` inherits itself"),
                            )
                            .about(id.clone())
                            .at_field("inherits"),
                    );
                }
            }

            // ── §4.7: effort is a FRACTION of run_speed, never a speed ───────
            // An effort above 1.0 asks the body to exceed the only absolute
            // speed it authors, and the seam silently clamps — so the archetype
            // reads as "tuned faster" and behaves identically.
            for (field, effort) in [
                ("patrol_effort", spec.patrol_effort),
                ("chase_effort", spec.chase_effort),
            ] {
                if !(0.0..=1.0).contains(&effort) {
                    out.report(
                        facet
                            .diagnostic(
                                DiagnosticCode::MalformedProviderBinding,
                                format!(
                                    "archetype `{key}` has `{field}: {effort}` — effort is a \
                                     fraction of `run_speed` in 0.0..=1.0, not a speed"
                                ),
                            )
                            .about(id.clone())
                            .at_field(field)
                            .fix(
                                "raise `run_speed` if the body should be faster; effort is how \
                                 hard the brain pushes it",
                            ),
                    );
                }
            }

            if spec.run_speed < 0.0 {
                out.report(
                    facet
                        .diagnostic(
                            DiagnosticCode::MalformedProviderBinding,
                            format!("archetype `{key}` has a negative `run_speed`"),
                        )
                        .about(id.clone())
                        .at_field("run_speed"),
                );
            }

            if spec.max_health <= 0 {
                out.report(
                    facet
                        .diagnostic(
                            DiagnosticCode::MalformedProviderBinding,
                            format!(
                                "archetype `{key}` has `max_health: {}` — it dies on spawn",
                                spec.max_health
                            ),
                        )
                        .about(id.clone())
                        .at_field("max_health"),
                );
            }
        }

        if !out.failed() {
            out.lower(archetypes);
        }
    }
}

/// Semantic canonical form. `Debug`, because these types are
/// `Deserialize`-only — see the note on the boss schemas for why that is the
/// right trade rather than adding `Serialize` to hash them.
fn canonical<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

/// The roster a prepared pack lowered to — the runtime's load path.
pub fn lowered_character_archetypes(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&CharacterArchetypes> {
    pack.lowered::<CharacterArchetypes>(&SchemaId::new(CHARACTER_ARCHETYPES_SCHEMA))
}

pub fn character_archetypes_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(CHARACTER_ARCHETYPES_SCHEMA),
        version: CHARACTER_ARCHETYPES_VERSION,
        capability: CapabilityId::new(COMBAT_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "The hostile-archetype roster: how each spawn brain key fights. Keys are the \
              spawn brain keys; `combatant` is the reserved fallback. Defines \
              `character_archetype` identities.",
        handler: Arc::new(CharacterArchetypesSchema),
    }
}
