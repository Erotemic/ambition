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
