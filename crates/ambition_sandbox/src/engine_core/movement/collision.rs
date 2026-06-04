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

/// Swept-AABB X-axis collision step. Shape-casts the player body
/// against the world by `delta_x`; on a TOI hit, snaps to the touch
/// face and zeros `vel.x` / arms `wall.on_wall`. Falls back to the
/// positional `resolve_axis_clusters` repair for stacked contacts or
/// pre-existing penetrations.
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
        let body_to_right_of_block = body.center().x > hit.block.aabb.center().x;
        let moving_away_from_block =
            (body_to_right_of_block && delta.x > 0.0) || (!body_to_right_of_block && delta.x < 0.0);
        let horizontal_overlap_moving_away =
            immediate_contact && overlap_x > 0.0 && moving_away_from_block;
        // Resolve the X penetration robustly via the shared helper: defer to the
        // Y pass when the vertical exit is shorter -- crucially REGARDLESS of
        // `immediate_contact`. A body sliding PARALLEL just under the wide thin
        // ceiling (its top grazing the ceiling's bottom edge) makes the swept
        // cast return a spurious *non-immediate* grazing hit; the old
        // immediate-only guard let that fall through to a far-X-edge push,
        // teleporting the body ~900px out of the room. `None` => not an X
        // contact to resolve here, so keep the swept motion going.
        let depen = resolve_x_penetration(body, hit.block.aabb, world.size.x);
        if horizontal_overlap_moving_away || depen.is_none() {
            kinematics.pos.x += delta.x * (1.0 - toi_fraction);
        } else {
            let (dx, normal) = depen.expect("checked is_none above");
            kinematics.pos.x += dx;
            wall.wall_normal_x = normal;
            kinematics.vel.x = 0.0;
            wall.on_wall = true;
        }
    } else {
        kinematics.pos.x += delta.x;
    }

    resolve_axis_clusters(world, kinematics, wall, body_mode, env_contact, Axis::X);
}

/// Swept-AABB Y-axis collision step. Handles the OneWay
/// landing-from-above gate, rejects pre-existing penetrations + wall-
/// cling-side contacts (the y-sweep can't resolve those), and snaps
/// to a TOI hit. Falls back to `resolve_vertical_clusters` for the
/// positional repair.
pub(super) fn sweep_player_y_clusters(
    world: &World,
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    ground: &mut crate::engine_core::player_clusters::PlayerGroundState,
    body_mode: &crate::engine_core::player_clusters::PlayerBodyModeState,
    env_contact: &crate::engine_core::player_clusters::PlayerEnvironmentContact,
    delta_y: f32,
    prev_bottom: f32,
    drop_through: bool,
    gravity_sign: f32,
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
            gravity_sign,
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
        let snap_to_top = approaching_from_above || body.center().y < hit.block.aabb.center().y;
        if snap_to_top {
            kinematics.pos.y += hit.block.aabb.top() - body.bottom();
        } else {
            kinematics.pos.y += hit.block.aabb.bottom() - body.top();
        }
        // Grounded when the contact is on the side gravity pulls toward: a
        // block top under normal gravity, a block bottom (standing on a
        // ceiling) under flipped gravity.
        if contact_is_gravity_side(snap_to_top, gravity_sign) {
            ground.on_ground = true;
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
        gravity_sign,
    );
}

/// Is a vertical contact on the side gravity pulls toward (so it's "ground")?
/// `snap_to_top` = the body snapped to a block's TOP (it's above the block).
/// Under normal gravity (`+`) a top contact is ground; under flipped gravity
/// (`-`) a bottom contact (standing on a ceiling) is ground.
fn contact_is_gravity_side(snap_to_top: bool, gravity_sign: f32) -> bool {
    if gravity_sign >= 0.0 {
        snap_to_top
    } else {
        !snap_to_top
    }
}

/// Resolve an X-axis penetration of `body` into `block`, returning the
/// `(dx, wall_normal_x)` to apply, or `None` to defer to the Y pass.
///
/// Two rules, both guarding the OOB class from flying into the hub's wide, thin
/// ceiling:
/// 1. If the vertical exit is shorter, it's a floor/ceiling contact -- defer to
///    the Y pass (which snaps the body out the short way) instead of shoving it
///    out the wide block's far X edge (hundreds of px).
/// 2. Otherwise push out the nearer X face, but NEVER out of the world: at a top
///    corner the nearer face of a boundary-spanning block IS the world edge, so
///    pick the other face; if both X exits would leave the world, defer to Y.
fn resolve_x_penetration(body: Aabb, block: Aabb, world_w: f32) -> Option<(f32, f32)> {
    let exit_left = body.right() - block.left(); // push left (-) this far
    let exit_right = block.right() - body.left(); // push right (+) this far
    let exit_up = body.bottom() - block.top();
    let exit_down = block.bottom() - body.top();
    if exit_up.min(exit_down) <= exit_left.min(exit_right) {
        return None; // vertical exit is shorter -> the Y pass owns it
    }
    let half_w = (body.right() - body.left()) * 0.5;
    let cx = body.center().x;
    let left = ((cx - exit_left) - half_w >= 0.0).then_some((-exit_left, -1.0));
    let right = ((cx + exit_right) + half_w <= world_w).then_some((exit_right, 1.0));
    // Prefer the shorter exit; fall back to the other if it would leave the world.
    if exit_left <= exit_right {
        left.or(right)
    } else {
        right.or(left)
    }
}

/// Penetration repair for the X axis. Pushes the body out of any block it
/// strictly intersects after the shape sweep via [`resolve_x_penetration`]
/// (shortest non-ejecting exit, or defer to the Y pass for floor/ceiling
/// contacts).
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
                if let Some((dx, normal)) =
                    resolve_x_penetration(aabb, block.aabb, world.size.x)
                {
                    kinematics.pos.x += dx;
                    wall.wall_normal_x = normal;
                    kinematics.vel.x = 0.0;
                    wall.on_wall = true;
                }
            }
            Axis::Y => {}
        }
        aabb = kinematics.aabb();
    }
}

/// Penetration repair for the Y axis. Mirrors `resolve_axis_clusters`
/// but for vertical contacts: handles one-way landing-from-above gating
/// and skips the wall-cling-side contact class so vertical snaps don't
/// teleport a clinging body to a wall's far edge.
fn resolve_vertical_clusters(
    world: &World,
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    ground: &mut crate::engine_core::player_clusters::PlayerGroundState,
    _body_mode: &crate::engine_core::player_clusters::PlayerBodyModeState,
    _env_contact: &crate::engine_core::player_clusters::PlayerEnvironmentContact,
    prev_bottom: f32,
    drop_through: bool,
    gravity_sign: f32,
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
        let snap_to_top = aabb.center().y < block.aabb.center().y;
        if snap_to_top {
            let push = block.aabb.top() - aabb.bottom();
            kinematics.pos.y += push;
        } else {
            let push = block.aabb.bottom() - aabb.top();
            kinematics.pos.y += push;
        }
        if contact_is_gravity_side(snap_to_top, gravity_sign) {
            ground.on_ground = true;
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
    let hit = world
        .blocks
        .iter()
        .find(|block| block.kind.is_pogo_target() && hitbox.strict_intersects(block.aabb));
    if let Some(block) = hit {
        let aabb = block.aabb;
        super::integration::set_jump_velocity(&mut kinematics.vel, tuning.gravity_dir, tuning.pogo_speed);
        crate::engine_core::player_clusters::refresh_movement_resources_clusters(
            abilities, dash, jump_state, tuning,
        );
        ground.on_ground = false;
        Some(aabb)
    } else {
        None
    }
}
