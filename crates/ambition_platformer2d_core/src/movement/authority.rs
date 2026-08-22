//! The body-motion AUTHORITIES besides the kernel (ADR 0024).
//!
//! Every production write to authoritative body pose/velocity belongs to
//! exactly one named authority:
//!
//! 1. Continuous integration — [`super::step_motion`], the movement kernel.
//! 2. Discrete transit — [`transit_body`]: blink and dive arrivals, recall,
//!    portal exits, respawns, room placement, scripted teleports. A transit is
//!    NOT a fake physics tick; it deliberately reconciles contact, attachment,
//!    and model-private state (semantics below).
//! 3. External kinematic constraint — [`carry_body`] (parent-frame carry:
//!    moving-platform ledge carry, attractor pull, straddle eviction) and
//!    [`constrain_body_pose`] (absolute pin: a mount's saddle, a scripted
//!    flagpole slide).
//! 4. Impulses — typed velocity operations that consume the body's resolved
//!    frame: [`super::set_jump_velocity`],
//!    [`AccelerationFrame::launch`](crate::AccelerationFrame::launch), and
//!    frame-rotated `vel +=` writes at combat/ability seams.
//!
//! Anything else writing `BodyKinematics.pos` in production is an authority
//! leak (guarded by workspace policy).

use super::model::MotionModel;
use crate::body_clusters::BodyClustersMut;
use crate::{SweepSample, Vec2};

/// What a ROOM ARRIVAL does to the body's incoming velocity.
///
///  this exists because the follow-up-call defect regrew.
/// [`crate::reset_body_clusters`]'s own doc records the lesson in as many words —
/// *"An authority that requires a follow-up call is not an authority — it is a
/// two-step ritual, and the second step is the one people forget"* — and then two
/// call sites grew exactly such a ritual around it, one layer up:
///
/// ```ignore
/// let old_velocity = clusters.kinematics.vel;
/// let fly_enabled  = clusters.flight.fly_enabled;
/// ae::reset_body_clusters(model, clusters, arrival, air_jumps);
/// clusters.flight.fly_enabled = fly_enabled && clusters.abilities.abilities.fly;
/// if edge_exit { clusters.kinematics.vel = old_velocity; }
/// ```
///
/// Five lines, character-for-character identical, in `ambition_platformer2d_actor_monolith`'s
/// room load and `ambition_platformer2d_runtime`'s lifecycle commit — two crates,
/// one arrival authority, no shared name. Surfaced by
/// `engine.velocity-writes-are-authority-only` on its first run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrivalMomentum {
    /// Arrive at rest: a door, a respawn, a scripted placement. The room change
    /// is a new beginning and the body's old speed means nothing in it.
    Reset,
    /// Keep the incoming velocity: an EDGE EXIT is the same run continuing
    /// through a seam, so a player who walked off the right edge at full tilt
    /// keeps walking.
    Preserve,
}

/// THE room-arrival authority: reset the body for its new room, then apply the
/// arrival's momentum policy — in one call, so neither half can be forgotten.
///
/// Restores flight only if the body still HAS the ability, which is the rule both
/// hand-written copies implemented and neither named.
pub fn arrive_body_in_room(
    model: &mut MotionModel,
    clusters: &mut BodyClustersMut<'_>,
    spawn: Vec2,
    air_jumps_default: u8,
    momentum: ArrivalMomentum,
) {
    let incoming = clusters.kinematics.vel;
    let fly_enabled = clusters.flight.fly_enabled;
    crate::reset_body_clusters(model, clusters, spawn, air_jumps_default);
    let abilities = clusters.abilities.abilities;
    clusters.flight.fly_enabled = if abilities.fly && !abilities.fly_toggle {
        true
    } else {
        fly_enabled && abilities.fly
    };
    if momentum == ArrivalMomentum::Preserve {
        //  AND THE RITUAL REGREW INSIDE THE FUNCTION WRITTEN TO KILL IT.
        //
        // `reset_body_clusters` transits at `TransitVelocity:Zero`, and a transit rebuilds the
        // collapsed `SweepSample` FROM the velocity it sees — so restoring `kinematics.vel`
        // afterwards left the body moving and its sweep sample stationary. Two motion facts,
        // one body, disagreeing: exactly the "authority that requires a follow-up call" this
        // type's own doc comment exists to name, one layer further in.
        //
        // Reconciling AFTER the momentum policy is what makes the arrival one
        // operation: whatever velocity the body leaves here with is the velocity
        // every derived motion fact was built from.
        clusters.kinematics.vel = incoming;
        reconcile_transit(model, clusters);
    }
}

/// What a transit does to the body's velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransitVelocity {
    /// Keep the pre-transit velocity (a blink preserves momentum).
    Keep,
    /// Arrive at rest (a respawn).
    Zero,
    /// Arrive with an explicitly transformed velocity (a portal's rotated
    /// exit velocity, a room hand-off's carried velocity).
    Set(Vec2),
}

/// THE discrete-transit authority: teleport a body to `pos` and reconcile
/// every fact that was true only of the departure point.
///
/// Reconciliation semantics (deliberate, documented, uniform):
/// - Contacts are invalidated: support (`on_ground`), wall contact, and any
///   in-flight wall cling/climb are cleared — they described surfaces at the
///   departure point. The destination re-acquires them through the ordinary
///   same-tick contact rules of the active policy, never by nearest-surface
///   guessing here.
/// - A ledge grab is released: its anchor is positional.
/// - Model-private attachment is invalidated: a riding momentum body
///   arrives `Airborne`; an attached crawler arrives detached. Axis maneuver
///   state (coyote, buffers, dash timers) is deliberately KEPT — those are
///   time facts, not place facts.
/// - The §3.1 motion record collapses to a zero-length sample at the
///   arrival: a transit is never a swept path (CC2 — a blink over spikes is
///   not a graze), and post-transit observers must not see the stale departure
///   segment.
pub fn transit_body(
    model: &mut MotionModel,
    clusters: &mut BodyClustersMut<'_>,
    pos: Vec2,
    velocity: TransitVelocity,
) {
    clusters.kinematics.pos = pos;
    match velocity {
        TransitVelocity::Keep => {}
        TransitVelocity::Zero => clusters.kinematics.vel = Vec2::ZERO,
        TransitVelocity::Set(vel) => clusters.kinematics.vel = vel,
    }
    reconcile_transit(model, clusters);
}

/// The reconciliation half of [`transit_body`], for transit implementations
/// that necessarily write the pose themselves (the portal core moves ANY
/// `BodyKinematics`, including cluster-less projectiles; its Ambition adapter
/// completes the kernel-body reconciliation with this after the crossing).
pub fn reconcile_transit(model: &mut MotionModel, clusters: &mut BodyClustersMut<'_>) {
    clusters.ground.invalidate();
    clusters.wall.on_wall = false;
    clusters.wall.wall_normal_x = 0.0;
    match model {
        MotionModel::AxisSwept(axis) => {
            // Wall engagement and the ledge anchor were facts of the departure
            // point; axis maneuver TIME facts (coyote, buffers, dash timers)
            // are deliberately kept.
            axis.state.wall_clinging = false;
            axis.state.wall_climbing = false;
            axis.state.ledge_grab = None;
        }
        MotionModel::SurfaceMomentum(momentum) => {
            momentum.state = super::surface_momentum::SurfaceMotion::Airborne;
        }
        MotionModel::AdhesiveCrawler(crawler) => crawler.detach(),
    }
    let pos = clusters.kinematics.pos;
    if let Some(sweep) = clusters.sweep.as_deref_mut() {
        *sweep = SweepSample {
            prev: pos,
            curr: pos,
            vel: clusters.kinematics.vel,
            half: clusters.kinematics.size * 0.5,
        };
    }
}

/// External kinematic CARRY: move the body with its parent frame by `delta`
/// (a moving platform carrying a ledge-grabber, an attractor's pull, the
/// portal-close straddle eviction). Contacts, attachment, and velocity are
/// deliberately untouched — a carried body is still supported/held; the next
/// kernel step re-resolves contact from the carried pose.
pub fn carry_body(kinematics: &mut crate::body_clusters::BodyKinematics, delta: Vec2) {
    kinematics.pos += delta;
}

/// External kinematic PIN: hold the body at an absolute pose with an imposed
/// velocity (a mount's saddle, a scripted end-of-level slide). The constraint
/// owner is the body's motion authority while engaged; like [`carry_body`] it
/// does not fabricate or clear contact facts.
pub fn constrain_body_pose(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    pos: Vec2,
    vel: Vec2,
) {
    kinematics.pos = pos;
    kinematics.vel = vel;
}

/// THE FROZEN TICK — the fifth authority: what may change a body the kernel
/// is about to step with `dt == 0`, where nothing integrates and nothing sweeps.
///
/// Clear the body's velocity because it has left play. Cleared rather than
/// frozen: the window ends in a respawn or reset, and a retained velocity would
/// be spent the instant the body came back. Idempotent, so a rollback may re-run
/// it.
pub fn halt_body(kinematics: &mut crate::body_clusters::BodyKinematics) {
    kinematics.vel = Vec2::ZERO;
}

/// SDI: the one thing a body may still do while frozen in hitlag.
///
/// Hitlag is a WINDOW rather than merely a pause, and this is what makes it one
/// — the victim shifts itself out of the next hit's way while the current one is
/// still stopped. Its offensive twin, DI, rides the launch the same freeze
/// precedes.
///
/// A pose write, and a SWEPT one: the frozen tick calls the kernel with
/// `dt == 0` and the kernel returns before its collision pass, so this is the
/// only place the displacement meets the world. An unswept shift lets a body
/// walk into or through geometry over a hitlag window.
pub fn shift_frozen_body(
    world: &crate::World,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    gravity_dir: Vec2,
    shift: Vec2,
) {
    if shift == Vec2::ZERO {
        return;
    }
    let body = kinematics.aabb_oriented(gravity_dir);
    // Only FULL solids stop a frozen nudge. One-way and bonk-only blocks are
    // directional by definition — a body standing on a platform may still SDI
    // sideways along it — and a blink wall is solid to a body that is not
    // blinking.
    let blocked = world.first_body_sweep(body, shift, |block| {
        matches!(
            block.kind,
            crate::BlockKind::Solid | crate::BlockKind::BlinkWall { .. }
        )
    });
    let allowed = match blocked {
        // Stop just short of the face rather than exactly on it, so the next
        // moving tick starts outside the block and its own sweep has a path to
        // resolve rather than a zero-length one from inside.
        Some(hit) => shift * (hit.time_of_impact - SKIN).max(0.0),
        None => shift,
    };
    kinematics.pos += allowed;
}

/// How far short of a contact face a frozen shift stops, in units of the
/// requested displacement. Small enough not to be seen, large enough that the
/// body is outside the block when the next tick sweeps.
const SKIN: f32 = 1.0e-3;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::adhesive_crawler::CrawlerState;

    #[test]
    fn a_halted_body_keeps_its_pose_and_loses_its_velocity() {
        let mut kin = crate::body_clusters::BodyKinematics {
            pos: Vec2::new(10.0, 20.0),
            vel: Vec2::new(300.0, -120.0),
            ..Default::default()
        };
        halt_body(&mut kin);
        assert_eq!(kin.vel, Vec2::ZERO);
        assert_eq!(kin.pos, Vec2::new(10.0, 20.0), "halting is not a teleport");
        halt_body(&mut kin);
        assert_eq!(kin.vel, Vec2::ZERO, "and it is idempotent under a rewind");
    }

    /// A wall whose NEAR FACE is at `wall_x` — `Block::solid` takes min + size,
    /// so the face a body approaches from the left is the min.
    fn walled_world(wall_x: f32, thickness: f32) -> crate::World {
        crate::World::new(
            "frozen_shift",
            Vec2::new(2000.0, 1000.0),
            Vec2::new(100.0, 100.0),
            vec![crate::Block::solid(
                "wall",
                Vec2::new(wall_x, 0.0),
                Vec2::new(thickness, 1000.0),
            )],
        )
    }

    fn body_at(x: f32) -> crate::body_clusters::BodyKinematics {
        crate::body_clusters::BodyKinematics {
            pos: Vec2::new(x, 500.0),
            size: Vec2::new(20.0, 40.0),
            ..Default::default()
        }
    }

    /// SDI is swept during hitlag and may not enter a wall.
    #[test]
    fn a_frozen_shift_stops_at_a_wall_instead_of_entering_it() {
        let world = walled_world(600.0, 40.0);
        let wall_near_face = 600.0;
        let mut kin = body_at(wall_near_face - 10.0 - 3.0);
        let before = kin.pos.x;

        shift_frozen_body(&world, &mut kin, Vec2::new(0.0, 1.0), Vec2::new(8.0, 0.0));

        assert!(
            kin.pos.x > before,
            "the shift moved nothing ({before} -> {}), so this proves nothing \
             about where it stopped",
            kin.pos.x
        );
        assert!(
            kin.pos.x + 10.0 <= wall_near_face,
            "SDI put the body's right edge at {}, past the wall face at \
             {wall_near_face} — a frozen displacement is not exempt from the world",
            kin.pos.x + 10.0
        );
    }

    /// AND IT MAY NOT CROSS A THIN ONE. The tunnelling case: a wall thinner
    /// than the requested shift is exactly what an unswept `pos +=` steps over.
    #[test]
    fn a_frozen_shift_cannot_tunnel_a_thin_wall() {
        let world = walled_world(600.0, 4.0);
        let mut kin = body_at(600.0 - 10.0 - 1.0);

        shift_frozen_body(&world, &mut kin, Vec2::new(0.0, 1.0), Vec2::new(40.0, 0.0));

        assert!(
            kin.pos.x + 10.0 <= 600.0,
            "the body ended at {} with its right edge past a 4px wall it should \
             have hit — the shift crossed the geometry without sweeping it",
            kin.pos.x
        );
    }

    /// REPEATED HITLAG TICKS DO NOT ACCUMULATE THROUGH IT EITHER. One tick
    /// stopping short is not the claim; a hitlag window is many ticks long.
    #[test]
    fn many_frozen_shifts_never_add_up_to_entering_the_wall() {
        let world = walled_world(600.0, 40.0);
        let wall_near_face = 600.0;
        let mut kin = body_at(500.0);

        for _ in 0..60 {
            shift_frozen_body(&world, &mut kin, Vec2::new(0.0, 1.0), Vec2::new(3.0, 0.0));
        }

        assert!(
            kin.pos.x + 10.0 <= wall_near_face,
            "sixty 3px SDI ticks put the body's edge at {}, past the face at \
             {wall_near_face}",
            kin.pos.x + 10.0
        );
        assert!(
            kin.pos.x > 500.0,
            "the body never advanced at all, so the bound above is vacuous"
        );
    }

    /// AND UNDER SIDEWAYS GRAVITY the body's box is oriented before it sweeps.
    /// The frame is not decoration here: an oriented body is 40 wide rather than
    /// 20, so a shift that fits under down-gravity does not under wall-gravity.
    #[test]
    fn a_frozen_shift_sweeps_the_oriented_box() {
        let world = walled_world(600.0, 40.0);
        let mut kin = body_at(560.0);
        let sideways = Vec2::new(1.0, 0.0);

        shift_frozen_body(&world, &mut kin, sideways, Vec2::new(30.0, 0.0));

        let half_width = kin.aabb_oriented(sideways).max.x - kin.pos.x;
        assert!(
            kin.pos.x + half_width <= 600.0 + 1.0e-3,
            "the oriented body ({half_width} half-width) ended overlapping the \
             wall at {}",
            kin.pos.x
        );
    }

    #[test]
    fn a_frozen_shift_moves_the_pose_and_leaves_the_velocity_alone() {
        let mut kin = crate::body_clusters::BodyKinematics {
            pos: Vec2::new(10.0, 20.0),
            vel: Vec2::new(300.0, -120.0),
            ..Default::default()
        };
        let open = crate::World::new("open", Vec2::splat(2000.0), Vec2::splat(100.0), Vec::new());
        shift_frozen_body(&open, &mut kin, Vec2::new(0.0, 1.0), Vec2::new(-2.0, 0.5));
        assert_eq!(kin.pos, Vec2::new(8.0, 20.5));
        assert_eq!(
            kin.vel,
            Vec2::new(300.0, -120.0),
            "SDI displaces the body; it does not re-aim the launch that follows"
        );
    }
    use crate::movement::surface_momentum::{SurfaceMotion, SurfaceRef};
    use crate::{AbilitySet, BodyClusterScratch, CrawlerParams, MomentumParams};

    /// A PRESERVED arrival leaves every motion fact telling the same story.
    ///
    ///  testing `transit_body` proves nothing about this. The transit is
    /// coherent on its own; the disagreement was manufactured one layer up, by
    /// `arrive_body_in_room` restoring the velocity AFTER the reset had already
    /// collapsed the sweep at zero. So this drives the wrapper and asserts the
    /// whole arrival — position, velocity, and the sweep sample derived from
    /// them — rather than the piece that was never wrong.
    #[test]
    fn a_preserved_arrival_leaves_the_sweep_agreeing_with_the_velocity() {
        let arrival = Vec2::new(640.0, 96.0);
        let incoming = Vec2::new(420.0, -30.0);

        let mut scratch =
            BodyClusterScratch::new_with_abilities(Vec2::new(10.0, 10.0), AbilitySet::default());
        scratch.kinematics.vel = incoming;
        let mut model = MotionModel::axis_swept(crate::AxisSweptParams::default());
        // The scratch body carries no sweep sample unless a test asks for one,
        // and a test that did not ask would pass over an absent fact.
        let mut sample = SweepSample {
            prev: Vec2::new(10.0, 10.0),
            curr: Vec2::new(10.0, 10.0),
            vel: incoming,
            half: Vec2::splat(8.0),
        };
        let (pos, vel) = {
            let mut clusters = scratch.as_mut();
            clusters.sweep = Some(&mut sample);
            arrive_body_in_room(
                &mut model,
                &mut clusters,
                arrival,
                1,
                ArrivalMomentum::Preserve,
            );
            (clusters.kinematics.pos, clusters.kinematics.vel)
        };

        assert_eq!(pos, arrival);
        assert_eq!(vel, incoming, "an edge exit keeps its run");
        assert_eq!(sample.prev, arrival);
        assert_eq!(sample.curr, arrival);
        assert_eq!(
            sample.vel, incoming,
            "the collapsed sweep must carry the velocity the body actually has, \
             not the zero the reset transited through",
        );
    }

    /// The other half of the policy: a RESET arrival is at rest everywhere.
    #[test]
    fn a_reset_arrival_is_at_rest_in_every_motion_fact() {
        let arrival = Vec2::new(200.0, 48.0);
        let mut scratch =
            BodyClusterScratch::new_with_abilities(Vec2::new(10.0, 10.0), AbilitySet::default());
        scratch.kinematics.vel = Vec2::new(-500.0, 220.0);
        let mut model = MotionModel::axis_swept(crate::AxisSweptParams::default());
        let mut sample = SweepSample {
            prev: Vec2::new(10.0, 10.0),
            curr: Vec2::new(10.0, 10.0),
            vel: Vec2::new(-500.0, 220.0),
            half: Vec2::splat(8.0),
        };
        let (pos, vel) = {
            let mut clusters = scratch.as_mut();
            clusters.sweep = Some(&mut sample);
            arrive_body_in_room(
                &mut model,
                &mut clusters,
                arrival,
                1,
                ArrivalMomentum::Reset,
            );
            (clusters.kinematics.pos, clusters.kinematics.vel)
        };

        assert_eq!(pos, arrival);
        assert_eq!(vel, Vec2::ZERO);
        assert_eq!(sample.vel, Vec2::ZERO);
        assert_eq!(sample.curr, arrival);
    }

    #[test]
    fn transit_reconciles_contacts_attachment_and_the_motion_record() {
        let mut scratch =
            BodyClusterScratch::new_with_abilities(Vec2::new(100.0, 100.0), AbilitySet::default());
        scratch.ground.on_ground = true;
        scratch.wall.on_wall = true;
        scratch.kinematics.vel = Vec2::new(300.0, -50.0);
        // A riding momentum body teleports: it must arrive Airborne with its
        // ride identity gone, but its velocity kept (a blink keeps momentum).
        let mut model = MotionModel::surface_momentum(MomentumParams::default());
        if let MotionModel::SurfaceMomentum(momentum) = &mut model {
            momentum.state = SurfaceMotion::Riding {
                on: SurfaceRef::Chain(2),
                s: 14.0,
                v_t: 600.0,
            };
        }
        let mut clusters = scratch.as_mut();
        transit_body(
            &mut model,
            &mut clusters,
            Vec2::new(900.0, 40.0),
            TransitVelocity::Keep,
        );
        assert_eq!(clusters.kinematics.pos, Vec2::new(900.0, 40.0));
        assert_eq!(clusters.kinematics.vel, Vec2::new(300.0, -50.0));
        assert!(!clusters.ground.on_ground, "support was a departure fact");
        assert!(
            !clusters.ground.contact_initialized,
            "the destination must establish a fresh contact baseline"
        );
        assert!(!clusters.wall.on_wall);
        let MotionModel::SurfaceMomentum(momentum) = &model else {
            panic!("transit never changes the policy");
        };
        assert_eq!(momentum.state, SurfaceMotion::Airborne);

        // An axis body: wall engagement + ledge anchor are place facts and
        // clear; maneuver TIME facts (coyote window) survive the transit.
        let mut axis_model = MotionModel::axis_swept(crate::AxisSweptParams::default());
        if let MotionModel::AxisSwept(axis) = &mut axis_model {
            axis.state.wall_clinging = true;
            axis.state.ledge_grab = Some(crate::LedgeGrabState::hanging(crate::LedgeContact {
                wall_normal_x: 1.0,
                anchor: Vec2::new(100.0, 100.0),
                climb_target: Vec2::new(90.0, 80.0),
            }));
            axis.state.coyote_timer = 0.07;
        }
        transit_body(
            &mut axis_model,
            &mut clusters,
            Vec2::new(500.0, 40.0),
            TransitVelocity::Keep,
        );
        let MotionModel::AxisSwept(axis) = &axis_model else {
            panic!("transit never changes the policy");
        };
        assert!(!axis.state.wall_clinging && !axis.state.wall_climbing);
        assert!(axis.state.ledge_grab.is_none(), "the anchor was positional");
        assert_eq!(axis.state.coyote_timer, 0.07, "time facts are kept");

        // The crawler variant arrives detached.
        let mut crawler = MotionModel::AdhesiveCrawler(crate::movement::AdhesiveCrawlerMotion {
            params: CrawlerParams::default(),
            state: CrawlerState::attached(Vec2::new(-1.0, 0.0)),
        });
        transit_body(
            &mut crawler,
            &mut clusters,
            Vec2::new(20.0, 20.0),
            TransitVelocity::Zero,
        );
        let MotionModel::AdhesiveCrawler(motion) = &crawler else {
            unreachable!();
        };
        assert!(!motion.state.is_attached());
        assert_eq!(clusters.kinematics.vel, Vec2::ZERO);
    }

    #[test]
    fn carry_and_constraint_leave_contact_facts_alone() {
        let mut scratch =
            BodyClusterScratch::new_with_abilities(Vec2::new(50.0, 50.0), AbilitySet::default());
        scratch.ground.on_ground = true;
        let clusters = scratch.as_mut();
        carry_body(clusters.kinematics, Vec2::new(3.0, 0.0));
        assert_eq!(clusters.kinematics.pos, Vec2::new(53.0, 50.0));
        assert!(clusters.ground.on_ground, "a carried body stays supported");
        constrain_body_pose(
            clusters.kinematics,
            Vec2::new(80.0, 40.0),
            Vec2::new(0.0, 5.0),
        );
        assert_eq!(clusters.kinematics.pos, Vec2::new(80.0, 40.0));
        assert_eq!(clusters.kinematics.vel, Vec2::new(0.0, 5.0));
        assert!(clusters.ground.on_ground);
    }
}
