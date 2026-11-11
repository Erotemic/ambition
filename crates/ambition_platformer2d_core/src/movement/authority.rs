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
    // ⛔⛤ `ResetFacing::Keep`, AND THE ARRIVAL IS WHERE THAT ANSWER LIVES.
    // `reset_body_clusters` used to spin every reset body to `facing = 1.0`; an
    // ARRIVAL is a body that TRAVELLED here, so its heading is a fact it brought
    // with it. With `ArrivalMomentum::Preserve` the old behaviour put the two
    // motion facts in disagreement in the same way the sweep sample below was:
    // a body that walked LEFT through a door came out still moving left and
    // looking right. This wrapper is a named verb, so it answers rather than
    // asking its caller — which is the difference between it and the primitive.
    crate::reset_body_clusters(model, clusters, spawn, crate::ResetFacing::Keep, air_jumps_default);
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
///   segment. The sample keeps the turn of the body: a transit moves a body
///   and does not turn it.
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
        // A transit moves the body and does not turn it. The box at the
        // arrival is the body's PRESENT size (a reset changes the size before
        // it transits) turned to the DOWN of its last step. The next step
        // turns the body in the frame it arrives in.
        let down = sweep.down;
        *sweep = SweepSample { turn: None, travel: Default::default(), time: [0.0, 1.0],
            material: clusters.kinematics.material,
            prev: pos,
            curr: pos,
            vel: clusters.kinematics.vel,
            half: clusters.kinematics.half_extent(),
            down,
        };
    }
}

/// External kinematic CARRY: move the body with its parent frame by `delta`
/// (a moving platform carrying a ledge-grabber, an attractor's pull, the
/// portal-close straddle eviction). Contacts, attachment, and velocity are
/// deliberately untouched — a carried body is still supported/held; the next
/// kernel step re-resolves contact from the carried pose.
///
/// The body really went there, so the travelled path goes there too: the
/// sample's end moves with it (see [`SweepSample::ending_at`]).
pub fn carry_body(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    sweep: Option<&mut SweepSample>,
    delta: Vec2,
) {
    kinematics.pos += delta;
    if let Some(sweep) = sweep {
        sweep.curr += delta;
    }
}

/// External kinematic PIN: hold the body at an absolute pose with an imposed
/// velocity (a mount's saddle, a scripted end-of-level slide). The constraint
/// owner is the body's motion authority while engaged; like [`carry_body`] it
/// does not fabricate or clear contact facts.
///
/// A valid recorded line ends at the pin. A body with no travel record is
/// placed at tick end. Admission validates the complete candidate before commit.
/// Refusal retains both the body and its prior accepted record.
pub fn constrain_body_pose(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    sweep: Option<&mut SweepSample>,
    desired: crate::body_clusters::BodyKinematics,
    admission: Option<crate::frame::thread::MotionAdmission<'_>>,
    carried_vectors: &[Vec2],
) -> Result<(), crate::frame::thread::Refusal> {
    let prepared = prepare_body_pose(kinematics, sweep.as_deref(), desired, admission, carried_vectors)?;
    prepared.commit(kinematics, sweep);
    Ok(())
}

/// A validated external pose. Prepare all members of a group before commit.
#[must_use]
pub struct PreparedBodyPose {
    body: crate::body_clusters::BodyKinematics,
    sweep: SweepSample,
}

impl PreparedBodyPose {
    /// Commit while the input bodies and admission context remain unchanged.
    pub fn commit(self, body: &mut crate::body_clusters::BodyKinematics, sweep: Option<&mut SweepSample>) {
        *body = self.body;
        if let Some(sweep) = sweep { *sweep = self.sweep; }
    }
}

/// An admitted new body and its carried vector payloads.
#[must_use]
pub struct PreparedBodyMaterialization<const N: usize> {
    pose: PreparedBodyPose,
    carried: [Vec2; N],
}

impl<const N: usize> PreparedBodyMaterialization<N> {
    /// Commit the body and return its admitted carried payloads.
    pub fn commit(self, body: &mut crate::BodyKinematics, sweep: Option<&mut SweepSample>) -> [Vec2; N] {
        self.pose.commit(body, sweep);
        self.carried
    }
}

/// Admit a new body from a reference root in the source chart. Its native
/// partition selects the chart of its centre before payload admission.
pub fn prepare_body_materialization<const N: usize>(
    mut desired: crate::BodyKinematics,
    reference: Vec2,
    admission: Option<crate::frame::thread::MotionAdmission<'_>>,
    mut carried: [Vec2; N],
) -> Result<PreparedBodyMaterialization<N>, crate::frame::thread::Refusal> {
    use crate::frame::thread::{MaterialPrimitive, Refusal, RefusalReason, partition};
    let numeric = |reason| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(reason) };
    if !reference.is_finite() { return Err(numeric(crate::frame::NumericFault::NonFinite)); }
    if let Some(admission) = admission {
        let domain = desired.canonical_box().map_err(numeric)?;
        let parts = partition(MaterialPrimitive { domain: &domain, to_world: desired.material_map().map_err(numeric)? },
            reference, admission.links, admission.first, admission.budget, admission.convention)?;
        let mut root = None;
        for piece in &parts {
            if piece.source_bounds_contain(Vec2::ZERO).map_err(numeric)?
                && root.as_ref().is_none_or(|old: &&crate::frame::thread::VolumePiece|
                    (piece.path.len(), &piece.path) < (old.path.len(), &old.path)) { root = Some(piece); }
        }
        let piece = root.ok_or_else(|| numeric(crate::frame::NumericFault::InvalidFrame))?;
        let map = piece.path_transform().map_err(numeric)?;
        desired.vel = admission.carry_velocity(&piece.path, desired.pos, desired.vel, 1.0)?;
        desired.pos = map.point(desired.pos).map_err(numeric)?;
        desired.material = desired.material.carried(map).map_err(numeric)?;
        for vector in &mut carried { *vector = map.vector(*vector).map_err(numeric)?; }
    }
    let sample = SweepSample::at_tick_end(desired, Vec2::ZERO);
    let admission = admission.map(|admission| crate::frame::thread::MotionAdmission { first: None, ..admission });
    let pose = prepare_body_pose(&desired, Some(&sample), desired, admission, &carried)?;
    Ok(PreparedBodyMaterialization { pose, carried })
}

/// Validate a complete external pose without changing its body or history.
pub fn prepare_body_pose(
    kinematics: &crate::body_clusters::BodyKinematics,
    sweep: Option<&SweepSample>,
    desired: crate::body_clusters::BodyKinematics,
    admission: Option<crate::frame::thread::MotionAdmission<'_>>,
    carried_vectors: &[Vec2],
) -> Result<PreparedBodyPose, crate::frame::thread::Refusal> {
    validate_body_pose(kinematics, sweep, &desired)?;
    use crate::frame::thread::{Refusal, RefusalReason};
    let numeric = |reason| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(reason) };
    let prior = sweep.as_deref().and_then(|sample| sample.ending_at(kinematics.pos)).copied();
    let mut candidate = match prior {
        Some(sample) if sample.time[0] != sample.time[1] => SweepSample {
            curr: desired.pos, vel: desired.vel, travel: crate::LinearTravel::Complete,
            half: desired.half_extent(), ..sample
        },
        Some(sample) => SweepSample { time: sample.time, ..SweepSample::at_tick_end(desired, sample.down) },
        None => SweepSample::at_tick_end(desired, Vec2::ZERO),
    };
    let domain = desired.canonical_box().map_err(numeric)?;
    let cast = candidate.material_cast(&domain)?;
    match admission {
        Some(admission) => admission.admit(cast, desired.vel, carried_vectors)?,
        None => cast.cells(None)?.admit_payloads(desired.vel, carried_vectors)?,
    }
    candidate.half = desired.half_extent();
    Ok(PreparedBodyPose { body: desired, sweep: candidate })
}

fn validate_body_pose(
    kinematics: &crate::BodyKinematics,
    sweep: Option<&SweepSample>,
    desired: &crate::BodyKinematics,
) -> Result<(), crate::frame::thread::Refusal> {
    use crate::frame::{thread::{Refusal, RefusalReason}, NumericFault};
    let numeric = |reason| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(reason) };
    if desired.material != kinematics.material { return Err(numeric(NumericFault::InvalidFrame)); }
    if !desired.pos.is_finite() || !desired.vel.is_finite() || !desired.facing.is_finite() || !desired.canonical_size.is_finite() {
        return Err(numeric(NumericFault::NonFinite));
    }
    if desired.canonical_size.min_element() <= 0.0 { return Err(numeric(NumericFault::InvalidScale)); }
    if let Some(sample) = sweep.as_deref() {
        sample.travel.line(sample.prev, sample.curr).map_err(numeric)?;
        if !sample.down.is_finite() || sample.time.iter().any(|time| !time.is_finite()) {
            return Err(numeric(NumericFault::NonFinite));
        }
        if sample.time[0] < 0.0 || sample.time[1] > 1.0 || sample.time[0] > sample.time[1] {
            return Err(numeric(NumericFault::InvalidFrame));
        }
    }
    Ok(())
}

/// Sweep an external translation against full solids before one pose commit.
pub fn prepare_body_translation(
    world: &crate::World,
    kinematics: &crate::body_clusters::BodyKinematics,
    sweep: Option<&SweepSample>,
    desired: crate::body_clusters::BodyKinematics,
    admission: Option<crate::frame::thread::MotionAdmission<'_>>,
    carried_vectors: &[Vec2],
) -> Result<PreparedBodyPose, crate::frame::thread::Refusal> {
    use crate::frame::{material_cast::MaterialCast, thread::{MaterialPrimitive, Refusal, RefusalReason}};
    let numeric = |reason| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(reason) };
    if desired.canonical_size != kinematics.canonical_size {
        return Err(numeric(crate::frame::NumericFault::InvalidFrame));
    }
    validate_body_pose(kinematics, sweep, &desired)?;
    let mut accepted = desired;
    let delta = desired.pos - kinematics.pos;
    let mut sample = SweepSample { curr: desired.pos, vel: desired.vel,
        ..SweepSample::at_rest(*kinematics, sweep.map_or(Vec2::ZERO, |sample| sample.down)) };
    let domain = desired.canonical_box().map_err(numeric)?;
    let cast = MaterialCast::new(MaterialPrimitive {
        domain: &domain, to_world: kinematics.material_map().map_err(numeric)?,
    }, kinematics.pos, delta, 0.0..1.0);
    let accepts = |collider, _: &crate::frame::material_cast::MaterialCell, _: &crate::frame::material_cast::MaterialCastHit| {
        Ok(match collider {
            crate::world::MaterialCollider::Block(block) => crate::collision_semantics::is_full_collision_surface(block.kind),
            crate::world::MaterialCollider::Chain { chain, .. } => chain.kind == crate::world::SurfaceKind::Solid,
        })
    };
    let contact = match world.first_material_sweep(cast.clone(), admission, accepts) {
        Ok(contact) => contact,
        Err(refusal) => {
            // A later refusal does not reject an earlier solid contact.
            cast.clone().prefix(0.0)?.cells(admission)?;
            let (mut valid, mut refused) = (0.0, 1.0);
            let mut contact = None;
            for _ in 0..128 {
                let middle = valid + (refused - valid) * 0.5;
                if middle == valid || middle == refused { break; }
                match world.first_material_sweep(cast.clone().prefix(middle)?, admission, accepts) {
                    Ok(Some(hit)) => { contact = Some(hit); break; }
                    Ok(None) => valid = middle,
                    Err(_) => refused = middle,
                }
            }
            match contact { Some(hit) => Some(hit), None => return Err(refusal) }
        }
    };
    if let Some(hit) = contact {
        accepted.pos = (kinematics.pos.as_dvec2() + delta.as_dvec2() * hit.contact.toi).as_vec2();
        sample.curr = accepted.pos;
        sample.travel = crate::LinearTravel::Prefix { delta, until: hit.contact.toi };
    }
    let cast = sample.material_cast(&domain)?;
    match admission {
        Some(admission) => admission.admit(cast, desired.vel, carried_vectors)?,
        None => cast.cells(None)?.admit_payloads(desired.vel, carried_vectors)?,
    }
    Ok(PreparedBodyPose { body: accepted, sweep: sample })
}

#[cfg(test)]
mod translation_tests {
    use super::*;
    use crate::frame::{MapConvention, PortalAperture, PortalFrame, thread::{MotionAdmission, PieceBudget, PortalLink}};

    fn aperture(x: f32, y: f32, normal: Vec2, half_length: f32) -> PortalAperture {
        PortalAperture { frame: PortalFrame::fixed(Vec2::new(x, y), normal), half_length }
    }

    #[test]
    fn materialization_uses_the_centre_chart_and_carries_velocity_and_payloads() {
        let links = [PortalLink::new(aperture(100.0, 300.0, -Vec2::Y, 40.0),
            aperture(600.0, 100.0, Vec2::Y, 20.0), 0)];
        let admission = MotionAdmission { links: &links, first: None,
            budget: PieceBudget::COMPOSED, convention: MapConvention::Reflection };
        let desired = crate::BodyKinematics { pos: Vec2::new(100.0, 310.0), vel: Vec2::new(20.0, 200.0),
            canonical_size: Vec2::splat(4.0), ..Default::default() };
        let vector = Vec2::new(4.0, -12.0);
        let prepared = prepare_body_materialization(desired, Vec2::new(100.0, 270.0), Some(admission), [vector]).unwrap();
        let mut body = desired;
        let [carried] = prepared.commit(&mut body, None);
        assert_eq!(body.pos, Vec2::new(600.0, 105.0));
        assert_eq!(body.material.scale(), 0.5);
        assert!(body.material.reflected());
        assert_eq!(body.canonical_size, desired.canonical_size);
        assert_eq!(body.vel.length(), desired.vel.length() * 0.5);
        assert_eq!(carried, links[0].map(MapConvention::Reflection).unwrap().vector(vector).unwrap());
        assert!(prepare_body_materialization(desired, Vec2::new(100.0, 270.0), Some(admission),
            [Vec2::splat(f32::NAN)]).is_err());
    }

    #[test]
    fn translation_admits_the_wall_stop_before_a_refused_portal_path() {
        let links = [PortalLink::new(aperture(100.0, 300.0, -Vec2::Y, 40.0),
            aperture(600.0, 100.0, Vec2::Y, 40.0), 0)];
        let admission = Some(MotionAdmission { links: &links, first: Some(0),
            budget: PieceBudget { max_hops: 0, ..PieceBudget::COMPOSED },
            convention: MapConvention::Reflection });
        let body = crate::BodyKinematics { pos: Vec2::new(100.0, 270.0),
            canonical_size: Vec2::splat(16.0), ..Default::default() };
        let sample = SweepSample::at_tick_end(body, Vec2::Y);
        let desired = crate::BodyKinematics { pos: body.pos + Vec2::Y * 200.0, ..body };
        let mut world = crate::World::new("prefix", Vec2::splat(2000.0), Vec2::ZERO, vec![]);
        assert!(prepare_body_translation(&world, &body, Some(&sample), desired, admission, &[]).is_err());
        world.blocks.push(crate::Block::solid("early stop", Vec2::new(0.0, 286.0), Vec2::new(400.0, 4.0)));
        let prepared = prepare_body_translation(&world, &body, Some(&sample), desired, admission, &[]).unwrap();
        assert_eq!(prepared.body.pos, Vec2::new(100.0, 278.0));
        assert!(matches!(prepared.sweep.travel, crate::LinearTravel::Prefix { until, .. } if until < 0.1));
        assert!(prepare_body_translation(&world, &body, Some(&sample), desired, admission,
            &[Vec2::splat(f32::NAN)]).is_err());
    }

    #[test]
    fn a_prepared_translation_stops_on_near_scaled_and_chained_material() {
        let entry = |half| aperture(100.0, 300.0, -Vec2::Y, half);
        let exit = |x, y, half| aperture(x, y, Vec2::Y, half);
        for (pairs, shelf, expected) in [
            (vec![], None, 292.0),
            (vec![(entry(40.0), exit(600.0, 100.0, 20.0))], Some((600.0, 140.0)), 372.0),
            (vec![(entry(20.0), exit(900.0, 100.0, 40.0))], Some((900.0, 180.0)), 332.0),
            (vec![(entry(40.0), exit(600.0, 100.0, 20.0)),
                (aperture(600.0, 120.0, -Vec2::Y, 20.0), exit(1000.0, 100.0, 20.0)),
                (aperture(1000.0, 120.0, -Vec2::Y, 20.0), exit(1400.0, 100.0, 40.0))],
                Some((1400.0, 140.0)), 412.0),
        ] {
            let links: Vec<_> = pairs.iter().enumerate().flat_map(|(index, &(a, b))| [
                PortalLink::new(a, b, index as u32 * 2), PortalLink::new(b, a, index as u32 * 2 + 1),
            ]).collect();
            let mut blocks = vec![crate::Block::solid("source floor", Vec2::new(0.0, 300.0), Vec2::new(400.0, 40.0))];
            if let Some((x, y)) = shelf {
                blocks.push(crate::Block::solid("far stop", Vec2::new(x - 60.0, y), Vec2::new(120.0, 10.0)));
            }
            let world = crate::World::new("translation", Vec2::splat(2000.0), Vec2::ZERO, blocks);
            let mut body = crate::BodyKinematics { pos: Vec2::new(100.0, 270.0),
                canonical_size: Vec2::splat(16.0), ..Default::default() };
            let mut sample = SweepSample::at_tick_end(body, Vec2::Y);
            let desired = crate::BodyKinematics { pos: body.pos + Vec2::Y * 200.0, ..body };
            let admission = (!links.is_empty()).then_some(MotionAdmission {
                links: &links, first: Some(0), budget: PieceBudget::COMPOSED, convention: MapConvention::Reflection,
            });
            prepare_body_translation(&world, &body, Some(&sample), desired, admission, &[])
                .unwrap().commit(&mut body, Some(&mut sample));
            assert_eq!(body.pos, Vec2::new(100.0, expected));
            assert_eq!(sample.curr, body.pos);
            assert_eq!(sample.time, [0.0, 1.0]);
            assert!(matches!(sample.travel, crate::LinearTravel::Prefix { until, .. } if until < 1.0));
        }
    }
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

/// Sweep a displacement while ordinary body integration is held.
/// SDI during hitlag and passage scenes use this path. Full solid geometry
/// stops the shift. Velocity and movement state stay unchanged.
pub fn shift_frozen_body(
    world: &crate::World,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    shift: Vec2,
) {
    if shift == Vec2::ZERO {
        return;
    }
    let body = kinematics.aabb();
    // Only FULL solids stop a frozen nudge. One-way and bonk-only blocks are
    // directional by definition — a body standing on a platform may still SDI
    // sideways along it — and a blink wall is solid to a body that is not
    // blinking.
    let blocked = world.first_body_sweep(body, shift, |block| {
        crate::collision_semantics::is_full_collision_surface(block.kind)
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
            canonical_size: Vec2::new(20.0, 40.0),
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

        shift_frozen_body(&world, &mut kin, Vec2::new(8.0, 0.0));

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

        shift_frozen_body(&world, &mut kin, Vec2::new(40.0, 0.0));

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
            shift_frozen_body(&world, &mut kin, Vec2::new(3.0, 0.0));
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
        kin.material = crate::frame::BodyMaterial::IDENTITY.turned(std::f64::consts::FRAC_PI_2).unwrap();

        shift_frozen_body(&world, &mut kin, Vec2::new(30.0, 0.0));

        let half_width = kin.aabb().max.x - kin.pos.x;
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
        shift_frozen_body(&open, &mut kin, Vec2::new(-2.0, 0.5));
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
        let mut sample = SweepSample { turn: None, material: Default::default(), travel: Default::default(), time: [0.0, 1.0],
            prev: Vec2::new(10.0, 10.0),
            curr: Vec2::new(10.0, 10.0),
            vel: incoming,
            half: Vec2::splat(8.0),
            down: Vec2::new(0.0, 1.0),
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

    /// AN ARRIVAL KEEPS THE HEADING IT TRAVELLED WITH.
    ///
    /// ⛔⛤ **`reset_body_clusters` HARDCODED `facing = 1.0` AND NOTHING AFTER
    /// THE ARRIVAL PUT IT BACK — 2026-09-21.** A body walking LEFT through a
    /// door arrived still moving left (`ArrivalMomentum::Preserve` restores the
    /// velocity) and looking right: the same two-motion-facts-disagreeing shape
    /// as the sweep-sample defect the arm above exists for, one field over.
    ///
    /// ⚠ Both momentum modes, because the facing is not a momentum fact and a
    /// one-mode arm would let the other regress. A `Reset` arrival stops, and a
    /// body that stops does not turn around.
    #[test]
    fn an_arrival_does_not_turn_the_body_around() {
        for momentum in [ArrivalMomentum::Preserve, ArrivalMomentum::Reset] {
            let mut scratch = BodyClusterScratch::new_with_abilities(
                Vec2::new(10.0, 10.0),
                AbilitySet::default(),
            );
            scratch.kinematics.vel = Vec2::new(-400.0, 0.0);
            scratch.kinematics.facing = -1.0;
            let mut model = MotionModel::axis_swept(crate::AxisSweptParams::default());
            let facing = {
                let mut clusters = scratch.as_mut();
                arrive_body_in_room(
                    &mut model,
                    &mut clusters,
                    Vec2::new(200.0, 48.0),
                    1,
                    momentum,
                );
                clusters.kinematics.facing
            };
            assert_eq!(
                facing, -1.0,
                "a body that walked in from the right arrived facing the door it \
                 came through ({momentum:?})",
            );
        }
    }

    /// The other half of the policy: a RESET arrival is at rest everywhere.
    #[test]
    fn a_reset_arrival_is_at_rest_in_every_motion_fact() {
        let arrival = Vec2::new(200.0, 48.0);
        let mut scratch =
            BodyClusterScratch::new_with_abilities(Vec2::new(10.0, 10.0), AbilitySet::default());
        scratch.kinematics.vel = Vec2::new(-500.0, 220.0);
        let mut model = MotionModel::axis_swept(crate::AxisSweptParams::default());
        let mut sample = SweepSample { turn: None, material: Default::default(), travel: Default::default(), time: [0.0, 1.0],
            prev: Vec2::new(10.0, 10.0),
            curr: Vec2::new(10.0, 10.0),
            vel: Vec2::new(-500.0, 220.0),
            half: Vec2::splat(8.0),
            down: Vec2::new(0.0, 1.0),
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

    /// Contacts are left alone, and the travelled path follows the body: after
    /// either authority the sample still ends where the body is, from where the
    /// step began, so a path reader sees one segment rather than none.
    #[test]
    fn carry_and_constraint_leave_contact_facts_alone_and_extend_the_path() {
        let mut scratch =
            BodyClusterScratch::new_with_abilities(Vec2::new(50.0, 50.0), AbilitySet::default());
        scratch.ground.on_ground = true;
        let mut sample = SweepSample { turn: None, material: Default::default(), travel: Default::default(), time: [0.0, 1.0],
            prev: Vec2::new(40.0, 50.0),
            curr: Vec2::new(50.0, 50.0),
            vel: Vec2::ZERO,
            half: Vec2::splat(8.0),
            down: Vec2::new(0.0, 1.0),
        };
        let clusters = scratch.as_mut();
        carry_body(clusters.kinematics, Some(&mut sample), Vec2::new(3.0, 0.0));
        assert_eq!(clusters.kinematics.pos, Vec2::new(53.0, 50.0));
        assert!(clusters.ground.on_ground, "a carried body stays supported");
        assert!(sample.ending_at(clusters.kinematics.pos).is_some(), "the carry extends the path");
        let desired = crate::BodyKinematics { pos: Vec2::new(80.0, 40.0), vel: Vec2::new(0.0, 5.0), ..*clusters.kinematics };
        constrain_body_pose(
            clusters.kinematics,
            Some(&mut sample),
            desired,
            None,
            &[],
        ).unwrap();
        assert_eq!(clusters.kinematics.pos, Vec2::new(80.0, 40.0));
        assert_eq!(clusters.kinematics.vel, Vec2::new(0.0, 5.0));
        assert!(clusters.ground.on_ground);
        assert!(sample.ending_at(clusters.kinematics.pos).is_some(), "the pin ends the path");
        assert_eq!(sample.prev, Vec2::new(40.0, 50.0), "from where the step began");
    }
}

#[cfg(test)]
mod pose_tests;
