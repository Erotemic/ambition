//! The game half of `smash.time_dilation`: who is slow, and for how long.
//!
//! ⭐⭐ THE MOVE ASKS; THE ADAPTER OWNS THE CLOCK. That is this campaign's rule
//! and it is the same shape the parasol's gravity modifier took: an authored
//! move writes a scale and a duration, and something outside the move spends the
//! seconds and puts the world back. Nothing here is a second authority on TIME —
//! `ambition_time::ProperTimeScale` is the engine's, `WorldTime::entity_dt` is
//! what reads it, and this decides only which body carries which number.
//!
//! ⛔ THE TIMER IS ITS OWN COMPONENT RATHER THAN A FIELD ON `ProperTimeScale`.
//! That component is `ambition_time`'s and is rollback-canonical under a stable
//! name; growing it a duration would make the time crate carry a smash rule and
//! would move a wire format shared by everything. ⇒ A component beside it says
//! the same thing and leaves the engine's vocabulary alone.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::smash_time_dilation::{TimeDilationParams, TIME_DILATION};
use ambition_platformer2d::engine_core as ae;

/// A body currently running on a slowed clock, and what to put back.
///
/// ⛔ IT REMEMBERS THE PRIOR SCALE RATHER THAN ASSUMING `1.0`. A body could be
/// dilated by something else the day another source exists, and restoring a
/// constant would silently become that source's off switch. ⚠ Today the prior is
/// always the default, which is exactly when this costs nothing and is exactly
/// when it is cheapest to get right.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct TimeDilated {
    /// World seconds left. ⛔ WORLD, NOT THE VICTIM'S OWN — a duration measured
    /// on the slowed clock would stretch itself, so this ticks on `sim_dt`.
    pub remaining_s: f32,
    /// The scale this body had before, restored when the clock runs out.
    pub prior: f32,
}

/// The rollback value projection: both fields decide how a body EXPERIENCES the
/// next tick, so a restore that lost either resimulates a different fight.
pub fn time_dilated_probe(d: &TimeDilated) -> u64 {
    (d.remaining_s.to_bits() as u64).rotate_left(23) ^ (d.prior.to_bits() as u64)
}

/// Put a body on a slower clock when a move asks.
pub fn apply_authored_time_dilations(
    mut commands: Commands,
    mut actions: MessageReader<ActorActionMessage>,
    mut bodies: Query<(
        Option<&mut ambition_platformer2d::time::ProperTimeScale>,
        Option<&TimeDilated>,
    )>,
) {
    for message in actions.read() {
        let ambition_platformer2d::characters::brain::action_set::ActionRequest::Special {
            spec,
            params,
        } = &message.request
        else {
            continue;
        };
        let ambition_platformer2d::characters::brain::action_set::SpecialActionSpec::Special(key) =
            spec;
        if key != TIME_DILATION {
            continue;
        }
        let Ok(params) = params.hydrate::<TimeDilationParams>() else {
            warn!("a time dilation did not hydrate its params");
            continue;
        };
        if !params.problems().is_empty() {
            warn!(
                "refusing an authored time dilation: {}",
                params.problems().join("; ")
            );
            continue;
        }
        let Ok((scale, already)) = bodies.get_mut(message.actor) else {
            continue;
        };
        // ⛔ A SECOND DILATION DOES NOT NEST. Two overlapping slows would
        // multiply into a body that is barely moving and would each try to
        // restore a prior the other had already changed. The newest wins and
        // keeps the ORIGINAL prior, so however many land, one restore returns
        // the body to the clock it started on.
        let prior = already.map(|d| d.prior).unwrap_or_else(|| {
            scale
                .as_ref()
                .map(|s| s.0)
                .unwrap_or(ambition_platformer2d::time::ProperTimeScale::default().0)
        });
        commands
            .entity(message.actor)
            .try_insert(ambition_platformer2d::time::ProperTimeScale(params.scale))
            .try_insert(TimeDilated {
                remaining_s: params.seconds,
                prior,
            });
        info!(
            target: "ambition::moves",
            "time dilated: scale={} for {}s", params.scale, params.seconds
        );
    }
}

/// Spend the dilation's clock and give the body its own time back.
///
/// ⛔⛔ ONE SYSTEM, because expiring and restoring are one decision about one
/// tick — the same reasoning the bomb and the plate give. A separate restorer
/// racing a separate expirer is how a body ends a match on somebody else's clock.
pub fn expire_time_dilations(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut dilated: Query<(
        Entity,
        &mut TimeDilated,
        &mut ambition_platformer2d::time::ProperTimeScale,
    )>,
) {
    // ⛔ THE WORLD'S SECOND, NOT THE BODY'S. `sim_dt` rather than `entity_dt`:
    // a slow that counted down on the clock it slowed would last `1/scale` times
    // as long as its author wrote, and halving the scale would more than double
    // the duration.
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    for (entity, mut dilation, mut scale) in &mut dilated {
        dilation.remaining_s -= dt;
        if dilation.remaining_s > 0.0 {
            continue;
        }
        scale.0 = dilation.prior;
        commands.entity(entity).try_remove::<TimeDilated>();
    }
}

/// Body-local units are unused here; the import keeps the module's engine
/// vocabulary consistent with its siblings.
#[allow(dead_code)]
fn _engine_units(_: ae::Vec2) {}

#[cfg(test)]
mod tests;
