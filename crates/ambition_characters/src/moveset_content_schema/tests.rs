//! ⭐ THESE TEST THE SCHEMA, NOT THE VALIDATOR. `EntityCatalogDoc::validate` has
//! its own probes in `ambition_entity_catalog`; what is asked here is whether a
//! document the TYPE calls broken is one the COMPILER refuses, and whether a pack
//! that compiles hands the runtime the value it validated rather than the bytes
//! again.
//!
//! ⛔ No temp directory. `ContentPackDraft::from_sources` is the same road a
//! shipped pack takes with its embedded text.

use super::*;
use ambition_content_pack::{
    compile, AssetsUnchecked, CompileFailure, ContentPackDraft, ContentPackManifest,
    ModuleNamespace, PackId, PackVersion, SchemaRegistry, SourceDeclaration,
};

/// One character with one real move, authored the way a shipped table is: a
/// verb bound to a move id, a three-phase timeline, and the Active window
/// carrying the volume.
const A_FIGHTER: &str = r#"(
    schema_version: 1,
    entities: [
        (
            id: "test_duelist",
            contracts: (
                moveset: Some((
                    verbs: { "attack": "swat" },
                    moves: [
                        (
                            id: "swat",
                            clip: (clip: "slash"),
                            duration_s: 0.68,
                            windows: [
                                (start_s: 0.0, end_s: 0.28, tag: Startup, volumes: []),
                                (start_s: 0.28, end_s: 0.36, tag: Active, volumes: [
                                    (shape: Rect(offset: (28.0, 0.0), half_extents: (14.0, 10.0)),
                                     damage: 7, knockback: 40.0),
                                ]),
                                (start_s: 0.36, end_s: 0.68, tag: Recovery, volumes: []),
                            ],
                        ),
                    ],
                )),
            ),
        ),
    ],
)"#;

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry.register(moveset_schema()).expect("fresh registry");
    registry
}

fn draft(files: &[(&str, &str)]) -> ContentPackDraft {
    ContentPackDraft::from_sources(
        ContentPackManifest {
            id: PackId("test_moves".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("test".into()),
            requires: Vec::new(),
            sources: files
                .iter()
                .map(|(path, _)| SourceDeclaration {
                    path: (*path).to_string(),
                    schema: SchemaId::new(MOVESET_SCHEMA),
                    version: MOVESET_VERSION,
                })
                .collect(),
        },
        files
            .iter()
            .map(|(path, text)| ((*path).to_string(), (*text).to_string())),
    )
    .expect("draft reads")
}

fn accept(files: &[(&str, &str)]) -> ambition_content_pack::PreparedContentPack {
    compile(&draft(files), &registry(), &AssetsUnchecked).expect("this pack must compile")
}

fn refuse(files: &[(&str, &str)]) -> CompileFailure {
    compile(&draft(files), &registry(), &AssetsUnchecked).expect_err("this pack must be refused")
}

/// ⛔ THE VERB SURVIVES, ASSERTED SEPARATELY FROM THE MOVE. A section carrying
/// only the move list would pass every other arm here and deliver a fighter
/// whose buttons are unbound — "the table loaded" being false in exactly the
/// half that matters.
#[test]
fn a_compiled_pack_carries_the_move_table_the_runtime_will_load() {
    let pack = accept(&[("moves/duelist.ron", A_FIGHTER)]);
    let table = lowered_movesets(&pack).expect("a Runtime schema lowers its artifact");
    assert_eq!(table.len(), 1);
    let contract = table
        .get("test_duelist")
        .expect("the table is keyed by the entity the document names");
    assert_eq!(
        contract.verbs.get("attack").map(String::as_str),
        Some("swat"),
        "the moves arrived and the press that plays them did not"
    );
    assert_eq!(contract.moves.len(), 1);
    assert_eq!(contract.moves[0].duration_s, 0.68);
}

/// ⭐ THE ARM THAT MAKES THE ONE ABOVE MEAN SOMETHING: an edited TIMING reaches
/// the runtime value. Without it a handler that lowered a constant would pass.
#[test]
fn an_edited_timing_reaches_the_lowered_table() {
    let edited = A_FIGHTER.replace("duration_s: 0.68", "duration_s: 0.92").replace(
        "(start_s: 0.36, end_s: 0.68, tag: Recovery",
        "(start_s: 0.36, end_s: 0.92, tag: Recovery",
    );
    assert_ne!(edited, A_FIGHTER, "the edit applied to the fixture");
    let pack = accept(&[("moves/duelist.ron", &edited)]);
    assert_eq!(
        lowered_movesets(&pack).expect("lowers")["test_duelist"].moves[0].duration_s,
        0.92,
        "the compiler handed back a value it did not read from this file"
    );
}

/// ⛔⛔ THE REASON THIS HANDLER OWNS NO VALIDATOR. `UnknownVerbMove` is
/// `EntityCatalogDoc::validate`'s, it predates this module, and the only thing
/// added here is a pack that ASKS it. A verb bound to a move that does not exist
/// is a press that plays nothing — which reads in a playtest as "the button is
/// broken" rather than as a line number.
#[test]
fn a_verb_bound_to_a_move_that_does_not_exist_is_refused() {
    let broken = A_FIGHTER.replace(r#""attack": "swat""#, r#""attack": "nothing""#);
    assert_ne!(broken, A_FIGHTER, "the edit applied to the fixture");
    let failure = refuse(&[("moves/duelist.ron", &broken)]);
    let rendered = failure.to_string();
    assert!(
        rendered.contains("nothing"),
        "the refusal does not name the unresolved verb target:\n{rendered}"
    );
}

/// ⛔ AND THE CONTROL: the same file with the verb intact COMPILES. Without this
/// the arm above would pass for a handler that refused every document.
#[test]
fn the_same_file_with_its_verb_intact_compiles() {
    accept(&[("moves/duelist.ron", A_FIGHTER)]);
}

/// ⛔ NOT LAST-WINS. Two files claiming one character is a question with two
/// answers, and picking one silently is how a fighter swings with the values
/// somebody meant to delete.
///
/// ⭐⭐ **AND THE THING THAT REFUSES IT IS THE COMPILER'S CONFLICT-DETECTION
/// STAGE, NOT THIS SCHEMA.** I wrote a collision arm in `aggregate` and the
/// poison left this test GREEN without it: because the handler `define`s a
/// content id per entity, `compile` catches the second claim before aggregation
/// runs, and names both files. ⇒ What this arm guards is that the handler still
/// DEFINES an id per character — remove the `out.define` and the duplicate goes
/// through.
#[test]
fn two_files_claiming_one_character_are_refused() {
    let failure = refuse(&[
        ("moves/duelist.ron", A_FIGHTER),
        ("moves/duelist_copy.ron", A_FIGHTER),
    ]);
    let rendered = failure.to_string();
    assert!(
        rendered.contains("test_duelist") && rendered.contains("duelist_copy"),
        "the refusal does not name both files:\n{rendered}"
    );
}

/// ⭐ AND TWO DIFFERENT CHARACTERS SHARE ONE TABLE — the aggregate runs for one
/// source too, so the artifact's TYPE does not depend on how many files an
/// author happened to write.
#[test]
fn two_characters_share_one_table() {
    let second = A_FIGHTER.replace("test_duelist", "test_gunner");
    let pack = accept(&[
        ("moves/duelist.ron", A_FIGHTER),
        ("moves/gunner.ron", &second),
    ]);
    let table = lowered_movesets(&pack).expect("lowers");
    assert_eq!(table.len(), 2);
    assert!(table.contains_key("test_duelist") && table.contains_key("test_gunner"));
}

/// ⛔⛤ **`schema_version` WAS WRITTEN THIRTEEN TIMES AND READ NOWHERE.**
/// `EntityCatalogDoc::validate` never looked at it and no parse path compared
/// it, so the field could not refuse anything — the silent misread a version
/// number exists to prevent. This arm is its first reader.
#[test]
fn a_document_from_another_schema_version_is_refused() {
    let future = A_FIGHTER.replace("schema_version: 1", "schema_version: 2");
    assert_ne!(future, A_FIGHTER, "the edit applied to the fixture");
    let failure = refuse(&[("moves/duelist.ron", &future)]);
    assert!(
        failure.to_string().contains("schema_version"),
        "the refusal does not say what was wrong:\n{failure}"
    );
}

/// ⛔ A FILE THAT AUTHORS NO MOVE TABLE. It parses and it validates, and it
/// contributes nothing — which reads downstream as "this character has no moves"
/// rather than as "somebody declared the wrong file".
#[test]
fn a_document_with_no_move_contract_at_all_is_refused() {
    const NO_MOVES: &str = r#"(
    schema_version: 1,
    entities: [ ( id: "test_crate", contracts: (body: Some((half_extents: (16.0, 16.0)))) ) ],
)"#;
    let failure = refuse(&[("moves/crate.ron", NO_MOVES)]);
    assert!(
        failure.to_string().contains("no move contract"),
        "the refusal does not say what was wrong:\n{failure}"
    );
}

/// ⛔⛤ **A TYPO THAT DISPLACES A REQUIRED FIELD IS CAUGHT, AND THAT IS NOT THE
/// SAME CLAIM AS "UNKNOWN FIELDS ARE CAUGHT".** `duration_sec` is refused
/// because `duration_s` then went MISSING, not because anything noticed the
/// field nobody consumes. My first version of this arm asserted the second
/// sentence from the first one's evidence.
#[test]
fn a_typo_that_displaces_a_required_field_is_refused() {
    let typo = A_FIGHTER.replace("duration_s: 0.68", "duration_sec: 0.68");
    assert_ne!(typo, A_FIGHTER, "the edit applied to the fixture");
    let failure = refuse(&[("moves/duelist.ron", &typo)]);
    let rendered = failure.to_string();
    assert!(
        rendered.contains("duration_s"),
        "the refusal does not name the field:\n{rendered}"
    );
}

/// ⛔⛔ **AN UNKNOWN FIELD THAT DISPLACES NOTHING IS A REFUSAL.**
/// `ContentSchemaHandler::check` says it in its own words — *"a handler MUST
/// report an authored field it does not consume. Serde's `deny_unknown_fields`
/// is the cheapest way to get this right; rolling your own field walk and
/// forgetting is how a typo becomes a mechanic that silently never fires."*
///
/// ⛔⛤ **AND IT WAS NOT MET WHEN THIS MODULE WAS WRITTEN. MEASURED 2026-09-11:**
/// exactly ONE of the forty `Deserialize` derives in `ambition_entity_catalog`
/// carried `deny_unknown_fields`, and no type on the move timeline was it — so
/// `nonsense_field: 3` beside a correct `duration_s` compiled clean. The twelve
/// types reachable from an `EntityCatalogDoc` carry it now, which is why this
/// arm is a refusal rather than the note recording a gap that it started as.
///
/// ⚠ NOT THE SAME CLAIM AS THE ARM ABOVE, and reading them as one is the error
/// this pair exists to prevent: a typo that DISPLACES a required field is caught
/// by the missing field, whatever any derive says.
#[test]
fn an_unknown_field_that_displaces_nothing_is_refused() {
    let extra = A_FIGHTER.replace(
        "duration_s: 0.68,",
        "duration_s: 0.68,\n                            nonsense_field: 3,",
    );
    assert_ne!(extra, A_FIGHTER, "the edit applied to the fixture");
    let rendered = refuse(&[("moves/duelist.ron", &extra)]).to_string();
    assert!(
        rendered.contains("nonsense_field"),
        "the refusal does not name the field nobody consumes:\n{rendered}"
    );
}

// ---------------------------------------------------------------------------
// ⭐⭐ **CAN THIS TRANSITION BE APPLIED COMPLETELY?** — `dropped_moveset_entities`
//
// One arm per row of the transition table these were derived from. The row that
// matters is the ENTITY DROP: no allow-list row can see it, because `moveset` IS
// the participating domain, and it is reachable by ordinary authoring.
//
// ⛔ A SINGLE FILE CARRYING TWO ENTITIES IS THE SHAPE THAT MATTERS, which is why
// the fixtures below are built that way rather than one-entity-per-file. The
// shipped `cellular_automaton.ron` holds TWO entities, so dropping one does not
// require deleting a file — a fixture whose only drop road was "remove a source"
// would miss the reachable case entirely.
//
// ⚠ SYNTHETIC RATHER THAN SHIPPED CONTENT, AND NOT BY PREFERENCE: the real move
// tables live under `game/ambition_content/assets`, which this crate must not
// depend on — it is the capability, not the game. What is reproduced here is the
// STRUCTURAL property (two entities, one source), which is the half the defect
// turns on. `reload_tests.rs` is where the shipped pack can be reached.
// ---------------------------------------------------------------------------

/// One entity's authored block, so a fixture composes instead of string-editing
/// a document. `duration_s` is what a RETIME moves.
fn entity_block(id: &str, duration_s: f32) -> String {
    format!(
        r#"        (
            id: "{id}",
            contracts: (
                moveset: Some((
                    verbs: {{ "attack": "swat" }},
                    moves: [
                        (
                            id: "swat",
                            clip: (clip: "slash"),
                            duration_s: {duration_s},
                            windows: [
                                (start_s: 0.0, end_s: 0.28, tag: Startup, volumes: []),
                                (start_s: 0.28, end_s: 0.36, tag: Active, volumes: [
                                    (shape: Rect(offset: (28.0, 0.0), half_extents: (14.0, 10.0)),
                                     damage: 7, knockback: 40.0),
                                ]),
                                (start_s: 0.36, end_s: {duration_s}, tag: Recovery, volumes: []),
                            ],
                        ),
                    ],
                )),
            ),
        )"#
    )
}

/// A one-file document carrying `entities`, the `cellular_automaton.ron` shape.
fn doc_of(entities: &[String]) -> String {
    format!(
        "(\n    schema_version: 1,\n    entities: [\n{}\n    ],\n)",
        entities.join(",\n")
    )
}

/// A compiled pack whose single move table authors `ids`, all at the same timing.
fn pack_authoring(ids: &[&str]) -> ambition_content_pack::PreparedContentPack {
    let blocks: Vec<String> = ids.iter().map(|id| entity_block(id, 0.68)).collect();
    accept(&[("moves/table.ron", &doc_of(&blocks))])
}

/// ⛔⛤ **A PACK THAT DECLARES NO MOVE SOURCE AT ALL** — the candidate half of
/// row 1. MEASURED that this compiles rather than assumed: `ambition_demo_smash`
/// ships a pack with no moveset source, so an empty manifest is a real shape and
/// not a test-only curiosity.
fn pack_authoring_nothing() -> ambition_content_pack::PreparedContentPack {
    accept(&[])
}

/// ⛔⛤ **ROW 2 IS UNREACHABLE THROUGH A COMPILED PACK — AND I HAD THE MECHANISM
/// WRONG.** I reported "a move table authoring zero entities is refused" as
/// REASONED, and attributed it to `ambition_content_pack`'s lower-must-define
/// rule. The outcome is right and the attribution was not: the refusal comes
/// from THIS schema's own check (`moveset_content_schema.rs:108-125`), at facet
/// validation, which is earlier — *"declares the `moveset` schema and carries no
/// move contract for any of its 0 entit(ies)"*. This arm asserts the refusal
/// that actually fires. A mechanism that explains an outcome is not evidence for
/// it, and the test is what separated them.
///
/// ⇒ So "the section went empty" cannot arrive as a compiled candidate. The
/// reachable way to lose every entity is to drop them from a table that still
/// authors something, or to remove the source — both covered below.
///
/// ⚠ DISTINCT FROM `a_document_with_no_move_contract_at_all_is_refused`, which
/// feeds a doc whose entity carries a `body` and no `moveset`. This feeds ZERO
/// entities, which reaches the same check because `!iter().any(..)` is vacuously
/// true over an empty list — a different input, one rule, and the vacuous path
/// is the one row 2 would travel.
#[test]
fn a_move_table_authoring_no_entity_is_refused_rather_than_lowering_an_empty_section() {
    let empty = doc_of(&[]);
    assert!(
        !empty.contains("id:"),
        "the fixture still authors an entity, so this arm is not testing an empty table:\n{empty}"
    );
    let rendered = refuse(&[("moves/table.ron", &empty)]).to_string();
    assert!(
        rendered.contains("no move contract"),
        "an empty move table was refused for some other reason, so row 2's \
         unreachability rests on a rule this arm has not identified:\n{rendered}"
    );
    assert!(
        rendered.contains("0 entit"),
        "the refusal does not report the entity COUNT, so it may be firing on a \
         contract-less entity rather than on an empty table:\n{rendered}"
    );
}

/// ⭐⭐ **THE ROW THAT MATTERS: ONE OF TWO ENTITIES IN ONE FILE, DROPPED.**
/// Nothing removes its authored moveset, and the fold republishes the stale one
/// under the new generation.
#[test]
fn dropping_one_of_two_entities_from_a_shared_table_names_exactly_that_entity() {
    let base = pack_authoring(&["test_brawler", "test_duelist"]);
    let candidate = pack_authoring(&["test_duelist"]);

    // ⛔ THE PREMISE, BOTH HALVES. Without the first the fixture is not a drop;
    // without the second it is a drop of something nobody authored.
    let base_section = lowered_movesets(&base).expect("the base authors a section");
    let candidate_section = lowered_movesets(&candidate).expect("the candidate authors one too");
    assert_eq!(base_section.len(), 2, "the base must author TWO entities");
    assert_eq!(candidate_section.len(), 1, "the candidate must author ONE");
    assert!(
        candidate_section.contains_key("test_duelist"),
        "the surviving entity is not the one this arm keeps"
    );

    assert_eq!(
        dropped_moveset_entities(&base, &candidate),
        vec!["test_brawler".to_string()],
        "a candidate that stops naming an entity the live cast is playing was \
         reported as losing nothing"
    );
}

/// ⛔ ROW 1: the candidate declares no move source at all.
#[test]
fn a_candidate_with_no_move_section_drops_every_entity_the_base_named() {
    let base = pack_authoring(&["test_brawler", "test_duelist"]);
    let candidate = pack_authoring_nothing();

    assert_eq!(
        lowered_movesets(&base).map(|s| s.len()),
        Some(2),
        "the premise: the base authors two entities"
    );
    assert!(
        lowered_movesets(&candidate).is_none(),
        "the premise: the candidate must carry NO move section, or this arm is \
         testing a drop rather than a removal"
    );

    assert_eq!(
        dropped_moveset_entities(&base, &candidate),
        vec!["test_brawler".to_string(), "test_duelist".to_string()],
        "a candidate that removed the whole family was not reported as losing \
         every entity"
    );
}

/// ⛔ AND THE OTHER DIRECTION IS NOT A LOSS: a base that authored nothing has
/// nothing for a candidate to take away.
#[test]
fn a_base_with_no_move_section_can_lose_nothing() {
    let base = pack_authoring_nothing();
    let candidate = pack_authoring(&["test_duelist"]);
    assert!(
        lowered_movesets(&base).is_none() && lowered_movesets(&candidate).is_some(),
        "the premise: base authors none, candidate authors one"
    );
    assert!(
        dropped_moveset_entities(&base, &candidate).is_empty(),
        "first authoring of a move family was reported as a loss"
    );
}

/// ⛔⛤ **CONTROL, AND WITHOUT IT "REFUSES EVERYTHING" SATISFIES EVERY ARM
/// ABOVE.** A retimed shared entity is the ordinary reload — the transition the
/// participant exists to apply — and it must report nothing.
#[test]
fn retiming_a_shared_entity_drops_nothing() {
    let base = accept(&[(
        "moves/table.ron",
        &doc_of(&[entity_block("test_duelist", 0.68)]),
    )]);
    let candidate = accept(&[(
        "moves/table.ron",
        &doc_of(&[entity_block("test_duelist", 0.92)]),
    )]);

    // ⛔ THE PREMISE: the packs must actually DIFFER, or this control passes
    // because it compared a pack with itself.
    assert_ne!(
        base.fingerprint, candidate.fingerprint,
        "the retime did not change the pack's identity, so this control has no power"
    );
    assert_eq!(
        lowered_movesets(&base).expect("a section")["test_duelist"].moves[0].duration_s,
        0.68
    );
    assert_eq!(
        lowered_movesets(&candidate).expect("a section")["test_duelist"].moves[0].duration_s,
        0.92,
        "the retime did not reach the lowered section"
    );

    assert!(
        dropped_moveset_entities(&base, &candidate).is_empty(),
        "a retimed entity was reported as dropped, so an ordinary reload would \
         now be refused"
    );
}

/// ⛔ CONTROL: adding an entity is legal authoring. Containment, not equality —
/// a build that cannot host the new one is refused by `UnknownCharacter`, which
/// is a different question asked somewhere else.
#[test]
fn adding_an_entity_drops_nothing() {
    let base = pack_authoring(&["test_duelist"]);
    let candidate = pack_authoring(&["test_brawler", "test_duelist"]);
    assert_eq!(
        lowered_movesets(&candidate).map(|s| s.len()),
        Some(2),
        "the premise: the candidate authors the extra entity"
    );
    assert!(
        dropped_moveset_entities(&base, &candidate).is_empty(),
        "authoring a NEW entity was reported as dropping one — the test is an \
         equality rather than a containment"
    );
}

/// ⚠ SORTED AND UNIQUE AS A PROPERTY, not as a fact about `BTreeMap`. If
/// `MoveSectionData` ever stops being ordered, a diagnostic that reorders
/// between runs is the failure this refuses.
#[test]
fn the_report_is_sorted_and_unique() {
    let base = pack_authoring(&["test_zeta", "test_alpha", "test_mid"]);
    let candidate = pack_authoring(&["test_mid"]);
    let dropped = dropped_moveset_entities(&base, &candidate);
    assert_eq!(dropped.len(), 2, "the premise: two entities were dropped");
    let mut expected = dropped.clone();
    expected.sort();
    expected.dedup();
    assert_eq!(
        dropped, expected,
        "the report is not sorted and deduplicated"
    );
}
