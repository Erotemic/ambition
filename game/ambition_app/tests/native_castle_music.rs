//! Native music through the visible host and Castle Godwin lifecycle.

#![cfg(all(feature = "visible", feature = "audio", not(target_arch = "wasm32")))]

use std::time::{Duration, Instant};

use ambition_app::app::{build_visible_app_with, VisibleRenderMode};
use ambition_platformer2d::audio::{
    catalog::PreparedMusicCatalog,
    library::KiraAudioSource,
    music::{
        playback::{MusicGroupPhase, MusicPlayback, MusicProduct},
        MusicChoice, MusicGoal, MusicIntent,
    },
    AudioBackendState, AudioOutputMode,
};
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounter, BossEncounterPhase};
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::{
    lifecycle::session_world_component, markers::PrimaryPlayerOnly,
};
use bevy::prelude::*;

const THRONE: &str = "castle_godwin_throne";
const HALL: &str = "castle_godwin_great_hall";

fn tick(app: &mut App, previous: &mut Instant) {
    std::thread::sleep(Duration::from_millis(8));
    let now = Instant::now();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        now - *previous,
    ));
    *previous = now;
    app.update();
    let playback = app.world().resource::<MusicPlayback>();
    assert!(playback.reserved_voices() <= 12);
    let intent = app.world().resource::<MusicIntent>();
    assert!(
        !matches!(&intent.goal,
        Some(MusicGoal::Play(selected)) if selected.choice == MusicChoice::Track("military_iron_resolve".into())),
        "the room change selected an intermediate simple track"
    );
}

fn wait(
    app: &mut App,
    previous: &mut Instant,
    what: &str,
    mut ready: impl FnMut(&mut App) -> bool,
) {
    let started = Instant::now();
    loop {
        tick(app, previous);
        if ready(app) {
            let current = app.world().resource::<MusicPlayback>().current();
            eprintln!(
                "native castle: {what}; transport={:?}",
                current.map(|group| (
                    group.occurrence,
                    &group.product,
                    group.position_seconds
                ))
            );
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(25),
            "waiting for {what}: goal={:?}, transport={:?}",
            app.world().resource::<MusicIntent>().goal,
            app.world().resource::<MusicPlayback>().current()
        );
    }
}

fn running(app: &App, cue: &str, section: &str) -> bool {
    app.world()
        .resource::<MusicPlayback>()
        .current()
        .is_some_and(|group| {
            group.phase == MusicGroupPhase::Running
                && group
                    .position_seconds
                    .is_some_and(|position| position > 0.1)
                && matches!(&group.product, MusicProduct::Section { cue: c, section: s, .. }
                if c == cue && s == section)
        })
}

fn entrance_decoded(app: &App) -> bool {
    let Some(catalog) = session_world_component::<PreparedMusicCatalog>(app.world()) else {
        return false;
    };
    let cues = catalog.cues.as_ref().expect("prepared cues");
    let binding = cues.binding_for_boss("wolfenstein_boss").unwrap();
    let cue = cues.cue(&binding.cue_id).unwrap();
    let server = app.world().resource::<AssetServer>();
    let decoded = app.world().resource::<Assets<KiraAudioSource>>();
    ["Intro", "Phase1"].into_iter().all(|phase| {
        let state = cue.state(binding.state_in(phase).unwrap()).unwrap();
        let section = cue.section(&state.section_id).unwrap();
        !section.sources.is_empty()
            && section.sources.iter().all(|source| {
                server
                    .get_handle::<KiraAudioSource>(source.path.clone())
                    .is_some_and(|handle| decoded.contains(handle.id()))
            })
    })
}

fn wolf(app: &mut App) -> (ae::Vec2, BossEncounterPhase) {
    let world = app.world_mut();
    let (_, body, encounter) = world
        .query::<(&BossConfig, &ae::BodyKinematics, &BossEncounter)>()
        .iter(world)
        .find(|(config, ..)| config.behavior.id == "wolfenstein_boss")
        .expect("Wolfenstein is in the throne room");
    (body.pos, encounter.encounter_phase())
}

fn approach(app: &mut App) {
    let (position, _) = wolf(app);
    let world = app.world_mut();
    let mut query = world.query_filtered::<(
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
        &mut BodyHealth,
    ), PrimaryPlayerOnly>();
    let (mut clusters, mut model, mut health) = query.single_mut(world).unwrap();
    health
        .health
        .invulnerable
        .set(Invulnerability::SCRIPTED, true);
    ae::movement::transit_body(
        &mut model,
        &mut clusters.as_clusters_mut(),
        ae::Vec2::new(position.x - 200.0, position.y),
        ae::movement::TransitVelocity::Zero,
    );
}

fn withhold_entrance(app: &mut App) -> Vec<(Handle<KiraAudioSource>, KiraAudioSource)> {
    let cue = session_world_component::<PreparedMusicCatalog>(app.world())
        .unwrap()
        .cues
        .as_ref()
        .unwrap()
        .cue("galvanic_march")
        .unwrap();
    let server = app.world().resource::<AssetServer>();
    let handles: Vec<_> = cue
        .section("awaken")
        .unwrap()
        .sources
        .iter()
        .map(|source| {
            server
                .get_handle::<KiraAudioSource>(source.path.clone())
                .unwrap()
        })
        .collect();
    let mut decoded = app.world_mut().resource_mut::<Assets<KiraAudioSource>>();
    handles
        .into_iter()
        .map(|handle| {
            let source = decoded
                .remove(handle.id())
                .expect("the prepared entrance was decoded");
            (handle, source)
        })
        .collect()
}

fn die_and_restore(app: &mut App, previous: &mut Instant) {
    use ambition_platformer2d::combat::death_rules::{ActorDiedMessage, DeathCause};
    use ambition_platformer2d::session::checkpoint::{
        CheckpointRestoreOutcome, SessionCheckpointOutcomes,
    };
    let before = session_world_component::<SessionCheckpointOutcomes>(app.world())
        .unwrap()
        .latest()
        .cloned();
    let victim = crate::neighbor_prefetch_prepares_rooms::alice(app);
    let pos = app.world().get::<ae::BodyKinematics>(victim).unwrap().pos;
    app.world_mut().write_message(ActorDiedMessage {
        victim,
        pos,
        cause: DeathCause {
            source: ambition_platformer2d::combat::HitSource::Hazard,
            attacker: None,
        },
    });
    wait(app, previous, "death restore committed", |app| {
        let now = session_world_component::<SessionCheckpointOutcomes>(app.world())
            .unwrap()
            .latest();
        now != before.as_ref() && matches!(now, Some(CheckpointRestoreOutcome::Committed { .. }))
    });
}

fn cross(app: &mut App, previous: &mut Instant, target: &str) {
    use ambition_platformer2d::characters::control::PlayerSlot;
    use ambition_platformer2d::platformer::lifecycle::LiveBodyId;
    use ambition_platformer2d::session::lifecycle_commit::{
        BodyTransferRequirement, LifecycleIntent, PendingLifecycleCommit, RoomArrival,
        RoomBodyTransfer, RoomTransitionIntent,
    };
    let body = crate::neighbor_prefetch_prepares_rooms::alice(app);
    let subject = LiveBodyId::of_entity(app.world(), body).unwrap();
    let intent = LifecycleIntent::Transition(RoomTransitionIntent {
        disposition: Default::default(),
        source_zone: None,
        initiator: RoomBodyTransfer {
            requirement: BodyTransferRequirement::InPlay,
            subject,
            arrival: RoomArrival::At(ae::Vec2::new(200.0, 200.0)),
            participant: Some(PlayerSlot(0)),
        },
        companions: Vec::new(),
        target_room: target.into(),
        edge_exit: false,
        zone_sfx: None,
    });
    assert!(app
        .world_mut()
        .resource_mut::<PendingLifecycleCommit>()
        .record(0, intent)
        .admitted());
    wait(app, previous, target, |app| {
        crate::neighbor_prefetch_prepares_rooms::room_of(app, body).as_deref() == Some(target)
    });
}

#[test]
#[ignore = "uses native audio and generated assets; run with a capture device"]
fn wolfensteins_prepared_entrance_and_room_exits_reach_native_output() {
    let mut app = build_visible_app_with(VisibleRenderMode::NoWindow, false, |app| {
        app.insert_resource(AudioOutputMode::Device);
        app.insert_resource(ambition_app::app::StartRoomOverride(THRONE.into()));
        app.insert_resource(ambition_app::app::StartRoomMustResolve);
    });
    assert!(
        app.world()
            .resource::<AudioBackendState>()
            .device_backend_installed
    );
    let mut previous = Instant::now();
    wait(&mut app, &mut previous, "dormant entrance decoded", |app| {
        entrance_decoded(app)
    });
    assert_eq!(wolf(&mut app).1, BossEncounterPhase::Dormant);
    approach(&mut app);
    wait(&mut app, &mut previous, "awaken", |app| {
        running(app, "galvanic_march", "awaken")
    });
    wait(&mut app, &mut previous, "march", |app| {
        running(app, "galvanic_march", "march")
    });
    cross(&mut app, &mut previous, HALL);
    wait(&mut app, &mut previous, "hall song", |app| {
        running(app, "tritone_decree", "song")
    });
    cross(&mut app, &mut previous, THRONE);
    assert_eq!(wolf(&mut app).1, BossEncounterPhase::Dormant);
    let withheld = withhold_entrance(&mut app);
    approach(&mut app);
    wait(
        &mut app,
        &mut previous,
        "cold entrance skipped for current march",
        |app| {
            assert!(!running(app, "galvanic_march", "awaken"));
            running(app, "galvanic_march", "march")
        },
    );
    assert_eq!(wolf(&mut app).1, BossEncounterPhase::Phase1);
    for (handle, source) in withheld {
        app.world_mut()
            .resource_mut::<Assets<KiraAudioSource>>()
            .insert(handle.id(), source)
            .unwrap();
    }
    for _ in 0..10 {
        tick(&mut app, &mut previous);
        assert!(running(&app, "galvanic_march", "march"));
    }
    die_and_restore(&mut app, &mut previous);
    approach(&mut app);
    wait(&mut app, &mut previous, "respawn awaken", |app| {
        running(app, "galvanic_march", "awaken")
    });
    wait(&mut app, &mut previous, "respawn march", |app| {
        running(app, "galvanic_march", "march")
    });
    cross(&mut app, &mut previous, HALL);
    wait(&mut app, &mut previous, "second hall song", |app| {
        running(app, "tritone_decree", "song")
    });
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) {
        tick(&mut app, &mut previous);
        assert!(running(&app, "tritone_decree", "song"));
    }
}
