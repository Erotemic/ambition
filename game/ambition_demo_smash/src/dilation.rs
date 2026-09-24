//! The game half of `smash.time_dilation`: who is slow, and for how long.
//!
//! The move asks; the adapter owns the clock. An authored move writes a scale
//! and a duration, and this module spends the seconds and restores the body.
//! `ambition_time::ProperTimeScale` is the engine's, `WorldTime::entity_dt`
//! reads it, and this module only decides which body carries which value.
//!
//! The timer is its own component, not a field on `ProperTimeScale`. That
//! component belongs to `ambition_time` and is rollback-canonical; adding a
//! duration would put a smash rule in the time crate and change a shared wire
//! format.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_time_dilation::{TimeDilationParams, TIME_DILATION};
use ambition_platformer2d::engine_core as ae;

/// A body currently running on a slowed clock, and what to put back.
///
/// It remembers the prior scale instead of assuming `1.0`, so that a future
/// second source of dilation is not switched off by this restore.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct TimeDilated {
    /// World seconds left. World time, not the victim's own: a duration on the
    /// slowed clock would stretch itself, so this ticks on `sim_dt`.
    pub remaining_s: f32,
    /// The scale this body had before, restored when the clock runs out.
    pub prior: f32,
}

/// The rollback value projection. Both fields change how a body experiences
/// the next tick, so a restore that lost either resimulates a different fight.
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
        // `message.actor` is the body the dilation lands on, not the caster.
        // `TimeDilationParams::scale` calls it the victim's clock; the effect
        // channel calls it the actor.
        //
        // The dispatcher picks the target. The clerk's Witch-Time reaches the
        // attacker because `smash.counter` sends its response with
        // `actor: parry.attacker` when `answers_the_attacker` is set.
        //
        // A move that emits this from its own timeline slows its own caster,
        // and that is correct.
        let Ok((scale, already)) = bodies.get_mut(message.actor) else {
            continue;
        };
        // A second dilation does not nest. Nested slows would multiply and
        // would each restore a prior the other changed. The newest wins and
        // keeps the original prior, so one restore returns the body to the
        // clock it started on.
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
/// One system, because expiring and restoring are one decision about one tick.
/// A separate restorer and expirer can leave a body on the wrong clock.
pub fn expire_time_dilations(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut dilated: Query<(
        Entity,
        &mut TimeDilated,
        &mut ambition_platformer2d::time::ProperTimeScale,
    )>,
) {
    // The world's second, not the body's (`sim_dt`, not `entity_dt`). A slow
    // that counted down on the clock it slowed would last `1/scale` times as
    // long as authored.
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
