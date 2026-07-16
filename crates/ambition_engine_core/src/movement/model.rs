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
use super::tuning::BLINK_DISTANCE;
use super::AxisSweptParams;
use crate::body_clusters::{BodyLedgeState, LEDGE_KNOCK_OFF_COOLDOWN};
use crate::Vec2;

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

/// The axis-swept policy's PRIVATE persistent maneuver state. Lives INSIDE the
/// model variant (ADR 0024): no other policy can read it, leaving axis movement
/// cannot leak stale maneuver facts, and a same-variant parameter refresh
/// preserves it by construction. The shared clusters keep only the CONTACT
/// facts the collision doctrine writes (`on_ground`, `on_wall`,
/// `wall_normal_x`) and the preserved body RESOURCES (charges, cooldowns).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisManeuverState {
    pub coyote_timer: f32,
    pub drop_through_timer: f32,
    pub rebound_cooldown: f32,
    pub wall_clinging: bool,
    pub wall_climbing: bool,
    pub pre_wall_vel: Vec2,
    pub pre_wall_vel_age: f32,
    /// Buffered MOVEMENT actions (jump/dash/blink press windows). Combat
    /// buffers (attack/pogo/projectile) stay on the shared BodyActionBuffer.
    pub buffer_jump: f32,
    pub buffer_dash: f32,
    pub buffer_blink: f32,
    pub dash_timer: f32,
    pub blink_hold_active: bool,
    pub blink_hold_timer: f32,
    pub blink_aiming: bool,
    pub blink_aim_offset: Vec2,
    pub blink_grace_timer: f32,
    pub dodge_roll_timer: f32,
    pub ledge_grab: Option<crate::LedgeGrabState>,
    pub gliding: bool,
    pub fast_falling: bool,
    pub flight_phase: f32,
}

impl Default for AxisManeuverState {
    /// No in-flight maneuver: everything zero/false/None except the blink aim
    /// offset, which rests at "one blink forward" (matching the historical
    /// blink-state default).
    fn default() -> Self {
        Self {
            coyote_timer: 0.0,
            drop_through_timer: 0.0,
            rebound_cooldown: 0.0,
            wall_clinging: false,
            wall_climbing: false,
            pre_wall_vel: Vec2::ZERO,
            pre_wall_vel_age: 0.0,
            buffer_jump: 0.0,
            buffer_dash: 0.0,
            buffer_blink: 0.0,
            dash_timer: 0.0,
            blink_hold_active: false,
            blink_hold_timer: 0.0,
            blink_aiming: false,
            blink_aim_offset: Vec2::new(BLINK_DISTANCE, 0.0),
            blink_grace_timer: 0.0,
            dodge_roll_timer: 0.0,
            ledge_grab: None,
            gliding: false,
            fast_falling: false,
            flight_phase: 0.0,
        }
    }
}

/// Axis-swept model-owned parameters and persistent maneuver state.
///
/// Cross-variant entry installs a fresh value (default state); a same-variant
/// parameter refresh touches only `params`, so maneuver state is preserved by
/// construction — no external initializer exists or is needed.
#[derive(Clone, Copy, Debug)]
pub struct AxisSweptMotion {
    pub params: AxisSweptParams,
    pub state: AxisManeuverState,
}

impl AxisSweptMotion {
    pub fn new(params: AxisSweptParams) -> Self {
        Self {
            params,
            state: AxisManeuverState::default(),
        }
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
    /// The junction half-edge most recently taken (see
    /// [`crate::RouteDeparture`]): keeps a held steering bias from re-opening
    /// the lap it just closed at a loop mouth.
    pub route_memory: Option<crate::RouteDeparture>,
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
            route_memory: None,
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
    /// Every variant carries its private state inside the variant value, so
    /// this IS the complete transition; [`switch_motion_model`] is the named
    /// runtime seam over it.
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
/// cannot be reset here. Every destination's fresh private state lives inside
/// the new variant value (default maneuver state / Airborne / detached), so
/// no cluster is touched: resource COUNTS (dash charges, air jumps), recharge
/// cooldowns, and ability mode facts (`fly_enabled`) survive by construction.
pub fn switch_motion_model(model: &mut MotionModel, spec: MotionModelSpec) {
    model.apply_spec(spec);
}

/// Drop any active ledge grab because the body was hit, arming a brief
/// re-grab lockout on the shared ledge cluster. Returns true if it was
/// hanging (so the caller can react — e.g. let the knockback carry it).
/// The typed combat→movement op over the axis policy's private hang state;
/// non-axis policies have no ledge grab and return false.
pub fn knock_off_ledge(model: &mut MotionModel, ledge: &mut BodyLedgeState) -> bool {
    let MotionModel::AxisSwept(axis) = model else {
        return false;
    };
    if axis.state.ledge_grab.take().is_some() {
        ledge.release_cooldown = ledge.release_cooldown.max(LEDGE_KNOCK_OFF_COOLDOWN);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod ledge_knock_off_tests {
    use super::*;
    use crate::ledge_grab::{LedgeContact, LedgeGrabState};

    fn hanging() -> LedgeGrabState {
        LedgeGrabState::hanging(LedgeContact {
            wall_normal_x: 1.0,
            anchor: Vec2::ZERO,
            climb_target: Vec2::ZERO,
        })
    }

    #[test]
    fn getting_hit_knocks_the_player_off_a_ledge_grab() {
        let mut model = MotionModel::axis_swept(AxisSweptParams::default());
        let MotionModel::AxisSwept(axis) = &mut model else {
            unreachable!();
        };
        axis.state.ledge_grab = Some(hanging());
        let mut ledge = BodyLedgeState {
            release_cooldown: 0.0,
        };
        assert!(
            knock_off_ledge(&mut model, &mut ledge),
            "was hanging → reports knocked off"
        );
        let MotionModel::AxisSwept(axis) = &model else {
            unreachable!();
        };
        assert!(
            axis.state.ledge_grab.is_none(),
            "ledge grab cleared so the player falls"
        );
        assert!(
            ledge.release_cooldown >= LEDGE_KNOCK_OFF_COOLDOWN,
            "re-grab lockout armed"
        );
    }

    #[test]
    fn knock_off_is_a_noop_when_not_grabbing() {
        let mut model = MotionModel::axis_swept(AxisSweptParams::default());
        let mut ledge = BodyLedgeState::default();
        assert!(!knock_off_ledge(&mut model, &mut ledge));
        assert_eq!(
            ledge.release_cooldown, 0.0,
            "no lockout when nothing to drop"
        );

        // A non-axis policy has no ledge grab to drop.
        let mut momentum = MotionModel::surface_momentum(MomentumParams::default());
        assert!(!knock_off_ledge(&mut momentum, &mut ledge));
        assert_eq!(ledge.release_cooldown, 0.0);
    }
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
            Some(crate::movement::CrawlAttachment::Block {
                normal: crate::Vec2::new(-1.0, 0.0),
            }),
            "same-variant refresh preserves the clung surface"
        );
        assert_eq!(motion.params, updated);
    }
}
