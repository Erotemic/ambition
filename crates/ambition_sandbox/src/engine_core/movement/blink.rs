use crate::engine_core::geometry::{Aabb, AabbExt};
use crate::engine_core::world::{BlinkWallTier, BlockKind, World};
use crate::engine_core::Vec2;

use super::events::{BlinkEvent, FrameEvents};
use super::ops::MovementOp;
use super::player::Player;
use super::tuning::MovementTuning;

pub(super) fn complete_blink(
    player: &mut Player,
    from: Vec2,
    to: Vec2,
    precision: bool,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    player.pos = to;
    apply_post_blink_motion(player, precision, tuning);
    player.blink_cooldown = tuning.blink_cooldown;
    player.blink_hold_active = false;
    player.blink_hold_timer = 0.0;
    player.blink_aiming = false;
    player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
    let op = if precision {
        MovementOp::PrecisionBlink
    } else {
        MovementOp::Blink
    };
    events.op(player, op);
    events.blinks.push(BlinkEvent {
        from,
        to,
        precision,
    });
}

/// Cluster-ref variant of [`complete_blink`]. Mutates kinematics
/// (pos, vel), flight (fast_falling), wall (wall_clinging, wall_climbing),
/// dash (timer), blink (cooldown, aim_offset, hold_*), and pushes
/// blink ops + the BlinkEvent.
pub fn complete_blink_clusters(
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    flight: &mut crate::engine_core::player_clusters::PlayerFlightState,
    wall: &mut crate::engine_core::player_clusters::PlayerWallState,
    dash: &mut crate::engine_core::player_clusters::PlayerDashState,
    blink: &mut crate::engine_core::player_clusters::PlayerBlinkState,
    combo_trace: &mut crate::engine_core::player_clusters::PlayerComboTrace,
    from: Vec2,
    to: Vec2,
    precision: bool,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    kinematics.pos = to;
    // apply_post_blink_motion equivalent
    let damping = if precision { 0.35 } else { 0.55 };
    let max_downward = if precision {
        tuning.precision_blink_max_downward_speed
    } else {
        tuning.blink_max_downward_speed
    };
    kinematics.vel.x *= damping;
    if kinematics.vel.y > max_downward {
        kinematics.vel.y = max_downward;
    } else {
        kinematics.vel.y *= damping;
    }
    flight.fast_falling = false;
    wall.wall_clinging = false;
    wall.wall_climbing = false;
    dash.timer = 0.0;
    blink.grace_timer = tuning.blink_grace_time;

    blink.cooldown = tuning.blink_cooldown;
    blink.hold_active = false;
    blink.hold_timer = 0.0;
    blink.aiming = false;
    blink.aim_offset = Vec2::new(tuning.blink_distance * kinematics.facing, 0.0);
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

fn apply_post_blink_motion(player: &mut Player, precision: bool, tuning: MovementTuning) {
    let damping = if precision { 0.35 } else { 0.55 };
    let max_downward = if precision {
        tuning.precision_blink_max_downward_speed
    } else {
        tuning.blink_max_downward_speed
    };

    player.vel.x *= damping;
    if player.vel.y > max_downward {
        player.vel.y = max_downward;
    } else {
        player.vel.y *= damping;
    }

    player.fast_falling = false;
    player.wall_clinging = false;
    player.wall_climbing = false;
    player.dash_timer = 0.0;
    player.blink_grace_timer = tuning.blink_grace_time;
}

pub fn blink_destination(world: &World, player: &Player, aim: Vec2, max_distance: f32) -> Vec2 {
    blink_destination_internal(world, player.pos, player.size, player.facing, &player.abilities, aim, max_distance)
}

pub fn blink_destination_to_point(world: &World, player: &Player, target: Vec2) -> Vec2 {
    blink_destination_to_point_internal(world, player.pos, player.size, &player.abilities, target)
}

/// Cluster-ref variant of [`blink_destination`].
pub fn blink_destination_clusters(
    world: &World,
    kinematics: &crate::engine_core::player_clusters::PlayerKinematics,
    abilities: &crate::engine_core::player_clusters::PlayerAbilities,
    aim: Vec2,
    max_distance: f32,
) -> Vec2 {
    blink_destination_internal(
        world,
        kinematics.pos,
        kinematics.size,
        kinematics.facing,
        &abilities.abilities,
        aim,
        max_distance,
    )
}

/// Cluster-ref variant of [`blink_destination_to_point`].
pub fn blink_destination_to_point_clusters(
    world: &World,
    kinematics: &crate::engine_core::player_clusters::PlayerKinematics,
    abilities: &crate::engine_core::player_clusters::PlayerAbilities,
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
    facing: f32,
    abilities: &crate::engine_core::abilities::AbilitySet,
    aim: Vec2,
    max_distance: f32,
) -> Vec2 {
    let direction = aim.normalize_or(Vec2::new(facing, 0.0));
    blink_destination_to_point_internal(world, pos, size, abilities, pos + direction * max_distance)
}

fn blink_destination_to_point_internal(
    world: &World,
    start: Vec2,
    size: Vec2,
    abilities: &crate::engine_core::abilities::AbilitySet,
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

#[allow(dead_code)]
fn blink_path_blocker(player: &Player, kind: BlockKind) -> bool {
    blink_path_blocker_abilities(&player.abilities, kind)
}

fn blink_path_blocker_abilities(abilities: &crate::engine_core::abilities::AbilitySet, kind: BlockKind) -> bool {
    match kind {
        BlockKind::Solid => true,
        BlockKind::BlinkWall { tier } => !abilities_can_blink_through(abilities, tier),
        BlockKind::OneWay | BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => {
            false
        }
    }
}

#[allow(dead_code)]
fn last_free_blink_position(
    world: &World,
    player: &Player,
    start: Vec2,
    target: Vec2,
    half: Vec2,
) -> Vec2 {
    last_free_blink_position_abilities(world, &player.abilities, start, target, half)
}

fn last_free_blink_position_abilities(
    world: &World,
    abilities: &crate::engine_core::abilities::AbilitySet,
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

#[allow(dead_code)]
fn blink_collision(world: &World, player: &Player, aabb: Aabb) -> BlinkCollision {
    blink_collision_abilities(world, &player.abilities, aabb)
}

fn blink_collision_abilities(
    world: &World,
    abilities: &crate::engine_core::abilities::AbilitySet,
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

#[allow(dead_code)]
fn player_can_blink_through(player: &Player, tier: BlinkWallTier) -> bool {
    abilities_can_blink_through(&player.abilities, tier)
}

fn abilities_can_blink_through(abilities: &crate::engine_core::abilities::AbilitySet, tier: BlinkWallTier) -> bool {
    match tier {
        BlinkWallTier::Soft => abilities.blink_through_soft_walls,
        BlinkWallTier::Hard => abilities.blink_through_hard_walls,
    }
}
