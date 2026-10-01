//! The rules that govern the active room.

use bevy::prelude::*;

use ambition_combat::scoped_rules::{ActiveRoom, DeclaredRules, RulesScope};

/// The active room as a rule scope sees it: none, untagged, or in a mode.
///
/// The one reading of the session for rule scopes. [`GoverningRules`] and the
/// run conditions that gate a game's systems both read it, so a game's systems
/// run exactly where its rules govern.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CurrentRoom<'w, 's> {
    // The one-live-room read: a rule scope governs THE live room.
    rooms: Option<ambition_platformer2d_world::rooms::SoleLiveRoomSpec<'w, 's>>,
    live: LiveRuleRooms<'w, 's>,
    roots: Query<
        'w,
        's,
        &'static ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
}

impl CurrentRoom<'_, '_> {
    /// THE live room as a rule scope sees it. ⚠ The one-live-room read: with
    /// two live rooms it is `NoRoom`. A reader with a subject asks
    /// [`RulesOf`] instead.
    pub fn get(&self) -> ActiveRoom<'_> {
        self.rooms.as_ref().map_or(ActiveRoom::NoRoom, |rooms| {
            ActiveRoom::live(rooms.spec().metadata.mode.as_deref())
        })
    }

    /// Whether `scope` governs ANY live room, or the absence of a room when
    /// none is live. The question a gate with no subject asks: a game's
    /// systems run while its rules govern some live room, and each system
    /// keys its work by its subject's room (OW1). With one live room this is
    /// the same answer as before.
    pub fn in_scope(&self, scope: RulesScope) -> bool {
        let mut live = self.roots.iter().peekable();
        if live.peek().is_none() {
            return scope.governs(self.get());
        }
        live.any(|room| scope.governs(self.live.of(*room)))
    }
}

/// Each live room as a rule scope sees it: the reading WITH a subject, where
/// [`CurrentRoom`] is the reading without one (OW1).
///
/// [`CurrentRoom`] reads THE live room, so with two live rooms it reads none,
/// and a rule resolved from it is the rule of no room in both.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LiveRuleRooms<'w, 's> {
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs<'w, 's>>,
}

impl LiveRuleRooms<'_, '_> {
    /// The live room `entity` is in, by the rule of `LiveRooms::of`.
    pub fn room_of(
        &self,
        entity: Entity,
    ) -> Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance> {
        self.rooms.as_ref()?.live().of(entity)
    }

    /// Live room `room` as a rule scope sees it; `NoRoom` when it is not live.
    pub fn of(
        &self,
        room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    ) -> ActiveRoom<'_> {
        self.rooms
            .as_ref()
            .and_then(|rooms| {
                rooms
                    .definition_in(room)
                    .map(|definition| rooms.rooms().spec(definition).metadata.mode.as_deref())
            })
            .map_or(ActiveRoom::NoRoom, ActiveRoom::live)
    }
}

/// Resolve one kind of rule `T` from the active room.
///
/// It stores nothing: the declarations are authored constants and the active
/// room is session state, so a rewind that restores the room restores the
/// answer. A resolved-rules resource written each tick would be a second copy
/// that rollback must also restore.
#[derive(bevy::ecs::system::SystemParam)]
pub struct GoverningRules<'w, 's, T: Clone + std::fmt::Debug + Send + Sync + 'static> {
    declared: Option<Res<'w, DeclaredRules<T>>>,
    room: CurrentRoom<'w, 's>,
}

impl<T: Clone + std::fmt::Debug + Send + Sync + 'static> GoverningRules<'_, '_, T> {
    /// The rules in force for the active room, or `None` when no game stated
    /// them for it. With no session, only a whole-process (`EveryRoom`)
    /// declaration governs.
    pub fn get(&self) -> Option<T> {
        self.in_room(self.room.get())
    }

    /// The rules in force for `room` ([`LiveRuleRooms::of`] names a live one).
    pub fn in_room(&self, room: ActiveRoom<'_>) -> Option<T> {
        self.declared.as_ref()?.governing(room)
    }
}

/// The rules `T` in force for the live room of an entity: the reading WITH a
/// subject, where [`GoverningRules::get`] reads THE live room (OW1).
///
/// An entity in no live room (with two rooms live and no stamp) reads the
/// rules of no room, as `CombatTuningOf` does. With no session, only a
/// whole-process (`EveryRoom`) declaration governs, as before.
#[derive(bevy::ecs::system::SystemParam)]
pub struct RulesOf<'w, 's, T: Clone + std::fmt::Debug + Send + Sync + 'static> {
    rules: GoverningRules<'w, 's, T>,
    rooms: LiveRuleRooms<'w, 's>,
}

impl<T: Clone + std::fmt::Debug + Send + Sync + 'static> RulesOf<'_, '_, T> {
    /// The rules in force for the live room `entity` is in.
    pub fn of(&self, entity: Entity) -> Option<T> {
        self.in_live_room(self.rooms.room_of(entity))
    }

    /// The rules for `subject`'s room. With no subject, the rules of THE live
    /// room, as before: one live room answers as it did, and two answer the
    /// rules of no room.
    pub fn of_subject(&self, subject: Option<Entity>) -> Option<T> {
        match subject {
            Some(subject) => self.of(subject),
            None => self.rules.get(),
        }
    }

    /// The live room `entity` is in. Readers that resolve many subjects key a
    /// cache by it and ask [`Self::in_live_room`] once for each room.
    pub fn room_of(
        &self,
        entity: Entity,
    ) -> Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance> {
        self.rooms.room_of(entity)
    }

    /// The rules in force for live room `room`; `None` is no room.
    pub fn in_live_room(
        &self,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    ) -> Option<T> {
        match room {
            Some(room) => self.rules.in_room(self.rooms.of(room)),
            None => self.rules.in_room(ActiveRoom::NoRoom),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
