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
    capture: (
        grab: (
            id: "george_grab",
            clip: "grab",
            startup_s: 0.16,
            active_s: 0.06,
            recover_s: 0.30,
            reach: (offset: (18.0, 0.0), half_extents: (26.0, 13.0), hold_offset: (20.0, -2.0)),
        ),
        pummel: (
            id: "george_pummel",
            clip: "attack",
            duration_s: 0.24,
            impact_at_s: 0.11,
            impact: (damage: 4),
        ),
        forward_throw: (
            id: "george_fthrow",
            clip: "attack",
            duration_s: 0.34,
            release_at_s: 0.20,
            launch: (damage: 11, knockback: 138.0, knockback_growth: 1.9, launch_dir: (1.0, -0.35)),
        ),
        back_throw: None,
        up_throw: None,
        down_throw: None,
    ),
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
    assert_eq!(facet.capture.grab.reach.half_extents, (26.0, 13.0));
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

/// the case that parses cleanly and is still wrong. Every field is a
/// plausible number; the grab is a recovery animation. This is the fault the
/// schema exists to name at load rather than as "the grab feels bad" after an
/// evening of play.
#[test]
fn a_grab_that_can_never_catch_anybody_is_refused_by_the_compiler() {
    let text = GEORGE_SHAPED.replace("active_s: 0.06", "active_s: 0.0");
    let failure = refuse(&[("fighters/george.ron", &text)]);
    let rendered = format!("{failure}");
    assert!(rendered.contains("never asked about"), "{rendered}");
    assert!(rendered.contains("fighters/george.ron"), "{rendered}");
}

/// A misspelled field is a typo an author can see, not a mechanic that silently
/// never fires — which is what `deny_unknown_fields` buys and what the handler
/// contract requires it to report.
#[test]
fn a_field_no_schema_consumes_is_refused_by_name() {
    let text = GEORGE_SHAPED.replace("knockback_growth", "knockback_grouth");
    let failure = refuse(&[("fighters/george.ron", &text)]);
    let rendered = format!("{failure}");
    assert!(rendered.contains("knockback_grouth"), "{rendered}");
}

/// A broken facet must not lower a partial book: the runtime never sees content
/// the compiler refused.
#[test]
fn a_refused_facet_lowers_nothing_at_all() {
    let broken = GEORGE_SHAPED.replace("release_at_s: 0.20", "release_at_s: 0.90");
    let failure = refuse(&[
        ("fighters/george.ron", GEORGE_SHAPED),
        ("fighters/broken.ron", &broken),
    ]);
    let rendered = format!("{failure}");
    assert!(rendered.contains("never released"), "{rendered}");
}
