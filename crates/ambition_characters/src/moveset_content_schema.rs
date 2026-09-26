//! The `moveset` authored-content schema — fast-iteration I2, step 4.
//!
//! ⭐⭐ **THE HOST'S HALF OF THE ARTIFACT ROAD, AND IT IS THE ROAD THIS GAME
//! ALREADY HAS.** I2 step 4 asks for *"a selected-host load path through the
//! current source/resolver policy"*. That policy is `pack.ron` plus
//! [`ambition_content_pack::compile`] — *"the compile that IS the load path"* —
//! and eight content families ship on it. A move table becomes the ninth by
//! registering a schema, not by growing a second loader.
//!
//! ⛔⛔ **AND THE CODEC AND VALIDATOR ARE NOT MINE TO WRITE.**
//! [`EntityCatalogDoc`] already is the authored shape of a move table: a
//! versioned document with `parse`/`to_ron` and a `validate()` that returns
//! fifteen structural refusals — duplicate move ids, windows outside
//! `[0, duration_s]`, a smash charge that freezes the clock where a strike is
//! already live, volumes on an inactive window, an unknown cancel target, and
//! `UnknownVerbMove`: a verb bound to a move that does not exist. MEASURED
//! 2026-09-11: it had ZERO production consumers, so a name-based search for a
//! "move codec" could not find it — a road nothing calls does not appear in the
//! grep you run to find callers. This handler is what gives it one.
//!
//! ⚠ **STRUCTURE IS NOT ADMISSION.** Whether this composition installed the
//! techniques a move REFERENCES is a different question, answered where it has
//! always been answered — `crate::prepared::unsupported_authored_effects`, at
//! the preparation barrier, with the site. Asking it twice would be the second
//! validator I2's own Stop clause forbids.
//!
//! ⛔ **ONE FILE PER CHARACTER, MERGED.** Same shape as
//! [`super::smash_fighter::content_schema`] and for the same reason: the runtime
//! looks a character up by id and must find exactly ONE answer, so two files
//! claiming one character is a refusal rather than a map-iteration-order winner.

use std::collections::BTreeMap;
use std::sync::Arc;

use ambition_content_pack::{
    AggregateOutcome, Aggregation, CapabilityId, ContentSchemaHandler, DiagnosticCode,
    FacetOutcome, FacetSource, LoweredFragment, RuntimeDisposition, SchemaId, SchemaRegistration,
    SchemaVersion,
};
use ambition_entity_catalog::move_section::MoveSectionData;
use ambition_entity_catalog::{
    EntityCatalogDoc, EntityDef, MovesetBorrow, MovesetContract,
};

use crate::actor::character_catalog::content_schema::CHARACTERS_CAPABILITY;

/// The authored FILE kind: one or more entities' move contracts.
pub const MOVESET_SCHEMA: &str = "moveset";

/// The schema version this handler reads.
///
/// ⛔ IT IS ALSO THE DOCUMENT'S OWN `schema_version`, CHECKED HERE. That field
/// was written in thirteen fixtures and read by NOTHING — a version nobody
/// compares cannot refuse anything, which is precisely the silent misread the
/// artifact envelope's `SectionTooNew` exists to prevent. One number, one
/// comparison, one place.
pub const MOVESET_VERSION: SchemaVersion =
    SchemaVersion(ambition_entity_catalog::ENTITY_CATALOG_SCHEMA_VERSION);

/// What one file contributes before the merge.
///
/// ⚠ JUST THE DOCUMENT. It carried its `declared_path` too, for a collision
/// message the compiler turned out to write better and earlier — see
/// [`MovesetSchema::aggregate`].
#[derive(Debug, Clone)]
struct Fragment {
    doc: EntityCatalogDoc,
}

struct MovesetSchema;

impl ContentSchemaHandler for MovesetSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let doc = match EntityCatalogDoc::parse(facet.text) {
            Ok(doc) => doc,
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

        if doc.schema_version != MOVESET_VERSION.0 {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::SchemaVersionMismatch,
                        format!(
                            "this document declares schema_version {} and this host reads {}",
                            doc.schema_version, MOVESET_VERSION.0
                        ),
                    )
                    .fix(
                        "migrate the document — a reader that assumes a different version reads \
                         different fields, so there is no safe fallback",
                    ),
            );
            return;
        }

        // ⛔ A FILE THAT AUTHORS NO MOVE TABLE. It parses, it validates, and it
        // contributes nothing — which reads downstream as "this character has no
        // moves" rather than as "somebody declared the wrong file".
        if !doc
            .entities
            .iter()
            .any(|entity| {
                entity.contracts.moveset.is_some()
                    || entity.contracts.borrows.is_some()
                    || !entity.contracts.takes.is_empty()
            })
        {
            out.report(
                facet
                    .diagnostic(
                        DiagnosticCode::MalformedProviderBinding,
                        format!(
                            "`{}` declares the `{MOVESET_SCHEMA}` schema and carries no move \
                             contract for any of its {} entit(ies)",
                            facet.declared_path,
                            doc.entities.len()
                        ),
                    )
                    .fix("give an entity a `moveset` contract, or drop the source declaration"),
            );
        }

        for entity in &doc.entities {
            // ⛔ A SLOT THIS SCHEMA DOES NOT READ. The entity document also has
            // `body`, `hurtboxes` and `presentation` slots, and the move lowering
            // reads none of them, so a value there is dropped with no error: it
            // looks authored and nothing happens. A character's hurtboxes and
            // body are its catalog row's to state.
            let unread = [
                ("body", entity.contracts.body.is_some()),
                ("hurtboxes", entity.contracts.hurtboxes.is_some()),
                ("presentation", entity.contracts.presentation.is_some()),
            ];
            for (slot, stated) in unread {
                if stated {
                    out.report(
                        facet
                            .diagnostic(
                                DiagnosticCode::MalformedSource,
                                format!(
                                    "`{}` states `{slot}` for `{}`, and a move file's `{slot}` is \
                                     read by nothing",
                                    facet.declared_path, entity.id
                                ),
                            )
                            .at_field(slot)
                            .fix("state it in the character's catalog row, or delete it"),
                    );
                }
            }
            let id = facet.content_id_in(MOVESET_SCHEMA, entity.id.clone());
            // A borrow or a take is part of what the entity says, so it moves
            // the fingerprint. A table that borrows and takes nothing keeps its
            // old one.
            let identity = match (&entity.contracts.borrows, entity.contracts.takes.is_empty()) {
                (None, true) => canonical(&entity.contracts.moveset),
                (Some(borrow), true) => canonical(&(borrow, &entity.contracts.moveset)),
                (borrow, false) => canonical(&(
                    borrow,
                    &entity.contracts.takes,
                    &entity.contracts.moveset,
                )),
            };
            out.define(id.clone(), identity);
        }

        // ⭐ EVERY STRUCTURAL FAULT AT ONCE, FROM THE VALIDATOR THAT ALREADY
        // OWNS THEM. The alternative to reporting an `UnknownVerbMove` here is a
        // press that plays nothing, which reads in a playtest as "the button is
        // broken" rather than as a line number.
        // A borrower's or a taker's table is incomplete until `aggregate`
        // resolves it against the other tables, so it is validated there, whole.
        let standalone = EntityCatalogDoc {
            schema_version: doc.schema_version,
            entities: doc
                .entities
                .iter()
                .filter(|entity| {
                    entity.contracts.borrows.is_none() && entity.contracts.takes.is_empty()
                })
                .cloned()
                .collect(),
        };
        for problem in standalone.validate() {
            out.report(
                facet
                    .diagnostic(DiagnosticCode::MalformedProviderBinding, problem.to_string())
                    .fix(
                        "a move table's windows lie inside the move, its verbs name moves it \
                         defines, and its cancel targets exist",
                    ),
            );
        }

        if !out.failed() {
            out.lower(Fragment { doc });
        }
    }

    /// ⭐⭐ **A STRAIGHT MERGE, AND THE COLLISION REFUSAL I WROTE HERE IS
    /// DELETED — THE POISON IS WHAT FOUND IT.** I copied
    /// [`super::smash_fighter::content_schema`]'s "two files claiming one
    /// character is a refusal, not last-wins" arm, and removing it left
    /// `two_files_claiming_one_character_are_refused` GREEN. MEASURED: the
    /// compiler's own CONFLICT DETECTION stage already refuses it, before
    /// aggregation ever runs, because both files `define` the id
    /// `test:moveset/test_duelist` — and it names both source paths, which my
    /// version did not do as well:
    ///
    /// ```text
    /// [duplicate-identity] `test:moveset/test_duelist` is defined twice:
    ///     in `moves/duelist.ron` and in `moves/duelist_copy.ron` <!-- cite-ok: an illustrative path, never a real one -->
    /// ```
    ///
    /// ⛔ A HANDLER THAT DEFINES A CONTENT ID PER ENTITY GETS THE COLLISION
    /// REFUSAL FOR FREE. Writing a second one is unreachable code that reads
    /// like the thing enforcing the rule.
    fn aggregate(
        &self,
        fragments: &[LoweredFragment<'_>],
        out: &mut AggregateOutcome,
    ) -> Aggregation {
        let mut table: MoveSectionData = BTreeMap::new();
        let mut borrowers: Vec<(&str, &EntityDef)> = Vec::new();
        for fragment in fragments {
            let Some(Fragment { doc }) = fragment.get::<Fragment>() else {
                continue;
            };
            for entity in &doc.entities {
                let derived =
                    entity.contracts.borrows.is_some() || !entity.contracts.takes.is_empty();
                match (derived, entity.contracts.moveset.as_ref()) {
                    (true, _) => borrowers.push((fragment.declared_path, entity)),
                    (false, Some(moveset)) => {
                        table.insert(entity.id.clone(), moveset.clone());
                    }
                    (false, None) => {}
                }
            }
        }
        // Borrowers and takers resolve after every authored table is merged.
        // The source must author its own table: a chain would make one
        // fighter's numbers depend on the order files are read.
        let mut resolved = Vec::with_capacity(borrowers.len());
        for (path, entity) in borrowers {
            let id = entity.id.as_str();
            match resolve_derived(&table, entity) {
                Ok(contract) => resolved.push((id.to_string(), contract)),
                Err(problem) => out.report(
                    AggregateOutcome::refusal(DiagnosticCode::MalformedProviderBinding, problem)
                        .in_source(path)
                        .fix(
                            "name an archetype that authors its own table, and prefixes \
                             that every one of its move ids carries; take moves only from \
                             an entity that authors them, by an id this table does not have",
                        ),
                ),
            }
        }
        table.extend(resolved);
        if !out.failed() {
            out.lower(table);
        }
        Aggregation::Defined
    }
}

/// A derived table, whole: the borrowed archetype's table under the borrower's
/// name (or nothing), the entity's own table laid over it, then each taken move
/// appended, and the result validated like any table.
fn resolve_derived(table: &MoveSectionData, entity: &EntityDef) -> Result<MovesetContract, String> {
    let id = entity.id.as_str();
    let own = entity.contracts.moveset.as_ref();
    let mut contract = match &entity.contracts.borrows {
        Some(borrow) => resolve_borrow(table, id, borrow, own)?,
        None => own.cloned().unwrap_or_default(),
    };
    for take in &entity.contracts.takes {
        let source = table.get(&take.from).ok_or_else(|| {
            format!(
                "`{id}` takes moves from `{}`, which authors no table of its own in this pack",
                take.from
            )
        })?;
        for move_id in &take.moves {
            let mv = source.move_by_id(move_id).ok_or_else(|| {
                format!("`{id}` takes `{move_id}` from `{}`, which has no such move", take.from)
            })?;
            if contract.move_by_id(move_id).is_some() {
                return Err(format!(
                    "`{id}` takes `{move_id}` from `{}` and already has a move with that \
                     id, so one id would name two different moves",
                    take.from
                ));
            }
            contract.moves.push(mv.clone());
        }
    }
    let whole = EntityCatalogDoc {
        schema_version: ambition_entity_catalog::ENTITY_CATALOG_SCHEMA_VERSION,
        entities: vec![EntityDef {
            id: id.to_string(),
            contracts: ambition_entity_catalog::EntityContracts {
                moveset: Some(contract.clone()),
                ..Default::default()
            },
        }],
    };
    let problems: Vec<String> = whole.validate().iter().map(ToString::to_string).collect();
    if !problems.is_empty() {
        return Err(format!(
            "`{id}`'s resolved table is not a valid table: {}",
            problems.join("; ")
        ));
    }
    Ok(contract)
}

/// A borrower's table: its archetype's under the borrower's name, with the
/// borrower's own table laid over it. [`resolve_derived`] validates the whole.
fn resolve_borrow(
    table: &MoveSectionData,
    id: &str,
    borrow: &MovesetBorrow,
    own: Option<&MovesetContract>,
) -> Result<MovesetContract, String> {
    let archetype = table.get(&borrow.archetype).ok_or_else(|| {
        format!(
            "`{id}` borrows the move table of `{}`, which authors no table of its own \
             in this pack",
            borrow.archetype
        )
    })?;
    // No prefixes: the borrower wears the archetype's table as it is, under the
    // archetype's move ids. That is safe only with nothing laid over it. With
    // an overlay, one id would name two different moves, one on each fighter.
    let mut contract = if borrow.prefixes.is_empty() {
        if own.is_some() {
            return Err(format!(
                "`{id}` borrows `{}` with no prefixes and also authors moves of its \
                 own, so one move id would name two different moves; name the \
                 prefixes to rename the table, or remove the own moves",
                borrow.archetype
            ));
        }
        archetype.clone()
    } else {
        archetype
            .clone()
            .under_own_name(&borrow.prefixes, id)
            .map_err(|problem| format!("`{id}` borrows `{}`: {problem}", borrow.archetype))?
    };
    if let Some(own) = own {
        contract = contract.overlaid_with(own);
    }
    Ok(contract)
}

/// The canonical form an entry contributes to the pack fingerprint.
///
/// Round-tripped through RON rather than hashing the authored bytes: reflowing a
/// comment or reordering two fields must NOT move the fingerprint, and changing
/// a timing must. Only the type knows which differences are semantic.
fn canonical<T: serde::Serialize>(value: &T) -> String {
    ron::ser::to_string(value).unwrap_or_else(|error| format!("<uncanonicalizable: {error}>"))
}

/// The runtime's load path: every character's move table this pack carries, or
/// `None` when it authored no moves.
///
/// ⭐ THIS IS WHAT MAKES THE MOVE TABLE CONTENT RATHER THAN CODE. A host reads
/// it and hands it to [`crate::prepared::stage_move_section`]; nothing between
/// the file and the fighter is a compile step.
pub fn lowered_movesets(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&MoveSectionData> {
    pack.lowered::<MoveSectionData>(&SchemaId::new(MOVESET_SCHEMA))
}

/// Every entity whose authored moveset the live cast is playing that `candidate`
/// stops naming.
///
/// ⭐⭐ **"IS THIS DOMAIN SUPPORTED" AND "CAN THIS TRANSITION BE APPLIED" ARE
/// DIFFERENT QUESTIONS, AND ONLY THE FIRST WAS BEING ASKED.** A reload refuses a
/// candidate that changes a domain nothing can publish, by asking whether the
/// changed schema is the participating one. `moveset` IS the participating one —
/// and the participant stages PER ENTITY, over the CANDIDATE's keys. An entity
/// the candidate stops naming is never visited, so nothing removes its authored
/// moveset: the fold is `active.clone()` with the staged entities overwritten,
/// which means the dropped entity's OLD table survives and is REPUBLISHED under
/// the new generation. The pack says one thing and the cast plays another, with
/// no diagnostic anywhere.
///
/// ⇒ This is the containment test that sees it: *every entity whose authored
/// moveset the live cast is currently playing must still be named by the
/// candidate.*
///
/// ⛔ CONTAINMENT, NOT EQUALITY. Adding an entity is legal authoring, and a build
/// that cannot host a new one is already refused by
/// [`crate::prepared::MovesetRevisionError::UnknownCharacter`] — a separate,
/// working refusal this must not duplicate.
///
/// ⛔⛤ **AND IT IS NOT A DIGEST COMPARISON.** `ambition_content_pack::changed_domains`
/// answers "did this domain change" by folding per-source fingerprints; this
/// answers "can the change be applied" and must read the KEYS of the lowered
/// section. Implementing it over digests would report a drop and a retiming
/// identically, which is exactly the collapse that hid this defect.
///
/// ⚠ NO STORED FIELD AND NO PER-DOMAIN COUNTER: both packs already carry the
/// lowered `BTreeMap`, so this is a read, not a new authority. A
/// `pack_generation` / `cast_generation` / `profile_generation` family is the
/// thing the architecture review warned against.
///
/// ⛔⛤ **AND THE MIDDLE CASE NEEDS NO ARM BECAUSE THE COMPILER FORBIDS THE STATE
/// — MEASURED at `8e1e4fb4d` by hitting it with a fixture, and recorded here so
/// the next reader does not spend the attempt.** A source that declares the
/// `moveset` schema and carries NO entities is refused by name: *"declares the
/// `moveset` schema and carries no move contract for any of its 0 entities"*, and
/// removing a table's only entity is refused the same way. So a pack whose
/// section lowers to `Some({})` cannot be built, and a fixture written with
/// `entities: []` panics in its own setup rather than exercising this branch. The
/// `None` road below is reachable only through a MANIFEST that declares no
/// source, which `compile_pack_with` — a source-text rewriter — can never
/// produce.
///
/// The three cases, and the middle one needs no arm of its own:
///
/// * `base` authored no section — nothing is playing an authored moveset, so
///   nothing can be lost;
/// * `candidate` authored none — every entity `base` named is dropped;
/// * both authored one — the keys `base` names and `candidate` does not.
///
/// ⚠ Sorted and unique because [`MoveSectionData`] is a `BTreeMap`, so a refusal
/// reads the same twice. `the_report_is_sorted_and_unique` pins that as a
/// PROPERTY rather than trusting the container, so swapping the map type cannot
/// quietly make a diagnostic reorder between runs.
pub fn dropped_moveset_entities(
    base: &ambition_content_pack::PreparedContentPack,
    candidate: &ambition_content_pack::PreparedContentPack,
) -> Vec<String> {
    let Some(base_section) = lowered_movesets(base) else {
        return Vec::new();
    };
    match lowered_movesets(candidate) {
        None => base_section.keys().cloned().collect(),
        Some(candidate_section) => base_section
            .keys()
            .filter(|id| !candidate_section.contains_key(*id))
            .cloned()
            .collect(),
    }
}

/// The character capability's move-table registration, for a composition to
/// install.
pub fn moveset_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(MOVESET_SCHEMA),
        version: MOVESET_VERSION,
        capability: CapabilityId::new(CHARACTERS_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "One or more characters' move tables: the verbs a press resolves through \
              and the move timelines they select. Structural faults are refused by the \
              entity catalog's own validator; whether the techniques a move references \
              are INSTALLED is asked at the preparation barrier.",
        handler: Arc::new(MovesetSchema),
    }
}

#[cfg(test)]
mod tests;
