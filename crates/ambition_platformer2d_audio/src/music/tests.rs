use super::compute_music_intent;
use ambition_audio::{
    library::{AudioLibrary, RadioStationState},
    music::*,
    selection::ActiveAudioSelection,
    spec::{MusicRegistry, MusicTrack},
};
use ambition_boss_encounter::{
    BossEncounterPhase,
    test_support::{test_boss_config, test_boss_status},
};
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_encounter::{
    Encounter, EncounterCommandKind, EncounterLifecycle, EncounterMusicRequest,
    EncounterParticipants, EncounterWaves, MusicClaimTier, MusicOccurrence,
};
use ambition_input::participant::InputParticipant;
use ambition_platformer2d_shared_tangle::{
    body::PrimaryBody,
    lifecycle::{
        InRoomInstance, LiveRoomInstance, RoomInstanceRoot, insert_session_world_component,
        session_world_component, session_world_component_mut,
    },
    sim_id::SimId,
};
use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};
use bevy::prelude::*;

fn cue(id: &str) -> MusicCueSpec {
    MusicCueSpec {
        id: id.into(),
        playback: Default::default(),
        media: MusicCueMedia::Inline {
            asset_root: "audio/fixture".into(),
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
                        path: format!("{id}.ogg"),
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
    }
}

fn fixture(override_track: bool) -> (App, Entity, [LiveRoomInstance; 2]) {
    let mut app = App::new();
    let registry = MusicRegistry {
        default_track: "ambient".into(),
        tracks: ["ambient", "wave", "override", "radio", "narrative"]
            .into_iter()
            .map(|id| MusicTrack {
                id: id.into(),
                display_name: id.into(),
                asset_path: Some(format!("audio/{id}.ogg")),
                one_shot: false,
            })
            .collect(),
    };
    app.insert_resource(AudioLibrary::new(&registry, None))
        .insert_resource(ActiveAudioSelection::selected_direct(
            "game",
            Some(registry),
            None,
            Default::default(),
        ))
        .init_resource::<MusicIntent>()
        .init_resource::<RadioStationState>()
        .init_resource::<ambition_conversation::NarrativeMusicRequest>()
        .add_message::<ambition_encounter::EncounterRelease>();
    let mut first = RoomSpec::new(
        "hall",
        ambition_platformer2d_core::World::new(
            "hall",
            Vec2::splat(800.0),
            Vec2::splat(16.0),
            Vec::new(),
        ),
    );
    first.metadata.music_track = Some("ambient".into());
    first.metadata.fight_music_track = override_track.then(|| "override".into());
    let second = RoomSpec::new(
        "chapel",
        ambition_platformer2d_core::World::new(
            "chapel",
            Vec2::splat(800.0),
            Vec2::splat(16.0),
            Vec::new(),
        ),
    );
    ambition_platformer2d_world::rooms::insert_room_set(
        app.world_mut(),
        RoomSet::from_parts_or_panic("hall", vec![first, second], Vec::new()),
    );
    let second_definition = session_world_component::<RoomSet>(app.world())
        .unwrap()
        .definition_by_id("chapel")
        .unwrap();
    let rooms = [
        LiveRoomInstance::ACTIVATION,
        LiveRoomInstance::ACTIVATION.next(),
    ];
    app.world_mut()
        .spawn((RoomInstanceRoot, rooms[1], second_definition));
    let primary = app
        .world_mut()
        .spawn((
            PrimaryBody,
            DrivingParticipant(PlayerSlot(0)),
            InRoomInstance(rooms[0]),
        ))
        .id();
    app.world_mut().spawn(InputParticipant::primary());
    insert_session_world_component(app.world_mut(), EncounterMusicRequest::default());
    let catalog = MusicCueCatalog::prepare(
        ambition_audio::cue_spec::AuthoredMusicCues {
            cues: vec![cue("fight_program"), cue("destination")],
            encounter_bindings: vec![EncounterMusicBinding {
                encounter_id: "fight".into(),
                cue_id: "fight_program".into(),
                starting_state: "intro".into(),
                wave_states: vec!["song".into()],
                wave2_reinforced_state: None,
                cleared_state: "outro".into(),
            }],
            boss_bindings: vec![BossMusicBinding {
                boss_id: "wolfenstein_boss".into(),
                cue_id: "fight_program".into(),
                phase_states: vec![
                    BossPhaseState {
                        phase: "Intro".into(),
                        state: "intro".into(),
                        policy: MusicPhasePolicy::FollowPhase,
                    },
                    BossPhaseState {
                        phase: "Phase1".into(),
                        state: "song".into(),
                        policy: MusicPhasePolicy::CompleteSection,
                    },
                ],
            }],
            room_bindings: vec![RoomMusicBinding {
                room_id: "chapel".into(),
                cue_id: "destination".into(),
                state: "song".into(),
                entry_state: None,
                fight_state: None,
            }],
            ..default()
        },
        &ambition_audio::music_media::MusicMediaCatalog::default(),
    )
    .unwrap();
    let mut catalogs = AdaptiveMusicCatalogRegistry::default();
    catalogs.register("game", catalog).unwrap();
    app.insert_resource(catalogs)
        .add_systems(Update, compute_music_intent);
    (app, primary, rooms)
}

fn started() -> EncounterLifecycle {
    let mut state = EncounterLifecycle::default();
    state.reduce(
        0.0,
        [&EncounterCommandKind::Start],
        &EncounterParticipants::default(),
        None,
    );
    state
}
fn source(room: LiveRoomInstance, id: &SimId, track: Option<&str>) -> MusicOccurrence {
    MusicOccurrence {
        room: Some(room),
        source_id: id.as_str().into(),
        attempt: 1,
        track: track.map(str::to_owned),
    }
}
fn publish(
    app: &mut App,
    kind: &'static str,
    tier: MusicClaimTier,
    tick: u64,
    sources: Vec<MusicOccurrence>,
) {
    session_world_component_mut::<EncounterMusicRequest>(app.world_mut())
        .unwrap()
        .publish_occurrences(kind, tier, tick, sources);
}
fn boss(app: &mut App, id: &str, room: LiveRoomInstance) -> (Entity, SimId) {
    let source = SimId::placement(id);
    let (status, _) = test_boss_status(18, BossEncounterPhase::Phase1);
    app.world_mut().spawn((
        test_boss_config(id, id, "wolfenstein_boss"),
        status,
        source.clone(),
        InRoomInstance(room),
    ));
    let lifecycle = app
        .world_mut()
        .spawn((
            Encounter::new(id),
            started(),
            SimId::encounter(id),
            InRoomInstance(room),
        ))
        .id();
    (lifecycle, source)
}
fn wave(app: &mut App, room: LiveRoomInstance) -> SimId {
    let spec = ron::from_str(r#"(id:"fight",room_id:"hall",waves:[],trigger_min:(0.0,0.0),trigger_size:(10.0,10.0),camera_zoom:1.0,lock_wall:None,intro_seconds:0.0,music_track:"")"#).unwrap();
    let id = SimId::encounter("fight");
    app.world_mut().spawn((
        Encounter::new("fight"),
        started(),
        EncounterWaves::new(spec),
        id.clone(),
        InRoomInstance(room),
    ));
    id
}
fn winner(app: &App) -> &MusicSelection {
    match app.world().resource::<MusicIntent>().goal.as_ref().unwrap() {
        MusicGoal::Play(winner) => winner,
        other => panic!("expected music, got {other:?}"),
    }
}

#[test]
fn departing_boss_selects_the_destination_program_in_one_revision() {
    let (mut app, primary, rooms) = fixture(false);
    let (_, id) = boss(&mut app, "wolf", rooms[0]);
    publish(
        &mut app,
        "boss_encounter",
        MusicClaimTier::Focused,
        1,
        vec![source(rooms[0], &id, None)],
    );
    app.update();
    assert!(matches!(&winner(&app).choice, MusicChoice::Cue { id, .. } if id == "fight_program"));
    let revision = app.world().resource::<MusicIntent>().revision;
    app.world_mut()
        .entity_mut(primary)
        .insert(InRoomInstance(rooms[1]));
    app.update();
    assert!(matches!(&winner(&app).choice, MusicChoice::Cue { id, .. } if id == "destination"));
    assert_eq!(app.world().resource::<MusicIntent>().revision, revision + 1);
    app.update();
    assert_eq!(app.world().resource::<MusicIntent>().revision, revision + 1);
}

#[test]
fn a_local_secondary_fight_beats_primary_ambient_but_a_remote_seat_does_not() {
    let (mut app, primary, rooms) = fixture(false);
    app.world_mut()
        .entity_mut(primary)
        .insert(InRoomInstance(rooms[1]));
    app.world_mut()
        .spawn((DrivingParticipant(PlayerSlot(1)), InRoomInstance(rooms[0])));
    let (_, id) = boss(&mut app, "wolf", rooms[0]);
    publish(
        &mut app,
        "boss_encounter",
        MusicClaimTier::Focused,
        1,
        vec![source(rooms[0], &id, None)],
    );
    app.update();
    assert_eq!(winner(&app).claim.scope, Some(rooms[1].ordinal()));
    app.world_mut().spawn(InputParticipant::with_id(
        ambition_input::participant::ParticipantId(1),
    ));
    app.update();
    assert_eq!(winner(&app).claim.scope, Some(rooms[0].ordinal()));
    assert!(matches!(&winner(&app).choice, MusicChoice::Cue { id, .. } if id == "fight_program"));
}

#[test]
fn room_fight_override_keeps_the_fights_owner_priority_and_attempt() {
    let (mut app, _, rooms) = fixture(true);
    let (_, id) = boss(&mut app, "wolf", rooms[0]);
    publish(
        &mut app,
        "boss_encounter",
        MusicClaimTier::Focused,
        1,
        vec![source(rooms[0], &id, None)],
    );
    app.update();
    let selected = winner(&app);
    assert_eq!(selected.choice, MusicChoice::Track("override".into()));
    assert_eq!((selected.priority, selected.claim.attempt), (300, 1));
    assert!(selected.claim.source.contains(id.as_str()));
    assert_eq!(
        selected.release_owner.as_ref().unwrap().source,
        SimId::encounter("wolf").as_str()
    );
}

#[test]
fn same_archetype_sources_are_stable_and_a_losing_failure_cannot_stop_the_winner() {
    for reverse in [false, true] {
        let (mut app, _, rooms) = fixture(false);
        let mut ids = ["a", "b"];
        if reverse {
            ids.reverse();
        }
        let mut rows = Vec::new();
        for id in ids {
            let (entity, source_id) = boss(&mut app, id, rooms[0]);
            rows.push((id, entity, source_id));
        }
        publish(
            &mut app,
            "boss_encounter",
            MusicClaimTier::Focused,
            1,
            rows.iter()
                .map(|(_, _, id)| source(rooms[0], id, None))
                .collect(),
        );
        app.update();
        let selected = winner(&app).clone();
        assert!(
            selected
                .claim
                .source
                .ends_with(SimId::placement("a").as_str())
        );
        let loser = rows.iter().find(|(id, _, _)| *id == "b").unwrap().1;
        app.world_mut()
            .get_mut::<EncounterLifecycle>(loser)
            .unwrap()
            .reduce(
                0.0,
                [&EncounterCommandKind::Fail],
                &EncounterParticipants::default(),
                None,
            );
        let id = SimId::placement("a");
        publish(
            &mut app,
            "boss_encounter",
            MusicClaimTier::Focused,
            2,
            vec![source(rooms[0], &id, None)],
        );
        app.update();
        assert_eq!(winner(&app), &selected);
    }
}

#[test]
fn within_room_recency_does_not_outrank_the_primary_room() {
    let (mut app, _, rooms) = fixture(false);
    app.world_mut()
        .spawn((DrivingParticipant(PlayerSlot(1)), InRoomInstance(rooms[1])));
    app.world_mut().spawn(InputParticipant::with_id(
        ambition_input::participant::ParticipantId(1),
    ));
    let first = SimId::placement("first");
    let second = SimId::placement("second");
    publish(
        &mut app,
        "encounter_script",
        MusicClaimTier::Focused,
        1,
        vec![source(rooms[0], &first, Some("wave"))],
    );
    publish(
        &mut app,
        "other_source",
        MusicClaimTier::Focused,
        99,
        vec![source(rooms[1], &second, Some("wave"))],
    );
    app.update();
    assert_eq!(winner(&app).claim.scope, Some(rooms[0].ordinal()));
}

#[test]
fn radio_replaces_an_adaptive_fight_and_releases_it_back_to_current_arbitration() {
    let (mut app, _, rooms) = fixture(false);
    let id = wave(&mut app, rooms[0]);
    publish(
        &mut app,
        "encounter",
        MusicClaimTier::Base,
        1,
        vec![source(rooms[0], &id, None)],
    );
    app.update();
    assert!(matches!(&winner(&app).choice, MusicChoice::Cue { id, .. } if id == "fight_program"));
    app.world_mut()
        .resource_mut::<RadioStationState>()
        .set_selected_track("radio");
    app.update();
    assert_eq!(winner(&app).choice, MusicChoice::Track("radio".into()));
    app.world_mut().resource_mut::<RadioStationState>().clear();
    app.update();
    assert!(matches!(&winner(&app).choice, MusicChoice::Cue { id, .. } if id == "fight_program"));
}

#[test]
fn cue_only_selection_does_not_require_a_simple_track_library() {
    let (mut app, _, rooms) = fixture(false);
    app.world_mut().remove_resource::<AudioLibrary>();
    let (_, id) = boss(&mut app, "wolf", rooms[0]);
    publish(
        &mut app,
        "boss_encounter",
        MusicClaimTier::Focused,
        1,
        vec![source(rooms[0], &id, None)],
    );
    app.update();
    assert!(matches!(&winner(&app).choice, MusicChoice::Cue { id, .. } if id == "fight_program"));
}

#[test]
fn identical_authored_boss_ids_in_different_rooms_keep_their_own_lifecycle() {
    let (mut app, _, rooms) = fixture(false);
    let (live, id) = boss(&mut app, "wolf", rooms[0]);
    let (failed, _) = boss(&mut app, "wolf", rooms[1]);
    app.world_mut()
        .get_mut::<EncounterLifecycle>(failed)
        .unwrap()
        .reduce(
            0.0,
            [&EncounterCommandKind::Fail],
            &EncounterParticipants::default(),
            None,
        );
    publish(
        &mut app,
        "boss_encounter",
        MusicClaimTier::Focused,
        1,
        vec![source(rooms[0], &id, None), source(rooms[1], &id, None)],
    );
    app.update();
    assert_eq!(winner(&app).claim.scope, Some(rooms[0].ordinal()));
    assert_eq!(winner(&app).priority, 300);
    assert!(
        app.world()
            .get::<EncounterLifecycle>(live)
            .unwrap()
            .phase()
            .in_flight()
    );
}

#[test]
fn a_complete_room_projection_without_eligible_music_retires_the_old_goal() {
    let (mut app, _, rooms) = fixture(false);
    let (_, id) = boss(&mut app, "wolf", rooms[0]);
    publish(
        &mut app,
        "boss_encounter",
        MusicClaimTier::Focused,
        1,
        vec![source(rooms[0], &id, None)],
    );
    app.update();
    assert_eq!(winner(&app).priority, 300);
    let empty = MusicRegistry {
        default_track: String::new(),
        tracks: Vec::new(),
    };
    app.insert_resource(AudioLibrary::new(&empty, None));
    publish(
        &mut app,
        "boss_encounter",
        MusicClaimTier::Focused,
        2,
        Vec::new(),
    );
    app.update();
    assert!(matches!(
        app.world().resource::<MusicIntent>().goal,
        Some(MusicGoal::Silence {
            reason: MusicSilenceReason::NoEligibleChoice,
            ..
        })
    ));
}

#[test]
fn a_remote_primary_body_cannot_clear_the_local_rooms_narrative_request() {
    let (mut app, remote, rooms) = fixture(false);
    app.add_systems(
        Update,
        super::release_narrative_music_on_room_change.before(compute_music_intent),
    );
    app.world_mut()
        .entity_mut(remote)
        .insert((DrivingParticipant(PlayerSlot(9)), InRoomInstance(rooms[1])));
    app.world_mut()
        .spawn((DrivingParticipant(PlayerSlot(0)), InRoomInstance(rooms[1])));
    app.world_mut()
        .resource_mut::<ambition_conversation::NarrativeMusicRequest>()
        .request("narrative");
    app.update();
    assert_eq!(winner(&app).choice, MusicChoice::Track("narrative".into()));
    app.world_mut()
        .entity_mut(remote)
        .insert(InRoomInstance(rooms[0]));
    app.update();
    assert_eq!(winner(&app).choice, MusicChoice::Track("narrative".into()));
}

#[test]
fn boss_phase_policy_follows_the_canonical_phase_and_attempt() {
    let (mut app, _, rooms) = fixture(false);
    let (_, id) = boss(&mut app, "placed_boss", rooms[0]);
    let body = app
        .world_mut()
        .query::<(Entity, &SimId)>()
        .iter(app.world())
        .find(|(_, candidate)| **candidate == id)
        .unwrap()
        .0;
    app.world_mut()
        .entity_mut(body)
        .insert(test_boss_status(18, BossEncounterPhase::Intro).0);
    app.add_systems(
        Update,
        ambition_boss_encounter::publish_boss_music.before(compute_music_intent),
    );
    app.update();
    let opening = winner(&app).clone();
    assert_eq!(opening.phase_policy, MusicPhasePolicy::FollowPhase);
    assert!(matches!(&opening.choice, MusicChoice::Cue { state, .. } if state == "intro"));
    app.world_mut()
        .entity_mut(body)
        .insert(test_boss_status(18, BossEncounterPhase::Phase1).0);
    app.update();
    let fighting = winner(&app);
    assert_eq!(fighting.claim, opening.claim);
    assert_eq!(fighting.phase_policy, MusicPhasePolicy::CompleteSection);
    assert!(matches!(&fighting.choice, MusicChoice::Cue { state, .. } if state == "song"));
}

#[test]
fn a_delayed_boss_track_waits_for_the_authored_phase_time() {
    for overridden in [false, true] {
        let (mut app, _, rooms) = fixture(overridden);
        app.insert_resource(AdaptiveMusicCatalogRegistry::default());
        let (_, id) = boss(&mut app, "delayed_boss", rooms[0]);
        let body = app
            .world_mut()
            .query::<(Entity, &SimId)>()
            .iter(app.world())
            .find(|(_, candidate)| **candidate == id)
            .unwrap()
            .0;
        {
            let mut config = app
                .world_mut()
                .get_mut::<ambition_boss_encounter::BossConfig>(body)
                .unwrap();
            config.seed.encounter.music_intro = "wave".into();
            config.seed.encounter.music_intro_from = 3.0;
        }
        let mut status = test_boss_status(18, BossEncounterPhase::Intro).0;
        status.encounter.as_mut().unwrap().phase_elapsed = 2.0;
        app.world_mut().entity_mut(body).insert(status);
        app.add_systems(
            Update,
            ambition_boss_encounter::publish_boss_music.before(compute_music_intent),
        );
        app.update();
        assert!(
            !session_world_component::<EncounterMusicRequest>(app.world())
                .unwrap()
                .candidates()
                .any(|candidate| candidate.source.kind()
                    == ambition_boss_encounter::BOSS_MUSIC_SOURCE_KIND)
        );
        assert_eq!(winner(&app).choice, MusicChoice::Track("ambient".into()));
        app.world_mut()
            .get_mut::<ambition_boss_encounter::BossEncounter>(body)
            .unwrap()
            .encounter
            .as_mut()
            .unwrap()
            .phase_elapsed = 3.0;
        app.update();
        let expected = if overridden { "override" } else { "wave" };
        assert_eq!(winner(&app).choice, MusicChoice::Track(expected.into()));
    }
}
