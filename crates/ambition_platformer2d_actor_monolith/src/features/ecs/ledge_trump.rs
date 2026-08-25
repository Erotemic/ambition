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
        // ⛔⛔ OPTIONAL, AND THAT IS NOT TIDINESS. Adding this as a REQUIRED
        // column silently narrowed the population the rule sees: a hanging body
        // without kinematics stopped being trumped at all, and two existing
        // tests went red with "two bodies shared one edge". Trumping is about
        // the EDGE; the pop is a bonus a body with a velocity can receive.
        Option<&mut ae::BodyKinematics>,
    )>,
    // The match's own answer to *what does losing the edge cost*. Optional
    // because a world that declares no combat rules still trumps — it simply
    // drops the loser, which is what every trump did before the knob.
    rules: Option<bevy::prelude::Res<crate::combat::rules::ResolvedCombatTuning>>,
) {
    // (anchor, elapsed, id, entity) for every body currently hanging.
    let mut holders: Vec<(ae::Vec2, f32, SimId, Entity)> = Vec::new();
    for (entity, id, model, _, _) in bodies.iter() {
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
    // ⭐⭐ THE POLICY IS THIS ONE COMPARISON. Whoever sorts FIRST keeps the
    // edge, so trumping and hogging are the same authority read in opposite
    // directions: Ultimate keeps the NEWEST grab (smallest `elapsed`) and
    // knocks the old holder off; Melee keeps the body that got there FIRST and
    // the newcomer is the one who loses. Both are coherent games.
    //
    // ⛔ NO SECOND RULE ABOUT WHO MAY GRAB. The loser is knocked off by the
    // same path with the same pop either way — a hog that refused the grab
    // outright would be a second ledge authority, which is what the parity row
    // rules out.
    //
    // The `SimId` tiebreak stays ascending in both, so a same-tick contest has
    // a deterministic winner independent of query or archetype order.
    let hog = matches!(
        rules.as_ref().map(|r| r.ledge_occupancy),
        Some(crate::combat::rules::LedgeOccupancy::Hog)
    );
    holders.sort_by(|a, b| {
        let by_time = a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal);
        let by_time = if hog { by_time.reverse() } else { by_time };
        by_time.then_with(|| a.2.cmp(&b.2))
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

    let pop = rules.map_or(0.0, |rules| rules.ledge_trump_pop);
    for entity in trumped {
        let Ok((_, _, mut model, mut ledge, mut kin)) = bodies.get_mut(entity) else {
            continue;
        };
        // ⭐ THE OUTWARD DIRECTION IS THE HANG'S, read BEFORE the knock-off
        // clears it. `wall_normal_x` is the way the wall pushes, so it already
        // points away from the stage — and it is right for a body hanging
        // FACING OUT, which a reading off `kin.facing` would get backwards.
        let outward = if let ae::MotionModel::AxisSwept(axis) = &*model {
            axis.state
                .ledge_grab
                .as_ref()
                .map(|hang| hang.contact.wall_normal_x)
        } else {
            None
        };
        if ae::movement::knock_off_ledge(&mut model, &mut ledge) {
            // ⭐⭐ THE POP, and it is a declared rule rather than the law:
            // trumping exists in every platform fighter, being thrown off it
            // does not. `0.0` drops the loser where it hung.
            if pop > 0.0 {
                if let (Some(outward), Some(kin)) =
                    (outward.filter(|n| n.abs() > 0.0), kin.as_mut())
                {
                    kin.vel.x = outward.signum() * pop;
                }
            }
            // the window goes with the edge. It was bought with airtime this
            // body no longer has, and a falling fighter that kept it would be
            // the safest thing on the stage.
            if let ae::MotionModel::AxisSwept(axis) = &mut *model {
                axis.state.ledge_invuln_timer = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests;
