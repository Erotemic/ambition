//! Every body's death beat: how long a dead body is drawn
//! ([`DeathBeat`](ambition_combat::components::DeathBeat)).

use bevy::prelude::*;

use ambition_characters::actor::Corpse;
use ambition_combat::components::DeathBeat;

/// Advance each body's death beat by one sim tick, and fix what its corpse
/// does on the tick it dies: the body's own `corpse` trait, else the rule of
/// its room's game, else [`Corpse::default`].
///
/// Runs after combat in the same tick, so the beat starts on the tick of the
/// kill and the body is not hidden for a frame before its death row plays.
pub fn advance_death_beats(
    world_time: Res<ambition_time::WorldTime>,
    rules: crate::session::governing_rules::RulesOf<Corpse>,
    mut bodies: Query<(
        Entity,
        &ambition_characters::actor::BodyHealth,
        &mut DeathBeat,
        Option<&ambition_combat::CombatCapabilities>,
    )>,
) {
    // Sim clock: a death row slows with bullet time, as the body did.
    let dt = world_time.sim_dt();
    for (entity, health, mut beat, caps) in &mut bodies {
        beat.advance(health.alive(), dt, || {
            caps.and_then(|caps| caps.corpse)
                .or_else(|| rules.of(entity))
                .unwrap_or_default()
        });
    }
}
