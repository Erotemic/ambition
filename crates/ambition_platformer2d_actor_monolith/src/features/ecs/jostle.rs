//! **Jostle: two grounded bodies do not stand inside each other.**
//!
//! The fourth body-vs-body interaction beside [`super::capture`],
//! [`super::footstool`] and [`super::ledge_trump`], and the one that makes the
//! stage have a spacing game at all.
//!
//! ```text
//! a CAPTURE    volume overlap        -> a relationship that outlives the move
//! a FOOTSTOOL  feet on a head + jump -> two impulses and a stun, over at once
//! a TRUMP      two hands on one edge -> the later arrival keeps it
//! a JOSTLE     two bodies in one place -> both are pushed apart, every tick
//! ```
//!
//! ## Why this is a PASS and not a term in the kernel
//!
//! ⭐⭐ **Jon ruled the scope of AVOID PUSHOUT on 2026-08-20, and the answer was
//! not the one either reading of the question expected:**
//!
//! > The no pushout rule I think is for portals, because I wanted them to be
//! > elegant. For bodies I think it might be ok. This isn't a hack, it is a game
//! > feel feature. If ultimate does it they must have rollback code for it. This
//! > is something that games will want, so we should be able to express it. **It
//! > should never be a mandatory part of the movement kernel though. It should be
//! > composable and not add to tech dept.**
//!
//! ⇒ so the binding constraint was never *whether*, it was *where*. A `step_body`
//! that jostled unconditionally would make every body in every composition pay
//! for a platform-fighter rule — the exact shape `BodyStaleMoves` and the capture
//! timeout each had to be moved OUT of after riding a generic bundle.
//!
//! A game opts in by declaring [`DeclaredCombatRules::jostle_accel`]. Undeclared
//! is `0.0`, this pass returns immediately, and nothing in that world moves
//! differently by one float.
//!
//! ## Why an ACCELERATION
//!
//! ⚠ **position is never written.** Each body takes a separating VELOCITY and the
//! kernel integrates it like any other force, so a rewind restores the same
//! answer from the same inputs and reversibility is untouched. A displacement
//! would separate more crisply; this reads as weight, which is what the genre
//! wants from two heavy bodies leaning on each other.
//!
//! ⛔ **and it is why this pass may not clamp the result.** Capping the separating
//! speed would make the push a function of the OVERLAP HISTORY rather than of the
//! current overlap, and that is state — the thing a derived pass must not carry.
//!
//! ## What it does not do
//!
//! ⚠ **grounded only.** Airborne bodies pass through each other, which is the
//! genre's rule: a fighter juggling another must be able to occupy the same space
//! to keep hitting them. ⚠ **no teams check** — jostle is geometry, not damage,
//! and Ultimate pushes teammates apart too.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Below this horizontal overlap, in px, two bodies are touching rather than
/// occupying each other and the push is not worth a float. Also the guard that
/// stops two exactly-coincident bodies producing a zero-length direction.
const TOUCHING_EPSILON: f32 = 0.5;

/// **Push every overlapping pair of grounded bodies apart.**
///
/// # Why the order is spelled out
///
/// The push is symmetric and additive, so the ARITHMETIC does not depend on
/// visiting order — but float addition is not associative, and two bodies each
/// accumulating three pushes in a different sequence give answers that differ in
/// the last bits. That is a desync across a rollback resimulation, where the
/// archetype order a query yields is stable within a run and not across one. So
/// the participants are collected and sorted by [`SimId`] before any pair is
/// considered.
pub fn resolve_jostle(
    rules: Option<Res<ambition_combat::rules::ResolvedCombatTuning>>,
    time: Res<ambition_time::WorldTime>,
    mut bodies: Query<(
        Entity,
        &SimId,
        &mut ae::BodyKinematics,
        &ae::BodyGroundState,
    )>,
) {
    // Undeclared is off, and off is the baseline every non-fighter world runs.
    let accel = rules.as_deref().map_or(0.0, |r| r.jostle_accel);
    if accel <= 0.0 {
        return;
    }
    let dt = time.scaled_dt;
    if dt <= 0.0 {
        return;
    }

    // (id, entity, centre x, half width) for every body standing on something.
    let mut standing: Vec<(SimId, Entity, f32, f32)> = Vec::new();
    for (entity, id, kin, ground) in bodies.iter() {
        if !ground.on_ground {
            continue;
        }
        standing.push((id.clone(), entity, kin.pos.x, kin.size.x * 0.5));
    }
    if standing.len() < 2 {
        return;
    }
    standing.sort_by(|a, b| a.0.cmp(&b.0));

    // Accumulate first, apply second: a body in three overlaps takes one
    // combined push, and no pair sees a velocity another pair already changed.
    let mut push: Vec<(Entity, f32)> = Vec::new();
    for i in 0..standing.len() {
        for j in (i + 1)..standing.len() {
            let (_, a_entity, a_x, a_half) = standing[i];
            let (_, b_entity, b_x, b_half) = standing[j];
            let gap = (b_x - a_x).abs();
            let touching = a_half + b_half;
            let overlap = touching - gap;
            if overlap <= TOUCHING_EPSILON {
                continue;
            }
            // ⚠ **proportional to how far INSIDE each other they are**, so a
            // glancing contact barely registers and a body shoved fully into
            // another is pushed hard. `touching` is never 0 for two real bodies,
            // and the epsilon above has already excluded the degenerate pair.
            let depth = (overlap / touching).clamp(0.0, 1.0);
            // ⛔ a deterministic direction for exactly-coincident bodies: the
            // SORTED order decides, so the answer does not depend on which body
            // the query happened to yield first.
            let a_pushed_left = if gap <= TOUCHING_EPSILON {
                true
            } else {
                a_x < b_x
            };
            let step = accel * depth * dt;
            let (a_dv, b_dv) = if a_pushed_left {
                (-step, step)
            } else {
                (step, -step)
            };
            push.push((a_entity, a_dv));
            push.push((b_entity, b_dv));
        }
    }

    for (entity, dv) in push {
        if let Ok((_, _, mut kin, _)) = bodies.get_mut(entity) {
            // ⚠ VELOCITY, never `pos`. See the module doc: the kernel integrates
            // this like any other force, which is what keeps a rewind honest.
            kin.vel.x += dv;
        }
    }
}

#[cfg(test)]
mod tests;
