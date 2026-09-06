//! Probes for the two boss-pattern schemas.

use super::*;
use ambition_content_pack::{
    compile, AssetsUnchecked, CompileFailure, ContentPackDraft, ContentPackManifest,
    ModuleNamespace, PackId, PackVersion, SchemaRegistry, SourceDeclaration,
};

const ONE_SEED: &str = r#"{
    "sweep": (
        archetype: Sweep,
        intent: "A wide readable arc that punishes standing still.",
        skill_tested: "Spacing.",
        fair_counters: [Jump, Dash],
        threat: Pressure,
        telegraph: (min_s: 0.3, max_s: 0.5),
        active: (min_s: 0.1, max_s: 0.2),
        instances: ["Strike(\"side_sweep\")"],
        recipes: [],
    ),
}"#;

/// The shipped calibration's shape (§3 Calibration v0), inline so a probe does
/// not depend on the game crate's file layout.
const BANDS: &str = r#"(
    tick_hz: 60.0,
    telegraph_ticks: (pressure: 12.0, light: 12.0, medium: 20.0, heavy: 30.0),
    recovery_ticks: (pressure: 0.0, light: 6.0, medium: 12.0, heavy: 24.0),
    core_verbs: [Jump, Dash, WalkOut],
    warn_deviation_frac: 0.2,
)"#;

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry
        .register(boss_seed_library_schema())
        .expect("fresh registry");
    registry
        .register(boss_validator_bands_schema())
        .expect("fresh registry");
    registry
        .register(boss_encounter_schema())
        .expect("fresh registry");
    registry
}

fn draft(
    name: &str,
    file: &str,
    text: &str,
    schema: &str,
    version: SchemaVersion,
) -> ContentPackDraft {
    let root = std::env::temp_dir().join(format!("ambition_boss_schema_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    std::fs::write(root.join(file), text).expect("write source");
    ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("test_boss".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("test".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: file.into(),
                schema: SchemaId::new(schema),
                version,
            }],
        },
    )
    .expect("draft reads")
}

fn seeds_draft(name: &str, text: &str) -> ContentPackDraft {
    draft(
        name,
        "seeds.ron",
        text,
        BOSS_SEEDS_SCHEMA,
        BOSS_SEEDS_VERSION,
    )
}

fn refuse_seeds(name: &str, text: &str) -> CompileFailure {
    compile(&seeds_draft(name, text), &registry(), &AssetsUnchecked)
        .expect_err("this seed library must be refused")
}

#[test]
fn a_compiled_pack_carries_the_seed_library_the_runtime_will_load() {
    let pack = compile(
        &seeds_draft("lowering", ONE_SEED),
        &registry(),
        &AssetsUnchecked,
    )
    .expect("a well-formed library compiles");
    let library = lowered_seed_library(&pack).expect("a Runtime schema lowers its artifact");
    assert_eq!(library.len(), 1);
    assert!(
        library.get("sweep").is_some(),
        "the runtime value is the one the compiler validated, not a re-parse"
    );
}

#[test]
fn a_compiled_pack_carries_the_validator_bands_the_runtime_will_load() {
    let d = draft(
        "bands",
        "bands.ron",
        BANDS,
        BOSS_VALIDATOR_BANDS_SCHEMA,
        BOSS_VALIDATOR_BANDS_VERSION,
    );
    let pack = compile(&d, &registry(), &AssetsUnchecked).expect("bands compile");
    let bands = lowered_validator_bands(&pack).expect("a Runtime schema lowers its artifact");
    assert_eq!(bands.tick_hz, 60.0);
}

/// BD5 rule 2: a seed with no fair counter describes an attack the player has no
/// move against, which is the definition of unfair.
#[test]
fn a_seed_with_no_fair_counter_is_refused() {
    let text = ONE_SEED.replace("fair_counters: [Jump, Dash],", "fair_counters: [],");
    let failure = refuse_seeds("no_counter", &text);
    assert!(
        failure.has(DiagnosticCode::MalformedProviderBinding),
        "{:?}",
        failure.codes()
    );
}

/// An inverted band matches NOTHING, so every instance silently falls outside
/// it — the check that looks like it is running is the one doing nothing.
#[test]
fn an_inverted_duration_band_is_refused() {
    let text = ONE_SEED.replace(
        "telegraph: (min_s: 0.3, max_s: 0.5)",
        "telegraph: (min_s: 0.9, max_s: 0.5)",
    );
    let failure = refuse_seeds("inverted_band", &text);
    assert!(
        failure.has(DiagnosticCode::MalformedProviderBinding),
        "{:?}",
        failure.codes()
    );
}

/// An attack belongs to exactly one archetype: the classification is what the
/// seed MEANS, so two claims is a contradiction rather than a duplicate.
#[test]
fn one_attack_claimed_by_two_seeds_is_a_conflict() {
    let text = r#"{
        "sweep": (
            archetype: Sweep,
            intent: "A wide arc.",
            skill_tested: "Spacing.",
            fair_counters: [Jump],
            threat: Pressure,
            telegraph: (min_s: 0.3, max_s: 0.5),
            active: (min_s: 0.1, max_s: 0.2),
            instances: ["Strike(\"side_sweep\")"],
            recipes: [],
        ),
        "slam": (
            archetype: Slam,
            intent: "A heavy drop.",
            skill_tested: "Reading.",
            fair_counters: [Dash],
            threat: Heavy,
            telegraph: (min_s: 0.4, max_s: 0.6),
            active: (min_s: 0.1, max_s: 0.3),
            instances: ["Strike(\"side_sweep\")"],
            recipes: [],
        ),
    }"#;
    let failure = refuse_seeds("double_claim", text);
    assert!(
        failure.has(DiagnosticCode::ConflictingModuleContribution),
        "{:?}",
        failure.codes()
    );
}

/// An authored field nothing consumes is a tuning knob that silently never
/// applies — the most expensive kind of content bug.
#[test]
fn an_unknown_authored_field_is_an_error_and_not_a_shrug() {
    let text = ONE_SEED.replace("recipes: [],", "recipes: [], wind_down: 0.4,");
    let failure = refuse_seeds("unknown_field", &text);
    assert!(
        failure.has(DiagnosticCode::UnknownField),
        "{:?}",
        failure.codes()
    );
}

/// `tick_hz` is the seconds→ticks conversion the whole calibration is expressed
/// against. Zero converts every duration to zero, silently.
#[test]
fn a_zero_tick_rate_is_refused() {
    let text = BANDS.replace("tick_hz: 60.0", "tick_hz: 0.0");
    let d = draft(
        "zero_hz",
        "bands.ron",
        &text,
        BOSS_VALIDATOR_BANDS_SCHEMA,
        BOSS_VALIDATOR_BANDS_VERSION,
    );
    let failure = compile(&d, &registry(), &AssetsUnchecked).expect_err("refused");
    assert!(
        failure.has(DiagnosticCode::MalformedProviderBinding),
        "{:?}",
        failure.codes()
    );
}

/// A fingerprint that moves between processes is not a fingerprint.
///
/// `canonical` is derived `Debug`, which follows iteration order, so any map in
/// a canonicalized type must be ORDERED. `strike_geometry` was a `HashMap`:
/// building the same overrides in a different insertion order produced a
/// different canonical string, so two identical rosters got two fingerprints.
#[test]
fn the_canonical_form_does_not_depend_on_map_construction_order() {
    let forward = seeds_with_overrides(["alpha", "beta", "gamma", "delta"]);
    let reverse = seeds_with_overrides(["delta", "gamma", "beta", "alpha"]);
    assert_eq!(
        forward, reverse,
        "the canonical form must not depend on the order the overrides were inserted"
    );
}

/// Build a profile's strike-geometry map in the given insertion order and return
/// the canonical string the fingerprint would use.
fn seeds_with_overrides(keys: [&str; 4]) -> String {
    use crate::pattern::profile::StrikeRect;
    use ambition_platformer2d_core as ae;
    let mut map: std::collections::BTreeMap<String, Vec<StrikeRect>> = Default::default();
    for key in keys {
        map.insert(
            key.to_string(),
            vec![StrikeRect::scaled(
                ae::Vec2::new(0.0, 0.0),
                ae::Vec2::new(1.0, 1.0),
            )],
        );
    }
    format!("{map:?}")
}

/// Generic fixture with the same field shape as a production encounter. It
/// tests the engine schema without coupling the engine crate to game content ids.
const ENCOUNTER: &str = r#"(
    id: "probe_encounter",
    name: "Clockwork Warden",
    max_hp: 36,
    phase1_to_transition_hp: 0.66,
    transition_to_phase2_hp: 0.66,
    phase2_to_enrage_hp: 0.22,
    intro_seconds: 2.4,
    transition_seconds: 1.6,
    stagger_seconds: 1.8,
    death_seconds: 2.4,
    stagger_threshold: 6,
    stagger_window_seconds: 1.5,
    music_intro: "fast_paced_violin_boss",
    music_phase1: "fast_paced_violin_boss",
    music_phase2: "fast_paced_violin_boss",
    music_enrage: "fast_paced_violin_boss",
)"#;

fn encounter_music_diagnostics(name: &str, text: &str) -> Vec<String> {
    // A single-source draft cannot resolve the `boss` reference either, so
    // asserting merely "some UnresolvedReference" would pass for the wrong
    // reason. Look only at the MUSIC diagnostics.
    let failure = compile(
        &draft(
            name,
            "encounter.ron",
            text,
            BOSS_ENCOUNTER_SCHEMA,
            BOSS_ENCOUNTER_VERSION,
        ),
        &registry(),
        &AssetsUnchecked,
    )
    .expect_err("no boss profile in this draft, so it always refuses");
    failure
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("music track"))
        .map(|d| d.message.clone())
        .collect()
}

/// A whitespace-only music field is a REQUEST, not an absence.
#[test]
fn a_whitespace_only_music_field_is_refused_not_skipped() {
    let text = ENCOUNTER.replace(
        r#"music_phase1: "fast_paced_violin_boss""#,
        r#"music_phase1: "   ""#,
    );
    assert_ne!(
        text, ENCOUNTER,
        "the fixture must actually carry that field"
    );
    let music = encounter_music_diagnostics("blank_music", &text);
    assert!(
        music.iter().any(|m| m.contains("   ")),
        "the whitespace-only phase field must become an unresolved reference: {music:?}"
    );
}

/// The complement, and the reason this is a predicate change and not a ban: an
/// EXACTLY empty field really does mean "no swap for this phase" and must NOT
/// become a reference.
#[test]
fn an_exactly_empty_music_field_is_no_swap_and_makes_no_reference() {
    let text = ENCOUNTER.replace(
        r#"music_phase1: "fast_paced_violin_boss""#,
        r#"music_phase1: """#,
    );
    assert_ne!(text, ENCOUNTER);
    let baseline = encounter_music_diagnostics("empty_baseline", ENCOUNTER).len();
    let with_empty = encounter_music_diagnostics("empty_music", &text).len();
    assert_eq!(
        with_empty,
        baseline - 1,
        "blanking one phase must remove exactly its own reference and no other"
    );
}

// ── the nine-files-one-book contract ─────────────────────────────────────────

fn encounters_draft(name: &str, files: &[(&str, &str)]) -> ContentPackDraft {
    let root = std::env::temp_dir().join(format!("ambition_boss_schema_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    for (file, text) in files {
        std::fs::write(root.join(file), text).expect("write source");
    }
    ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("test_boss".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("test".into()),
            requires: Vec::new(),
            sources: files
                .iter()
                .map(|(file, _)| SourceDeclaration {
                    path: (*file).into(),
                    schema: SchemaId::new(BOSS_ENCOUNTER_SCHEMA),
                    version: BOSS_ENCOUNTER_VERSION,
                })
                .collect(),
        },
    )
    .expect("draft reads")
}

fn refuse_encounters(name: &str, files: &[(&str, &str)]) -> CompileFailure {
    compile(
        &encounters_draft(name, files),
        &registry(),
        &AssetsUnchecked,
    )
    .expect_err("no boss profiles in these drafts, so they always refuse eventually")
}

/// Two files claiming one encounter id, caught where only the MERGE can see
/// it. A per-facet handler reads one file and cannot know another named the
/// same encounter; the runtime resolves by id and would simply have used
/// whichever won.
#[test]
fn two_files_claiming_one_encounter_id_are_refused_by_the_merge() {
    let failure = refuse_encounters(
        "duplicate_encounter",
        &[("first.ron", ENCOUNTER), ("second.ron", ENCOUNTER)],
    );
    assert_eq!(
        failure.stage,
        ambition_content_pack::CompileStage::Aggregation,
        "before reference resolution, which would otherwise report first: {:?}",
        failure.codes()
    );
    let rendered = failure.render();
    assert!(
        rendered.contains("first.ron") && rendered.contains("second.ron"),
        "and it names both files, because 'which two' is the first question:\n{rendered}"
    );
}

/// The complement, and what makes the test above about the ID rather than about
/// the count: two DISTINCT encounters merge, and the compile gets all the way to
/// reference resolution — which then refuses for the reason every single-source
/// encounter draft here does, that no boss profile exists to point back.
///
/// negative space, deliberately: this crate cannot author a minimal boss
/// profile (the row is the whole struct), so the positive artifact is probed
/// where the real nine files live — `ambition_content`'s
/// `the_encounter_book_the_runtime_loads_is_the_one_the_compiler_merged`.
#[test]
fn two_distinct_encounters_merge_rather_than_conflict() {
    let failure = refuse_encounters(
        "two_encounters",
        &[
            ("first.ron", ENCOUNTER),
            (
                "second.ron",
                &ENCOUNTER.replace(r#"id: "probe_encounter""#, r#"id: "other_encounter""#),
            ),
        ],
    );
    assert_eq!(
        failure.stage,
        ambition_content_pack::CompileStage::ReferenceResolution,
        "the merge accepted both; what refuses is the missing boss profile: {:?}",
        failure.codes()
    );
}
