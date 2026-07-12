//! Movement-model identity, parameters, and persistent solver state.
//!
//! A body always owns one explicit [`MotionModel`].  The variant is the
//! swappable physics policy; each variant owns the authored parameters and
//! private runtime state its solver needs.  World-space body state
//! (`BodyKinematics` and the shared clusters) remains outside the model so
//! changing policies preserves position, velocity, facing, abilities, and body
//! mode by construction.
//!
//! ## Transition semantics
//!
//! [`switch_motion_model`] is THE runtime policy-transition operation:
//!
//! - same-variant → refresh authored parameters, preserve ALL private runtime
//!   state (surface identity, arc position, tangential speed, depth lane,
//!   crawler attachment — whatever the active solver owns);
//! - cross-variant → preserve every shared body fact and initialize ONLY the
//!   destination's private state. The destination acquires support through its
//!   own same-tick contact rules — never by nearest-surface search, teleport,
//!   or a stale flag from the previous policy.
//!
//! A frame change is not a model change (nothing here reads the environment),
//! and a model change is not a frame change (nothing here writes it). The
//! operation is independent of who controls the body.

use bevy_ecs::component::Component;

use super::adhesive_crawler::{AdhesiveCrawlerMotion, CrawlerParams};
use super::surface_momentum::{MomentumParams, SurfaceMotion};
use super::AxisSweptParams;
use crate::body_clusters::BodyClustersMut;

/// Stable identity for diagnostics, authoring, and transition tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionModelKind {
    AxisSwept,
    SurfaceMomentum,
    AdhesiveCrawler,
}

/// Authored/runtime request for a movement policy.
///
/// This is intentionally state-free. Apply it with [`switch_motion_model`],
/// which preserves private state when the variant is unchanged and initializes
/// only the destination solver's private state when the variant changes.
#[derive(Clone, Copy, Debug)]
pub enum MotionModelSpec {
    AxisSwept(AxisSweptParams),
    SurfaceMomentum(MomentumParams),
    AdhesiveCrawler(CrawlerParams),
}

/// Axis-swept model-owned data.
///
/// The axis solver's persistent runtime state (coyote/jump buffers, wall and
/// ledge engagement, dash/blink maneuver timers, axis contact caches) lives on
/// the per-body movement clusters, which double as the snapshot ledger's
/// registered components. Ownership is the policy's regardless of physical
/// placement: [`switch_motion_model`] initializes that state on cross-variant
/// entry, and no other policy reads it.
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
    /// A fresh surface-momentum body begins `Airborne` on the unchanged pose
    /// and velocity; it may attach only through its normal same-tick
    /// contact/sweep rules.
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
/// Absence is not a policy: every integrated body carries a variant from
/// spawn, and no query may interpret a missing component as axis-swept.
/// Runtime swaps go through [`switch_motion_model`] so destination-private
/// state is initialized without touching unrelated body state.
#[derive(Component, Clone, Debug)]
pub enum MotionModel {
    AxisSwept(AxisSweptMotion),
    SurfaceMomentum(SurfaceMomentumMotion),
    AdhesiveCrawler(AdhesiveCrawlerMotion),
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

    pub fn adhesive_crawler(params: CrawlerParams) -> Self {
        Self::AdhesiveCrawler(AdhesiveCrawlerMotion::new(params))
    }

    pub const fn kind(&self) -> MotionModelKind {
        match self {
            Self::AxisSwept(_) => MotionModelKind::AxisSwept,
            Self::SurfaceMomentum(_) => MotionModelKind::SurfaceMomentum,
            Self::AdhesiveCrawler(_) => MotionModelKind::AdhesiveCrawler,
        }
    }

    pub fn spec(&self) -> MotionModelSpec {
        match self {
            Self::AxisSwept(motion) => MotionModelSpec::AxisSwept(motion.params),
            Self::SurfaceMomentum(motion) => MotionModelSpec::SurfaceMomentum(motion.params),
            Self::AdhesiveCrawler(motion) => MotionModelSpec::AdhesiveCrawler(motion.params),
        }
    }

    /// Model-internal half of a policy request: refresh parameters in place on
    /// a same-variant spec, install a fresh destination on a cross-variant one.
    ///
    /// Runtime code paths use [`switch_motion_model`], which also initializes
    /// the destination's cluster-resident private state; this method alone is
    /// correct only at spawn/insert time, before a body has accumulated any.
    pub fn apply_spec(&mut self, spec: MotionModelSpec) {
        match (self, spec) {
            (Self::AxisSwept(current), MotionModelSpec::AxisSwept(params)) => {
                current.params = params;
            }
            (Self::SurfaceMomentum(current), MotionModelSpec::SurfaceMomentum(params)) => {
                current.params = params;
            }
            (Self::AdhesiveCrawler(current), MotionModelSpec::AdhesiveCrawler(params)) => {
                current.params = params;
            }
            (slot, MotionModelSpec::AxisSwept(params)) => {
                *slot = Self::axis_swept(params);
            }
            (slot, MotionModelSpec::SurfaceMomentum(params)) => {
                *slot = Self::surface_momentum(params);
            }
            (slot, MotionModelSpec::AdhesiveCrawler(params)) => {
                *slot = Self::adhesive_crawler(params);
            }
        }
    }
}

/// THE runtime policy-transition operation (see the module doc for the
/// semantics). Shared body state — position, velocity, facing, size, body
/// mode, abilities, resources, health, identity, controller ownership — is
/// deliberately not an argument of the destination initializer and therefore
/// cannot be reset here.
pub fn switch_motion_model(
    model: &mut MotionModel,
    spec: MotionModelSpec,
    clusters: &mut BodyClustersMut<'_>,
) {
    let crossed = model.kind() != spec_kind(spec);
    model.apply_spec(spec);
    if !crossed {
        return;
    }
    if let MotionModel::AxisSwept(_) = model {
        initialize_axis_private_state(clusters);
    }
    // SurfaceMomentum / AdhesiveCrawler destinations carry their fresh private
    // state inside the new variant value (Airborne / detached); they own no
    // cluster-resident state.
}

const fn spec_kind(spec: MotionModelSpec) -> MotionModelKind {
    match spec {
        MotionModelSpec::AxisSwept(_) => MotionModelKind::AxisSwept,
        MotionModelSpec::SurfaceMomentum(_) => MotionModelKind::SurfaceMomentum,
        MotionModelSpec::AdhesiveCrawler(_) => MotionModelKind::AdhesiveCrawler,
    }
}

/// Initialize the axis-swept policy's cluster-resident private state on
/// cross-variant entry: empty support/contact caches and no in-flight maneuver.
/// The same tick's ordinary collision phase computes current contacts from the
/// unchanged pose. Resource COUNTS (dash charges, air jumps) and recharge
/// cooldowns are body resources, not maneuver state, and are preserved — as is
/// `fly_enabled`, an ability mode fact.
fn initialize_axis_private_state(clusters: &mut BodyClustersMut<'_>) {
    clusters.ground.coyote_timer = 0.0;
    clusters.ground.drop_through_timer = 0.0;
    clusters.ground.rebound_cooldown = 0.0;
    *clusters.action_buffer = Default::default();
    *clusters.wall = Default::default();
    clusters.dash.timer = 0.0;
    clusters.blink.hold_active = false;
    clusters.blink.hold_timer = 0.0;
    clusters.blink.aiming = false;
    clusters.blink.grace_timer = 0.0;
    clusters.dodge.roll_timer = 0.0;
    clusters.ledge.grab = None;
    clusters.flight.gliding = false;
    clusters.flight.fast_falling = false;
    clusters.flight.flight_phase = 0.0;
    clusters.flight.carried_run = 0.0;
}

#[cfg(test)]
mod tests {
    use super::super::surface_momentum::{SurfaceMotion, SurfaceRef};
    use super::*;

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

    #[test]
    fn a_fresh_crawler_begins_detached_and_a_crawler_refresh_keeps_attachment() {
        use super::super::adhesive_crawler::CrawlerState;

        let mut model = MotionModel::adhesive_crawler(CrawlerParams::default());
        let MotionModel::AdhesiveCrawler(motion) = &model else {
            panic!("crawler variant was not installed");
        };
        assert!(!motion.state.is_attached(), "fresh crawler begins detached");

        let MotionModel::AdhesiveCrawler(motion) = &mut model else {
            unreachable!();
        };
        motion.state = CrawlerState::attached(crate::Vec2::new(-1.0, 0.0));
        let mut updated = CrawlerParams::default();
        updated.crawl_speed += 25.0;
        model.apply_spec(MotionModelSpec::AdhesiveCrawler(updated));
        let MotionModel::AdhesiveCrawler(motion) = &model else {
            panic!("same-variant refresh changed movement policy");
        };
        assert_eq!(
            motion.state.attachment(),
            Some(crate::Vec2::new(-1.0, 0.0)),
            "same-variant refresh preserves the clung surface"
        );
        assert_eq!(motion.params, updated);
    }
}
