//! Physical righting and admission of the recorded body path.

use crate::{BodyKinematics, LinearTravel, SourceTurn, SweepSample, Vec2, World};
use crate::collision_semantics::Contact;
use crate::frame::{NumericFault, thread::{MotionAdmission, Refusal, RefusalReason}};

/// The smallest turn worth proving: a nanoradian moves the corner of a
/// thousand-pixel body by a micropixel.
const MIN_TURN: f64 = 1e-9;

/// Keep an accepted translation when a solid blocks the proposed turn.
/// Numerical and representation failures refuse the complete candidate.
///
/// With support, try a smaller turn that moves the centre away from the surface.
#[allow(clippy::too_many_arguments)]
pub fn admit_body_righting(
    body: &mut BodyKinematics,
    record: &mut SweepSample,
    target_down: Vec2,
    max_step: f64,
    support_turn: Option<(&Contact, f64)>,
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
    // A body already aligned asks for a turn of rounding noise (1e-17 rad).
    // Proving a turn that slow clear of a curved surface exhausts the cast's
    // work budget, and the refusal stopped a rider dead on a hill.
    let turn = if turn.abs() < MIN_TURN { 0.0 } else { turn };
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
                // A turn may not take an end of the opening inside the body.
                crate::world::MaterialCollider::Rim(_) => true,
            })
        })?;
        Ok(contact.is_none())
    };
    if turn != 0.0 {
        let proposal = SweepSample { turn: Some(SourceTurn { radians: turn, duration }), ..*record };
        let mut admitted = clear(&proposal)?.then_some((proposal, Vec2::ZERO));
        match support_turn.filter(|(_, step)| *step > 0.0) {
            Some((support, step)) if admitted.is_none() && record.travel == LinearTravel::Complete => {
                let step = turn.clamp(-step, step);
                let lift = support_lift(body, record, step, support).map_err(numeric)?;
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

fn support_lift(body: &BodyKinematics, record: &SweepSample, step: f64, support: &Contact) -> Result<Vec2, NumericFault> {
    let half = body.canonical_size * 0.5;
    let map = record.material.turned(step)?.at(Vec2::ZERO)?;
    let normal = support.normal;
    let reach = map.vector(Vec2::new(half.x, 0.0))?.dot(normal).abs()
        + map.vector(Vec2::new(0.0, half.y))?.dot(normal).abs();
    // Bound the difference between each corner's arc and its chord. The
    // centre travels in a straight line. The extra clearance at the end
    // keeps the arc above the support for the full turn.
    let radius = f64::from(half.length()) * map.scale_wide();
    let slack = (radius * step * step * 0.5) as f32;
    let clearance = (body.pos - support.point).dot(normal);
    Ok(normal * (reach + slack - clearance).max(0.0))
}
