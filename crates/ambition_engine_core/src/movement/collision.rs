use crate::collision_semantics::{
    axis_role, body_on_support_side, is_contact_range_snap, is_full_collision_surface,
    is_solid_for_axis, moving_toward_feet, one_way_landing_from_previous_feet, snap_feet_to_surface,
    supporting_block, surface_supports_body_at_rest, Axis, AxisRole,
};
use crate::geometry::{Aabb, AabbExt};
use crate::world::{BlockKind, World};
use crate::Vec2;

/// Apply a penetration snap/push to the body position only when it is a genuine
/// bounded contact correction, never a pushout-teleport. Returns whether it was
/// applied so callers can gate the matching velocity-zero / contact flags: a
/// rejected (catastrophic) push leaves the body and its velocity untouched so it
/// depenetrates out the near face over subsequent frames. See
/// [`is_contact_range_snap`] — the engine's shared no-artificial-pushout guard.
#[must_use]
fn apply_bounded_resolution(
    kinematics: &mut crate::player_clusters::BodyKinematics,
    gravity_dir: Vec2,
    delta: Vec2,
) -> bool {
    if !is_contact_range_snap(delta, kinematics.aabb_oriented(gravity_dir)) {
        return false;
    }
    kinematics.pos += delta;
    true
}

fn one_way_landing_from_feet(
    body: Aabb,
    block: Aabb,
    delta: Vec2,
    gravity_dir: Vec2,
    drop_through: bool,
) -> bool {
    one_way_landing_from_previous_feet(
        body,
        block,
        delta,
        gravity_dir,
        drop_through,
        body.feet_coord(gravity_dir),
    )
}

fn axis_face_resolution(body: Aabb, block: Aabb, axis: Axis) -> (Vec2, Vec2) {
    match axis {
        Axis::X => {
            if body.center().x <= block.center().x {
                (
                    Vec2::new(block.left() - body.right(), 0.0),
                    Vec2::new(-1.0, 0.0),
                )
            } else {
                (
                    Vec2::new(block.right() - body.left(), 0.0),
                    Vec2::new(1.0, 0.0),
                )
            }
        }
        Axis::Y => {
            if body.center().y <= block.center().y {
                (
                    Vec2::new(0.0, block.top() - body.bottom()),
                    Vec2::new(0.0, -1.0),
                )
            } else {
                (
                    Vec2::new(0.0, block.bottom() - body.top()),
                    Vec2::new(0.0, 1.0),
                )
            }
        }
    }
}

fn apply_side_contact(
    wall: &mut crate::player_clusters::BodyWallState,
    world_normal: Vec2,
    gravity_dir: Vec2,
) {
    let frame = crate::AccelerationFrame::new(gravity_dir);
    let local_side_normal = world_normal.dot(frame.side);
    if local_side_normal.abs() >= 0.5 {
        wall.on_wall = true;
        wall.wall_normal_x = local_side_normal.signum();
    }
}

fn block_passable_during_climb_clusters(
    body_mode: &crate::player_clusters::BodyModeState,
    env_contact: &crate::player_clusters::BodyEnvironmentContact,
    block: &crate::world::Block,
) -> bool {
    if !matches!(body_mode.body_mode, crate::player_state::BodyMode::Climbing) {
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
    kinematics: &mut crate::player_clusters::BodyKinematics,
    wall: &mut crate::player_clusters::BodyWallState,
    body_mode: &crate::player_clusters::BodyModeState,
    env_contact: &crate::player_clusters::BodyEnvironmentContact,
    delta_x: f32,
    drop_through: bool,
    gravity_dir: Vec2,
) {
    let axis = Axis::X;
    let role = axis_role(axis, gravity_dir);
    let delta = Vec2::new(delta_x, 0.0);
    if delta.x.abs() <= 1.0e-5 {
        resolve_axis_clusters(
            world,
            kinematics,
            wall,
            body_mode,
            env_contact,
            axis,
            gravity_dir,
        );
        return;
    }

    let start_body = kinematics.aabb_oriented(gravity_dir);
    if let Some(hit) = world.first_body_sweep(start_body, delta, |block| {
        if !is_solid_for_axis(block.kind, axis, gravity_dir) {
            return false;
        }
        if block_passable_during_climb_clusters(body_mode, env_contact, block) {
            return false;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            return one_way_landing_from_feet(
                start_body,
                block.aabb,
                delta,
                gravity_dir,
                drop_through,
            );
        }
        if start_body.strict_intersects(block.aabb) {
            return false;
        }
        true
    }) {
        let toi_fraction = sweep_fraction(hit.time_of_impact);
        kinematics.pos.x += delta.x * toi_fraction;
        let body = kinematics.aabb_oriented(gravity_dir);
        if matches!(hit.block.kind, BlockKind::OneWay)
            || (role == AxisRole::Gravity && moving_toward_feet(delta, gravity_dir))
        {
            let snap = snap_feet_to_surface(body, hit.block.aabb, gravity_dir);
            let _ = apply_bounded_resolution(kinematics, gravity_dir, snap);
            kinematics.vel.x = 0.0;
        } else if role == AxisRole::Gravity {
            let (push, _) = axis_face_resolution(body, hit.block.aabb, axis);
            let _ = apply_bounded_resolution(kinematics, gravity_dir, push);
            kinematics.vel.x = 0.0;
        } else {
            let body = kinematics.aabb_oriented(gravity_dir);
            let immediate_contact = hit.time_of_impact <= 1.0e-5;
            let overlap_x = (body.right().min(hit.block.aabb.right())
                - body.left().max(hit.block.aabb.left()))
            .max(0.0);
            let body_to_right_of_block = body.center().x > hit.block.aabb.center().x;
            let moving_away_from_block = (body_to_right_of_block && delta.x > 0.0)
                || (!body_to_right_of_block && delta.x < 0.0);
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
        }
    } else {
        kinematics.pos.x += delta.x;
    }

    resolve_axis_clusters(
        world,
        kinematics,
        wall,
        body_mode,
        env_contact,
        axis,
        gravity_dir,
    );
}

/// Swept-AABB Y-axis collision step. Handles the OneWay
/// landing-from-above gate, rejects pre-existing penetrations + wall-
/// cling-side contacts (the y-sweep can't resolve those), and snaps
/// to a TOI hit. Falls back to `resolve_vertical_clusters` for the
/// positional repair.
pub(super) fn sweep_player_y_clusters(
    world: &World,
    kinematics: &mut crate::player_clusters::BodyKinematics,
    ground: &mut crate::player_clusters::BodyGroundState,
    wall: &mut crate::player_clusters::BodyWallState,
    body_mode: &crate::player_clusters::BodyModeState,
    env_contact: &crate::player_clusters::BodyEnvironmentContact,
    delta_y: f32,
    prev_feet_coord: f32,
    drop_through: bool,
    gravity_dir: Vec2,
) {
    let axis = Axis::Y;
    let role = axis_role(axis, gravity_dir);
    let delta = Vec2::new(0.0, delta_y);
    if delta.y.abs() <= 1.0e-5 {
        resolve_vertical_clusters(
            world,
            kinematics,
            ground,
            wall,
            body_mode,
            env_contact,
            prev_feet_coord,
            drop_through,
            gravity_dir,
        );
        return;
    }

    let start_body = kinematics.aabb_oriented(gravity_dir);
    if let Some(hit) = world.first_body_sweep(start_body, delta, |block| {
        if !is_solid_for_axis(block.kind, axis, gravity_dir) {
            return false;
        }
        if block_passable_during_climb_clusters(body_mode, env_contact, block) {
            return false;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            return one_way_landing_from_feet(
                start_body,
                block.aabb,
                delta,
                gravity_dir,
                drop_through,
            );
        }
        if role == AxisRole::Gravity && body_is_side_contact(start_body, block.aabb) {
            return false;
        }
        if start_body.strict_intersects(block.aabb) {
            return false;
        }
        true
    }) {
        kinematics.pos.y += delta.y * sweep_fraction(hit.time_of_impact);
        let body = kinematics.aabb_oriented(gravity_dir);
        if matches!(hit.block.kind, BlockKind::OneWay)
            || (role == AxisRole::Gravity && moving_toward_feet(delta, gravity_dir))
        {
            let snap = snap_feet_to_surface(body, hit.block.aabb, gravity_dir);
            let _ = apply_bounded_resolution(kinematics, gravity_dir, snap);
            kinematics.vel.y = 0.0;
            if role == AxisRole::Gravity {
                ground.on_ground = true;
            }
        } else {
            let (push, world_normal) = axis_face_resolution(body, hit.block.aabb, axis);
            if apply_bounded_resolution(kinematics, gravity_dir, push) {
                kinematics.vel.y = 0.0;
                if role == AxisRole::Side {
                    apply_side_contact(wall, world_normal, gravity_dir);
                }
            }
        }
    } else {
        kinematics.pos.y += delta.y;
    }

    resolve_vertical_clusters(
        world,
        kinematics,
        ground,
        wall,
        body_mode,
        env_contact,
        prev_feet_coord,
        drop_through,
        gravity_dir,
    );
}

/// Is the body resting on a surface on the side gravity pulls toward? Probes the
/// controlled body's feet face against any support surface (Solid, BlinkWall, or
/// OneWay) using the same support-face rule as the sweeps. Cardinal `gravity_dir`.
pub(super) fn grounded_against_gravity(
    world: &World,
    body: Aabb,
    gravity_dir: Vec2,
    drop_through: bool,
) -> bool {
    supporting_block(world, body, gravity_dir, drop_through).is_some()
}

/// Stabilize a body that is already touching a support face on the gravity side.
///
/// Sweeps own the time-of-impact contacts. This helper owns the at-rest/probe
/// case: if the oriented body is resting on a support surface, snap its feet to
/// that support face and clear any velocity that is still trying to move toward
/// the feet. This keeps sideways wall-walking from reporting `on_ground` while
/// carrying a stale fall velocity.
pub(super) fn stabilize_on_support(
    world: &World,
    kinematics: &mut crate::player_clusters::BodyKinematics,
    gravity_dir: Vec2,
    drop_through: bool,
) -> bool {
    let body = kinematics.aabb_oriented(gravity_dir);
    let Some(support) = supporting_block(world, body, gravity_dir, drop_through) else {
        return false;
    };
    kinematics.pos += snap_feet_to_surface(body, support.aabb, gravity_dir);
    let descend = kinematics.vel.dot(gravity_dir);
    if descend > 0.0 {
        kinematics.vel -= gravity_dir * descend;
    }
    true
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
/// 3. And NEVER a pushout-teleport: a chosen exit deeper than the body's own
///    half-extent means the body is embedded, not in contact — defer (the body's
///    velocity carries it out the near face over subsequent frames). See
///    [`is_contact_range_snap`].
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
    let chosen = if exit_left <= exit_right {
        left.or(right)
    } else {
        right.or(left)
    };
    chosen.filter(|&(dx, _)| is_contact_range_snap(Vec2::new(dx, 0.0), body))
}

/// Penetration repair for one axis. X/Y remain the low-level sweep axes because
/// the world is axis-aligned, but support and wall decisions are expressed in
/// controlled-body terms: feet/head along the gravity axis, side normals along
/// the local side axis.
fn resolve_axis_clusters(
    world: &World,
    kinematics: &mut crate::player_clusters::BodyKinematics,
    wall: &mut crate::player_clusters::BodyWallState,
    _body_mode: &crate::player_clusters::BodyModeState,
    _env_contact: &crate::player_clusters::BodyEnvironmentContact,
    axis: Axis,
    gravity_dir: Vec2,
) {
    let role = axis_role(axis, gravity_dir);
    let mut aabb = kinematics.aabb_oriented(gravity_dir);
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, axis, gravity_dir) || !aabb.strict_intersects(block.aabb)
        {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            continue;
        }
        match role {
            AxisRole::Gravity => {
                let delta = if body_on_support_side(aabb, block.aabb, gravity_dir) {
                    snap_feet_to_surface(aabb, block.aabb, gravity_dir)
                } else {
                    axis_face_resolution(aabb, block.aabb, axis).0
                };
                if apply_bounded_resolution(kinematics, gravity_dir, delta) {
                    match axis {
                        Axis::X => kinematics.vel.x = 0.0,
                        Axis::Y => kinematics.vel.y = 0.0,
                    }
                }
            }
            AxisRole::Side => {
                if axis == Axis::X {
                    if let Some((dx, normal)) =
                        resolve_x_penetration(aabb, block.aabb, world.size.x)
                    {
                        kinematics.pos.x += dx;
                        wall.wall_normal_x = normal;
                        kinematics.vel.x = 0.0;
                        wall.on_wall = true;
                    }
                } else {
                    let (push, world_normal) = axis_face_resolution(aabb, block.aabb, axis);
                    if apply_bounded_resolution(kinematics, gravity_dir, push) {
                        kinematics.vel.y = 0.0;
                        apply_side_contact(wall, world_normal, gravity_dir);
                    }
                }
            }
        }
        aabb = kinematics.aabb_oriented(gravity_dir);
    }
}

/// Penetration repair for the Y axis. Mirrors `resolve_axis_clusters`
/// but also owns grounding because the Y sweep receives `ground`.
fn resolve_vertical_clusters(
    world: &World,
    kinematics: &mut crate::player_clusters::BodyKinematics,
    ground: &mut crate::player_clusters::BodyGroundState,
    wall: &mut crate::player_clusters::BodyWallState,
    _body_mode: &crate::player_clusters::BodyModeState,
    _env_contact: &crate::player_clusters::BodyEnvironmentContact,
    prev_feet_coord: f32,
    drop_through: bool,
    gravity_dir: Vec2,
) {
    let axis = Axis::Y;
    let role = axis_role(axis, gravity_dir);
    let mut aabb = kinematics.aabb_oriented(gravity_dir);
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, axis, gravity_dir) || !aabb.strict_intersects(block.aabb)
        {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            if role != AxisRole::Gravity {
                continue;
            }
            let delta = kinematics.vel * 1.0e-3;
            if !one_way_landing_from_previous_feet(
                aabb,
                block.aabb,
                delta,
                gravity_dir,
                drop_through,
                prev_feet_coord,
            ) {
                continue;
            }
        }
        if role == AxisRole::Gravity
            && is_full_collision_surface(block.kind)
            && body_is_side_contact(aabb, block.aabb)
        {
            continue;
        }
        match role {
            AxisRole::Gravity => {
                let on_support = matches!(block.kind, BlockKind::OneWay)
                    || body_on_support_side(aabb, block.aabb, gravity_dir);
                let delta = if on_support {
                    snap_feet_to_surface(aabb, block.aabb, gravity_dir)
                } else {
                    axis_face_resolution(aabb, block.aabb, axis).0
                };
                if apply_bounded_resolution(kinematics, gravity_dir, delta) {
                    if on_support {
                        ground.on_ground = true;
                    }
                    kinematics.vel.y = 0.0;
                }
            }
            AxisRole::Side => {
                let (push, world_normal) = axis_face_resolution(aabb, block.aabb, axis);
                if apply_bounded_resolution(kinematics, gravity_dir, push) {
                    kinematics.vel.y = 0.0;
                    apply_side_contact(wall, world_normal, gravity_dir);
                }
            }
        }
        aabb = kinematics.aabb_oriented(gravity_dir);
    }
}

/// AABB-only variant of [`standing_on_one_way`]. Cluster-aware
/// callers pass the oriented body AABB directly. Gravity-relative: the body
/// rests on the one-way's anti-gravity support face, so drop-through detection
/// flips with gravity like the landing sweep already does.
pub fn standing_on_one_way_aabb(world: &World, body: Aabb, gravity_dir: Vec2) -> bool {
    world.blocks.iter().any(|block| {
        matches!(block.kind, BlockKind::OneWay)
            && surface_supports_body_at_rest(block.kind, body, block.aabb, gravity_dir, false)
    })
}

/// Tile-set-only hazard touch test. Cluster-aware callers
/// pass `BodyKinematics::aabb()` directly without building an
/// `ae::Player`.
pub fn touching_hazard_aabb(world: &World, aabb: crate::Aabb) -> bool {
    world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, BlockKind::Hazard) && aabb.strict_intersects(b.aabb))
}

/// Rebound impulse lookup for a body AABB.
pub fn touching_rebound_aabb(world: &World, aabb: crate::Aabb) -> Option<Vec2> {
    world.blocks.iter().find_map(|b| match b.kind {
        BlockKind::Rebound { impulse } if aabb.strict_intersects(b.aabb) => Some(impulse),
        _ => None,
    })
}

// `try_pogo_clusters` (the probe-based engine pogo) was removed 2026-06-16 — it
// was a redundant duplicate of the sandbox hitbox pogo (`advance_attack`), which
// detects the target with the real attack hitbox and bounces gravity-relatively.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::aabb_from_min_size;

    fn body(center: Vec2, half: Vec2) -> Aabb {
        Aabb::new(center, half)
    }

    #[test]
    fn support_faces_are_gravity_relative_for_full_solids() {
        let floor = aabb_from_min_size(Vec2::new(0.0, 100.0), Vec2::new(100.0, 20.0));
        let b = body(Vec2::new(40.0, 80.0), Vec2::new(10.0, 20.0));
        assert!(surface_supports_body_at_rest(
            BlockKind::Solid,
            b,
            floor,
            Vec2::new(0.0, 1.0),
            false,
        ));

        let wall = aabb_from_min_size(Vec2::new(100.0, 0.0), Vec2::new(20.0, 100.0));
        let sideways = body(Vec2::new(80.0, 40.0), Vec2::new(20.0, 10.0));
        assert!(surface_supports_body_at_rest(
            BlockKind::BlinkWall {
                tier: crate::world::BlinkWallTier::Soft
            },
            sideways,
            wall,
            Vec2::new(1.0, 0.0),
            false,
        ));
    }

    #[test]
    fn one_way_support_faces_are_gravity_relative() {
        let platform = aabb_from_min_size(Vec2::new(100.0, 0.0), Vec2::new(20.0, 100.0));
        let b = body(Vec2::new(80.0, 40.0), Vec2::new(20.0, 10.0));
        assert!(surface_supports_body_at_rest(
            BlockKind::OneWay,
            b,
            platform,
            Vec2::new(1.0, 0.0),
            false,
        ));
        assert!(!surface_supports_body_at_rest(
            BlockKind::OneWay,
            b,
            platform,
            Vec2::new(1.0, 0.0),
            true,
        ));
    }
}
