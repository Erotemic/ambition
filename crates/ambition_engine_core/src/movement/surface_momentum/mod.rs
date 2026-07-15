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
use crate::world::{SurfaceChain, SurfaceFrame, SurfacePort, World};
use crate::{MotionFrame, Vec2};

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
    /// Simulated-depth lane retained while airborne.
    ///
    /// Lanes are discrete collision planes: a body collides only with authored
    /// chain segments on the same lane. Route traversal may change lanes while
    /// riding, and ordinary solid blocks remain depth-agnostic. Treating lane
    /// `0` as a wildcard made a rider shed from the loop's center plane and
    /// immediately snag on its foreground/background rails.
    pub depth_lane: i8,
    pub motion: SurfaceMotion,
}

impl SurfaceBody {
    #[allow(dead_code, reason = "fixture constructor for kernel-private tests")]
    pub fn new(pos: Vec2, radius: f32) -> Self {
        Self {
            pos,
            vel: Vec2::ZERO,
            radius,
            depth_lane: 0,
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
    /// Controller intent in the body's current acceleration-relative frame:
    /// `+x` is local side/right and `+y` is local down/toward-feet.
    ///
    /// The solver receives this artifact unchanged from the common kernel
    /// boundary and uses the supplied [`MotionFrame`] for every world-space
    /// interpretation. No movement policy is allowed to reinterpret raw screen
    /// input or construct a private control frame.
    pub local_axes: crate::reference_frame::LocalAxes,
    pub jump_pressed: bool,
}

/// One frame of surface-momentum physics in the body's current acceleration
/// frame. The exact same [`MotionFrame`] is supplied to every movement policy;
/// this solver never reconstructs a private gravity/reference frame.
pub(crate) fn step_surface_body(
    body: &mut SurfaceBody,
    world: &World,
    params: &MomentumParams,
    frame: MotionFrame,
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
                frame,
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
                frame,
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
            id: world.blocks[i].id.clone(),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn step_riding(
    body: &mut SurfaceBody,
    world: &World,
    params: &MomentumParams,
    motion_frame: MotionFrame,
    inputs: SurfaceInputs,
    dt: f32,
    on: SurfaceRef,
    s: f32,
    mut v_t: f32,
    mut contacts: Option<&mut Vec<Contact>>,
) {
    let gravity = motion_frame.acceleration();
    let run = inputs.local_axes.x.clamp(-1.0, 1.0);
    let (on, s) =
        choose_route_branch_at_rest(world, on, s, v_t, motion_frame, inputs.local_axes.vec())
            .unwrap_or((on, s));
    let Some(chain) = resolve_surface(world, on) else {
        body.motion = SurfaceMotion::Airborne;
        return;
    };
    let chain = chain.as_ref();
    // Arc length alone is ambiguous exactly at a polyline joint: `frame_at`
    // resolves the segment before the join, even when the segment after it is
    // the only load-bearing branch. At speed that ambiguity is resolved by
    // travel direction. At rest, resolve it by controller intent and finally by
    // gravity support. Otherwise a body stopped on a ramp/loop vertex can be
    // classified against a wall-like segment, shed to Airborne, and lose jump,
    // crouch, and walking while appearing to stand on the edge.
    let stabilized_s = stabilize_joint_rest(chain, s, v_t, run, gravity);
    let frame = chain.frame_at(stabilized_s);
    body.depth_lane = chain.segment_depth(frame.segment);
    if stabilized_s != s {
        // Riding position is slaved to the selected surface frame. Move around
        // the tiny joint ambiguity before a jump or zero-distance step consumes
        // it; the displacement is one representable arc nudge, not a pushout.
        body.pos = frame.point + frame.normal * body.radius;
    }
    let s = stabilized_s;

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
            motion_frame,
            SurfaceInputs::default(),
            dt,
            contacts,
        );
        return;
    }

    // 1) Tangent dynamics: input (capped), slope (uncapped), friction.
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
    match advance_riding(
        world,
        on,
        stabilized_s,
        v_t * dt,
        v_t,
        motion_frame,
        params,
        body.radius,
        inputs.local_axes.vec(),
    ) {
        RideOutcome::Riding {
            on: new_on,
            s: new_s,
            v_t: new_v_t,
        } => {
            let Some(final_chain) = resolve_surface(world, new_on) else {
                body.motion = SurfaceMotion::Airborne;
                return;
            };
            let final_chain = final_chain.as_ref();
            let f = final_chain.frame_at(new_s);
            body.pos = f.point + f.normal * body.radius;
            body.vel = new_v_t * f.tangent + per_frame_to_per_sec(final_chain.velocity, dt);
            body.depth_lane = final_chain.segment_depth(f.segment);
            body.motion = SurfaceMotion::Riding {
                on: new_on,
                s: new_s,
                v_t: new_v_t,
            };
            if let Some(sink) = contacts.as_deref_mut() {
                sink.push(Contact {
                    kind: crate::collision_semantics::ContactKind::Support,
                    point: f.point,
                    normal: f.normal,
                    toi: 0.0,
                    surface_velocity: final_chain.velocity,
                    source: ride_contact_source(world, new_on, f.segment),
                });
            }
        }
        RideOutcome::Launch {
            on: launch_on,
            frame,
            v_t: launch_v_t,
        } => {
            let Some(launch_chain) = resolve_surface(world, launch_on) else {
                body.motion = SurfaceMotion::Airborne;
                return;
            };
            let launch_chain = launch_chain.as_ref();
            body.pos = frame.point + frame.normal * body.radius;
            body.depth_lane = launch_chain.segment_depth(frame.segment);
            shed(body, launch_chain, frame.tangent, launch_v_t, dt);
        }
    }
}

/// Resolve an explicitly authored route junction while stationary.
///
/// Arc-length state still identifies which occurrence the rider reached, so a
/// neutral stick preserves that route. A meaningful 2-D direction may select a
/// different coincident occurrence and outgoing half-edge before acceleration
/// is applied. This is what makes "hold up into the loop / hold down onto the
/// lower ramp" work even from a dead stop at the switch.
fn choose_route_branch_at_rest(
    world: &World,
    on: SurfaceRef,
    s: f32,
    v_t: f32,
    frame: MotionFrame,
    local_axis: Vec2,
) -> Option<(SurfaceRef, f32)> {
    let SurfaceRef::Chain(chain_index) = on else {
        return None;
    };
    if v_t.abs() > 1.0e-3
        || local_axis.length_squared() <= ROUTE_BIAS_DEADZONE * ROUTE_BIAS_DEADZONE
    {
        return None;
    }
    let chain = world.chains.get(chain_index)?;
    let total = chain.total_length();
    let s = if chain.closed {
        s.rem_euclid(total)
    } else {
        s.clamp(0.0, total)
    };
    let desired = frame.to_world(local_axis).normalize_or_zero();
    let mut current_vertex = None;
    let mut ports = None;
    for vertex in 0..chain.points.len() {
        let vertex_s = chain.arc_at_vertex(vertex);
        if (s - vertex_s).abs() <= joint_nudge(vertex_s) * 2.0 {
            if let Some(found) = route_junction_ports(world, chain_index, vertex) {
                current_vertex = Some(vertex);
                ports = Some(found);
                break;
            }
        }
    }
    let current_vertex = current_vertex?;
    let ports = ports?;
    let mut best: Option<(f32, RouteBranch)> = None;
    for port in ports {
        for mut branch in outgoing_route_branches(world, port) {
            branch.is_default = port.chain == chain_index && port.vertex == current_vertex;
            let score = branch.heading.dot(desired);
            let replace = match best {
                None => true,
                Some((best_score, best_branch)) => {
                    score > best_score + ROUTE_SWITCH_MARGIN
                        || ((score - best_score).abs() <= ROUTE_SWITCH_MARGIN
                            && branch.is_default
                            && !best_branch.is_default)
                }
            };
            if replace {
                best = Some((score, branch));
            }
        }
    }
    let (_, branch) = best?;
    let target = resolve_surface(world, branch.on)?;
    let target = target.as_ref();
    let vertex_s = target.arc_at_vertex(branch.vertex);
    let nudge = joint_nudge(vertex_s);
    let selected = vertex_s + branch.direction * nudge;
    let selected = if target.closed {
        selected.rem_euclid(target.total_length())
    } else {
        selected.clamp(0.0, target.total_length())
    };
    Some((branch.on, selected))
}

/// Resolve the tangent/normal ambiguity of a body resting exactly on an
/// interior polyline joint.
///
/// `SurfaceMotion::Riding` stores arc length, not a segment id. At a joint that
/// is sufficient while moving: the sign of `v_t` says which branch is entered.
/// At zero speed, [`SurfaceChain::frame_at`] necessarily picks one side by an
/// arbitrary `<=` tie. If that side is wall-like while the other is a floor,
/// the straight-run stick rule ejects the body even though a valid support
/// exists. The visible result is a body frozen on a ramp/loop edge that cannot
/// jump, crouch, or walk away.
///
/// For a genuinely stationary body, nudge onto exactly one adjacent branch
/// using, in order:
/// 1. the branch gravity presses the body onto most strongly,
/// 2. held run intent when support is tied (positive = increasing arc).
///
/// A moving body is left exactly at the joint. [`advance_riding`] must observe
/// and classify that crossing; pre-selecting the entered branch here would skip
/// its convex/concave launch rule.
///
/// Support precedes intent at rest so walking off a supported lip begins on the
/// supporting tangent; [`advance_riding`] then applies the ordinary convex-joint
/// launch rule. Selecting the unsupported wall branch first would launch down
/// the wall instead of carrying the body's floor momentum over the edge.
///
/// Open endpoints are not joints and remain governed by the ordinary launch
/// rule. The nudge uses [`joint_nudge`], so it is representable at any arc
/// magnitude and remains far below gameplay geometry scale.
fn stabilize_joint_rest(chain: &SurfaceChain, s: f32, v_t: f32, run: f32, gravity: Vec2) -> f32 {
    let count = chain.segment_count();
    if count < 2 {
        return s;
    }
    let total = chain.total_length();
    let s = if chain.closed {
        s.rem_euclid(total)
    } else {
        s.clamp(0.0, total)
    };

    // Moving bodies must reach the joint through `advance_riding`, which owns
    // the convex/concave turn rule. Nudging a moving body onto the entered
    // segment here bypasses that rule entirely: a runner at a block lip can be
    // placed directly on the wall, while a flush block seam can shed before the
    // ordinary same-frame handoff. This helper exists only for the genuinely
    // ambiguous zero-speed case.
    const SPEED_EPS: f32 = 1.0e-3;
    if v_t.abs() > SPEED_EPS {
        return s;
    }

    let choose_direction = |prev: usize, next: usize| -> f32 {
        const INPUT_EPS: f32 = 0.1;
        const PRESS_EPS: f32 = 1.0e-3;
        let prev_press = gravity.dot(-chain.normal(prev));
        let next_press = gravity.dot(-chain.normal(next));
        if next_press > prev_press + PRESS_EPS {
            return 1.0;
        }
        if prev_press > next_press + PRESS_EPS {
            return -1.0;
        }
        if run.abs() > INPUT_EPS {
            return run.signum();
        }
        // Ties preserve the historical/pre-join branch. This keeps a body at a
        // symmetric valley deterministic until input supplies a side.
        -1.0
    };

    let nudge_onto = |joint_s: f32, direction: f32| -> f32 {
        let nudge = joint_nudge(joint_s.max(0.0));
        if chain.closed {
            (joint_s + direction * nudge).rem_euclid(total)
        } else {
            (joint_s + direction * nudge).clamp(0.0, total)
        }
    };

    let mut joint_s = 0.0;
    for next in 1..count {
        joint_s += chain.segment_length(next - 1);
        let tolerance = joint_nudge(joint_s) * 2.0;
        if (s - joint_s).abs() <= tolerance {
            return nudge_onto(joint_s, choose_direction(next - 1, next));
        }
    }

    if chain.closed {
        let tolerance = joint_nudge(total) * 2.0;
        if s <= tolerance || total - s <= tolerance {
            let direction = choose_direction(count - 1, 0);
            return nudge_onto(0.0, direction);
        }
    }

    s
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
    // `project_to_segment` deliberately nudges a shared endpoint onto the exact
    // segment reported by the sweep. The endpoint guard must use at least that
    // same scale or an outward-moving body can reattach one nudge before the
    // end and hover at the lip forever.
    let total = chain.total_length();
    let end_eps = joint_nudge(total) * 2.0;
    // A body landing exactly on an endpoint with v_t pointing INWARD (or at rest)
    // is a legitimate landing at the tip of a ramp, and must still attach.
    (s <= end_eps && v_t < 0.0) || (s >= total - end_eps && v_t > 0.0)
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
    Riding {
        on: SurfaceRef,
        s: f32,
        v_t: f32,
    },
    /// Exact departure frame from the branch being LEFT. Recomputing
    /// `frame_at(s)` at a joint is ambiguous and can substitute the entered
    /// wall tangent for the floor tangent, turning "walk off" into "drop down
    /// the wall" or another reattachment loop.
    Launch {
        on: SurfaceRef,
        frame: SurfaceFrame,
        v_t: f32,
    },
}

fn departure_frame(chain: &SurfaceChain, s: f32, segment: usize) -> SurfaceFrame {
    let point = chain.frame_at(s).point;
    let tangent = chain.tangent(segment);
    SurfaceFrame {
        point,
        tangent,
        normal: Vec2::new(tangent.y, -tangent.x),
        segment,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResolvedPort {
    chain: usize,
    vertex: usize,
}

fn resolve_port(owner_chain: usize, port: SurfacePort) -> ResolvedPort {
    match port {
        SurfacePort::Local(vertex) => ResolvedPort {
            chain: owner_chain,
            vertex,
        },
        SurfacePort::Chain { chain, vertex } => ResolvedPort { chain, vertex },
    }
}

/// Find the authored route switch that contains this exact chain/vertex
/// occurrence. Junction declarations live on a chain so the world schema stays
/// compact, but their ports may reference other authored chains.
fn route_junction_ports(
    world: &World,
    current_chain: usize,
    current_vertex: usize,
) -> Option<Vec<ResolvedPort>> {
    let needle = ResolvedPort {
        chain: current_chain,
        vertex: current_vertex,
    };
    for (owner_index, owner) in world.chains.iter().enumerate() {
        for junction in &owner.junctions {
            let ports: Vec<_> = junction
                .ports
                .iter()
                .copied()
                .map(|port| resolve_port(owner_index, port))
                .filter(|port| {
                    world
                        .chains
                        .get(port.chain)
                        .is_some_and(|chain| port.vertex < chain.points.len())
                })
                .collect();
            if ports.contains(&needle) {
                return Some(ports);
            }
        }
    }
    None
}

#[derive(Clone, Copy, Debug)]
struct RouteBranch {
    on: SurfaceRef,
    vertex: usize,
    segment: usize,
    direction: f32,
    tangent: Vec2,
    /// Coarse direction of the route after the junction, sampled far enough
    /// beyond the common tangent to distinguish an up-ramp from a down-ramp.
    heading: Vec2,
    is_default: bool,
}

const ROUTE_LOOKAHEAD: f32 = 128.0;
const ROUTE_SWITCH_MARGIN: f32 = 0.07;
const ROUTE_BIAS_DEADZONE: f32 = 0.25;

/// Return the player's deliberate route-selection direction.
///
/// Locomotion and route choice are different controls. Left/Right drive signed
/// speed along the current surface; only Up/Down in the acceleration frame may
/// override an authored continuation. Projecting against the incoming tangent
/// is incorrect on a slope: plain Left then acquires a fake downward component
/// and can route a reverse runner into another loop lap.
fn route_bias_direction(frame: MotionFrame, local_axis: Vec2) -> Option<Vec2> {
    let amount = local_axis.y;
    let down = frame.down();
    if amount.abs() <= ROUTE_BIAS_DEADZONE {
        None
    } else {
        Some(down * amount.signum())
    }
}

fn route_branch_heading(chain: &SurfaceChain, vertex: usize, direction: f32) -> Vec2 {
    let origin = chain.points[vertex % chain.points.len()];
    let total = chain.total_length();
    let start = chain.arc_at_vertex(vertex);
    let target_s = if chain.closed {
        (start + direction * ROUTE_LOOKAHEAD).rem_euclid(total)
    } else {
        (start + direction * ROUTE_LOOKAHEAD).clamp(0.0, total)
    };
    let delta = chain.frame_at(target_s).point - origin;
    if delta.length_squared() > 1.0e-6 {
        delta.normalize()
    } else if direction > 0.0 {
        chain.tangent(vertex % chain.segment_count())
    } else {
        -chain.tangent((vertex + chain.segment_count() - 1) % chain.segment_count())
    }
}

fn outgoing_route_branches(world: &World, port: ResolvedPort) -> Vec<RouteBranch> {
    let Some(chain) = world.chains.get(port.chain) else {
        return Vec::new();
    };
    let count = chain.segment_count();
    let mut out = Vec::with_capacity(2);
    if chain.closed || port.vertex < count {
        let segment = port.vertex % count;
        out.push(RouteBranch {
            on: SurfaceRef::Chain(port.chain),
            vertex: port.vertex,
            segment,
            direction: 1.0,
            tangent: chain.tangent(segment),
            heading: route_branch_heading(chain, port.vertex, 1.0),
            is_default: false,
        });
    }
    if chain.closed || port.vertex > 0 {
        let segment = (port.vertex + count - 1) % count;
        out.push(RouteBranch {
            on: SurfaceRef::Chain(port.chain),
            vertex: port.vertex,
            segment,
            direction: -1.0,
            tangent: -chain.tangent(segment),
            heading: route_branch_heading(chain, port.vertex, -1.0),
            is_default: false,
        });
    }
    out
}

/// Select an authored route switch at a coincident-vertex junction.
///
/// The chain's ordinary continuation wins unless acceleration-frame Up/Down
/// clearly prefers another branch. Branch preference uses a short route
/// LOOKAHEAD rather than only the immediate tangent: two paths can be
/// tangent-continuous at the switch while one rises and the other falls a few
/// pixels later. Left/Right never changes topology; it remains locomotion.
fn choose_route_branch(
    world: &World,
    on: SurfaceRef,
    current_vertex: usize,
    incoming_segment: usize,
    travel_sign: f32,
    frame: MotionFrame,
    local_axis: Vec2,
) -> Option<RouteBranch> {
    let SurfaceRef::Chain(current_chain) = on else {
        return None;
    };
    let current_surface = world.chains.get(current_chain)?;
    let ports = route_junction_ports(world, current_chain, current_vertex)?;
    let incoming = travel_sign * current_surface.tangent(incoming_segment);
    let mut candidates = Vec::new();
    for port in ports {
        for mut branch in outgoing_route_branches(world, port) {
            // Never synthesize an immediate U-turn back over the segment just
            // traversed. Reversing remains ordinary braking/input behavior on a
            // later tick, not a route-switch side effect.
            if branch.on == on
                && branch.segment == incoming_segment
                && branch.tangent.dot(incoming) < -0.999
            {
                continue;
            }
            branch.is_default = port.chain == current_chain
                && port.vertex == current_vertex
                && branch.direction.signum() == travel_sign.signum();
            candidates.push(branch);
        }
    }
    let default = candidates
        .iter()
        .copied()
        .find(|branch| branch.is_default)
        .or_else(|| {
            candidates
                .iter()
                .copied()
                .max_by(|a, b| a.tangent.dot(incoming).total_cmp(&b.tangent.dot(incoming)))
        })?;

    // Forward/back locomotion is NOT route bias. Route choice is authored in
    // the acceleration frame: Up/Down may select a branch, while Left/Right
    // only drive along the current route. This stays true on diagonal ramps and
    // under rotated gravity; projecting relative to the incoming tangent made
    // plain Left look like Down on an upslope and caused reverse loop re-entry.
    let Some(desired) = route_bias_direction(frame, local_axis) else {
        return Some(default);
    };
    let branch_bias = |branch: RouteBranch| branch.heading.dot(desired);
    let default_score = branch_bias(default);
    let (best_score, best) = candidates
        .iter()
        .copied()
        .map(|branch| (branch_bias(branch), branch))
        .max_by(|(a, _), (b, _)| a.total_cmp(b))?;

    if best.is_default || best_score > default_score + ROUTE_SWITCH_MARGIN {
        Some(best)
    } else {
        Some(default)
    }
}

/// Walk `ds` of arc from `s`, applying the joint rule at every segment join
/// crossed. An authored route switch may move the rider to another chain while
/// preserving signed speed and the unspent distance from this tick.
fn advance_riding(
    world: &World,
    mut on: SurfaceRef,
    s: f32,
    ds: f32,
    v_t: f32,
    motion_frame: MotionFrame,
    params: &MomentumParams,
    radius: f32,
    local_axis: Vec2,
) -> RideOutcome {
    let gravity = motion_frame.acceleration();
    let mut current = s;
    let mut remaining = ds;
    let mut routed_v_t = v_t;
    let max_hops = world
        .chains
        .iter()
        .map(SurfaceChain::segment_count)
        .sum::<usize>()
        + world.blocks.len() * 4
        + 8;

    for _ in 0..=max_hops {
        let chain = resolve_surface(world, on)
            .expect("advance_riding only follows live surfaces and validated route ports");
        let chain = chain.as_ref();
        let total = chain.total_length();
        if remaining == 0.0 {
            return RideOutcome::Riding {
                on,
                s: current,
                v_t: routed_v_t,
            };
        }
        let frame = chain.frame_at(current);
        let seg_start = arc_at_segment_start(chain, frame.segment);
        let seg_len = chain.segment_length(frame.segment);
        let to_join = if remaining > 0.0 {
            (seg_start + seg_len) - current
        } else {
            seg_start - current
        };
        if remaining.abs() <= to_join.abs() || (to_join.abs() < 1.0e-6 && remaining.abs() < 1.0e-6)
        {
            let landed = current + remaining;
            if !chain.closed && (landed < 0.0 || landed > total) {
                return RideOutcome::Launch {
                    on,
                    frame: departure_frame(chain, landed.clamp(0.0, total), frame.segment),
                    v_t: routed_v_t,
                };
            }
            return RideOutcome::Riding {
                on,
                s: landed,
                v_t: routed_v_t,
            };
        }

        let at_join = current + to_join;
        let at_open_end = !chain.closed && (at_join <= 0.0 || at_join >= total);
        remaining -= to_join;
        current = at_join;
        let seg_i = frame.segment;
        let travel_sign = remaining.signum();
        let current_vertex = if travel_sign > 0.0 {
            (seg_i + 1) % chain.points.len()
        } else {
            seg_i
        };
        if let Some(branch) = choose_route_branch(
            world,
            on,
            current_vertex,
            seg_i,
            travel_sign,
            motion_frame,
            local_axis,
        ) {
            if !branch.is_default {
                let Some(target) = resolve_surface(world, branch.on) else {
                    continue;
                };
                let target = target.as_ref();
                let branch_s = target.arc_at_vertex(branch.vertex);
                let nudge = joint_nudge(branch_s);
                current = branch_s + branch.direction * nudge;
                if target.closed {
                    current = current.rem_euclid(target.total_length());
                } else {
                    current = current.clamp(0.0, target.total_length());
                }
                on = branch.on;
                remaining = branch.direction * (remaining.abs() - nudge).max(0.0);
                routed_v_t = branch.direction * routed_v_t.abs();
                continue;
            }
        }
        if at_open_end {
            return RideOutcome::Launch {
                on,
                frame: departure_frame(chain, at_join.clamp(0.0, total), frame.segment),
                v_t: routed_v_t,
            };
        }
        let entered = if remaining > 0.0 {
            (seg_i + 1) % chain.segment_count()
        } else {
            (seg_i + chain.segment_count() - 1) % chain.segment_count()
        };
        let (auth_a, auth_b) = if remaining > 0.0 {
            (seg_i, entered)
        } else {
            (entered, seg_i)
        };
        let t_a = chain.tangent(auth_a);
        let t_b = chain.tangent(auth_b);
        let cross = t_a.perp_dot(t_b);
        if cross > 1.0e-6 {
            let angle = t_a.dot(t_b).clamp(-1.0, 1.0).acos();
            let n_entered = chain.normal(entered);
            let press = gravity.dot(-n_entered).max(0.0);
            let smoothing = radius.max(1.0);
            let demand = routed_v_t * routed_v_t * angle / smoothing;
            if demand > params.stick_factor * press {
                return RideOutcome::Launch {
                    on,
                    frame: departure_frame(chain, current, frame.segment),
                    v_t: routed_v_t,
                };
            }
        }
        let nudge = joint_nudge(current);
        current += remaining.signum() * nudge;
        remaining -= remaining.signum() * nudge;
        if chain.closed {
            current = current.rem_euclid(total);
        }
    }
    RideOutcome::Riding {
        on,
        s: current,
        v_t: routed_v_t,
    }
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
    frame: MotionFrame,
    inputs: SurfaceInputs,
    dt: f32,
    mut contacts: Option<&mut Vec<Contact>>,
) {
    // Ballistic + air control use the same frame the dispatcher supplied.
    let gravity = frame.acceleration();
    body.vel += gravity * dt;
    let run = inputs.local_axes.x.clamp(-1.0, 1.0);
    if run.abs() > 0.1 {
        // Air steering is authored along the body's local side axis.
        let side = frame.side();
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
    match first_circle_hit(world, body.pos, body.radius, body.depth_lane, delta) {
        Some(hit) => {
            body.pos += delta * hit.toi;
            let mut report_contact = true;
            // Landing is load-bearing: attach only to a surface gravity
            // presses the body onto (floors/up-slopes in the local gravity
            // frame). Walls and ceilings hit from the air DEFLECT — riding
            // them is reached by continuity, never by bonking.
            let press = gravity.dot(-hit.normal);
            if press > 0.0 {
                let (on, hit_segment) = match hit.what {
                    CircleHitTarget::Chain { chain, segment } => {
                        (SurfaceRef::Chain(chain), Some(segment))
                    }
                    CircleHitTarget::Block { block } => (SurfaceRef::Block(block), None),
                };
                let surface = resolve_surface(world, on)
                    .expect("first_circle_hit only reports live surfaces");
                let surface = surface.as_ref();
                // Preserve the segment reported by the sweep. A global nearest
                // projection is ambiguous at a 2.5D crossover where two route
                // occurrences share the same screen-space point; projecting
                // globally can teleport the rider back to the other visit and
                // make a reverse loop repeat forever.
                let s = hit_segment.map_or_else(
                    || surface.project(body.pos).0,
                    |segment| project_to_segment(surface, segment, body.pos),
                );
                let f = surface.frame_at(s);
                let rel = body.vel - per_frame_to_per_sec(surface.velocity, dt);
                let v_t = rel.dot(f.tangent);
                if leaving_an_open_end(surface, s, v_t) {
                    // The sweep touched only the endpoint of a chain the body is
                    // already leaving. This is not a collision: consume the
                    // remaining ballistic displacement instead of dropping the
                    // frame remainder at TOI=0. Merely declining to attach leaves
                    // the circle at the lip, where the same endpoint is reported
                    // forever on subsequent frames.
                    body.pos += delta * (1.0 - hit.toi);
                    report_contact = false;
                } else {
                    body.pos = f.point + f.normal * body.radius;
                    body.depth_lane = surface.segment_depth(f.segment);
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
            if report_contact {
                if let Some(sink) = contacts.as_deref_mut() {
                    // Landing (now Riding) is a support contact by construction;
                    // a deflect classifies frame-relatively (this solver has no
                    // structural wall pass).
                    let kind = if body.riding() {
                        crate::collision_semantics::ContactKind::Support
                    } else {
                        crate::collision_semantics::classify_contact_normal(
                            hit.normal,
                            frame.down(),
                        )
                    };
                    sink.push(Contact {
                        kind,
                        point: body.pos - hit.normal * body.radius,
                        normal: hit.normal,
                        toi: hit.toi,
                        surface_velocity: hit.surface_velocity,
                        source: hit.source,
                    });
                }
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
    Chain { chain: usize, segment: usize },
    Block { block: usize },
}

fn project_to_segment(chain: &SurfaceChain, segment: usize, point: Vec2) -> f32 {
    let (a, b) = chain.segment(segment);
    let ab = b - a;
    let len_sq = ab.length_squared();
    let t = if len_sq > 0.0 {
        ((point - a).dot(ab) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let start = chain.arc_at_vertex(segment);
    let len = chain.segment_length(segment);
    let raw = start + t * len;
    // Keep `frame_at` on the segment the sweep actually hit even at a shared
    // endpoint. This is a representational nudge, never a spatial pushout.
    if t <= 1.0e-5 {
        raw + joint_nudge(raw)
    } else if t >= 1.0 - 1.0e-5 {
        raw - joint_nudge(raw)
    } else {
        raw
    }
}

fn depth_lanes_collide(body_lane: i8, surface_lane: i8) -> bool {
    body_lane == surface_lane
}

/// Ignore a numerically immediate, nearly tangent chain contact.
///
/// A circle released from a polygonal track joint is exactly tangent to the
/// departure segment and may overlap the neighboring segment by a few
/// hundredths of a pixel. Parry reports that as a TOI-zero hit. Reattaching on
/// that hit creates the visible "caught on the rail" limit cycle. Genuine
/// landings have either meaningful separation before impact or a substantial
/// into-surface component, so they remain collision candidates.
fn grazing_chain_contact_at_release(
    center: Vec2,
    radius: f32,
    segment_start: Vec2,
    normal: Vec2,
    delta: Vec2,
    toi: f32,
) -> bool {
    const CONTACT_SLOP: f32 = 0.5;
    const TOI_EPSILON: f32 = 1.0e-4;
    const MAX_NORMAL_FRACTION: f32 = 0.12;

    if toi > TOI_EPSILON {
        return false;
    }
    let travel = delta.length();
    if travel <= 1.0e-6 {
        return false;
    }
    let signed_distance = (center - segment_start).dot(normal);
    let penetration = radius - signed_distance;
    let inward_distance = (-delta.dot(normal)).max(0.0);
    penetration > 0.0
        && penetration <= CONTACT_SLOP
        && inward_distance <= travel * MAX_NORMAL_FRACTION
}

/// Earliest swept-circle hit against chains (one-sided) and solid blocks.
fn first_circle_hit(
    world: &World,
    center: Vec2,
    radius: f32,
    depth_lane: i8,
    delta: Vec2,
) -> Option<CircleHit> {
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
            if !depth_lanes_collide(depth_lane, chain.segment_depth(i)) {
                continue;
            }
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
            if grazing_chain_contact_at_release(center, radius, a, n, delta, toi) {
                continue;
            }
            if best.as_ref().is_none_or(|b| toi < b.toi) {
                best = Some(CircleHit {
                    toi,
                    normal: n,
                    surface_velocity: chain.velocity,
                    source: ContactSource::Chain {
                        chain: ci as u32,
                        segment: i as u32,
                    },
                    what: CircleHitTarget::Chain {
                        chain: ci,
                        segment: i,
                    },
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
                    source: ContactSource::Block {
                        kind: block.kind,
                        id: block.id.clone(),
                    },
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
