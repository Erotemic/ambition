use crate::geometry::Aabb;
use crate::Vec2;

use super::{MovementOp, Player};

/// Engine event emitted when a blink teleports the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlinkEvent {
    pub from: Vec2,
    pub to: Vec2,
    pub precision: bool,
}

/// Engine events emitted by one player simulation step.
#[derive(Clone, Debug, Default)]
pub struct FrameEvents {
    pub operations: Vec<MovementOp>,
    pub blinks: Vec<BlinkEvent>,
    pub reset: bool,
    pub hazard: bool,
    /// AABBs of pogo-orb-like blocks the player bounced off this frame.
    /// The sandbox uses this to damage breakable pogo orbs whose runtime
    /// AABB matches; non-breakable pogo orbs are ignored.
    pub pogo_hits: Vec<Aabb>,
}

impl FrameEvents {
    pub(crate) fn op(&mut self, player: &mut Player, op: MovementOp) {
        self.operations.push(op);
        player.record(op);
    }

    /// Merge another event bundle into this frame.
    ///
    /// This is used by the two-clock update path: control/intent is processed
    /// in real time, then physical evolution is processed in scaled game time.
    pub fn extend(&mut self, other: FrameEvents) {
        self.operations.extend(other.operations);
        self.blinks.extend(other.blinks);
        self.reset |= other.reset;
        self.hazard |= other.hazard;
        self.pogo_hits.extend(other.pogo_hits);
    }
}
