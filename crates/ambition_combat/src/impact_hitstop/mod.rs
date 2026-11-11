//! The match-level impact freeze: an absolute expiry tick, held in rollback
//! state, that stops the sim clock for a connect nobody is playing.
//!
//! Every rung of the clock ladder read slot zero, so a CPU-versus-CPU match
//! produced local hitlag on both bodies and NO screen freeze — the beat that
//! sells a connect was a player-shaped affordance in a game whose fights are
//! frequently between two CPUs. ⛔ the fix is not a fake primary player: a
//! match's freeze is a fact about the MATCH.
//!
//! ⛔⛔ IT CANNOT BE DERIVED FROM `BodyCombat::hitstop_timer`, which is the
//! obvious implementation. A player's reaction timers decay on `wall_dt`; an
//! actor's and a boss's decay on `sim_dt` (a test in the monolith guards that
//! split). A freeze driven off a CPU's hitstop would stop the clock that decays
//! the hitstop — the stuck-at-zero shape this engine has already paid for once.
//!
//! ⛔⛔ AND IT CANNOT BE AN UNREGISTERED WALL-CLOCK VALUE on the camera-shake
//! precedent, which is how it was first written. A shake writes PRESENTATION;
//! this writes the `dt` every body integrates with, so a resimulated frame ran
//! at a different pace and
//! `combat_equipment_switch_and_breakable_survive_forced_rollback_identically`
//! reported a checksum mismatch at frames [18, 19, 20]. Registering the float
//! would not have helped either: wall time is not a deterministic input to a
//! replayed frame.
//!
//! ⭐⭐ SO IT IS AN ABSOLUTE EXPIRY AGAINST `SimTick`, which is already rollback
//! state and already advances while `sim_dt == 0`. That is the whole design:
//!
//! * it cannot freeze its own expiry, because the tick it is measured against
//!   does not stop when the clock is scaled to zero;
//! * it is the same on a replayed frame, because both the expiry and the clock
//!   it is compared to rewind together;
//! * overlapping connects are `max`, which is deterministic and needs no
//!   ordering rule;
//! * and there is NO HAND-BACK. Nothing here writes a scale. The ladder re-reads
//!   every fact every frame and its last rung is `1.0`, so an expired hold
//!   restores the pace by ceasing to be true. The imperative
//!   `set 0 … remember to set 1` lifecycle is what produced the original
//!   stuck-at-zero bug and is deliberately absent.

use bevy::prelude::*;

/// The tick this freeze ends on, or `None` when the world is not frozen.
///
/// ⛔ AN ABSOLUTE EXPIRY, not a decrementing counter. A counter is mutable state
/// touched every tick for no reason; an expiry is written once per connect and
/// read thereafter, and two overlapping hits combine with `max` rather than
/// with an accumulation rule somebody has to remember.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImpactHitstop {
    pub until_tick: Option<u64>,
}

impl ImpactHitstop {
    /// Is the world frozen on this tick?
    pub fn is_freezing(&self, now: ambition_time::SimTick) -> bool {
        self.until_tick.is_some_and(|until| now.0 < until)
    }
}

type Room = ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;

/// One live room's own pace (ISL-1b), on the room's root entity: the tick a
/// connect's hold of the room ends on, and the room's eased clock scale.
///
/// With one live room the shared clock carries both (the impact hold stops
/// it, bullet time scales it). With several it carries neither, and each
/// room's pace is read from here instead, so a hit in Alice's room freezes
/// Alice's room and Player 1's bullet time slows Player 1's room only.
///
/// ⭐ ON THE ROOT, NOT IN A RESOURCE. A room rewinds alone: the room snapshot
/// saves and restores every rollback component of the room's entities, its
/// root included, so a rewound room gets back the pace it had at the save. In
/// a resource keyed by room, a hold written after the save outlived the
/// rewind, and the replay ran frozen (`a_rewound_room_hurts_its_enemy_again`).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RoomPace {
    /// The tick the room's impact hold ends on; held while `now < held_until`.
    pub held_until: u64,
    /// The room's eased clock scale; `1.0` is the shared pace.
    pub clock_scale: f32,
}

impl Default for RoomPace {
    fn default() -> Self {
        Self { held_until: 0, clock_scale: 1.0 }
    }
}

impl RoomPace {
    /// The room's pace on tick `now`: `0.0` while held, else its clock scale.
    pub fn at(&self, now: ambition_time::SimTick) -> f32 {
        if now.0 < self.held_until {
            0.0
        } else {
            self.clock_scale
        }
    }
}

/// Whether the rooms' own clock scales are the clock: more than one room is
/// live. With one, the shared clock carries the slow-down and a room's scale
/// is not applied, so no effect is applied by both (review 2026-11-11, P2:
/// with 0.25 on each, a room ran at 0.0625). A room hidden for another
/// room's replay (`Disabled`) counts, as the clock's writer counts it.
pub fn rooms_keep_their_own_clocks(live_roots: usize) -> bool {
    live_roots > 1
}

/// The live rooms' roots, to hold a room: the writer's half of [`RoomPace`].
#[derive(bevy::ecs::system::SystemParam)]
pub struct RoomHolds<'w, 's> {
    roots: Query<'w, 's, (Entity, &'static Room, Option<&'static mut RoomPace>), With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>>,
    commands: Commands<'w, 's>,
}

impl RoomHolds<'_, '_> {
    /// Hold each room until its tick, keeping a later hold it already has.
    /// Several holds of one room this tick keep the latest, also for a room
    /// whose root has no pace yet.
    pub fn hold_until(&mut self, holds: impl IntoIterator<Item = (Room, u64)>) {
        let mut latest = std::collections::BTreeMap::<Room, u64>::new();
        for (room, until) in holds {
            let held = latest.entry(room).or_insert(until);
            *held = (*held).max(until);
        }
        for (room, until) in latest {
            let Some((root, _, pace)) = self.roots.iter_mut().find(|(_, at, _)| **at == room) else {
                continue;
            };
            match pace {
                Some(mut pace) => {
                    if pace.held_until < until {
                        pace.held_until = until;
                    }
                }
                None => {
                    self.commands.entity(root).insert(RoomPace { held_until: until, ..RoomPace::default() });
                }
            }
        }
    }
}

/// The pace of one body's room this tick ([`RoomPace`], `1.0` for a room with
/// none). A reader multiplies the sim `dt` it gives that body by it. With one
/// live room the shared clock carries the hold and the scale, and a room's
/// own scale is not applied ([`rooms_keep_their_own_clocks`]); with several,
/// only the room's own bodies change pace.
#[derive(bevy::ecs::system::SystemParam)]
pub struct RoomFreeze<'w, 's> {
    paces: Query<'w, 's, (&'static Room, &'static RoomPace), With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>>,
    // Every live room's root, one hidden for another's replay included.
    roots: Query<
        'w,
        's,
        (),
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
            bevy::ecs::query::Allow<bevy::ecs::entity_disabling::Disabled>,
        ),
    >,
    tick: Option<Res<'w, ambition_time::SimTick>>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms<'w, 's>,
}

impl RoomFreeze<'_, '_> {
    /// The pace of live room `room`.
    pub fn scale_in(&self, room: Option<Room>) -> f32 {
        let (Some(room), Some(tick)) = (room, self.tick.as_deref()) else {
            return 1.0;
        };
        let Some((_, pace)) = self.paces.iter().find(|(at, _)| **at == room) else {
            return 1.0;
        };
        let pace = pace.at(*tick);
        // A hold is the same stop on either clock; a scale is the shared
        // clock's while one room is live.
        if pace == 0.0 || rooms_keep_their_own_clocks(self.roots.iter().count()) {
            pace
        } else {
            1.0
        }
    }

    /// The live rooms this pace is read against.
    pub fn live(&self) -> &ambition_platformer2d_shared_tangle::lifecycle::LiveRooms<'_, '_> {
        &self.live
    }

    /// The live room `entity` is in ([`LiveRooms::of`](ambition_platformer2d_shared_tangle::lifecycle::LiveRooms::of)).
    pub fn of(&self, entity: Entity) -> Option<Room> {
        self.live.of(entity)
    }

    /// The pace of the room `entity` is in.
    pub fn scale_of(&self, entity: Entity) -> f32 {
        if self.paces.is_empty() {
            return 1.0;
        }
        self.scale_in(self.live.of(entity))
    }
}

/// The tick rate the authored hitlag seconds are converted against.
///
/// ⛔ A CONSTANT, not a frame delta. The conversion has to give the same answer
/// on a resimulated frame as it did live.
const TICKS_PER_SECOND: f32 = 60.0;

/// Does this source produce the beat a freeze is FOR?
///
/// ⛔⛔ A CONNECT, NOT AN INJURY. `ResolvedBodyHit` comes off the resolver, which
/// also serves contact attrition, hazards and the blast zone — a fighter leaning
/// on another was measured producing SEVENTEEN `Contact` resolutions in
/// twenty-three ticks, one damage each. A freeze armed by those alternates
/// frozen and moving forever, and three smash fixtures stopped reaching the gait
/// they were waiting for. Standing in lava is not a hit connecting.
fn is_a_connect(source: &crate::HitSource) -> bool {
    matches!(
        source,
        crate::HitSource::Melee | crate::HitSource::Projectile | crate::HitSource::Pogo
    )
}

/// A RESOLVED connect sets the freeze's expiry, bounded by the authored hitlag.
///
/// ⭐ THE VICTIM'S OWN RESOLVED HITLAG, not a flat rate — a jab barely stops the
/// screen and a smash stops it hard, which is the genre's behaviour. A flat
/// `feel.hitlag_time` per connect was measured and is too much: it is the
/// reference for the HARDEST hit.
///
/// ⛔⛔ AND IT READS THE RESOLUTION, NOT THE VICTIM. This used to take
/// `LandedBodyHit` — which means OVERLAP — and go looking for
/// `BodyCombat::hitstop_timer` on the victim. That timer is written this frame
/// for an actor victim and NEXT frame for a player victim, because player-victim
/// hits are staged into a rollback FIFO the player resolver drains in
/// `PlayerSimulation`. **So the connect that stopped the world for a CPU did not
/// stop it for the human** — measured 2026-08-25 in the real schedule: 0.036s of
/// player hitlag, zero frozen frames, and every existing arm blind to it because
/// they all injected the timer before firing the message.
///
/// ⛔ THE FIX IS NOT A SCHEDULE REORDER. `ResolvedBodyHit` carries the resolver's
/// own answer, so this system no longer has an opinion about which road resolved
/// the hit or on which frame — at the price of having to say which SOURCES are a
/// connect, which `LandedBodyHit` used to answer by construction.
pub fn request_impact_hitstop_on_resolved_hits(
    mut hits: MessageReader<crate::hitbox::ResolvedBodyHit>,
    feel: Option<Res<crate::feel::Platformer2dFeelTuningMonolith>>,
    room_rules: crate::rules::CombatTuningOf,
    tick: Option<Res<ambition_time::SimTick>>,
    mut hold: ResMut<ImpactHitstop>,
    // A connect holds the victim's room.
    mut holds: RoomHolds,
) {
    let (Some(feel), Some(tick)) = (feel, tick) else {
        // A headless fixture that installed no feel route still runs this
        // schedule; no tuning means no freeze rather than a panic.
        hits.clear();
        return;
    };
    let mut wanted = 0.0f32;
    // The hold of each struck room, by the victim's room.
    let mut in_rooms: Vec<(Room, f32)> = Vec::new();
    for hit in hits.read() {
        if !is_a_connect(&hit.source) {
            continue;
        }
        // The victim's room states the game's rules; they fold into a copy
        // of the feel, as every damage system folds them (the resource is
        // the engine's). A game whose hitlag is per body (a platform
        // fighter) asks for no world freeze: the two bodies hold, the stage
        // plays on.
        let mut feel = *feel;
        room_rules.of(hit.victim).unwrap_or_default().fold_into_feel(&mut feel);
        if !feel.world_freezes_on_hit {
            continue;
        }
        let seconds = hit.hitlag_seconds.min(feel.hitlag_time);
        wanted = wanted.max(seconds);
        if let Some(room) = room_rules.room_of(hit.victim) {
            in_rooms.push((room, seconds));
        }
    }
    let until_after = |seconds: f32| tick.0 + (seconds * TICKS_PER_SECOND).round().max(0.0) as u64;
    holds.hold_until(in_rooms.into_iter().filter(|(_, seconds)| *seconds > 0.0).map(|(room, seconds)| (room, until_after(seconds))));
    if wanted <= 0.0 {
        return;
    }
    let until = until_after(wanted);
    hold.until_tick = Some(hold.until_tick.map_or(until, |held| held.max(until)));
}

#[cfg(test)]
mod tests;
