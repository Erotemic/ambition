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

use ambition_engine_core::Vec2;
use ambition_engine_core::{Aabb, AabbExt};
use ambition_engine_core::{Block, BlockKind, World};

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
    );
    sweep_axis(
        body,
        world,
        gravity_axis,
        g,
        inputs.drop_through,
        prev_feet_coord,
        dt,
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
        }
    }

    resolve_penetration(body, world, g);
}

const CONTACT_SLOP: f32 = 4.0;
const ONE_WAY_CROSSING_SLOP: f32 = 8.0;
const MOTION_EPS: f32 = 1.0e-5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

impl Axis {
    fn perpendicular(self) -> Self {
        match self {
            Axis::X => Axis::Y,
            Axis::Y => Axis::X,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AxisRole {
    Gravity,
    Side,
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

fn gravity_axis(gravity_dir: Vec2) -> Axis {
    if gravity_dir.x.abs() > gravity_dir.y.abs() {
        Axis::X
    } else {
        Axis::Y
    }
}

fn axis_role(axis: Axis, gravity_dir: Vec2) -> AxisRole {
    if axis == gravity_axis(gravity_dir) {
        AxisRole::Gravity
    } else {
        AxisRole::Side
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

fn moving_toward_feet(delta: Vec2, gravity_dir: Vec2) -> bool {
    delta.dot(gravity_dir) > MOTION_EPS
}

fn is_support_surface(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Solid | BlockKind::BlinkWall { .. } | BlockKind::OneWay
    )
}

fn is_full_collision_surface(kind: BlockKind) -> bool {
    matches!(kind, BlockKind::Solid | BlockKind::BlinkWall { .. })
}

fn is_solid_for_axis(kind: BlockKind, axis: Axis, gravity_dir: Vec2) -> bool {
    match kind {
        BlockKind::Solid | BlockKind::BlinkWall { .. } => true,
        BlockKind::OneWay => axis_role(axis, gravity_dir) == AxisRole::Gravity,
        BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => false,
    }
}

fn perpendicular_overlap(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> bool {
    if gravity_dir.x.abs() > gravity_dir.y.abs() {
        body.bottom() > surface.top() && body.top() < surface.bottom()
    } else {
        body.right() > surface.left() && body.left() < surface.right()
    }
}

fn one_way_landing_from_previous_feet(
    body: Aabb,
    block: Aabb,
    delta: Vec2,
    gravity_dir: Vec2,
    drop_through: bool,
    prev_feet_coord: f32,
) -> bool {
    if drop_through {
        return false;
    }
    moving_toward_feet(delta, gravity_dir)
        && prev_feet_coord <= block.head_coord(gravity_dir) + ONE_WAY_CROSSING_SLOP
        && perpendicular_overlap(body, block, gravity_dir)
}

fn support_face_separation(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> f32 {
    body.feet_coord(gravity_dir) - surface.head_coord(gravity_dir)
}

fn body_on_support_side(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> bool {
    body.center().dot(gravity_dir) <= surface.center().dot(gravity_dir)
}

fn surface_supports_body_at_rest(
    kind: BlockKind,
    body: Aabb,
    surface: Aabb,
    gravity_dir: Vec2,
    drop_through: bool,
) -> bool {
    if !is_support_surface(kind) || !perpendicular_overlap(body, surface, gravity_dir) {
        return false;
    }
    if matches!(kind, BlockKind::OneWay) && drop_through {
        return false;
    }
    body_on_support_side(body, surface, gravity_dir)
        && support_face_separation(body, surface, gravity_dir).abs() <= CONTACT_SLOP
}

fn supporting_block<'a>(
    world: &'a World,
    body: Aabb,
    gravity_dir: Vec2,
    drop_through: bool,
) -> Option<&'a Block> {
    world.blocks.iter().find(|block| {
        surface_supports_body_at_rest(block.kind, body, block.aabb, gravity_dir, drop_through)
    })
}

fn snap_feet_to_surface(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> Vec2 {
    gravity_dir * (surface.head_coord(gravity_dir) - body.feet_coord(gravity_dir))
}

/// True when a feet-to-surface resting snap is a genuine small contact
/// correction rather than a pushout-teleport. A legitimate "resting on a
/// support" snap moves the body at most a contact-slop distance; a snap
/// larger than the body's own half-extent means the matched "support" is a
/// block the body is deeply penetrating (or matched in error), and snapping
/// feet to its far surface would fling the body clear across — or out of —
/// the world.
///
/// This is the mockingbird "flies above the arena" bug: a gravity-free,
/// oversized boss jammed into a tall wall block was treated as resting on it
/// and snapped its feet (bottom edge) up to the block's top surface at y=0,
/// teleporting it to y=-half in a single tick. Deep overlap is
/// `resolve_penetration`'s bounded job; resting snaps must never pushout
/// (per the engine's no-artificial-pushout invariant).
fn is_contact_range_snap(snap: Vec2, body: Aabb) -> bool {
    snap.length() <= body.half_size().length()
}

fn sweep_axis(
    body: &mut KinematicBody,
    world: &World,
    axis: Axis,
    gravity_dir: Vec2,
    drop_through: bool,
    prev_feet_coord: f32,
    dt: f32,
) {
    let delta_amount = axis_component(body.vel, axis) * dt;
    if delta_amount.abs() <= MOTION_EPS {
        resolve_axis(body, world, axis, gravity_dir, drop_through);
        return;
    }

    let delta = axis_delta(axis, delta_amount);
    let start_body = body.aabb();
    if let Some(hit) = world.first_body_sweep(start_body, delta, |block| {
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
            let snap = snap_feet_to_surface(body.aabb(), hit.block.aabb, gravity_dir);
            body.pos += snap;
            body.on_ground = true;
            clear_axis_velocity(&mut body.vel, axis);
            clear_velocity_toward_feet(&mut body.vel, gravity_dir);
        } else {
            clear_axis_velocity(&mut body.vel, axis);
        }
    } else {
        add_axis(&mut body.pos, axis, delta_amount);
    }

    resolve_axis(body, world, axis, gravity_dir, drop_through);
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
        }
    }
}

fn resolve_penetration(body: &mut KinematicBody, world: &World, gravity_dir: Vec2) {
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
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with(blocks: Vec<Block>) -> World {
        World {
            name: "kinematic-test".into(),
            size: Vec2::new(800.0, 600.0),
            spawn: Vec2::new(0.0, 0.0),
            blocks,
            water_regions: Vec::new(),
            climbable_regions: Vec::new(),
        }
    }

    fn body(pos: Vec2) -> KinematicBody {
        KinematicBody::new(pos, Vec2::new(28.0, 46.0))
    }

    fn tuning() -> KinematicTuning {
        KinematicTuning {
            gravity: 1450.0,
            max_fall_speed: 760.0,
            gravity_dir: Vec2::new(0.0, 1.0),
        }
    }

    /// Regression for the mockingbird "flies above the arena" OOB
    /// (2026-06-21, caught by the actor OOB trace): a gravity-free body
    /// deeply overlapping a solid whose nearest in-axis exit face is far
    /// away must NOT be pushout-teleported across / out of the world by a
    /// single resolution step. Depenetration stays bounded (≤ the body's
    /// half-extent); the body's own velocity carries it out at the near
    /// face over subsequent frames. The bug snapped the boss's feet to a
    /// block face at the world's top edge, flinging it to y = -half in one
    /// tick.
    #[test]
    fn deeply_embedded_body_is_not_pushout_teleported() {
        let world = World {
            name: "embed".into(),
            size: Vec2::new(800.0, 600.0),
            spawn: Vec2::ZERO,
            blocks: vec![Block::solid(
                "slab",
                Vec2::new(200.0, 100.0),
                Vec2::new(400.0, 400.0),
            )],
            water_regions: Vec::new(),
            climbable_regions: Vec::new(),
        };
        let start = Vec2::new(400.0, 300.0);
        let mut body = KinematicBody::new(start, Vec2::new(100.0, 100.0));
        let tuning = KinematicTuning {
            gravity: 0.0,
            max_fall_speed: 0.0,
            gravity_dir: Vec2::new(0.0, 1.0),
        };
        step_kinematic(&mut body, &world, tuning, KinematicInputs::default(), 1.0 / 60.0);

        let moved = (body.pos - start).length();
        let cap = body.aabb().half_size().length();
        assert!(
            moved <= cap + 1.0,
            "embedded body was pushout-teleported {moved:.1}px (cap {cap:.1}px); \
             a single resolution step must stay bounded"
        );
        assert!(
            body.pos.y > 0.0 && body.pos.y < world.size.y,
            "body must stay inside the world; got y={}",
            body.pos.y
        );
    }

    #[derive(Clone, Copy, Debug)]
    struct ConformanceArm {
        name: &'static str,
        dir: Vec2,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct KinematicSample {
        pos: Vec2,
        vel: Vec2,
        on_ground: bool,
    }

    const CONFORMANCE_CENTER: Vec2 = Vec2::new(400.0, 300.0);
    const CONFORMANCE_ARMS: [ConformanceArm; 4] = [
        ConformanceArm {
            name: "down",
            dir: Vec2::new(0.0, 1.0),
        },
        ConformanceArm {
            name: "right",
            dir: Vec2::new(1.0, 0.0),
        },
        ConformanceArm {
            name: "up",
            dir: Vec2::new(0.0, -1.0),
        },
        ConformanceArm {
            name: "left",
            dir: Vec2::new(-1.0, 0.0),
        },
    ];

    fn conf_frame(dir: Vec2) -> ambition_engine_core::AccelerationFrame {
        ambition_engine_core::AccelerationFrame::new(dir)
    }

    fn conf_world_from_local(dir: Vec2, local: Vec2) -> Vec2 {
        let f = conf_frame(dir);
        CONFORMANCE_CENTER + f.side * local.x + f.down * local.y
    }

    fn conf_local_vec(dir: Vec2, world: Vec2) -> Vec2 {
        let f = conf_frame(dir);
        Vec2::new(world.dot(f.side), world.dot(f.down))
    }

    fn conf_local_pos(dir: Vec2, world: Vec2) -> Vec2 {
        conf_local_vec(dir, world - CONFORMANCE_CENTER)
    }

    fn conf_tuning(dir: Vec2) -> KinematicTuning {
        KinematicTuning {
            gravity: 1450.0,
            max_fall_speed: 760.0,
            gravity_dir: dir,
        }
    }

    fn conf_block_solid(name: &'static str, dir: Vec2, local_min: Vec2, local_size: Vec2) -> Block {
        let f = conf_frame(dir);
        let world_center = conf_world_from_local(dir, local_min + local_size * 0.5);
        let world_half = f.to_world_half(local_size * 0.5);
        Block::solid(name, world_center - world_half, world_half * 2.0)
    }

    fn conf_block_one_way(
        name: &'static str,
        dir: Vec2,
        local_min: Vec2,
        local_size: Vec2,
    ) -> Block {
        let f = conf_frame(dir);
        let world_center = conf_world_from_local(dir, local_min + local_size * 0.5);
        let world_half = f.to_world_half(local_size * 0.5);
        Block::one_way(name, world_center - world_half, world_half * 2.0)
    }

    fn square_body_at_local(dir: Vec2, local_pos: Vec2) -> KinematicBody {
        KinematicBody::new(conf_world_from_local(dir, local_pos), Vec2::splat(32.0))
    }

    fn conf_sample(dir: Vec2, body: &KinematicBody) -> KinematicSample {
        KinematicSample {
            pos: conf_local_pos(dir, body.pos),
            vel: conf_local_vec(dir, body.vel),
            on_ground: body.on_ground,
        }
    }

    fn assert_sample_close(label: &str, expected: KinematicSample, actual: KinematicSample) {
        let pos_diff = actual.pos - expected.pos;
        let vel_diff = actual.vel - expected.vel;
        assert!(
            pos_diff.x.abs() <= 2.0 && pos_diff.y.abs() <= 2.0,
            "{label} local pos: got {:?}, expected {:?}, diff {:?}",
            actual.pos,
            expected.pos,
            pos_diff
        );
        assert!(
            vel_diff.x.abs() <= 4.0 && vel_diff.y.abs() <= 4.0,
            "{label} local vel: got {:?}, expected {:?}, diff {:?}",
            actual.vel,
            expected.vel,
            vel_diff
        );
        assert_eq!(
            actual.on_ground, expected.on_ground,
            "{label} on_ground should be gravity-relative"
        );
    }

    fn assert_trace_c4_symmetric(
        name: &str,
        make_world: impl Fn(Vec2) -> World,
        make_body: impl Fn(Vec2) -> KinematicBody,
        drive: impl Fn(&mut KinematicBody, &World, KinematicTuning, usize),
        ticks: usize,
    ) {
        let mut reference = Vec::new();
        for (idx, arm) in CONFORMANCE_ARMS.iter().enumerate() {
            let world = make_world(arm.dir);
            let mut body = make_body(arm.dir);
            let tuning = conf_tuning(arm.dir);
            let mut trace = Vec::new();
            for tick in 0..ticks {
                drive(&mut body, &world, tuning, tick);
                trace.push(conf_sample(arm.dir, &body));
            }
            if idx == 0 {
                reference = trace;
            } else {
                for (tick, (expected, actual)) in reference.iter().zip(trace.iter()).enumerate() {
                    assert_sample_close(
                        &format!("{name} / {} arm tick {tick}", arm.name),
                        *expected,
                        *actual,
                    );
                }
            }
        }
    }

    #[test]
    fn frame_conformance_solid_support_is_c4_symmetric() {
        // A body falling onto a solid support should produce the same local trace
        // no matter which world axis gravity currently occupies.
        assert_trace_c4_symmetric(
            "solid support",
            |dir| {
                world_with(vec![conf_block_solid(
                    "support",
                    dir,
                    Vec2::new(-160.0, 160.0),
                    Vec2::new(320.0, 32.0),
                )])
            },
            |dir| square_body_at_local(dir, Vec2::new(0.0, 40.0)),
            |body, world, tuning, _tick| {
                step_kinematic(body, world, tuning, KinematicInputs::default(), 1.0 / 60.0)
            },
            48,
        );
    }

    #[test]
    fn frame_conformance_side_wall_is_not_ground() {
        // Moving into the local side face is a wall in every frame, not support.
        assert_trace_c4_symmetric(
            "side wall",
            |dir| {
                world_with(vec![conf_block_solid(
                    "wall",
                    dir,
                    Vec2::new(120.0, -80.0),
                    Vec2::new(32.0, 240.0),
                )])
            },
            |dir| {
                let mut b = square_body_at_local(dir, Vec2::new(0.0, 0.0));
                b.vel = conf_frame(dir).to_world(Vec2::new(260.0, 0.0));
                b
            },
            |body, world, tuning, _tick| {
                step_kinematic(
                    body,
                    world,
                    KinematicTuning {
                        gravity: 0.0,
                        ..tuning
                    },
                    KinematicInputs::default(),
                    1.0 / 60.0,
                )
            },
            32,
        );
    }

    #[test]
    fn frame_conformance_one_way_drop_through_is_c4_symmetric() {
        assert_trace_c4_symmetric(
            "one-way drop-through",
            |dir| {
                world_with(vec![
                    conf_block_one_way(
                        "one_way",
                        dir,
                        Vec2::new(-120.0, 120.0),
                        Vec2::new(240.0, 14.0),
                    ),
                    conf_block_solid(
                        "catcher",
                        dir,
                        Vec2::new(-160.0, 300.0),
                        Vec2::new(320.0, 32.0),
                    ),
                ])
            },
            |dir| square_body_at_local(dir, Vec2::new(0.0, 40.0)),
            |body, world, tuning, tick| {
                let inputs = if tick >= 30 {
                    KinematicInputs { drop_through: true }
                } else {
                    KinematicInputs::default()
                };
                step_kinematic(body, world, tuning, inputs, 1.0 / 60.0)
            },
            80,
        );
    }

    #[test]
    fn frame_conformance_moving_support_carries_along_local_side() {
        assert_trace_c4_symmetric(
            "moving support",
            |dir| {
                let mut support = conf_block_solid(
                    "moving_support",
                    dir,
                    Vec2::new(-160.0, 120.0),
                    Vec2::new(320.0, 32.0),
                );
                support.velocity = conf_frame(dir).to_world(Vec2::new(3.0, 0.0));
                world_with(vec![support])
            },
            |dir| square_body_at_local(dir, Vec2::new(0.0, 40.0)),
            |body, world, tuning, _tick| {
                step_kinematic(body, world, tuning, KinematicInputs::default(), 1.0 / 60.0)
            },
            54,
        );
    }

    #[test]
    fn flipped_gravity_makes_a_body_rise_and_land_on_a_ceiling() {
        // A ceiling block above the body; flipped gravity pulls the body UP onto
        // it and it registers as grounded (standing on the ceiling).
        let world = world_with(vec![Block::solid(
            "ceiling",
            Vec2::new(0.0, 0.0),
            Vec2::new(200.0, 32.0),
        )]);
        let mut b = body(Vec2::new(50.0, 300.0));
        let mut tuning = tuning();
        tuning.gravity_dir = Vec2::new(0.0, -1.0); // up
        for _ in 0..120 {
            step_kinematic(
                &mut b,
                &world,
                tuning,
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(
            b.pos.y < 300.0,
            "flipped gravity should pull the body up, got y={}",
            b.pos.y
        );
        assert!(
            b.on_ground,
            "the body should stand on the ceiling under flipped gravity"
        );
    }

    #[test]
    fn sideways_gravity_makes_a_body_fall_into_and_land_on_a_wall() {
        // A wall on the RIGHT; right-pointing gravity pulls the body into it and
        // it registers as grounded (standing on the wall). This is the enemy/NPC
        // bug — under the old Y-only `gravity_sign` a sideways-gravity body never
        // fell toward the wall at all.
        let world = world_with(vec![Block::solid(
            "right_wall",
            Vec2::new(400.0, -400.0),
            Vec2::new(40.0, 1200.0),
        )]);
        let mut b = body(Vec2::new(100.0, 50.0));
        let mut tuning = tuning();
        tuning.gravity_dir = Vec2::new(1.0, 0.0); // right
        let start_x = b.pos.x;
        for _ in 0..180 {
            step_kinematic(
                &mut b,
                &world,
                tuning,
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(
            b.pos.x > start_x + 100.0,
            "right gravity should pull the body toward the wall, got x={} (start {start_x})",
            b.pos.x
        );
        assert!(
            b.on_ground,
            "the body should land on (be grounded against) the wall it fell into",
        );
        assert!(
            b.pos.x <= 400.0,
            "the body should stop at the wall's left face, got x={}",
            b.pos.x
        );
    }

    #[test]
    fn gravity_caps_a_normal_fall_at_terminal_velocity() {
        // No floor: a body falling under gravity should accelerate UP TO the
        // terminal velocity and sit there (the equilibrium), never exceeding it.
        let world = world_with(vec![]);
        let mut b = body(Vec2::new(50.0, 0.0));
        for _ in 0..600 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(
            (b.vel.y - tuning().max_fall_speed).abs() < 1.0,
            "a normal fall should settle at terminal velocity {}, got {}",
            tuning().max_fall_speed,
            b.vel.y
        );
    }

    #[test]
    fn a_fling_above_terminal_is_preserved_not_braked() {
        // A body already moving faster than terminal (a portal fling) must NOT be
        // decelerated by the fall cap — gravity is an equilibrium it accelerates
        // toward, not a brake. The over-cap speed persists (no air drag on the
        // fall axis), so momentum carries through.
        let world = world_with(vec![]);
        let mut b = body(Vec2::new(50.0, 0.0));
        let fling = tuning().max_fall_speed * 2.0;
        b.vel.y = fling;
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
        assert!(
            b.vel.y >= fling,
            "an over-terminal fling ({fling}) should be preserved, got {}",
            b.vel.y
        );
    }

    #[test]
    fn lands_on_solid() {
        // Body falls and stops on a Solid floor.
        let world = world_with(vec![Block::solid(
            "floor",
            Vec2::new(0.0, 100.0),
            Vec2::new(200.0, 32.0),
        )]);
        let mut b = body(Vec2::new(50.0, 0.0));
        for _ in 0..30 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(b.on_ground, "expected to land on solid floor");
        assert!(b.vel.y.abs() < 0.01, "vel.y reset on landing");
    }

    #[test]
    fn lands_on_one_way_from_above() {
        // OneWay platform behaves like a floor when the body is
        // descending from above.
        let world = world_with(vec![Block::one_way(
            "platform",
            Vec2::new(0.0, 100.0),
            Vec2::new(200.0, 16.0),
        )]);
        let mut b = body(Vec2::new(50.0, 0.0));
        for _ in 0..30 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(
            b.on_ground,
            "expected to land on one-way platform from above"
        );
    }

    #[test]
    fn drop_through_passes_one_way() {
        // Same scene, but drop_through=true → no landing.
        let world = world_with(vec![Block::one_way(
            "platform",
            Vec2::new(0.0, 100.0),
            Vec2::new(200.0, 16.0),
        )]);
        let mut b = body(Vec2::new(50.0, 50.0));
        // First, settle on the platform.
        for _ in 0..20 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(b.on_ground, "precondition: must be on the platform");
        // Now drop through. Past the platform's bottom (y=116 in
        // top-left coords) is the success condition; y=160ish after
        // 20 frames of free-fall is well clear.
        let drop = KinematicInputs { drop_through: true };
        for _ in 0..20 {
            step_kinematic(&mut b, &world, tuning(), drop, 1.0 / 60.0);
        }
        assert!(
            b.pos.y - b.size.y * 0.5 > 116.0,
            "drop_through should clear the platform's bottom edge; body top y={}",
            b.pos.y - b.size.y * 0.5
        );
        assert!(!b.on_ground, "should not be grounded mid-fall");
    }

    #[test]
    fn drop_through_does_not_pass_solid() {
        // Drop-through is a OneWay-only affordance — Solid still blocks.
        let world = world_with(vec![Block::solid(
            "floor",
            Vec2::new(0.0, 100.0),
            Vec2::new(200.0, 32.0),
        )]);
        let mut b = body(Vec2::new(50.0, 50.0));
        let drop = KinematicInputs { drop_through: true };
        for _ in 0..40 {
            step_kinematic(&mut b, &world, tuning(), drop, 1.0 / 60.0);
        }
        assert!(b.on_ground, "Solid must still catch the body");
    }

    #[test]
    fn walks_off_ledge_falls() {
        // Solid ledge that ends at x=100. Body starts on the ledge,
        // walks right past the edge — should fall once it's no longer
        // overlapping the ledge horizontally.
        let world = world_with(vec![Block::solid(
            "ledge",
            Vec2::new(0.0, 100.0),
            Vec2::new(100.0, 32.0),
        )]);
        let mut b = body(Vec2::new(60.0, 50.0));
        // Settle on the ledge.
        for _ in 0..20 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(b.on_ground, "precondition: on ledge");
        // Walk right past the edge.
        b.vel.x = 200.0;
        for _ in 0..30 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(
            b.pos.x > 110.0,
            "must clear the ledge horizontally; x={}",
            b.pos.x
        );
        assert!(!b.on_ground, "should be airborne after clearing the edge");
        assert!(b.vel.y > 0.0, "should be falling");
    }

    #[test]
    fn a_body_rides_a_horizontally_moving_platform() {
        // A solid floor carrying a rightward per-frame velocity. ANY body resting on
        // it is carried right by that velocity — emergent riding, no per-actor flag.
        let mut platform = Block::solid("platform", Vec2::new(0.0, 100.0), Vec2::new(400.0, 32.0));
        platform.velocity = Vec2::new(3.0, 0.0); // 3 px/frame to the right
        let world = world_with(vec![platform]);
        let mut b = body(Vec2::new(50.0, 50.0));
        for _ in 0..30 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        assert!(b.on_ground, "precondition: resting on the platform");
        let x_before = b.pos.x;
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
        assert!(
            (b.pos.x - (x_before + 3.0)).abs() < 1e-3,
            "body should ride +3px right with the platform, got dx={}",
            b.pos.x - x_before
        );
    }

    #[test]
    fn a_body_does_not_ride_static_geometry() {
        // velocity ZERO (the static default) → no carry. Standing on normal ground is
        // byte-identical to before riding existed.
        let world = world_with(vec![Block::solid(
            "floor",
            Vec2::new(0.0, 100.0),
            Vec2::new(200.0, 32.0),
        )]);
        let mut b = body(Vec2::new(50.0, 50.0));
        for _ in 0..30 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
        }
        let x_before = b.pos.x;
        step_kinematic(
            &mut b,
            &world,
            tuning(),
            KinematicInputs::default(),
            1.0 / 60.0,
        );
        assert!(
            (b.pos.x - x_before).abs() < 1e-3,
            "a body must NOT drift on static ground, got dx={}",
            b.pos.x - x_before
        );
    }

    #[test]
    fn rising_through_one_way_does_not_get_stuck() {
        // OneWay should never block upward motion. Body starts below
        // the platform with negative vel.y (jumping up).
        let world = world_with(vec![Block::one_way(
            "platform",
            Vec2::new(0.0, 50.0),
            Vec2::new(200.0, 16.0),
        )]);
        let mut b = body(Vec2::new(50.0, 200.0));
        b.vel.y = -800.0;
        // Step a few frames; gravity will reduce vel.y but the body
        // should not be pinned by the one-way platform on the way up.
        let mut min_y = b.pos.y;
        for _ in 0..15 {
            step_kinematic(
                &mut b,
                &world,
                tuning(),
                KinematicInputs::default(),
                1.0 / 60.0,
            );
            if b.pos.y < min_y {
                min_y = b.pos.y;
            }
        }
        assert!(
            min_y < 60.0,
            "rising body should pass through OneWay; min_y={}",
            min_y
        );
    }
}
