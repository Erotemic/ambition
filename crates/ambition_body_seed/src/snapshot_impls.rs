//! Rollback wire encoders for the seed's own components. The orphan rule puts
//! them here: `SnapshotCursor` is core's and the type is this crate's, so the
//! actor kernel may implement neither. The kernel still REGISTERS the type under
//! its stable name (`actor.motion_path`).

use ambition_platformer2d_core::snapshot::{put_bool, put_u64, SnapshotCursor};

use crate::ActorMotionPath;

impl SnapshotCursor for ActorMotionPath {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        match &self.0 {
            // The route's continuation is its phase; the prepared path is
            // authored content and never changes.
            Some(motion) => {
                put_bool(out, true);
                put_u64(out, motion.phase().0.to_bits());
            }
            // A body with no path is a state a body with a path can reach.
            None => put_bool(out, false),
        }
    }
}
