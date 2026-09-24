//! The room graph's own rollback declaration.
//!
//! The world crate registers the types it defines (`RoomSet`, the live-room
//! instance, gate portals) rather than the actor runtime, so a capability's
//! state is declared by its owner. The stable names are wire identities, not
//! addresses; the owner label is organizational and the readable baseline
//! omits it.
//!
//! The active room's METADATA has no row: it is `RoomSet::active_metadata()`,
//! read where needed, so the set and a copy of its active entry cannot disagree
//! after a restore.

use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64, RollbackRegistrar};

/// Named to match `GATE_PORTAL_ROLLBACK_OWNER` beside it rather than derived
/// from `CARGO_PKG_NAME`: both declarations belong to the same crate and should
/// read the same way in a dump.
const OWNER: &str = "ambition_platformer2d_world";

/// The active/start room identity, which is what a desync check needs from the
/// graph.
///
/// ⭐ THE IDENTITY, NOT THE GRAPH. Rewinding into a different ROOM is the
/// divergence worth catching; the specs themselves are authored content and do
/// not change under simulation, so hashing them would cost a walk of the whole
/// world every checksum to detect nothing.
fn room_set_checksum(rooms: &super::RoomSet) -> u64 {
    let mut bytes = Vec::new();
    put_u64(&mut bytes, rooms.active as u64);
    put_u64(&mut bytes, rooms.start as u64);
    put_str(&mut bytes, &rooms.active_spec().id);
    checksum_bytes(&bytes)
}

/// Register the room graph's rewound state.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.require_rollback::<super::RoomSet>(OWNER, "root:room_set");
    registrar.rollback_component_clone_checksum::<super::RoomSet>(
        OWNER,
        "root.room_set",
        "active/start room identity checksum",
        room_set_checksum,
    );
    // ⭐ **WHICH LIVE ROOM, WHICH `root.room_set` CANNOT SAY.** The room-set
    // checksum above folds the active INDEX and the active room's id, and both
    // read the same after a session leaves a room and comes back — so a rewind
    // past the second publication restores a world the checksum above cannot
    // distinguish from the one before it. This row is the distinction, and it
    // is canonical rather than clone because the whole value is the identity.
    registrar.rollback_component_canonical::<super::LiveRoomInstance>(
        OWNER,
        "root.live_room_instance",
    );
}
