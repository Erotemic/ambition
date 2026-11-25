//! Doors a script made (`create_mechanism`): each one an object of its own
//! live room.
//!
//! An authored lock wall is room DATA, shared by every live instance of its
//! room, so it cannot be one copy's door. A mechanism door is an occurrence
//! instead: its box and whether it is open live on its own entity, in the
//! live room it was made in. It uses the gate road an authored wall does: a
//! shut door is a solid in its own room's `gate_solids`, rebuilt each tick,
//! and open is the absence of that solid.

use bevy_ecs::prelude::*;

use ambition_platformer2d_core::snapshot::{put_bool, put_str, put_vec2, Reader, SnapshotState};
use ambition_platformer2d_shared_tangle::feature_overlay::RoomOverlays;
use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;

/// The block-name prefix a shut mechanism door contributes under.
pub const MECHANISM_DOOR_BLOCK_PREFIX: &str = "mechanism_door:";

/// A scripted door: a wall at `min`/`size` in its own live room while it is
/// shut. `id` is its construction identity, which a switch of the same
/// mechanism names to open or shut it.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct MechanismDoor {
    pub id: String,
    pub min: ambition_platformer2d_core::Vec2,
    pub size: ambition_platformer2d_core::Vec2,
    pub open: bool,
}

impl MechanismDoor {
    /// Open a shut door, shut an open one.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }
}

impl SnapshotState for MechanismDoor {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.id);
        put_vec2(out, self.min);
        put_vec2(out, self.size);
        put_bool(out, self.open);
    }

    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            id: reader.str()?.to_owned(),
            min: reader.vec2()?,
            size: reader.vec2()?,
            open: reader.bool()?,
        })
    }
}

/// Each shut door stands in the overlay of its own live room, and in no
/// other: a second copy of the room keeps its own doors.
pub fn contribute_mechanism_doors(doors: Query<(&MechanismDoor, Option<&InRoomInstance>)>, mut overlays: RoomOverlays) {
    for (door, room) in &doors {
        if door.open {
            continue;
        }
        let Some(mut overlay) = overlays.for_room(room) else {
            continue;
        };
        overlay.gate_solids.push(ambition_platformer2d_core::Block::solid(
            format!("{MECHANISM_DOOR_BLOCK_PREFIX}{}", door.id),
            door.min,
            door.size,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_door_round_trips_through_its_snapshot() {
        let door = MechanismDoor {
            id: "caller:3".into(),
            min: ambition_platformer2d_core::Vec2::new(10.0, -4.0),
            size: ambition_platformer2d_core::Vec2::new(16.0, 64.0),
            open: true,
        };
        let bytes = ambition_platformer2d_core::snapshot::encode_state(&door);
        assert_eq!(ambition_platformer2d_core::snapshot::decode_state::<MechanismDoor>(&bytes), Some(door));
    }
}
