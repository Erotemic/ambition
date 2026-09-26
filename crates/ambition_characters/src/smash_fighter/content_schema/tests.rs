//! these test the SCHEMA, not the facet. [`SmashFighterFacet::problems`] has
//! its own probes next door; what is asked here is whether a facet the TYPE
//! calls broken is one the COMPILER refuses, and whether a pack that compiles
//! hands the runtime the value it validated rather than the bytes again.
//!
//! no temp directory. [`ContentPackDraft::from_sources`] is the same road
//! a shipped pack takes with its embedded text, so these probes exercise the
//! production reading path instead of a filesystem the production path does not
//! use.

use super::*;
use ambition_content_pack::{
    compile, AssetsUnchecked, CompileFailure, ContentPackDraft, ContentPackManifest,
    ModuleNamespace, PackId, PackVersion, SchemaRegistry, SourceDeclaration,
};

const GEORGE_SHAPED: &str = r#"(
    character: "test_george",
    body: Some((gravity: Some(1900.0), max_fall_speed: Some(640.0))),
    knockback_weight: Some(1.35),
)"#;

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry
        .register(smash_fighter_schema())
        .expect("fresh registry");
    registry
}

fn draft(files: &[(&str, &str)]) -> ContentPackDraft {
    ContentPackDraft::from_sources(
        ContentPackManifest {
            id: PackId("test_smash".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("test".into()),
            requires: Vec::new(),
            sources: files
                .iter()
                .map(|(path, _)| SourceDeclaration {
                    path: (*path).to_string(),
                    schema: SchemaId::new(SMASH_FIGHTER_SCHEMA),
                    version: SMASH_FIGHTER_VERSION,
                })
                .collect(),
        },
        files
            .iter()
            .map(|(path, text)| ((*path).to_string(), (*text).to_string())),
    )
    .expect("draft reads")
}

fn refuse(files: &[(&str, &str)]) -> CompileFailure {
    compile(&draft(files), &registry(), &AssetsUnchecked).expect_err("this pack must be refused")
}

#[test]
fn a_compiled_pack_carries_the_fighter_book_the_runtime_will_load() {
    let pack = compile(
        &draft(&[("fighters/george.ron", GEORGE_SHAPED)]),
        &registry(),
        &AssetsUnchecked,
    )
    .expect("a well-formed facet compiles");
    let book = lowered_smash_fighters(&pack).expect("a Runtime schema lowers its artifact");
    assert_eq!(book.len(), 1);
    let facet = book
        .get("test_george")
        .expect("the book is keyed by the character the facet names");
    assert_eq!(facet.knockback_weight, Some(1.35));
    assert_eq!(facet.body.and_then(|body| body.gravity), Some(1900.0));
}

/// the aggregate runs for ONE source too. Without that the artifact's
/// TYPE would depend on how many files an author happened to write — see the
/// same rule on `ContentSchemaHandler::aggregate` — and the test above would be
/// the only shape that ever worked.
#[test]
fn two_characters_share_one_book() {
    let second = GEORGE_SHAPED.replace("test_george", "test_alice");
    let pack = compile(
        &draft(&[
            ("fighters/george.ron", GEORGE_SHAPED),
            ("fighters/alice.ron", &second),
        ]),
        &registry(),
        &AssetsUnchecked,
    )
    .expect("two well-formed facets compile");
    let book = lowered_smash_fighters(&pack).expect("the book lowers");
    assert_eq!(book.len(), 2);
    assert!(book.contains_key("test_george") && book.contains_key("test_alice"));
}

/// two files claiming one fighter is a question with two answers. The refusal
/// names both FILES, because that is what an author has to open.
#[test]
fn two_files_claiming_one_character_are_refused_and_both_are_named() {
    let failure = refuse(&[
        ("fighters/george.ron", GEORGE_SHAPED),
        ("fighters/george_copy.ron", GEORGE_SHAPED),
    ]);
    let rendered = format!("{failure}");
    assert!(rendered.contains("fighters/george.ron"), "{rendered}");
    assert!(rendered.contains("fighters/george_copy.ron"), "{rendered}");
    assert!(rendered.contains("test_george"), "{rendered}");
}

/// The case that parses cleanly and is still wrong: a weight the launch law
/// divides by. It is named at load, not found as "this fighter flies wrong"
/// after an evening of play.
#[test]
fn a_weight_the_launch_divides_by_is_refused_by_the_compiler_when_not_positive() {
    let text = GEORGE_SHAPED.replace("knockback_weight: Some(1.35)", "knockback_weight: Some(0.0)");
    let failure = refuse(&[("fighters/george.ron", &text)]);
    let rendered = format!("{failure}");
    assert!(rendered.contains("knockback_weight"), "{rendered}");
    assert!(rendered.contains("fighters/george.ron"), "{rendered}");
}

/// A misspelled field is a typo an author can see, not a mechanic that silently
/// never fires — which is what `deny_unknown_fields` buys and what the handler
/// contract requires it to report.
#[test]
fn a_field_no_schema_consumes_is_refused_by_name() {
    let text = GEORGE_SHAPED.replace("max_fall_speed", "max_fall_sped");
    let failure = refuse(&[("fighters/george.ron", &text)]);
    let rendered = format!("{failure}");
    assert!(rendered.contains("max_fall_sped"), "{rendered}");
}

/// A broken facet must not lower a partial book: the runtime never sees content
/// the compiler refused.
#[test]
fn a_refused_facet_lowers_nothing_at_all() {
    let broken = GEORGE_SHAPED.replace("knockback_weight: Some(1.35)", "knockback_weight: Some(-1.0)");
    let failure = refuse(&[
        ("fighters/george.ron", GEORGE_SHAPED),
        ("fighters/broken.ron", &broken),
    ]);
    let rendered = format!("{failure}");
    assert!(rendered.contains("knockback_weight"), "{rendered}");
}
