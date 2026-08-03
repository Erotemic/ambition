//! Probes for the `character_archetypes` schema.
//!
//! ⛔ **The governing contract here is that the compiler must not APPROVE what
//! the runtime REFUSES.** `CharacterRosterFragment`'s assembly rejects blank
//! brain keys and inheritance cycles and Ambition turns that into a startup
//! panic, so anything it refuses has to be refused here too — a validator that
//! says yes to content the game says no to has moved the failure to the worst
//! possible place. (GPT 5.6 review, finding 3.)

use super::*;
use ambition_content_pack::{
    compile, AssetsUnchecked, CompileFailure, ContentPackDraft, ContentPackManifest,
    ModuleNamespace, PackId, PackVersion, SchemaRegistry, SourceDeclaration,
};

/// One well-formed row, optionally inheriting another.
///
/// ⚠ the field set is the SHIPPED `combatant` row's, not an invention: a probe
/// whose fixture drifts from the real authored shape stops testing the schema
/// the game actually uses.
fn row(inherits: Option<&str>) -> String {
    let inherits = match inherits {
        Some(parent) => format!("inherits: Some(\"{parent}\"),"),
        None => String::new(),
    };
    format!(
        r#"(
            {inherits}
            respawn: OnRoomReenter,
            max_health: 4,
            run_speed: 155.0,
            patrol_effort: 0.6774,
            chase_effort: 1.0,
            aggro_radius: 460.0,
            attack_range: 150.0,
            contact_strength: 0.70,
            damage_amount: 1,
            brain_template: Smash,
            move_style: Walk,
        )"#
    )
}

/// A roster from `(key, parent)` pairs, always carrying the reserved fallback.
fn roster(rows: &[(&str, Option<&str>)]) -> String {
    let mut entries = vec![format!("\"combatant\": {}", row(None))];
    for (key, parent) in rows {
        if *key == "combatant" {
            entries.clear();
        }
        entries.push(format!("\"{key}\": {}", row(*parent)));
    }
    format!("{{{}}}", entries.join(","))
}

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry
        .register(character_archetypes_schema())
        .expect("fresh registry");
    registry
}

fn draft(name: &str, text: &str) -> ContentPackDraft {
    let root = std::env::temp_dir().join(format!("ambition_archetype_schema_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    std::fs::write(root.join("archetypes.ron"), text).expect("write source");
    ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("test_archetypes".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("test".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: "archetypes.ron".into(),
                schema: SchemaId::new(CHARACTER_ARCHETYPES_SCHEMA),
                version: CHARACTER_ARCHETYPES_VERSION,
            }],
        },
    )
    .expect("draft reads")
}

fn refuse(name: &str, text: &str) -> CompileFailure {
    compile(&draft(name, text), &registry(), &AssetsUnchecked)
        .expect_err("this roster must be refused")
}

#[test]
fn a_compiled_pack_carries_the_roster_the_runtime_will_load() {
    let text = roster(&[("skirmisher", None), ("heavy", Some("skirmisher"))]);
    let pack = compile(&draft("lowering", &text), &registry(), &AssetsUnchecked)
        .expect("a well-formed roster compiles");
    let archetypes =
        lowered_character_archetypes(&pack).expect("a Runtime schema lowers its artifact");
    assert_eq!(archetypes.len(), 3);
    assert!(archetypes.contains_key("combatant"));
}

/// ⛔ **The two-node cycle the old handler let through.** It checked missing
/// parents and DIRECT self-inheritance only, so `a -> b -> a` compiled clean and
/// then panicked at startup on `MovementInheritanceCycle`.
#[test]
fn a_two_node_inheritance_cycle_is_refused() {
    let text = roster(&[("a", Some("b")), ("b", Some("a"))]);
    let failure = refuse("two_node_cycle", &text);
    assert!(
        failure.has(DiagnosticCode::ConflictingModuleContribution),
        "a two-row loop must be refused at compile time: {:?}",
        failure.codes()
    );
}

/// The same defect one hop longer — a walk that only looks one step deep misses
/// it just as completely.
#[test]
fn a_three_node_inheritance_cycle_is_refused() {
    let text = roster(&[("a", Some("b")), ("b", Some("c")), ("c", Some("a"))]);
    let failure = refuse("three_node_cycle", &text);
    assert!(
        failure.has(DiagnosticCode::ConflictingModuleContribution),
        "{:?}",
        failure.codes()
    );
}

/// The loop is reported ONCE, not once per member — three diagnostics saying the
/// same thing is how a real problem gets skimmed past.
#[test]
fn a_cycle_is_reported_once_not_once_per_member() {
    let text = roster(&[("a", Some("b")), ("b", Some("c")), ("c", Some("a"))]);
    let failure = refuse("cycle_once", &text);
    let cycles = failure
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("forms a cycle"))
        .count();
    assert_eq!(cycles, 1, "expected one cycle diagnostic, got {cycles}");
}

/// A long chain that does NOT close must still compile — a cycle check that
/// refuses ordinary deep inheritance is worse than none.
#[test]
fn a_deep_chain_that_does_not_close_still_compiles() {
    let text = roster(&[
        ("base", None),
        ("mid", Some("base")),
        ("leaf", Some("mid")),
        ("tip", Some("leaf")),
    ]);
    compile(&draft("deep_chain", &text), &registry(), &AssetsUnchecked)
        .expect("a four-deep chain is ordinary inheritance, not a cycle");
}

/// The runtime refuses a blank brain key (`EmptyBrainId`); so must this.
#[test]
fn a_blank_archetype_key_is_refused() {
    let text = format!("{{\"combatant\": {}, \"\": {}}}", row(None), row(None));
    let failure = refuse("blank_key", &text);
    assert!(
        failure.has(DiagnosticCode::MalformedProviderBinding),
        "{:?}",
        failure.codes()
    );
}

#[test]
fn a_dangling_parent_is_refused() {
    let text = roster(&[("orphan", Some("nobody"))]);
    let failure = refuse("dangling_parent", &text);
    assert!(
        failure.has(DiagnosticCode::UnresolvedReference),
        "{:?}",
        failure.codes()
    );
}

/// §4.7: effort is a FRACTION of `run_speed`. The seam clamps an out-of-range
/// value, so it reads as tuned and behaves identically.
#[test]
fn an_effort_outside_zero_to_one_is_refused() {
    let text = roster(&[("sprinter", None)]).replace("chase_effort: 1.0", "chase_effort: 2.5");
    let failure = refuse("effort_range", &text);
    assert!(
        failure.has(DiagnosticCode::MalformedProviderBinding),
        "{:?}",
        failure.codes()
    );
}

/// Without the reserved fallback an unknown spawn brain key has nothing to
/// downgrade to.
#[test]
fn a_roster_with_no_fallback_row_is_refused() {
    let text = format!("{{\"skirmisher\": {}}}", row(None));
    let failure = refuse("no_fallback", &text);
    assert!(
        failure.has(DiagnosticCode::UnresolvedReference),
        "{:?}",
        failure.codes()
    );
}
