// Portal integration test: only built with the portal mechanic + RL stepping
// API. Compiled out (empty test binary) when `portal` is disabled.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! Station A is a linked ground↔ground authored pair at x≈300 / x≈600 on the floor.
//! Walking the player right across the entry portal must produce a TELEPORT —
//! a single-frame position jump far larger than walking speed — which only the
//! portal transfer can do. Driven through the public Platformer2dSimHarness API, asserting
//! only on observed player position.

use crate::common::{base, fixed_60hz_room_sim};

use ambition_app::AgentAction;

#[test]
fn a_body_can_return_through_the_authored_slope_portal() {
    use ambition_app::AmbitionSim;
    use ambition_platformer2d::engine_core::{BodyKinematics, SweepSample};
    use ambition_platformer2d::actors::body::BodyFrameOutput;
    use ambition_platformer2d::platformer::markers::PrimaryPlayer;
    use bevy::prelude::*;

    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("portal_lab").with_sync_test_rollback_settings(4, 10),
    ).unwrap();
    for _ in 0..12 { sim.step(base()); }
    let resets = sim.observation().resets;
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, With<PrimaryPlayer>>().single(world).unwrap();
    let mut pose = *world.get::<BodyKinematics>(player).unwrap();
    pose.pos = Vec2::new(3444.092, 921.99994);
    pose.vel = Vec2::ZERO;
    world.entity_mut(player).insert((pose, SweepSample::at_rest(pose, Vec2::Y)));
    sim.rebase_rollback_history().unwrap();
    let mut crossed = false;
    for tick in 0..24 {
        sim.step(AgentAction { move_x: 1.0, ..base() });
        let pose = sim.world().get::<BodyKinematics>(player).unwrap();
        let out = sim.world().get::<BodyFrameOutput>(player).unwrap();
        crossed |= pose.pos.x < 3200.0;
        assert!(out.events.motion_refusal.is_none(), "reverse tick {tick}: {:?}", out.events.motion_refusal);
    }
    assert!(crossed, "the body must return from the wall to the slope");
    let end = sim.world().get::<BodyKinematics>(player).unwrap();
    assert!(end.pos.distance(Vec2::new(2968.5095, 880.05524)) > end.half_extent().length(),
        "the body must move clear of the reverse stuck pose: {end:?}");
    assert_eq!(sim.observation().resets, resets, "the body must leave without a reset");
}

#[test]
fn a_body_can_leave_the_authored_slope_portal() {
    use ambition_app::AmbitionSim;
    use ambition_platformer2d::engine_core::{BodyKinematics, SweepSample};
    use ambition_platformer2d::actors::body::BodyFrameOutput;
    use ambition_platformer2d::platformer::markers::PrimaryPlayer;
    use bevy::prelude::*;

    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("portal_lab").with_sync_test_rollback_settings(4, 10),
    ).unwrap();
    for _ in 0..12 { sim.step(base()); }
    let resets = sim.observation().resets;
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, With<PrimaryPlayer>>().single(world).unwrap();
    let mut pose = *world.get::<BodyKinematics>(player).unwrap();
    pose.pos = Vec2::new(2944.9165, 861.8095);
    pose.vel = Vec2::new(0.0, 707.55005);
    world.entity_mut(player).insert((pose, SweepSample::at_rest(pose, Vec2::Y)));
    sim.rebase_rollback_history().unwrap();
    let mut crossed = false;
    for tick in 0..12 {
        sim.step(base());
        let world = sim.world();
        let pose = world.get::<BodyKinematics>(player).unwrap();
        let out = world.get::<BodyFrameOutput>(player).unwrap();
        crossed |= pose.pos.x > 3400.0;
        assert!(out.events.motion_refusal.is_none(), "slope tick {tick}: {:?}", out.events.motion_refusal);
    }
    assert!(crossed, "the body must cross the slope-to-wall pair");
    let end = sim.world().get::<BodyKinematics>(player).unwrap();
    assert!(end.pos.distance(Vec2::new(3450.9165, 891.1381)) > end.half_extent().length(),
        "the body must move clear of the recorded stuck pose: {end:?}");
    assert_eq!(sim.observation().resets, resets, "the body must leave without a reset");
}

#[test]
fn portal_lab_authored_portals_are_enterable_without_the_gun() {
    let mut sim = fixed_60hz_room_sim("portal_lab");

    // Spawns on the left floor (PlayerStart x≈92), left of station A (x≈300).
    let spawn = sim.observation().player_pos;
    assert!(spawn.0 < 200.0, "spawns on the left, got x={}", spawn.0);

    // Walk right across the first floor portal. Track the biggest single-frame
    // position jump: a normal walk step is only a few px at 60 Hz, so any jump of
    // >150px is a portal transfer (station A jumps ~300px).
    let mut prev = spawn;
    let mut max_jump = 0.0_f32;
    let mut resets = sim.observation().resets;
    for _ in 0..240 {
        let obs = sim.step(AgentAction {
            move_x: 1.0,
            ..base()
        });
        let cur = obs.player_pos;
        let jump = ((cur.0 - prev.0).powi(2) + (cur.1 - prev.1).powi(2)).sqrt();
        max_jump = max_jump.max(jump);
        prev = cur;
        resets = obs.resets;
        if max_jump > 150.0 {
            break;
        }
    }

    assert!(
        max_jump > 150.0,
        "walking onto the authored floor portal should teleport the player \
         (biggest single-frame move was {max_jump:.1}px) — the static portals are \
         not transiting; is the gun-less transit / carve path wired?"
    );
    assert_eq!(
        resets, 0,
        "transited the portal without dying (resets={resets})"
    );
}
