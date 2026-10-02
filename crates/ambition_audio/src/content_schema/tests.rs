//! Probes for the two audio schemas.

use super::*;
use ambition_content_pack::{
    compile, AssetsUnchecked, CompileFailure, ContentPackDraft, ContentPackManifest, FixedAssets,
    ModuleNamespace, PackId, PackVersion, SchemaRegistry, SourceDeclaration,
};

const MUSIC: &str = r#"(
    default_track: "theme",
    tracks: [
        (id: "theme", display_name: "Theme"),
        (id: "fanfare", display_name: "Fanfare", one_shot: true),
    ],
)"#;

/// The shipped file's shape: a typed `cue` or an open `id`, never both.
const SFX: &str = r#"(
    sample_rate: 44100,
    sfx: [
        (cue: Some(Jump), waveform: Sine, frequency: 460.0, frequency_end: 720.0, duration: 0.085, volume: 0.22, attack: 0.003, release: 0.045, noise: 0.0),
        (id: Some("ui.menu.accept"), waveform: Sine, frequency: 620.0, frequency_end: 920.0, duration: 0.090, volume: 0.18, attack: 0.002, release: 0.040, noise: 0.0),
    ],
)"#;

fn registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry
        .register(music_registry_schema())
        .expect("fresh registry");
    registry
        .register(sfx_registry_schema())
        .expect("fresh registry");
    registry
        .register(music_cue_catalog_schema())
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
    let root = std::env::temp_dir().join(format!("ambition_audio_schema_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    std::fs::write(root.join(file), text).expect("write source");
    ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("test_audio".into()),
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

fn music_draft(name: &str, text: &str) -> ContentPackDraft {
    draft(
        name,
        "music.ron",
        text,
        MUSIC_REGISTRY_SCHEMA,
        MUSIC_REGISTRY_VERSION,
    )
}

fn refuse_music(name: &str, text: &str) -> CompileFailure {
    compile(&music_draft(name, text), &registry(), &AssetsUnchecked)
        .expect_err("this registry must be refused")
}

#[test]
fn a_compiled_pack_carries_the_music_registry_the_runtime_will_load() {
    let pack = compile(
        &music_draft("lowering", MUSIC),
        &registry(),
        &AssetsUnchecked,
    )
    .expect("a well-formed registry compiles");
    let music = lowered_music_registry(&pack).expect("a Runtime schema lowers its artifact");
    assert_eq!(music.default_track, "theme");
    assert_eq!(music.tracks.len(), 2);
}

#[test]
fn a_compiled_pack_carries_the_sfx_registry_the_runtime_will_load() {
    let d = draft(
        "sfx",
        "sfx.ron",
        SFX,
        SFX_REGISTRY_SCHEMA,
        SFX_REGISTRY_VERSION,
    );
    let pack = compile(&d, &registry(), &AssetsUnchecked).expect("sfx compiles");
    let sfx = lowered_sfx_registry(&pack).expect("a Runtime schema lowers its artifact");
    assert_eq!(sfx.sample_rate, 44100);
}

/// Validation rejects a `default_track` that names no authored track.
#[test]
fn a_default_track_naming_no_track_is_refused() {
    let text = MUSIC.replace(r#"default_track: "theme""#, r#"default_track: "missing""#);
    let failure = refuse_music("dangling_default", &text);
    assert!(
        failure.has(DiagnosticCode::MalformedProviderBinding),
        "{:?}",
        failure.codes()
    );
}

/// `validate()` cannot see this one: serde has already dropped the field by the
/// time it runs. This is what the schema adds over the existing validator.
#[test]
fn an_unknown_authored_field_is_an_error_and_not_a_shrug() {
    let text = MUSIC.replace(
        r#"(id: "theme", display_name: "Theme")"#,
        r#"(id: "theme", display_name: "Theme", bpm: 120)"#,
    );
    let failure = refuse_music("unknown_field", &text);
    assert!(
        failure.has(DiagnosticCode::UnknownField),
        "{:?}",
        failure.codes()
    );
}

/// The asset half nothing checked before. `AudioCatalogFragment` validates
/// the registry's shape and never asks whether the OGG it points at exists.
#[test]
fn a_track_whose_audio_file_is_missing_is_reported() {
    // Only the conventional path for `theme` is present; `fanfare`'s is not.
    let assets = FixedAssets::new(["audio/music/generated/theme/full.ogg"]);
    let failure = compile(&music_draft("missing_ogg", MUSIC), &registry(), &assets)
        .expect_err("a missing track file is refused under a strict asset source");
    assert!(
        failure.has(DiagnosticCode::MissingAsset),
        "the track's audio file is a declared requirement: {:?}",
        failure.codes()
    );
}

/// The conventional path is derived from the id when `asset_path` is absent, so
/// the requirement must follow that convention rather than only explicit paths.
#[test]
fn both_tracks_resolve_when_their_conventional_files_exist() {
    let assets = FixedAssets::new([
        "audio/music/generated/theme/full.ogg",
        "audio/music/generated/fanfare/full.ogg",
    ]);
    compile(&music_draft("both_present", MUSIC), &registry(), &assets)
        .expect("every track's conventional file is there");
}

// ── the fingerprint covers REGISTRY-LEVEL state, not only rows ───────────────
//
// The pack fingerprint is taken over `out.define(...)` entries only. Defining
// one entry per track/cue left `default_track`, the track ORDER, and
// `sample_rate` outside the pack's identity — so two packs that start on
// different music, sequence the radio differently, or synthesize at a different
// rate were indistinguishable to a cache or a session-compatibility check.

fn music_fingerprint(name: &str, text: &str) -> u64 {
    compile(&music_draft(name, text), &registry(), &AssetsUnchecked)
        .expect("compiles")
        .fingerprint
        .0
}

#[test]
fn changing_only_the_default_track_moves_the_fingerprint() {
    let other = MUSIC.replace(r#"default_track: "theme""#, r#"default_track: "fanfare""#);
    assert_ne!(
        music_fingerprint("default_base", MUSIC),
        music_fingerprint("default_moved", &other),
        "the track the game starts on is part of what the pack IS"
    );
}

#[test]
fn changing_only_the_track_order_moves_the_fingerprint() {
    let reordered = r#"(
    default_track: "theme",
    tracks: [
        (id: "fanfare", display_name: "Fanfare", one_shot: true),
        (id: "theme", display_name: "Theme"),
    ],
)"#;
    assert_ne!(
        music_fingerprint("order_base", MUSIC),
        music_fingerprint("order_moved", reordered),
        "order drives radio next/prev (`music_tracks[next]`), so it is semantic"
    );
}

#[test]
fn changing_only_the_sfx_sample_rate_moves_the_fingerprint() {
    let at = |name: &str, text: &str| {
        let d = draft(
            name,
            "sfx.ron",
            text,
            SFX_REGISTRY_SCHEMA,
            SFX_REGISTRY_VERSION,
        );
        compile(&d, &registry(), &AssetsUnchecked)
            .expect("compiles")
            .fingerprint
            .0
    };
    let other = SFX.replace("sample_rate: 44100", "sample_rate: 22050");
    assert_ne!(
        at("rate_base", SFX),
        at("rate_moved", &other),
        "sample_rate changes every procedurally synthesized cue"
    );
}

/// The complement, and the reason the fingerprint is worth having: reflowing the
/// file must NOT move it.
#[test]
fn reformatting_the_registry_does_not_move_the_fingerprint() {
    let reflowed = MUSIC.replace(
        "\n    tracks: [",
        "\n\n    // a comment nobody reads\n    tracks: [",
    );
    assert_eq!(
        music_fingerprint("reflow_base", MUSIC),
        music_fingerprint("reflow_moved", &reflowed),
        "a comment is not content"
    );
}

/// A delimiter is not a serialization. Track ids need only be non-empty
/// and unique, so commas are legal; `join(",")` let two different orders encode
/// identically while the per-track entries stayed the same, holding the whole
/// fingerprint still.
#[test]
fn two_orders_of_comma_bearing_track_ids_do_not_collide() {
    let pack = |name: &str, ids: [&str; 4]| {
        let tracks = ids
            .iter()
            .map(|id| format!(r#"(id: "{id}", display_name: "n")"#))
            .collect::<Vec<_>>()
            .join(",");
        // Hold `default_track` constant so track ordering is the only difference
        // between the two fingerprints.
        let text = format!("(default_track: \"a\", tracks: [{tracks}])");
        music_fingerprint(name, &text)
    };
    assert_ne!(
        pack("collide_a", ["a", "b,c", "a,b", "c"]),
        pack("collide_b", ["a,b", "c", "a", "b,c"]),
        "both used to flatten to `a,b,c,a,b,c`"
    );
}

// ── music_cue_catalog ────────────────────────────────────────────────────────

/// A cue with two sections, one layer and two states, and one binding.
const CUES: &str = r#"(
    cues: [(
        id: "lab",
        asset_root: "audio/music/generated/lab",
        bpm: 120.0,
        beats_per_bar: 4.0,
        relative_volume: 1.0,
        sections: [
            (id: "fight", duration_beats: 32.0, looped: true, sources: [(layer_id: "full", path: "fight.ogg")]),
            (id: "done", duration_beats: 8.0, looped: false, sources: [(layer_id: "full", path: "done.ogg")]),
        ],
        layers: [(id: "full", slot: 0)],
        states: [
            (id: "fight", section_id: "fight", gains: [(layer_id: "full", gain: 1.0)]),
            (id: "done", section_id: "done", gains: [(layer_id: "full", gain: 1.0)]),
        ],
        outro_state: Some("done"),
    )],
    encounter_bindings: [(
        encounter_id: "lab_fight",
        cue_id: "lab",
        starting_state: "fight",
        wave_states: ["fight"],
        cleared_state: "done",
    )],
)"#;

fn cue_draft(name: &str, text: &str) -> ContentPackDraft {
    draft(name, "cues.ron", text, MUSIC_CUE_CATALOG_SCHEMA, MUSIC_CUE_CATALOG_VERSION)
}

/// The cue file with one edit, which must apply exactly once.
fn cues_with(from: &str, to: &str) -> String {
    assert_eq!(CUES.matches(from).count(), 1, "the edit anchor {from:?}");
    CUES.replace(from, to)
}

fn refuse_cues(name: &str, text: &str) -> CompileFailure {
    compile(&cue_draft(name, text), &registry(), &AssetsUnchecked)
        .expect_err("this cue file must be refused")
}

#[test]
fn a_compiled_pack_carries_the_cues_and_bindings_the_director_will_load() {
    let pack = compile(&cue_draft("cues", CUES), &registry(), &AssetsUnchecked)
        .expect("a well-formed cue file compiles");
    let cues = lowered_music_cues(&pack).expect("a Runtime schema lowers its artifact");
    assert_eq!(cues.cues.len(), 1);
    assert_eq!(cues.cues[0].post_clear_bridge_state, None, "an absent option is None");
    assert_eq!(cues.encounter_bindings[0].cue_id, "lab");
    assert_eq!(cues.encounter_bindings[0].wave2_reinforced_state, None);
}

/// The director's own reference rules refuse the pack, not only the runtime
/// registry.
#[test]
fn a_reference_to_nothing_refuses_the_cue_file() {
    for (name, from, to) in [
        ("cue_section", r#"(id: "done", section_id: "done""#, r#"(id: "done", section_id: "gone""#),
        ("cue_binding_state", r#"cleared_state: "done""#, r#"cleared_state: "gone""#),
        ("cue_binding_cue", r#"cue_id: "lab""#, r#"cue_id: "gone""#),
    ] {
        let failure = refuse_cues(name, &cues_with(from, to));
        assert!(failure.has(DiagnosticCode::MalformedSource), "{name}: {:?}", failure.codes());
    }
}

/// The numbers the director reads as a tempo, a length, a channel and a
/// volume.
#[test]
fn a_cue_the_director_cannot_play_refuses_the_cue_file() {
    for (name, from, to) in [
        ("cue_bpm", "bpm: 120.0", "bpm: 0.0"),
        ("cue_length", "duration_beats: 8.0", "duration_beats: -8.0"),
        ("cue_slot", "slot: 0", "slot: 6"),
        ("cue_gain", r#"(id: "done", section_id: "done", gains: [(layer_id: "full", gain: 1.0)])"#,
            r#"(id: "done", section_id: "done", gains: [(layer_id: "full", gain: -1.0)])"#),
    ] {
        let failure = refuse_cues(name, &cues_with(from, to));
        assert!(failure.has(DiagnosticCode::MalformedSource), "{name}: {:?}", failure.codes());
    }
}

#[test]
fn a_misspelt_cue_field_is_refused() {
    let failure = refuse_cues("cue_unknown", &cues_with("looped: false", "loops: false"));
    assert!(failure.has(DiagnosticCode::UnknownField), "{:?}", failure.codes());
}

#[test]
fn one_encounter_bound_twice_is_refused() {
    let second = r#"encounter_bindings: [(
        encounter_id: "lab_fight",
        cue_id: "lab",
        starting_state: "done",
        wave_states: [],
        cleared_state: "done",
    ), ("#;
    let failure = refuse_cues("cue_twice", &cues_with("encounter_bindings: [(", second));
    assert!(failure.has(DiagnosticCode::DuplicateIdentity), "{:?}", failure.codes());
}

/// Each section's audio file is a declared requirement.
#[test]
fn a_cue_section_whose_audio_file_is_missing_is_reported() {
    let some = FixedAssets::new(["audio/music/generated/lab/fight.ogg"]);
    let failure = compile(&cue_draft("cue_missing_ogg", CUES), &registry(), &some)
        .expect_err("a missing section file is refused under a strict asset source");
    assert!(failure.has(DiagnosticCode::MissingAsset), "{:?}", failure.codes());
    let all = FixedAssets::new([
        "audio/music/generated/lab/fight.ogg",
        "audio/music/generated/lab/done.ogg",
    ]);
    compile(&cue_draft("cue_all_ogg", CUES), &registry(), &all).expect("every section file is there");
}
