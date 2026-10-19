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
    // The kill's own beat: the world holds a moment, and the screen shakes.
    tick: Option<Res<ambition_time::SimTick>>,
    mut hold: Option<ResMut<ambition_combat::impact_hitstop::ImpactHitstop>>,
    mut shake: MessageWriter<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>,
) {
    // Sim clock: a death row slows with bullet time, as the body did.
    let dt = world_time.sim_dt();
    let mut killed = false;
    for (entity, health, mut beat, caps) in &mut bodies {
        let was_dead = beat.since_s.is_some();
        beat.advance(health.alive(), dt, || {
            caps.and_then(|caps| caps.corpse)
                .or_else(|| rules.of(entity))
                .unwrap_or_default()
        });
        killed |= !was_dead && beat.since_s.is_some();
    }
    if killed {
        // A KILL LANDS: the world holds still a few ticks past the blow's own
        // hitlag (the impact freeze: an absolute tick, so a replayed frame
        // holds the same), and the screen shakes once.
        if let (Some(tick), Some(hold)) = (tick.as_deref(), hold.as_deref_mut()) {
            let until = tick.0 + KILL_FREEZE_TICKS;
            hold.until_tick = Some(hold.until_tick.map_or(until, |held| held.max(until)));
        }
        shake.write(ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest {
            amplitude_px: KILL_SHAKE_PX,
        });
    }
}

/// The world holds still this many sim ticks on a kill (60 Hz).
const KILL_FREEZE_TICKS: u64 = 5;
/// The screen shakes this much on a kill (px).
const KILL_SHAKE_PX: f32 = 4.0;
