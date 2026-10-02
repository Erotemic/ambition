//! Probes for the `cutscene_library` schema, through the real compiler.

use super::*;
use ambition_content_pack::{
    compile, AssetsUnchecked, ContentPackDraft, ContentPackManifest, ModuleNamespace, PackId,
    PackVersion, SchemaRegistry, SourceDeclaration,
};

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry.register(cutscene_library_schema()).expect("fresh registry");
    registry
}

/// A pack of the given cutscene files, declared in this order.
fn draft(name: &str, files: &[(&str, &str)]) -> ContentPackDraft {
    let root = std::env::temp_dir().join(format!("ambition_cutscene_schema_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    for (path, text) in files {
        std::fs::write(root.join(path), text).expect("write cutscenes");
    }
    ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("test_cutscenes".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("test".into()),
            requires: Vec::new(),
            sources: files
                .iter()
                .map(|(path, _)| SourceDeclaration {
                    path: (*path).into(),
                    schema: SchemaId::new(CUTSCENE_LIBRARY_SCHEMA),
                    version: CUTSCENE_LIBRARY_VERSION,
                })
                .collect(),
        },
    )
    .expect("draft reads")
}

fn script(id: &str, beats: &str) -> String {
    format!(r#"(id: "{id}", beats: [{beats}], seen_flag: Some("{id}_seen"))"#)
}

const ONE_BEAT: &str = r#"Banner(text: "hello", seconds: 1.0)"#;

fn refusal(name: &str, files: &[(&str, &str)]) -> String {
    let failure = compile(&draft(name, files), &registry(), &AssetsUnchecked)
        .expect_err("these cutscenes must be refused");
    format!("{failure:?}")
}

/// The control: two files merge into one library, the files in manifest
/// order and each file in authored order.
#[test]
fn two_cutscene_files_merge_into_one_library_in_manifest_order() {
    let first = format!("[{}, {}]", script("b", ONE_BEAT), script("a", ONE_BEAT));
    let second = format!("[{}]", script("c", r#"Wait(seconds: 0.5)"#));
    let pack = compile(
        &draft("ok", &[("one.ron", &first), ("two.ron", &second)]),
        &registry(),
        &AssetsUnchecked,
    )
    .expect("compiles");
    let ids: Vec<&str> = lowered_cutscenes(&pack)
        .expect("the library lowers")
        .iter()
        .map(|script| script.id.as_str())
        .collect();
    assert_eq!(ids, vec!["b", "a", "c"]);
}

#[test]
fn one_id_in_two_files_is_refused() {
    let one = format!("[{}]", script("a", ONE_BEAT));
    let failure = refusal("dup", &[("one.ron", &one), ("two.ron", &one)]);
    assert!(failure.contains("DuplicateIdentity"), "{failure}");
}

#[test]
fn a_cutscene_with_no_beats_is_refused() {
    let failure = refusal("empty", &[("one.ron", &format!("[{}]", script("a", "")))]);
    assert!(failure.contains("has no beats"), "{failure}");
}

#[test]
fn a_fade_outside_clear_and_black_is_refused() {
    let fade = r#"Fade(from_alpha: 1.0, to_alpha: 1.5, seconds: 0.5)"#;
    let failure = refusal("fade", &[("one.ron", &format!("[{}]", script("a", fade)))]);
    assert!(failure.contains("not in 0..=1"), "{failure}");
}

#[test]
fn a_negative_duration_is_refused() {
    let wait = r#"Wait(seconds: -1.0)"#;
    let failure = refusal("wait", &[("one.ron", &format!("[{}]", script("a", wait)))]);
    assert!(failure.contains("negative or not finite"), "{failure}");
}

/// A misspelt field is refused, not read as its default.
#[test]
fn a_misspelt_beat_field_is_refused() {
    let banner = r#"Banner(text: "hello", secs: 1.0)"#;
    let failure = refusal("typo", &[("one.ron", &format!("[{}]", script("a", banner)))]);
    assert!(failure.contains("UnknownField"), "{failure}");
}
