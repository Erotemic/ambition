//! The death beat, driven through the real systems on a bare app.

use super::*;
use ambition_platformer2d::engine_core as ae;
use bevy::prelude::IntoScheduleConfigs;

const DT: f32 = 1.0 / 60.0;

fn app() -> App {
    let mut app = App::new();
    app.add_message::<ambition_platformer2d::actors::ActorDiedMessage>();
    app.add_message::<ambition_platformer2d::actors::session::reset::RoomReplayRequested>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: DT,
        ..Default::default()
    });
    app.add_systems(
        bevy::prelude::Update,
        (
            begin_death_sequence,
            run_death_sequence,
            restart_level_after_death,
        )
            .chain(),
    );
    app
}

fn spawn_owner_and_body(app: &mut App, at: ae::Vec2) -> bevy::prelude::Entity {
    app.world_mut().spawn(MaryODeathSequence::default());
    let body = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: at,
                ..Default::default()
            },
            ambition_platformer2d::characters::brain::ActorControl::default(),
            ambition_platformer2d::actors::actor::BodyAnimFacts::default(),
        ))
        .id();
    app.insert_resource(ambition_platformer2d::platformer::markers::ControlledSubject(Some(body)));
    body
}

fn kill(app: &mut App, at: ae::Vec2) {
    app.world_mut()
        .write_message(ambition_platformer2d::actors::ActorDiedMessage {
            pos: at,
            cause: ambition_platformer2d::actors::DeathCause {
                source: ambition_platformer2d::actors::combat::HitSource::EnemyBody,
                attacker: None,
            },
        });
}

fn replays(app: &mut App) -> usize {
    app.world()
        .resource::<bevy::ecs::message::Messages<
            ambition_platformer2d::actors::session::reset::RoomReplayRequested,
        >>()
        .iter_current_update_messages()
        .count()
}

/// The whole beat: she dies where she died, holds the death pose for the dwell,
/// and only THEN does the level restart.
#[test]
fn she_dies_in_place_holds_the_pose_and_then_the_level_restarts() {
    let mut app = app();
    let died_at = ae::Vec2::new(640.0, 300.0);
    let body = spawn_owner_and_body(&mut app, died_at);

    kill(&mut app, died_at);
    app.update();

    // The engine respawns her instantly — the beat is what puts her back at the
    // place the player last saw her.
    app.world_mut()
        .get_mut::<ae::BodyKinematics>(body)
        .unwrap()
        .pos = ae::Vec2::new(64.0, 64.0);

    let mut frames = 0;
    while frames < 600 {
        app.update();
        frames += 1;
        let sequence = *app
            .world_mut()
            .query::<&MaryODeathSequence>()
            .single(app.world())
            .unwrap();
        if !sequence.active() {
            break;
        }
        let kin = app
            .world()
            .get::<ae::BodyKinematics>(body)
            .copied()
            .unwrap();
        assert_eq!(kin.pos, died_at, "she is held where she died, not at spawn");
        assert_eq!(kin.vel, ae::Vec2::ZERO, "and she does not slide or fall");
        let death_timer = app
            .world()
            .get::<ambition_platformer2d::actors::actor::BodyAnimFacts>(body)
            .map(|anim| anim.death_anim_timer)
            .unwrap();
        assert!(
            death_timer > 0.0,
            "the death row plays for the whole beat — re-armed every tick, \
             because the engine's respawn resets these facts"
        );
        assert_eq!(
            replays(&mut app),
            0,
            "the level must NOT restart while the beat is still playing"
        );
    }

    assert!(
        (frames as f32 * DT - DEATH_DWELL).abs() < 4.0 * DT,
        "the beat runs for its authored dwell; took {frames} frames"
    );
    assert_eq!(
        replays(&mut app),
        1,
        "and exactly one replay is requested when it ends"
    );
}

/// A second death landing DURING the beat does not restart it. Otherwise a body
/// still overlapping whatever killed it would extend its own death forever.
#[test]
fn the_death_beat_makes_her_untouchable_and_gives_her_back() {
    // Jon, from play: "when maryo is in her death animation, she still gets hit by
    // enemies." The beat owned her controls and her pose and left her hurtbox
    // live, so a snake walking into a body that has already lost still landed.
    let mut app = app();
    let died_at = ae::Vec2::new(64.0, 32.0);
    let body = spawn_owner_and_body(&mut app, died_at);
    app.world_mut()
        .entity_mut(body)
        .insert(ambition_platformer2d::characters::actor::BodyHealth::new(
            ambition_platformer2d::characters::actor::Health::new(1),
        ));
    kill(&mut app, died_at);
    app.update();

    let immune = |app: &mut App| {
        app.world()
            .get::<ambition_platformer2d::characters::actor::BodyHealth>(body)
            .expect("body has health")
            .health
            .invulnerable
            .holds(ambition_platformer2d::characters::actor::Invulnerability::SCRIPTED)
    };
    assert!(immune(&mut app), "the beat holds her untouchable while it plays");

    // ...and RELEASES it. An immunity a scripted beat forgets to drop is worse
    // than the bug it fixed: she would walk the replay invincible.
    for _ in 0..600 {
        app.update();
    }
    assert!(
        !immune(&mut app),
        "the beat gives the body back when the dwell runs out"
    );
}

#[test]
fn a_second_death_during_the_beat_does_not_extend_it() {
    let mut app = app();
    let died_at = ae::Vec2::new(100.0, 100.0);
    spawn_owner_and_body(&mut app, died_at);

    kill(&mut app, died_at);
    app.update();
    let after_first = app
        .world_mut()
        .query::<&MaryODeathSequence>()
        .single(app.world())
        .unwrap()
        .remaining;

    for _ in 0..10 {
        kill(&mut app, ae::Vec2::new(999.0, 999.0));
        app.update();
    }
    let sequence = *app
        .world_mut()
        .query::<&MaryODeathSequence>()
        .single(app.world())
        .unwrap();
    assert!(
        sequence.remaining < after_first,
        "the beat keeps counting down through further deaths"
    );
    assert_eq!(
        sequence.at,
        Some(died_at),
        "and keeps holding the FIRST death's place"
    );
}

/// No death, no beat — and no restart. The level is only interrupted by an
/// actual death.
#[test]
fn a_quiet_level_never_restarts_itself() {
    let mut app = app();
    spawn_owner_and_body(&mut app, ae::Vec2::ZERO);
    for _ in 0..200 {
        app.update();
        assert_eq!(replays(&mut app), 0);
    }
    assert!(
        !app.world_mut()
            .query::<&MaryODeathSequence>()
            .single(app.world())
            .unwrap()
            .active()
    );
}

/// **The death track is DECLARED, not just requested.**
///
/// Under provider-relative playback a session plays only the tracks its own
/// audio fragment names, so asking for an undeclared id is gated to silence —
/// the request succeeds, the music does not. That is the exact shape of every
/// other thing in this demo that "worked" while producing nothing, so pin the
/// declaration rather than trusting it.
#[test]
fn the_death_track_is_authorized_by_the_provider_fragment() {
    let mut app = App::new();
    app.add_plugins(crate::provider::MaryOExperiencePlugin);
    let registry = app
        .world()
        .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>()
        .combined_music_registry(crate::provider::MARY_O_EXPERIENCE)
        .expect("Mary-O's audio fragment assembles");
    assert!(
        registry
            .tracks
            .iter()
            .any(|track| track.id == crate::provider::MARY_O_DEATH_MUSIC_TRACK),
        "her death track must be declared by the fragment that authorizes it; \
         got {:?}",
        registry.tracks.iter().map(|t| &t.id).collect::<Vec<_>>()
    );
}

/// It plays for the beat and hands the level's own theme back afterwards.
#[test]
fn the_death_music_claims_the_priority_tier_and_releases_it() {
    let mut app = app();
    let died_at = ae::Vec2::new(300.0, 300.0);
    spawn_owner_and_body(&mut app, died_at);
    let root = app
        .world_mut()
        .spawn((
            ambition_platformer2d::platformer::lifecycle::SessionRoot(session_scope_for_test()),
            ambition_platformer2d::actors::encounter::EncounterMusicRequest::default(),
        ))
        .id();
    // Same order as the shipped chain: the beat is armed before the music that
    // plays over it is chosen, or the claim lands a frame late.
    app.add_systems(
        bevy::prelude::Update,
        play_death_music.after(restart_level_after_death),
    );

    app.update();
    assert_eq!(
        requested(&app, root),
        None,
        "a level nobody has died on plays its own theme"
    );

    kill(&mut app, died_at);
    app.update();
    assert_eq!(
        requested(&app, root).as_deref(),
        Some(crate::provider::MARY_O_DEATH_MUSIC_TRACK),
        "her death takes the priority tier — the one slot that outranks the room"
    );

    for _ in 0..((DEATH_DWELL / DT).ceil() as usize + 10) {
        app.update();
    }
    assert_eq!(
        requested(&app, root),
        None,
        "and gives it straight back, so the level theme returns on its own"
    );
}

/// A session scope id for a bare test root.
fn session_scope_for_test() -> ambition_platformer2d::platformer::lifecycle::SessionScopeId {
    let mut scope = ambition_platformer2d::platformer::lifecycle::ActiveSessionScope::default();
    scope.begin()
}

fn requested(app: &App, root: bevy::prelude::Entity) -> Option<String> {
    app.world()
        .get::<ambition_platformer2d::actors::encounter::EncounterMusicRequest>(root)
        .and_then(|music| music.priority_track.clone())
}
