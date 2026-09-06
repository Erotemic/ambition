//! The room graph's own rollback declaration.
//!
//! ⛔⛔ IT LIVED IN THE ACTOR MONOLITH, WHOSE HEADER PROMISES OTHERWISE. That
//! file says *"the actor runtime names only state defined in this crate"* and
//! then registered `RoomSet`, `ActiveRoomMetadata` and `RoomMusicRequest` — all
//! three defined here — along with the room-set checksum. Removing the
//! `crate::rooms` re-export facade made the contradiction impossible to miss:
//! the code stopped LAUNDERING the world's types through a monolith path, and
//! the rollback census went on claiming them anyway.
//!
//! ⭐ THE RUNTIME'S RULE, applied: *"each capability names its own concrete
//! types and projections; adding state to an existing domain edits only that
//! domain."* `register_gate_portal_rollback_state` beside this was already the
//! proof that a world-owned declaration composes.
//!
//! ⛔ THE STABLE NAMES DO NOT MOVE. `root.room_set`, `root.active_room_metadata`
//! and `root.room_music_request` are identities on the wire, not addresses, so
//! this is a repoint and NOT a schema change. Only the owner label changes, and
//! the readable baseline omits owner labels because ownership is organizational.

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
    registrar.rollback_component_clone::<super::ActiveRoomMetadata>(
        OWNER,
        "root.active_room_metadata",
    );
    registrar
        .rollback_component_clone::<super::RoomMusicRequest>(OWNER, "root.room_music_request");
}
