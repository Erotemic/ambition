//! The combat capability's authored-content SCHEMA registration.
//!
//! `character_archetypes.ron` — the enemy roster — is a `{ "<brain_key>":
//! ArchetypeSpec }` table, and this crate owns that type
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

use crate::archetype_spec::ArchetypeSpec;

/// The capability that owns this schema.
pub const COMBAT_CAPABILITY: &str = "combat";

pub const CHARACTER_ARCHETYPES_SCHEMA: &str = "character_archetypes";
/// One authored archetype.
pub const ARCHETYPE_SCHEMA: &str = "character_archetype";

pub const CHARACTER_ARCHETYPES_VERSION: SchemaVersion = SchemaVersion(1);

/// The reserved fallback row every unknown brain key resolves to.
const FALLBACK_BRAIN_KEY: &str = "combatant";

/// The parsed roster: `{ "<brain_key>": ArchetypeSpec }`.
pub type Archetypes = std::collections::BTreeMap<String, ArchetypeSpec>;

struct ArchetypesSchema;

impl ContentSchemaHandler for ArchetypesSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let archetypes: Archetypes = match ron::from_str(facet.text) {
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

        // ⛔ **THE COMPILER MUST NOT APPROVE WHAT THE RUNTIME REFUSES.** The
        // roster assembly rejects a blank brain key (`EmptyBrainId`) and any
        // inheritance CYCLE (`MovementInheritanceCycle`, a full DFS). Checking
        // only missing parents and direct self-inheritance here left `a inherits
        // b, b inherits a` compiling clean and then panicking at startup —
        // a validator that says yes to content the game says no to is worse than
        // no validator, because it moves the failure to the worst place.
        // (GPT 5.6 review, finding 3.)
        report_inheritance_cycles(facet, &archetypes, out);

        for (key, spec) in &archetypes {
            let id = facet.content_id_in(ARCHETYPE_SCHEMA, key);
            out.define(id.clone(), canonical(spec));

            if key.trim().is_empty() {
                out.report(
                    facet
                        .diagnostic(
                            DiagnosticCode::MalformedProviderBinding,
                            "an archetype key is blank — nothing can spawn it, and the roster \
                             assembly refuses it at startup",
                        )
                        .about(id.clone())
                        .fix("the key IS the spawn brain key a LoadingZone authors"),
                );
            }

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

/// Report every inheritance CYCLE in the roster, naming the loop.
///
/// The runtime's `resolve_movement_inheritance` does the same walk and returns
/// `MovementInheritanceCycle`; this is that check moved to where it can be seen
/// without booting a game. Reported once per cycle rather than once per member,
/// so a two-row loop is one diagnostic and not two saying the same thing.
fn report_inheritance_cycles(
    facet: &FacetSource<'_>,
    archetypes: &Archetypes,
    out: &mut FacetOutcome,
) {
    let mut settled: std::collections::BTreeSet<&str> = Default::default();
    let mut reported: std::collections::BTreeSet<String> = Default::default();

    for start in archetypes.keys() {
        if settled.contains(start.as_str()) {
            continue;
        }
        // Walk the parent chain from `start`, remembering the path so a loop can
        // report the ring it actually closes rather than just "a cycle exists".
        let mut chain: Vec<&str> = Vec::new();
        let mut cursor = start.as_str();
        loop {
            if let Some(at) = chain.iter().position(|seen| *seen == cursor) {
                let ring: Vec<&str> = chain[at..].to_vec();
                // Canonical spelling of the ring (rotate to its smallest member)
                // so the same loop found from two entry points reports once.
                let pivot = ring
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, name)| **name)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let mut rotated: Vec<&str> = ring[pivot..].to_vec();
                rotated.extend_from_slice(&ring[..pivot]);
                let key = rotated.join(" -> ");
                if reported.insert(key.clone()) {
                    out.report(
                        facet
                            .diagnostic(
                                DiagnosticCode::ConflictingModuleContribution,
                                format!(
                                    "archetype inheritance forms a cycle: {key} -> {}",
                                    rotated[0]
                                ),
                            )
                            .about(facet.content_id_in(ARCHETYPE_SCHEMA, rotated[0]))
                            .at_field("inherits")
                            .fix(
                                "inheritance folds BASELINE <- parent <- this row, so a loop has \
                                 no baseline to start from — break the ring",
                            ),
                    );
                }
                break;
            }
            chain.push(cursor);
            match archetypes
                .get(cursor)
                .and_then(|spec| spec.inherits.as_deref())
            {
                // A missing parent is already reported as an unresolved
                // reference; stop rather than reporting it twice.
                Some(parent) if archetypes.contains_key(parent) => cursor = parent,
                _ => break,
            }
        }
        settled.extend(chain);
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
) -> Option<&Archetypes> {
    pack.lowered::<Archetypes>(&SchemaId::new(CHARACTER_ARCHETYPES_SCHEMA))
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
        handler: Arc::new(ArchetypesSchema),
    }
}

#[cfg(test)]
mod tests;
