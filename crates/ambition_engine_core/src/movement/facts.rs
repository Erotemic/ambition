//! The published, model-independent movement-facts vocabulary (ADR 0024).
//!
//! Axis maneuver state is policy-PRIVATE (it lives inside
//! [`AxisSweptMotion`](super::AxisSweptMotion)); animation, combat gates,
//! affordances, HUD, traces, time-control, and RL observations consume the
//! SEMANTIC facts published here instead of inspecting a policy's internals.
//! The facts are a projection: the drivers rewrite [`BodyMotionFacts`] from the
//! body's model right after each movement step, so a body running a non-axis
//! policy can never expose stale axis maneuver state — the projection of a
//! non-axis model is simply the default (no maneuver in flight).

use bevy_ecs::component::Component;

use super::model::MotionModel;
use crate::ledge_grab::LedgeGetupKind;
use crate::Vec2;

/// Semantic ledge-engagement facts (presentation-facing; the anchor and climb
/// curves stay policy-private).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LedgeFacts {
    /// False while hanging; true once a getup (climb/roll/attack) committed.
    pub climbing: bool,
    pub getup_kind: LedgeGetupKind,
}

/// Per-body semantic movement facts, republished every movement tick from the
/// active policy. THE read surface for everything outside the kernel that used
/// to peek at raw maneuver timers.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyMotionFacts {
    /// An active dash is in flight.
    pub dashing: bool,
    /// Dodge-roll i-frames are active.
    pub dodge_rolling: bool,
    /// The blink telegraph is showing (precision aim or charge hold).
    pub blink_telegraph: bool,
    /// Precision blink aim specifically (drives the aim preview).
    pub blink_aiming: bool,
    /// The precision-blink aim offset (body-local; presentation preview data).
    pub blink_aim_offset: Vec2,
    /// Post-blink grace i-frames are active.
    pub blink_grace: bool,
    pub wall_clinging: bool,
    pub wall_climbing: bool,
    pub gliding: bool,
    pub fast_falling: bool,
    /// Ledge engagement, if any.
    pub ledge: Option<LedgeFacts>,
}

impl BodyMotionFacts {
    /// Project the active policy's semantic facts. Non-axis policies have no
    /// axis maneuvers by construction — their projection is the default.
    pub fn from_model(model: &MotionModel) -> Self {
        let MotionModel::AxisSwept(axis) = model else {
            return Self::default();
        };
        let state = &axis.state;
        Self {
            dashing: state.dash_timer > 0.0,
            dodge_rolling: state.dodge_roll_timer > 0.0,
            blink_telegraph: state.blink_aiming || state.blink_hold_active,
            blink_aiming: state.blink_aiming,
            blink_aim_offset: state.blink_aim_offset,
            blink_grace: state.blink_grace_timer > 0.0,
            wall_clinging: state.wall_clinging,
            wall_climbing: state.wall_climbing,
            gliding: state.gliding,
            fast_falling: state.fast_falling,
            ledge: state.ledge_grab.as_ref().map(|grab| LedgeFacts {
                climbing: grab.climbing,
                getup_kind: grab.getup_kind,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::{AxisSweptParams, MomentumParams};

    #[test]
    fn axis_maneuvers_project_to_semantic_facts() {
        let mut model = MotionModel::axis_swept(AxisSweptParams::default());
        let MotionModel::AxisSwept(axis) = &mut model else {
            unreachable!();
        };
        axis.state.dash_timer = 0.1;
        axis.state.blink_hold_active = true;
        axis.state.wall_clinging = true;
        let facts = BodyMotionFacts::from_model(&model);
        assert!(facts.dashing && facts.blink_telegraph && facts.wall_clinging);
        assert!(!facts.blink_aiming && !facts.gliding && facts.ledge.is_none());
    }

    #[test]
    fn a_non_axis_policy_can_never_expose_stale_axis_facts() {
        // Even if a body previously dashed/clung as an axis body, its facts
        // under another policy are the default — the leak O4 exists to close.
        let model = MotionModel::surface_momentum(MomentumParams::default());
        assert_eq!(
            BodyMotionFacts::from_model(&model),
            BodyMotionFacts::default()
        );
    }
}
