use super::*;
use crate::cue_spec::{MusicLayerGainSpec, MusicLayerSourceSpec, MusicLayerSpec, MusicSectionSpec};
use crate::music::catalog::MusicSourceKey;
use crate::selection::MusicAuthority;
use bevy::asset::AssetApp;
use bevy_kira_audio::prelude::{Frame, StaticSoundData, StaticSoundSettings};
use std::time::Duration;

fn authored_cues(bpm: f32) -> crate::cue_spec::AuthoredMusicCues {
    let cue = MusicCueSpec {
        id: "trail".into(),
        asset_root: "content://library/music".into(),
        bpm,
        beats_per_bar: 4.0,
        relative_volume: 1.0,
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
    let mut loaded = LoadedMusicCueAssets::default();
    let mut handles = Vec::new();
    for cue in &authored.cues {
        for section in &cue.sections {
            for source in &section.sources {
                let handle = app
                    .world()
                    .resource::<Assets<KiraAudioSource>>()
                    .reserve_handle();
                loaded.sources.insert(
                    MusicSourceKey::new("game", 1, &cue.id, &section.id, &source.layer_id),
                    (cue.source_path(source), handle.clone()),
                );
                handles.push(handle);
            }
        }
    }
    let mut catalogs = AdaptiveMusicCatalogRegistry::default();
    catalogs
        .register("game", MusicCueCatalog::prepare(authored).unwrap())
        .unwrap();
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
fn step(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs_f32(seconds));
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
fn decoded() -> App {
    let (mut app, handles) = fixture();
    for handle in handles {
        decode(&mut app, &handle);
    }
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
    assert_eq!(director.seconds_in_mode, 0.0);
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
    let next = MusicCueCatalog::prepare(authored_cues(300.0)).unwrap();
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
