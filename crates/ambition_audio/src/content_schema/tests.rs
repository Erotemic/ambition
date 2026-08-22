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
