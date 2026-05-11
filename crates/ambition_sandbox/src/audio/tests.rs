use super::render::render_lofi_theme;
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
    let library = AudioLibrary::new(&mut assets, &spec.audio, None, Some(&provider));

    for cue in SoundCue::ALL {
        let handle = library.sfx_handle(cue);
        assert_ne!(
            handle,
            Handle::<KiraAudioSource>::default(),
            "no handle produced for {cue:?}"
        );
    }
}

#[test]
fn embedded_music_renders_expected_durations() {
    let spec = SandboxDataSpec::load_embedded();
    let sample_rate = 8_000;
    let original = &spec
        .audio
        .track(ORIGINAL_TRACK_ID)
        .expect("original track exists")
        .arrangement;
    let long = &spec
        .audio
        .track("long_lofi_drift")
        .expect("long track exists")
        .arrangement;

    let original_render = render_lofi_theme(original, sample_rate);
    let long_render = render_lofi_theme(long, sample_rate);
    assert!((original_render.duration_seconds() - original.duration_seconds()).abs() < 0.01);
    assert!((long_render.duration_seconds() - long.duration_seconds()).abs() < 0.01);
    assert!(long_render.frames.len() > original_render.frames.len() * 3);
}

#[test]
fn long_track_authors_full_chord_and_bass_phrase() {
    let spec = SandboxDataSpec::load_embedded();
    let long = &spec
        .audio
        .track("long_lofi_drift")
        .expect("long track exists")
        .arrangement;
    assert_eq!(long.chords.len(), long.bar_count());
    assert_eq!(long.bass_roots.len(), long.bar_count());
    assert!(long.chords.windows(2).any(|pair| pair[0] != pair[1]));
    assert!(long.bass_roots.windows(2).any(|pair| pair[0] != pair[1]));
    assert_ne!(long.chords[0], *long.chords.last().unwrap());
    assert_ne!(long.bass_roots[0], *long.bass_roots.last().unwrap());
}

#[test]
fn music_track_order_cycles() {
    let spec = SandboxDataSpec::load_embedded();
    let mut assets = Assets::<KiraAudioSource>::default();
    let library = AudioLibrary::new(&mut assets, &spec.audio, None, None);
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

#[test]
fn preview_wav_writer_produces_riff_wave_header() {
    let spec = SandboxDataSpec::load_embedded();
    let track = spec
        .audio
        .track(ORIGINAL_TRACK_ID)
        .expect("original track exists");
    let rendered = render_music_preview(track, 8_000);
    assert!(rendered.duration_seconds() > 0.0);
    let wav = wav_bytes_from_rendered_audio(&rendered);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[12..16], b"fmt ");
    assert_eq!(&wav[36..40], b"data");
    assert!(wav.len() > 44);
}

#[test]
fn example_tune_parses_and_renders() {
    let track: MusicTrackSpec = ron::from_str(include_str!(
        "../../assets/ambition/tune_examples/example_drift.ron"
    ))
    .expect("example tune parses");
    track
        .arrangement
        .validate()
        .expect("example tune validates");
    let rendered = render_music_preview(&track, 8_000);
    assert!(rendered.duration_seconds() > 0.0);
}
