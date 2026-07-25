//! The death beat, driven through the real systems on a bare app.

use super::*;
use ambition::engine_core as ae;

const DT: f32 = 1.0 / 60.0;

fn app() -> App {
    let mut app = App::new();
    app.add_message::<ambition::actors::ActorDiedMessage>();
    app.add_message::<ambition::actors::session::reset::RoomReplayRequested>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    app.insert_resource(ambition::time::WorldTime {
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
            ambition::characters::brain::ActorControl::default(),
            ambition::actors::actor::BodyAnimFacts::default(),
        ))
        .id();
    app.insert_resource(ambition::platformer::markers::ControlledSubject(Some(body)));
    body
}

fn kill(app: &mut App, at: ae::Vec2) {
    app.world_mut()
        .write_message(ambition::actors::ActorDiedMessage {
            pos: at,
            cause: ambition::actors::DeathCause {
                source: ambition::actors::combat::HitSource::EnemyBody,
                attacker: None,
            },
        });
}

fn replays(app: &mut App) -> usize {
    app.world()
        .resource::<bevy::ecs::message::Messages<
            ambition::actors::session::reset::RoomReplayRequested,
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
            .get::<ambition::actors::actor::BodyAnimFacts>(body)
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
    assert!(!app
        .world_mut()
        .query::<&MaryODeathSequence>()
        .single(app.world())
        .unwrap()
        .active());
}
