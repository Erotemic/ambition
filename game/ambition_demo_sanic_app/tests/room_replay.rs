//! **A replay request actually replays the act** — in the standalone binary.
//!
//! The Sanic half of tracks §2.5. `cycle_act_after_clear` restarts the act by
//! emitting the engine's generic `RoomReplayRequested`, and until 2026-07-21
//! the only consumer of that message was registered by `ambition_app` — which
//! this crate does not depend on. So the shipped standalone binary asked for a
//! restart that never came: Sanic was not returned to spawn, the rings were not
//! restored, the badniks did not respawn.
//!
//! The existing completion proof (`act_completion.rs`) could not catch it: it
//! stops 30 frames into a 4-second `ACT_CLEAR_DWELL`, so `cycle_act_after_clear`
//! never even reaches the line that writes the message. Extending it in place
//! was tried and abandoned — see `the_act_clear_restarts_the_act_after_the_full_dwell`
//! below for why the post-goal death makes that run unable to isolate a replay.

use ambition::engine_core as ae;
use ambition::platformer::markers::PrimaryPlayer;
use ambition_demo_sanic_app::build_demo_app;
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

/// Relocate through the engine's discrete-transit authority (ADR 0024) rather
/// than poking `BodyKinematics.pos`. Sanic rides the momentum kernel, so his
/// motion model carries surface/attachment state that a raw position write
/// would leave describing the old place.
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
#[test]
fn a_replay_request_returns_the_body_to_spawn() {
    let mut app = boot();
    settle_until_playable(&mut app);
    let spawn = room_spawn(&mut app);

    displace(&mut app, spawn + Vec2::new(900.0, 0.0));
    app.update();
    let displaced = player_pos(&mut app).expect("he is still in the world");
    assert!(
        displaced.distance(spawn) > 300.0,
        "the fixture must actually move him off spawn before a replay can be \
         shown to bring him back (he is at {displaced:?}, spawn is {spawn:?})"
    );

    app.world_mut()
        .write_message(ambition::actors::session::reset::RoomReplayRequested);
    app.update();

    let home = player_pos(&mut app).expect("he is still in the world");
    assert!(
        home.distance(spawn) < 64.0,
        "a replay request must return him to the act's spawn; he is at {home:?} \
         and spawn is {spawn:?}. Before 2026-07-21 this failed by the full \
         displacement, because the standalone binary registered no consumer at \
         all — the request was written into a channel nothing drained."
    );
}

/// **The act clear restarts the act** — through the FULL `ACT_CLEAR_DWELL`.
///
/// `cycle_act_after_clear` counts the results card down and then asks for a
/// replay. The existing completion run (`act_completion.rs`) stops 30 frames
/// into that 4-second dwell, so it never reaches the line that emits, and it
/// cannot be extended to: past the goal the script's held stick carries Sanic
/// off the end of the speedway into a pit death, whose respawn returns him to
/// spawn and rebuilds the room for reasons unrelated to the restart. (That the
/// act is clearable and then immediately fatal is a content matter, logged
/// rather than fixed here.)
///
/// So this drives the beat under controlled conditions: park him off spawn
/// somewhere he will stay, stamp the cleared phase the goal would have stamped,
/// and let the real dwell run out.
#[test]
fn the_act_clear_restarts_the_act_after_the_full_dwell() {
    let mut app = boot();
    settle_until_playable(&mut app);
    let spawn = room_spawn(&mut app);

    displace(&mut app, spawn + Vec2::new(900.0, 0.0));
    app.update();
    let parked = player_pos(&mut app).expect("he is still in the world");
    assert!(
        parked.distance(spawn) > 300.0,
        "he must be parked off spawn for the restart to be observable \
         ({parked:?} vs {spawn:?})"
    );

    {
        let mut query = app
            .world_mut()
            .query::<&mut ambition_demo_sanic::SanicActState>();
        let world = app.world_mut();
        let mut act = query
            .iter_mut(world)
            .next()
            .expect("the act owner exists once the demo is playable");
        act.phase = ambition_demo_sanic::SanicActPhase::Cleared {
            time: 12.0,
            rings: 26,
            dwell: ambition_demo_sanic::ACT_CLEAR_DWELL,
        };
    }

    // Half the dwell: the card is still up and nothing has restarted yet.
    let dwell_frames = (ambition_demo_sanic::ACT_CLEAR_DWELL / (1.0 / 60.0)).ceil() as usize;
    for _ in 0..dwell_frames / 2 {
        app.update();
    }
    let mid = player_pos(&mut app).expect("he is still in the world");
    assert!(
        mid.distance(spawn) > 300.0,
        "the act must not restart before its dwell elapses — the results card \
         is what makes the run legible ({mid:?} vs {spawn:?})"
    );

    // Past it.
    for _ in 0..dwell_frames / 2 + 30 {
        app.update();
    }
    let home = player_pos(&mut app).expect("he is still in the world");
    assert!(
        home.distance(spawn) < 64.0,
        "past the full dwell the act restarts and puts him back at the start \
         line; he is at {home:?} and spawn is {spawn:?}. Before 2026-07-21 he \
         stood exactly where he was: `cycle_act_after_clear` reset its own phase \
         and emitted a request the standalone binary drained with nothing."
    );
}

/// **One request, one replay.** The consumer moved crates; it must not now be
/// registered twice (once by the engine group, once by a host that kept its own
/// copy). A duplicate is invisible in the body position — the second reset is
/// idempotent — so this counts the room-feature reset each one requests.
#[test]
fn one_replay_request_is_processed_exactly_once() {
    let mut app = boot();
    settle_until_playable(&mut app);

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
