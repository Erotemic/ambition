use crate::engine_core::geometry::{Aabb, AabbExt};
use crate::engine_core::world::{BlockKind, World};
use crate::engine_core::Vec2;

use super::tuning::MovementTuning;

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

fn is_solid_for_axis(kind: BlockKind, axis: Axis) -> bool {
    match kind {
        BlockKind::Solid | BlockKind::BlinkWall { .. } => true,
        BlockKind::OneWay => matches!(axis, Axis::Y),
        BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => false,
    }
}

fn block_passable_during_climb_clusters(
    body_mode: &crate::engine_core::player_clusters::PlayerBodyModeState,
    env_contact: &crate::engine_core::player_clusters::PlayerEnvironmentContact,
    block: &crate::engine_core::world::Block,
) -> bool {
    if !matches!(
        body_mode.body_mode,
        crate::engine_core::player_state::BodyMode::Climbing
    ) {
        return false;
    }
    let Some(contact) = env_contact.climbable else {
        return false;
    };
    if matches!(block.kind, BlockKind::Hazard) {
        return false;
    }
    contact.region_aabb.strict_intersects(block.aabb)
}

fn sweep_fraction(time_of_impact: f32) -> f32 {
    time_of_impact.clamp(0.0, 1.0)
}

pub(super) fn body_is_side_contact(body: Aabb, block: Aabb) -> bool {
    const Y_NESTED_EPS: f32 = 1.0e-4;
    body.top() >= block.top() - Y_NESTED_EPS && body.bottom() <= block.bottom() + Y_NESTED_EPS
}

/// Cluster-native X-axis sweep. Mirrors [`sweep_player_x_clusters`] field-for-
/// field but writes through cluster components.
pub(super) fn sweep_player_x_clusters(
    world: &World,
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    wall: &mut crate::engine_core::player_clusters::PlayerWallState,
    body_mode: &crate::engine_core::player_clusters::PlayerBodyModeState,
    env_contact: &crate::engine_core::player_clusters::PlayerEnvironmentContact,
    delta_x: f32,
) {
    let delta = Vec2::new(delta_x, 0.0);
    if delta.x.abs() <= 1.0e-5 {
        resolve_axis_clusters(world, kinematics, wall, body_mode, env_contact, Axis::X);
        return;
    }

    if let Some(hit) = world.first_body_sweep(kinematics.aabb(), delta, |block| {
        is_solid_for_axis(block.kind, Axis::X)
            && !matches!(block.kind, BlockKind::OneWay)
            && !block_passable_during_climb_clusters(body_mode, env_contact, block)
    }) {
        let toi_fraction = sweep_fraction(hit.time_of_impact);
        kinematics.pos.x += delta.x * toi_fraction;
        let body = kinematics.aabb();
        let immediate_contact = hit.time_of_impact <= 1.0e-5;
        let overlap_x = (body.right().min(hit.block.aabb.right())
            - body.left().max(hit.block.aabb.left()))
        .max(0.0);
        let overlap_y = (body.bottom().min(hit.block.aabb.bottom())
            - body.top().max(hit.block.aabb.top()))
        .max(0.0);
        let vertical_dominant = immediate_contact && overlap_y > 0.0 && overlap_x > overlap_y;
        let body_to_right_of_block = body.center().x > hit.block.aabb.center().x;
        let moving_away_from_block =
            (body_to_right_of_block && delta.x > 0.0) || (!body_to_right_of_block && delta.x < 0.0);
        let horizontal_overlap_moving_away =
            immediate_contact && overlap_x > 0.0 && moving_away_from_block;
        if vertical_dominant || horizontal_overlap_moving_away {
            kinematics.pos.x += delta.x * (1.0 - toi_fraction);
        } else {
            if body_to_right_of_block {
                kinematics.pos.x += hit.block.aabb.right() - body.left();
                wall.wall_normal_x = 1.0;
            } else {
                kinematics.pos.x += hit.block.aabb.left() - body.right();
                wall.wall_normal_x = -1.0;
            }
            kinematics.vel.x = 0.0;
            wall.on_wall = true;
        }
    } else {
        kinematics.pos.x += delta.x;
    }

    resolve_axis_clusters(world, kinematics, wall, body_mode, env_contact, Axis::X);
}

/// Cluster-native Y-axis sweep. Mirrors [`sweep_player_y`] field-for-
/// field but writes through cluster components.
pub(super) fn sweep_player_y_clusters(
    world: &World,
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    ground: &mut crate::engine_core::player_clusters::PlayerGroundState,
    body_mode: &crate::engine_core::player_clusters::PlayerBodyModeState,
    env_contact: &crate::engine_core::player_clusters::PlayerEnvironmentContact,
    delta_y: f32,
    prev_bottom: f32,
    drop_through: bool,
) {
    let delta = Vec2::new(0.0, delta_y);
    if delta.y.abs() <= 1.0e-5 {
        resolve_vertical_clusters(
            world,
            kinematics,
            ground,
            body_mode,
            env_contact,
            prev_bottom,
            drop_through,
        );
        return;
    }

    let start_body = kinematics.aabb();
    if let Some(hit) = world.first_body_sweep(kinematics.aabb(), delta, |block| {
        if !is_solid_for_axis(block.kind, Axis::Y) {
            return false;
        }
        if block_passable_during_climb_clusters(body_mode, env_contact, block) {
            return false;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            let landing_from_above = delta.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0;
            return landing_from_above && !drop_through;
        }
        if body_is_side_contact(start_body, block.aabb) {
            return false;
        }
        if start_body.strict_intersects(block.aabb) {
            return false;
        }
        true
    }) {
        kinematics.pos.y += delta.y * sweep_fraction(hit.time_of_impact);
        let body = kinematics.aabb();
        let approaching_from_above = delta.y > 0.0 && prev_bottom <= hit.block.aabb.top() + 4.0;
        if approaching_from_above || body.center().y < hit.block.aabb.center().y {
            kinematics.pos.y += hit.block.aabb.top() - body.bottom();
            ground.on_ground = true;
        } else {
            kinematics.pos.y += hit.block.aabb.bottom() - body.top();
        }
        kinematics.vel.y = 0.0;
    } else {
        kinematics.pos.y += delta.y;
    }

    resolve_vertical_clusters(
        world,
        kinematics,
        ground,
        body_mode,
        env_contact,
        prev_bottom,
        drop_through,
    );
}

/// Cluster-native variant of [`resolve_axis`].
fn resolve_axis_clusters(
    world: &World,
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    wall: &mut crate::engine_core::player_clusters::PlayerWallState,
    _body_mode: &crate::engine_core::player_clusters::PlayerBodyModeState,
    _env_contact: &crate::engine_core::player_clusters::PlayerEnvironmentContact,
    axis: Axis,
) {
    let mut aabb = kinematics.aabb();
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, axis) || !aabb.strict_intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            continue;
        }
        match axis {
            Axis::X => {
                let overlap_x = (aabb.right().min(block.aabb.right())
                    - aabb.left().max(block.aabb.left()))
                .max(0.0);
                let overlap_y = (aabb.bottom().min(block.aabb.bottom())
                    - aabb.top().max(block.aabb.top()))
                .max(0.0);
                if overlap_x > overlap_y {
                    continue;
                }
                if aabb.center().x < block.aabb.center().x {
                    let push = block.aabb.left() - aabb.right();
                    kinematics.pos.x += push;
                    wall.wall_normal_x = -1.0;
                } else {
                    let push = block.aabb.right() - aabb.left();
                    kinematics.pos.x += push;
                    wall.wall_normal_x = 1.0;
                }
                kinematics.vel.x = 0.0;
                wall.on_wall = true;
            }
            Axis::Y => {}
        }
        aabb = kinematics.aabb();
    }
}

/// Cluster-native variant of [`resolve_vertical`].
fn resolve_vertical_clusters(
    world: &World,
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    ground: &mut crate::engine_core::player_clusters::PlayerGroundState,
    _body_mode: &crate::engine_core::player_clusters::PlayerBodyModeState,
    _env_contact: &crate::engine_core::player_clusters::PlayerEnvironmentContact,
    prev_bottom: f32,
    drop_through: bool,
) {
    let mut aabb = kinematics.aabb();
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, Axis::Y) || !aabb.strict_intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            let landing_from_above =
                kinematics.vel.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0;
            if !landing_from_above || drop_through {
                continue;
            }
        }
        if !matches!(block.kind, BlockKind::OneWay) && body_is_side_contact(aabb, block.aabb) {
            continue;
        }
        if aabb.center().y < block.aabb.center().y {
            let push = block.aabb.top() - aabb.bottom();
            kinematics.pos.y += push;
            ground.on_ground = true;
        } else {
            let push = block.aabb.bottom() - aabb.top();
            kinematics.pos.y += push;
        }
        kinematics.vel.y = 0.0;
        aabb = kinematics.aabb();
    }
}

/// AABB-only variant of [`standing_on_one_way`]. Cluster-aware
/// callers pass `PlayerKinematics::aabb()` directly.
pub fn standing_on_one_way_aabb(world: &World, body: Aabb) -> bool {
    for block in &world.blocks {
        if !matches!(block.kind, BlockKind::OneWay) {
            continue;
        }
        let horizontally_overlaps =
            body.right() > block.aabb.left() + 1.0 && body.left() < block.aabb.right() - 1.0;
        let near_top = (body.bottom() - block.aabb.top()).abs() <= 4.0;
        if horizontally_overlaps && near_top {
            return true;
        }
    }
    false
}

/// Tile-set-only hazard touch test. Cluster-aware callers
/// pass `PlayerKinematics::aabb()` directly without building an
/// `ae::Player`.
pub fn touching_hazard_aabb(world: &World, aabb: crate::engine_core::Aabb) -> bool {
    world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, BlockKind::Hazard) && aabb.strict_intersects(b.aabb))
}

/// Rebound impulse lookup for a body AABB.
pub fn touching_rebound_aabb(world: &World, aabb: crate::engine_core::Aabb) -> Option<Vec2> {
    world.blocks.iter().find_map(|b| match b.kind {
        BlockKind::Rebound { impulse } if aabb.strict_intersects(b.aabb) => Some(impulse),
        _ => None,
    })
}

/// Pogo attempt: spawn a downward hitbox, return the orb AABB if hit.
/// Mutates kinematics velocity,
/// refreshes movement resources on the dash/jump clusters, and
/// clears the ground flag.
pub fn try_pogo_clusters(
    world: &World,
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    abilities: &crate::engine_core::player_clusters::PlayerAbilities,
    dash: &mut crate::engine_core::player_clusters::PlayerDashState,
    jump_state: &mut crate::engine_core::player_clusters::PlayerJumpState,
    ground: &mut crate::engine_core::player_clusters::PlayerGroundState,
    tuning: MovementTuning,
) -> Option<Aabb> {
    let feet = kinematics.aabb();
    let hitbox = Aabb::new(
        Vec2::new(feet.center().x, feet.bottom() + 18.0),
        Vec2::new(feet.half_size().x * 0.76, 22.0),
    );
    let hit = world.blocks.iter().find(|block| {
        let valid_target = matches!(
            block.kind,
            BlockKind::PogoOrb
                | BlockKind::Solid
                | BlockKind::BlinkWall { .. }
                | BlockKind::Rebound { .. }
        );
        valid_target && hitbox.strict_intersects(block.aabb)
    });
    if let Some(block) = hit {
        let aabb = block.aabb;
        kinematics.vel.y = -tuning.pogo_speed;
        crate::engine_core::player_clusters::refresh_movement_resources_clusters(
            abilities, dash, jump_state, tuning,
        );
        ground.on_ground = false;
        Some(aabb)
    } else {
        None
    }
}
