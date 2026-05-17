//! Bevy-side resource wrapping [`ae::CombatSlotBoard`].
//!
//! The slot board is per-target. Today we have one player target, so
//! we hold one shared board as a global resource. When co-op /
//! multiple targets land, this becomes a per-player resource or a
//! map keyed on player slot id.
//!
//! The board is initialized on app startup with a default layout
//! (3 melee ring slots, 3 aerial arc slots) and re-armed (assignments
//! cleared) on every room transition so dead enemies don't leak
//! ghost reservations into the new room.

use ambition_engine as ae;
use bevy::prelude::*;

#[derive(Resource)]
pub struct CombatSlotsRes(pub ae::CombatSlotBoard);

impl Default for CombatSlotsRes {
    fn default() -> Self {
        Self(ae::CombatSlotBoard::new(3, 90.0, 3, 220.0, 160.0))
    }
}

/// Clear every assignment on the board. Called from the room
/// transition path so freshly spawned enemies see an empty slot list.
pub fn clear_combat_slots_on_room_change(mut slots: ResMut<CombatSlotsRes>) {
    slots.0.clear_assignments();
}
