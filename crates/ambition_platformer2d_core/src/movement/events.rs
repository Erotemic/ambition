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
    Landed {
        impact_speed: f32,
        /// This body did not choose to be here: it was still falling out of a
        /// launch when the floor arrived (`AxisManeuverState::tumble_until_landing`
        /// on the way in). A crash, not a landing.
        ///
        /// ⛔ presentation must not re-derive this from `MovementOp::Knockdown`.
        /// That op is emitted by the control phase, which PRECEDES integration,
        /// so it sees `on_ground` only the tick after touchdown and is never in
        /// the same bundle as the impact speed that measured the fall — a splat
        /// built on the pair reads zero forever.
        involuntary: bool,
    },
    /// A known supported baseline became airborne during this step.
    LeftGround,
}

impl GroundContactTransition {
    pub const fn landing_impact_speed(self) -> Option<f32> {
        match self {
            Self::Landed { impact_speed, .. } => Some(impact_speed),
            _ => None,
        }
    }

    /// Did this step end in a CRASH — a landing the body did not choose?
    ///
    /// `false` for every transition that is not a landing, so a consumer asking
    /// "was that a splat" needs no match of its own.
    pub const fn landed_involuntarily(self) -> bool {
        matches!(self, Self::Landed { involuntary: true, .. })
    }
}

/// Why the body's owner is being asked to apply its reset policy this step.
///
/// The distinction is not cosmetic: "left the world" is the blast zone every
/// platform fighter is built on, and it has to survive the seam to be built on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetCause {
    /// The reset verb, pressed by whoever is driving this body. Not hazardous —
    /// nothing hurt the body, it asked.
    Requested,
    /// The body overlapped a hazard volume authored into the world.
    Hazard,
    /// The body was in water it cannot swim in.
    Drowned,
    /// The body passed the world's bounds along the fall direction, further
    /// than [`crate::world::WorldEdgeMargins::fall`]. Gravity-relative, so this is "fell out"
    /// under any gravity direction, not just downward.
    LeftTheWorld,
}

impl ResetCause {
    /// Whether the world did this TO the body, as opposed to the body asking.
    /// This is the predicate the old `hazard` bool carried, kept as a named
    /// question so reaction code does not re-derive it by listing variants.
    pub const fn is_hazardous(self) -> bool {
        !matches!(self, Self::Requested)
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
    /// Why this step is asking the body's owner to apply its reset policy, or
    /// `None` if it is not. Replaces the old `reset`/`hazard` bool pair, which
    /// could say THAT the world reset a body but never WHICH world did it.
    pub reset: Option<ResetCause>,
    /// The body's semantic ground-support transition for this movement step.
    /// Presentation and gameplay reactions consume this rather than deriving
    /// edges from default-initialized booleans.
    pub ground_contact: GroundContactTransition,
    pub contacts: Vec<crate::collision_semantics::Contact>,
    /// Axes on which the solids claiming the body admitted NO position this
    /// step — the body is over-constrained (crushed) between two surfaces. At
    /// most one per world axis.
    ///
    /// the kernel reports it and decides nothing about it, the same split
    /// [`ResetCause`] states: damage, death, a stock, a respawn, a forced
    /// displacement or crush immunity are the owner's policy. Nothing in the
    /// engine consumes this yet, deliberately.
    pub constraint_conflicts: Vec<crate::collision_semantics::AxisConstraintConflict>,
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
        // The FIRST cause across the two clocks wins. A body that asked to
        // reset during the control pass and then drifted out of the world
        // during the physics pass was reset by its own request; the void got
        // there second and is reporting a body that is already leaving.
        self.reset = self.reset.or(other.reset);
        if other.ground_contact != GroundContactTransition::Unchanged {
            self.ground_contact = other.ground_contact;
        }
        self.contacts.extend(other.contacts);
        self.constraint_conflicts.extend(other.constraint_conflicts);
    }
}
