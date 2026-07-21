// Drives the real Ambition app, which needs the RL stepping API.
#![cfg(feature = "rl_sim")]
//! **The hosted half of the room-replay seam** (tracks §2.5).
//!
//! `apply_room_replay_request_system` used to be registered right here, by
//! `ambition_app`. It now rides `PlatformerEnginePlugins`, because content in
//! EVERY host emits `RoomReplayRequested` and the standalone demo binaries —
//! which depend on `ambition` but never on `ambition_app` — were draining it
//! with nothing.
//!
//! Two things have to hold on this side of that move. Ambition must not have
//! LOST the consumer when its own registration was deleted, and it must not
//! now have TWO (the engine group's, plus a leftover local one). The demo-side
//! counterparts live in `ambition_demo_{mary_o,sanic}_app/tests/room_replay.rs`.

use crate::common::{base, fixed_60hz_sim};

use ambition::combat::ResetRoomFeaturesEvent;
use ambition::engine_core as ae;
use ambition::platformer::markers::PrimaryPlayer;
use ambition_app::SandboxSim;
use bevy::prelude::*;

fn player_pos(sim: &mut SandboxSim) -> Vec2 {
    let mut q = sim
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    let world = sim.world();
    q.iter(world)
        .next()
        .expect("the hosted app has a primary player")
        .pos
}

fn room_spawn(sim: &mut SandboxSim) -> Vec2 {
    let mut q = sim
        .world_mut()
        .query_filtered::<&ae::RoomGeometry, With<ambition::platformer::lifecycle::SessionRoot>>();
    let world = sim.world();
    q.iter(world)
        .next()
        .expect("an active session publishes its room geometry")
        .0
        .spawn
}

fn displace(sim: &mut SandboxSim, to: Vec2) {
    let mut q = sim.world_mut().query_filtered::<(
        ae::BodyClusterQueryData,
        &mut ambition::actors::features::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = sim.world_mut();
    let (mut cluster_item, mut motion_model) = q
        .iter_mut(world)
        .next()
        .expect("the hosted app has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ae::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        to,
        ae::movement::TransitVelocity::Zero,
    );
}

/// **Ambition still drains the request** after its own registration was deleted.
#[test]
fn a_replay_request_returns_the_hosted_body_to_spawn() {
    let mut sim = fixed_60hz_sim();
    sim.step(base());

    let spawn = room_spawn(&mut sim);
    displace(&mut sim, spawn + Vec2::new(500.0, 0.0));
    sim.step(base());
    let displaced = player_pos(&mut sim);
    assert!(
        displaced.distance(spawn) > 200.0,
        "the fixture must move the body off spawn before a replay can be shown \
         to bring it back ({displaced:?} vs {spawn:?})"
    );

    sim.world_mut()
        .write_message(ambition::actors::session::reset::RoomReplayRequested);
    sim.step(base());

    let home = player_pos(&mut sim);
    assert!(
        home.distance(spawn) < 64.0,
        "the hosted app must still return the body to spawn on a replay request \
         ({home:?} vs spawn {spawn:?}) — deleting Ambition's own registration \
         must not have taken the behaviour with it"
    );
}

/// **And exactly once.** A leftover local registration beside the engine
/// group's would reset twice; the body cannot show that (the second reset is
/// idempotent), so this counts the room-feature reset each consumer requests.
///
/// In THIS app a duplicate happens to be a hard Bevy panic rather than a
/// double reset, because `apply_player_reset_input_system` carries a `.before`
/// edge to the consumer and that edge cannot resolve against a system
/// registered twice. That is a happy accident of Ambition's schedule, not a
/// property of the seam — the demo apps have no such edge, and there a
/// duplicate is silent. Assert the count so this reads the same everywhere.
#[test]
fn the_hosted_app_drains_a_replay_request_exactly_once() {
    let mut sim = fixed_60hz_sim();
    sim.step(base());

    // A `Messages` buffer retains for two frames, so read the drain-count for
    // ONE step in isolation: step to clear, then measure the next step alone.
    let resets_this_step = |sim: &mut SandboxSim| -> usize {
        sim.world_mut()
            .resource_mut::<Messages<ResetRoomFeaturesEvent>>()
            .drain()
            .count()
    };

    resets_this_step(&mut sim);
    sim.step(base());
    assert_eq!(
        resets_this_step(&mut sim),
        0,
        "an idle step must not reset room features, or the count below proves nothing"
    );

    sim.world_mut()
        .write_message(ambition::actors::session::reset::RoomReplayRequested);
    sim.step(base());

    assert_eq!(
        resets_this_step(&mut sim),
        1,
        "exactly one consumer may drain a replay request — 0 means Ambition lost \
         it when the local registration was deleted, 2 means the engine group's \
         and a leftover local one both ran"
    );
}
