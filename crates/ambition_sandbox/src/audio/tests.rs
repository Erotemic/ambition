use super::*;
use crate::data::SandboxDataSpec;

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

    let spec = SandboxDataSpec::load_embedded();
    let mut assets = Assets::<KiraAudioSource>::default();
    let library = AudioLibrary::new(&mut assets, &spec.audio, None, Some(&provider), None);

    for cue in SoundCue::ALL {
        let handle = library.sfx_handle(cue);
        assert_ne!(
            handle,
            Handle::<KiraAudioSource>::default(),
            "no handle produced for {cue:?}"
        );
    }
}

/// Every embedded music track must declare an `asset_path`. Procedural
/// music generation was retired in favor of pre-rendered OGGs; a track
/// with `asset_path = None` would silently disappear from the radio.
#[test]
fn embedded_music_tracks_all_have_asset_paths() {
    let spec = SandboxDataSpec::load_embedded();
    for track in &spec.audio.music_tracks {
        assert!(
            track.asset_path.is_some(),
            "music track '{}' has no asset_path. Author a pre-rendered OGG \
             (tools/ambition_music_renderer) or remove the track — the runtime no \
             longer falls back to procedural synthesis.",
            track.id,
        );
    }
}

/// `AudioLibrary` only exposes music tracks that actually have a
/// resolvable path. The build skips any spec row missing `asset_path`,
/// so the radio menu never offers a silent entry.
#[test]
fn audio_library_skips_music_tracks_without_asset_path() {
    use crate::data::{
        AudioSpec, MusicGainsSpec, MusicSpec, MusicTrackSpec, SfxSpec, WaveformSpec, SoundCueKey,
    };

    fn synthetic_arrangement() -> MusicSpec {
        MusicSpec {
            bpm: 72.0,
            total_beats: 32.0,
            root_hz: 220.0,
            bass_root_hz: 110.0,
            key_root_hz: 220.0,
            master_gain: 0.5,
            lowpass_alpha: 0.5,
            tape_hiss: 0.0,
            lead: Vec::new(),
            chords: vec![[0, 4, 7, 11]],
            bass_roots: vec![0],
            gains: MusicGainsSpec {
                chord_pad: 1.0,
                lead: 1.0,
                soft_keys: 1.0,
                bass: 1.0,
                drums: 1.0,
            },
        }
    }

    let spec = AudioSpec {
        sample_rate: 44_100,
        sfx: vec![SfxSpec {
            cue: SoundCueKey::Jump,
            waveform: WaveformSpec::Sine,
            frequency: 440.0,
            frequency_end: 440.0,
            duration: 0.05,
            volume: 0.2,
            attack: 0.003,
            release: 0.04,
            noise: 0.0,
        }],
        default_music_track: "with_path".into(),
        music_tracks: vec![
            MusicTrackSpec {
                id: "with_path".into(),
                display_name: "With path".into(),
                arrangement: synthetic_arrangement(),
                asset_path: Some("audio/music/x.ogg".into()),
            },
            MusicTrackSpec {
                id: "no_path".into(),
                display_name: "No path".into(),
                arrangement: synthetic_arrangement(),
                asset_path: None,
            },
        ],
    };

    let mut assets = Assets::<KiraAudioSource>::default();
    let library = AudioLibrary::new(&mut assets, &spec, None, None, None);
    assert_eq!(library.track_count(), 1);
    assert!(library.track("with_path").is_some());
    assert!(library.track("no_path").is_none());
}

#[test]
fn music_track_order_cycles() {
    let spec = SandboxDataSpec::load_embedded();
    let mut assets = Assets::<KiraAudioSource>::default();
    let library = AudioLibrary::new(&mut assets, &spec.audio, None, None, None);
    let ids: Vec<&str> = spec
        .audio
        .music_tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect();
    assert_eq!(library.track_count(), ids.len());
    assert!(
        ids.len() >= 2,
        "cycle test needs at least 2 tracks, got {}",
        ids.len()
    );
    // Pin only the head of the list — the seed tracks the radio ships
    // with. Adding tracks after these must not break this test.
    assert_eq!(ids[0], ORIGINAL_TRACK_ID);
    assert_eq!(ids[1], "long_lofi_drift");

    // Forward step from the head.
    assert_eq!(library.next_track_id(ORIGINAL_TRACK_ID), Some(ids[1]));
    // Backward step round-trips with forward step.
    assert_eq!(library.previous_track_id(ids[1]), Some(ORIGINAL_TRACK_ID),);
    // Cycle wraps: next of the last track is the first.
    let last = *ids.last().expect("non-empty list");
    assert_eq!(library.next_track_id(last), Some(ORIGINAL_TRACK_ID));
    // Cycle wraps the other way too.
    assert_eq!(library.previous_track_id(ORIGINAL_TRACK_ID), Some(last));
}

/// Live-runtime guardrail: the audio module must not gain a new
/// fundsp / procedural-music reference. The renderer was retired
/// (see `docs/fundsp_audio.md`) and re-introducing it would silently
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
    let entries = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", root.display()));
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
    let this_file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/audio/tests.rs");
    findings.retain(|line| !line.starts_with(&this_file.display().to_string()));
    assert!(
        findings.is_empty(),
        "runtime audio module re-introduced retired procedural / fundsp \
         references:\n{}\n\nThe fundsp procedural renderer was retired in \
         favor of authored OGGs + the SFX bank (see docs/fundsp_audio.md). \
         If a new realtime DSP/effects layer is wanted, gate it behind an \
         `audio_fx` feature and a separate module — do not re-thread it \
         through the runtime audio module.",
        findings.join("\n")
    );
}

/// Live-runtime guardrail: every music track in the embedded sandbox
/// spec must have a `WebServedAssets`-resolvable catalog path. This
/// pins the "music works on the served-web profile" contract — if a
/// new track lands without an `asset_path`, the catalog drops it and
/// this test fails loudly instead of letting the radio silently lose
/// an entry on web.
#[test]
fn every_live_music_track_resolves_under_web_served_assets() {
    use crate::data::SandboxDataSpec;
    use crate::game_assets::GameAssetConfig;
    use ambition_asset_manager::AssetProfile;

    let spec = SandboxDataSpec::load_embedded();
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::WebServedAssets;
    let catalog = crate::sandbox_assets::build_sandbox_catalog(&config, &spec.audio);

    let mut missing: Vec<String> = Vec::new();
    for track in &spec.audio.music_tracks {
        let id = crate::sandbox_assets::ids::music_track(&track.id);
        let path = catalog.path_for(&id);
        if path.is_none() {
            missing.push(track.id.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "music tracks without a WebServedAssets-resolvable path: {missing:?}. \
         Either add a pre-rendered OGG (asset_path: Some(\"audio/music/...\")) \
         or remove the track from sandbox.ron — the procedural fallback is \
         retired."
    );
}
