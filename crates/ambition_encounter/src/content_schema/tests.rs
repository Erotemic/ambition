//! Probes for the `encounter_waves` schema.

use super::*;
use ambition_content_pack::{
    compile, AssetsUnchecked, ContentPackDraft, ContentPackManifest, ModuleNamespace, PackId,
    PackVersion, SchemaRegistry, SourceDeclaration,
};

/// ⛔ COPIED FROM THE SHAPE `goblin_encounter.ron` ACTUALLY USES. Written from
/// the field names one would expect (`archetype`/`at`), both positive cases
/// fail: a fixture invented rather than read tests the invention.
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

// ── a stand-in cast, so the character reference has something to resolve ────
//
// The `character` schema is minted by `character_catalog`, which lives in
// `ambition_characters` — a crate this one does not depend on and must not.
//  a cross-schema reference is by SCHEMA ID, not by Rust type, which is
// exactly how `boss_encounter` names a `music_track` across the same kind of
// boundary. So the whole fixture is a handler that mints `character` identities
// from a whitespace-separated list; what it proves is what the compiler
// actually does — match `ambition:character/<name>` against the pack's defined
// rows.

const CAST_SCHEMA: &str = "test_cast";

struct CastSchema;

impl ContentSchemaHandler for CastSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        for name in facet.text.split_whitespace() {
            out.define(facet.content_id_in(CHARACTER_SCHEMA, name), name);
        }
    }
}

fn registry_with_cast() -> SchemaRegistry {
    let mut registry = registry();
    registry
        .register(SchemaRegistration {
            id: SchemaId::new(CAST_SCHEMA),
            version: SchemaVersion(1),
            capability: CapabilityId::new("test_cast"),
            // Nothing lowers: this fixture exists to DEFINE identities, and
            // saying so is what keeps the compiler from demanding an artifact.
            disposition: RuntimeDisposition::AuthoringOnly,
            doc: "test fixture: mints `character` identities from a name list",
            handler: Arc::new(CastSchema),
        })
        .expect("fresh registry");
    registry
}

fn draft_with_cast(name: &str, waves: &str, cast: &str) -> ContentPackDraft {
    let root = std::env::temp_dir().join(format!("ambition_encounter_schema_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    std::fs::write(root.join("waves.ron"), waves).expect("write waves");
    std::fs::write(root.join("cast.txt"), cast).expect("write cast");
    ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("test_encounters".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("test".into()),
            requires: Vec::new(),
            sources: vec![
                SourceDeclaration {
                    path: "cast.txt".into(),
                    schema: SchemaId::new(CAST_SCHEMA),
                    version: SchemaVersion(1),
                },
                SourceDeclaration {
                    path: "waves.ron".into(),
                    schema: SchemaId::new(ENCOUNTER_WAVES_SCHEMA),
                    version: ENCOUNTER_WAVES_VERSION,
                },
            ],
        },
    )
    .expect("draft reads")
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

///  The motivating case: the invariant a serde parse cannot see. An
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

///  the loader matches the trigger id VERBATIM against the level's trigger, so
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

/// Runtime-significant wave edits must change the pack fingerprint even when
/// content ids are unchanged.
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
        compile(&draft(name, &book(delay)), &registry(), &AssetsUnchecked)
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

/// Unknown authored fields are refused at every nested schema level so typos
/// cannot be accepted and then ignored.
#[test]
fn a_field_the_schema_does_not_know_is_refused_at_every_level() {
    // On the MOB, the innermost authored shape.
    let failure = refuse(
        "unknown_mob_field",
        r#"{"goblin_encounter": [(label: "first", mobs: [(kind: "medium_striker", spawn: (1180.0, 580.0), size: (22.0, 38.0), delay: 0.0, favourite_snack: "worms")])]}"#,
    );
    assert!(
        failure
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UnknownField),
        "a mob field nothing reads must be reported as unknown: {}",
        failure.render()
    );

    // And on the WAVE, because the two types are guarded separately and one of
    // them being right is how the other stays wrong.
    let failure = refuse(
        "unknown_wave_field",
        &format!(
            r#"{{"goblin_encounter": [(label: "first", mobs: [{ONE_MOB}], favourite_snack: "worms")]}}"#
        ),
    );
    assert!(
        failure
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UnknownField),
        "a wave field nothing reads must be reported as unknown: {}",
        failure.render()
    );
}

/// A mob whose character reference does not resolve is refused during content
/// compilation.
#[test]
fn a_mob_naming_a_character_the_pack_does_not_define_is_refused() {
    let book = |character: &str| {
        format!(
            r#"{{"goblin_encounter": [(label: "first", mobs: [(kind: "medium_striker", \
               character: Some("{character}"), spawn: (1180.0, 580.0), size: (22.0, 38.0), \
               delay: 0.0)])]}}"#
        )
        .replace("\\\n               ", "")
    };
    // The cast this pack defines. `goblin` is spelled correctly here and nowhere
    // else, which is what makes the poison below discriminating: a refusal that
    // fired for every character would prove nothing.
    const CAST: &str = "goblin sandbag";

    compile(
        &draft_with_cast("named_character", &book("goblin"), CAST),
        &registry_with_cast(),
        &AssetsUnchecked,
    )
    .expect("a mob naming a character the pack defines compiles");

    let failure = compile(
        &draft_with_cast("misspelled_character", &book("gobln"), CAST),
        &registry_with_cast(),
        &AssetsUnchecked,
    )
    .expect_err("a mob naming a character nothing defines must be refused");
    assert!(
        failure.has(DiagnosticCode::UnresolvedReference),
        "a misspelled character must be an UNRESOLVED REFERENCE — an unknown field or a \
         parse error would mean the reference was never emitted: {}",
        failure.render()
    );
}
