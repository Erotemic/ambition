use super::*;
use crate::audio::cue_spec::*;
use crate::platformer::lifecycle::{
    InRoomInstance, LiveRoomInstance, RoomInstanceRoot, insert_session_world_component,
    session_world_component,
};
use crate::world::rooms::{Authored, RoomSet};

fn catalog() -> PreparedMusicCatalog {
    let cue = MusicCueSpec {
        id: "guard".into(),
        playback: Default::default(),
        media: MusicCueMedia::Inline {
            asset_root: "audio/guard".into(),
            bpm: 120.0,
            beats_per_bar: 4.0,
            sections: ["intro", "song", "outro"]
                .into_iter()
                .map(|id| MusicSectionSpec {
                    id: id.into(),
                    duration_beats: 4.0,
                    looped: id == "song",
                    sources: vec![MusicLayerSourceSpec {
                        layer_id: "full".into(),
                        path: format!("{id}.wav"),
                    }],
                })
                .collect(),
        },
        relative_volume: 1.0,
        layers: vec![MusicLayerSpec {
            id: "full".into(),
            slot: 0,
        }],
        states: ["intro", "song", "outro"]
            .into_iter()
            .map(|id| MusicStateSpec {
                id: id.into(),
                section_id: id.into(),
                next_state: (id == "intro").then(|| "song".into()),
                gains: vec![MusicLayerGainSpec {
                    layer_id: "full".into(),
                    gain: 1.0,
                }],
            })
            .collect(),
        outro_state: Some("outro".into()),
        post_clear_bridge_state: None,
        runtime_balance_overrides: Vec::new(),
    };
    let cues = PreparedMusicCues::prepare(
        AuthoredMusicCues {
            cues: vec![cue],
            boss_bindings: vec![BossMusicBinding {
                boss_id: "sleeping_guard".into(),
                cue_id: "guard".into(),
                phase_states: [("Intro", "intro"), ("Phase1", "song")]
                    .into_iter()
                    .map(|(phase, state)| BossPhaseState {
                        phase: phase.into(),
                        state: state.into(),
                        policy: MusicPhasePolicy::FollowPhase,
                    })
                    .collect(),
            }],
            room_bindings: vec![RoomMusicBinding {
                room_id: "chapel".into(),
                cue_id: "guard".into(),
                state: "song".into(),
                entry_state: Some("intro".into()),
                fight_state: None,
            }],
            ..default()
        },
        &Default::default(),
    )
    .unwrap();
    PreparedMusicCatalog {
        provider_id: "game".into(),
        cues: Some(cues),
        music: Some(crate::audio::spec::MusicRegistry {
            default_track: "ambient".into(),
            tracks: ["ambient", "away", "override"]
                .into_iter()
                .map(|id| crate::audio::spec::MusicTrack {
                    id: id.into(),
                    display_name: id.into(),
                    asset_path: Some(format!("audio/{id}.wav")),
                    one_shot: false,
                })
                .collect(),
        }),
    }
}

fn room(id: &str) -> RoomSpec {
    RoomSpec::new(
        id,
        crate::engine_core::World::new(id, Vec2::splat(800.0), Vec2::ZERO, Vec::new()),
    )
}

fn throne() -> RoomSpec {
    let mut room = room("throne");
    room.boss_spawns.push(Authored::new(
        "placed-guard",
        "Display name",
        crate::engine_core::Aabb::new(Vec2::ZERO, Vec2::splat(30.0)),
        crate::entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "sleeping_guard".into(),
        },
    ));
    room
}

fn bosses() -> BossCatalog {
    use crate::boss_encounter::{BossCatalogFragment, BossCatalogRegistry};
    let profiles = crate::boss_encounter::behavior::BossProfileRegistry::from_ron(include_str!(
        "../../../../../game/ambition_content/assets/data/boss_profiles.ron"
    ));
    let mut behavior = profiles.get("clockwork_warden").unwrap().clone();
    behavior.id = "sleeping_guard".into();
    let mut encounter = crate::boss_encounter::BossEncounterSpec::gradient_sentinel();
    encounter.id = behavior.id.clone();
    let fragment = BossCatalogFragment::from_prepared(
        "fixture",
        Some("sleeping_guard"),
        None::<String>,
        BTreeMap::from([(behavior.id.clone(), behavior)]),
        BTreeMap::from([(encounter.id.clone(), encounter)]),
        "{}",
        Default::default(),
    )
    .unwrap();
    let mut registry = BossCatalogRegistry::default();
    registry.register(fragment).unwrap();
    registry.assemble().unwrap()
}

struct Files(std::path::PathBuf);
impl Files {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "ambition-music-preparation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        Self(directory)
    }
    fn wav(&self, path: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let data_bytes = 960u32;
        let mut wav = Vec::new();
        wav.extend(b"RIFF");
        wav.extend((36 + data_bytes).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(48_000u32.to_le_bytes());
        wav.extend(96_000u32.to_le_bytes());
        wav.extend(2u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend(data_bytes.to_le_bytes());
        wav.resize(44 + data_bytes as usize, 0);
        std::fs::write(path, wav).unwrap();
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn selected_rooms_decode_the_dormant_entrance_without_a_fight_claim_and_reuse_it_at_activation() {
    let files = Files::new();
    for path in [
        "audio/ambient.wav",
        "audio/away.wav",
        "audio/guard/intro.wav",
        "audio/guard/song.wav",
    ] {
        files.wav(path);
    }
    let catalog = catalog();
    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        bevy::asset::AssetPlugin {
            file_path: files.0.to_string_lossy().into_owned(),
            ..default()
        },
    ))
    .insert_resource(crate::audio::AudioOutputMode::Recording)
    .add_plugins(crate::audio::AmbitionAudioBackendPlugin)
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ))
    .insert_resource(
        crate::audio::selection::ActiveAudioSelection::selected_direct(
            "game",
            catalog.music.clone(),
            None,
            Default::default(),
        ),
    )
    .insert_resource(crate::audio::library::AudioLibrary::new(
        catalog.music.as_ref().unwrap(),
        None,
    ))
    .init_resource::<crate::audio::music::MusicIntent>()
    .add_message::<crate::encounter::EncounterRelease>()
    .init_resource::<RoomMusicPreparation>()
    .init_resource::<super::super::room_transition_assets::RoomPreparationPrefetchState>()
    .insert_resource(bosses())
    .insert_resource(crate::construction::mechanics::SessionMechanics {
        bosses: bosses(),
        ..Default::default()
    })
    .add_systems(
        Update,
        (
            prepare_room_music,
            crate::platformer_audio::music::compute_music_intent,
        )
            .chain(),
    );
    // The host has no cue catalog for the incoming session.
    app.init_resource::<AdaptiveMusicCatalogRegistry>();
    let mut away = room("away");
    away.metadata.music_track = Some("away".into());
    crate::world::rooms::insert_room_set(
        app.world_mut(),
        RoomSet::from_parts_or_panic("hall", vec![room("hall"), throne(), away], Vec::new()),
    );
    insert_session_world_component(app.world_mut(), catalog);
    insert_session_world_component(
        app.world_mut(),
        crate::encounter::EncounterMusicRequest::default(),
    );
    app.world_mut().spawn((
        crate::platformer::body::PrimaryBody,
        crate::characters::control::DrivingParticipant(crate::characters::control::PlayerSlot(0)),
        InRoomInstance(LiveRoomInstance::ACTIVATION),
    ));
    app.world_mut()
        .resource_mut::<super::super::room_transition_assets::RoomPreparationPrefetchState>()
        .selected_targets = vec!["throne".into()];
    for _ in 0..300 {
        app.update();
        let handles = &app.world().resource::<RoomMusicPreparation>().sources;
        let decoded = app.world().resource::<Assets<KiraAudioSource>>();
        if handles.len() == 3 && handles.values().all(|handle| decoded.contains(handle.id())) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let prepared = &app.world().resource::<RoomMusicPreparation>().sources;
    assert_eq!(
        prepared.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "audio/ambient.wav",
            "audio/guard/intro.wav",
            "audio/guard/song.wav"
        ]
    );
    assert!(prepared.values().all(|handle| {
        app.world()
            .resource::<Assets<KiraAudioSource>>()
            .contains(handle.id())
    }));
    let opening = prepared["audio/guard/intro.wav"].id();
    assert!(
        session_world_component::<crate::encounter::EncounterMusicRequest>(app.world())
            .unwrap()
            .candidates()
            .next()
            .is_none()
    );
    assert!(
        matches!(&app.world().resource::<crate::audio::music::MusicIntent>().goal,
        Some(crate::audio::music::MusicGoal::Play(selected)) if selected.choice == crate::audio::music::MusicChoice::Track("ambient".into()))
    );
    let definition = session_world_component::<RoomSet>(app.world())
        .unwrap()
        .definition_by_id("throne")
        .unwrap();
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<RoomInstanceRoot>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(root).insert(definition);
    app.world_mut()
        .resource_mut::<super::super::room_transition_assets::RoomPreparationPrefetchState>()
        .selected_targets
        .clear();
    app.update();
    assert_eq!(
        app.world().resource::<RoomMusicPreparation>().sources["audio/guard/intro.wav"].id(),
        opening
    );
    let definition = session_world_component::<RoomSet>(app.world())
        .unwrap()
        .definition_by_id("away")
        .unwrap();
    app.world_mut().entity_mut(root).insert(definition);
    app.update();
    assert_eq!(
        app.world()
            .resource::<RoomMusicPreparation>()
            .sources
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["audio/away.wav"]
    );
}

#[test]
fn an_adaptive_room_prepares_its_entry_and_a_room_fight_override_replaces_the_boss_products() {
    let catalog = catalog();
    let mut chapel = room("chapel");
    chapel.metadata.music_track = Some("away".into());
    let sources = room_music_sources(&catalog, &chapel, &bosses(), None, None);
    assert_eq!(
        sources.opening,
        BTreeSet::from(["audio/guard/intro.wav".into()])
    );
    assert_eq!(
        sources.anticipated,
        BTreeSet::from(["audio/guard/song.wav".into()])
    );
    let mut throne = throne();
    throne.metadata.fight_music_track = Some("override".into());
    let sources = room_music_sources(&catalog, &throne, &bosses(), None, None);
    assert_eq!(
        sources.opening,
        BTreeSet::from(["audio/ambient.wav".into()])
    );
    assert_eq!(
        sources.anticipated,
        BTreeSet::from(["audio/override.wav".into()])
    );
}
