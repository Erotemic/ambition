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
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotionModelSpec {
    AxisSwept(AxisSweptParams),
    SurfaceMomentum(MomentumParams),
    AdhesiveCrawler(CrawlerParams),
}

/// Persistent state for a phased-gravity jump arc.
///
/// The selected launch band and release latch survive frame changes and rollback,
/// but no world-space direction is cached: every tick reinterprets the arc in
/// the environment's current [`crate::MotionFrame`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhasedJumpState {
    pub active: bool,
    pub launch_band: u8,
    pub hold_cancelled: bool,
}

impl PhasedJumpState {
    pub fn begin(&mut self, launch_band: u8) {
        self.active = true;
        self.launch_band = launch_band.min(3);
        self.hold_cancelled = false;
    }

    pub fn cancel_hold(&mut self) {
        if self.active {
            self.hold_cancelled = true;
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
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
    /// How long this body has been OFF a ledge, in seconds, clamped at
    /// [`crate::ledge_grab::LEDGE_INVULN_FULL_AIRTIME`].
    ///
    /// what a fresh grab's intangibility is bought with — see
    /// [`crate::ledge_grab::ledge_grab_invuln_earned`]. it starts FULL, not
    /// zero: a body that has never touched a ledge has been off one forever, and
    /// a zero would hand every first grab in a match the minimum window.
    pub time_off_ledge: f32,
    /// Buffered MOVEMENT actions (jump/burst/blink press windows). The combat
    /// verbs (attack/pogo/grab/special) buffer on the shared
    /// `BodyActionBuffer`; both halves are live, and they decay on the same
    /// authored window.
    pub buffer_jump: f32,
    /// The BURST press window — one buffer for the one button that dodge and
    /// dash share (see [`crate::movement::abilities::BurstManeuver`]). it was
    /// `buffer_dash`, and the name was load-bearing in the wrong direction:
    /// `apply_intent` filled it only for `abilities.dash`, so a body authored
    /// with the dodge and not the traversal burst could never spend an evade.
    pub buffer_burst: f32,
    pub buffer_blink: f32,
    /// Time left in a committed jump-squat (see
    /// [`crate::movement::tuning::AxisLocomotion::jump_squat_time`]). Non-zero
    /// means the press is ALREADY SPENT and the leap is owed — it is the
    /// opposite of `buffer_jump`, which means a press is waiting to be spent.
    pub jump_squat_timer: f32,
    pub dash_timer: f32,
    pub blink_hold_active: bool,
    pub blink_hold_timer: f32,
    pub blink_aiming: bool,
    pub blink_aim_offset: Vec2,
    pub blink_grace_timer: f32,
    pub dodge_roll_timer: f32,
    /// The window on [`Self::dodge_roll_timer`] is a SPOT DODGE, not a roll.
    ///
    /// one timer, two verbs, and the flag is what tells them apart — the
    /// i-frames are the same term the damage rule reads either way, so splitting
    /// the TIMER would have made `evading()` a two-place question for no gain.
    /// What differs is only what it is DRAWN as, and that is a presentation
    /// fact.
    pub spot_dodging: bool,
    /// The AIR dodge's own clock — seconds of the committed aerial evade.
    ///
    /// not `dodge_roll_timer`, and the separation is the design. Both
    /// grant i-frames, and reusing the roll's timer would have been the cheap
    /// road; they are different maneuvers with different commitments, and a
    /// body cannot animate, debug or tune them apart if the simulation cannot
    /// tell them apart either. A ground roll travels along the floor and ends
    /// standing; an air dodge spends the body's one aerial evade and ends in
    /// endlag with gravity waiting.
    pub air_dodge_timer: f32,
    /// Endlag after an air dodge: control is back, the evade is not.
    pub air_dodge_endlag_timer: f32,
    /// Tumble, the helpless part: launched with no control at all, scaled by
    /// how hard the launch was. See [`super::knockdown`].
    pub tumble_timer: f32,
    /// Tumble, the part that outlives the helplessness: this body is still
    /// falling out of a launch, so its next landing is a knockdown unless it is
    /// teched.
    ///
    /// two fields because they are two facts. Ultimate's tumble works exactly
    /// this way — control comes back before the tumble does, and you either act
    /// out of it or you hit the floor — and a single timer models neither: too
    /// short and a launch that peaks high lands on its feet as if nothing
    /// happened, too long and the body is helpless for the whole arc.
    pub tumble_until_landing: bool,
    /// A launch began a tumble and the frame has not reported it yet.
    ///
    /// A resimulation that replays the launch replays this with it.
    pub tumble_unannounced: bool,
    /// A live tech press, waiting for a surface. Expires into a lockout.
    pub tech_press_timer: f32,
    /// No teching until this runs out — the cost of a mistimed one.
    pub tech_lockout_timer: f32,
    /// Knockdown: prone on the floor, with getup options.
    pub knockdown_timer: f32,
    /// Invulnerability from a tech or a getup.
    pub getup_invuln_timer: f32,
    pub ledge_grab: Option<crate::LedgeGrabState>,
    pub gliding: bool,
    pub fast_falling: bool,
    /// This body is in a RUN — grounded, steering the way it is travelling,
    /// at or above [`crate::MovementTuning::run_commit_frac`] of its own top
    /// speed. Written once at the end of the integration step and projected as
    /// `BodyMotionFacts::running`.
    ///
    /// not the traversal dash ([`Self::dash_timer`]), which is a discrete
    /// charge-gated burst a platform-fighter kit deliberately switches OFF. The
    /// running attack reads THIS.
    ///
    /// derived every tick and still SERIALIZED, for the same reason
    /// [`Self::gliding`] and [`Self::fast_falling`] beside it are: a restore
    /// that lands mid-run must not present a standing body for the tick before
    /// the next integration rewrites it.
    pub running: bool,
    pub flight_phase: f32,
    pub phased_jump: PhasedJumpState,
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
            time_off_ledge: crate::ledge_grab::LEDGE_INVULN_FULL_AIRTIME,
            buffer_jump: 0.0,
            jump_squat_timer: 0.0,
            buffer_burst: 0.0,
            buffer_blink: 0.0,
            dash_timer: 0.0,
            blink_hold_active: false,
            blink_hold_timer: 0.0,
            blink_aiming: false,
            blink_aim_offset: Vec2::new(BLINK_DISTANCE, 0.0),
            blink_grace_timer: 0.0,
            dodge_roll_timer: 0.0,
            spot_dodging: false,
            air_dodge_timer: 0.0,
            air_dodge_endlag_timer: 0.0,
            tumble_timer: 0.0,
            tumble_until_landing: false,
            tumble_unannounced: false,
            tech_press_timer: 0.0,
            tech_lockout_timer: 0.0,
            knockdown_timer: 0.0,
            getup_invuln_timer: 0.0,
            ledge_grab: None,
            gliding: false,
            fast_falling: false,
            running: false,
            flight_phase: 0.0,
            phased_jump: PhasedJumpState::default(),
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
    /// Foreign-lane track the current flight launched coincident with (see
    /// [`crate::OcclusionSpan`]): non-collidable until the body separates.
    /// Empty while riding.
    pub occlusions: crate::DepthOcclusions,
}

impl SurfaceMomentumMotion {
    /// A fresh surface-momentum body begins `Airborne` on the unchanged pose
    /// and velocity; it may attach only through its normal same-tick
    /// contact/sweep rules.
    /// Set the tangential speed of a riding body. Returns `false` when the
    /// body is airborne and there is no tangent to set.
    ///
    /// the SIGN CONVENTION lives here now, not in each caller. The kernel
    /// integrates `v_t += run * accel * dt` with `run = locomotion.x`, so `v_t`
    /// and facing share a sign — a fact Sanic's ball dash had written out in
    /// three separate comments because it was reaching into
    /// [`SurfaceMotion::Riding`] itself.
    ///
    /// returning `false` rather than doing nothing quietly is the point. A
    /// caller that must also handle the airborne case (the ball dash's launch
    /// writes `BodyKinematics::vel` instead) is made to say so; one that has no
    /// airborne answer, like a brake, may ignore it.
    pub fn set_tangential_speed(&mut self, speed: f32) -> bool {
        match &mut self.state {
            SurfaceMotion::Riding { v_t, .. } => {
                *v_t = speed;
                true
            }
            SurfaceMotion::Airborne => false,
        }
    }

    /// Scale the tangential speed of a riding body — a brake or a boost.
    /// Returns `false` when airborne, for the reason above.
    ///
    /// Separate from [`Self::set_tangential_speed`] because scaling preserves
    /// direction and setting does not: a brake that went through the setter would
    /// have to read the current sign back out, which is the reach-in this exists
    /// to remove.
    pub fn scale_tangential_speed(&mut self, factor: f32) -> bool {
        match &mut self.state {
            SurfaceMotion::Riding { v_t, .. } => {
                *v_t *= factor;
                true
            }
            SurfaceMotion::Airborne => false,
        }
    }

    /// The tangential speed of a riding body, or `None` airborne.
    pub fn tangential_speed(&self) -> Option<f32> {
        match &self.state {
            SurfaceMotion::Riding { v_t, .. } => Some(*v_t),
            SurfaceMotion::Airborne => None,
        }
    }

    pub fn new(params: MomentumParams) -> Self {
        Self {
            params,
            state: SurfaceMotion::Airborne,
            depth_lane: 0,
            route_memory: None,
            occlusions: crate::DepthOcclusions::default(),
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

    /// Seconds left in a committed jump-squat, or `0.0` for a policy that has
    /// no such thing. The projection an OBSERVER wants: "is this body crouching
    /// before a leap" is asked of any body, and only one variant can answer it.
    pub fn jump_squat_remaining(&self) -> f32 {
        match self {
            Self::AxisSwept(axis) => axis.state.jump_squat_timer,
            _ => 0.0,
        }
    }

    /// Has this body caught hold of a ledge? `false` for a policy that has
    /// no such thing. The sibling of [`Self::jump_squat_remaining`], and it
    /// exists for the same reason: a hang holds a body against its frame's pull
    /// WITHOUT producing a contact, so `SupportFact` cannot see it, and
    /// "is this body still falling" is asked of any body while only one variant
    /// can answer it. Asking by matching the variant at the call site is how the
    /// question ends up answered two different ways.
    pub fn holds_a_ledge(&self) -> bool {
        match self {
            Self::AxisSwept(axis) => axis.state.ledge_grab.is_some(),
            Self::SurfaceMomentum(_) | Self::AdhesiveCrawler(_) => false,
        }
    }

    /// This body's shield resource, or [`ShieldTuning::OFF`] for a policy
    /// that has no guard. The sibling of [`Self::jump_squat_remaining`]: the
    /// damage resolver has to spend the guard it just granted, and matching the
    /// variant at that call site is how one rule ends up spelled twice.
    pub fn shield_tuning(&self) -> crate::ShieldTuning {
        match self {
            Self::AxisSwept(axis) => axis.params.abilities.shield,
            Self::SurfaceMomentum(_) | Self::AdhesiveCrawler(_) => crate::ShieldTuning::OFF,
        }
    }

    /// This body's footstool rules, or [`FootstoolTuning::OFF`] for a policy
    /// that has none. The sibling of [`Self::shield_tuning`].
    pub fn footstool_tuning(&self) -> crate::FootstoolTuning {
        match self {
            Self::AxisSwept(axis) => axis.params.abilities.footstool,
            Self::SurfaceMomentum(_) | Self::AdhesiveCrawler(_) => crate::FootstoolTuning::OFF,
        }
    }

    /// What a full-deflection direct command means for this body, in px/s.
    ///
    /// The projection a CONTROLLER wants, and the sibling of
    /// [`Self::jump_squat_remaining`]: `ActorControlFrame::velocity_target` is an
    /// ABSOLUTE world-space velocity, so anything turning a normalized stick into
    /// one needs the body's own top speed. Every movement policy can name that
    /// number in its own words, which is exactly why the question belongs here —
    /// asking the body's *actor configuration* instead makes a generic controlled
    /// seam depend on one game's authoring types.
    pub fn commanded_top_speed(&self) -> f32 {
        match self {
            Self::AxisSwept(axis) => axis.params.locomotion.max_run_speed,
            Self::SurfaceMomentum(momentum) => momentum.params.top_speed,
            Self::AdhesiveCrawler(crawler) => crawler.params.crawl_speed,
        }
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
                // A same-variant refresh preserves private maneuver state — that
                // is the whole point of the in-place path (a live tuning edit
                // must not restart the arc the body is mid-way through). But the
                // JUMP LAW is what gives `phased_jump` its meaning: swapping the
                // law re-interprets, rather than re-tunes, the arc. Drop the arc
                // when the law's variant changes so a dormant phased arc cannot
                // survive a trip through `VelocityCut` and re-arm later.
                if std::mem::discriminant(&current.params.locomotion.jump_law)
                    != std::mem::discriminant(&params.locomotion.jump_law)
                {
                    current.state.phased_jump.clear();
                }
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

/// Returns the seconds of hard control lock the caller must record on the
/// body's combat state; `0.0` when the tumble this started already owns
/// control, because the floor game neutralizes input for as long as it runs and
/// a second lock beside it would only disagree with it.
///
/// the split is the mechanic. Ultimate footstools two different things:
/// a GROUNDED victim has nowhere to be shoved and takes a brief flinch — which
/// is what makes a grounded footstool a combo starter rather than a punish —
/// while an AIRBORNE one is driven down into a tumble it cannot cancel early.
/// Both are techable on landing, and that comes free: the tech window is the
/// floor game's, and this hands the body to it.
pub fn footstool_victim(
    model: &mut MotionModel,
    kinematics: &mut crate::BodyKinematics,
    grounded: bool,
    gravity_dir: Vec2,
    rules: crate::FootstoolTuning,
) -> f32 {
    if grounded {
        return rules.flinch_time;
    }
    // SET, not add: a body arriving at terminal velocity and one barely
    // falling must be driven down at the same speed, or being stood on costs
    // more the further your attacker fell to reach you.
    let along = kinematics.vel.dot(gravity_dir);
    kinematics.vel -= gravity_dir * (along - rules.press_speed);
    let MotionModel::AxisSwept(axis) = model else {
        return rules.flinch_time;
    };
    if super::knockdown::tumble_from_footstool(&mut axis.state, axis.params, rules.air_tumble_time)
    {
        0.0
    } else {
        // A body that does not tumble still owes the shove a beat.
        rules.flinch_time
    }
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
    fn same_axis_model_parameter_refresh_preserves_phased_jump_state() {
        let mut model = MotionModel::axis_swept(AxisSweptParams::default());
        let MotionModel::AxisSwept(motion) = &mut model else {
            unreachable!();
        };
        motion.state.phased_jump.begin(2);
        motion.state.phased_jump.cancel_hold();
        motion.state.coyote_timer = 0.075;

        let mut updated = AxisSweptParams::default();
        updated.locomotion.max_run_speed += 25.0;
        switch_motion_model(&mut model, MotionModelSpec::AxisSwept(updated));

        let MotionModel::AxisSwept(motion) = model else {
            panic!("same-variant refresh changed movement policy");
        };
        assert_eq!(motion.params, updated);
        assert_eq!(
            motion.state.phased_jump,
            PhasedJumpState {
                active: true,
                launch_band: 2,
                hold_cancelled: true,
            },
            "a live tuning refresh must not restart or reinterpret the active arc"
        );
        assert_eq!(motion.state.coyote_timer, 0.075);
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

#[cfg(test)]
mod tangential_op_tests {
    use super::super::surface_momentum::{SurfaceMotion, SurfaceRef};
    use super::*;

    fn riding(v_t: f32) -> SurfaceMomentumMotion {
        let mut m = SurfaceMomentumMotion::new(MomentumParams::default());
        m.state = SurfaceMotion::Riding {
            on: SurfaceRef::Chain(0),
            s: 0.0,
            v_t,
        };
        m
    }

    /// Setting replaces, scaling preserves direction — the reason these are
    /// two operations rather than one. A brake routed through the setter would
    /// have to read the sign back out, which is the reach-in they exist to
    /// remove.
    #[test]
    fn set_replaces_and_scale_preserves_direction() {
        let mut m = riding(-700.0);
        assert!(m.scale_tangential_speed(0.5));
        assert_eq!(
            m.tangential_speed(),
            Some(-350.0),
            "a brake reversed the ride's direction"
        );
        assert!(m.set_tangential_speed(120.0));
        assert_eq!(m.tangential_speed(), Some(120.0));
    }

    /// the property the typed op ADDS: an airborne body says so.
    ///
    /// The launch has an answer (write the kinematic velocity along the local side axis); the
    /// brake does not, and silently doing nothing is correct for it. Both are now a visible
    /// `bool` rather than a branch each site had to remember to write.
    #[test]
    fn an_airborne_body_refuses_a_tangential_op_rather_than_absorbing_it() {
        let mut m = SurfaceMomentumMotion::new(MomentumParams::default());
        assert!(
            matches!(m.state, SurfaceMotion::Airborne),
            "a fresh surface-momentum body begins airborne"
        );
        assert!(
            !m.set_tangential_speed(500.0),
            "an airborne body reported that it set a tangential speed it has no \
             tangent for"
        );
        assert!(!m.scale_tangential_speed(0.5));
        assert_eq!(
            m.tangential_speed(),
            None,
            "an airborne body reported a tangential speed"
        );
        assert!(
            matches!(m.state, SurfaceMotion::Airborne),
            "a refused op still changed the motion state"
        );
    }
}
