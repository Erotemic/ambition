use super::*;
use crate::cue_spec::{MusicLayerGainSpec, MusicLayerSourceSpec, MusicLayerSpec, MusicSectionSpec};
use crate::music::catalog::{LoadedMusicSource, MusicSourceKey};
use crate::music::playback::{MusicPlayback, RecordedMusicOutput};
use crate::selection::MusicAuthority;
use bevy::asset::AssetApp;
use bevy_kira_audio::prelude::{Frame, StaticSoundData, StaticSoundSettings};
use std::time::Duration;

fn authored_cues(bpm: f32) -> crate::cue_spec::AuthoredMusicCues {
    let cue = MusicCueSpec {
        id: "trail".into(),
        media: MusicCueMedia::Inline {
            asset_root: "content://library/music".into(),
            bpm,
            beats_per_bar: 4.0,
            sections: [
                ("intro", false),
                ("song", true),
                ("later", true),
                ("outro", false),
            ]
            .into_iter()
            .map(|(id, looped)| MusicSectionSpec {
                id: id.into(),
                duration_beats: 4.0,
                looped,
                sources: ["bed", "tune"]
                    .into_iter()
                    .map(|layer| MusicLayerSourceSpec {
                        layer_id: layer.into(),
                        path: format!("{id}.{layer}.ogg"),
                    })
                    .collect(),
            })
            .collect(),
        },
        relative_volume: 1.0,
        layers: ["bed", "tune"]
            .into_iter()
            .enumerate()
            .map(|(slot, id)| MusicLayerSpec {
                id: id.into(),
                slot,
            })
            .collect(),
        states: ["intro", "song", "later", "outro"]
            .into_iter()
            .map(|id| MusicStateSpec {
                id: id.into(),
                section_id: id.into(),
                next_state: (id == "intro").then(|| "song".into()),
                gains: ["bed", "tune"]
                    .into_iter()
                    .map(|layer| MusicLayerGainSpec {
                        layer_id: layer.into(),
                        gain: 1.0,
                    })
                    .collect(),
            })
            .collect(),
        outro_state: Some("outro".into()),
        post_clear_bridge_state: None,
        runtime_balance_overrides: Vec::new(),
    };
    let mut destination = cue.clone();
    destination.id = "destination".into();
    if let MusicCueMedia::Inline { asset_root, .. } = &mut destination.media {
        *asset_root = "content://library/destination".into();
    }
    crate::cue_spec::AuthoredMusicCues {
        cues: vec![cue, destination],
        ..default()
    }
}

fn fixture() -> (App, Vec<Handle<KiraAudioSource>>) {
    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        bevy::asset::AssetPlugin::default(),
    ))
    .init_asset::<KiraAudioSource>()
    .init_resource::<DynamicAudioChannels>()
    .init_resource::<Time<Real>>()
    .init_resource::<MusicMix>()
    .init_resource::<MusicDirectorState>()
    .insert_resource(crate::output::AudioOutputMode::Recording);
    crate::music::add_music_playback(&mut app);
    let authored = authored_cues(60.0);
    let catalog =
        MusicCueCatalog::prepare(authored, &crate::music_media::MusicMediaCatalog::default())
            .unwrap();
    let mut loaded = LoadedMusicCueAssets::default();
    let mut handles = Vec::new();
    for id in ["trail", "destination"] {
        let cue = catalog.cue(id).unwrap();
        for section in &cue.sections {
            for source in &section.sources {
                let handle = app
                    .world()
                    .resource::<AssetServer>()
                    .load(source.path.clone());
                loaded.sources.insert(
                    MusicSourceKey::new("game", 1, &cue.id, &section.id, &source.layer_id),
                    LoadedMusicSource::new(source.path.clone(), handle.clone()),
                );
                handles.push(handle);
            }
        }
    }
    let mut catalogs = AdaptiveMusicCatalogRegistry::default();
    catalogs.register("game", catalog).unwrap();
    let registry = crate::spec::MusicRegistry {
        default_track: String::new(),
        tracks: Vec::new(),
    };
    let (library, playback) = AudioLibrary::new_with_playback_state(&registry, None);
    let mut authority = MusicAuthority::governed(Vec::<String>::new());
    authority.authorize_cues(["trail".to_owned(), "destination".to_owned()]);
    let mut intent = MusicIntent {
        context: Some(ambition_sfx::AudioContextOwner::Direct),
        provider_id: Some("game".into()),
        authority,
        ..default()
    };
    intent.publish(MusicGoal::Play(selection("trail", "intro", "room", 0, 10)));
    app.insert_resource(catalogs)
        .insert_resource(loaded)
        .insert_resource(library)
        .insert_resource(playback)
        .insert_resource(intent)
        .add_systems(Update, drive_music_director);
    (app, handles)
}

fn selection(cue: &str, state: &str, source: &str, attempt: u64, priority: u16) -> MusicSelection {
    let claim = MusicClaim {
        context: ambition_sfx::AudioContextOwner::Direct,
        session: Some(0),
        scope: Some(0),
        source: source.into(),
        attempt,
    };
    MusicSelection {
        provider: "game".into(),
        generation: 1,
        claim: claim.clone(),
        release_owner: (attempt > 0).then_some(claim),
        choice: MusicChoice::Cue {
            id: cue.into(),
            state: state.into(),
            entry: None,
        },
        continuity: MusicContinuity::SharedProgram,
        entry_policy: if attempt > 0 {
            MusicEntryPolicy::FreshOccurrence
        } else {
            MusicEntryPolicy::ContinueCompatible
        },
        phase_policy: MusicPhasePolicy::default(),
        priority,
        crossfade_s: 0.22,
        completed_state: (attempt > 0).then(|| "outro".into()),
    }
}

fn decode(app: &mut App, handle: &Handle<KiraAudioSource>) {
    app.world_mut()
        .resource_mut::<Assets<KiraAudioSource>>()
        .insert(
            handle.id(),
            KiraAudioSource {
                sound: StaticSoundData {
                    sample_rate: 48000,
                    frames: vec![Frame::new(0.0, 0.0); 4 * 48000].into(),
                    settings: StaticSoundSettings::default(),
                    slice: None,
                },
            },
        )
        .unwrap();
}
fn step(app: &mut App, seconds: f64) {
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs_f64(seconds));
    app.update();
}
fn frames(app: &mut App, count: usize) {
    for _ in 0..count {
        step(app, 0.25);
    }
}
fn publish(app: &mut App, selection: MusicSelection) {
    app.world_mut()
        .resource_mut::<MusicIntent>()
        .publish(MusicGoal::Play(selection));
}
#[derive(Resource)]
struct PreparedFixtureSources {
    _handles: Vec<Handle<KiraAudioSource>>,
}

fn decoded() -> App {
    let (mut app, handles) = fixture();
    for handle in &handles {
        decode(&mut app, handle);
    }
    app.insert_resource(PreparedFixtureSources { _handles: handles });
    app
}

#[test]
fn decoding_and_admission_do_not_consume_an_entrance() {
    let (mut app, handles) = fixture();
    for handle in &handles[1..] {
        decode(&mut app, handle);
    }
    frames(&mut app, 24);
    assert_eq!(
        app.world().resource::<MusicDirectorState>().mode,
        MusicDirectorMode::Idle
    );
    decode(&mut app, &handles[0]);
    app.insert_resource(crate::output::AudioOutputMode::Device);
    frames(&mut app, 4);
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.mode, MusicDirectorMode::AdaptiveIntro);
    assert_eq!(director.position_seconds, None);
    assert!(director.admitted().is_none());
    assert_eq!(
        app.world()
            .resource::<super::super::playback::MusicPlayback>()
            .reserved_voices(),
        2
    );
}

#[test]
fn a_held_entrance_continues_once_while_the_simulation_is_paused() {
    let mut app = decoded();
    app.init_resource::<Time<Virtual>>();
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    frames(&mut app, 64);
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.current_state_id.as_deref(), Some("song"));
    assert_eq!(app.world().resource::<Time<Virtual>>().elapsed_secs(), 0.0);
    assert_eq!(
        app.world()
            .resource::<super::super::playback::RecordedMusicOutput>()
            .submitted,
        4
    );
}

#[test]
fn a_new_attempt_with_identical_sources_starts_a_fresh_entry() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "fight", 1, 200));
    frames(&mut app, 6);
    let old = app
        .world()
        .resource::<super::super::playback::MusicPlayback>()
        .current()
        .unwrap()
        .clone();
    publish(&mut app, selection("trail", "song", "fight", 2, 200));
    frames(&mut app, 6);
    let playback = app
        .world()
        .resource::<super::super::playback::MusicPlayback>();
    let new = playback.current().unwrap().clone();
    assert_ne!(old.entry, new.entry);
    assert_ne!(old.occurrence, new.occurrence);
    assert_eq!(old.product, new.product);
    assert_eq!(playback.reserved_voices(), 2);
    frames(&mut app, 12);
    assert_eq!(
        app.world()
            .resource::<super::super::playback::MusicPlayback>()
            .current()
            .unwrap()
            .entry,
        new.entry
    );
}

#[test]
fn a_fresh_fight_does_not_inherit_a_rooms_identical_media() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "room", 0, 10));
    frames(&mut app, 6);
    let old = app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .unwrap()
        .entry;
    publish(&mut app, selection("trail", "song", "fight", 1, 200));
    frames(&mut app, 6);
    assert_ne!(
        app.world()
            .resource::<MusicDirectorState>()
            .admitted()
            .unwrap()
            .entry,
        old
    );
}

#[test]
fn compatible_owner_transfer_keeps_the_playback_occurrence() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "room_a", 0, 10));
    frames(&mut app, 6);
    let old = app
        .world()
        .resource::<super::super::playback::MusicPlayback>()
        .current()
        .unwrap()
        .occurrence;
    publish(&mut app, selection("trail", "song", "room_b", 0, 10));
    frames(&mut app, 6);
    assert_eq!(
        app.world()
            .resource::<super::super::playback::MusicPlayback>()
            .current()
            .unwrap()
            .occurrence,
        old
    );
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .admitted()
            .unwrap()
            .selection
            .claim
            .source,
        "room_b"
    );
}

#[test]
fn an_ending_uses_the_latest_successor_and_cannot_clear_a_new_attempt() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "fight", 1, 200));
    frames(&mut app, 6);
    let old = app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .unwrap()
        .entry;
    publish(&mut app, selection("destination", "song", "room_a", 0, 10));
    app.world_mut().resource_mut::<MusicIntent>().outgoing = Some(MusicDisposition {
        claim: selection("trail", "song", "fight", 1, 200).claim,
        entry: old,
        reason: MusicReleaseReason::Completed,
    });
    step(&mut app, 0.25);
    app.world_mut().resource_mut::<MusicIntent>().outgoing = None;
    assert!(app.world().resource::<MusicDirectorState>().has_ending());
    publish(&mut app, selection("destination", "later", "room_b", 0, 10));
    frames(&mut app, 32);
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.active_cue_id.as_deref(), Some("destination"));
    assert_eq!(director.current_state_id.as_deref(), Some("later"));
    publish(&mut app, selection("trail", "song", "fight", 2, 200));
    app.world_mut().resource_mut::<MusicIntent>().outgoing = Some(MusicDisposition {
        claim: selection("trail", "song", "fight", 1, 200).claim,
        entry: old,
        reason: MusicReleaseReason::Completed,
    });
    frames(&mut app, 24);
    let director = app.world().resource::<MusicDirectorState>();
    assert!(!director.has_ending());
    assert_eq!(director.admitted().unwrap().selection.claim.attempt, 2);
}

#[test]
fn a_speculative_withdrawal_waits_for_settlement_but_radio_can_interrupt() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "fight", 1, 200));
    frames(&mut app, 6);
    let old = app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .unwrap()
        .entry;
    publish(&mut app, selection("destination", "song", "room", 0, 10));
    app.world_mut().resource_mut::<MusicIntent>().outgoing = Some(MusicDisposition {
        claim: selection("trail", "song", "fight", 1, 200).claim,
        entry: old,
        reason: MusicReleaseReason::AwaitingSettlement,
    });
    frames(&mut app, 8);
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .active_cue_id
            .as_deref(),
        Some("trail")
    );
    publish(&mut app, selection("destination", "song", "radio", 0, 400));
    frames(&mut app, 8);
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .active_cue_id
            .as_deref(),
        Some("destination")
    );
    assert!(!app.world().resource::<MusicDirectorState>().has_ending());
}

#[test]
fn held_targets_emit_no_repeated_plays_or_gain_changes() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "room", 0, 10));
    frames(&mut app, 8);
    let before = app
        .world()
        .resource::<super::super::playback::RecordedMusicOutput>();
    let counts = (before.submitted, before.gain_changes);
    frames(&mut app, 20);
    let after = app
        .world()
        .resource::<super::super::playback::RecordedMusicOutput>();
    assert_eq!((after.submitted, after.gain_changes), counts);
}

#[test]
fn admission_is_published_before_the_next_goal_reduction() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "fight", 1, 200));
    frames(&mut app, 2);
    let playback = app
        .world()
        .resource::<super::super::playback::MusicPlayback>();
    let entry = playback.current().unwrap().entry;
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .admitted()
            .unwrap()
            .entry,
        entry
    );
    publish(&mut app, selection("destination", "song", "room", 0, 10));
    app.world_mut().resource_mut::<MusicIntent>().outgoing = Some(MusicDisposition {
        claim: selection("trail", "song", "fight", 1, 200).claim,
        entry,
        reason: MusicReleaseReason::Completed,
    });
    step(&mut app, 0.25);
    assert!(app.world().resource::<MusicDirectorState>().has_ending());
}

#[test]
fn an_already_finished_entry_cannot_create_an_ending() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "outro", "fight", 1, 200));
    frames(&mut app, 3);
    let entry = app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .unwrap()
        .entry;
    frames(&mut app, 24);
    assert!(
        app.world()
            .resource::<MusicDirectorState>()
            .admitted()
            .is_none()
    );
    assert_eq!(
        app.world()
            .resource::<super::super::playback::MusicPlayback>()
            .outcome(),
        super::super::playback::MusicRequestOutcome::Completed
    );
    publish(&mut app, selection("destination", "song", "room", 0, 10));
    app.world_mut().resource_mut::<MusicIntent>().outgoing = Some(MusicDisposition {
        claim: selection("trail", "song", "fight", 1, 200).claim,
        entry,
        reason: MusicReleaseReason::Completed,
    });
    frames(&mut app, 6);
    let director = app.world().resource::<MusicDirectorState>();
    assert!(!director.has_ending());
    assert_eq!(director.active_cue_id.as_deref(), Some("destination"));
}

#[test]
fn an_ending_keeps_the_generation_that_its_voices_admitted() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "fight", 1, 200));
    frames(&mut app, 6);
    let entry = app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .unwrap()
        .entry;
    let registry = app.world().resource::<AdaptiveMusicCatalogRegistry>();
    let next = MusicCueCatalog::prepare(
        authored_cues(300.0),
        &crate::music_media::MusicMediaCatalog::default(),
    )
    .unwrap();
    let registry = registry.with_replaced("game", Some(next)).unwrap();
    let generation = registry.generation("game");
    app.insert_resource(registry);
    let mut successor = selection("destination", "song", "room", 0, 10);
    successor.generation = generation;
    publish(&mut app, successor);
    app.world_mut().resource_mut::<MusicIntent>().outgoing = Some(MusicDisposition {
        claim: selection("trail", "song", "fight", 1, 200).claim,
        entry,
        reason: MusicReleaseReason::Completed,
    });
    for _ in 0..20 {
        step(&mut app, 0.25);
        if app
            .world()
            .resource::<MusicDirectorState>()
            .ending
            .as_ref()
            .is_some_and(|ending| ending.started)
        {
            break;
        }
    }
    frames(&mut app, 4);
    let director = app.world().resource::<MusicDirectorState>();
    assert!(director.has_ending());
    assert_eq!(director.ending.as_ref().unwrap().duration, 4.0);
    assert_eq!(
        director
            .ending
            .as_ref()
            .unwrap()
            .admitted
            .selection
            .generation,
        1
    );
}

#[test]
fn a_superseded_unadmitted_entrance_cannot_start_while_its_replacement_loads() {
    let (mut app, handles) = fixture();
    for handle in &handles[..2] {
        decode(&mut app, handle);
    }
    app.insert_resource(crate::output::AudioOutputMode::Device);
    step(&mut app, 0.25);
    assert_eq!(
        app.world()
            .resource::<super::super::playback::MusicPlayback>()
            .reserved_voices(),
        2
    );
    publish(&mut app, selection("trail", "later", "room", 0, 10));
    step(&mut app, 0.25);
    app.insert_resource(crate::output::AudioOutputMode::Recording);
    frames(&mut app, 4);
    assert!(
        app.world()
            .resource::<MusicDirectorState>()
            .admitted()
            .is_none()
    );
    for handle in &handles[4..6] {
        decode(&mut app, handle);
    }
    frames(&mut app, 6);
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.current_state_id.as_deref(), Some("later"));
    assert!(director.admitted().is_some());
}

#[test]
fn an_ending_with_missing_media_yields_to_the_latest_goal() {
    let (mut app, handles) = fixture();
    for (index, handle) in handles.iter().enumerate() {
        if index != 6 && index != 7 {
            decode(&mut app, handle);
        }
    }
    let fight = selection("trail", "song", "fight", 1, 200);
    publish(&mut app, fight.clone());
    frames(&mut app, 6);
    let entry = app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .unwrap()
        .entry;
    publish(&mut app, selection("destination", "song", "room", 0, 10));
    app.world_mut().resource_mut::<MusicIntent>().outgoing = Some(MusicDisposition {
        claim: fight.claim,
        entry,
        reason: MusicReleaseReason::Completed,
    });
    step(&mut app, 0.25);
    app.world_mut().resource_mut::<MusicIntent>().outgoing = None;
    frames(&mut app, 30);
    let director = app.world().resource::<MusicDirectorState>();
    assert!(!director.has_ending());
    assert_eq!(
        director.last_ending_result,
        Some((entry, MusicEndingResult::MediaUnavailable))
    );
    assert_eq!(director.active_cue_id.as_deref(), Some("destination"));
}

#[test]
fn restarting_during_an_admitted_ending_replaces_only_the_old_attempt() {
    let mut app = decoded();
    let fight = selection("trail", "song", "fight", 1, 200);
    publish(&mut app, fight.clone());
    frames(&mut app, 6);
    let entry = app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .unwrap()
        .entry;
    publish(&mut app, selection("destination", "song", "room", 0, 10));
    app.world_mut().resource_mut::<MusicIntent>().outgoing = Some(MusicDisposition {
        claim: fight.claim,
        entry,
        reason: MusicReleaseReason::Completed,
    });
    step(&mut app, 0.25);
    app.world_mut().resource_mut::<MusicIntent>().outgoing = None;
    for _ in 0..20 {
        if app
            .world()
            .resource::<MusicDirectorState>()
            .ending
            .as_ref()
            .is_some_and(|ending| ending.started)
        {
            break;
        }
        step(&mut app, 0.25);
    }
    assert!(
        app.world()
            .resource::<MusicDirectorState>()
            .ending
            .as_ref()
            .unwrap()
            .started
    );
    publish(&mut app, selection("trail", "song", "fight", 2, 200));
    frames(&mut app, 24);
    let director = app.world().resource::<MusicDirectorState>();
    assert!(!director.has_ending());
    assert_eq!(
        director.last_ending_result,
        Some((entry, MusicEndingResult::Interrupted))
    );
    assert_ne!(director.admitted().unwrap().entry, entry);
    assert_eq!(director.admitted().unwrap().selection.claim.attempt, 2);
}

#[test]
fn an_old_disposition_cannot_end_a_compatible_new_owner() {
    let mut app = decoded();
    let fight = selection("trail", "song", "fight", 1, 200);
    publish(&mut app, fight.clone());
    frames(&mut app, 6);
    let entry = app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .unwrap()
        .entry;
    publish(&mut app, selection("trail", "song", "room_a", 0, 10));
    frames(&mut app, 2);
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .admitted()
            .unwrap()
            .entry,
        entry
    );
    publish(&mut app, selection("destination", "song", "room_b", 0, 10));
    app.world_mut().resource_mut::<MusicIntent>().outgoing = Some(MusicDisposition {
        claim: fight.claim,
        entry,
        reason: MusicReleaseReason::Completed,
    });
    frames(&mut app, 6);
    let director = app.world().resource::<MusicDirectorState>();
    assert!(!director.has_ending());
    assert_eq!(director.active_cue_id.as_deref(), Some("destination"));
}

#[test]
fn cue_only_playback_needs_no_simple_track_library() {
    let mut app = decoded();
    app.world_mut().remove_resource::<AudioLibrary>();
    app.world_mut().remove_resource::<MusicPlaybackState>();
    frames(&mut app, 6);
    assert!(
        app.world()
            .resource::<MusicDirectorState>()
            .admitted()
            .is_some()
    );
}

#[test]
fn a_published_variable_clock_controls_the_running_sections_handoff() {
    use super::super::playback::{MusicPlayback, MusicProduct, RecordedMusicOutput};
    use crate::cue_spec::{MusicCueMedia, MusicProductSet};
    use crate::music_media::{
        MusicMediaCatalog,
        tests::{Publication, authored, description},
    };
    use serde_json::json;
    let mut data = description();
    let body = &mut data["sections"][0];
    body["loop"] = true.into();
    body["tail"] = serde_json::Value::Null;
    body["tail_end_seconds"] = serde_json::Value::Null;
    body["tail_audio_end_seconds"] = serde_json::Value::Null;
    body["end_seconds"] = 6.0.into();
    body["musical_end_seconds"] = 6.0.into();
    body["file_duration_seconds"] = 6.0.into();
    body["clock"] = json!({"kind":"boundaries","bar_seconds":[0.0,2.0,6.0]});
    body["markers"][1]["seconds"] = 6.0.into();
    body["material"]["tail"] = "none".into();
    body["frame_plan"]["musical_end_frame"] = 48000.into();
    body["frame_plan"]["file_frames"] = 48000.into();
    body["frame_plan"]["loop_frames"] = 48000.into();
    body["frame_plan"]["tail_source_frames"] = 0.into();
    body["frame_plan"]["tail_source_audio_frames"] = 0.into();
    body["frame_plan"]["bar_frames"] = json!([0, 16000, 48000]);
    body["frame_plan"]["markers"][1]["frame"] = 48000.into();
    body["products"][0]["frames"] = 48000.into();
    let mut later = body.clone();
    later["id"] = "later".into();
    later["form"] = json!(["later"]);
    later["end_seconds"] = 4.0.into();
    later["musical_end_seconds"] = 4.0.into();
    later["file_duration_seconds"] = 4.0.into();
    later["clock"] =
        json!({"kind":"constant","bpm":120.0,"beats_per_bar":4.0,"bar_origin_seconds":0.0});
    later["markers"][1]["seconds"] = 4.0.into();
    later["frame_plan"]["markers"][1]["frame"] = 32000.into();
    later["frame_plan"]["musical_end_frame"] = 32000.into();
    later["frame_plan"]["file_frames"] = 32000.into();
    later["frame_plan"]["loop_frames"] = 32000.into();
    later["frame_plan"]["bar_frames"] = json!([0, 16000, 32000]);
    later["products"][0]["frames"] = 32000.into();
    later["products"][0]["path"] = "playback/later/later.full.ogg".into();
    data["sections"].as_array_mut().unwrap().push(later);
    let publication = Publication::new();
    let snapshot = publication.select("variable", data);
    let reference = "content://media/music/cue/report.json";
    let media = MusicMediaCatalog::from_snapshot(&snapshot, [reference]).unwrap();
    let mut policy = authored(MusicProductSet::MasteredFull, &["full"]);
    policy.cues[0].id = "trail".into();
    policy.cues[0].media = MusicCueMedia::Published {
        description: reference.into(),
        products: MusicProductSet::MasteredFull,
    };
    let mut later_state = policy.cues[0].states[0].clone();
    later_state.id = "later".into();
    later_state.section_id = "later".into();
    policy.cues[0].states.push(later_state);
    let catalog = MusicCueCatalog::prepare(policy, &media).unwrap();
    let (mut app, _) = fixture();
    let mut assets = LoadedMusicCueAssets::default();
    let mut prepared_sources = Vec::new();
    for cue in catalog.cues() {
        for section in &cue.sections {
            for source in &section.sources {
                let product = source.product.as_ref().unwrap();
                let handle = app
                    .world()
                    .resource::<AssetServer>()
                    .load(source.path.clone());
                app.world_mut()
                    .resource_mut::<Assets<KiraAudioSource>>()
                    .insert(
                        handle.id(),
                        KiraAudioSource {
                            sound: StaticSoundData {
                                sample_rate: product.sample_rate,
                                frames: vec![Frame::new(0.0, 0.0); product.frames as usize].into(),
                                settings: StaticSoundSettings::default(),
                                slice: None,
                            },
                        },
                    )
                    .unwrap();
                prepared_sources.push(handle.clone());
                assets.sources.insert(
                    MusicSourceKey::new("game", 1, &cue.id, &section.id, &source.layer_id),
                    LoadedMusicSource::new(source.path.clone(), handle),
                );
            }
        }
    }
    let mut catalogs = AdaptiveMusicCatalogRegistry::default();
    catalogs.register("game", catalog).unwrap();
    app.insert_resource(catalogs)
        .insert_resource(assets)
        .insert_resource(PreparedFixtureSources {
            _handles: prepared_sources,
        });
    publish(&mut app, selection("trail", "song", "room", 0, 10));
    frames(&mut app, 6);
    let position = app
        .world()
        .resource::<MusicDirectorState>()
        .position_seconds
        .unwrap();
    step(&mut app, 3.0 - position);
    publish(&mut app, selection("trail", "later", "room", 0, 10));
    step(&mut app, 0.0);
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 1);
    step(&mut app, 2.9);
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 1);
    step(&mut app, 0.2);
    frames(&mut app, 6);
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 2);
    assert!(
        matches!(&app.world().resource::<MusicPlayback>().current().unwrap().product, MusicProduct::Section { section, .. } if section == "later")
    );
}

#[test]
fn a_host_hitch_cannot_advance_an_intro_or_a_queued_loop_exit() {
    use bevy::ecs::system::RunSystemOnce;
    for initial in ["intro", "song"] {
        let mut app = decoded();
        publish(&mut app, selection("trail", initial, "room", 0, 10));
        frames(&mut app, 6);
        let cursor = app
            .world()
            .resource::<MusicPlayback>()
            .current()
            .unwrap()
            .position_seconds
            .unwrap();
        publish(&mut app, selection("trail", "later", "room", 0, 10));
        app.world_mut()
            .run_system_once(drive_music_director)
            .unwrap();
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs(30));
        for _ in 0..3 {
            app.world_mut()
                .run_system_once(drive_music_director)
                .unwrap();
        }
        let director = app.world().resource::<MusicDirectorState>();
        assert_eq!(director.current_section_id.as_deref(), Some(initial));
        assert_eq!(director.position_seconds, Some(cursor));
        assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 2);
    }
}

#[test]
fn an_undecoded_target_misses_its_exit_and_waits_for_the_next_one() {
    let (mut app, handles) = fixture();
    for (index, handle) in handles.iter().enumerate() {
        if !(4..6).contains(&index) {
            decode(&mut app, handle);
        }
    }
    publish(&mut app, selection("trail", "song", "room", 0, 10));
    frames(&mut app, 7);
    publish(&mut app, selection("trail", "later", "room", 0, 10));
    step(&mut app, 0.0);
    let position = app
        .world()
        .resource::<MusicPlayback>()
        .current()
        .unwrap()
        .position_seconds
        .unwrap();
    step(&mut app, 4.5 - position);
    step(&mut app, 0.0);
    for handle in &handles[4..6] {
        decode(&mut app, handle);
    }
    step(&mut app, 0.0);
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .current_section_id
            .as_deref(),
        Some("song")
    );
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 2);
    step(&mut app, 3.5);
    frames(&mut app, 4);
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .current_section_id
            .as_deref(),
        Some("later")
    );
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 4);
}

#[test]
fn an_observed_loop_boundary_has_no_extra_minimum_delay() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "room", 0, 10));
    frames(&mut app, 7);
    let position = app
        .world()
        .resource::<MusicPlayback>()
        .current()
        .unwrap()
        .position_seconds
        .unwrap();
    step(&mut app, 4.0 - position);
    step(&mut app, 0.0);
    publish(&mut app, selection("trail", "later", "room", 0, 10));
    step(&mut app, 0.0);
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 4);
    frames(&mut app, 3);
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .current_section_id
            .as_deref(),
        Some("later")
    );
}

#[test]
fn a_host_hitch_cannot_consume_an_admitted_ending() {
    use bevy::ecs::system::RunSystemOnce;
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "fight", 1, 200));
    frames(&mut app, 6);
    let entry = app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .unwrap()
        .entry;
    publish(&mut app, selection("destination", "song", "room", 0, 10));
    app.world_mut().resource_mut::<MusicIntent>().outgoing = Some(MusicDisposition {
        claim: selection("trail", "song", "fight", 1, 200).claim,
        entry,
        reason: MusicReleaseReason::Completed,
    });
    for _ in 0..24 {
        step(&mut app, 0.25);
        if app
            .world()
            .resource::<MusicDirectorState>()
            .ending
            .as_ref()
            .is_some_and(|ending| ending.started)
        {
            break;
        }
    }
    assert!(
        app.world()
            .resource::<MusicDirectorState>()
            .ending
            .as_ref()
            .unwrap()
            .started
    );
    let cursor = app
        .world()
        .resource::<MusicPlayback>()
        .current()
        .unwrap()
        .position_seconds
        .unwrap();
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(30));
    for _ in 0..3 {
        app.world_mut()
            .run_system_once(drive_music_director)
            .unwrap();
    }
    let director = app.world().resource::<MusicDirectorState>();
    assert!(director.has_ending());
    assert_eq!(director.active_cue_id.as_deref(), Some("trail"));
    assert_eq!(director.current_state_id.as_deref(), Some("outro"));
    assert_eq!(director.ending.as_ref().unwrap().elapsed, cursor);
}

#[test]
fn withdrawing_a_transition_discards_its_old_exit() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "room", 0, 10));
    frames(&mut app, 7);
    publish(&mut app, selection("trail", "later", "room", 0, 10));
    step(&mut app, 0.0);
    publish(&mut app, selection("trail", "song", "room", 0, 10));
    step(&mut app, 0.0);
    let position = app
        .world()
        .resource::<MusicPlayback>()
        .current()
        .unwrap()
        .position_seconds
        .unwrap();
    step(&mut app, 4.5 - position);
    step(&mut app, 0.0);
    publish(&mut app, selection("trail", "later", "room", 0, 10));
    step(&mut app, 0.0);
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .current_section_id
            .as_deref(),
        Some("song")
    );
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 2);
}

#[test]
fn a_phase_binding_controls_whether_an_admitted_one_shot_finishes() {
    for policy in [
        MusicPhasePolicy::CompleteSection,
        MusicPhasePolicy::FollowPhase,
    ] {
        let mut app = decoded();
        let mut opening = selection("trail", "intro", "fight", 1, 200);
        opening.phase_policy = policy;
        publish(&mut app, opening);
        frames(&mut app, 6);
        let director = app.world().resource::<MusicDirectorState>();
        assert!(director.admitted().is_some());
        assert!(director.position_seconds.unwrap() < 4.0);
        publish(&mut app, selection("trail", "song", "fight", 1, 200));
        step(&mut app, 0.0);
        assert_eq!(
            app.world().resource::<RecordedMusicOutput>().submitted,
            if policy == MusicPhasePolicy::FollowPhase {
                4
            } else {
                2
            }
        );
        frames(&mut app, 3);
        let expected = if policy == MusicPhasePolicy::FollowPhase {
            "song"
        } else {
            "intro"
        };
        assert_eq!(
            app.world()
                .resource::<MusicDirectorState>()
                .current_state_id
                .as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn delayed_preparation_starts_the_current_phase_and_never_replays_the_old_opening() {
    let (mut app, handles) = fixture();
    let mut opening = selection("trail", "intro", "fight", 1, 200);
    opening.phase_policy = MusicPhasePolicy::FollowPhase;
    publish(&mut app, opening);
    frames(&mut app, 24);
    publish(&mut app, selection("trail", "song", "fight", 1, 200));
    for handle in &handles[..2] {
        decode(&mut app, handle);
    }
    frames(&mut app, 4);
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 0);
    for handle in &handles[2..4] {
        decode(&mut app, handle);
    }
    frames(&mut app, 4);
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.current_state_id.as_deref(), Some("song"));
    assert!(director.admitted().is_some());
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 2);
}

#[test]
fn the_active_phase_policy_survives_waiting_for_the_next_sections_decoder() {
    let (mut app, handles) = fixture();
    for handle in &handles[..2] {
        decode(&mut app, handle);
    }
    let mut opening = selection("trail", "intro", "fight", 1, 200);
    opening.phase_policy = MusicPhasePolicy::FollowPhase;
    publish(&mut app, opening);
    frames(&mut app, 6);
    publish(&mut app, selection("trail", "song", "fight", 1, 200));
    frames(&mut app, 2);
    for handle in &handles[2..4] {
        decode(&mut app, handle);
    }
    step(&mut app, 0.0);
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 4);
    frames(&mut app, 2);
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .current_state_id
            .as_deref(),
        Some("song")
    );
}

#[test]
fn selection_prepares_the_opening_and_its_continuation_without_distant_sections() {
    let (mut app, _) = fixture();
    let mut authored = authored_cues(60.0);
    for cue in &mut authored.cues {
        if let MusicCueMedia::Inline { asset_root, .. } = &mut cue.media {
            *asset_root = "audio/fixture".into();
        }
    }
    for cue in &mut authored.cues {
        if let MusicCueMedia::Inline { sections, .. } = &mut cue.media {
            sections
                .iter_mut()
                .find(|section| section.id == "song")
                .unwrap()
                .looped = false;
        }
        cue.states
            .iter_mut()
            .find(|state| state.id == "song")
            .unwrap()
            .next_state = Some("later".into());
    }
    let catalog =
        MusicCueCatalog::prepare(authored, &crate::music_media::MusicMediaCatalog::default())
            .unwrap();
    let mut catalogs = AdaptiveMusicCatalogRegistry::default();
    catalogs.register("game", catalog).unwrap();
    app.insert_resource(catalogs)
        .insert_resource(LoadedMusicCueAssets::default());
    step(&mut app, 0.0);
    let assets = app.world().resource::<LoadedMusicCueAssets>();
    assert_eq!(assets.sources.len(), 4);
    for (section, wanted) in [
        ("intro", true),
        ("song", true),
        ("later", false),
        ("outro", false),
    ] {
        assert_eq!(
            assets
                .sources
                .contains_key(&MusicSourceKey::new("game", 1, "trail", section, "bed")),
            wanted
        );
    }
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 0);
    assert!(
        app.world()
            .resource::<MusicDirectorState>()
            .admitted()
            .is_none()
    );
    let opening_handles: Vec<_> = ["intro", "song"]
        .into_iter()
        .flat_map(|section| {
            ["bed", "tune"]
                .into_iter()
                .map(move |layer| (section, layer))
        })
        .map(|(section, layer)| assets.get("game", 1, "trail", section, layer).unwrap())
        .collect();
    for handle in &opening_handles {
        decode(&mut app, handle);
    }
    frames(&mut app, 22);
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .current_state_id
            .as_deref(),
        Some("song")
    );
    let continuation: Vec<_> = ["bed", "tune"]
        .into_iter()
        .map(|layer| {
            app.world()
                .resource::<LoadedMusicCueAssets>()
                .get("game", 1, "trail", "later", layer)
                .unwrap()
        })
        .collect();
    for handle in &continuation {
        decode(&mut app, handle);
    }
    frames(&mut app, 20);
    assert_eq!(
        app.world()
            .resource::<MusicDirectorState>()
            .current_state_id
            .as_deref(),
        Some("later")
    );
}

#[test]
fn a_second_phase_change_can_cancel_a_submitted_section_without_losing_the_admitted_one() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "fight", 1, 200));
    frames(&mut app, 6);
    let position = app
        .world()
        .resource::<MusicPlayback>()
        .current()
        .unwrap()
        .position_seconds
        .unwrap();
    step(&mut app, 4.0 - position);
    step(&mut app, 0.0);
    publish(&mut app, selection("trail", "later", "fight", 1, 200));
    step(&mut app, 0.0);
    let admitted = app
        .world()
        .resource::<MusicDirectorState>()
        .admitted()
        .unwrap();
    assert!(
        matches!(&admitted.selection.choice, MusicChoice::Cue { state, .. } if state == "song")
    );
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 4);
    publish(&mut app, selection("trail", "intro", "fight", 1, 200));
    frames(&mut app, 5);
    let director = app.world().resource::<MusicDirectorState>();
    assert_eq!(director.current_state_id.as_deref(), Some("intro"));
    assert!(
        matches!(&director.admitted().unwrap().selection.choice, MusicChoice::Cue { state, .. } if state == "intro")
    );
    assert_eq!(app.world().resource::<RecordedMusicOutput>().resumed, 4);
}

#[test]
fn unused_cue_sources_leave_the_cache_while_outgoing_voices_retain_them() {
    let mut app = decoded();
    publish(&mut app, selection("trail", "song", "room", 0, 10));
    frames(&mut app, 4);
    let old_sources: Vec<_> = ["bed", "tune"]
        .into_iter()
        .map(|layer| {
            app.world()
                .resource::<LoadedMusicCueAssets>()
                .get("game", 1, "trail", "song", layer)
                .unwrap()
                .id()
        })
        .collect();
    app.world_mut()
        .resource_mut::<PreparedFixtureSources>()
        ._handles
        .retain(|handle| !old_sources.contains(&handle.id()));
    publish(&mut app, selection("destination", "song", "room", 0, 10));
    for _ in 0..4 {
        step(&mut app, 0.0);
    }
    let assets = app.world().resource::<LoadedMusicCueAssets>();
    assert!(assets.get("game", 1, "trail", "song", "bed").is_none());
    assert!(
        assets
            .get("game", 1, "destination", "song", "bed")
            .is_some()
    );
    assert_eq!(assets.sources.len(), 2);
    assert!(
        app.world()
            .resource::<MusicPlayback>()
            .groups()
            .any(|group| {
                group.phase == MusicGroupPhase::Retiring
                    && matches!(&group.product,
            MusicProduct::Section { cue, .. } if cue == "trail")
            })
    );
    assert!(old_sources.iter().all(|id| {
        app.world()
            .resource::<Assets<KiraAudioSource>>()
            .contains(*id)
    }));
    app.world_mut()
        .resource_mut::<MusicIntent>()
        .publish(MusicGoal::Silence {
            context: Some(ambition_sfx::AudioContextOwner::Direct),
            reason: MusicSilenceReason::NoEligibleChoice,
        });
    step(&mut app, 0.0);
    step(&mut app, 0.0);
    assert!(
        app.world()
            .resource::<LoadedMusicCueAssets>()
            .sources
            .is_empty()
    );
    step(&mut app, 1.0);
    for _ in 0..4 {
        step(&mut app, 0.0);
    }
    assert!(old_sources.iter().all(|id| {
        !app.world()
            .resource::<Assets<KiraAudioSource>>()
            .contains(*id)
    }));
}

#[test]
fn simple_tracks_and_explicit_preloads_have_separate_source_lifetimes() {
    let mut app = decoded();
    let registry = crate::spec::MusicRegistry {
        default_track: "ambient".into(),
        tracks: ["ambient", "radio"]
            .into_iter()
            .map(|id| crate::spec::MusicTrack {
                id: id.into(),
                display_name: id.into(),
                asset_path: Some(format!("audio/{id}.ogg")),
                one_shot: false,
            })
            .collect(),
    };
    let mut library = AudioLibrary::new(&registry, None);
    let ambient = library
        .resolve_track_handle("ambient", app.world().resource::<AssetServer>())
        .unwrap();
    decode(&mut app, &ambient);
    let ambient_id = ambient.id();
    drop(ambient);
    let preview = library
        .preload_track("radio", app.world().resource::<AssetServer>())
        .unwrap();
    decode(&mut app, &preview);
    let radio_id = preview.id();
    app.insert_resource(library);
    let mut authority = MusicAuthority::governed(["ambient".into(), "radio".into()]);
    authority.authorize_cues(["trail".into(), "destination".into()]);
    app.world_mut().resource_mut::<MusicIntent>().authority = authority;
    let mut target = selection("", "", "radio", 0, 400);
    target.choice = MusicChoice::Track("ambient".into());
    publish(&mut app, target.clone());
    frames(&mut app, 4);
    assert!(
        app.world()
            .resource::<AudioLibrary>()
            .resolved_track_handle("radio")
            .is_none()
    );
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 1);
    assert!(
        app.world()
            .resource::<Assets<KiraAudioSource>>()
            .contains(radio_id)
    );
    target.choice = MusicChoice::Track("radio".into());
    publish(&mut app, target);
    for _ in 0..4 {
        step(&mut app, 0.0);
    }
    drop(preview);
    assert!(
        app.world()
            .resource::<AudioLibrary>()
            .resolved_track_handle("ambient")
            .is_none()
    );
    assert!(
        app.world()
            .resource::<Assets<KiraAudioSource>>()
            .contains(ambient_id)
    );
    assert!(
        app.world()
            .resource::<MusicPlayback>()
            .groups()
            .any(|group| {
                group.phase == MusicGroupPhase::Retiring
                    && matches!(&group.product,
            MusicProduct::Track { id, .. } if id == "ambient")
            })
    );
    step(&mut app, 1.0);
    for _ in 0..4 {
        step(&mut app, 0.0);
    }
    assert!(
        !app.world()
            .resource::<Assets<KiraAudioSource>>()
            .contains(ambient_id)
    );
    publish(&mut app, selection("trail", "song", "room", 0, 10));
    for _ in 0..4 {
        step(&mut app, 0.0);
    }
    assert!(
        app.world()
            .resource::<AudioLibrary>()
            .resolved_track_handle("radio")
            .is_none()
    );
    assert!(
        app.world()
            .resource::<Assets<KiraAudioSource>>()
            .contains(radio_id)
    );
    assert!(
        app.world()
            .resource::<MusicPlayback>()
            .groups()
            .any(|group| {
                group.phase == MusicGroupPhase::Retiring
                    && matches!(&group.product,
            MusicProduct::Track { id, .. } if id == "radio")
            })
    );
}

#[test]
fn diagnostics_observe_prepared_sources_without_a_playback_cache_entry() {
    use crate::music::diagnostics::{
        MusicPlaybackDiagnostics, MusicPlaybackStatus, observe_music_playback,
    };
    use bevy::ecs::system::RunSystemOnce;

    let (mut app, prepared) = fixture();
    app.insert_resource(LoadedMusicCueAssets::default())
        .init_resource::<crate::selection::ActiveAudioSelection>()
        .init_resource::<MusicPlaybackDiagnostics>()
        .insert_resource(crate::web_unlock::AudioUnlockState {
            unlocked: true,
            ..default()
        })
        .insert_resource(crate::output::AudioOutputMode::Device);
    let observe = |app: &mut App| {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs_f64(0.25));
        app.world_mut()
            .run_system_once(observe_music_playback)
            .unwrap();
        app.world()
            .resource::<MusicPlaybackDiagnostics>()
            .status
            .clone()
            .unwrap()
    };

    assert_eq!(observe(&mut app), MusicPlaybackStatus::Loading);
    decode(&mut app, &prepared[0]);
    assert_eq!(observe(&mut app), MusicPlaybackStatus::Loading);
    decode(&mut app, &prepared[1]);
    assert_eq!(observe(&mut app), MusicPlaybackStatus::BackendSilent);
    assert!(
        app.world()
            .resource::<LoadedMusicCueAssets>()
            .sources
            .is_empty()
    );

    let registry = crate::spec::MusicRegistry {
        default_track: "ambient".into(),
        tracks: vec![crate::spec::MusicTrack {
            id: "ambient".into(),
            display_name: "Ambient".into(),
            asset_path: Some("audio/ambient.ogg".into()),
            one_shot: false,
        }],
    };
    let library = AudioLibrary::new(&registry, None);
    let prepared_track = library
        .preload_track("ambient", app.world().resource::<AssetServer>())
        .unwrap();
    app.insert_resource(library);
    let mut target = selection("", "", "room", 0, 10);
    target.choice = MusicChoice::Track("ambient".into());
    publish(&mut app, target);
    assert_eq!(observe(&mut app), MusicPlaybackStatus::Loading);
    decode(&mut app, &prepared_track);
    assert_eq!(observe(&mut app), MusicPlaybackStatus::BackendSilent);
    assert!(
        app.world()
            .resource::<AudioLibrary>()
            .resolved_track_handle("ambient")
            .is_none()
    );
    assert_eq!(app.world().resource::<RecordedMusicOutput>().submitted, 0);
}
