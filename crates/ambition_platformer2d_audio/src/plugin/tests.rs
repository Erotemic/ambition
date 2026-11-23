use super::*;
use ambition_audio::{
    library::{AudioLibrary, MusicPlaybackState},
    music::playback::{MusicPlayback, MusicProduct},
    selection::{ActiveAudioSelection, FrontendAudioProfile},
    spec::{MusicRegistry, MusicTrack},
};
use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn fixture() -> (App, PathBuf, MusicRegistry) {
    let dir = std::env::temp_dir().join(format!(
        "ambition-music-plugin-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&2036_u32.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1000_u32.to_le_bytes());
    wav.extend_from_slice(&2000_u32.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&2000_u32.to_le_bytes());
    wav.resize(2044, 0);
    std::fs::write(dir.join("tone.wav"), wav).unwrap();
    let registry = MusicRegistry {
        default_track: "title".into(),
        tracks: vec![MusicTrack {
            id: "title".into(),
            display_name: "Title".into(),
            asset_path: Some("tone.wav".into()),
            one_shot: false,
        }],
    };
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin {
            file_path: dir.to_string_lossy().into_owned(),
            ..default()
        },
    ))
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(100),
    ))
    .add_plugins((ambition_time::TimePlugin, bevy::input::InputPlugin))
    .init_resource::<ambition_audio::catalog::AudioCatalogRegistry>()
    .init_resource::<ambition_persistence::settings::UserSettings>()
    .insert_resource(ambition_audio::AudioOutputMode::Recording)
    .add_message::<ambition_sfx::OwnedSfxMessage>()
    .add_plugins(Platformer2dAudioPlugin);
    let (mut library, state) = AudioLibrary::new_with_playback_state(&registry, None);
    library
        .resolve_track_handle("title", app.world().resource::<AssetServer>())
        .unwrap();
    app.insert_resource(library).insert_resource(state);
    app.finish();
    app.cleanup();
    (app, dir, registry)
}
fn frames(app: &mut App, count: usize) {
    for _ in 0..count {
        app.update();
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn ready(app: &mut App) {
    for _ in 0..100 {
        frames(app, 1);
        let handle = app
            .world()
            .resource::<AudioLibrary>()
            .resolved_track_handle("title")
            .unwrap();
        if app
            .world()
            .resource::<Assets<ambition_audio::library::KiraAudioSource>>()
            .contains(handle.id())
        {
            return;
        }
    }
    panic!("the WAV did not decode");
}

#[test]
fn frontend_activation_keeps_the_same_title_occurrence_and_silence_retires_it() {
    let (mut app, dir, registry) = fixture();
    let profile = FrontendAudioProfile::new("game").with_title_track("title");
    app.world_mut()
        .resource_mut::<ActiveAudioSelection>()
        .select_frontend(
            1,
            &profile,
            Some(registry.clone()),
            None,
            Default::default(),
        );
    ready(&mut app);
    frames(&mut app, 8);
    let occurrence = app
        .world()
        .resource::<MusicPlayback>()
        .current()
        .unwrap()
        .occurrence;
    assert_eq!(app.world().resource::<MusicPlayback>().reserved_voices(), 1);
    app.world_mut()
        .resource_mut::<ActiveAudioSelection>()
        .select_frontend(
            2,
            &profile,
            Some(registry.clone()),
            None,
            Default::default(),
        );
    app.world_mut()
        .write_message(ambition_audio::selection::AudioContextChanged {
            previous: Some(ambition_sfx::AudioContextOwner::Frontend(1)),
            current: Some(ambition_sfx::AudioContextOwner::Frontend(2)),
        });
    frames(&mut app, 8);
    assert_eq!(
        app.world()
            .resource::<MusicPlayback>()
            .current()
            .unwrap()
            .occurrence,
        occurrence
    );
    let quiet = FrontendAudioProfile::new("game");
    app.world_mut()
        .resource_mut::<ActiveAudioSelection>()
        .select_frontend(3, &quiet, Some(registry), None, Default::default());
    frames(&mut app, 4);
    assert_eq!(app.world().resource::<MusicPlayback>().reserved_voices(), 0);
    assert!(
        app.world()
            .resource::<MusicPlaybackState>()
            .active_track()
            .is_empty()
    );
    drop(app);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn direct_default_uses_one_goal_after_initial_readiness() {
    let (mut app, dir, registry) = fixture();
    app.insert_resource(ActiveAudioSelection::selected_direct(
        "game",
        Some(registry),
        None,
        Default::default(),
    ))
    .insert_resource(InitialGameplayReadiness::closed());
    let room = ambition_platformer2d_world::rooms::RoomSpec::new(
        "room",
        ambition_platformer2d_core::World::new(
            "room",
            Vec2::splat(800.0),
            Vec2::splat(16.0),
            Vec::new(),
        ),
    );
    ambition_platformer2d_world::rooms::insert_room_set(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
            "room",
            vec![room],
            Vec::new(),
        ),
    );
    ready(&mut app);
    frames(&mut app, 4);
    assert_eq!(app.world().resource::<MusicPlayback>().reserved_voices(), 0);
    app.world_mut()
        .resource_mut::<InitialGameplayReadiness>()
        .mark_ready();
    frames(&mut app, 8);
    let playback = app.world().resource::<MusicPlayback>();
    assert_eq!(playback.reserved_voices(), 1);
    assert!(
        matches!(&playback.current().unwrap().product, MusicProduct::Track { id, .. } if id == "title")
    );
    let revision = app
        .world()
        .resource::<ambition_audio::music::MusicIntent>()
        .revision;
    frames(&mut app, 8);
    assert_eq!(
        app.world()
            .resource::<ambition_audio::music::MusicIntent>()
            .revision,
        revision
    );
    drop(app);
    std::fs::remove_dir_all(dir).unwrap();
}
