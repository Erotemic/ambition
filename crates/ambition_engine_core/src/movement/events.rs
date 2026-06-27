use crate::Vec2;

use super::MovementOp;

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
}

impl FrameEvents {
    /// Push to the per-frame op list and append a fresh `ComboMark`
    /// to the cluster-side combo trace.
    pub fn op_clusters(
        &mut self,
        combo_trace: &mut crate::player_clusters::BodyComboTrace,
        op: MovementOp,
    ) {
        self.operations.push(op);
        combo_trace.combo.push(super::ComboMark { op, age: 0.0 });
        if combo_trace.combo.len() > 18 {
            let excess = combo_trace.combo.len() - 18;
            combo_trace.combo.drain(0..excess);
        }
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
    }
}
