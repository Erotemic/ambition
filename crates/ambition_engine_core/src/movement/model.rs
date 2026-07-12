//! Movement-model identity, parameters, and persistent solver state.
//!
//! A body always owns one explicit [`MotionModel`].  The variant is the
//! swappable physics policy; each variant owns the parameters and private state
//! its solver needs.  World-space body state (`BodyKinematics` and the shared
//! clusters) remains outside the model so changing policies preserves position,
//! velocity, facing, abilities, and body mode by construction.

use bevy_ecs::component::Component;

use super::surface_momentum::{MomentumParams, SurfaceMotion};
use super::AxisSweptParams;

/// Stable identity for diagnostics, authoring, and transition tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionModelKind {
    AxisSwept,
    SurfaceMomentum,
}

/// Authored/runtime request for a movement policy.
///
/// This is intentionally state-free. Applying a spec to an existing model
/// preserves private state when the variant is unchanged and resets only the
/// destination solver's private state when the variant changes.
#[derive(Clone, Copy, Debug)]
pub enum MotionModelSpec {
    AxisSwept(AxisSweptParams),
    SurfaceMomentum(MomentumParams),
}

/// Axis-swept model-owned data.
///
/// Its historical timers/contact state still live in shared body clusters as
/// migration debt. This wrapper owns the solver parameters and is the destination
/// for coyote, jump-buffer, wall/ledge, dash/blink, and axis-contact state whose
/// meaning does not survive a policy change.
#[derive(Clone, Copy, Debug)]
pub struct AxisSweptMotion {
    pub params: AxisSweptParams,
}

impl AxisSweptMotion {
    pub const fn new(params: AxisSweptParams) -> Self {
        Self { params }
    }
}

impl Default for AxisSweptMotion {
    fn default() -> Self {
        Self::new(AxisSweptParams::default())
    }
}

/// Surface-momentum model-owned parameters and persistent solver state.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceMomentumMotion {
    pub params: MomentumParams,
    pub state: SurfaceMotion,
    /// Simulated-depth lane retained through airborne crossover motion.
    pub depth_lane: i8,
}

impl SurfaceMomentumMotion {
    pub fn new(params: MomentumParams) -> Self {
        Self {
            params,
            state: SurfaceMotion::Airborne,
            depth_lane: 0,
        }
    }
}

/// The one movement-policy component carried by a movable body.
///
/// Absence is not a policy. Assemblers should install an explicit model and
/// runtime swaps should call [`MotionModel::apply_spec`] instead of replacing
/// unrelated body clusters.
#[derive(Component, Clone, Debug)]
pub enum MotionModel {
    AxisSwept(AxisSweptMotion),
    SurfaceMomentum(SurfaceMomentumMotion),
}

impl Default for MotionModel {
    fn default() -> Self {
        Self::AxisSwept(AxisSweptMotion::default())
    }
}

impl MotionModel {
    pub fn axis_swept(params: AxisSweptParams) -> Self {
        Self::AxisSwept(AxisSweptMotion::new(params))
    }

    pub fn surface_momentum(params: MomentumParams) -> Self {
        Self::SurfaceMomentum(SurfaceMomentumMotion::new(params))
    }

    pub const fn kind(&self) -> MotionModelKind {
        match self {
            Self::AxisSwept(_) => MotionModelKind::AxisSwept,
            Self::SurfaceMomentum(_) => MotionModelKind::SurfaceMomentum,
        }
    }

    pub fn spec(&self) -> MotionModelSpec {
        match self {
            Self::AxisSwept(motion) => MotionModelSpec::AxisSwept(motion.params),
            Self::SurfaceMomentum(motion) => MotionModelSpec::SurfaceMomentum(motion.params),
        }
    }

    /// Apply a policy request while preserving all state that is meaningful to
    /// the selected solver.
    ///
    /// Same-variant updates replace parameters without erasing runtime state.
    /// Cross-variant updates reset only destination-private state. Shared body
    /// position and velocity are deliberately not arguments and therefore
    /// cannot be accidentally reset here.
    pub fn apply_spec(&mut self, spec: MotionModelSpec) {
        match (self, spec) {
            (Self::AxisSwept(current), MotionModelSpec::AxisSwept(params)) => {
                current.params = params;
            }
            (Self::SurfaceMomentum(current), MotionModelSpec::SurfaceMomentum(params)) => {
                current.params = params;
            }
            (slot, MotionModelSpec::AxisSwept(params)) => {
                *slot = Self::axis_swept(params);
            }
            (slot, MotionModelSpec::SurfaceMomentum(params)) => {
                *slot = Self::surface_momentum(params);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::surface_momentum::{SurfaceMotion, SurfaceRef};

    #[test]
    fn same_surface_model_parameter_refresh_preserves_runtime_state() {
        let riding = SurfaceMotion::Riding {
            on: SurfaceRef::Chain(3),
            s: 42.0,
            v_t: -700.0,
        };
        let mut model = MotionModel::surface_momentum(MomentumParams::default());
        let MotionModel::SurfaceMomentum(motion) = &mut model else {
            unreachable!();
        };
        motion.state = riding;
        motion.depth_lane = -1;

        let mut updated = MomentumParams::default();
        updated.top_speed += 100.0;
        model.apply_spec(MotionModelSpec::SurfaceMomentum(updated));

        let MotionModel::SurfaceMomentum(motion) = model else {
            panic!("same-variant refresh changed movement policy");
        };
        assert_eq!(motion.state, riding);
        assert_eq!(motion.depth_lane, -1);
        assert_eq!(motion.params, updated);
    }

    #[test]
    fn cross_model_transition_resets_only_destination_private_state() {
        let mut model = MotionModel::surface_momentum(MomentumParams::default());
        model.apply_spec(MotionModelSpec::AxisSwept(AxisSweptParams::default()));
        assert_eq!(model.kind(), MotionModelKind::AxisSwept);

        model.apply_spec(MotionModelSpec::SurfaceMomentum(MomentumParams::default()));
        let MotionModel::SurfaceMomentum(motion) = model else {
            panic!("surface destination was not installed");
        };
        assert_eq!(motion.state, SurfaceMotion::Airborne);
        assert_eq!(motion.depth_lane, 0);
    }
}
