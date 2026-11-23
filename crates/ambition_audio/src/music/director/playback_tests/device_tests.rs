use super::*;
use std::time::Instant;

fn decode_tone(app: &mut App, handle: &Handle<KiraAudioSource>, index: usize) {
    let frequency = 220.0 + index as f64 * 55.0;
    let frame_count = if index % 8 < 2 { 72000 } else { 48000 };
    let frames = (0..frame_count)
        .map(|frame| {
            let value =
                (std::f64::consts::TAU * frequency * frame as f64 / 48000.0).sin() as f32 * 0.08;
            Frame::new(value, value)
        })
        .collect::<Vec<_>>();
    app.world_mut()
        .resource_mut::<Assets<KiraAudioSource>>()
        .insert(
            handle.id(),
            KiraAudioSource {
                sound: StaticSoundData {
                    sample_rate: 48000,
                    frames: frames.into(),
                    settings: StaticSoundSettings::default(),
                    slice: None,
                },
            },
        )
        .unwrap();
}

fn run_for(app: &mut App, seconds: f64) {
    let start = Instant::now();
    let mut previous = start;
    while start.elapsed().as_secs_f64() < seconds {
        let now = Instant::now();
        step(app, now.duration_since(previous).as_secs_f64());
        previous = now;
        std::thread::sleep(Duration::from_millis(8));
    }
    let director = app.world().resource::<MusicDirectorState>();
    eprintln!(
        "native music: cue={:?} section={:?} cursor={:?} mode={:?}",
        director.active_cue_id,
        director.current_section_id,
        director.position_seconds,
        director.mode,
    );
}

fn assert_running(app: &App, cue: &str, section: &str) {
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.active_cue_id.as_deref(), Some(cue));
    assert_eq!(director.current_section_id.as_deref(), Some(section));
    assert!(director.admitted().is_some());
    assert!(director
        .position_seconds
        .is_some_and(|position| position > 0.05));
    assert!(app.world().resource::<MusicPlayback>().reserved_voices() <= 4);
}

fn native_fixture() -> (App, Vec<Handle<KiraAudioSource>>) {
    let mut authored = authored_cues(240.0);
    for cue in &mut authored.cues {
        cue.playback.section_crossfade_seconds = Some(0.1);
        cue.playback.continuation_crossfade_seconds = Some(0.0);
    }
    let catalog = MusicCueCatalog::prepare(authored, &Default::default()).unwrap();
    let (mut app, handles) =
        fixture_catalog_output(catalog, crate::output::AudioOutputMode::Device);
    app.finish();
    app.cleanup();
    (app, handles)
}

#[test]
#[ignore = "uses the native audio device; run with an explicit output or capture device"]
fn native_output_survives_cold_decode_and_repeated_fresh_handoffs() {
    let (mut app, handles) = native_fixture();
    run_for(&mut app, 1.25);
    assert!(app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .is_none());
    publish(&mut app, selection("trail", "song", "room", 0, 10));

    for (index, handle) in handles.iter().enumerate() {
        decode_tone(&mut app, handle, index);
    }
    app.update();
    let submitted_sound = app
        .world()
        .resource::<Assets<KiraAudioSource>>()
        .get(&handles[2])
        .unwrap()
        .sound
        .clone();
    app.world_mut()
        .resource_mut::<Assets<KiraAudioSource>>()
        .get_mut(&handles[2])
        .unwrap()
        .sound
        .frames = vec![Frame::new(0.0, 0.0); 96000].into();
    run_for(&mut app, 2.0);
    assert_running(&app, "trail", "song");
    app.world_mut()
        .resource_mut::<Assets<KiraAudioSource>>()
        .get_mut(&handles[2])
        .unwrap()
        .sound = submitted_sound;

    publish(&mut app, selection("destination", "intro", "room", 1, 10));
    run_for(&mut app, 0.5);
    // Native playback must cross the file end while the host does no work.
    std::thread::sleep(Duration::from_secs(1));
    run_for(&mut app, 0.5);
    assert_running(&app, "destination", "song");

    publish(&mut app, selection("trail", "intro", "room", 2, 10));
    run_for(&mut app, 2.0);
    assert_running(&app, "trail", "song");

    app.world_mut()
        .resource_mut::<MusicIntent>()
        .publish(MusicGoal::Silence {
            context: Some(ambition_sfx::AudioContextOwner::Direct),
            reason: MusicSilenceReason::NoEligibleChoice,
        });
    run_for(&mut app, 1.0);
    assert!(app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .is_none());
    assert_eq!(app.world().resource::<MusicPlayback>().reserved_voices(), 0);
    assert!(app.world().resource::<Assets<AudioInstance>>().is_empty());
    assert_eq!(
        app.world_mut()
            .non_send_mut::<bevy_kira_audio::AudioOutput>()
            .manager()
            .unwrap()
            .num_sub_tracks(),
        0
    );
}

#[test]
#[ignore = "uses the native audio device; run with an explicit output or capture device"]
fn native_canceling_a_prepared_entry_cancels_its_predecessors_stop() {
    let (mut app, handles) = native_fixture();
    for (index, handle) in handles
        .iter()
        .enumerate()
        .filter(|(index, _)| index % 8 < 4)
    {
        decode_tone(&mut app, handle, index);
    }
    run_for(&mut app, 0.5);
    assert!(app
        .world()
        .resource::<MusicPlayback>()
        .groups()
        .any(|group| group.scheduled_state() == Some("song")));

    publish(&mut app, selection("trail", "later", "room", 0, 10));
    run_for(&mut app, 0.001);
    std::thread::sleep(Duration::from_secs_f64(0.8));
    run_for(&mut app, 0.01);
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.current_section_id.as_deref(), Some("intro"));
    assert!(director
        .position_seconds
        .is_some_and(|position| position > 1.05));
    for (index, handle) in handles.iter().enumerate().take(6).skip(4) {
        decode_tone(&mut app, handle, index);
    }
    run_for(&mut app, 1.0);
    assert_running(&app, "trail", "later");
}

#[test]
#[ignore = "uses the native audio device; run with an explicit output or capture device"]
fn a_native_short_continuation_finishes_once_between_host_observations() {
    let mut authored = authored_cues(240.0);
    for cue in &mut authored.cues {
        cue.playback.continuation_crossfade_seconds = Some(0.0);
        if let MusicCueMedia::Inline { sections, .. } = &mut cue.media {
            sections[0].duration_beats = 2.0;
            sections[1].duration_beats = 0.02;
            sections[1].looped = false;
        }
    }
    let catalog = MusicCueCatalog::prepare(authored, &Default::default()).unwrap();
    let (mut app, handles) =
        fixture_catalog_output(catalog, crate::output::AudioOutputMode::Device);
    app.finish();
    app.cleanup();
    for (index, handle) in handles.iter().enumerate().take(4) {
        decode_tone(&mut app, handle, index);
        let count = if index < 2 { 24000 } else { 240 };
        let mut sources = app.world_mut().resource_mut::<Assets<KiraAudioSource>>();
        let mut source = sources.get_mut(handle).unwrap();
        source.sound.frames = source.sound.frames[..count].to_vec().into();
    }
    let mut prepared = false;
    for _ in 0..60 {
        run_for(&mut app, 0.001);
        prepared = app
            .world()
            .resource::<MusicPlayback>()
            .groups()
            .any(|group| {
                group.scheduled_state() == Some("song") && group.transport_origin.is_some()
            });
        if prepared {
            break;
        }
    }
    assert!(
        prepared,
        "the short continuation was not admitted before its deadline"
    );
    std::thread::sleep(Duration::from_secs_f64(0.8));
    run_for(&mut app, 0.05);
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 4);
    assert_eq!(
        app.world().resource::<MusicPlayback>().outcome(),
        super::super::super::playback::MusicRequestOutcome::Completed
    );
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.current_state_id.as_deref(), Some("song"));
    assert!(director.transport_completed);
    run_for(&mut app, 0.2);
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 4);
}
