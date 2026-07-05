use crate::collision_semantics::{
    axis_role, block_face_contact, body_on_support_side, is_contact_range_snap,
    is_full_collision_surface, is_solid_for_axis, moving_toward_feet,
    one_way_landing_from_previous_feet, snap_feet_to_surface, surface_supports_body_at_rest, Axis,
    AxisRole, Contact,
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
    kinematics: &mut crate::body_clusters::BodyKinematics,
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
    wall: &mut crate::body_clusters::BodyWallState,
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
    body_mode: &crate::body_clusters::BodyModeState,
    env_contact: &crate::body_clusters::BodyEnvironmentContact,
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

/// `(min, max)` span of an AABB along one world axis.
fn axis_span(aabb: Aabb, axis: Axis) -> (f32, f32) {
    match axis {
        Axis::X => (aabb.left(), aabb.right()),
        Axis::Y => (aabb.top(), aabb.bottom()),
    }
}

fn axis_vec(axis: Axis, along: f32) -> Vec2 {
    match axis {
        Axis::X => Vec2::new(along, 0.0),
        Axis::Y => Vec2::new(0.0, along),
    }
}

fn axis_component(v: Vec2, axis: Axis) -> f32 {
    match axis {
        Axis::X => v.x,
        Axis::Y => v.y,
    }
}

fn perp(axis: Axis) -> Axis {
    match axis {
        Axis::X => Axis::Y,
        Axis::Y => Axis::X,
    }
}

fn zero_axis_vel(kinematics: &mut crate::body_clusters::BodyKinematics, axis: Axis) {
    match axis {
        Axis::X => kinematics.vel.x = 0.0,
        Axis::Y => kinematics.vel.y = 0.0,
    }
}

/// The body's span ALONG the swept axis is nested inside the block's span: the
/// contact can only be on a face perpendicular to the sweep (a side graze while
/// sweeping the gravity axis), never a support/head face — so the gravity-axis
/// pass must not resolve it. The axis-role generalization of the old Y-only
/// `body_is_side_contact` (fable review 2026-07-02 §B5: the guard was welded to
/// world axes, so under sideways gravity exact-edge grazes became spurious
/// landings).
fn body_is_nested_along(body: Aabb, block: Aabb, axis: Axis) -> bool {
    const NESTED_EPS: f32 = 1.0e-4;
    let (body_min, body_max) = axis_span(body, axis);
    let (block_min, block_max) = axis_span(block, axis);
    body_min >= block_min - NESTED_EPS && body_max <= block_max + NESTED_EPS
}

/// Down-gravity flavor kept for the focused unit tests; production goes
/// through [`body_is_nested_along`] with the swept axis.
#[cfg(test)]
pub(super) fn body_is_side_contact(body: Aabb, block: Aabb) -> bool {
    body_is_nested_along(body, block, Axis::Y)
}

/// Resolve a SIDE-axis penetration of `body` into `block` along `axis`,
/// returning `(delta_along, world_normal_sign)` to apply, or `None` to defer to
/// the gravity pass. Axis-role generalization of the old X-only
/// `resolve_x_penetration`; the guards protect whichever axis currently plays
/// the side role, so they rotate with gravity.
///
/// Three rules, all guarding the OOB class from flying into a wide, thin block:
/// 1. If the perpendicular exit is shorter, it's a support/head contact — defer
///    to the gravity pass (which snaps the body out the short way) instead of
///    shoving it out the wide block's far side edge (hundreds of px).
/// 2. Otherwise push out the nearer side face, but NEVER out of the world: at a
///    corner the nearer face of a boundary-spanning block IS the world edge, so
///    pick the other face; if both exits would leave the world, defer.
/// 3. And NEVER a pushout-teleport: a chosen exit deeper than the body's own
///    half-extent means the body is embedded, not in contact — defer (the body's
///    velocity carries it out the near face over subsequent frames). See
///    [`is_contact_range_snap`].
fn resolve_side_penetration(
    body: Aabb,
    block: Aabb,
    axis: Axis,
    world_extent_along: f32,
) -> Option<(f32, f32)> {
    let (body_min, body_max) = axis_span(body, axis);
    let (block_min, block_max) = axis_span(block, axis);
    let exit_neg = body_max - block_min; // push toward -axis this far
    let exit_pos = block_max - body_min; // push toward +axis this far
    let (pbody_min, pbody_max) = axis_span(body, perp(axis));
    let (pblock_min, pblock_max) = axis_span(block, perp(axis));
    let exit_perp = (pbody_max - pblock_min).min(pblock_max - pbody_min);
    if exit_perp <= exit_neg.min(exit_pos) {
        return None; // perpendicular exit is shorter -> the gravity pass owns it
    }
    let half = (body_max - body_min) * 0.5;
    let center = (body_min + body_max) * 0.5;
    let neg = ((center - exit_neg) - half >= 0.0).then_some((-exit_neg, -1.0));
    let pos = ((center + exit_pos) + half <= world_extent_along).then_some((exit_pos, 1.0));
    // Prefer the shorter exit; fall back to the other if it would leave the world.
    let chosen = if exit_neg <= exit_pos {
        neg.or(pos)
    } else {
        pos.or(neg)
    };
    chosen.filter(|&(d, _)| is_contact_range_snap(axis_vec(axis, d), body))
}

/// Swept-AABB collision step for ONE world axis, role-aware: the same function
/// serves the side pass and the gravity pass for every cardinal gravity, so no
/// guard can exist on one axis and not the other (fable review 2026-07-02
/// §B5 — the old `sweep_player_x/y` pair each carried protections the other
/// lacked, which surfaced whenever gravity rotated onto the unguarded axis).
///
/// Role behavior:
/// - **Gravity axis**: OneWay landing gate, nested side-graze rejection, feet
///   snap + `on_ground` when moving toward the feet, head-face push otherwise.
/// - **Side axis**: guarded side resolution ([`resolve_side_penetration`]:
///   defer / world-bounds / no-pushout) with grazing-motion continuation, and
///   wall contact armed in the body's LOCAL frame via [`apply_side_contact`]
///   (the old X path stored the raw world sign, breaking cling under up
///   gravity).
///
/// Falls back to [`resolve_axis_repair`] for stacked contacts or pre-existing
/// penetrations.
#[allow(clippy::too_many_arguments)]
pub(super) fn sweep_player_axis_clusters(
    world: &World,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    ground: &mut crate::body_clusters::BodyGroundState,
    wall: &mut crate::body_clusters::BodyWallState,
    body_mode: &crate::body_clusters::BodyModeState,
    env_contact: &crate::body_clusters::BodyEnvironmentContact,
    axis: Axis,
    delta_along: f32,
    prev_feet_coord: f32,
    drop_through: bool,
    gravity_dir: Vec2,
    contacts: &mut Vec<Contact>,
) {
    let role = axis_role(axis, gravity_dir);
    let delta = axis_vec(axis, delta_along);
    if delta_along.abs() <= 1.0e-5 {
        resolve_axis_repair(
            world,
            kinematics,
            ground,
            wall,
            axis,
            prev_feet_coord,
            drop_through,
            gravity_dir,
            contacts,
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
        if role == AxisRole::Gravity && body_is_nested_along(start_body, block.aabb, axis) {
            return false;
        }
        if start_body.strict_intersects(block.aabb) {
            return false;
        }
        true
    }) {
        let toi_fraction = sweep_fraction(hit.time_of_impact);
        kinematics.pos += axis_vec(axis, delta_along * toi_fraction);
        let body = kinematics.aabb_oriented(gravity_dir);
        if matches!(hit.block.kind, BlockKind::OneWay)
            || (role == AxisRole::Gravity && moving_toward_feet(delta, gravity_dir))
        {
            let snap = snap_feet_to_surface(body, hit.block.aabb, gravity_dir);
            let _ = apply_bounded_resolution(kinematics, gravity_dir, snap);
            zero_axis_vel(kinematics, axis);
            if role == AxisRole::Gravity {
                ground.on_ground = true;
            }
            contacts.push(block_face_contact(
                body,
                hit.block,
                -gravity_dir,
                toi_fraction,
            ));
        } else if role == AxisRole::Gravity {
            let (push, push_normal) = axis_face_resolution(body, hit.block.aabb, axis);
            if apply_bounded_resolution(kinematics, gravity_dir, push) {
                zero_axis_vel(kinematics, axis);
                contacts.push(block_face_contact(
                    body,
                    hit.block,
                    push_normal,
                    toi_fraction,
                ));
            }
        } else {
            let immediate_contact = hit.time_of_impact <= 1.0e-5;
            let (body_min, body_max) = axis_span(body, axis);
            let (block_min, block_max) = axis_span(hit.block.aabb, axis);
            let overlap = (body_max.min(block_max) - body_min.max(block_min)).max(0.0);
            let body_beyond_block = (body_min + body_max) * 0.5 > (block_min + block_max) * 0.5;
            let moving_away_from_block = (body_beyond_block && delta_along > 0.0)
                || (!body_beyond_block && delta_along < 0.0);
            let grazing_overlap_moving_away =
                immediate_contact && overlap > 0.0 && moving_away_from_block;
            // Resolve the side penetration robustly via the shared helper: defer
            // to the gravity pass when the perpendicular exit is shorter —
            // crucially REGARDLESS of `immediate_contact`. A body sliding
            // PARALLEL just under a wide thin block (its head grazing the far
            // face) makes the swept cast return a spurious *non-immediate*
            // grazing hit; an immediate-only guard let that fall through to a
            // far-edge push, teleporting the body ~900px out of the room.
            // `None` => not a side contact to resolve here, so keep the swept
            // motion going.
            let depen = resolve_side_penetration(
                body,
                hit.block.aabb,
                axis,
                axis_component(world.size, axis),
            );
            if grazing_overlap_moving_away || depen.is_none() {
                kinematics.pos += axis_vec(axis, delta_along * (1.0 - toi_fraction));
            } else {
                let (d, normal_sign) = depen.expect("checked is_none above");
                kinematics.pos += axis_vec(axis, d);
                zero_axis_vel(kinematics, axis);
                apply_side_contact(wall, axis_vec(axis, normal_sign), gravity_dir);
                contacts.push(block_face_contact(
                    body,
                    hit.block,
                    axis_vec(axis, normal_sign),
                    toi_fraction,
                ));
            }
        }
    } else {
        kinematics.pos += axis_vec(axis, delta_along);
    }

    resolve_axis_repair(
        world,
        kinematics,
        ground,
        wall,
        axis,
        prev_feet_coord,
        drop_through,
        gravity_dir,
        contacts,
    );
}

/// Positional penetration repair for ONE world axis, role-aware — the merge of
/// the old `resolve_axis_clusters` (X) / `resolve_vertical_clusters` (Y) pair,
/// which had drifted: only the Y flavor owned OneWay landings, the nested-graze
/// skip, and grounding, so those semantics vanished whenever gravity rotated
/// onto X. Support and wall decisions are expressed in controlled-body terms:
/// feet/head along the gravity axis, side normals along the local side axis.
#[allow(clippy::too_many_arguments)]
fn resolve_axis_repair(
    world: &World,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    ground: &mut crate::body_clusters::BodyGroundState,
    wall: &mut crate::body_clusters::BodyWallState,
    axis: Axis,
    prev_feet_coord: f32,
    drop_through: bool,
    gravity_dir: Vec2,
    contacts: &mut Vec<Contact>,
) {
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
            && body_is_nested_along(aabb, block.aabb, axis)
        {
            continue;
        }
        match role {
            AxisRole::Gravity => {
                let on_support = matches!(block.kind, BlockKind::OneWay)
                    || body_on_support_side(aabb, block.aabb, gravity_dir);
                let (delta, normal) = if on_support {
                    (
                        snap_feet_to_surface(aabb, block.aabb, gravity_dir),
                        -gravity_dir,
                    )
                } else {
                    axis_face_resolution(aabb, block.aabb, axis)
                };
                if apply_bounded_resolution(kinematics, gravity_dir, delta) {
                    if on_support {
                        ground.on_ground = true;
                    }
                    zero_axis_vel(kinematics, axis);
                    contacts.push(block_face_contact(aabb, block, normal, 0.0));
                }
            }
            AxisRole::Side => {
                if let Some((d, normal_sign)) = resolve_side_penetration(
                    aabb,
                    block.aabb,
                    axis,
                    axis_component(world.size, axis),
                ) {
                    kinematics.pos += axis_vec(axis, d);
                    zero_axis_vel(kinematics, axis);
                    apply_side_contact(wall, axis_vec(axis, normal_sign), gravity_dir);
                    contacts.push(block_face_contact(
                        aabb,
                        block,
                        axis_vec(axis, normal_sign),
                        0.0,
                    ));
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
