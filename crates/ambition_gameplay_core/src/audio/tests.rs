//! Unit + guardrail tests for the sandbox audio runtime: `SfxMessage`→cue
//! mapping, dB conversion, `AudioLibrary` loading from the real SFX bank /
//! music specs, and source/Cargo.toml guards that the retired fundsp
//! procedural renderer (and non-Kira audio backends) stay out, plus that
//! `web_audio` implies `audio`.

use super::*;
// `SfxMessage` no longer re-exported by the parent module (§D1).
use crate::session::data::{
    authored_music_registry, authored_sfx_registry, MusicRegistry, MusicTrack,
};
use ambition_sfx::SfxMessage;
use ambition_sfx::SfxProvider;

#[test]
fn amplitude_to_decibels_silent_floor() {
    assert_eq!(amplitude_to_decibels(0.0), -60.0);
    assert_eq!(amplitude_to_decibels(0.0005), -60.0);
    assert_eq!(amplitude_to_decibels(-1.0), -60.0);
}

#[test]
fn amplitude_to_decibels_unit_is_zero() {
    assert!((amplitude_to_decibels(1.0)).abs() < 1e-4);
}

#[test]
fn amplitude_to_decibels_half_is_minus_six() {
    let db = amplitude_to_decibels(0.5);
    assert!((db - (-6.0205)).abs() < 0.01);
}

#[test]
fn sfx_message_maps_to_sound_cue() {
    let pos = ae::Vec2::ZERO;
    assert_eq!(SfxMessage::Jump { pos }.cue(), Some(SoundCue::Jump));
    assert_eq!(
        SfxMessage::DoubleJump { pos }.cue(),
        Some(SoundCue::DoubleJump)
    );
    assert_eq!(SfxMessage::Dash { pos }.cue(), Some(SoundCue::Dash));
    assert_eq!(
        SfxMessage::Blink {
            pos,
            precision: false
        }
        .cue(),
        Some(SoundCue::Blink)
    );
    assert_eq!(
        SfxMessage::Blink {
            pos,
            precision: true
        }
        .cue(),
        Some(SoundCue::PrecisionBlink)
    );
    assert_eq!(SfxMessage::Pogo { pos }.cue(), Some(SoundCue::Pogo));
    assert_eq!(SfxMessage::Slash { pos }.cue(), Some(SoundCue::Slash));
    assert_eq!(SfxMessage::Hit { pos }.cue(), Some(SoundCue::Hit));
    assert_eq!(SfxMessage::Death { pos }.cue(), Some(SoundCue::Death));
    assert_eq!(SfxMessage::Reset { pos }.cue(), Some(SoundCue::Reset));
    assert_eq!(
        SfxMessage::Play {
            id: sfx::ids::PLAYER_JUMP,
            pos
        }
        .cue(),
        None
    );
}

#[test]
fn sfx_message_carries_position() {
    let pos = ae::Vec2::new(120.0, 64.0);
    if let SfxMessage::Hit { pos: at } = (SfxMessage::Hit { pos }) {
        assert_eq!(at, pos);
    } else {
        panic!("variant pattern match failed");
    }
}

#[test]
fn audio_library_loads_every_cue_from_real_bank() {
    let bank_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("audio")
        .join("sfx.bank");
    if !bank_path.exists() {
        eprintln!(
            "(skipped) {} missing — run `python3 tools/ambition_sfx_pack/pack.py`",
            bank_path.display()
        );
        return;
    }

    let provider = ambition_sfx::BankProvider::from_path(&bank_path).expect("load real bank");
    for cue in SoundCue::ALL {
        assert!(
            provider.has(cue.sfx_id()),
            "bank is missing entry for typed cue {:?} (id {})",
            cue,
            cue.sfx_id()
        );
    }

    let sfx = authored_sfx_registry().clone();
    let music = authored_music_registry().clone();
    let mut assets = Assets::<KiraAudioSource>::default();
    let library = AudioLibrary::new(&mut assets, &sfx, &music, None, Some(&provider), None);

    for cue in SoundCue::ALL {
        let handle = library.sfx_handle(cue);
        assert_ne!(
            handle,
            Handle::<KiraAudioSource>::default(),
            "no handle produced for {cue:?}"
        );
    }
}

/// Every embedded music track resolves to a `.ogg` under `audio/music/`.
/// The path is derived from the id (or an explicit override) — procedural
/// music generation was retired, so a non-OGG / off-tree path would mean a
/// silent radio entry.
#[test]
fn embedded_music_tracks_resolve_to_ogg_assets() {
    let music = authored_music_registry().clone();
    for track in &music.tracks {
        let path = track.resolved_asset_path();
        assert!(
            path.starts_with("audio/music/") && path.ends_with(".ogg"),
            "music track '{}' resolved to an unexpected asset path '{path}'",
            track.id,
        );
    }
}

/// A track with no explicit `asset_path` derives the conventional
/// `audio/music/generated/<id>/full.ogg`; an explicit override wins. The
/// library builds one runtime track per registry entry — there is no
/// skip/None path anymore.
#[test]
fn audio_library_resolves_default_and_override_paths() {
    let sfx = authored_sfx_registry().clone();
    let music = MusicRegistry {
        default_track: "convention".into(),
        tracks: vec![
            MusicTrack {
                id: "convention".into(),
                display_name: "Convention".into(),
                asset_path: None,
            },
            MusicTrack {
                id: "override".into(),
                display_name: "Override".into(),
                asset_path: Some("audio/music/x.ogg".into()),
            },
        ],
    };

    assert_eq!(
        music.tracks[0].resolved_asset_path(),
        "audio/music/generated/convention/full.ogg"
    );
    assert_eq!(music.tracks[1].resolved_asset_path(), "audio/music/x.ogg");

    let mut assets = Assets::<KiraAudioSource>::default();
    let library = AudioLibrary::new(&mut assets, &sfx, &music, None, None, None);
    assert_eq!(library.track_count(), 2);
    assert!(library.track("convention").is_some());
    assert!(library.track("override").is_some());
}

#[test]
fn embedded_audio_catalog_includes_tech_bro_banger_tracks() {
    let music = authored_music_registry().clone();
    for id in [
        "pivot_protocol",
        "minimum_viable_apocalypse",
        "terms_and_conditions",
        "burn_rate_bossa",
        "shareholder_ritual",
    ] {
        let track = music
            .track(id)
            .unwrap_or_else(|| panic!("missing music track {id}"));
        let expected_path = format!("audio/music/generated/{id}/full.ogg");
        assert_eq!(track.resolved_asset_path(), expected_path);
    }
}

#[test]
fn music_track_order_cycles() {
    let sfx = authored_sfx_registry().clone();
    let music = authored_music_registry().clone();
    let mut assets = Assets::<KiraAudioSource>::default();
    let library = AudioLibrary::new(&mut assets, &sfx, &music, None, None, None);
    let ids: Vec<&str> = music.tracks.iter().map(|track| track.id.as_str()).collect();
    assert_eq!(library.track_count(), ids.len());
    assert!(
        ids.len() >= 2,
        "cycle test needs at least 2 tracks, got {}",
        ids.len()
    );
    // The registry leads with the curated special entries; `original_lofi_loop`
    // is pinned first (the fallback the library special-cases). The default
    // `long_lofi_drift` must also be present.
    assert_eq!(ids[0], ORIGINAL_TRACK_ID);
    assert_eq!(music.default_track, "long_lofi_drift");
    assert!(ids.contains(&"long_lofi_drift"));

    // Forward step from the head lands on the second entry; back round-trips.
    assert_eq!(library.next_track_id(ORIGINAL_TRACK_ID), Some(ids[1]));
    assert_eq!(library.previous_track_id(ids[1]), Some(ORIGINAL_TRACK_ID));
    // Cycle wraps: next of the last track is the first.
    let last = *ids.last().expect("non-empty list");
    assert_eq!(library.next_track_id(last), Some(ORIGINAL_TRACK_ID));
    // Cycle wraps the other way too.
    assert_eq!(library.previous_track_id(ORIGINAL_TRACK_ID), Some(last));
}

/// Live-runtime guardrail: the audio module must not gain a new
/// fundsp / procedural-music reference. The renderer was retired
/// (see `docs/archive/retired/fundsp-audio.md`) and re-introducing it would silently
/// resurrect the dead code paths the rest of this task tore out.
///
/// This walks the `audio/*.rs` source tree at test time and rejects
/// any of the historical sentinel identifiers. Comments referencing
/// the deletion are fine (they live in docs / EOL notes), but a *use*
/// or *fn definition* would re-introduce them.
#[test]
fn no_runtime_references_to_retired_procedural_renderer() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/audio");
    let banned: &[&str] = &[
        "use fundsp",
        "fundsp::",
        "render_lofi_theme(",
        "render_sfx_with_fundsp_osc(",
        "TrackSource::Procedural",
    ];
    let mut findings: Vec<String> = Vec::new();
    let entries =
        std::fs::read_dir(&root).unwrap_or_else(|e| panic!("read_dir {}: {e}", root.display()));
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for (lineno, line) in contents.lines().enumerate() {
            for needle in banned {
                if line.contains(needle) {
                    findings.push(format!(
                        "{}:{}: contains `{needle}` -> {line}",
                        path.display(),
                        lineno + 1
                    ));
                }
            }
        }
    }
    // This test file itself names the sentinels in `banned` and
    // emits findings for *every* mention. Filter out matches inside
    // this very test source so the assertion only fires on real
    // re-introductions in sibling files.
    let this_file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/audio/tests.rs");
    findings.retain(|line| !line.starts_with(&this_file.display().to_string()));
    assert!(
        findings.is_empty(),
        "runtime audio module re-introduced retired procedural / fundsp \
         references:\n{}\n\nThe fundsp procedural renderer was retired in \
         favor of authored OGGs + the SFX bank (see docs/archive/retired/fundsp-audio.md). \
         If a new realtime DSP/effects layer is wanted, gate it behind an \
         `audio_fx` feature and a separate module — do not re-thread it \
         through the runtime audio module.",
        findings.join("\n")
    );
}

/// Cargo-level guardrail: the sandbox crate's own `Cargo.toml` must
/// not list `fundsp` as a runtime dependency or feature input. Pairs
/// with `no_runtime_references_to_retired_procedural_renderer` —
/// that one catches `use fundsp::` *inside* a `.rs` file, this one
/// catches a `fundsp = "..."` line that hasn't been called yet but
/// would silently re-arm the procedural path. Comments are stripped
/// before scanning so the existing "fundsp was retired" prose
/// blocks pass.
#[test]
fn ambition_gameplay_core_cargo_toml_has_no_fundsp_dep() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let contents = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest.display()));
    let mut violations: Vec<String> = Vec::new();
    for (lineno, raw_line) in contents.lines().enumerate() {
        let line = raw_line
            .split_once('#')
            .map(|(code, _comment)| code)
            .unwrap_or(raw_line)
            .trim();
        if line.is_empty() {
            continue;
        }
        if line.contains("fundsp") {
            violations.push(format!(
                "{}:{}: {}",
                manifest.display(),
                lineno + 1,
                raw_line
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "ambition_gameplay_core/Cargo.toml re-introduced `fundsp` outside \
         documentation comments:\n{}\n\n`fundsp` was retired as a \
         runtime audio backend. The new realtime DSP layer must go \
         through Kira (see docs/archive/retired/fundsp-audio.md).",
        violations.join("\n")
    );
}

/// Cargo-level guardrail: `web_audio` MUST imply `audio`, not just
/// `authored_audio`. The source uses `#[cfg(feature = "audio")]`
/// gates everywhere; if `web_audio` only enables `authored_audio`,
/// `bevy_kira_audio` is in the dep graph but every audio runtime
/// module is compiled out — the wasm boots silent and the only
/// symptom is "no `[ambition-audio] AudioContext created` log even
/// though the boot banner says `web_served_assets`". That is exactly
/// the regression Jon hit; pin it in CI.
#[test]
fn web_audio_feature_implies_audio_feature() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let contents = std::fs::read_to_string(&manifest).expect("read sandbox Cargo.toml");
    let mut in_features = false;
    let mut web_audio_line: Option<String> = None;
    for raw in contents.lines() {
        let stripped = raw.split_once('#').map(|(c, _)| c).unwrap_or(raw).trim();
        if stripped.starts_with('[') {
            in_features = stripped == "[features]";
            continue;
        }
        if !in_features {
            continue;
        }
        if let Some((name, rest)) = stripped.split_once('=') {
            if name.trim() == "web_audio" {
                web_audio_line = Some(rest.to_string());
            }
        }
    }
    let rhs = web_audio_line.expect(
        "Cargo.toml lost the `web_audio` feature definition — the web build pipeline \
         depends on it (build_for_web.sh --served and the web_served_assets composite).",
    );
    let has_audio = rhs.contains("\"audio\"");
    assert!(
        has_audio,
        "web_audio must include the `audio` feature, not just `authored_audio`. \
         Found: web_audio = {rhs}\n\nWithout this, every `#[cfg(feature = \"audio\")]` \
         gate in the audio runtime is false on web builds and the wasm boots silent \
         (no kira plugin install, no AudioContext, no music, no SFX). See \
         docs/recipes/web-audio-manual-test.md and src/audio/web_unlock.rs."
    );
}

/// Cargo-level guardrail: a runtime-DSP layer must compose with
/// Kira, not bypass it. Re-introducing a non-Kira playback path
/// would split the audio graph (mixer / underwater effect / unlock
/// telemetry all live on the Kira side), so the only audio backend
/// the sandbox is allowed to pull is `bevy_kira_audio`.
#[test]
fn ambition_gameplay_core_uses_only_bevy_kira_audio() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let contents = std::fs::read_to_string(&manifest).expect("read sandbox Cargo.toml");
    // Other Bevy audio integrations we want to refuse silently
    // sneaking in via `bevy_audio` or alternative wrappers.
    let banned: &[&str] = &["bevy_audio", "bevy_oddio", "oddio"];
    for needle in banned {
        for (lineno, raw_line) in contents.lines().enumerate() {
            let stripped = raw_line.split_once('#').map(|(c, _)| c).unwrap_or(raw_line);
            // Allow `default-features = false` Bevy mentions that may
            // include the word in the disable list. We only care
            // about `<crate> = "..."` style dep declarations.
            let is_dep_decl = stripped.contains(needle)
                && stripped.contains('=')
                && !stripped.contains("default-features");
            if is_dep_decl {
                panic!(
                    "{}:{}: introduced alternative audio backend `{needle}` -> {raw_line}",
                    manifest.display(),
                    lineno + 1
                );
            }
        }
    }
}

/// Live-runtime guardrail: every music track in the embedded sandbox
/// spec must have a `WebServedAssets`-resolvable catalog path. This
/// pins the "music works on the served-web profile" contract — if a
/// new track lands without an `asset_path`, the catalog drops it and
/// this test fails loudly instead of letting the radio silently lose
/// an entry on web.
#[test]
fn every_live_music_track_resolves_under_web_served_assets() {
    use crate::assets::game_assets::GameAssetConfig;
    use ambition_asset_manager::AssetProfile;

    let music = authored_music_registry().clone();
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::WebServedAssets;
    let catalog = crate::assets::sandbox_assets::build_sandbox_catalog(&config, &music);

    let mut missing: Vec<String> = Vec::new();
    for track in &music.tracks {
        let id = crate::assets::sandbox_assets::ids::music_track(&track.id);
        let path = catalog.path_for(&id);
        if path.is_none() {
            missing.push(track.id.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "music tracks without a WebServedAssets-resolvable path: {missing:?}. \
         Either add a pre-rendered OGG or remove the track from \
         music_registry.ron — the procedural fallback is retired."
    );
}
