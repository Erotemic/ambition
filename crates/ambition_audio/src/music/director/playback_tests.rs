use super::*;
use std::time::Duration;
use crate::cue_spec::{MusicLayerGainSpec, MusicLayerSourceSpec, MusicLayerSpec, MusicSectionSpec};
use crate::music::catalog::MusicSourceKey;
use crate::selection::MusicAuthority;
use bevy::asset::AssetApp;
use bevy_kira_audio::prelude::{Frame, StaticSoundData, StaticSoundSettings};

fn fixture() -> (App, Vec<Handle<KiraAudioSource>>) {
    let mut app = App::new();
    app.add_plugins((bevy::app::TaskPoolPlugin::default(), bevy::asset::AssetPlugin::default()))
        .init_asset::<KiraAudioSource>()
        .init_resource::<DynamicAudioChannels>()
        .init_resource::<Time<Real>>()
        .init_resource::<MusicMix>()
        .init_resource::<MusicDirectorState>()
        .insert_resource(crate::output::AudioOutputMode::Recording);
    crate::music::add_music_playback(&mut app);
    let cue = MusicCueSpec {
        id: "trail".into(), asset_root: "content://library/music".into(),
        bpm: 60.0, beats_per_bar: 4.0, relative_volume: 1.0,
        sections: [("intro", false), ("song", true), ("later", true)].into_iter().map(|(id, looped)| {
            MusicSectionSpec {
                id: id.into(), duration_beats: 4.0, looped,
                sources: ["bed", "tune"].into_iter().map(|layer| MusicLayerSourceSpec {
                    layer_id: layer.into(), path: format!("{id}.{layer}.ogg"),
                }).collect(),
            }
        }).collect(),
        layers: ["bed", "tune"].into_iter().enumerate().map(|(slot, id)| {
            MusicLayerSpec { id: id.into(), slot }
        }).collect(),
        states: ["intro", "song", "later"].into_iter().map(|id| MusicStateSpec {
            id: id.into(), section_id: id.into(), next_state: (id == "intro").then(|| "song".into()),
            gains: ["bed", "tune"].into_iter().map(|layer| MusicLayerGainSpec {
                layer_id: layer.into(), gain: 1.0,
            }).collect(),
        }).collect(),
        outro_state: None, post_clear_bridge_state: None, runtime_balance_overrides: Vec::new(),
    };
    let mut loaded = LoadedMusicCueAssets::default();
    let mut handles = Vec::new();
    for section in &cue.sections {
        for source in &section.sources {
            let handle = app.world().resource::<Assets<KiraAudioSource>>().reserve_handle();
            loaded.sources.insert(
                MusicSourceKey::new("game", &cue.id, &section.id, &source.layer_id),
                (cue.source_path(source), handle.clone()),
            );
            handles.push(handle);
        }
    }
    let mut catalogs = AdaptiveMusicCatalogRegistry::default();
    catalogs.register("game", MusicCueCatalog::from_parts(vec![cue], Vec::new())).unwrap();
    let registry = crate::spec::MusicRegistry { default_track: String::new(), tracks: Vec::new() };
    let (library, playback) = AudioLibrary::new_with_playback_state(&registry, None);
    let mut authority = MusicAuthority::governed(Vec::<String>::new());
    authority.authorize_cues(["trail".to_owned()]);
    app.insert_resource(catalogs).insert_resource(loaded).insert_resource(library)
        .insert_resource(playback).insert_resource(MusicIntent {
            provider_id: Some("game".into()),
            adaptive: Some(AdaptiveCueDirective::Play { cue_id: "trail".into(), state_id: "intro".into(), claim: MusicClaim("room".into()) }),
            authority, ..default()
        }).add_systems(Update, drive_music_director);
    (app, handles)
}

fn decode(app: &mut App, handle: &Handle<KiraAudioSource>) {
    app.world_mut().resource_mut::<Assets<KiraAudioSource>>().insert(handle.id(), KiraAudioSource {
        sound: StaticSoundData {
            sample_rate: 48000, frames: vec![Frame::new(0.0, 0.0)].into(),
            settings: StaticSoundSettings::default(), slice: None,
        },
    }).unwrap();
}

fn step(app: &mut App, seconds: f32) {
    app.world_mut().resource_mut::<Time<Real>>().advance_by(Duration::from_secs_f32(seconds));
    app.update();
}

#[test]
fn the_music_watchdog_separates_requests_decoding_backend_gaps_and_explicit_silence() {
    use crate::music::diagnostics::{observe_music_playback, MusicPlaybackDiagnostics, MusicPlaybackStatus};
    let (mut app, handles) = fixture();
    app.insert_resource(crate::output::AudioOutputMode::Device)
        .init_resource::<MusicPlaybackDiagnostics>()
        .init_resource::<crate::selection::ActiveAudioSelection>()
        .insert_resource(crate::web_unlock::AudioUnlockState { unlocked: true, ..default() })
        .add_systems(PostUpdate, observe_music_playback);
    step(&mut app, 0.25);
    assert_eq!(app.world().resource::<MusicPlaybackDiagnostics>().status, Some(MusicPlaybackStatus::Loading));
    for handle in &handles { decode(&mut app, handle); }
    step(&mut app, 0.75);
    let report = app.world().resource::<MusicPlaybackDiagnostics>();
    assert_eq!(report.status, Some(MusicPlaybackStatus::BackendSilent));
    assert_eq!(report.expected, "cue:trail state:intro");
    assert_eq!(report.section.as_deref(), Some("intro"));
    assert_eq!(report.gap_seconds, 1.0);
    app.world_mut().resource_mut::<MusicIntent>().adaptive = Some(AdaptiveCueDirective::Play {
        cue_id: "not_authored".into(), state_id: "intro".into(), claim: MusicClaim("room".into()),
    });
    step(&mut app, 0.25);
    assert_eq!(app.world().resource::<MusicPlaybackDiagnostics>().status, Some(MusicPlaybackStatus::MissingContent));
    app.world_mut().resource_mut::<MusicIntent>().authority = MusicAuthority::governed(Vec::<String>::new());
    step(&mut app, 0.25);
    let report = app.world().resource::<MusicPlaybackDiagnostics>();
    assert_eq!(report.status, Some(MusicPlaybackStatus::DeliberateSilence));
    assert_eq!(report.gap_seconds, 0.0);
    assert_eq!(report.gap_count, 1);
    app.insert_resource(crate::output::AudioPlaybackConditions {
        music_context_required: true, ..default()
    });
    step(&mut app, 0.25);
    let report = app.world().resource::<MusicPlaybackDiagnostics>();
    assert_eq!(report.status, Some(MusicPlaybackStatus::MissingContext));
    assert_eq!(report.gap_count, 2);
}

/// A slow layer must not start the intro clock before the complete cue can play.
#[test]
fn decoding_layers_does_not_consume_the_intro_or_split_their_start() {
    let (mut app, handles) = fixture();
    for handle in &handles[1..] { decode(&mut app, handle); }
    for _ in 0..20 { step(&mut app, 0.25); }
    assert_eq!(app.world().resource::<MusicDirectorState>().mode, MusicDirectorMode::Idle);
    decode(&mut app, &handles[0]);
    step(&mut app, 0.25);
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.mode, MusicDirectorMode::AdaptiveIntro);
    assert_eq!(director.seconds_in_mode, 0.0);
    assert_eq!(app.world().resource::<super::super::playback::MusicPlayback>().reserved_voices(), 2);
    app.world_mut().resource_mut::<MusicIntent>().adaptive = Some(AdaptiveCueDirective::Play {
        cue_id: "trail".into(), state_id: "song".into(), claim: MusicClaim("room".into()),
    });
    for _ in 0..15 { step(&mut app, 0.25); }
    assert_eq!(app.world().resource::<MusicDirectorState>().mode, MusicDirectorMode::AdaptiveIntro);
    step(&mut app, 0.25);
    assert_eq!(app.world().resource::<MusicDirectorState>().mode, MusicDirectorMode::AdaptiveLoop);
    step(&mut app, 0.25);
    assert_eq!(app.world().resource::<MusicDirectorState>().seconds_in_loop, 0.25);
}

#[test]
fn a_held_intro_request_continues_on_real_time_while_the_world_is_paused() {
    let (mut app, handles) = fixture();
    app.init_resource::<Time<Virtual>>();
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    for handle in &handles { decode(&mut app, handle); }
    step(&mut app, 0.25);
    for _ in 0..15 { step(&mut app, 0.25); }
    assert_eq!(app.world().resource::<MusicDirectorState>().mode, MusicDirectorMode::AdaptiveIntro);
    step(&mut app, 0.25);
    for _ in 0..40 { step(&mut app, 0.25); }
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.current_state_id.as_deref(), Some("song"));
    assert_eq!(director.seconds_in_loop, 10.0);
    assert_eq!(app.world().resource::<Time<Virtual>>().elapsed_secs(), 0.0);
    // The encounter catches up after resume; the playing loop keeps its clock.
    app.world_mut().resource_mut::<MusicIntent>().adaptive = Some(AdaptiveCueDirective::Play {
        cue_id: "trail".into(), state_id: "song".into(), claim: MusicClaim("room".into()),
    });
    step(&mut app, 0.25);
    assert_eq!(app.world().resource::<MusicDirectorState>().seconds_in_loop, 10.25);
    // A later entry into the one-shot is a new request and must play it again.
    app.world_mut().resource_mut::<MusicIntent>().adaptive = Some(AdaptiveCueDirective::Play {
        cue_id: "trail".into(), state_id: "intro".into(), claim: MusicClaim("room".into()),
    });
    for _ in 0..20 { step(&mut app, 0.25); }
    assert_eq!(app.world().resource::<MusicDirectorState>().mode, MusicDirectorMode::AdaptiveIntro);
}

#[test]
fn a_slow_later_section_does_not_delay_the_opening_or_start_partial_layers() {
    let (mut app, handles) = fixture();
    // Only the opening is decoded. The held intro request must still start.
    for handle in &handles[..2] { decode(&mut app, handle); }
    step(&mut app, 0.25);
    assert_eq!(app.world().resource::<MusicDirectorState>().mode, MusicDirectorMode::AdaptiveIntro);
    for _ in 0..20 { step(&mut app, 0.25); }
    assert_eq!(app.world().resource::<MusicDirectorState>().current_section_id.as_deref(), Some("intro"));
    decode(&mut app, &handles[2]);
    step(&mut app, 0.25);
    assert_eq!(app.world().resource::<MusicDirectorState>().current_section_id.as_deref(), Some("intro"));
    decode(&mut app, &handles[3]);
    step(&mut app, 0.25);
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.mode, MusicDirectorMode::AdaptiveLoop);
    assert_eq!(director.current_section_id.as_deref(), Some("song"));
    assert_eq!(director.seconds_in_mode, 0.0);
    app.world_mut().resource_mut::<MusicIntent>().adaptive = Some(AdaptiveCueDirective::Play {
        cue_id: "trail".into(), state_id: "later".into(), claim: MusicClaim("room".into()),
    });
    decode(&mut app, &handles[4]);
    for _ in 0..20 { step(&mut app, 0.25); }
    assert_eq!(app.world().resource::<MusicDirectorState>().current_section_id.as_deref(), Some("song"));
    decode(&mut app, &handles[5]);
    for _ in 0..20 { step(&mut app, 0.25); }
    assert_eq!(app.world().resource::<MusicDirectorState>().current_section_id.as_deref(), Some("later"));
}

#[test]
fn recording_consumes_starts_gains_and_cancellation_without_a_device_drain() {
    use crate::music::playback::{MusicPlayback, RecordedMusicOutput};
    let (mut app, handles) = fixture();
    for handle in &handles {
        decode(&mut app, handle);
    }
    app.world_mut().resource_mut::<MusicIntent>().adaptive = Some(AdaptiveCueDirective::Play {
        cue_id: "trail".into(),
        state_id: "song".into(),
        claim: MusicClaim("room".into()),
    });
    for _ in 0..10 {
        step(&mut app, 0.25);
    }
    let recording = app.world().resource::<RecordedMusicOutput>();
    assert_eq!((recording.submitted, recording.resumed), (2, 2));
    let gain_changes = recording.gain_changes;
    for _ in 0..10 {
        step(&mut app, 0.25);
    }
    assert_eq!(
        app.world().resource::<RecordedMusicOutput>().gain_changes,
        gain_changes
    );
    app.world_mut().resource_mut::<MusicIntent>().authority =
        MusicAuthority::governed(Vec::<String>::new());
    for _ in 0..3 {
        step(&mut app, 0.25);
    }
    assert_eq!(app.world().resource::<RecordedMusicOutput>().stopped, 2);
    assert_eq!(app.world().resource::<MusicPlayback>().reserved_voices(), 0);
}
