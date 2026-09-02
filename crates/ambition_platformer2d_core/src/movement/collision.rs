use crate::Vec2;
use crate::collision_semantics::{
    Axis, AxisConstraintConflict, AxisRole, Contact, ContactKind, ContactSource, axis_role,
    block_face_contact, body_on_support_side, is_contact_range_snap, is_full_collision_surface,
    is_solid_for_axis, moving_toward_feet, one_way_landing_from_previous_feet,
    snap_feet_to_surface, surface_supports_body_at_rest,
};
use crate::geometry::{Aabb, AabbExt};
use crate::world::{Block, BlockKind, World};

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

/// The outward normal of the SUPPORT face resolved on the gravity-role world
/// axis: the cardinal face the body's feet snapped onto. For cardinal gravity
/// this equals `-gravity_dir`; under an oblique frame the surface is still a
/// cardinal face and the contact must carry ITS normal (the `Contact` doc's
/// "outward normal of the SURFACE" contract).
fn support_face_normal(axis: Axis, gravity_dir: Vec2) -> Vec2 {
    axis_vec(axis, -axis_component(gravity_dir, axis).signum())
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

/// A SUBMERGED body passes through everything, because it is not in the world.
///
/// ⛔⛔ UNCONDITIONAL, AND THAT IS THE DIFFERENCE FROM THE CLIMB RULE BELOW.
/// A climbing body is still on the stage and only ignores the blocks its
/// climbable region overlaps — lose the region and the floor is a floor again.
/// There is no region under the stage to lose contact with, and a submerged
/// body that could be stopped by geometry would be a fighter wedged inside the
/// floor she went under, with nothing to push her out.
///
/// ⛔ HAZARDS TOO. The climb rule deliberately keeps hazards solid so a ladder
/// cannot be used to walk through a spike; a submerged body is intangible for
/// the same beats it is invisible, and a hazard that could touch her would
/// contradict the `Invuln` window the move authors over the same span.
fn block_passable_while_submerged(body_mode: &crate::body_clusters::BodyModeState) -> bool {
    matches!(
        body_mode.body_mode,
        crate::player_state::BodyMode::Submerged
    )
}

/// The blocks this body passes through this step — decided once, consulted by
/// BOTH collision stages.
///
/// ⛔⛔ THE TWO STAGES HAD DIFFERENT IDEAS OF WHAT `Submerged` MEANS. The
/// continuous sweep took `BodyModeState` and knew a submerged body is not in the
/// world; [`resolve_axis_repair`] took neither it nor anything equivalent, so a
/// body that legitimately travelled THROUGH a block was then found overlapping
/// it and pushed back out — by the second half of the same function call. The
/// mode that exists to make a body intangible was intangible to one stage only.
///
/// ⚠ CLIMBING HAD THE SAME GAP, and it is the reason this is a shared value
/// rather than another `if Submerged` in the repair. A climbing body passes
/// through exactly the blocks its climbable region overlaps; the repair could
/// not tell which those were either, so it shoved a climber out of the wall she
/// was on. One policy, asked the same question by both stages, is what makes
/// "passable" mean one thing.
#[derive(Clone, Copy)]
pub(super) struct BodyCollisionPolicy<'a> {
    body_mode: &'a crate::body_clusters::BodyModeState,
    env_contact: &'a crate::body_clusters::BodyEnvironmentContact,
}

impl BodyCollisionPolicy<'_> {
    /// Whether `block` is not there, for this body, this step.
    fn passes_through(&self, block: &crate::world::Block) -> bool {
        block_passable_while_submerged(self.body_mode)
            || block_passable_during_climb_clusters(self.body_mode, self.env_contact, block)
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

/// The body's span ALONG the swept axis is nested inside the block's span: the contact can only
/// be on a face perpendicular to the sweep (a side graze while sweeping the gravity axis),
/// never a support/head face — so the gravity-axis pass must not resolve it.
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

/// Role behavior:
/// - Gravity axis: OneWay landing gate, nested side-graze rejection, feet
///   snap + `on_ground` when moving toward the feet, head-face push otherwise.
/// - Side axis: guarded side resolution ([`resolve_side_penetration`]:
///   defer / world-bounds / no-pushout) with grazing-motion continuation, and
///   wall contact armed in the body's LOCAL frame via [`apply_side_contact`]
///   (the old X path stored the raw world sign, breaking cling under up
///   gravity).
///
/// Falls back to [`resolve_axis_repair`] for stacked contacts or pre-existing
/// penetrations — and returns that pass's verdict: `Some` when the solids
/// claiming this axis this step admit NO position at all (see
/// [`AxisConstraintConflict`]; the consequence of a crush is the owner's, not
/// the kernel's).
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
) -> Option<AxisConstraintConflict> {
    let role = axis_role(axis, gravity_dir);
    let delta = axis_vec(axis, delta_along);
    // ONE policy for both stages below. See [`BodyCollisionPolicy`] for what the
    // repair used to do without it.
    let policy = BodyCollisionPolicy {
        body_mode,
        env_contact,
    };
    if delta_along.abs() <= 1.0e-5 {
        return resolve_axis_repair(
            world,
            kinematics,
            ground,
            wall,
            policy,
            axis,
            prev_feet_coord,
            drop_through,
            gravity_dir,
            contacts,
        );
    }

    let start_body = kinematics.aabb_oriented(gravity_dir);
    if let Some(hit) = world.first_body_sweep(start_body, delta, |block| {
        if !is_solid_for_axis(block.kind, axis, gravity_dir) {
            return false;
        }
        if policy.passes_through(block) {
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
        // the MIRROR, and without it a `BonkOnly` block is solid from every
        // side — which is the invisible floor it exists to stop being. It
        // blocks a head coming up into it and nothing else.
        if matches!(block.kind, BlockKind::BonkOnly) {
            return crate::collision_semantics::bonk_strike_from_head(
                start_body,
                block.aabb,
                delta,
                gravity_dir,
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
        // and the arm was never load-bearing. Its whole stated job was
        // "not the feet-snap arm", which the condition below already refuses on
        // its own terms: a `BonkOnly` hit exists only when a head is RISING into
        // the block (`bonk_strike_from_head` is the sweep filter), so
        // `moving_toward_feet` is false and the kind is not `OneWay`. Deleting
        // the arm is what makes its comment true.
        if matches!(hit.block.kind, BlockKind::OneWay)
            || (role == AxisRole::Gravity && moving_toward_feet(delta, gravity_dir))
        {
            let snap = snap_feet_to_surface(body, hit.block.aabb, gravity_dir);
            let _ = apply_bounded_resolution(kinematics, gravity_dir, snap);
            let support_normal = support_face_normal(axis, gravity_dir);
            // BEFORE the zero below destroys it — the whole reason the contact
            // carries it (see `Contact::impact_speed`).
            let impact = crate::collision_semantics::closing_speed(kinematics.vel, support_normal);
            zero_axis_vel(kinematics, axis);
            if role == AxisRole::Gravity {
                ground.on_ground = true;
            }
            contacts.push(block_face_contact(
                body,
                hit.block,
                support_normal,
                toi_fraction,
                ContactKind::Support,
                impact,
            ));
        } else if role == AxisRole::Gravity {
            let (push, push_normal) = axis_face_resolution(body, hit.block.aabb, axis);
            if apply_bounded_resolution(kinematics, gravity_dir, push) {
                let impact = crate::collision_semantics::closing_speed(kinematics.vel, push_normal);
                zero_axis_vel(kinematics, axis);
                // The other end of the axis `on_ground` reports. Published so
                // the floor game can decide a ceiling tech at the TOP of the
                // next tick, one phase before contacts exist.
                ground.head_contact = true;
                contacts.push(block_face_contact(
                    body,
                    hit.block,
                    push_normal,
                    toi_fraction,
                    ContactKind::Head,
                    impact,
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
                let side_normal = axis_vec(axis, normal_sign);
                // ⭐ THE WALL HIT, measured. This is the value nothing
                // downstream could recover: the zero on the next line is what
                // destroys it, and a body already stopped against a wall looks
                // the same however hard it arrived.
                let impact = crate::collision_semantics::closing_speed(kinematics.vel, side_normal);
                zero_axis_vel(kinematics, axis);
                apply_side_contact(wall, side_normal, gravity_dir);
                contacts.push(block_face_contact(
                    body,
                    hit.block,
                    side_normal,
                    toi_fraction,
                    ContactKind::Side,
                    impact,
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
        policy,
        axis,
        prev_feet_coord,
        drop_through,
        gravity_dir,
        contacts,
    )
}

/// One solid's CLAIM on the body's centre coordinate along the repaired axis.
///
/// A claim is not a position to write — it is a BOUND. `delta_along` is its component on the
/// repaired axis, and its SIGN says which bound this is: a positive correction demands `centre
/// >= bound`, a negative one demands `centre <= bound`.
///
/// Keeping the delta rather than only the bound is not bookkeeping: applying it
/// reproduces the old arithmetic exactly, where `pos + (centre + delta - centre)`
/// would not for large coordinates.
struct AxisClaim<'a> {
    delta: Vec2,
    delta_along: f32,
    /// `centre + delta_along` — the coordinate this claim will accept.
    bound: f32,
    normal: Vec2,
    kind: ContactKind,
    on_support: bool,
    block: &'a Block,
}

/// Resolve penetration along one world axis using order-independent interval constraints.
///
/// Each intersecting solid contributes a lower or upper center bound. A feasible
/// interval resolves to its binding edge; an empty interval returns
/// [`AxisConstraintConflict`] without moving the body. Claims beyond
/// [`is_contact_range_snap`] are excluded before interval construction, so an
/// accepted repair is always one an individual contact was allowed to request.
#[allow(clippy::too_many_arguments)]
fn resolve_axis_repair(
    world: &World,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    ground: &mut crate::body_clusters::BodyGroundState,
    wall: &mut crate::body_clusters::BodyWallState,
    policy: BodyCollisionPolicy<'_>,
    axis: Axis,
    prev_feet_coord: f32,
    drop_through: bool,
    gravity_dir: Vec2,
    contacts: &mut Vec<Contact>,
) -> Option<AxisConstraintConflict> {
    let role = axis_role(axis, gravity_dir);
    let aabb = kinematics.aabb_oriented(gravity_dir);
    let centre = axis_component(aabb.center(), axis);
    // The deepest claim in each direction: `toward_pos` demands the largest
    // minimum, `toward_neg` the smallest maximum.
    let mut toward_pos: Option<AxisClaim> = None;
    let mut toward_neg: Option<AxisClaim> = None;
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, axis, gravity_dir) || !aabb.strict_intersects(block.aabb)
        {
            continue;
        }
        // ⭐ THE SAME QUESTION THE SWEEP ASKED. A block this body passed through
        // contributes no repair claim, or the repair undoes the traversal the
        // sweep just allowed.
        if policy.passes_through(block) {
            continue;
        }
        // `BonkOnly` reacts to rising-head contact but never supplies support/repair geometry.
        if crate::collision_semantics::blocks_only_a_rising_head(block.kind) {
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
        let claim = match role {
            AxisRole::Gravity => {
                let on_support = matches!(block.kind, BlockKind::OneWay)
                    || body_on_support_side(aabb, block.aabb, gravity_dir);
                let (delta, normal) = if on_support {
                    (
                        snap_feet_to_surface(aabb, block.aabb, gravity_dir),
                        support_face_normal(axis, gravity_dir),
                    )
                } else {
                    axis_face_resolution(aabb, block.aabb, axis)
                };
                // The no-artificial-pushout refusal, unchanged in meaning and in
                // effect: `apply_bounded_resolution` asked exactly this of the
                // same snap against a body whose half-extent does not depend on
                // where its centre is. A refused block claims nothing.
                if !is_contact_range_snap(delta, aabb) {
                    continue;
                }
                let delta_along = axis_component(delta, axis);
                AxisClaim {
                    delta,
                    delta_along,
                    bound: centre + delta_along,
                    normal,
                    kind: if on_support {
                        ContactKind::Support
                    } else {
                        ContactKind::Head
                    },
                    on_support,
                    block,
                }
            }
            AxisRole::Side => {
                // `resolve_side_penetration` carries its own refusals (defer to
                // the gravity pass, never out of the world, never a pushout
                // teleport); `None` is "this block claims nothing here".
                let Some((d, normal_sign)) = resolve_side_penetration(
                    aabb,
                    block.aabb,
                    axis,
                    axis_component(world.size, axis),
                ) else {
                    continue;
                };
                AxisClaim {
                    delta: axis_vec(axis, d),
                    delta_along: d,
                    bound: centre + d,
                    normal: axis_vec(axis, normal_sign),
                    kind: ContactKind::Side,
                    on_support: false,
                    block,
                }
            }
        };
        // A correction of exactly zero demands nothing — the face is already
        // flush — so it never becomes a bound.
        if claim.delta_along > 0.0 {
            if toward_pos
                .as_ref()
                .is_none_or(|deepest| claim.delta_along > deepest.delta_along)
            {
                toward_pos = Some(claim);
            }
        } else if claim.delta_along < 0.0
            && toward_neg
                .as_ref()
                .is_none_or(|deepest| claim.delta_along < deepest.delta_along)
        {
            toward_neg = Some(claim);
        }
    }

    // Claims in BOTH directions == an empty interval: the +axis claimant's
    // minimum sits above the -axis claimant's maximum, and no position on this
    // axis satisfies both. What that MEANS to the body is the owner's call.
    let conflict = match (&toward_pos, &toward_neg) {
        (Some(demands_min), Some(demands_max)) => Some(AxisConstraintConflict {
            axis,
            min_center: demands_min.bound,
            max_center: demands_max.bound,
            min_source: ContactSource::Block {
                kind: demands_min.block.kind,
                id: demands_min.block.id.clone(),
            },
            max_source: ContactSource::Block {
                kind: demands_max.block.kind,
                id: demands_max.block.id.clone(),
            },
        }),
        _ => None,
    };
    if conflict.is_none() {
        // Feasible: at most one direction claimed, so its deepest claim IS the
        // binding edge of the interval.
        if let Some(binding) = toward_pos.as_ref().or(toward_neg.as_ref()) {
            kinematics.pos += binding.delta;
        }
    }
    for claim in [toward_pos.as_ref(), toward_neg.as_ref()]
        .into_iter()
        .flatten()
    {
        if claim.on_support {
            ground.on_ground = true;
        }
        let impact = crate::collision_semantics::closing_speed(kinematics.vel, claim.normal);
        zero_axis_vel(kinematics, axis);
        if claim.kind == ContactKind::Side {
            apply_side_contact(wall, claim.normal, gravity_dir);
        }
        contacts.push(block_face_contact(
            aabb,
            claim.block,
            claim.normal,
            0.0,
            claim.kind,
            impact,
        ));
    }
    conflict
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

/// IS THIS BODY STANDING ON THE BRINK — supported where it is, but not if it
/// leaned `margin` of its own half-width further along `facing`?
///
/// ⭐ A FACT, NOT A RULE. Collision is untouched: this asks the SAME
/// `surface_supports_body_at_rest` question the landing sweep asks, at a probe
/// shifted toward the edge the body is facing. Control and animation read the
/// answer; nothing here moves a body or refuses it a step.
///
/// `margin <= 0.0` can never teeter, which is what every body did before this
/// existed.
pub fn teetering_at_edge(
    world: &World,
    body: Aabb,
    frame: crate::MotionFrame,
    facing: f32,
    margin: f32,
) -> bool {
    if margin <= 0.0 || facing == 0.0 {
        return false;
    }
    let gravity_dir = frame.down();
    let supported = |at: Aabb| {
        world.blocks.iter().any(|block| {
            surface_supports_body_at_rest(block.kind, at, block.aabb, gravity_dir, false)
        })
    };
    if !supported(body) {
        return false;
    }
    // ⛔⛔ WHAT MATTERS IS WHERE THE PROBE'S TRAILING EDGE SITS, because
    // support is decided by `perpendicular_overlap` — ANY lateral overlap
    // counts. Measured: a body hanging 14px past a platform with 15px of
    // half-width is still fully supported, so a probe that still touches the
    // platform anywhere reports "supported" however far its far side reaches.
    //
    // ⭐ So the question is asked as "is the outermost `margin` of my own
    // footprint over air" — the leading foot. A first attempt leaned the whole
    // body by `half_width * margin` and found no edge anywhere, because that
    // shift was too small to lift the probe's trailing edge clear.
    //
    // ⚠ AND A WHOLE-BODY SHIFT BY THE SAME `cut` IS EQUIVALENT, which a poison
    // proved: both put the trailing edge at `min + cut`, and the far side never
    // matters. The foot is written this way because it says what it means.
    let outward = frame.side() * facing.signum();
    let width = (body.max - body.min).dot(outward).abs();
    let cut = width * (1.0 - margin.clamp(0.0, 1.0));
    let toward = outward * cut;
    let foot = Aabb {
        min: body.min + toward.max(Vec2::ZERO),
        max: body.max + toward.min(Vec2::ZERO),
    };
    !supported(foot)
}

/// Tile-set-only hazard touch test. Cluster-aware callers
/// pass `BodyKinematics::aabb()` directly without building an
/// `ae::Player`.
///
/// ⚠ ENDPOINT ONLY. A body fast enough to cross a thin hazard between two
/// samples is not detected here. Callers that have the tick's canonical
/// `SweepSample` must use [`hazard_contact_on_path`]; this remains the honest
/// answer for a body that has no sample.
pub fn touching_hazard_aabb(world: &World, aabb: crate::Aabb) -> bool {
    world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, BlockKind::Hazard) && aabb.strict_intersects(b.aabb))
}

/// Did the body CONTACT a hazard anywhere along the segment it actually
/// travelled this tick?
///
/// `center`/`half` describe the body at the segment's END and `delta` is the
/// segment (`SweepSample::delta()`), so the path tested is `curr - delta ->
/// curr` — the simulation phase's own record. It is deliberately NOT
/// reconstructed from `vel * dt`: velocity at phase entry does not describe a
/// step a collision resolve shortened, and a second motion model that disagrees
/// with the kernel's is how a hazard fires on a frame the body never entered.
///
/// ⚠ THIS FUNCTION ALONE IS NOT AT PARITY WITH THE DISCRETE TEST, and must not
/// be used as if it were. It queries the hazard's INTERIOR, so a body overlapping
/// a hazard by less than [`HAZARD_SURFACE_EPSILON`] is not a hit here. Parity is
/// a property of the PAIR — `kernel::touching_hazard` runs the endpoint arm,
/// which is exactly `strict_intersects` against the real AABB, and only then
/// this one — so every hit the discrete test would find is still a hit, and what
/// this adds is genuine tunnels rather than surface contact.
pub fn hazard_contact_on_path(world: &World, center: Vec2, half: Vec2, delta: Vec2) -> bool {
    world.blocks.iter().any(|b| {
        matches!(b.kind, BlockKind::Hazard)
            && interior_of(b.aabb)
                .is_some_and(|inside| crate::cast::aabb_path_contacts(center, half, delta, inside))
    })
}

/// How far a swept query is pulled inside a hazard's faces before it counts.
///
/// ⛔⛔ WITHOUT THIS, WALKING UP TO A GAP KILLS YOU. `aabb_path_contacts` is a
/// CONTACT test — `sweep_hit` reports boxes that touch — while the discrete
/// `strict_intersects` it claims parity with reports only boxes that OVERLAP.
/// The two agree everywhere except on a shared face, and an authored hazard gap
/// shares its top face with the floor either side of it by construction.
///
/// Measured 2026-09-02 in `blink_run`: a body walking the start floor at
/// `feet y = 128.0` against a hazard whose top face is also `y = 128.0` reported
/// a swept hit the instant its leading edge passed `x = 288.0` — endpoint AABB
/// `max = (288.87, 128.0)` versus hazard `min = (288.0, 128.0)`. It never
/// penetrated anything; it walked along a surface. `reject_grazing_contact`
/// inside `sweep_hit` does not cover a coplanar slide.
///
/// ⛔ THREE DIFFERENT QUESTIONS OWN THREE DIFFERENT CASES. Do not add a fourth
/// epsilon; work out which of these a new case belongs to.
///
/// - the DISCRETE arm (`touching_hazard_aabb`) owns the ENDPOINT, and carries no
///   epsilon at all — it is `strict_intersects` against the hazard's real AABB.
///   A body that ends the tick even `1e-4` inside a hazard is caught there, so
///   this inset cannot let a real penetration through the back door;
/// - `reject_grazing_contact` (inside `sweep_hit`) owns motion PARALLEL to the
///   face it contacts — a slide that never approaches the surface;
/// - this epsilon owns contact at ZERO penetration, where the approach IS
///   head-on on the other axis, so grazing rejection correctly does not apply.
///
/// ⇒ what the inset can miss is exactly one thing: a body that dips no more than
/// `1e-3` into a hazard MID-tick and is back out by the tick's end. At a
/// thousandth of a pixel that is not a traversal, and the endpoint arm still
/// answers for where the body actually finished.
const HAZARD_SURFACE_EPSILON: f32 = 1.0e-3;

/// A hazard's INTERIOR — the volume a body has to be inside to have gone
/// through it, rather than along it. `None` for a hazard thinner than the
/// epsilon, which has no interior to tunnel.
fn interior_of(hazard: Aabb) -> Option<Aabb> {
    let inset = Vec2::splat(HAZARD_SURFACE_EPSILON);
    let inside = Aabb {
        min: hazard.min + inset,
        max: hazard.max - inset,
    };
    (inside.min.x < inside.max.x && inside.min.y < inside.max.y).then_some(inside)
}

/// Rebound impulse lookup for a body AABB.
pub fn touching_rebound_aabb(world: &World, aabb: crate::Aabb) -> Option<Vec2> {
    world.blocks.iter().find_map(|b| match b.kind {
        BlockKind::Rebound { impulse } if aabb.strict_intersects(b.aabb) => Some(impulse),
        _ => None,
    })
}

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
