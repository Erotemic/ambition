use crate::geometry::{Aabb, AabbExt};
use crate::world::{BlinkWallTier, BlockKind, World};
use crate::Vec2;

use super::events::{BlinkEvent, FrameEvents};
use super::ops::MovementOp;
use super::tuning::MovementTuning;

/// Complete a blink: teleport to `to`, damp post-blink velocity,
/// clamp downward speed, clear fast-fall / wall-cling / dash state,
/// arm the post-blink grace timer + cooldown, and push the
/// `Blink` / `PrecisionBlink` op + `BlinkEvent`. Mutates kinematics
/// (pos, vel), flight (fast_falling), wall (wall_clinging, wall_climbing),
/// dash (timer), blink (cooldown, aim_offset, hold_*), and pushes
/// blink ops + the BlinkEvent.
pub fn complete_blink_clusters(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    flight: &mut crate::body_clusters::BodyFlightState,
    wall: &mut crate::body_clusters::BodyWallState,
    dash: &mut crate::body_clusters::BodyDashState,
    blink: &mut crate::body_clusters::BodyBlinkState,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    from: Vec2,
    to: Vec2,
    precision: bool,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    kinematics.pos = to;
    // Post-blink cleanup happens in the body's LOCAL frame: damp the side
    // (run) velocity, clamp runaway FALL velocity, damp a rising one. The old
    // world-X/Y form never clamped the true fall axis under sideways gravity,
    // so chained blinks inherited unbounded fall speed (fable review
    // 2026-07-02 §B3).
    let frame = crate::AccelerationFrame::new(tuning.gravity_dir);
    let damping = if precision { 0.35 } else { 0.55 };
    let max_downward = if precision {
        tuning.precision_blink_max_downward_speed
    } else {
        tuning.blink_max_downward_speed
    };
    let mut local_vel = frame.to_local(kinematics.vel);
    local_vel.x *= damping;
    if local_vel.y > max_downward {
        local_vel.y = max_downward;
    } else {
        local_vel.y *= damping;
    }
    kinematics.vel = frame.to_world(local_vel);
    flight.fast_falling = false;
    wall.wall_clinging = false;
    wall.wall_climbing = false;
    dash.timer = 0.0;
    blink.grace_timer = tuning.blink_grace_time;

    blink.cooldown = tuning.blink_cooldown;
    blink.hold_active = false;
    blink.hold_timer = 0.0;
    blink.aiming = false;
    blink.aim_offset = frame.side * (tuning.blink_distance * kinematics.facing);
    let op = if precision {
        MovementOp::PrecisionBlink
    } else {
        MovementOp::Blink
    };
    events.op_clusters(combo_trace, op);
    events.blinks.push(BlinkEvent {
        from,
        to,
        precision,
    });
}

/// Compute the blink destination in the player's aim direction,
/// clamped to a collision-safe stopping point + the
/// `blink_through_soft_walls` ability gate.
///
/// `aim` must be the already-resolved world-space direction (the caller owns
/// the zero-stick fallback — "forward along facing" is `frame.side * facing`,
/// a frame-dependent vector this function deliberately does not guess at). A
/// zero `aim` blinks nowhere.
pub fn blink_destination_clusters(
    world: &World,
    kinematics: &crate::body_clusters::BodyKinematics,
    abilities: &crate::body_clusters::BodyAbilities,
    aim: Vec2,
    max_distance: f32,
) -> Vec2 {
    blink_destination_internal(
        world,
        kinematics.pos,
        kinematics.size,
        &abilities.abilities,
        aim,
        max_distance,
    )
}

/// Blink to a specific aim point, clamped to a collision-safe destination.
pub fn blink_destination_to_point_clusters(
    world: &World,
    kinematics: &crate::body_clusters::BodyKinematics,
    abilities: &crate::body_clusters::BodyAbilities,
    target: Vec2,
) -> Vec2 {
    blink_destination_to_point_internal(
        world,
        kinematics.pos,
        kinematics.size,
        &abilities.abilities,
        target,
    )
}

fn blink_destination_internal(
    world: &World,
    pos: Vec2,
    size: Vec2,
    abilities: &crate::abilities::AbilitySet,
    aim: Vec2,
    max_distance: f32,
) -> Vec2 {
    let direction = aim.normalize_or(Vec2::ZERO);
    blink_destination_to_point_internal(world, pos, size, abilities, pos + direction * max_distance)
}

fn blink_destination_to_point_internal(
    world: &World,
    start: Vec2,
    size: Vec2,
    abilities: &crate::abilities::AbilitySet,
    target: Vec2,
) -> Vec2 {
    let half = size * 0.5;
    let mut target = target;
    target.x = target.x.clamp(half.x, world.size.x - half.x);
    target.y = target.y.clamp(half.y, world.size.y - half.y);
    let delta = target - start;
    let distance = delta.length();
    if distance <= 1.0e-5 {
        return start;
    }

    let start_body = Aabb::new(start, half);
    let max_t = world
        .first_body_sweep(start_body, delta, |block| {
            blink_path_blocker_abilities(abilities, block.kind)
        })
        .map(|hit| hit.time_of_impact)
        .unwrap_or(1.0);
    let sweep_target = start + delta * max_t;
    last_free_blink_position_abilities(world, abilities, start, sweep_target, half)
}

fn blink_path_blocker_abilities(abilities: &crate::abilities::AbilitySet, kind: BlockKind) -> bool {
    match kind {
        BlockKind::Solid => true,
        BlockKind::BlinkWall { tier } => !abilities_can_blink_through(abilities, tier),
        BlockKind::OneWay | BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => {
            false
        }
    }
}

fn last_free_blink_position_abilities(
    world: &World,
    abilities: &crate::abilities::AbilitySet,
    start: Vec2,
    target: Vec2,
    half: Vec2,
) -> Vec2 {
    let delta = target - start;
    let distance = delta.length();
    if distance <= 1.0e-5 {
        return start;
    }

    let steps = ((distance / 14.0).ceil() as usize).clamp(8, 64);
    let mut last_safe = start;
    for step in 1..=steps {
        let t = step as f32 / steps as f32;
        let candidate = start + delta * t;
        let candidate_aabb = Aabb::new(candidate, half);
        match blink_collision_abilities(world, abilities, candidate_aabb) {
            BlinkCollision::Free => last_safe = candidate,
            BlinkCollision::PassThrough => {}
            BlinkCollision::Blocked => break,
        }
    }
    last_safe
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlinkCollision {
    Free,
    PassThrough,
    Blocked,
}

fn blink_collision_abilities(
    world: &World,
    abilities: &crate::abilities::AbilitySet,
    aabb: Aabb,
) -> BlinkCollision {
    let mut pass_through = false;
    for block in &world.blocks {
        if !aabb.strict_intersects(block.aabb) {
            continue;
        }
        match block.kind {
            BlockKind::Solid => return BlinkCollision::Blocked,
            BlockKind::BlinkWall { tier } => {
                if abilities_can_blink_through(abilities, tier) {
                    pass_through = true;
                } else {
                    return BlinkCollision::Blocked;
                }
            }
            BlockKind::OneWay => pass_through = true,
            BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => {}
        }
    }
    if pass_through {
        BlinkCollision::PassThrough
    } else {
        BlinkCollision::Free
    }
}

fn abilities_can_blink_through(
    abilities: &crate::abilities::AbilitySet,
    tier: BlinkWallTier,
) -> bool {
    match tier {
        BlinkWallTier::Soft => abilities.blink_through_soft_walls,
        BlinkWallTier::Hard => abilities.blink_through_hard_walls,
    }
}
