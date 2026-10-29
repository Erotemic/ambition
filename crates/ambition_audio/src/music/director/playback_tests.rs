use super::*;
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
        .insert_resource(crate::output::AudioOutputMode::Recording)
        .insert_resource(AudioChannel::<MusicChannel>::default());
    crate::music::add_music_layer_channels(&mut app);
    let cue = MusicCueSpec {
        id: "trail".into(), asset_root: "content://library/music".into(),
        bpm: 60.0, beats_per_bar: 4.0, relative_volume: 1.0,
        sections: [("intro", false), ("song", true)].into_iter().map(|(id, looped)| {
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
        states: ["intro", "song"].into_iter().map(|id| MusicStateSpec {
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
            adaptive: Some(AdaptiveCueDirective::Play { cue_id: "trail".into(), state_id: "intro".into() }),
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

/// A slow layer must not start the intro clock before the complete cue can play.
#[test]
fn decoding_layers_does_not_consume_the_intro_or_split_their_start() {
    let (mut app, handles) = fixture();
    for handle in &handles[..handles.len() - 1] { decode(&mut app, handle); }
    for _ in 0..20 { step(&mut app, 0.25); }
    assert_eq!(app.world().resource::<MusicDirectorState>().mode, MusicDirectorMode::Idle);
    decode(&mut app, handles.last().unwrap());
    step(&mut app, 0.25);
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.mode, MusicDirectorMode::AdaptiveIntro);
    assert_eq!(director.seconds_in_mode, 0.0);
    assert_eq!(&director.target_gains[director.active_bank.index()][..2], &[1.0, 1.0]);
    app.world_mut().resource_mut::<MusicIntent>().adaptive = Some(AdaptiveCueDirective::Play {
        cue_id: "trail".into(), state_id: "song".into(),
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
        cue_id: "trail".into(), state_id: "song".into(),
    });
    step(&mut app, 0.25);
    assert_eq!(app.world().resource::<MusicDirectorState>().seconds_in_loop, 10.25);
    // A later entry into the one-shot is a new request and must play it again.
    app.world_mut().resource_mut::<MusicIntent>().adaptive = Some(AdaptiveCueDirective::Play {
        cue_id: "trail".into(), state_id: "intro".into(),
    });
    for _ in 0..20 { step(&mut app, 0.25); }
    assert_eq!(app.world().resource::<MusicDirectorState>().mode, MusicDirectorMode::AdaptiveIntro);
}
