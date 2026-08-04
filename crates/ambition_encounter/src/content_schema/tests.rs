//! Probes for the `encounter_waves` schema.
//!
//! Each negative case names the failure it prevents, not the prose of the
//! message — a diagnostic's wording is a rendering detail and pinning it makes
//! these fail on a reworded fix line.

use super::*;
use ambition_content_pack::{
    compile, AssetsUnchecked, ContentPackDraft, ContentPackManifest, ModuleNamespace, PackId,
    PackVersion, SchemaRegistry, SourceDeclaration,
};

/// ⚠ copied from the shape `goblin_encounter.ron` actually uses — I first wrote
/// this from the field names I expected (`archetype`/`at`) and both positive
/// cases failed. A fixture invented rather than read tests the invention.
const ONE_MOB: &str =
    r#"(kind: "medium_striker", spawn: (1180.0, 580.0), size: (22.0, 38.0), delay: 0.0)"#;

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry
        .register(encounter_waves_schema())
        .expect("fresh registry");
    registry
}

fn draft(name: &str, waves: &str) -> ContentPackDraft {
    let root = std::env::temp_dir().join(format!("ambition_encounter_schema_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    std::fs::write(root.join("waves.ron"), waves).expect("write waves");
    ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("test_encounters".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("test".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: "waves.ron".into(),
                schema: SchemaId::new(ENCOUNTER_WAVES_SCHEMA),
                version: ENCOUNTER_WAVES_VERSION,
            }],
        },
    )
    .expect("draft reads")
}

fn refuse(name: &str, waves: &str) -> ambition_content_pack::CompileFailure {
    compile(&draft(name, waves), &registry(), &AssetsUnchecked)
        .expect_err("this wave book must be refused")
}

/// A book with waves compiles and LOWERS — the runtime path this replaces.
#[test]
fn a_compiled_pack_carries_the_wave_book_the_runtime_will_load() {
    let pack = compile(
        &draft(
            "lowering",
            &format!(r#"{{"goblin_encounter": [(label: "first", mobs: [{ONE_MOB}])]}}"#),
        ),
        &registry(),
        &AssetsUnchecked,
    )
    .expect("a well-formed book compiles");
    let book = lowered_encounter_waves(&pack).expect("a Runtime schema lowers its artifact");
    assert_eq!(
        book["goblin_encounter"].len(),
        1,
        "the runtime value is the one the compiler validated, not a re-parse"
    );
}

/// ⛔ **The motivating case: the invariant a serde parse cannot see.** An
/// encounter authored with an empty wave list parses perfectly and means exactly
/// what OMITTING the key means — the loader falls back to marker-derived spawns.
/// So the file claims a timeline it does not have, and nothing says so.
#[test]
fn an_encounter_with_no_waves_is_refused_rather_than_silently_ignored() {
    let failure = refuse("empty_book", r#"{"goblin_encounter": []}"#);
    assert!(
        format!("{failure:?}").contains("ZERO waves"),
        "the diagnostic must name the defect: {failure:?}"
    );
}

/// A wave with no mobs is cleared the instant it starts — it reads as a pause
/// the encounter never actually takes.
#[test]
fn a_wave_with_no_mobs_is_refused() {
    let failure = refuse(
        "empty_wave",
        r#"{"goblin_encounter": [(label: "empty", mobs: [])]}"#,
    );
    assert!(format!("{failure:?}").contains("no mobs"), "{failure:?}");
}

/// ⚠ the loader matches the trigger id VERBATIM against the level's trigger, so
/// a padded key compiles and is unreachable — the same class as the item
/// catalog's un-normalized `dialog_id`.
#[test]
fn a_padded_trigger_id_is_refused_because_the_lookup_is_verbatim() {
    let failure = refuse(
        "padded_id",
        &format!(r#"{{" goblin_encounter ": [(label: "w", mobs: [{ONE_MOB}])]}}"#),
    );
    assert!(format!("{failure:?}").contains("whitespace"), "{failure:?}");
}

/// ⛔⛔ **Editing a wave must MOVE THE PACK'S IDENTITY.**
///
/// This schema lowered its book and defined nothing, and `canonical_bytes` is
/// built from defined rows — so changing a mob's delay, its archetype, or the
/// wave ORDER changed what the game runs and left the fingerprint byte-identical.
/// Two peers could carry different encounters and agree they had the same
/// content (GPT 5.6 review of `1a05b98`, finding 1).
///
/// ⚠ the probe varies a field that is NOT part of any content id, which is the
/// whole point: an id change would move the fingerprint even under the bug.
#[test]
fn changing_a_wave_moves_the_pack_fingerprint() {
    let book = |delay: &str| {
        format!(
            r#"{{"goblin_encounter": [(label: "first", mobs: [(kind: "medium_striker", \
               spawn: (1180.0, 580.0), size: (22.0, 38.0), delay: {delay})])]}}"#
        )
        .replace("\\\n               ", "")
    };
    let fingerprint = |name: &str, delay: &str| {
        compile(
            &draft(name, &book(delay)),
            &registry(),
            &AssetsUnchecked,
        )
        .expect("a well-formed book compiles")
        .fingerprint
    };
    assert_ne!(
        fingerprint("delay_zero", "0.0"),
        fingerprint("delay_two", "2.0"),
        "a mob that now arrives two seconds later is different content, and the \
         pack has to say so"
    );
    // The complement, so the test is about the CHANGE and not about the pack id:
    // the same book twice fingerprints the same.
    assert_eq!(
        fingerprint("same_a", "0.0"),
        fingerprint("same_b", "0.0"),
        "identical content must keep one identity"
    );
}
