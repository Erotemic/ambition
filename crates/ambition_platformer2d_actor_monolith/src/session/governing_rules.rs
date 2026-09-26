//! The rules that govern the active room.

use bevy::prelude::*;

use ambition_combat::scoped_rules::DeclaredRules;
use ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef;
use ambition_platformer2d_world::rooms::RoomSet;

/// Resolve one kind of rule `T` from the active room's mode.
///
/// It stores nothing: the declarations are authored constants and the active
/// room is session state, so a rewind that restores the room restores the
/// answer. A resolved-rules resource written each tick would be a second copy
/// that rollback must also restore.
#[derive(bevy::ecs::system::SystemParam)]
pub struct GoverningRules<'w, 's, T: Copy + std::fmt::Debug + Send + Sync + 'static> {
    declared: Option<Res<'w, DeclaredRules<T>>>,
    rooms: Option<SessionWorldRef<'w, 's, RoomSet>>,
}

impl<T: Copy + std::fmt::Debug + Send + Sync + 'static> GoverningRules<'_, '_, T> {
    /// The rules in force for the active room, or `None` when no game stated
    /// them for it (or there is no session yet).
    pub fn get(&self) -> Option<T> {
        let mode = self
            .rooms
            .as_ref()
            .and_then(|rooms| rooms.active_metadata().mode.as_deref());
        self.declared.as_ref()?.governing(mode)
    }
}
