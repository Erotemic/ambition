//! Generic kinematic body — gravity + axis-separated sweep against a `World`.
//!
//! Why this exists: the player's `movement` module owns a sophisticated
//! sweep with jump-buffer, dash, blink, climb, and other player-only
//! affordances. Enemies and NPCs only need a small subset (gravity,
//! ground/wall collision, optional drop-through), but the sandbox used
//! to ship hand-rolled `blocked` / `blocked_y` predicates that diverged
//! from player physics in subtle ways — most visibly, hostile NPCs and
//! chasing enemies could not drop through one-way platforms or fall off
//! ledges in the same situations the player could.
//!
//! [`KinematicBody`] + [`step_kinematic`] are the shared sweep both
//! enemies and NPCs go through. Authored player physics still lives
//! in `movement`, but it agrees with this primitive on the load-bearing
//! semantics:
//!
//! - `Solid` and `BlinkWall` always block both axes.
//! - `OneWay` blocks only on the current gravity/support axis, and only
//!   when the body's previous feet coordinate crossed the platform's
//!   anti-gravity face. Under normal gravity this is the historical
//!   "landing from above" rule; under side/up gravity it rotates with
//!   the controlled body's frame.
//! - `drop_through` set on a tick suppresses the OneWay support block
//!   so a chasing enemy can follow a controlled body who dropped through
//!   the same platform a frame earlier.
//! - `Hazard`, `PogoOrb`, and `Rebound` are visited by gameplay logic
//!   (damage, bounce, impulse) elsewhere; they are not collision blockers
//!   for kinematic bodies.
//!
//! When/if the player migrates to this primitive, the player's tuning
//! gains a few abilities-shaped fields and we delete the duplicate
//! sweep helpers in `movement`.

use ambition_engine_core::collision_semantics::{
    axis_role, block_face_contact, body_on_support_side, gravity_axis, is_contact_range_snap,
    is_full_collision_surface, is_solid_for_axis, moving_toward_feet,
    one_way_landing_from_previous_feet, perpendicular_overlap, snap_feet_to_surface,
    supporting_block, surface_supports_body_at_rest, Axis, AxisRole, Contact, MOTION_EPS,
};
use ambition_engine_core::Vec2;
use ambition_engine_core::{Aabb, AabbExt};
use ambition_engine_core::{BlockKind, World};

/// Per-tick configuration for [`step_kinematic`].
#[derive(Clone, Copy, Debug)]
pub struct KinematicTuning {
    pub gravity: f32,
    /// Maximum fall speed (pixels/sec), measured ALONG `gravity_dir`.
    pub max_fall_speed: f32,
    /// Unit gravity DIRECTION (cardinal): down `(0,1)`, up `(0,-1)`, or sideways
    /// `(±1,0)`. Gravity accelerates the body along this, and "ground" is a
    /// contact on this (feet) side — so actors fall the way the player does,
    /// including SIDEWAYS. (Supersedes the Y-only `gravity_sign`, which only
    /// handled down/up: the reason enemies/NPCs didn't fall under left/right
    /// gravity. Vertical gravity is byte-identical: `gravity_dir.y` is the old
    /// `gravity_sign`.)
    pub gravity_dir: Vec2,
}

/// Per-tick AI/control inputs to [`step_kinematic`].
#[derive(Clone, Copy, Debug, Default)]
pub struct KinematicInputs {
    /// Suppress the OneWay vertical block this tick so the body falls
    /// through the platform it is currently standing on. Mirrors the
    /// player's `drop_through_pressed` input.
    pub drop_through: bool,
}

/// A body that gravity pulls down and the world sweeps horizontally.
///
/// The shape is intentionally minimal: position, velocity, size,
/// `on_ground`, and `facing`. AI / brain code lives in callers (enemy
/// chase, NPC patrol, future RL agent inputs); this struct just owns
/// the axis-separated sweep.
#[derive(Clone, Copy, Debug)]
pub struct KinematicBody {
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub on_ground: bool,
    /// +1 right, -1 left. Updated by callers — this primitive does not
    /// flip facing, but it is a useful place for shared state.
    pub facing: f32,
}

impl KinematicBody {
    pub fn new(pos: Vec2, size: Vec2) -> Self {
        Self {
            pos,
            vel: Vec2::ZERO,
            size,
            on_ground: false,
            facing: 1.0,
        }
    }

    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.pos, self.size * 0.5)
    }
}

/// Apply one frame of physics: gravity, then independent X/Y sweeps.
///
/// Returns the new `on_ground` state in `body.on_ground`. Does not
/// touch `facing`; callers update it from their AI signal (chase
/// direction, patrol bound bounce, etc).
pub fn step_kinematic(
    body: &mut KinematicBody,
    world: &World,
    tuning: KinematicTuning,
    inputs: KinematicInputs,
    dt: f32,
) {
    step_kinematic_observed(body, world, tuning, inputs, dt, None);
}

/// [`step_kinematic`] with contact observability (fable review 2026-07-05
/// AJ10): every resolved world contact this step is pushed into `contacts`
/// (landing = feet contact with normal `-gravity_dir`, wall = the surface's
/// outward normal from the parry cast, rest = a support contact carrying the
/// support's `surface_velocity`). Pass `None` for the plain step — resolution
/// is IDENTICAL either way; the sink is pure observability.
pub fn step_kinematic_observed(
    body: &mut KinematicBody,
    world: &World,
    tuning: KinematicTuning,
    inputs: KinematicInputs,
    dt: f32,
    mut contacts: Option<&mut Vec<Contact>>,
) {
    let g = cardinal_gravity(tuning.gravity_dir);

    // 1. Gravity along the body's local down direction, capped along that same
    //    gravity axis. This is the free-body invariant the rest of the sweep
    //    consumes: the world axes are an implementation detail.
    let fall_before = body.vel.dot(g).max(0.0);
    let cap = tuning.max_fall_speed.max(fall_before);
    body.vel += tuning.gravity * g * dt;
    let along = body.vel.dot(g);
    if along > cap {
        body.vel -= (along - cap) * g;
    }

    // Capture the FEET edge before motion. One-way eligibility is a crossing
    // test from the previous feet coordinate to the support face; using bottom
    // only works under normal gravity.
    let prev_feet_coord = body.aabb().feet_coord(g);

    body.on_ground = false;

    // 2. Sweep in controlled-body-local order: local side first, local down
    //    second. For normal gravity this remains world X then world Y; for
    //    sideways gravity it becomes world Y then world X. This matches the
    //    controlled actor path and removes order-dependent C4 asymmetry.
    let gravity_axis = gravity_axis(g);
    let side_axis = gravity_axis.perpendicular();
    sweep_axis(
        body,
        world,
        side_axis,
        g,
        inputs.drop_through,
        prev_feet_coord,
        dt,
        contacts.as_deref_mut(),
    );
    sweep_axis(
        body,
        world,
        gravity_axis,
        g,
        inputs.drop_through,
        prev_feet_coord,
        dt,
        contacts.as_deref_mut(),
    );

    // 3. Resting support stabilization. Swept motion handles crossings; this
    //    handles bodies spawned, carried, or nudged into contact with a support.
    if let Some(support) = supporting_block(world, body.aabb(), g, inputs.drop_through) {
        let snap = snap_feet_to_surface(body.aabb(), support.aabb, g);
        // Only stabilize on a genuine resting-contact snap. A catastrophic
        // snap (deep penetration / far block matched in error) is left for
        // `resolve_penetration` to depenetrate the bounded way — snapping it
        // here would pushout-teleport the body out of the world. See
        // `is_contact_range_snap`.
        if is_contact_range_snap(snap, body.aabb()) {
            if snap.length_squared() > 0.0 {
                body.pos += snap;
            }
            clear_velocity_toward_feet(&mut body.vel, g);
            body.on_ground = true;

            // Emergent platform riding: any body resting on a moving support
            // rides the component perpendicular to gravity. The gravity-axis
            // component is already represented by support/contact resolution.
            let v = support.velocity;
            body.pos += v - v.dot(g) * g;
            if let Some(sink) = contacts.as_deref_mut() {
                sink.push(block_face_contact(body.aabb(), support, -g, 0.0));
            }
        }
    }

    resolve_penetration(body, world, g, contacts);
}

fn cardinal_gravity(gravity_dir: Vec2) -> Vec2 {
    if gravity_dir.x.abs() > gravity_dir.y.abs() {
        Vec2::new(gravity_dir.x.signum(), 0.0)
    } else if gravity_dir.y != 0.0 {
        Vec2::new(0.0, gravity_dir.y.signum())
    } else {
        Vec2::new(0.0, 1.0)
    }
}

fn axis_delta(axis: Axis, amount: f32) -> Vec2 {
    match axis {
        Axis::X => Vec2::new(amount, 0.0),
        Axis::Y => Vec2::new(0.0, amount),
    }
}

fn axis_component(v: Vec2, axis: Axis) -> f32 {
    match axis {
        Axis::X => v.x,
        Axis::Y => v.y,
    }
}

fn add_axis(pos: &mut Vec2, axis: Axis, amount: f32) {
    match axis {
        Axis::X => pos.x += amount,
        Axis::Y => pos.y += amount,
    }
}

fn clear_axis_velocity(vel: &mut Vec2, axis: Axis) {
    match axis {
        Axis::X => vel.x = 0.0,
        Axis::Y => vel.y = 0.0,
    }
}

fn clear_velocity_toward_feet(vel: &mut Vec2, gravity_dir: Vec2) {
    let toward_feet = vel.dot(gravity_dir);
    if toward_feet > 0.0 {
        *vel -= toward_feet * gravity_dir;
    }
}

fn sweep_axis(
    body: &mut KinematicBody,
    world: &World,
    axis: Axis,
    gravity_dir: Vec2,
    drop_through: bool,
    prev_feet_coord: f32,
    dt: f32,
    mut contacts: Option<&mut Vec<Contact>>,
) {
    let delta_amount = axis_component(body.vel, axis) * dt;
    if delta_amount.abs() <= MOTION_EPS {
        resolve_axis(body, world, axis, gravity_dir, drop_through, contacts);
        return;
    }

    let delta = axis_delta(axis, delta_amount);
    let start_body = body.aabb();
    // CC1: body-vs-world sweeps route through the one `cast` entry point.
    if let Some(hit) = ambition_engine_core::cast::body_sweep(world, start_body, delta, |block| {
        if !is_solid_for_axis(block.kind, axis, gravity_dir) {
            return false;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            return one_way_landing_from_previous_feet(
                start_body,
                block.aabb,
                delta,
                gravity_dir,
                drop_through,
                prev_feet_coord,
            );
        }
        // Pre-existing penetration is repaired by resolve_axis / resolve_penetration.
        !start_body.strict_intersects(block.aabb)
    }) {
        let toi = hit.time_of_impact.clamp(0.0, 1.0);
        add_axis(&mut body.pos, axis, delta_amount * toi);
        if matches!(hit.block.kind, BlockKind::OneWay)
            || (axis_role(axis, gravity_dir) == AxisRole::Gravity
                && moving_toward_feet(delta, gravity_dir))
        {
            // Already moved to the time-of-impact above, so the landing snap
            // is a small alignment; guard it anyway for uniformity (never
            // pushout-teleport on a feet-to-surface snap). See
            // `is_contact_range_snap`.
            let snap = snap_feet_to_surface(body.aabb(), hit.block.aabb, gravity_dir);
            if is_contact_range_snap(snap, body.aabb()) {
                body.pos += snap;
            }
            body.on_ground = true;
            clear_axis_velocity(&mut body.vel, axis);
            clear_velocity_toward_feet(&mut body.vel, gravity_dir);
            if let Some(sink) = contacts.as_deref_mut() {
                sink.push(block_face_contact(
                    body.aabb(),
                    hit.block,
                    -gravity_dir,
                    toi,
                ));
            }
        } else {
            clear_axis_velocity(&mut body.vel, axis);
            if let Some(sink) = contacts.as_deref_mut() {
                // The SURFACE outward normal: parry's `normal1` is the moving
                // shape's outward normal, so negate it; fall back to the
                // swept-axis face when the cast reported none (t=0 overlap).
                let normal = if hit.normal1.length_squared() > 0.5 {
                    -hit.normal1
                } else {
                    axis_delta(axis, -delta_amount.signum())
                };
                sink.push(block_face_contact(body.aabb(), hit.block, normal, toi));
            }
        }
    } else {
        add_axis(&mut body.pos, axis, delta_amount);
    }

    resolve_axis(body, world, axis, gravity_dir, drop_through, contacts);
}

fn axis_resolution(body: Aabb, block: Aabb, axis: Axis) -> Vec2 {
    match axis {
        Axis::X => {
            if body.center().x <= block.center().x {
                Vec2::new(block.left() - body.right(), 0.0)
            } else {
                Vec2::new(block.right() - body.left(), 0.0)
            }
        }
        Axis::Y => {
            if body.center().y <= block.center().y {
                Vec2::new(0.0, block.top() - body.bottom())
            } else {
                Vec2::new(0.0, block.bottom() - body.top())
            }
        }
    }
}

fn resolve_axis(
    body: &mut KinematicBody,
    world: &World,
    axis: Axis,
    gravity_dir: Vec2,
    drop_through: bool,
    mut contacts: Option<&mut Vec<Contact>>,
) {
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, axis, gravity_dir) {
            continue;
        }
        let aabb = body.aabb();
        if !aabb.strict_intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            if !surface_supports_body_at_rest(
                block.kind,
                aabb,
                block.aabb,
                gravity_dir,
                drop_through,
            ) {
                continue;
            }
            let snap = snap_feet_to_surface(aabb, block.aabb, gravity_dir);
            if !is_contact_range_snap(snap, aabb) {
                continue;
            }
            body.pos += snap;
            body.on_ground = true;
            clear_axis_velocity(&mut body.vel, axis);
            clear_velocity_toward_feet(&mut body.vel, gravity_dir);
            if let Some(sink) = contacts.as_deref_mut() {
                sink.push(block_face_contact(aabb, block, -gravity_dir, 0.0));
            }
        } else {
            let push = axis_resolution(aabb, block.aabb, axis);
            // A penetration push larger than the body's own half-extent is a
            // pushout-teleport, not a contact resolve: a body overlapping a
            // tall wall whose nearest in-axis exit is the wall's FAR face
            // (e.g. its top) gets flung out of the world. The perpendicular
            // axis's resolve handles the real, near exit (a side wall is
            // exited sideways, not over the top); never pushout-teleport.
            if !is_contact_range_snap(push, aabb) {
                continue;
            }
            body.pos += push;
            clear_axis_velocity(&mut body.vel, axis);
            if axis_role(axis, gravity_dir) == AxisRole::Gravity
                && body_on_support_side(aabb, block.aabb, gravity_dir)
            {
                body.on_ground = true;
                clear_velocity_toward_feet(&mut body.vel, gravity_dir);
            }
            if let Some(sink) = contacts.as_deref_mut() {
                let normal = push.normalize_or_zero();
                if normal != Vec2::ZERO {
                    sink.push(block_face_contact(aabb, block, normal, 0.0));
                }
            }
        }
    }
}

fn resolve_penetration(
    body: &mut KinematicBody,
    world: &World,
    gravity_dir: Vec2,
    mut contacts: Option<&mut Vec<Contact>>,
) {
    // Last-resort support-side depenetration. This is deliberately phrased in
    // feet/head coordinates rather than vertical top/bottom so it handles bodies
    // spawned into a support under any cardinal gravity.
    for block in &world.blocks {
        if !is_full_collision_surface(block.kind) {
            continue;
        }
        let aabb = body.aabb();
        if !aabb.strict_intersects(block.aabb)
            || !body_on_support_side(aabb, block.aabb, gravity_dir)
        {
            continue;
        }
        if !perpendicular_overlap(aabb, block.aabb, gravity_dir) {
            continue;
        }
        let snap = snap_feet_to_surface(aabb, block.aabb, gravity_dir);
        if is_contact_range_snap(snap, aabb) {
            body.pos += snap;
            body.on_ground = true;
            clear_velocity_toward_feet(&mut body.vel, gravity_dir);
            if let Some(sink) = contacts.as_deref_mut() {
                sink.push(block_face_contact(aabb, block, -gravity_dir, 0.0));
            }
            break;
        }
    }
}

#[cfg(test)]
mod tests;
