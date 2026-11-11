use crate::Vec2;
use crate::collision_semantics::{
    Axis, AxisConstraintConflict, AxisRole, Contact, ContactKind, ContactSource, axis_role,
    is_contact_range_snap, is_full_collision_surface, surface_supports_body_at_rest,
};
use crate::geometry::{Aabb, AabbExt};
use crate::world::{BlockKind, World};

pub(crate) fn apply_side_contact(
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

/// A submerged body has no block contacts, including hazards.
fn block_passable_while_submerged(body_mode: &crate::body_clusters::BodyModeState) -> bool {
    matches!(
        body_mode.body_mode,
        crate::player_state::BodyMode::Submerged
    )
}

/// Movement and repair use one passability policy. A climb permits only
/// blocks in the active climbable region. It does not permit hazards.
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

fn zero_axis_vel(kinematics: &mut crate::body_clusters::BodyKinematics, axis: Axis) {
    match axis {
        Axis::X => kinematics.vel.x = 0.0,
        Axis::Y => kinematics.vel.y = 0.0,
    }
}

/// Sweep and repair canonical material. Conflicting correction bounds report
/// a crush without choosing an occupied position.
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
    admission: &mut Option<crate::frame::thread::MotionAdmission<'_>>,
    refusal: &mut Option<crate::frame::thread::Refusal>,
) -> Option<AxisConstraintConflict> {
    let delta = axis_vec(axis, delta_along);
    let policy = BodyCollisionPolicy {
        body_mode,
        env_contact,
    };
    // Correct gravity penetration before transverse motion. Keep the imposed
    // velocity and contact state until the movement sweep resolves them.
    if axis_role(axis, gravity_dir) == AxisRole::Side {
        let conflict = resolve_material_axis_repair(world, kinematics, ground, wall, policy,
            crate::collision_semantics::gravity_axis(gravity_dir), prev_feet_coord,
            drop_through, gravity_dir, contacts, *admission, refusal, RepairStage::BeforeSweep);
        if conflict.is_some() || refusal.is_some() { return conflict; }
    }
    if delta_along == 0.0 {
        return resolve_material_axis_repair(
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
            *admission,
            refusal,
            RepairStage::AfterSweep,
        );
    }

    let query = (|| {
        use crate::frame::{material_cast::MaterialCast, thread::{MaterialPrimitive, Refusal, RefusalReason}};
        let numeric = |fault| Refusal { path: vec![], link: None, reason: RefusalReason::Numeric(fault) };
        let domain = kinematics.canonical_box().map_err(numeric)?;
        let cast = MaterialCast::new(MaterialPrimitive {
            domain: &domain, to_world: kinematics.material_map().map_err(numeric)?,
        }, kinematics.pos, delta, 0.0..1.0);
        *admission = cast.motion_admission(*admission)?;
        let cells = cast.cells(*admission)?;
        let result = world.first_material_contact(&cells, |collider, _, hit| {
            let normal = hit.map.direction(hit.normal);
            let relative = hit.map.vector(delta).map_err(numeric)? - collider.displacement();
            if relative.dot(normal) >= -1.0e-7 { return Ok(false); }
            let gravity_pass = axis_role(axis, gravity_dir) == AxisRole::Gravity;
            Ok(match collider {
                crate::world::MaterialCollider::Block(block) => {
                    if policy.passes_through(block) { false }
                    else { match block.kind {
                        BlockKind::OneWay => gravity_pass && !drop_through && normal.dot(-gravity_dir) > 0.5,
                        BlockKind::BonkOnly => normal.dot(gravity_dir) > 0.5,
                        kind => is_full_collision_surface(kind),
                    } }
                }
                crate::world::MaterialCollider::Chain { chain, segment, .. } => {
                    chain.kind == crate::world::SurfaceKind::Solid && chain.closed
                        || gravity_pass && !drop_through && normal.dot(chain.normal_at_time(segment, hit.world_time).map_err(numeric)?) > 0.5
                }
            })
        });
        result
    })();
    let hit = match query {
        Ok(hit) => hit,
        Err(reason) => { *refusal = Some(reason); return None; }
    };
    if let Some(hit) = hit {
        let contact = hit.contact;
        let step = delta.as_dvec2() * contact.toi;
        let normal = contact.normal;
        let position = kinematics.pos.as_dvec2() + step;
        kinematics.pos = position.as_vec2();
        for component in 0..2 {
            if f64::from(normal[component]) * (f64::from(kinematics.pos[component]) - position[component]) < 0.0 {
                kinematics.pos[component] = if normal[component] > 0.0 {
                    kinematics.pos[component].next_up()
                } else { kinematics.pos[component].next_down() };
            }
        }
        let physical_normal = contact.map.direction(normal);
        let remaining_surface = hit.collider.displacement() * (1.0 - contact.toi) as f32;
        let carry = physical_normal * remaining_surface.dot(physical_normal).max(0.0);
        let carry = match contact.map.inverse().and_then(|inverse| inverse.vector(carry)) {
            Ok(carry) => carry,
            Err(fault) => {
                *refusal = Some(crate::frame::thread::Refusal { path: contact.path.clone(), link: None,
                    reason: crate::frame::thread::RefusalReason::Numeric(fault) });
                return None;
            }
        };
        kinematics.pos += axis_vec(axis, axis_component(carry, axis));
        let impact = crate::collision_semantics::closing_speed(kinematics.vel, normal);
        zero_axis_vel(kinematics, axis);
        let kind = if normal.dot(-gravity_dir) > 0.5 {
            ground.on_ground = true;
            ContactKind::Support
        } else if normal.dot(gravity_dir) > 0.5 {
            ground.head_contact = true;
            ContactKind::Head
        } else {
            apply_side_contact(wall, normal, gravity_dir);
            ContactKind::Side
        };
        let surface_velocity = match contact.map.inverse().and_then(|map| map.vector(hit.collider.displacement())) {
            Ok(velocity) => velocity,
            Err(fault) => {
                *refusal = Some(crate::frame::thread::Refusal { path: contact.path.clone(), link: None,
                    reason: crate::frame::thread::RefusalReason::Numeric(fault) });
                return None;
            }
        };
        contacts.push(Contact { kind, point: contact.point, normal, toi: contact.toi as f32,
            surface_velocity, impact_speed: impact, involuntary: false, source: hit.collider.source() });
    } else {
        kinematics.pos += delta;
    }
    resolve_material_axis_repair(world, kinematics, ground, wall, policy, axis,
        prev_feet_coord, drop_through, gravity_dir, contacts, *admission, refusal, RepairStage::AfterSweep)
}

#[derive(Clone, Copy)]
enum RepairStage { BeforeSweep, AfterSweep }

#[allow(clippy::too_many_arguments)]
fn material_axis_hits<'w>(
    world: &'w World, kinematics: &crate::BodyKinematics, policy: BodyCollisionPolicy<'_>,
    axis: Axis, prev_feet_coord: f32, drop_through: bool, gravity_dir: Vec2,
    admission: Option<crate::frame::thread::MotionAdmission<'_>>,
) -> Result<Vec<(crate::world::MaterialCollider<'w>, crate::frame::material_cast::PolygonAxisContact)>, crate::frame::thread::Refusal> {
    use crate::frame::{material_cast::MaterialCast, thread::{MaterialPrimitive, Refusal, RefusalReason}};
    let numeric = |fault| Refusal { path: vec![], link: None, reason: RefusalReason::Numeric(fault) };
    let domain = kinematics.canonical_box().map_err(numeric)?;
    let cells = MaterialCast::new(MaterialPrimitive { domain: &domain,
        to_world: kinematics.material_map().map_err(numeric)? }, kinematics.pos,
        Vec2::ZERO, 1.0..1.0).cells(admission)?;
    let hits = world.material_axis_contacts(&cells, axis_vec(axis, 1.0))?;
    let role = axis_role(axis, gravity_dir);
    let mut accepted = Vec::new();
    for (collider, hit) in hits {
        let normal = hit.contact.normal;
        let support = normal.dot(-gravity_dir) > 0.5;
        let eligible = match collider {
            crate::world::MaterialCollider::Block(block) => !policy.passes_through(block) && match block.kind {
                BlockKind::OneWay => role == AxisRole::Gravity && !drop_through && support
                    && prev_feet_coord <= hit.contact.point.dot(gravity_dir) + crate::collision_semantics::ONE_WAY_CROSSING_SLOP,
                BlockKind::BonkOnly => false,
                kind => is_full_collision_surface(kind),
            },
            crate::world::MaterialCollider::Chain { chain, segment, .. } => {
                if chain.closed && chain.kind == crate::world::SurfaceKind::Solid { true }
                else {
                    let face = match chain.normal_at_time(segment, 1.0) {
                        Ok(face) => face,
                        Err(fault) => return Err(numeric(fault)),
                    };
                    // A static sample cannot board an open one-way line.
                    // Its crossing or walking policy admits new support.
                    hit.displacement == 0.0 && role == AxisRole::Gravity && !drop_through && support
                        && hit.contact.map.direction(normal).dot(face) > 0.5
                }
            }
        };
        let gravity_contact = hit.minimum_normal.dot(gravity_dir).abs()
            >= hit.minimum_normal.dot(gravity_dir.perp()).abs();
        if !eligible || (role == AxisRole::Gravity) != gravity_contact { continue; }
        let delta = axis_vec(axis, hit.displacement as f32);
        if !is_contact_range_snap(delta, kinematics.aabb()) { continue; }
        accepted.push((collider, hit));
    }
    Ok(accepted)
}

/// Sample support with the same geometry and policy as material repair.
pub(super) fn material_support_at_rest(
    world: &World, clusters: &crate::body_clusters::BodyClustersMut<'_>, gravity_dir: Vec2,
    admission: Option<crate::frame::thread::MotionAdmission<'_>>,
) -> Result<bool, crate::frame::thread::Refusal> {
    let policy = BodyCollisionPolicy { body_mode: clusters.body_mode, env_contact: clusters.env_contact };
    let hits = material_axis_hits(world, clusters.kinematics, policy,
        crate::collision_semantics::gravity_axis(gravity_dir), clusters.kinematics.aabb().feet_coord(gravity_dir),
        false, gravity_dir, admission)?;
    Ok(hits.iter().any(|(_, hit)| hit.contact.normal.dot(-gravity_dir) > 0.5))
}

/// Repair and support use the same material cells as continuous contact.
#[allow(clippy::too_many_arguments)]
fn resolve_material_axis_repair(
    world: &World,
    kinematics: &mut crate::BodyKinematics,
    ground: &mut crate::body_clusters::BodyGroundState,
    wall: &mut crate::body_clusters::BodyWallState,
    policy: BodyCollisionPolicy<'_>,
    axis: Axis,
    prev_feet_coord: f32,
    drop_through: bool,
    gravity_dir: Vec2,
    contacts: &mut Vec<Contact>,
    admission: Option<crate::frame::thread::MotionAdmission<'_>>,
    refusal: &mut Option<crate::frame::thread::Refusal>,
    stage: RepairStage,
) -> Option<AxisConstraintConflict> {
    use crate::frame::thread::{Refusal, RefusalReason};
    let numeric = |fault| Refusal { path: vec![], link: None, reason: RefusalReason::Numeric(fault) };
    let mut hits = match material_axis_hits(world, kinematics, policy, axis, prev_feet_coord,
        drop_through, gravity_dir, admission) {
        Ok(hits) => hits,
        Err(reason) => { *refusal = Some(reason); return None; }
    };
    hits.sort_by(|(a, x), (b, y)| x.displacement.total_cmp(&y.displacement)
        .then(a.cmp_identity(*b)).then(x.contact.path.cmp(&y.contact.path)));
    let centre = axis_component(kinematics.pos, axis);
    let mut lower: Option<(f64, ContactSource)> = None;
    let mut upper: Option<(f64, ContactSource)> = None;
    let mut accepted = Vec::new();
    for (collider, hit) in hits {
        let bound = f64::from(centre) + hit.displacement;
        if hit.displacement > 0.0 && lower.as_ref().is_none_or(|(old, _)| bound > *old) {
            lower = Some((bound, collider.source()));
        }
        if hit.displacement < 0.0 && upper.as_ref().is_none_or(|(old, _)| bound < *old) {
            upper = Some((bound, collider.source()));
        }
        accepted.push((collider, hit));
    }
    let conflict = match (&lower, &upper) {
        (Some((min, min_source)), Some((max, max_source))) if min > max => Some(AxisConstraintConflict {
            axis, min_center: *min as f32, max_center: *max as f32,
            min_source: min_source.clone(), max_source: max_source.clone(),
        }),
        _ => None,
    };
    if conflict.is_none() {
        if let Some((bound, _)) = lower.as_ref().or(upper.as_ref()) {
            let sign = (*bound - f64::from(centre)).signum();
            let mut coordinate = *bound as f32;
            if sign * (f64::from(coordinate) - *bound) < 0.0 {
                coordinate = if sign > 0.0 { coordinate.next_up() } else { coordinate.next_down() };
            }
            match axis { Axis::X => kinematics.pos.x = coordinate, Axis::Y => kinematics.pos.y = coordinate }
        }
    }
    if matches!(stage, RepairStage::BeforeSweep) { return conflict; }
    for (collider, hit) in accepted {
        let normal = hit.contact.normal;
        let kind = if normal.dot(-gravity_dir) > 0.5 {
            ground.on_ground = true;
            ContactKind::Support
        } else if normal.dot(gravity_dir) > 0.5 {
            ground.head_contact = true;
            ContactKind::Head
        } else {
            apply_side_contact(wall, normal, gravity_dir);
            ContactKind::Side
        };
        let surface_velocity = match hit.contact.map.inverse().and_then(|map| map.vector(collider.displacement())) {
            Ok(velocity) => velocity,
            Err(fault) => { *refusal = Some(numeric(fault)); return None; }
        };
        let impact_speed = crate::collision_semantics::closing_speed(kinematics.vel, normal);
        zero_axis_vel(kinematics, axis);
        contacts.push(Contact { kind, point: hit.contact.point, normal, toi: 0.0,
            surface_velocity, impact_speed, involuntary: false, source: collider.source() });
    }
    conflict
}

/// AABB-only variant of [`standing_on_one_way`]. Cluster-aware
/// callers pass the oriented body AABB directly. Gravity-relative: the body
/// rests on the one-way's anti-gravity support face, so drop-through detection
/// flips with gravity like the landing sweep already does. A walkable line
/// (`walkable_lines`) is one-way too, so standing on one counts.
pub fn standing_on_one_way_aabb(world: &World, body: Aabb, gravity_dir: Vec2) -> bool {
    world.blocks.iter().any(|block| {
        matches!(block.kind, BlockKind::OneWay)
            && surface_supports_body_at_rest(block.kind, body, block.aabb, gravity_dir, false)
    }) || crate::walkable_lines::standing_on_line(world, body, gravity_dir).is_some()
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

    #[test]
    fn contact_positions_keep_canonical_material_outside_each_face() {
        let block = Aabb::new(Vec2::splat(300.0), Vec2::splat(40.0));
        for normal in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
            for down in [Vec2::X, Vec2::Y, Vec2::new(0.6, 0.8)] {
                for scale in [0.5, 1.0, 2.0] {
                    for parity in [-1.0, 1.0] {
                        let mut body = crate::BodyKinematics {
                            canonical_size: Vec2::new(30.0, 48.0_f32.next_up()),
                            material: crate::frame::BodyMaterial::new(scale,
                                bevy_math::Mat2::from_cols(Vec2::X, Vec2::Y * parity)).unwrap(),
                            ..Default::default()
                        };
                        let half = body.projected_half_extent(normal.as_dvec2()).unwrap();
                        body.pos = block.center() + normal * (40.0 + half as f32);
                        let world = World::new("rounding", Vec2::splat(1000.0), Vec2::ZERO,
                            vec![crate::Block::solid("face", block.min, block.max - block.min)]);
                        let axis = if normal.x != 0.0 { Axis::X } else { Axis::Y };
                        let mut refusal = None;
                        let feet = body.aabb().feet_coord(down);
                        sweep_player_axis_clusters(&world, &mut body, &mut Default::default(),
                            &mut Default::default(), &Default::default(), &Default::default(), axis,
                            0.0, feet, false, down, &mut Vec::new(), &mut None, &mut refusal);
                        assert!(refusal.is_none(), "{refusal:?}");
                        let result = body.pos;
                        let face = normal.as_dvec2().dot(block.center().as_dvec2()) + 40.0;
                        let edge = normal.as_dvec2().dot(result.as_dvec2()) - half;
                        assert!(edge >= face, "normal={normal}, down={down}, scale={scale}: {edge} < {face}");
                        let axis = if normal.x != 0.0 { Axis::X } else { Axis::Y };
                        let coordinate = axis_component(result, axis);
                        let inward = if axis_component(normal, axis) > 0.0 { coordinate.next_down() } else { coordinate.next_up() };
                        let inward_edge = f64::from(axis_component(normal, axis)) * f64::from(inward) - half;
                        assert!(inward_edge < face, "the adjacent inward position must penetrate");
                    }
                }
            }
        }
    }

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
