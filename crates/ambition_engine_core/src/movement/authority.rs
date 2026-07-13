//! The body-motion AUTHORITIES besides the kernel (ADR 0024).
//!
//! Every production write to authoritative body pose/velocity belongs to
//! exactly one named authority:
//!
//! 1. **Continuous integration** — [`super::step_motion`], the movement kernel.
//! 2. **Discrete transit** — [`transit_body`]: blink and dive arrivals, recall,
//!    portal exits, respawns, room placement, scripted teleports. A transit is
//!    NOT a fake physics tick; it deliberately reconciles contact, attachment,
//!    and model-private state (semantics below).
//! 3. **External kinematic constraint** — [`carry_body`] (parent-frame carry:
//!    moving-platform ledge carry, attractor pull, straddle eviction) and
//!    [`constrain_body_pose`] (absolute pin: a mount's saddle, a scripted
//!    flagpole slide).
//! 4. **Impulses** — typed velocity operations that consume the body's resolved
//!    frame: [`super::set_jump_velocity`],
//!    [`AccelerationFrame::launch`](crate::AccelerationFrame::launch), and
//!    frame-rotated `vel +=` writes at combat/ability seams.
//!
//! Anything else writing `BodyKinematics.pos` in production is an authority
//! leak (guarded by workspace policy).

use super::model::MotionModel;
use crate::body_clusters::BodyClustersMut;
use crate::{SweepSample, Vec2};

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
/// - **Contacts are invalidated**: support (`on_ground`), wall contact, and any
///   in-flight wall cling/climb are cleared — they described surfaces at the
///   departure point. The destination re-acquires them through the ordinary
///   same-tick contact rules of the active policy, never by nearest-surface
///   guessing here.
/// - **A ledge grab is released**: its anchor is positional.
/// - **Model-private attachment is invalidated**: a riding momentum body
///   arrives `Airborne`; an attached crawler arrives detached. Axis maneuver
///   state (coyote, buffers, dash timers) is deliberately KEPT — those are
///   time facts, not place facts.
/// - **The §3.1 motion record collapses** to a zero-length sample at the
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
    clusters.ground.on_ground = false;
    clusters.wall.on_wall = false;
    clusters.wall.wall_normal_x = 0.0;
    clusters.wall.wall_clinging = false;
    clusters.wall.wall_climbing = false;
    clusters.ledge.grab = None;
    match model {
        MotionModel::AxisSwept(_) => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::adhesive_crawler::CrawlerState;
    use crate::movement::surface_momentum::{SurfaceMotion, SurfaceRef};
    use crate::{AbilitySet, BodyClusterScratch, CrawlerParams, MomentumParams};

    #[test]
    fn transit_reconciles_contacts_attachment_and_the_motion_record() {
        let mut scratch =
            BodyClusterScratch::new_with_abilities(Vec2::new(100.0, 100.0), AbilitySet::default());
        scratch.ground.on_ground = true;
        scratch.wall.on_wall = true;
        scratch.wall.wall_clinging = true;
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
        assert!(!clusters.wall.on_wall && !clusters.wall.wall_clinging);
        let MotionModel::SurfaceMomentum(momentum) = &model else {
            panic!("transit never changes the policy");
        };
        assert_eq!(momentum.state, SurfaceMotion::Airborne);

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
