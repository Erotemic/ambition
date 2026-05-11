use crate::geometry::{Aabb, AabbExt};
use crate::world::{BlinkWallTier, BlockKind, World};
use crate::Vec2;

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
    let direction = aim.normalize_or(Vec2::new(player.facing, 0.0));
    blink_destination_to_point(world, player, player.pos + direction * max_distance)
}

pub fn blink_destination_to_point(world: &World, player: &Player, target: Vec2) -> Vec2 {
    let start = player.pos;
    let half = player.size * 0.5;
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
            blink_path_blocker(player, block.kind)
        })
        .map(|hit| hit.time_of_impact)
        .unwrap_or(1.0);
    let sweep_target = start + delta * max_t;
    last_free_blink_position(world, player, start, sweep_target, half)
}

fn blink_path_blocker(player: &Player, kind: BlockKind) -> bool {
    match kind {
        BlockKind::Solid => true,
        BlockKind::BlinkWall { tier } => !player_can_blink_through(player, tier),
        BlockKind::OneWay | BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => {
            false
        }
    }
}

fn last_free_blink_position(
    world: &World,
    player: &Player,
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
        match blink_collision(world, player, candidate_aabb) {
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

fn blink_collision(world: &World, player: &Player, aabb: Aabb) -> BlinkCollision {
    let mut pass_through = false;
    for block in &world.blocks {
        if !aabb.strict_intersects(block.aabb) {
            continue;
        }
        match block.kind {
            BlockKind::Solid => return BlinkCollision::Blocked,
            BlockKind::BlinkWall { tier } => {
                if player_can_blink_through(player, tier) {
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

fn player_can_blink_through(player: &Player, tier: BlinkWallTier) -> bool {
    match tier {
        BlinkWallTier::Soft => player.abilities.blink_through_soft_walls,
        BlinkWallTier::Hard => player.abilities.blink_through_hard_walls,
    }
}
