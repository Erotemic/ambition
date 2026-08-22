//! Probes for the `item_catalog` schema.

use super::*;
use ambition_content_pack::{
    compile, AssetsUnchecked, ContentPackDraft, ContentPackManifest, ModuleNamespace, PackId,
    PackVersion, SchemaRegistry, SourceDeclaration,
};

/// One well-formed row. `slot` only varies the ids so rows stay distinguishable.
fn row(slot: usize) -> String {
    format!(
        r#"(
            display_name: "Item {slot}",
            description: "The {slot}th thing.",
            category: Weapon,
            held_item_id: Some("held_{slot}"),
            dialog_id: "item{slot}",
        )"#
    )
}

/// A full, valid grid — exactly [`ITEM_COUNT`] rows.
fn full_grid() -> String {
    let rows: Vec<String> = (0..ITEM_COUNT).map(row).collect();
    format!("[{}]", rows.join(","))
}

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry
        .register(item_catalog_schema())
        .expect("fresh registry");
    registry
}

fn draft(name: &str, items: &str) -> ContentPackDraft {
    let root = std::env::temp_dir().join(format!("ambition_item_schema_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    std::fs::write(root.join("items.ron"), items).expect("write items");
    ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("test_items".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("test".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: "items.ron".into(),
                schema: SchemaId::new(ITEM_CATALOG_SCHEMA),
                version: ITEM_CATALOG_VERSION,
            }],
        },
    )
    .expect("draft reads")
}

fn refuse(name: &str, items: &str) -> ambition_content_pack::CompileFailure {
    compile(&draft(name, items), &registry(), &AssetsUnchecked)
        .expect_err("this catalog must be refused")
}

#[test]
fn a_compiled_pack_carries_the_item_catalog_the_runtime_will_load() {
    let pack = compile(
        &draft("lowering", &full_grid()),
        &registry(),
        &AssetsUnchecked,
    )
    .expect("a full grid compiles");
    let catalog = lowered_item_catalog(&pack).expect("a Runtime schema lowers its artifact");
    assert_eq!(catalog.rows().len(), ITEM_COUNT);
    assert_eq!(
        catalog.rows()[0].display_name,
        "Item 0",
        "the runtime value is the one the compiler validated, not a re-parse"
    );
}

/// The motivating case. `items.ron` is positional, so a deleted row does
/// not remove one item — it shifts every later row up a slot and silently
/// re-authors the wrong ones, with the tail falling back to built-in defaults so
/// the grid still looks full. `ItemCatalog::from_ron` accepted this happily.
#[test]
fn a_short_grid_is_refused_because_the_file_is_positional() {
    let rows: Vec<String> = (0..ITEM_COUNT - 1).map(row).collect();
    let failure = refuse("short_grid", &format!("[{}]", rows.join(",")));
    assert!(
        failure.has(DiagnosticCode::MalformedSource),
        "a short grid must be refused, not silently shifted: {:?}",
        failure.codes()
    );
}

/// The same defect from the other side: a grid with one row too many binds a row
/// to a slot the item enum does not have, so it is authored content that can
/// never be reached.
#[test]
fn a_long_grid_is_refused_too() {
    let rows: Vec<String> = (0..ITEM_COUNT + 1).map(row).collect();
    let failure = refuse("long_grid", &format!("[{}]", rows.join(",")));
    assert!(
        failure.has(DiagnosticCode::MalformedSource),
        "{:?}",
        failure.codes()
    );
}

/// This is what `deny_unknown_fields` on `ItemMeta` buys.
#[test]
fn an_unknown_authored_field_is_an_error_and_not_a_shrug() {
    let mut rows: Vec<String> = (0..ITEM_COUNT).map(row).collect();
    rows[3] = r#"(
        display_name: "Typo",
        description: "Has a field nothing reads.",
        category: Weapon,
        held_item_id: None,
        dialog_id: "typo",
        stack_limit: 99,
    )"#
    .to_string();
    let failure = refuse("unknown_field", &format!("[{}]", rows.join(",")));
    assert!(
        failure.has(DiagnosticCode::UnknownField),
        "an unconsumed field must be named as such, not folded into a parse error: {:?}",
        failure.codes()
    );
}

/// Two rows answering one `inventory.holds(...)` is an authority conflict: every
/// script asking the question gets whichever the lookup reaches first.
#[test]
fn two_rows_sharing_a_dialog_id_are_a_conflict() {
    let mut rows: Vec<String> = (0..ITEM_COUNT).map(row).collect();
    rows[5] = rows[4].clone();
    let failure = refuse("duplicate_dialog_id", &format!("[{}]", rows.join(",")));
    assert!(
        failure.has(DiagnosticCode::ConflictingModuleContribution),
        "{:?}",
        failure.codes()
    );
}

/// `None` means "not equippable" and is a decision.
#[test]
fn a_half_wired_held_item_is_refused() {
    let mut rows: Vec<String> = (0..ITEM_COUNT).map(row).collect();
    rows[7] = r#"(
        display_name: "Half Wired",
        description: "Somebody started wiring this and stopped.",
        category: Weapon,
        held_item_id: Some(""),
        dialog_id: "halfwired",
    )"#
    .to_string();
    let failure = refuse("empty_held_item", &format!("[{}]", rows.join(",")));
    assert!(
        failure.has(DiagnosticCode::MalformedProviderBinding),
        "{:?}",
        failure.codes()
    );
}

/// A row nothing can name is unreachable content.
#[test]
fn an_empty_dialog_id_is_refused() {
    let mut rows: Vec<String> = (0..ITEM_COUNT).map(row).collect();
    rows[9] = r#"(
        display_name: "Nameless",
        description: "No script can ask for this.",
        category: Weapon,
        held_item_id: None,
        dialog_id: "",
    )"#
    .to_string();
    let failure = refuse("empty_dialog_id", &format!("[{}]", rows.join(",")));
    assert!(
        failure.has(DiagnosticCode::MalformedProviderBinding),
        "{:?}",
        failure.codes()
    );
}

/// A caller must never receive a runtime value out of a pack that was refused —
/// a half-valid catalog reaching the game is worse than no catalog.
#[test]
fn a_refused_pack_hands_out_no_runtime_value() {
    let rows: Vec<String> = (0..ITEM_COUNT - 1).map(row).collect();
    let failure = compile(
        &draft("no_value_on_refusal", &format!("[{}]", rows.join(","))),
        &registry(),
        &AssetsUnchecked,
    );
    assert!(failure.is_err(), "a short grid is refused");
}

/// Swapping two rows changes which metadata belongs to which `Item`, so it
/// must change the fingerprint.
///
/// The pack fingerprint sorts definitions by content id, so a canonical form
/// keyed only by `dialog_id` made a full row swap invisible — the same set of
/// `(dialog_id, row)` pairs, a different game. Exactly the hole the music track
/// ORDER had.
#[test]
fn swapping_two_item_rows_moves_the_fingerprint() {
    let mut rows: Vec<String> = (0..ITEM_COUNT).map(row).collect();
    let base = compile(
        &draft("slot_base", &format!("[{}]", rows.join(","))),
        &registry(),
        &AssetsUnchecked,
    )
    .expect("compiles")
    .fingerprint
    .0;

    rows.swap(3, 9);
    let swapped = compile(
        &draft("slot_swapped", &format!("[{}]", rows.join(","))),
        &registry(),
        &AssetsUnchecked,
    )
    .expect("compiles")
    .fingerprint
    .0;

    assert_ne!(
        base, swapped,
        "the grid is positional: moving a row re-authors two different items"
    );
}

/// The complement — the fingerprint must still be about CONTENT, not layout.
#[test]
fn reformatting_the_item_grid_does_not_move_the_fingerprint() {
    let rows: Vec<String> = (0..ITEM_COUNT).map(row).collect();
    let plain = format!("[{}]", rows.join(","));
    let reflowed = format!("[\n  // a comment nobody reads\n{}\n]", rows.join(",\n"));
    let at = |name: &str, text: &str| {
        compile(&draft(name, text), &registry(), &AssetsUnchecked)
            .expect("compiles")
            .fingerprint
            .0
    };
    assert_eq!(at("reflow_base", &plain), at("reflow_moved", &reflowed));
}

/// An id no script can reach is unreachable content. `Item::from_dialog_id`
/// normalizes the QUERY (lowercase, alphanumerics only) and compares it to the
/// stored spelling verbatim, so an un-normalized authored id silently never
/// resolves.
#[test]
fn a_dialog_id_the_runtime_lookup_can_never_resolve_is_refused() {
    // Include leading/trailing whitespace because normalization must reject it.
    for spelling in [
        "PortalGun",
        "portal_gun",
        "portal gun",
        "portal-gun",
        " portalgun ",
        "portalgun ",
        " portalgun",
    ] {
        let mut rows: Vec<String> = (0..ITEM_COUNT).map(row).collect();
        rows[2] = format!(
            r#"(
                display_name: "Unreachable",
                description: "No script can ask for this.",
                category: Weapon,
                held_item_id: None,
                dialog_id: "{spelling}",
            )"#
        );
        let failure = refuse("unreachable_id", &format!("[{}]", rows.join(",")));
        assert!(
            failure.has(DiagnosticCode::MalformedProviderBinding),
            "`{spelling}` normalizes to something else at lookup, so it must be refused: {:?}",
            failure.codes()
        );
    }
}

/// The shipped ids are already canonical — the contract above must not be a
/// change to what ships, only a guard on what can be authored next.
#[test]
fn an_already_normalized_dialog_id_is_accepted() {
    let mut rows: Vec<String> = (0..ITEM_COUNT).map(row).collect();
    rows[2] = r#"(
        display_name: "Fine",
        description: "Canonical id.",
        category: Weapon,
        held_item_id: None,
        dialog_id: "portalgun2",
    )"#
    .to_string();
    compile(
        &draft("normalized_ok", &format!("[{}]", rows.join(","))),
        &registry(),
        &AssetsUnchecked,
    )
    .expect("a lowercase alphanumeric id is exactly what the lookup resolves");
}
