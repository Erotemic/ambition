//! Ledge trumping enforces one hanging body per edge.
//!
//! The most recent grab keeps the ledge; older holders are knocked off and lose
//! ledge-grab intangibility. This implements the contested-edge rule without the
//! outward helpless pop used by some platform fighters.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// How close two anchors must be to be the same edge, in world px. A ledge
/// anchor is written by the kernel from the same contact geometry for both
/// bodies, so this is a float-equality tolerance rather than a reach.
const SAME_EDGE_EPSILON: f32 = 1.0;

/// Keep the newest holder of each edge and knock older holders off.
///
/// Sorting by `(elapsed, SimId)` gives same-tick grabs a deterministic winner
/// independent of query/archetype order.
pub fn resolve_ledge_trumps(
    mut bodies: Query<(
        Entity,
        &SimId,
        &mut crate::features::MotionModel,
        &mut ae::BodyLedgeState,
    )>,
) {
    // (anchor, elapsed, id, entity) for every body currently hanging.
    let mut holders: Vec<(ae::Vec2, f32, SimId, Entity)> = Vec::new();
    for (entity, id, model, _) in bodies.iter() {
        let ae::MotionModel::AxisSwept(axis) = &*model else {
            continue;
        };
        // HANGING, not climbing: a body already pulling itself up has left the
        // edge as far as this rule is concerned, and trumping it would cancel a
        // getup that is no longer contesting anything.
        let Some(hang) = axis.state.ledge_grab.as_ref().filter(|l| !l.climbing) else {
            continue;
        };
        holders.push((hang.contact.anchor, hang.elapsed, id.clone(), entity));
    }
    if holders.len() < 2 {
        return;
    }
    holders.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.2.cmp(&b.2))
    });

    let mut kept: Vec<ae::Vec2> = Vec::new();
    let mut trumped: Vec<Entity> = Vec::new();
    for (anchor, _, _, entity) in &holders {
        if kept
            .iter()
            .any(|held| held.distance_squared(*anchor) <= SAME_EDGE_EPSILON * SAME_EDGE_EPSILON)
        {
            trumped.push(*entity);
        } else {
            kept.push(*anchor);
        }
    }

    for entity in trumped {
        let Ok((_, _, mut model, mut ledge)) = bodies.get_mut(entity) else {
            continue;
        };
        if ae::movement::knock_off_ledge(&mut model, &mut ledge) {
            // the window goes with the edge. It was bought with airtime this
            // body no longer has, and a falling fighter that kept it would be
            // the safest thing on the stage.
            if let ae::MotionModel::AxisSwept(axis) = &mut *model {
                axis.state.dodge_roll_timer = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests;
