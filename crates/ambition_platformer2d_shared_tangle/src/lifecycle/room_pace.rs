//! A live room's own pace (ISL-1b): its impact hold and its eased clock
//! scale, on its root ([`RoomPace`]), the writer's half ([`RoomHolds`]) and
//! the reader's ([`RoomFreeze`]).
//!
//! Here, beside the live rooms, so a system in any crate that ticks a body's
//! own clock can run it at its room's pace without naming combat. Combat holds
//! a room on a connect (`ambition_combat::impact_hitstop`), the clock control
//! eases a room's scale, and combat registers the component for rollback.

use bevy::prelude::*;

type Room = crate::lifecycle::LiveRoomInstance;

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
    roots: Query<'w, 's, (Entity, &'static Room, Option<&'static mut RoomPace>), With<crate::lifecycle::RoomInstanceRoot>>,
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
    paces: Query<'w, 's, (&'static Room, &'static RoomPace), With<crate::lifecycle::RoomInstanceRoot>>,
    // Every live room's root, one hidden for another's replay included.
    roots: Query<
        'w,
        's,
        (),
        (
            With<crate::lifecycle::RoomInstanceRoot>,
            bevy::ecs::query::Allow<bevy::ecs::entity_disabling::Disabled>,
        ),
    >,
    tick: Option<Res<'w, ambition_time::SimTick>>,
    live: crate::lifecycle::LiveRooms<'w, 's>,
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
    pub fn live(&self) -> &crate::lifecycle::LiveRooms<'_, '_> {
        &self.live
    }

    /// The live room `entity` is in ([`LiveRooms::of`](crate::lifecycle::LiveRooms::of)).
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

