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
}

impl CurrentRoom<'_, '_> {
    pub fn get(&self) -> ActiveRoom<'_> {
        self.rooms.as_ref().map_or(ActiveRoom::NoRoom, |rooms| {
            ActiveRoom::live(rooms.spec().metadata.mode.as_deref())
        })
    }

    /// Whether `scope` governs the active room.
    pub fn in_scope(&self, scope: RulesScope) -> bool {
        scope.governs(self.get())
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
