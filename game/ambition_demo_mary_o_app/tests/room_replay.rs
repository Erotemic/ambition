//! **A replay request actually replays the room** — in the standalone binary.
//!
//! This is the proof that was missing (tracks §2.5). `RoomReplayRequested` is
//! the engine's generic "replay the active room" request, and Mary-O emits it
//! from two beats: the flag tally cycling the level, and the clock running out.
//! Until 2026-07-21 its only consumer was registered by `ambition_app`, and
//! this crate depends on `ambition`, never on `ambition_app` — so in the
//! shipped standalone binary the message went into a registered channel that
//! nothing drained.
//!
//! Nothing caught it because every existing proof observed the EMIT rather
//! than the effect: the content-crate unit test asserts the clock refill that
//! `cycle_level_on_flag_tally` writes one line before the message, and the
//! scripted run returns the instant it sees that same refill. Both are green
//! whether or not a consumer exists anywhere. The assertions below are about
//! the BODY, which only moves if something drained the request.

use ambition::engine_core as ae;
use ambition::platformer::markers::PrimaryPlayer;
use ambition_demo_mary_o_app::build_demo_app;
use bevy::prelude::*;

/// Counts every `ResetRoomFeaturesEvent` observed, so a second consumer of the
/// same request would show up as a second room-feature reset.
#[derive(Resource, Default)]
struct RoomResetsSeen(usize);

fn count_room_resets(
    mut seen: ResMut<RoomResetsSeen>,
    mut resets: MessageReader<ambition::combat::ResetRoomFeaturesEvent>,
) {
    seen.0 += resets.read().count();
}

fn boot() -> App {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    app.init_resource::<RoomResetsSeen>();
    app.add_systems(Last, count_room_resets);
    app
}

fn player_pos(app: &mut App) -> Option<Vec2> {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    query.iter(app.world()).next().map(|k| k.pos)
}

/// The room's authored spawn — where a replay must put her back.
fn room_spawn(app: &mut App) -> Vec2 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::RoomGeometry, With<ambition::platformer::lifecycle::SessionRoot>>();
    query
        .iter(app.world())
        .next()
        .expect("an active session publishes its room geometry")
        .0
        .spawn
}

fn settle_until_playable(app: &mut App) -> Vec2 {
    for _ in 0..600 {
        app.update();
        if let Some(pos) = player_pos(app) {
            return pos;
        }
    }
    panic!("the demo never activated a playable body");
}

/// Move her somewhere a replay would have to undo. `transit_body` is the engine
/// authority for discretely relocating a body (ADR 0024) — poking
/// `BodyKinematics.pos` would leave the motion model's private attachment state
/// describing the old position.
fn displace(app: &mut App, to: Vec2) {
    let mut query = app.world_mut().query_filtered::<(
        ae::BodyClusterQueryData,
        &mut ambition::actors::features::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut cluster_item, mut motion_model) = query
        .iter_mut(world)
        .next()
        .expect("gameplay has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ae::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        to,
        ae::movement::TransitVelocity::Zero,
    );
}

/// **The seam itself.** One request in, and the body comes home.
///
/// Deliberately emits the message directly rather than playing to a beat that
/// emits it: this is the HOST-side half of the contract, and it should hold in
/// this binary regardless of which content beat asks for the replay.
#[test]
fn a_replay_request_returns_the_body_to_spawn() {
    let mut app = boot();
    settle_until_playable(&mut app);
    let spawn = room_spawn(&mut app);

    let away = spawn + Vec2::new(600.0, 0.0);
    displace(&mut app, away);
    app.update();
    let displaced = player_pos(&mut app).expect("she is still in the world");
    assert!(
        displaced.distance(spawn) > 200.0,
        "the fixture must actually move her off spawn before a replay can be \
         shown to bring her back (she is at {displaced:?}, spawn is {spawn:?})"
    );

    app.world_mut()
        .write_message(ambition::actors::session::reset::RoomReplayRequested);
    app.update();

    let home = player_pos(&mut app).expect("she is still in the world");
    assert!(
        home.distance(spawn) < 64.0,
        "a replay request must return her to the room spawn; she is at {home:?} \
         and spawn is {spawn:?}. Before 2026-07-21 this failed by the full \
         displacement, because the standalone binary registered no consumer at \
         all — the request was written into a channel nothing drained."
    );
}

/// **One request, one replay.** The consumer moved crates; it must not now be
/// registered twice (once by the engine group, once by a host that kept its
/// own copy).
///
/// A duplicate consumer is invisible in the body position — the second reset is
/// idempotent — so this counts the room-feature reset each one requests. In a
/// quiet frame nothing else writes `ResetRoomFeaturesEvent`, which is what
/// makes the count readable.
#[test]
fn one_replay_request_is_processed_exactly_once() {
    let mut app = boot();
    settle_until_playable(&mut app);

    // Establish that the window is quiet: no room-feature resets without a
    // request, so the count below is attributable to the replay alone.
    app.world_mut().resource_mut::<RoomResetsSeen>().0 = 0;
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<RoomResetsSeen>().0,
        0,
        "an idle frame must not reset room features, or the count below proves nothing"
    );

    app.world_mut()
        .write_message(ambition::actors::session::reset::RoomReplayRequested);
    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        app.world().resource::<RoomResetsSeen>().0,
        1,
        "exactly one consumer may drain a replay request — 0 means the host \
         carries none, 2 means the engine group and the host both registered one"
    );
}

/// **The clock running out replays the room.** Mary-O's timeout beat, end to
/// end: `spend_lives_on_death` spends the life and asks for a replay, and the
/// room actually rebuilds under her.
///
/// The clock is drained rather than waited out — `STARTING_TIME` is 400 in-game
/// seconds and this is a seam proof, not an endurance run. The beat under test
/// is what the demo does when it reaches zero.
#[test]
fn the_level_timeout_actually_replays_the_room() {
    let mut app = boot();
    settle_until_playable(&mut app);
    let spawn = room_spawn(&mut app);

    displace(&mut app, spawn + Vec2::new(600.0, 0.0));
    app.update();

    let lives_before = {
        let mut query = app
            .world_mut()
            .query::<&ambition_demo_mary_o::MaryOLevelState>();
        query
            .iter(app.world())
            .next()
            .expect("the level owner exists once the demo is playable")
            .lives
    };

    // Run the clock out.
    {
        let mut query = app
            .world_mut()
            .query::<&mut ambition_demo_mary_o::MaryOLevelState>();
        let world = app.world_mut();
        let mut level = query.iter_mut(world).next().expect("the level owner");
        level.time_remaining = 0.0;
    }
    for _ in 0..4 {
        app.update();
    }

    let (lives_after, remaining) = {
        let mut query = app
            .world_mut()
            .query::<&ambition_demo_mary_o::MaryOLevelState>();
        let level = query.iter(app.world()).next().expect("the level owner");
        (level.lives, level.time_remaining)
    };
    assert_eq!(
        lives_after,
        lives_before - 1,
        "running the clock out costs one life"
    );
    // Refilled to `STARTING_TIME`, then ticking down again — the fresh attempt
    // is already running, so this is a proximity check, not equality.
    assert!(
        (ambition_demo_mary_o::STARTING_TIME - remaining) < 1.0 && remaining > 0.0,
        "and rearms the clock for the fresh attempt (got {remaining}, expected \
         just under {})",
        ambition_demo_mary_o::STARTING_TIME
    );

    // The part no existing test reached: the fresh attempt actually starts at
    // spawn. Every previous proof stopped at the clock refill above, which is
    // written one line before the replay request.
    let home = player_pos(&mut app).expect("she is still in the world");
    assert!(
        home.distance(spawn) < 64.0,
        "a timeout must put the fresh attempt at spawn; she is at {home:?} and \
         spawn is {spawn:?}"
    );
}
