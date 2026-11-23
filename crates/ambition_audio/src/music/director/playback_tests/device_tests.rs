use super::*;
use std::time::Instant;

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
    assert!(director.position_seconds.is_some_and(|position| position > 0.05));
    assert!(app.world().resource::<MusicPlayback>().reserved_voices() <= 4);
}

#[test]
#[ignore = "uses the native audio device; run with an explicit output or capture device"]
fn native_output_survives_cold_decode_and_repeated_fresh_handoffs() {
    let mut authored = authored_cues(240.0);
    for cue in &mut authored.cues {
        cue.playback.section_crossfade_seconds = Some(0.1);
        cue.playback.continuation_crossfade_seconds = Some(0.05);
    }
    let catalog = MusicCueCatalog::prepare(authored, &Default::default()).unwrap();
    let (mut app, handles) =
        fixture_catalog_output(catalog, crate::output::AudioOutputMode::Device);
    app.finish();
    app.cleanup();
    run_for(&mut app, 1.25);
    assert!(app.world().resource::<MusicDirectorState>().admitted().is_none());
    publish(&mut app, selection("trail", "song", "room", 0, 10));

    for (index, handle) in handles.iter().enumerate() {
        let frequency = 220.0 + index as f64 * 55.0;
        let frames = (0..48000)
            .map(|frame| {
                let value = (std::f64::consts::TAU * frequency * frame as f64 / 48000.0)
                    .sin() as f32 * 0.08;
                Frame::new(value, value)
            })
            .collect::<Vec<_>>();
        app.world_mut().resource_mut::<Assets<KiraAudioSource>>().insert(
            handle.id(),
            KiraAudioSource {
                sound: StaticSoundData {
                    sample_rate: 48000,
                    frames: frames.into(),
                    settings: StaticSoundSettings::default(),
                    slice: None,
                },
            },
        ).unwrap();
    }
    run_for(&mut app, 2.0);
    assert_running(&app, "trail", "song");

    publish(&mut app, selection("destination", "intro", "room", 1, 10));
    run_for(&mut app, 2.0);
    assert_running(&app, "destination", "song");

    publish(&mut app, selection("trail", "intro", "room", 2, 10));
    run_for(&mut app, 2.0);
    assert_running(&app, "trail", "song");

    app.world_mut().resource_mut::<MusicIntent>().publish(MusicGoal::Silence {
        context: Some(ambition_sfx::AudioContextOwner::Direct),
        reason: MusicSilenceReason::NoEligibleChoice,
    });
    run_for(&mut app, 1.0);
    assert!(app.world().resource::<MusicDirectorState>().admitted().is_none());
}
