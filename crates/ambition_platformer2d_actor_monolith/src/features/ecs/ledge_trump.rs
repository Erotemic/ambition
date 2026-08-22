//! The ledge trump: one edge, one body.
//!
//! The third body-vs-body interaction beside [`super::capture`] and
//! [`super::footstool`], and it exists because the kernel's ledge grab cannot
//! see anybody else. `try_start_ledge_grab` asks one body about one wall, so two
//! fighters could hang on the SAME edge — both intangible, both safe, and the
//! stage's only recovery point shared rather than contested.
//!
//! ```text
//! a CAPTURE    volume overlap        -> a relationship that outlives the move
//! a FOOTSTOOL  feet on a head + jump -> two impulses and a stun, over at once
//! a TRUMP      two hands on one edge -> the later arrival keeps it
//! ```
//!
//! ## The later arrival wins, and that is the mechanic
//!
//! the edge belongs to whoever caught it MOST RECENTLY. That is the
//! genre's rule and it is the one that makes an edge contested: a fighter
//! hanging on the ledge to wait out a recovery can be taken off it by the very
//! body they were waiting for. `LedgeGrabState::elapsed` already counts the
//! seconds since a grab, so the trumper is simply the smaller number.
//!
//! and the trumped body loses its intangibility with the edge. It was
//! bought with airtime it no longer has — see
//! [`ae::ledge_grab::ledge_grab_invuln_earned`] — and a body that kept the
//! window while falling would be the safest thing on the stage.
//!
//! PARTIAL against the genre, and named rather than implied: a trumped
//! body is dropped, where Ultimate pops it outward into a brief helpless state.
//! The drop is the half that makes the edge contested; the pop is feel.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// How close two anchors must be to be the same edge, in world px. A ledge
/// anchor is written by the kernel from the same contact geometry for both
/// bodies, so this is a float-equality tolerance rather than a reach.
const SAME_EDGE_EPSILON: f32 = 1.0;

/// Take the edge away from whoever caught it first.
///
/// # Why the order is spelled out
///
/// Three bodies can be on one edge, and `elapsed` can tie — two fighters that
/// grabbed on the same tick have the same age to the float. Taking whichever
/// pair the query yields first makes the outcome depend on archetype order,
/// which is stable within a run and NOT stable across a rollback resimulation.
/// So the holders are sorted by `(elapsed, SimId)` and only the FIRST survives.
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
