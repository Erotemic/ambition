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
    rules: ambition_combat::governing_rules::RulesOf<Corpse>,
    mut bodies: Query<(
        Entity,
        &ambition_characters::actor::BodyHealth,
        &mut DeathBeat,
        Option<&ambition_combat::CombatCapabilities>,
    )>,
    // The kill's own beat, as the game of the body's room states it.
    beats: ambition_combat::governing_rules::RulesOf<ambition_combat::KillBeat>,
    tick: Option<Res<ambition_time::SimTick>>,
    mut hold: Option<ResMut<ambition_combat::impact_hitstop::ImpactHitstop>>,
    mut shake: MessageWriter<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>,
) {
    // Sim clock: a death row slows with bullet time, as the body did.
    let dt = world_time.sim_dt();
    let mut beat_of_kills: Option<ambition_combat::KillBeat> = None;
    for (entity, health, mut beat, caps) in &mut bodies {
        let was_dead = beat.since_s.is_some();
        beat.advance(health.alive(), dt, || {
            caps.and_then(|caps| caps.corpse)
                .or_else(|| rules.of(entity).copied())
                .unwrap_or_default()
        });
        if !was_dead && beat.since_s.is_some() {
            if let Some(kill) = beats.of(entity).copied() {
                beat_of_kills = Some(beat_of_kills.map_or(kill, |held| ambition_combat::KillBeat {
                    freeze_s: held.freeze_s.max(kill.freeze_s),
                    shake_px: held.shake_px.max(kill.shake_px),
                }));
            }
        }
    }
    if let Some(kill) = beat_of_kills {
        // A KILL LANDS: the world holds still past the blow's own hitlag (the
        // impact freeze: an absolute tick, so a replayed frame holds the
        // same), and the screen shakes once.
        if let (Some(tick), Some(hold)) = (tick.as_deref(), hold.as_deref_mut()) {
            let until = tick.0 + (kill.freeze_s * TICKS_PER_SECOND).round().max(0.0) as u64;
            hold.until_tick = Some(hold.until_tick.map_or(until, |held| held.max(until)));
        }
        if kill.shake_px > 0.0 {
            shake.write(ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest { amplitude_px: kill.shake_px });
        }
    }
}

/// The tick rate a kill beat's seconds are counted in: a constant, so a
/// replayed frame counts the same.
const TICKS_PER_SECOND: f32 = 60.0;
