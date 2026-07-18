use crate::Vec2;

use super::MovementOp;

/// Semantic change in gravity-relative ground support across one movement
/// step. Initialization is explicit so a fresh body resting on authored floor
/// geometry does not impersonate a landing, while a body spawned airborne can
/// still land during its very first integration step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GroundContactTransition {
    /// A known contact baseline existed and did not change this step.
    #[default]
    Unchanged,
    /// The body's first contact sample found it supported, and it remained
    /// supported through this step.
    InitializedGrounded,
    /// The body's first contact sample found it airborne, and it remained
    /// airborne through this step.
    InitializedAirborne,
    /// A known airborne baseline became supported during this step.
    Landed { impact_speed: f32 },
    /// A known supported baseline became airborne during this step.
    LeftGround,
}

impl GroundContactTransition {
    pub const fn landing_impact_speed(self) -> Option<f32> {
        match self {
            Self::Landed { impact_speed } => Some(impact_speed),
            _ => None,
        }
    }
}

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
    /// The body's semantic ground-support transition for this movement step.
    /// Presentation and gameplay reactions consume this rather than deriving
    /// edges from default-initialized booleans.
    pub ground_contact: GroundContactTransition,
    /// World contacts resolved this step (fable review 2026-07-05 AJ10: the
    /// contact vocabulary). Pure observability — resolution is unchanged;
    /// readers interpret (the debug overlay, a future general resolver).
    /// Landing pushes a feet contact, a wall push a side contact, and a
    /// grounded frame a rest contact carrying the support's `surface_velocity`.
    pub contacts: Vec<crate::collision_semantics::Contact>,
}

impl FrameEvents {
    /// Push to the per-frame op list and append a fresh `ComboMark`
    /// to the cluster-side combo trace.
    pub fn op_clusters(
        &mut self,
        combo_trace: &mut crate::body_clusters::BodyComboTrace,
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
        if other.ground_contact != GroundContactTransition::Unchanged {
            self.ground_contact = other.ground_contact;
        }
        self.contacts.extend(other.contacts);
    }
}
