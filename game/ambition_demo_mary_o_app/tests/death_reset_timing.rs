//! **When does the world reset, relative to the death beat?**
//!
//! Jon, from play: *"When maryo-dies the enemies seem to reset before the death
//! animation is finished. The level reset needs to happen all at once at a time
//! that is easy to express in the game code."*
//!
//! Reading the chain says that should not happen: `ResetRoomFeaturesEvent` has
//! one production writer, reached only by draining `RoomReplayRequested`, and
//! Mary-O emits that from `restart_level_after_death`, which returns early while
//! `sequence.active()`. So the reset is already gated behind the beat.
//!
//! ⚠ **which makes this a REPRODUCTION question, not a design one**, and a
//! reproduction question is settled by stepping the sim rather than by reading
//! more of it. This records, per frame, the beat's `remaining` against the frame
//! the world actually resets, and asserts the ordering the code claims. A red
//! here is the real row; a green says the symptom is something else wearing the
//! same clothes — most likely the PLAYER resetting mid-animation, which
//! `death_respawn_player` does do on the fatal hit and which the death module's
//! own doc calls out.
//!
//! ⛔ **it kills her by dropping her in a pit, not by writing
//! `ActorDiedMessage`.** Writing the message would prove the beat gates the
//! reset when the beat is armed by hand — which is the one case nobody doubted.
//! The pit is the path a player takes.

use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

/// The frames on which the world was told to put its features back.
#[derive(Resource, Default)]
struct ResetFrames(Vec<usize>);

#[derive(Resource, Default)]
struct FrameCounter(usize);

fn record_resets(
    frame: Res<FrameCounter>,
    mut seen: ResMut<ResetFrames>,
    mut resets: MessageReader<ambition_platformer2d::combat::ResetRoomFeaturesEvent>,
) {
    for _ in resets.read() {
        seen.0.push(frame.0);
    }
}

fn advance_frame(mut frame: ResMut<FrameCounter>) {
    frame.0 += 1;
}

fn boot() -> App {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    app.init_resource::<ResetFrames>();
    app.init_resource::<FrameCounter>();
    app.add_systems(Last, (record_resets, advance_frame).chain());
    app
}

fn settle_until_playable(app: &mut App) {
    for _ in 0..600 {
        app.update();
        let mut query = app
            .world_mut()
            .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
        if query.iter(app.world()).next().is_some() {
            return;
        }
    }
    panic!("the demo never activated a playable body");
}

/// Relocate through the engine's authority (ADR 0024) rather than poking `pos`,
/// which would leave the motion model's attachment state describing the old
/// position — the same reason `room_replay.rs` does it this way.
fn displace(app: &mut App, to: Vec2) {
    let mut query = app.world_mut().query_filtered::<(
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d::actors::features::MotionModel,
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

fn beat_remaining(app: &mut App) -> Option<f32> {
    let mut query = app
        .world_mut()
        .query::<&ambition_demo_mary_o::death::MaryODeathSequence>();
    query.iter(app.world()).next().map(|s| s.remaining)
}

/// A cheap signature of every non-player body's placement, so "the enemies moved
/// back" is observable without naming any particular enemy.
fn enemy_signature(app: &mut App) -> (usize, i64) {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, Without<PrimaryPlayer>>();
    let mut count = 0usize;
    let mut sum = 0i64;
    for kin in query.iter(app.world()) {
        count += 1;
        sum += kin.pos.x.round() as i64 + kin.pos.y.round() as i64;
    }
    (count, sum)
}

/// **The world does not come back before the beat is over.**
#[test]
fn the_room_resets_no_earlier_than_the_death_beat_ends() {
    let mut app = boot();
    settle_until_playable(&mut app);

    // Drop her far below the room. The pit rule is what kills her, so the death
    // arrives by the door a player uses.
    displace(&mut app, Vec2::new(200.0, 4000.0));

    let mut beat_started: Option<usize> = None;
    let mut beat_ended: Option<usize> = None;
    let mut enemy_moved: Option<usize> = None;
    let mut previous_signature: Option<(usize, i64)> = None;
    let mut log: Vec<String> = Vec::new();

    for _ in 0..600 {
        app.update();
        let frame = app.world().resource::<FrameCounter>().0;
        let remaining = beat_remaining(&mut app);
        let signature = enemy_signature(&mut app);

        if let Some(r) = remaining {
            if r > 0.0 && beat_started.is_none() {
                beat_started = Some(frame);
            }
            if r <= 0.0 && beat_started.is_some() && beat_ended.is_none() {
                beat_ended = Some(frame);
            }
        }
        // Only interesting once the beat is running: before that she is walking
        // around a live room and everything in it is legitimately moving.
        if beat_started.is_some() && enemy_moved.is_none() {
            if let Some(previous) = previous_signature {
                if previous != signature {
                    enemy_moved = Some(frame);
                }
            }
        }
        if beat_started.is_some() && log.len() < 200 {
            log.push(format!(
                "f{frame}: remaining={:?} enemies={:?}",
                remaining, signature
            ));
        }
        previous_signature = Some(signature);
        if beat_ended.is_some() && !app.world().resource::<ResetFrames>().0.is_empty() {
            break;
        }
    }

    let resets = app.world().resource::<ResetFrames>().0.clone();
    let started = beat_started.expect(
        "she never died in a pit 4000 units below the room — the fixture is not \
         reproducing anything, so a green result here would be vacuous",
    );
    let ended = beat_ended.unwrap_or_else(|| {
        panic!("the death beat never ran down within 600 frames (started f{started})")
    });
    let first_reset = *resets.first().unwrap_or_else(|| {
        panic!(
            "the beat ran from f{started} to f{ended} and the room was never \
             reset — the replay the beat owes was never delivered"
        )
    });

    assert!(
        first_reset >= ended,
        "THE WORLD CAME BACK BEFORE THE DEATH BEAT ENDED — reset on f{first_reset}, \
         beat ran f{started}..f{ended}. This is Jon's report reproduced, and it \
         means the reset is NOT gated behind the beat the way the chain reads.\n{}",
        log.join("\n")
    );
    // ⭐ **WHAT THIS MEASURED, 2026-08-03, and it is not what the report said.**
    // The reset is correctly gated — but the world never HOLDS STILL: the
    // non-player bodies move on essentially every frame of the ~0.55 s dwell
    // (signature drifting 37188 → 37131 across f163..f195) and then snap on the
    // single frame after it ends. To a player that reads exactly as "the enemies
    // reset before the death animation finished": they were walking the whole
    // time, and then jumped.
    //
    // ⛔ **deliberately NOT asserted.** Whether the world freezes during a death
    // beat is an unmade design decision, and pinning today's answer either way
    // would be a regression test over unpolished behaviour. The measurement is
    // printed and the row carries it; see D2 in the 08-03 spine.
    println!(
        "[death beat] started f{started}, ended f{ended}, room reset f{first_reset}; \
         first world motion during the beat: {enemy_moved:?}"
    );
    for line in &log {
        println!("{line}");
    }
}
