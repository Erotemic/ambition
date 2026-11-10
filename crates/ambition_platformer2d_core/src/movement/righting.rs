//! Physical righting and admission of the recorded body path.

use crate::{BodyKinematics, SourceTurn, SweepSample, Vec2, World};
use crate::frame::{NumericFault, thread::{MotionAdmission, Refusal, RefusalReason}};

/// Keep an accepted translation when a solid blocks the proposed turn.
/// Numerical and representation failures refuse the complete candidate.
#[allow(clippy::too_many_arguments)]
pub fn admit_body_righting(
    body: &mut BodyKinematics,
    record: &mut SweepSample,
    target_down: Vec2,
    max_step: f64,
    duration: f64,
    world: &World,
    admission: Option<MotionAdmission<'_>>,
    carried_vectors: &[Vec2],
) -> Result<bool, Refusal> {
    let numeric = |fault| Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(fault) };
    if !body.canonical_size.is_finite() { return Err(numeric(NumericFault::NonFinite)); }
    if body.canonical_size.min_element() <= 0.0 { return Err(numeric(NumericFault::InvalidScale)); }
    let turn = body.material.righting_turn(target_down, max_step).map_err(numeric)?;
    let domain = body.canonical_box().map_err(numeric)?;
    if turn != 0.0 {
        let proposal = SweepSample { turn: Some(SourceTurn { radians: turn, duration }), ..*record };
        let cells = proposal.material_cast(&domain)?.cells(admission)?;
        cells.admit_payloads(body.vel, carried_vectors)?;
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
        let blocked = contact.is_some();
        if !blocked {
            let (_, until) = proposal.travel.line(proposal.prev, proposal.curr).map_err(numeric)?;
            body.material = record.material.turned(turn * until).map_err(numeric)?;
            *record = proposal;
            record.half = body.half_extent();
            return Ok(true);
        }
    }
    let cells = record.material_cast(&domain)?.cells(admission)?;
    cells.admit_payloads(body.vel, carried_vectors)?;
    Ok(false)
}
