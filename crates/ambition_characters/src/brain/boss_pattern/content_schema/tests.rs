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
