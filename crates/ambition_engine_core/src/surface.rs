//! The surface-follower solver — momentum locomotion over [`SurfaceChain`]s
//! (fable review 2026-07-05, AJ10 layer 3).
//!
//! The ONE new mover. A surface-momentum body is a **circle proxy** that is
//! either ballistic (`Airborne`) or attached to a surface (`Riding { on, s,
//! v_t }`). While riding, integration is **1-D along the chain's arc
//! length** — position is slaved to `s`, velocity is the scalar `v_t` along
//! the tangent — which is what makes ramps, valleys, and loops deterministic
//! and headless-testable. The body's `kin.size` AABB stays authoritative for
//! everything else (hurtboxes, triggers, portals, camera); the circle exists
//! only for surface contact, because a circle rolls cleanly through chain
//! joints and yields an unambiguous tangent frame.
//!
//! Physics rules (v1, deliberately small — the feel pass tunes numbers, the
//! STRUCTURE is the deliverable):
//! - Gravity projects onto the tangent (`g·t̂ * slope_factor`) — slopes
//!   accelerate downhill, decelerate uphill. Input accelerates along the
//!   tangent up to `top_speed`; slope may exceed it.
//! - **Stick rules.** On a straight run: shed the surface when gravity does
//!   not meaningfully press the body on (`L = g·(-n̂) < press threshold`) AND
//!   `|v_t| < min_stick_speed` — walls and ceilings hold only a fast body.
//!   At a CONVEX joint (surface bends away from the rideable side,
//!   `cross(t_i, t_j) > 0`): launch when the centripetal demand
//!   `v_t²·θ / r_smooth` exceeds what the pressing load can supply
//!   (`stick_factor · max(L, 0)`). Concave joints (loop interiors) always
//!   follow — the surface can push.
//! - **No pushout** (M10): all airborne motion is swept to TOI; landing snaps
//!   only by the contact-range discipline; nothing teleports.
//! - Chains are one-sided: a body approaching from the back side passes
//!   through. A solid [`Block`](crate::world::Block) IS a surface too — its
//!   exterior boundary is a closed rectangular chain
//!   ([`Block::boundary_chain`](crate::world::Block::boundary_chain)), so the
//!   ONE riding model covers authored chains and ordinary room geometry
//!   alike: a momentum body lands on, runs along, and jumps from block floors
//!   with the same stick/joint rules. Block corners are convex joints whose
//!   entered face carries no pressing load, so walking off an edge launches
//!   (correct) and a body can never wrap around a block by accident.
//! - **Landing is load-bearing**: an airborne body ATTACHES only to a surface
//!   gravity presses it onto (`g·(-n̂) > 0` — floors and up-slopes in the
//!   local gravity frame). Walls and ceilings hit from the air deflect (the
//!   into-surface velocity dies, flight continues) — wall/ceiling riding is
//!   reached by CONTINUITY (riding through a loop or an authored curve),
//!   never by bonking into a corridor roof. Frame-agnostic by construction.
//!   One-ways/hazards/pogo/rebound blocks are gameplay-layer concerns, not
//!   follower collision (same split as the kinematic sweep).
//!
//! Everything here is vector math — no cardinal-axis assumptions — so the C4
//! rotation rig holds by construction (see tests).

use parry2d::{
    math::{Pose, Vector},
    query::{self, ShapeCastOptions},
    shape::{Ball, Segment},
};

use crate::collision_semantics::{is_full_collision_surface, Contact, ContactSource};
use crate::world::{SurfaceChain, World};
use crate::Vec2;

/// Motion-feel parameters for a surface-momentum body. RON-authorable on the
/// archetype row (the gameplay layer hydrates these; the kernel just consumes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MomentumParams {
    /// Input acceleration along the tangent while riding (px/s²).
    pub ground_accel: f32,
    /// Deceleration when input opposes the current `v_t` (px/s²).
    pub brake: f32,
    /// Hands-off tangent friction while riding (px/s²).
    pub friction: f32,
    /// Multiplier on the gravity-tangent projection (1.0 = physical).
    pub slope_factor: f32,
    /// Input-driven speed cap along the tangent (slopes may exceed it).
    pub top_speed: f32,
    /// Airborne input acceleration along the gravity-side axis (px/s²).
    pub air_accel: f32,
    /// Jump launch speed along the surface normal (px/s).
    pub jump_speed: f32,
    /// How much of the pressing load convex joints can spend as centripetal
    /// hold (1.0 = physical; >1 is sticky, <1 is slippery).
    pub stick_factor: f32,
    /// Below this speed, surfaces gravity does not press the body onto
    /// (walls, ceilings) shed it.
    pub min_stick_speed: f32,
}

impl Default for MomentumParams {
    fn default() -> Self {
        Self {
            ground_accel: 700.0,
            brake: 1800.0,
            friction: 400.0,
            slope_factor: 1.0,
            top_speed: 900.0,
            air_accel: 500.0,
            jump_speed: 640.0,
            stick_factor: 1.5,
            min_stick_speed: 240.0,
        }
    }
}

/// Which world surface a riding body is attached to. An authored chain and a
/// solid block's exterior boundary are the SAME thing to the solver — a
/// polyline with one-sided outward normals — so `Riding` names either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceRef {
    /// `world.chains[i]`.
    Chain(usize),
    /// The exterior boundary of `world.blocks[i]`
    /// ([`crate::world::Block::boundary_chain`]).
    Block(usize),
}

/// Where the body is relative to the world's surfaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfaceMotion {
    Airborne,
    /// Attached to the surface `on` at arc length `s`, moving at scalar
    /// speed `v_t` along the tangent (signed: + = increasing arc length).
    Riding {
        on: SurfaceRef,
        s: f32,
        v_t: f32,
    },
}

/// The surface-momentum body: a circle proxy + the motion state.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceBody {
    /// Circle center (world).
    pub pos: Vec2,
    /// World velocity. Authoritative while `Airborne`; DERIVED (published for
    /// observers) while `Riding`.
    pub vel: Vec2,
    pub radius: f32,
    pub motion: SurfaceMotion,
}

impl SurfaceBody {
    pub fn new(pos: Vec2, radius: f32) -> Self {
        Self {
            pos,
            vel: Vec2::ZERO,
            radius,
            motion: SurfaceMotion::Airborne,
        }
    }

    pub fn riding(&self) -> bool {
        matches!(self.motion, SurfaceMotion::Riding { .. })
    }
}

/// Per-tick controller intent (the body enforces — two-port discipline).
#[derive(Clone, Copy, Debug, Default)]
pub struct SurfaceInputs {
    /// -1..1 run intent. Riding: along the tangent. Airborne: along the
    /// gravity frame's side axis.
    pub run: f32,
    pub jump_pressed: bool,
}

/// One frame of surface-momentum physics. `gravity` is the full vector
/// (direction × magnitude, e.g. `(0, 1450)`) so gravity zones compose.
pub fn step_surface_body(
    body: &mut SurfaceBody,
    world: &World,
    params: &MomentumParams,
    gravity: Vec2,
    inputs: SurfaceInputs,
    dt: f32,
    mut contacts: Option<&mut Vec<Contact>>,
) {
    if dt <= 0.0 {
        return;
    }
    match body.motion {
        SurfaceMotion::Riding { on, s, v_t } => {
            step_riding(
                body,
                world,
                params,
                gravity,
                inputs,
                dt,
                on,
                s,
                v_t,
                contacts.as_deref_mut(),
            );
        }
        SurfaceMotion::Airborne => {
            step_airborne(
                body,
                world,
                params,
                gravity,
                inputs,
                dt,
                contacts.as_deref_mut(),
            );
        }
    }
}

/// Materialize the chain a [`SurfaceRef`] names. `None` when the referenced
/// surface no longer exists (room rebuilt under the rider — go airborne).
fn resolve_surface(world: &World, on: SurfaceRef) -> Option<std::borrow::Cow<'_, SurfaceChain>> {
    match on {
        SurfaceRef::Chain(i) => world.chains.get(i).map(std::borrow::Cow::Borrowed),
        SurfaceRef::Block(i) => {
            let block = world.blocks.get(i)?;
            if !is_full_collision_surface(block.kind) {
                return None;
            }
            Some(std::borrow::Cow::Owned(block.boundary_chain()))
        }
    }
}

/// The contact source a ride on `on` reports (chains carry their segment;
/// blocks carry their kind, matching the kinematic sweep's vocabulary).
fn ride_contact_source(world: &World, on: SurfaceRef, segment: usize) -> ContactSource {
    match on {
        SurfaceRef::Chain(i) => ContactSource::Chain {
            chain: i as u32,
            segment: segment as u32,
        },
        SurfaceRef::Block(i) => ContactSource::Block {
            kind: world.blocks[i].kind,
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn step_riding(
    body: &mut SurfaceBody,
    world: &World,
    params: &MomentumParams,
    gravity: Vec2,
    inputs: SurfaceInputs,
    dt: f32,
    on: SurfaceRef,
    s: f32,
    mut v_t: f32,
    mut contacts: Option<&mut Vec<Contact>>,
) {
    let Some(chain) = resolve_surface(world, on) else {
        body.motion = SurfaceMotion::Airborne;
        return;
    };
    let chain = chain.as_ref();
    let frame = chain.frame_at(s);

    // Jump: leave along the outward normal (+n̂ points off the surface,
    // toward the body side), keeping the tangent momentum.
    if inputs.jump_pressed {
        body.vel = v_t * frame.tangent + params.jump_speed * frame.normal;
        body.motion = SurfaceMotion::Airborne;
        // One airborne substep so the jump moves this frame.
        step_airborne(
            body,
            world,
            params,
            gravity,
            SurfaceInputs::default(),
            dt,
            contacts,
        );
        return;
    }

    // 1) Tangent dynamics: input (capped), slope (uncapped), friction.
    let run = inputs.run.clamp(-1.0, 1.0);
    if run.abs() > 0.1 {
        let opposing = run.signum() != v_t.signum() && v_t.abs() > 1.0;
        let accel = if opposing {
            params.brake
        } else {
            params.ground_accel
        };
        let before = v_t;
        v_t += run * accel * dt;
        // Input never pushes past top_speed; slope-earned speed is preserved.
        let cap = params.top_speed.max(before.abs());
        v_t = v_t.clamp(-cap, cap);
    } else {
        v_t = approach(v_t, 0.0, params.friction * dt);
    }
    // Slope force evaluated at the MIDPOINT of the step's arc — cancels the
    // first-order energy drift a start-of-step evaluation pumps into every
    // joint crossing (downhill accel applied across the flat, etc).
    let mid = chain.frame_at(s + v_t * dt * 0.5);
    let slope_accel = gravity.dot(mid.tangent) * params.slope_factor;
    v_t += slope_accel * dt;

    // 2) Straight-run stick rule at the CURRENT frame.
    let press = gravity.dot(-frame.normal); // >0: gravity pushes body onto surface
    let press_threshold = 0.25 * gravity.length();
    if press < press_threshold && v_t.abs() < params.min_stick_speed {
        shed(body, chain, frame.tangent, v_t, dt);
        return;
    }

    // 3) Advance along the arc, applying the joint rule at every crossed join.
    match advance_riding(chain, s, v_t * dt, v_t, gravity, params, body.radius) {
        RideOutcome::Riding { s: new_s } => {
            let f = chain.frame_at(new_s);
            body.pos = f.point + f.normal * body.radius;
            body.vel = v_t * f.tangent + per_frame_to_per_sec(chain.velocity, dt);
            body.motion = SurfaceMotion::Riding { on, s: new_s, v_t };
            if let Some(sink) = contacts.as_deref_mut() {
                sink.push(Contact {
                    point: f.point,
                    normal: f.normal,
                    toi: 0.0,
                    surface_velocity: chain.velocity,
                    source: ride_contact_source(world, on, f.segment),
                });
            }
        }
        RideOutcome::Launch { s: launch_s } => {
            let f = chain.frame_at(launch_s);
            body.pos = f.point + f.normal * body.radius;
            shed(body, chain, f.tangent, v_t, dt);
        }
    }
}

/// Is `s` clamped to an OPEN chain's endpoint with the body's tangential velocity
/// pointing off it?
///
/// `SurfaceChain::project` clamps arc length into `[0, total_length]`, so a body
/// that is physically past the end still projects TO the end. Landing there
/// re-attaches it at the last vertex, the ride step launches it off the same
/// vertex next tick, and the pair form a two-frame limit cycle: the body hovers
/// at the lip with its position frozen. Closed chains (and every block boundary)
/// have no ends and are never affected.
fn leaving_an_open_end(chain: &SurfaceChain, s: f32, v_t: f32) -> bool {
    if chain.closed {
        return false;
    }
    // A body landing exactly on an endpoint with v_t pointing INWARD (or at rest)
    // is a legitimate landing at the tip of a ramp, and must still attach.
    const END_EPS: f32 = 1e-3;
    (s <= END_EPS && v_t < 0.0) || (s >= chain.total_length() - END_EPS && v_t > 0.0)
}

/// Leave the surface with the tangent momentum (plus the chain's own motion).
fn shed(body: &mut SurfaceBody, chain: &SurfaceChain, tangent: Vec2, v_t: f32, dt: f32) {
    body.vel = v_t * tangent + per_frame_to_per_sec(chain.velocity, dt);
    body.motion = SurfaceMotion::Airborne;
}

/// `SurfaceChain::velocity` is a per-frame delta (Block semantics); observers
/// and launches want px/s.
fn per_frame_to_per_sec(per_frame: Vec2, dt: f32) -> Vec2 {
    if dt > 0.0 {
        per_frame / dt
    } else {
        Vec2::ZERO
    }
}

enum RideOutcome {
    Riding { s: f32 },
    Launch { s: f32 },
}

/// Walk `ds` of arc from `s`, applying the joint rule at every segment join
/// crossed. Returns where the body ends up, or where it launches.
fn advance_riding(
    chain: &SurfaceChain,
    s: f32,
    ds: f32,
    v_t: f32,
    gravity: Vec2,
    params: &MomentumParams,
    radius: f32,
) -> RideOutcome {
    let total = chain.total_length();
    let mut current = s;
    let mut remaining = ds;
    // Bounded walk: no step crosses more joints than the chain has segments.
    for _ in 0..=chain.segment_count() {
        if remaining == 0.0 {
            return RideOutcome::Riding { s: current };
        }
        let frame = chain.frame_at(current);
        // Distance to the next joint in the direction of travel.
        let seg_start = arc_at_segment_start(chain, frame.segment);
        let seg_len = chain.segment_length(frame.segment);
        let to_join = if remaining > 0.0 {
            (seg_start + seg_len) - current
        } else {
            seg_start - current // negative or zero
        };
        if remaining.abs() <= to_join.abs() || to_join.abs() < 1.0e-6 && remaining.abs() < 1.0e-6 {
            let landed = current + remaining;
            if !chain.closed && (landed < 0.0 || landed > total) {
                // Ran off an open end: launch with the end tangent.
                return RideOutcome::Launch {
                    s: landed.clamp(0.0, total),
                };
            }
            return RideOutcome::Riding { s: landed };
        }
        // Cross the joint: spend the arc up to it, then test the turn.
        let at_join = current + to_join;
        if !chain.closed && (at_join <= 0.0 || at_join >= total) {
            return RideOutcome::Launch {
                s: at_join.clamp(0.0, total),
            };
        }
        remaining -= to_join;
        current = at_join;
        let seg_i = frame.segment;
        let entered = if remaining > 0.0 {
            (seg_i + 1) % chain.segment_count()
        } else {
            (seg_i + chain.segment_count() - 1) % chain.segment_count()
        };
        // Convexity is a property of the GEOMETRY, not the travel direction:
        // always test the authored-order pair at this join (a hill crest is a
        // crest whichever way you run over it).
        let (auth_a, auth_b) = if remaining > 0.0 {
            (seg_i, entered)
        } else {
            (entered, seg_i)
        };
        let t_a = chain.tangent(auth_a);
        let t_b = chain.tangent(auth_b);
        let cross = t_a.perp_dot(t_b);
        if cross > 1.0e-6 {
            // CONVEX: surface bends away from the rideable side. Stay only if
            // the pressing load on the segment being ENTERED can supply the
            // centripetal demand.
            let angle = t_a.dot(t_b).clamp(-1.0, 1.0).acos();
            let n_entered = chain.normal(entered);
            let press = gravity.dot(-n_entered).max(0.0);
            let smoothing = radius.max(1.0);
            let demand = v_t * v_t * angle / smoothing;
            if demand > params.stick_factor * press {
                return RideOutcome::Launch { s: current };
            }
        }
        // Concave (or tiny) joins always follow: the surface pushes.
        // Nudge past the join so frame_at resolves the next segment.
        //
        // The nudge must be RELATIVE. A fixed `1e-4` is under one f32 ULP once the
        // arc length passes ~800px (ULP at 857 is 6.1e-5), so on a long chain the
        // nudge rounded back to the joint, `frame_at` kept resolving the segment
        // that STARTS there, `to_join` stayed 0, and the bounded walk spun out
        // without advancing. The body froze on the joint, riding, forever — with
        // its velocity intact, which is what made it look like a physics puzzle
        // rather than a rounding bug. Found by the `SurfaceRamp` winding oracle
        // (Q27), whose lead-in chain is long enough to reach the failure.
        let nudge = joint_nudge(current);
        current += remaining.signum() * nudge;
        remaining -= remaining.signum() * nudge;
        if chain.closed {
            current = current.rem_euclid(total);
        }
    }
    RideOutcome::Riding { s: current }
}

/// A step past a joint that is guaranteed to be representable at `s`.
///
/// `f32` spacing grows with magnitude: at `s = 857` one ULP is 6.1e-5, so an
/// absolute `1e-4` is barely more than one and can round away entirely. Eight ULPs
/// is unambiguous and still far below any geometric scale, with a floor for small
/// `s` where ULPs are tiny.
fn joint_nudge(s: f32) -> f32 {
    (s.abs() * f32::EPSILON * 8.0).max(1.0e-4)
}

/// Arc length at the START of segment `i`.
fn arc_at_segment_start(chain: &SurfaceChain, i: usize) -> f32 {
    (0..i).map(|k| chain.segment_length(k)).sum()
}

fn step_airborne(
    body: &mut SurfaceBody,
    world: &World,
    params: &MomentumParams,
    gravity: Vec2,
    inputs: SurfaceInputs,
    dt: f32,
    mut contacts: Option<&mut Vec<Contact>>,
) {
    // Ballistic + air control along the gravity frame's side axis.
    body.vel += gravity * dt;
    let run = inputs.run.clamp(-1.0, 1.0);
    if run.abs() > 0.1 {
        // The local side axis is the along-surface tangent of the FLOOR a body
        // would be standing on, and a floor's normal is `-gravity`. Using
        // `tangent_of(gravity)` here — the exact negation — mirrored air control
        // for every momentum body. `AccelerationFrame::new(gravity).side` is the
        // same vector; `tangent_of` names why.
        let side = crate::frame::tangent_of(-gravity.normalize_or_zero());
        let along = body.vel.dot(side);
        let target = run * params.top_speed;
        // Equilibrium steering: accelerate toward the held direction up to
        // top_speed, never brake speed already beyond it in that direction.
        let new_along = if run > 0.0 {
            approach(along, target.max(along), params.air_accel * dt)
        } else {
            approach(along, target.min(along), params.air_accel * dt)
        };
        body.vel += (new_along - along) * side;
    }

    let delta = body.vel * dt;
    match first_circle_hit(world, body.pos, body.radius, delta) {
        Some(hit) => {
            body.pos += delta * hit.toi;
            // Landing is load-bearing: attach only to a surface gravity
            // presses the body onto (floors/up-slopes in the local gravity
            // frame). Walls and ceilings hit from the air DEFLECT — riding
            // them is reached by continuity, never by bonking.
            let press = gravity.dot(-hit.normal);
            if press > 0.0 {
                let on = match hit.what {
                    CircleHitTarget::Chain { chain } => SurfaceRef::Chain(chain),
                    CircleHitTarget::Block { block } => SurfaceRef::Block(block),
                };
                let surface = resolve_surface(world, on)
                    .expect("first_circle_hit only reports live surfaces");
                let surface = surface.as_ref();
                let (s, _) = surface.project(body.pos);
                let f = surface.frame_at(s);
                let rel = body.vel - per_frame_to_per_sec(surface.velocity, dt);
                let v_t = rel.dot(f.tangent);
                if leaving_an_open_end(surface, s, v_t) {
                    // The body already walked off this chain's end and is moving
                    // AWAY from it. `project` clamps `s` to the end, so attaching
                    // here would snap it back onto the last vertex — from which
                    // the ride step immediately launches it again, at the same
                    // point, forever. A body hovering in place at the lip of a
                    // ledge is what that looks like on screen. Fall.
                } else {
                    body.pos = f.point + f.normal * body.radius;
                    body.motion = SurfaceMotion::Riding { on, s, v_t };
                    body.vel = v_t * f.tangent + per_frame_to_per_sec(surface.velocity, dt);
                }
            } else {
                // Deflect: kill the into-surface velocity component; the
                // remainder of this frame's motion is dropped (one swept
                // TOI per frame, never a pushout).
                let n = hit.normal;
                let into = body.vel.dot(-n).max(0.0);
                body.vel += into * n;
            }
            if let Some(sink) = contacts.as_deref_mut() {
                sink.push(Contact {
                    point: body.pos - hit.normal * body.radius,
                    normal: hit.normal,
                    toi: hit.toi,
                    surface_velocity: hit.surface_velocity,
                    source: hit.source,
                });
            }
        }
        None => {
            body.pos += delta;
        }
    }
}

struct CircleHit {
    toi: f32,
    /// Surface outward normal (toward the body).
    normal: Vec2,
    surface_velocity: Vec2,
    source: ContactSource,
    what: CircleHitTarget,
}

enum CircleHitTarget {
    Chain { chain: usize },
    Block { block: usize },
}

/// Earliest swept-circle hit against chains (one-sided) and solid blocks.
fn first_circle_hit(world: &World, center: Vec2, radius: f32, delta: Vec2) -> Option<CircleHit> {
    if delta.length_squared() <= 1.0e-12 {
        return None;
    }
    let ball = Ball::new(radius);
    let options = ShapeCastOptions {
        max_time_of_impact: 1.0,
        target_distance: 0.0,
        stop_at_penetration: false,
        compute_impact_geometry_on_penetration: true,
    };
    let pose = Pose::translation(center.x, center.y);
    let vel = Vector::new(delta.x, delta.y);
    let mut best: Option<CircleHit> = None;

    // Chains: one-sided segments — land only when approaching the rideable
    // (+normal) side and moving into the surface.
    for (ci, chain) in world.chains.iter().enumerate() {
        for i in 0..chain.segment_count() {
            let (a, b) = chain.segment(i);
            let n = chain.normal(i);
            if delta.dot(n) >= 0.0 {
                continue; // moving away from / along the surface
            }
            if (center - a).dot(n) < 0.0 {
                continue; // approaching from the back (solid) side
            }
            let seg = Segment::new(Vector::new(a.x, a.y), Vector::new(b.x, b.y));
            let Ok(Some(hit)) = query::cast_shapes(
                &pose,
                vel,
                &ball,
                &Pose::identity(),
                Vector::ZERO,
                &seg,
                options,
            ) else {
                continue;
            };
            let toi = hit.time_of_impact.clamp(0.0, 1.0);
            if best.as_ref().is_none_or(|b| toi < b.toi) {
                best = Some(CircleHit {
                    toi,
                    normal: n,
                    surface_velocity: chain.velocity,
                    source: ContactSource::Chain {
                        chain: ci as u32,
                        segment: i as u32,
                    },
                    what: CircleHitTarget::Chain { chain: ci },
                });
            }
        }
    }

    // Solid blocks: their exterior boundaries are surfaces, swept exactly
    // like chain segments (per-face, one-sided from outside). A convex AABB
    // approached from outside always presents its facing faces, so per-face
    // segment casts cover everything the old whole-cuboid cast did — and
    // every hit is attachable.
    for (bi, block) in world.blocks.iter().enumerate() {
        if !is_full_collision_surface(block.kind) {
            continue;
        }
        let boundary = block.boundary_chain();
        for i in 0..boundary.segment_count() {
            let (a, b) = boundary.segment(i);
            let n = boundary.normal(i);
            if delta.dot(n) >= 0.0 {
                continue; // moving away from / along the face
            }
            if (center - a).dot(n) < 0.0 {
                continue; // behind the face plane (inside/adjacent) — not approachable
            }
            let seg = Segment::new(Vector::new(a.x, a.y), Vector::new(b.x, b.y));
            let Ok(Some(hit)) = query::cast_shapes(
                &pose,
                vel,
                &ball,
                &Pose::identity(),
                Vector::ZERO,
                &seg,
                options,
            ) else {
                continue;
            };
            let toi = hit.time_of_impact.clamp(0.0, 1.0);
            // Interior faces of flush composite geometry are NOT surfaces: a
            // floor tiled from several blocks buries each block's side walls
            // inside its neighbors. Probe just outside the face at the
            // contact point — buried inside another solid ⇒ skip the hit.
            let center_at_impact = center + delta * toi;
            let ab = b - a;
            let t = if ab.length_squared() > 0.0 {
                ((center_at_impact - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let probe = a + ab * t + n * 0.5;
            let buried = world.blocks.iter().enumerate().any(|(bj, other)| {
                bj != bi
                    && is_full_collision_surface(other.kind)
                    && probe.x >= other.aabb.min.x
                    && probe.x <= other.aabb.max.x
                    && probe.y >= other.aabb.min.y
                    && probe.y <= other.aabb.max.y
            });
            if buried {
                continue;
            }
            if best.as_ref().is_none_or(|b| toi < b.toi) {
                best = Some(CircleHit {
                    toi,
                    normal: n,
                    surface_velocity: block.velocity,
                    source: ContactSource::Block { kind: block.kind },
                    what: CircleHitTarget::Block { block: bi },
                });
            }
        }
    }
    best
}

fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

#[cfg(test)]
mod tests;
