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
    rules: ambition_platformer2d_world::governing_rules::RulesOf<Corpse>,
    mut bodies: Query<(
        Entity,
        &ambition_characters::actor::BodyHealth,
        &mut DeathBeat,
        Option<&ambition_combat::CombatCapabilities>,
        Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
    )>,
    // The kill's own beat, as the game of the body's room states it.
    beats: ambition_platformer2d_world::governing_rules::RulesOf<ambition_combat::KillBeat>,
    tick: Option<Res<ambition_time::SimTick>>,
    mut hold: Option<ResMut<ambition_combat::impact_hitstop::ImpactHitstop>>,
    mut shake: MessageWriter<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>,
    // A death row plays at its room's pace, and a kill holds its own room
    // (ISL-1b): with several live rooms the shared hold stops no clock.
    mut paces: bevy::ecs::system::ParamSet<(
        ambition_combat::impact_hitstop::RoomFreeze,
        ambition_combat::impact_hitstop::RoomHolds,
    )>,
) {
    // Sim clock: a death row slows with bullet time, as the body did.
    let dt = world_time.sim_dt();
    let mut beat_of_kills: Option<ambition_combat::KillBeat> = None;
    // The loudest kill of each live room shakes that room's views (ISL-1b).
    let mut shake_of_rooms: std::collections::BTreeMap<Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>, f32> =
        std::collections::BTreeMap::new();
    // And the longest kill freeze of each live room, which holds that room.
    let mut freeze_of_rooms = std::collections::BTreeMap::new();
    for (entity, health, mut beat, caps, room) in &mut bodies {
        let was_dead = beat.since_s.is_some();
        let dt = dt * paces.p0().scale_of(entity);
        beat.advance(health.alive(), dt, || {
            caps.and_then(|caps| caps.corpse)
                .or_else(|| rules.of(entity).copied())
                .unwrap_or_default()
        });
        if !was_dead && beat.since_s.is_some() {
            if let Some(kill) = beats.of(entity).copied() {
                let held = shake_of_rooms.entry(room.map(|room| room.0)).or_insert(0.0);
                *held = held.max(kill.shake_px);
                if let Some(room) = room {
                    let freeze = freeze_of_rooms.entry(room.0).or_insert(0.0f32);
                    *freeze = freeze.max(kill.freeze_s);
                }
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
            // And each room a kill landed in, for its own kill's freeze: the
            // hold that stops it while other rooms are live.
            let after = |seconds: f32| tick.0 + (seconds * TICKS_PER_SECOND).round().max(0.0) as u64;
            paces.p1().hold_until(
                freeze_of_rooms.iter().filter(|(_, seconds)| **seconds > 0.0).map(|(room, seconds)| (*room, after(*seconds))),
            );
        }
        for (room, amplitude_px) in shake_of_rooms {
            if amplitude_px > 0.0 {
                shake.write(ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest { amplitude_px, room });
            }
        }
    }
}

/// The tick rate a kill beat's seconds are counted in: a constant, so a
/// replayed frame counts the same.
const TICKS_PER_SECOND: f32 = 60.0;
