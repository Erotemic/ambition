//! Rollback wire encoders for the seed's own components. The orphan rule puts
//! them here: `SnapshotCursor` is core's and the type is this crate's, so the
//! actor kernel may implement neither. The kernel still REGISTERS the type under
//! its stable name (`actor.motion_path`).

use ambition_platformer2d_core::snapshot::{put_bool, put_i32, put_u32, SnapshotCursor};

use crate::ActorMotionPath;

impl SnapshotCursor for ActorMotionPath {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        match &self.0 {
            Some(motion) => {
                let (segment, dir) = motion.cursor();
                put_bool(out, true);
                put_u32(out, segment as u32);
                put_i32(out, dir);
            }
            // A body with no path is a state a body with a path can reach.
            None => put_bool(out, false),
        }
    }
}
