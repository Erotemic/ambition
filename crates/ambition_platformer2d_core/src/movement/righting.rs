//! Physical righting and admission of the recorded body path.

use crate::{BodyKinematics, SourceTurn, SweepSample, Vec2, World};
use crate::frame::{NumericFault, thread::{MotionAdmission, Refusal, RefusalReason}};

/// Keep an accepted translation when a solid blocks the proposed turn.
/// Numerical and representation failures refuse the complete candidate.
///
/// `pivot_step`: the body stands on what it rights against. A body lying on
/// a floor cannot turn about its centre, whose corners sweep below the
/// corner it rests on, so when that turn is blocked it stands up about a
/// corner instead, at most this far a tick, as a box tipped on its side is
/// stood up on its edge. Without it, a body that landed half turned lay
/// there for good.
#[allow(clippy::too_many_arguments)]
pub fn admit_body_righting(
    body: &mut BodyKinematics,
    record: &mut SweepSample,
    target_down: Vec2,
    max_step: f64,
    pivot_step: Option<f64>,
    duration: f64,
    world: &World,
    admission: Option<MotionAdmission<'_>>,
    carried_vectors: &[Vec2],
    // The body's key for the lines it carries (`MaterialCast::carried_by`).
    carrier: Option<u64>,
) -> Result<bool, Refusal> {
    let numeric = |fault| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(fault) };
    if !body.canonical_size.is_finite() { return Err(numeric(NumericFault::NonFinite)); }
    if body.canonical_size.min_element() <= 0.0 { return Err(numeric(NumericFault::InvalidScale)); }
    let turn = body.material.righting_turn(target_down, max_step).map_err(numeric)?;
    let domain = body.canonical_box().map_err(numeric)?;
    let vel = body.vel;
    // Is `proposal`'s whole path clear of what the body stands or turns against?
    let clear = |proposal: &SweepSample| -> Result<bool, Refusal> {
        let cells = proposal.material_cast(&domain)?.carried_by(carrier).cells(admission)?;
        cells.admit_payloads(vel, carried_vectors)?;
        let contact = world.first_material_contact(&cells, |collider, _, hit| {
            Ok(match collider {
                crate::world::MaterialCollider::Block(block) => {
                    crate::collision_semantics::is_full_collision_surface(block.kind)
                        || block.kind == crate::BlockKind::OneWay && hit.normal.dot(-target_down) > 0.5
                }
                crate::world::MaterialCollider::Chain { chain, segment, .. } => {
                    if chain.kind == crate::world::SurfaceKind::Solid && chain.closed { true }
                    else {
                        let normal = chain.normal_at_time(segment, hit.world_time).map_err(numeric)?;
                        hit.map.direction(hit.normal).dot(normal) > 0.5
                    }
                }
            })
        })?;
        Ok(contact.is_none())
    };
    if turn != 0.0 {
        let proposal = SweepSample { turn: Some(SourceTurn { radians: turn, duration }), ..*record };
        let mut admitted = clear(&proposal)?.then_some((proposal, Vec2::ZERO));
        match pivot_step.filter(|step| *step > 0.0) {
            Some(step) if admitted.is_none() => {
                let step = turn.clamp(-step, step);
                let lift = pivot_lift(body, record.material, step, target_down).map_err(numeric)?;
                let proposal = SweepSample { turn: Some(SourceTurn { radians: step, duration }), curr: record.curr + lift, ..*record };
                admitted = clear(&proposal)?.then_some((proposal, lift));
            }
            _ => {}
        }
        if let Some((proposal, lift)) = admitted {
            let (_, until) = proposal.travel.line(proposal.prev, proposal.curr).map_err(numeric)?;
            let turn = proposal.turn.map_or(0.0, |turn| turn.radians);
            body.material = record.material.turned(turn * until).map_err(numeric)?;
            body.pos += lift;
            *record = proposal;
            record.half = body.half_extent();
            return Ok(true);
        }
    }
    let cells = record.material_cast(&domain)?.carried_by(carrier).cells(admission)?;
    cells.admit_payloads(body.vel, carried_vectors)?;
    Ok(false)
}

/// How far the centre moves when `body` turns by `step` about the corner of
/// its box that keeps the turned box highest along `down`: the corner it
/// stands up on, which stays where it is.
fn pivot_lift(body: &BodyKinematics, from: crate::frame::BodyMaterial, step: f64, down: Vec2) -> Result<Vec2, NumericFault> {
    let half = body.canonical_size * 0.5;
    let corners = [Vec2::new(-half.x, -half.y), Vec2::new(half.x, -half.y), Vec2::new(half.x, half.y), Vec2::new(-half.x, half.y)];
    let (before, after) = (from.at(Vec2::ZERO)?, from.turned(step)?.at(Vec2::ZERO)?);
    let mut best: Option<(f32, Vec2)> = None;
    for corner in corners {
        let lift = before.vector(corner)? - after.vector(corner)?;
        let mut lowest = f32::NEG_INFINITY;
        for other in corners { lowest = lowest.max((lift + after.vector(other)?).dot(down)); }
        if best.is_none_or(|(old, _)| lowest < old) { best = Some((lowest, lift)); }
    }
    Ok(best.map_or(Vec2::ZERO, |(_, lift)| lift))
}
