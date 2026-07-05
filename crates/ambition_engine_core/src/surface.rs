//! The surface-follower solver — momentum locomotion over [`SurfaceChain`]s
//! (fable review 2026-07-05, AJ10 layer 3).
//!
//! The ONE new mover. A surface-momentum body is a **circle proxy** that is
//! either ballistic (`Airborne`) or attached to a chain (`Riding { chain, s,
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
//!   through. Blocks remain universal obstacles (swept circle vs solids); in
//!   v1 a block is an obstacle to slide along, not a stance — jumping happens
//!   from chains. One-ways/hazards/pogo/rebound blocks are gameplay-layer
//!   concerns, not follower collision (same split as the kinematic sweep).
//!
//! Everything here is vector math — no cardinal-axis assumptions — so the C4
//! rotation rig holds by construction (see tests).

use parry2d::{
    math::{Pose, Vector},
    query::{self, ShapeCastOptions},
    shape::{Ball, Cuboid, Segment},
};

use crate::collision_semantics::{is_full_collision_surface, Contact, ContactSource};
use crate::geometry::AabbExt;
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

/// Where the body is relative to the world's surfaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfaceMotion {
    Airborne,
    /// Attached to `world.chains[chain]` at arc length `s`, moving at scalar
    /// speed `v_t` along the tangent (signed: + = increasing arc length).
    Riding {
        chain: usize,
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
        SurfaceMotion::Riding { chain, s, v_t } => {
            step_riding(
                body,
                world,
                params,
                gravity,
                inputs,
                dt,
                chain,
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

#[allow(clippy::too_many_arguments)]
fn step_riding(
    body: &mut SurfaceBody,
    world: &World,
    params: &MomentumParams,
    gravity: Vec2,
    inputs: SurfaceInputs,
    dt: f32,
    chain_idx: usize,
    s: f32,
    mut v_t: f32,
    mut contacts: Option<&mut Vec<Contact>>,
) {
    let Some(chain) = world.chains.get(chain_idx) else {
        body.motion = SurfaceMotion::Airborne;
        return;
    };
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
            body.motion = SurfaceMotion::Riding {
                chain: chain_idx,
                s: new_s,
                v_t,
            };
            if let Some(sink) = contacts.as_deref_mut() {
                sink.push(Contact {
                    point: f.point,
                    normal: f.normal,
                    toi: 0.0,
                    surface_velocity: chain.velocity,
                    source: ContactSource::Chain {
                        chain: chain_idx as u32,
                        segment: f.segment as u32,
                    },
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
        current += remaining.signum() * 1.0e-4;
        remaining -= remaining.signum() * 1.0e-4;
        if chain.closed {
            current = current.rem_euclid(total);
        }
    }
    RideOutcome::Riding { s: current }
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
        let g_dir = gravity.normalize_or_zero();
        let side = Vec2::new(-g_dir.y, g_dir.x);
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
            match hit.what {
                CircleHitTarget::Chain { chain, .. } => {
                    let surface = &world.chains[chain];
                    let (s, _) = surface.project(body.pos);
                    let f = surface.frame_at(s);
                    body.pos = f.point + f.normal * body.radius;
                    let rel = body.vel - per_frame_to_per_sec(surface.velocity, dt);
                    let v_t = rel.dot(f.tangent);
                    body.motion = SurfaceMotion::Riding { chain, s, v_t };
                    body.vel = v_t * f.tangent + per_frame_to_per_sec(surface.velocity, dt);
                }
                CircleHitTarget::Block { .. } => {
                    // Slide: kill the into-surface velocity component; the
                    // remainder of this frame's motion is dropped (v1 — one
                    // swept TOI per frame, never a pushout).
                    let n = hit.normal;
                    let into = body.vel.dot(-n).max(0.0);
                    body.vel += into * n;
                }
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
    Block {},
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

    // Solid blocks: universal obstacles.
    for block in &world.blocks {
        if !is_full_collision_surface(block.kind) {
            continue;
        }
        let half = block.aabb.half_size();
        let cuboid = Cuboid::new(Vector::new(half.x.max(0.0), half.y.max(0.0)));
        let block_center = block.aabb.center();
        let Ok(Some(hit)) = query::cast_shapes(
            &pose,
            vel,
            &ball,
            &Pose::translation(block_center.x, block_center.y),
            Vector::ZERO,
            &cuboid,
            options,
        ) else {
            continue;
        };
        let toi = hit.time_of_impact.clamp(0.0, 1.0);
        // Parry's normal1 is the moving shape's outward normal; the surface
        // normal is its negation.
        let n = -Vec2::new(hit.normal1.x, hit.normal1.y);
        if n.dot(delta) >= 0.0 {
            continue; // grazing/leaving contact, not a blocker
        }
        if best.as_ref().is_none_or(|b| toi < b.toi) {
            best = Some(CircleHit {
                toi,
                normal: n,
                surface_velocity: block.velocity,
                source: ContactSource::Block { kind: block.kind },
                what: CircleHitTarget::Block {},
            });
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
mod tests {
    use super::*;
    use crate::world::SurfaceChain;

    const DT: f32 = 1.0 / 60.0;
    const G: Vec2 = Vec2::new(0.0, 1450.0);

    fn frictionless() -> MomentumParams {
        MomentumParams {
            friction: 0.0,
            ..Default::default()
        }
    }

    fn world_with_chains(chains: Vec<SurfaceChain>) -> World {
        World::new(
            "surface-test",
            Vec2::new(4000.0, 4000.0),
            Vec2::ZERO,
            Vec::new(),
        )
        .with_chains(chains)
    }

    /// A V-valley: down-slope, flat bottom, up-slope (authored left→right so
    /// normals face up).
    fn valley() -> SurfaceChain {
        SurfaceChain::open(
            "valley",
            vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(400.0, 300.0),
                Vec2::new(800.0, 300.0),
                Vec2::new(1200.0, 0.0),
            ],
        )
    }

    /// A 16-gon loop, interior-rideable winding (negative shoelace area),
    /// centered at `c` with radius `r`, starting at the bottom point.
    fn loop_chain(c: Vec2, r: f32) -> SurfaceChain {
        let n = 16;
        let mut pts = Vec::new();
        for k in 0..n {
            // Start at the bottom; wind so the interior stays on the +normal
            // side (negative shoelace = interior-rideable).
            let ang = std::f32::consts::TAU * (k as f32) / (n as f32);
            let (sin, cos) = ang.sin_cos();
            pts.push(c + Vec2::new(r * sin, r * cos));
        }
        let chain = SurfaceChain::closed_loop("loop", pts);
        assert!(chain.validate().is_empty(), "{:?}", chain.validate());
        assert!(chain.signed_area() < 0.0, "interior-rideable winding");
        chain
    }

    fn ride(chain_idx: usize, s: f32, v_t: f32, world: &World, radius: f32) -> SurfaceBody {
        let f = world.chains[chain_idx].frame_at(s);
        let mut b = SurfaceBody::new(f.point + f.normal * radius, radius);
        b.motion = SurfaceMotion::Riding {
            chain: chain_idx,
            s,
            v_t,
        };
        b
    }

    #[test]
    fn slope_accelerates_downhill_and_energy_never_grows() {
        let world = world_with_chains(vec![valley()]);
        // Start at rest near the top of the left slope.
        let mut body = ride(0, 10.0, 0.0, &world, 14.0);
        let start_height = -body.pos.y; // y-down: height = -y
        let params = frictionless();
        let mut max_speed: f32 = 0.0;
        let mut max_height_after_start = f32::MIN;
        let mut left_start = false;
        for _ in 0..600 {
            step_surface_body(
                &mut body,
                &world,
                &params,
                G,
                SurfaceInputs::default(),
                DT,
                None,
            );
            max_speed = max_speed.max(body.vel.length());
            if let SurfaceMotion::Riding { s, .. } = body.motion {
                if s > 100.0 {
                    left_start = true;
                }
                if left_start {
                    max_height_after_start = max_height_after_start.max(-body.pos.y);
                }
            }
        }
        assert!(left_start, "gravity pulled the body down the slope");
        assert!(max_speed > 500.0, "gained real speed downhill: {max_speed}");
        // Energy sanity: it never climbs ABOVE its start height (small
        // integration slack allowed).
        assert!(
            max_height_after_start <= start_height + 2.0,
            "energy grew: start height {start_height}, reached {max_height_after_start}"
        );
    }

    #[test]
    fn uphill_decelerates_and_the_body_oscillates_in_the_valley() {
        let world = world_with_chains(vec![valley()]);
        let mut body = ride(0, 10.0, 0.0, &world, 14.0);
        let params = frictionless();
        // Track the sign of v_t: it must flip at least twice (down, up the
        // far slope, back down) within a few seconds.
        let mut flips = 0;
        let mut last_sign = 0.0f32;
        for _ in 0..900 {
            step_surface_body(
                &mut body,
                &world,
                &params,
                G,
                SurfaceInputs::default(),
                DT,
                None,
            );
            if let SurfaceMotion::Riding { v_t, .. } = body.motion {
                let sign = if v_t > 10.0 {
                    1.0
                } else if v_t < -10.0 {
                    -1.0
                } else {
                    last_sign
                };
                if last_sign != 0.0 && sign != last_sign {
                    flips += 1;
                }
                last_sign = sign;
            }
        }
        assert!(flips >= 2, "valley oscillation (got {flips} turn flips)");
    }

    #[test]
    fn input_cannot_exceed_top_speed_but_slopes_can() {
        let flat = SurfaceChain::open(
            "flat",
            vec![Vec2::new(0.0, 300.0), Vec2::new(20000.0, 300.0)],
        );
        let world = world_with_chains(vec![flat]);
        let mut body = ride(0, 10.0, 0.0, &world, 14.0);
        let params = frictionless();
        for _ in 0..600 {
            step_surface_body(
                &mut body,
                &world,
                &params,
                G,
                SurfaceInputs {
                    run: 1.0,
                    ..Default::default()
                },
                DT,
                None,
            );
        }
        let SurfaceMotion::Riding { v_t, .. } = body.motion else {
            panic!("still riding the flat");
        };
        assert!(
            (v_t - params.top_speed).abs() < 20.0,
            "input holds top speed: {v_t}"
        );
        // A steep downhill exceeds it.
        let world2 = world_with_chains(vec![valley()]);
        let mut fast = ride(0, 10.0, params.top_speed, &world2, 14.0);
        for _ in 0..25 {
            step_surface_body(
                &mut fast,
                &world2,
                &params,
                G,
                SurfaceInputs {
                    run: 1.0,
                    ..Default::default()
                },
                DT,
                None,
            );
        }
        if let SurfaceMotion::Riding { v_t, .. } = fast.motion {
            assert!(v_t > params.top_speed + 50.0, "slope exceeds cap: {v_t}");
        }
    }

    #[test]
    fn loop_completes_above_threshold_speed_and_sheds_below() {
        let world = world_with_chains(vec![loop_chain(Vec2::new(500.0, 300.0), 150.0)]);
        let params = frictionless();
        // FAST: launch around the loop from the bottom.
        let mut fast = ride(0, 1.0, 1400.0, &world, 14.0);
        let mut reached_top = false;
        let mut always_riding = true;
        for _ in 0..240 {
            step_surface_body(
                &mut fast,
                &world,
                &params,
                G,
                SurfaceInputs::default(),
                DT,
                None,
            );
            match fast.motion {
                SurfaceMotion::Riding { .. } => {
                    // Top of the loop = body above the center.
                    if fast.pos.y < 300.0 - 100.0 {
                        reached_top = true;
                    }
                }
                SurfaceMotion::Airborne => always_riding = false,
            }
            if reached_top && matches!(fast.motion, SurfaceMotion::Riding { .. }) {
                break;
            }
        }
        assert!(
            reached_top && always_riding,
            "fast body rides through the loop top (reached_top={reached_top}, always_riding={always_riding})"
        );
        // SLOW: cannot complete the loop — it either sheds off the wall or
        // oscillates in the bowl (halfpipe-style); it must never reach the top.
        let mut slow = ride(0, 1.0, 600.0, &world, 14.0);
        let mut slow_reached_top = false;
        for _ in 0..240 {
            step_surface_body(
                &mut slow,
                &world,
                &params,
                G,
                SurfaceInputs::default(),
                DT,
                None,
            );
            if matches!(slow.motion, SurfaceMotion::Riding { .. }) && slow.pos.y < 300.0 - 100.0 {
                slow_reached_top = true;
            }
        }
        assert!(!slow_reached_top, "slow body must not complete the loop");
    }

    #[test]
    fn open_ramp_end_launches_with_the_end_tangent() {
        let ramp = SurfaceChain::open(
            "launch-ramp",
            vec![
                Vec2::new(0.0, 300.0),
                Vec2::new(300.0, 300.0),
                Vec2::new(500.0, 200.0),
            ],
        );
        let world = world_with_chains(vec![ramp]);
        let params = frictionless();
        let mut body = ride(0, 10.0, 800.0, &world, 14.0);
        let mut launched_vel = None;
        for _ in 0..120 {
            step_surface_body(
                &mut body,
                &world,
                &params,
                G,
                SurfaceInputs::default(),
                DT,
                None,
            );
            if matches!(body.motion, SurfaceMotion::Airborne) {
                launched_vel = Some(body.vel);
                break;
            }
        }
        let v = launched_vel.expect("ran off the ramp end");
        // The end tangent points up-and-right: (200,-100)/|.| — the launch
        // velocity must be along it (within a step of gravity).
        let t = (Vec2::new(500.0, 200.0) - Vec2::new(300.0, 300.0)).normalize();
        let along = v.normalize().dot(t);
        assert!(
            along > 0.98,
            "launched along the ramp tangent: {v:?} vs {t:?}"
        );
    }

    #[test]
    fn convex_crest_launches_at_speed_and_follows_at_a_walk() {
        // A flat run into a gentle downhill: convex joint.
        let crest = SurfaceChain::open(
            "crest",
            vec![
                Vec2::new(0.0, 300.0),
                Vec2::new(400.0, 300.0),
                Vec2::new(800.0, 500.0),
                Vec2::new(1600.0, 900.0),
            ],
        );
        let world = world_with_chains(vec![crest]);
        let params = frictionless();
        // FAST over the crest: leaves the ground.
        let mut fast = ride(0, 10.0, 1200.0, &world, 14.0);
        let mut went_airborne = false;
        for _ in 0..60 {
            step_surface_body(
                &mut fast,
                &world,
                &params,
                G,
                SurfaceInputs::default(),
                DT,
                None,
            );
            if matches!(fast.motion, SurfaceMotion::Airborne) {
                went_airborne = true;
                break;
            }
        }
        assert!(went_airborne, "fast body launches off the crest");
        // Slow walk: follows the surface over the joint.
        let mut slow = ride(0, 380.0, 120.0, &world, 14.0);
        let mut crossed_riding = false;
        for _ in 0..120 {
            step_surface_body(
                &mut slow,
                &world,
                &params,
                G,
                SurfaceInputs {
                    run: 1.0,
                    ..Default::default()
                },
                DT,
                None,
            );
            if let SurfaceMotion::Riding { s, .. } = slow.motion {
                if s > 450.0 {
                    crossed_riding = true;
                    break;
                }
            } else {
                break;
            }
        }
        assert!(crossed_riding, "slow body follows the crest joint");
    }

    #[test]
    fn airborne_body_lands_on_a_chain_and_starts_riding() {
        let floor = SurfaceChain::open(
            "floor",
            vec![Vec2::new(0.0, 400.0), Vec2::new(1000.0, 400.0)],
        );
        let world = world_with_chains(vec![floor]);
        let params = MomentumParams::default();
        let mut body = SurfaceBody::new(Vec2::new(500.0, 200.0), 14.0);
        let mut contacts = Vec::new();
        for _ in 0..240 {
            step_surface_body(
                &mut body,
                &world,
                &params,
                G,
                SurfaceInputs::default(),
                DT,
                Some(&mut contacts),
            );
            if body.riding() {
                break;
            }
        }
        assert!(body.riding(), "fell onto the chain and stuck");
        // Snapped to the surface: center = point + normal*radius.
        assert!(
            (body.pos.y - (400.0 - 14.0)).abs() < 0.5,
            "pos {:?}",
            body.pos
        );
        // The landing contact is a Chain source with an up normal.
        assert!(contacts
            .iter()
            .any(|c| matches!(c.source, ContactSource::Chain { .. })
                && (c.normal - Vec2::new(0.0, -1.0)).length() < 1e-3));
    }

    #[test]
    fn chains_are_one_sided_a_body_passes_from_behind() {
        let floor = SurfaceChain::open(
            "floor",
            vec![Vec2::new(0.0, 400.0), Vec2::new(1000.0, 400.0)],
        );
        let world = world_with_chains(vec![floor]);
        let params = MomentumParams::default();
        // Below the floor moving up: approaches from the solid side.
        let mut body = SurfaceBody::new(Vec2::new(500.0, 500.0), 14.0);
        body.vel = Vec2::new(0.0, -900.0);
        for _ in 0..30 {
            step_surface_body(
                &mut body,
                &world,
                &params,
                G,
                SurfaceInputs::default(),
                DT,
                None,
            );
        }
        assert!(
            !body.riding() && body.pos.y < 400.0,
            "passed through from the back side: {:?}",
            body.pos
        );
    }

    #[test]
    fn airborne_body_sweeps_into_solid_blocks_no_tunneling() {
        let mut world = world_with_chains(vec![]);
        world.blocks.push(crate::world::Block::solid(
            "wall",
            Vec2::new(600.0, 0.0),
            Vec2::new(50.0, 1000.0),
        ));
        let params = MomentumParams::default();
        let mut body = SurfaceBody::new(Vec2::new(100.0, 500.0), 14.0);
        body.vel = Vec2::new(30000.0, 0.0); // 500px per frame: must not tunnel
        let mut contacts = Vec::new();
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs::default(),
            DT,
            Some(&mut contacts),
        );
        assert!(
            body.pos.x <= 600.0 - 14.0 + 0.5,
            "stopped at the wall face: {:?}",
            body.pos
        );
        assert!(body.vel.x.abs() < 1.0, "into-wall velocity killed");
        assert!(contacts
            .iter()
            .any(|c| (c.normal - Vec2::new(-1.0, 0.0)).length() < 1e-3));
    }

    #[test]
    fn jump_leaves_along_the_surface_normal_with_tangent_momentum() {
        let floor = SurfaceChain::open(
            "floor",
            vec![Vec2::new(0.0, 400.0), Vec2::new(2000.0, 400.0)],
        );
        let world = world_with_chains(vec![floor]);
        let params = MomentumParams::default();
        let mut body = ride(0, 500.0, 600.0, &world, 14.0);
        step_surface_body(
            &mut body,
            &world,
            &params,
            G,
            SurfaceInputs {
                run: 0.0,
                jump_pressed: true,
            },
            DT,
            None,
        );
        assert!(!body.riding());
        assert!(body.vel.x > 550.0, "kept tangent momentum: {:?}", body.vel);
        assert!(
            body.vel.y < -500.0,
            "launched along +normal (up): {:?}",
            body.vel
        );
    }

    /// THE C4 rig: the whole scenario rotated 90° (points and gravity) must
    /// produce the SAME trajectory rotated 90°. The follower is pure vector
    /// math — no cardinal-axis assumptions — so this holds tightly.
    #[test]
    fn c4_rotation_symmetry_the_rotated_valley_matches() {
        // Exact 90° rotation in f32: (x, y) -> (y, -x) (plus a translation to
        // keep coordinates positive; translations are exact for these values).
        let rot = |p: Vec2| Vec2::new(p.y, -p.x) + Vec2::new(0.0, 2000.0);
        let rot_v = |p: Vec2| Vec2::new(p.y, -p.x);

        let chain_a = valley();
        let chain_b = SurfaceChain::open(
            "valley-rot",
            chain_a.points.iter().map(|&p| rot(p)).collect(),
        );
        let world_a = world_with_chains(vec![chain_a]);
        let world_b = world_with_chains(vec![chain_b]);
        let params = frictionless();
        let g_a = G;
        let g_b = rot_v(G);

        let mut a = ride(0, 10.0, 0.0, &world_a, 14.0);
        let mut b = ride(0, 10.0, 0.0, &world_b, 14.0);
        for frame in 0..600 {
            let input = SurfaceInputs {
                // Scripted input: run right for 2s, coast, then brake.
                run: if frame < 120 {
                    1.0
                } else if frame < 300 {
                    0.0
                } else {
                    -1.0
                },
                jump_pressed: frame == 360,
            };
            step_surface_body(&mut a, &world_a, &params, g_a, input, DT, None);
            step_surface_body(&mut b, &world_b, &params, g_b, input, DT, None);
            let mapped = rot(a.pos);
            // Sub-pixel agreement: the translation in `rot` shifts f32
            // rounding between the two runs, so exact bit-equality is not
            // available — but half a pixel over 10 seconds of riding, joint
            // crossings, launches, and ballistic fall IS the symmetry.
            assert!(
                (mapped - b.pos).length() < 0.5,
                "frame {frame}: rotated trajectory diverged: {:?} vs {:?} (orig {:?})",
                mapped,
                b.pos,
                a.pos
            );
            assert_eq!(a.riding(), b.riding(), "frame {frame}: state diverged");
        }
    }
}
