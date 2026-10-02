//! Probes for the `quest_book` schema, through the real compiler.

use super::*;
use ambition_content_pack::{
    compile, AssetsUnchecked, ContentPackDraft, ContentPackManifest, ModuleNamespace, PackId,
    PackVersion, SchemaRegistry, SourceDeclaration,
};

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry.register(quest_book_schema()).expect("fresh registry");
    registry
}

fn draft(name: &str, book: &str) -> ContentPackDraft {
    let root = std::env::temp_dir().join(format!("ambition_quest_schema_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    std::fs::write(root.join("quests.ron"), book).expect("write quests");
    ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("test_quests".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("test".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: "quests.ron".into(),
                schema: SchemaId::new(QUEST_BOOK_SCHEMA),
                version: QUEST_BOOK_VERSION,
            }],
        },
    )
    .expect("draft reads")
}

fn quest(id: &str, steps: &str) -> String {
    format!(r#"(id: "{id}", title: "T", summary: "S", steps: [{steps}], auto_start: true)"#)
}

const ONE_STEP: &str = r#"(description: "Enter the lab.", condition: RoomEntered("quest_lab"))"#;

fn refusal(name: &str, book: &str) -> String {
    let failure = compile(&draft(name, book), &registry(), &AssetsUnchecked)
        .expect_err("this quest book must be refused");
    format!("{failure:?}")
}

/// The control: a book of two quests compiles and lowers in authored order,
/// with the `auto_start` it says.
#[test]
fn a_quest_book_compiles_and_lowers_its_quests_in_order() {
    let book = format!("[{}, {}]", quest("b", ONE_STEP), quest("a", ONE_STEP));
    let pack = compile(&draft("ok", &book), &registry(), &AssetsUnchecked).expect("compiles");
    let lowered = lowered_quest_book(&pack).expect("the book lowers");
    let ids: Vec<(&str, bool)> = lowered.iter().map(|q| (q.id.as_str(), q.auto_start)).collect();
    assert_eq!(ids, vec![("b", true), ("a", true)]);
    assert_eq!(lowered[0].steps[0].condition, QuestStepCondition::RoomEntered("quest_lab".into()));
}

#[test]
fn a_quest_with_no_steps_is_refused() {
    let failure = refusal("empty", &format!("[{}]", quest("a", "")));
    assert!(failure.contains("has no steps"), "{failure}");
}

#[test]
fn two_quests_with_one_id_are_refused() {
    let failure = refusal("dup", &format!("[{}, {}]", quest("a", ONE_STEP), quest("a", ONE_STEP)));
    assert!(failure.contains("two quests have the id"), "{failure}");
}

#[test]
fn a_step_with_an_empty_condition_id_is_refused() {
    let step = r#"(description: "Talk.", condition: FlagSet(""))"#;
    let failure = refusal("blank", &format!("[{}]", quest("a", step)));
    assert!(failure.contains("empty id"), "{failure}");
}

/// A misspelt field is refused, not read as its default: `auto_strat: true`
/// would otherwise compile to a quest that never starts.
#[test]
fn a_misspelt_quest_field_is_refused() {
    let book = format!(r#"[(id: "a", title: "T", summary: "S", steps: [{ONE_STEP}], auto_strat: true)]"#);
    let failure = refusal("typo", &book);
    assert!(failure.contains("UnknownField"), "{failure}");
}
