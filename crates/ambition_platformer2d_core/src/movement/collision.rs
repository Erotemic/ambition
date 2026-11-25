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
    /// The lines this body carries are not obstacles to it.
    carrier: Option<u64>,
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
    carrier: Option<u64>,
) -> Option<AxisConstraintConflict> {
    let delta = axis_vec(axis, delta_along);
    let policy = BodyCollisionPolicy {
        body_mode,
        env_contact,
        carrier,
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

    // A grounded body's feet, for the seam rule below.
    let grounded_feet = ground.on_ground.then(|| kinematics.aabb().feet_coord(gravity_dir));
    let query = (|| {
        use crate::frame::{material_cast::MaterialCast, thread::{MaterialPrimitive, Refusal, RefusalReason}};
        let numeric = |fault| Refusal { path: vec![], link: None, reason: RefusalReason::Numeric(fault) };
        let domain = kinematics.canonical_box().map_err(numeric)?;
        let cast = MaterialCast::new(MaterialPrimitive {
            domain: &domain, to_world: kinematics.material_map().map_err(numeric)?,
        }, kinematics.pos, delta, 0.0..1.0).carried_by(policy.carrier);
        *admission = cast.motion_admission(*admission)?;
        let cells = cast.cells(*admission)?;
        let result = world.first_material_contact(&cells, |collider, _, hit| {
            let normal = hit.map.direction(hit.normal);
            let relative = hit.map.vector(delta).map_err(numeric)? - collider.displacement_at(hit.world_point, hit.world_time);
            if relative.dot(normal) >= -1.0e-7 { return Ok(false); }
            let gravity_pass = axis_role(axis, gravity_dir) == AxisRole::Gravity;
            Ok(match collider {
                crate::world::MaterialCollider::Block(block) => {
                    if policy.passes_through(block) { false }
                    else { match block.kind {
                        // Crossed from above only: feet that began inside the
                        // platform (a climber passing up through it) fall
                        // through it, as `one_way_landing_from_previous_feet`.
                        BlockKind::OneWay => gravity_pass && !drop_through && normal.dot(-gravity_dir) > 0.5
                            && prev_feet_coord <= block.aabb.head_coord(gravity_dir)
                                + crate::collision_semantics::ContactBands::of(kinematics.material).crossing
                                + (-block.velocity.dot(gravity_dir)).max(0.0),
                        BlockKind::BonkOnly => normal.dot(gravity_dir) > 0.5,
                        // A SEAM IS NOT A WALL. A body that walks up a line
                        // onto a block is put on the line, which can leave its
                        // feet a fraction below the block's top where the two
                        // meet (0.32 px measured at a 28° slope); the side of
                        // the block then stopped it at the top of the slope.
                        // A face whose top is no more than the edge slop above
                        // a grounded body's feet is walked onto, and the
                        // gravity repair stands the body on the top.
                        // A seam joins two surfaces: geometry, not a body
                        // band, so it does not convert with the body.
                        kind => is_full_collision_surface(kind) && !(!gravity_pass && grounded_feet.is_some_and(|feet|
                            block.aabb.head_coord(gravity_dir) >= feet - crate::collision_semantics::EDGE_OVERLAP_SLOP)),
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
        let surface_step = hit.collider.displacement_at(contact.world_point, contact.world_time);
        let remaining_surface = surface_step * (1.0 - contact.toi) as f32;
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
        let surface_velocity = match contact.map.inverse().and_then(|map| map.vector(surface_step)) {
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
        Vec2::ZERO, 1.0..1.0).carried_by(policy.carrier).cells(admission)?;
    let hits = world.material_axis_contacts(&cells, axis_vec(axis, 1.0))?;
    let role = axis_role(axis, gravity_dir);
    let mut accepted = Vec::new();
    for (collider, hit) in hits {
        let normal = hit.contact.normal;
        let support = normal.dot(-gravity_dir) > 0.5;
        let eligible = match collider {
            crate::world::MaterialCollider::Block(block) => !policy.passes_through(block) && match block.kind {
                BlockKind::OneWay => role == AxisRole::Gravity && !drop_through && support
                    && prev_feet_coord <= hit.contact.point.dot(gravity_dir) + crate::collision_semantics::ContactBands::of(kinematics.material).crossing,
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
    let policy = BodyCollisionPolicy { body_mode: clusters.body_mode, env_contact: clusters.env_contact, carrier: clusters.carrier };
    // Feet that rest ON a floor do not overlap it, and the hits are overlaps:
    // the body is asked CONTACT_SLOP further along gravity, the band the
    // resting-support rule allows (`surface_supports_body_at_rest`). Asked
    // where it stands, every body placed on its floor read airborne and
    // "landed" on its first tick, with a landing sound.
    let mut probe = *clusters.kinematics;
    probe.pos += gravity_dir.normalize_or_zero() * crate::collision_semantics::ContactBands::of(clusters.kinematics.material).contact;
    let hits = material_axis_hits(world, &probe, policy,
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
        let surface_velocity = match hit.contact.map.inverse().and_then(|map| map.vector(collider.displacement_at(hit.contact.world_point, hit.contact.world_time))) {
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
pub fn standing_on_one_way_aabb(world: &World, body: Aabb, gravity_dir: Vec2, bands: crate::collision_semantics::ContactBands) -> bool {
    world.blocks.in_region(crate::collision_semantics::support_region(body, gravity_dir, bands.contact)).any(|block| {
        matches!(block.kind, BlockKind::OneWay)
            && surface_supports_body_at_rest(block.kind, body, block.aabb, gravity_dir, false, bands)
    }) || crate::walkable_lines::standing_on_line(world, body, gravity_dir, bands).is_some()
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
    bands: crate::collision_semantics::ContactBands,
) -> bool {
    if margin <= 0.0 || facing == 0.0 {
        return false;
    }
    let gravity_dir = frame.down();
    let supported = |at: Aabb| {
        crate::collision_semantics::supporting_block(world, at, gravity_dir, false, bands).is_some()
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

/// HOW FAR ALONG THE FLOOR A BODY MAY STEP AND KEEP ITS MIDDLE OVER IT: the
/// side step `delta` (along `side`), cut short where the floor under the
/// body's middle ends. A grounded attack that stops at a ledge
/// (`InputState::holds_floor_edge`) is held here.
///
/// Its middle, not its box: support is any overlap
/// (`surface_supports_body_at_rest`), so the box would hang almost wholly
/// off. A platform fighter stops with the bottom of its body's middle on the
/// lip.
///
/// A body whose middle is already over no floor (on a slope this cannot see,
/// or teetering past the lip) is not held: the cut only ever refuses to carry
/// a supported middle off.
pub fn floor_held_step(world: &World, body: Aabb, down: Vec2, axis: Vec2, delta: f32, bands: crate::collision_semantics::ContactBands) -> f32 {
    if delta == 0.0 {
        return 0.0;
    }
    let centre = body.center();
    // A sliver one pixel across the body's middle, as tall as the body.
    let across = axis.abs();
    let half = (body.max - body.min).abs() * 0.5 * (Vec2::ONE - across) + across * 0.5;
    let sliver = |shift: f32| {
        let at = centre + axis * shift;
        Aabb { min: at - half, max: at + half }
    };
    let supported = |shift: f32| {
        let probe = sliver(shift);
        crate::collision_semantics::supporting_block(world, probe, down, false, bands).is_some()
            || crate::walkable_lines::standing_on_line(world, probe, down, bands).is_some()
    };
    if !supported(0.0) || supported(delta) {
        return delta;
    }
    // The last supported shift, to a tenth of a pixel.
    let (mut good, mut bad) = (0.0_f32, delta);
    while (bad - good).abs() > 0.1 {
        let mid = (good + bad) * 0.5;
        if supported(mid) { good = mid } else { bad = mid }
    }
    good
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
        .near(aabb)
        .into_iter()
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
    world.body_sweep_candidates(crate::Aabb::new(center - delta, half), delta).into_iter().any(|b| {
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
    world.blocks.near(aabb).into_iter().find_map(|b| match b.kind {
        BlockKind::Rebound { impulse } if aabb.strict_intersects(b.aabb) => Some(impulse),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::aabb_from_min_size;

    #[test]
    fn indexed_hazard_and_rebound_contacts_match_a_visible_ordered_scan() {
        let blocks = (0..100).map(|i| {
            let min = Vec2::new((i % 10) as f32 * 64.0 - 320.0, (i / 10) as f32 * 64.0 - 320.0);
            match i % 3 {
                0 => crate::Block::hazard("hazard", min, Vec2::splat(24.0)),
                1 => crate::Block::rebound("rebound", min, Vec2::splat(24.0), Vec2::new(i as f32, -100.0)),
                _ => crate::Block::solid("solid", min, Vec2::splat(24.0)),
            }
        }).collect();
        let mut world = World::new("indexed contacts", Vec2::splat(4096.0), Vec2::ZERO, blocks);
        world.blocks.near(crate::layered::UNBOUNDED);
        world.blocks.hide(0);
        world.blocks.hide(8);
        world.blocks.add(crate::Block::hazard("thin hazard", Vec2::ZERO, Vec2::new(0.01, 32.0)));
        for impulse in [Vec2::X, Vec2::Y] {
            world.blocks.add(crate::Block::rebound("overlapping rebound", Vec2::ZERO, Vec2::splat(32.0), impulse));
        }
        for i in 0..200 {
            let center = Vec2::new((i % 20) as f32 * 32.0 - 320.0, (i / 20) as f32 * 64.0 - 320.0);
            let half = Vec2::splat(if i % 2 == 0 { 1.0 } else { 16.0 });
            let body = crate::Aabb::new(center, half);
            let hazard = world.blocks.iter().any(|b| matches!(b.kind, BlockKind::Hazard) && body.strict_intersects(b.aabb));
            let rebound = world.blocks.iter().find_map(|b| match b.kind {
                BlockKind::Rebound { impulse } if body.strict_intersects(b.aabb) => Some(impulse),
                _ => None,
            });
            assert_eq!(touching_hazard_aabb(&world, body), hazard);
            assert_eq!(touching_rebound_aabb(&world, body), rebound);
            for delta in [Vec2::ZERO, Vec2::X * 1024.0, Vec2::Y * -1024.0, Vec2::splat(1024.0)] {
                let swept = world.blocks.iter().any(|b| matches!(b.kind, BlockKind::Hazard)
                    && interior_of(b.aabb).is_some_and(|inside| crate::cast::aabb_path_contacts(center, half, delta, inside)));
                assert_eq!(hazard_contact_on_path(&world, center, half, delta), swept, "center={center:?}, delta={delta:?}");
            }
        }
    }

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
                            0.0, feet, false, down, &mut Vec::new(), &mut None, &mut refusal, None);
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
            false, crate::collision_semantics::ContactBands::CANONICAL,
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
            false, crate::collision_semantics::ContactBands::CANONICAL,
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
            false, crate::collision_semantics::ContactBands::CANONICAL,
        ));
        assert!(!surface_supports_body_at_rest(
            BlockKind::OneWay,
            b,
            platform,
            Vec2::new(1.0, 0.0),
            true, crate::collision_semantics::ContactBands::CANONICAL,
        ));
    }
}
