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

/// A BODY ON A WIRE — the flyline's persistent state, and the one maneuver whose
/// position is not a velocity.
///
/// ⭐⭐ ITS POSITION IS `(anchor, length, angle)`, WHICH IS WHY IT IS A STATE AND
/// NOT AN IMPULSE. A pendulum on a winch is not a body that was pushed: the wire
/// decides where she is, the winch decides how far up, and the stick decides
/// only which way she is swinging. Jon, 2026-08-29: *"she gets lifted up by the
/// wire… while she is being lifted by the wire her motion controls should let
/// her swing like a pendulum so she has a bit of horizontal recovery with it
/// too."*
///
/// ⛔ THE ANGLE IS BODY-LOCAL, measured from the frame's own DOWN axis toward
/// local `+x`, so a gravity-flipped stage swings the same way round without this
/// struct knowing the stage exists. `anchor` is the one world-space fact,
/// because a wire hangs from a fixed point in the room rather than from the
/// body.
///
/// ⛔⛔ THE AUTHORED KNOBS RIDE HERE TOO — `winch_speed`, `max_angle`,
/// `swing_accel`, `release_rise`. They are parameters, not state, and they are
/// still in the snapshot: the kernel cannot see the MOVE that authored them, so
/// a restore that dropped them would resume the lift with a different wire. The
/// same reason [`crate::LedgeGrabState`] carries its anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WireState {
    /// Where the wire comes down FROM, in world space. Fixed for the whole lift.
    pub anchor: Vec2,
    /// Rope length in world px, from `anchor` to the body's centre. The winch
    /// shortens this, and shortening it is the whole of the rise.
    pub length: f32,
    /// Radians from the frame's down axis, positive toward local `+x`.
    pub angle: f32,
    /// Radians per second.
    pub ang_vel: f32,
    /// How fast the winch reels in AT THE CATCH, in px/s.
    ///
    /// ⭐⭐ THE WINCH DECELERATES, and that is why this is the *initial* rate
    /// rather than the rate. It ramps linearly to [`Self::release_rise`] across
    /// the lift, so the speed the rope is pulling her at when it lets go is
    /// exactly the speed she leaves with — the release adds no step at all.
    ///
    /// ⛔⛔ IT WAS A CONSTANT RATE, AND THE DUMP IS WHY IT IS NOT. She rose at
    /// 764 px/s for the whole beat and was cut to 90: a hard stop at the apex,
    /// which is the teleport's own feel arriving through a different mechanic.
    /// A recovery has to hand the body back still moving the way it was moving.
    pub winch_speed: f32,
    /// Seconds of lift left. At zero the wire lets go, and that release is the
    /// ONE writer of the exit velocity.
    pub lift_remaining_s: f32,
    /// The lift's whole duration, kept so the ramp above knows where in it she
    /// is. ⛔ Not derivable from [`Self::lift_remaining_s`] alone, which is the
    /// only reason a second clock is here.
    pub lift_total_s: f32,
    /// How far the swing may reach, in radians. ⛔ A CAP, not a suggestion: an
    /// uncapped pendulum on a shortening rope gains angle every tick
    /// (the skater's spin), and Jon asked for *"a bit"* of horizontal recovery.
    pub max_angle: f32,
    /// Angular acceleration the stick contributes, in rad/s².
    pub swing_accel: f32,
    /// Upward speed the body keeps when the wire lets go, in px/s.
    ///
    /// ⭐ IT IS ALSO THE WINCH'S FINAL RATE — see [`Self::winch_speed`]. The two
    /// are one number seen from either side of the release, which is what makes
    /// the handover seamless instead of a stop.
    ///
    /// ⛔ Part of the ONE release write; nothing else may add to the exit
    /// velocity.
    pub release_rise: f32,
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
    /// How much longer this evade is INTANGIBLE — the staled half of an evade,
    /// and the only half staling is allowed to touch.
    ///
    /// ⭐⭐ SEPARATE FROM [`Self::dodge_roll_timer`] BECAUSE THEY ANSWER DIFFERENT
    /// QUESTIONS. The maneuver clock says "this roll is still happening" and owns
    /// travel, endlag, animation and commitment; this one says "and it is still
    /// safe". One field answering both made a spammed roll SHORTER instead of
    /// unsafe, which is the opposite of what the mechanic is for.
    ///
    /// ⛔ ONE FIELD FOR ALL THREE EVADES: only one can run at a time, and three
    /// copies would drift apart the first time one was tuned.
    pub evade_invuln_timer: f32,
    /// THE ROLL'S OWN PUSH — signed speed along the body's side axis, stamped
    /// when a ground roll starts and shed when it ends.
    ///
    /// ⛔⛔ TRACKED SEPARATELY BECAUSE VELOCITY IS SHARED. A roll that ended by
    /// zeroing `vel` erased the KNOCKBACK of a body struck mid-roll —
    /// `an_up_tilt_launches_much_further_at_a_high_percent` measured a victim
    /// rising 4.5px at 0% and 0.0px at 1427%. A maneuver that ends may only
    /// take back what it itself put in, which means it has to remember it.
    ///
    /// `0.0` for the spot dodge, which covers no distance and has nothing to
    /// shed.
    pub dodge_roll_push: f32,
    /// How many times this trip through the floor game has been PINNED by a
    /// weak hit — the jab lock's budget
    /// ([`crate::TraversalAbilityTuning::jab_lock_limit`]).
    ///
    /// Zeroed when a fresh knockdown starts, so it bounds one knockdown rather
    /// than a whole stock. Without a bound the rule is an infinite.
    pub jab_locks: u8,
    /// THE INITIAL DASH's remaining window
    /// ([`crate::LocomotionTuning::initial_dash_time`]), and the direction it
    /// committed to. `0.0` whenever the phase is not running, which is always
    /// for a body that declares no phase.
    pub initial_dash_timer: f32,
    /// Which way this dash is going: `-1.0`, `0.0` or `+1.0` along the body's
    /// own side axis.
    pub initial_dash_dir: f32,
    /// The steer direction the ground law saw LAST tick, which is the whole of
    /// how a fresh dash is told apart from a held one.
    ///
    /// ⭐ A CHANGE of direction is the entry rule, so dash-dancing and the
    /// foxtrot are the SAME rule rather than two more mechanics: reversing
    /// restarts the phase, and re-tapping a direction restarts it too. A held
    /// direction never re-triggers, which is what lets the phase end and a run
    /// begin.
    pub prev_steer_dir: f32,
    /// THE TURNAROUND's remaining window
    /// ([`crate::LocomotionTuning::turnaround_time`]) — how long until this
    /// body actually faces the way its stick is pointing.
    ///
    /// `0.0` whenever no turnaround is running, which is always for a body
    /// that declares none and always for a body reversing inside its dash.
    pub turnaround_timer: f32,
    /// THIS BODY IS ON THE BRINK — supported where it stands, but not if it
    /// leaned any further the way it faces
    /// ([`crate::LocomotionTuning::teeter_margin`]).
    ///
    /// Derived every tick from the world, and SERIALIZED for the same reason
    /// [`Self::running`] is: a restore that lands on an edge must not present a
    /// planted body for the tick before the next integration rewrites it.
    pub teetering: bool,
    /// HOW MUCH OF THIS BODY'S RISE ITS OWN AIR JUMP PUT IN, and still has —
    /// the amount a double-jump cancel is allowed to take back (read through
    /// [`crate::BodyMotionFacts::air_jump_rise_owned`]).
    ///
    /// ⭐⭐ AN AMOUNT, NOT A PREDICATE, and that is the correction of
    /// 2026-08-25. It was `air_jump_rising: bool`, derived as *"an air jump was
    /// spent at some point AND the current rise is no faster than one"* — which
    /// is a MAGNITUDE standing in for OWNERSHIP. The resource half stays true
    /// for the whole airtime, so a fighter who had double-jumped and then been
    /// launched upward at any speed below `double_jump_speed` read as riding
    /// its own jump, and an aerial DELETED the opponent's launch.
    ///
    /// ⛔⛔ IT ONLY EVER SHRINKS. Set to the jump's authored speed when the air
    /// jump is SPENT, then each tick clamped down to whatever rise survives
    /// (`min(rise)`, floored at zero). Gravity eating the jump reduces it;
    /// somebody else's launch adding rise cannot grow it back, because nothing
    /// but a spend ever raises it. That is what makes it ownership rather than
    /// a speed test — the same rule as the roll's shed and the dash's floor,
    /// written as a quantity instead of a comparison.
    ///
    /// SERIALIZED like [`Self::running`] beside it: a restore mid-rise must not
    /// present a grounded body for the tick before integration rewrites it.
    pub air_jump_rise_owned: f32,
    /// Intangibility earned AT A LEDGE — the grab's window, the getup roll,
    /// and the getup attack's.
    ///
    /// ⛔ it was `dodge_roll_timer`, on the argument that "that field already
    /// gates damage — same pipeline, single source of truth". The pipeline was
    /// the right instinct and the FIELD was the wrong place to join it: the
    /// join belongs at [`super::BodyMotionFacts::evading`], which is what the
    /// damage rule actually reads, and sharing the timer instead made a body
    /// hanging on an edge indistinguishable from one mid-dodge-roll. Nothing
    /// downstream could tell them apart — so a stale `spot_dodging` flag drew a
    /// ledge grab as a spot dodge, and nobody could tune, blink, or reason
    /// about one without the other.
    ///
    /// Separate for the same reason [`Self::air_dodge_timer`] below is separate
    /// from the roll: two maneuvers that grant the same term are still two
    /// maneuvers.
    pub ledge_invuln_timer: f32,
    /// THE SLIVER OF EXPOSURE AT A LEDGE CATCH
    /// ([`crate::ledge_grab::LEDGE_GRAB_VULNERABLE_TIME`]) — while this runs the
    /// body is hanging but still hittable, and [`Self::ledge_invuln_timer`] is
    /// HELD rather than spent.
    pub ledge_vulnerable_timer: f32,
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
    /// Endlag after a GROUND ROLL, the same shape as the air dodge's above and
    /// for the same reason: the roll's invulnerability and its commitment are
    /// different lengths, and a defender reads the gap between them.
    ///
    /// ⛔ Before this existed the roll simply stopped being invulnerable while
    /// the body kept its roll speed — "a roll sends the character flying across
    /// the stage" (Jon, 2026-08-24) — so it was both the fastest way to cross
    /// the stage and a safe one.
    pub dodge_roll_endlag_timer: f32,
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
    /// This tumble came from a launch too hard to TECH out of.
    ///
    /// ⭐ DECIDED AT THE LAUNCH, because that is the only moment the launch
    /// SPEED exists: by the time the body reaches a wall or the floor, nothing
    /// downstream knows how hard the hit was. Cleared on every tumble entry, so
    /// it cannot latch onto a later, gentler one.
    pub tumble_untechable: bool,
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
    /// THE WIRE SHE IS HANGING FROM, or `None` for a body that is not on one.
    ///
    /// ⭐ `Option`, LIKE [`Self::ledge_grab`] BESIDE IT, and for the same reason:
    /// "on a wire" is not a flag beside eight fields that mean nothing without
    /// it. One `is_some()` is the whole of the mode test.
    pub wire: Option<WireState>,
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
            evade_invuln_timer: 0.0,
            dodge_roll_push: 0.0,
            jab_locks: 0,
            initial_dash_timer: 0.0,
            initial_dash_dir: 0.0,
            prev_steer_dir: 0.0,
            turnaround_timer: 0.0,
            teetering: false,
            air_jump_rise_owned: 0.0,
            ledge_invuln_timer: 0.0,
            ledge_vulnerable_timer: 0.0,
            spot_dodging: false,
            air_dodge_timer: 0.0,
            air_dodge_endlag_timer: 0.0,
            dodge_roll_endlag_timer: 0.0,
            tumble_timer: 0.0,
            tumble_until_landing: false,
            tumble_untechable: false,
            tumble_unannounced: false,
            tech_press_timer: 0.0,
            tech_lockout_timer: 0.0,
            knockdown_timer: 0.0,
            getup_invuln_timer: 0.0,
            ledge_grab: None,
            wire: None,
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

    /// This body's authored air-jump budget — the number a refresh restores.
    ///
    /// The sibling of [`Self::shield_tuning`], and it exists for the same
    /// reason: a caller holding the model should not have to know which policy
    /// keeps the number, and a policy with no air jumps answers `0` rather than
    /// forcing a match at every site.
    pub fn air_jumps(&self) -> u8 {
        match self {
            Self::AxisSwept(axis) => axis.params.locomotion.air_jumps,
            Self::SurfaceMomentum(_) | Self::AdhesiveCrawler(_) => 0,
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

/// PUT A BODY ON A WIRE — the typed content→movement op the flyline enters
/// through, and the only way [`AxisManeuverState::wire`] is ever set.
///
/// The caller supplies the ROPE and the anchor is derived from it: a wire comes
/// down out of the sky directly above her, so the swing starts at rest
/// (`angle == 0`) and every pixel of horizontal travel is one the player asked
/// for.
///
/// ⛔ NON-AXIS POLICIES HAVE NO WIRE and refuse, exactly as
/// [`knock_off_ledge`] does. A crawler on a rope is a different mechanic and
/// pretending otherwise would silently do nothing.
///
/// Returns false if the body runs another policy, or if the rope is degenerate
/// (a zero-length wire has no angle and its pendulum is undefined).
#[allow(clippy::too_many_arguments)]
pub fn catch_the_wire(
    model: &mut MotionModel,
    pos: Vec2,
    frame: crate::MotionFrame,
    rope_length: f32,
    lift_s: f32,
    winch_speed: f32,
    max_angle: f32,
    swing_accel: f32,
    release_rise: f32,
) -> bool {
    let MotionModel::AxisSwept(axis) = model else {
        return false;
    };
    if !(rope_length > f32::EPSILON) || !(lift_s > 0.0) {
        return false;
    }
    axis.state.wire = Some(WireState {
        // Straight up from her, in the frame's own up: `down()` is toward the
        // feet, so the sky is the other way.
        anchor: pos - frame.down() * rope_length,
        length: rope_length,
        angle: 0.0,
        ang_vel: 0.0,
        winch_speed,
        lift_remaining_s: lift_s,
        lift_total_s: lift_s,
        max_angle,
        swing_accel,
        release_rise,
    });
    // ⛔⛔ AND THE MANEUVERS SHE WAS MID-WAY THROUGH ARE OVER. A dash timer that
    // survived the catch resumes on the frame the wire lets go and fires her
    // sideways out of her own recovery; the same is true of a jump squat owed a
    // leap. Being lifted out of the scene outranks being busy — the rule
    // `integrate_submerged_clusters` states for the trapdoor.
    axis.state.dash_timer = 0.0;
    axis.state.jump_squat_timer = 0.0;
    true
}

/// CUT THE WIRE — she is off it, and whatever velocity she has now is hers.
///
/// ⛔⛔ THIS IS NOT THE RELEASE. The release at the end of the lift is the
/// kernel's, and it WRITES an exit velocity ([`WireState::release_rise`] plus
/// the swing's own tangential speed). This is the interruption: a hit, a death,
/// a body that left the world. It writes NO velocity, because the thing that
/// interrupted the wire is the thing that owns the body's motion now — a cut
/// that also launched her would be the second authority that deleted the
/// trapdoor's leap for a month.
///
/// Returns true if she was actually on one.
pub fn cut_the_wire(model: &mut MotionModel) -> bool {
    let MotionModel::AxisSwept(axis) = model else {
        return false;
    };
    axis.state.wire.take().is_some()
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
        // ⛔⛔ AND THE CATCH'S UNVESTED PROTECTION GOES WITH THE CATCH. A ledge
        // catch arms its earned invulnerability immediately but holds it behind
        // `ledge_vulnerable_timer` — those are the exposed frames a hit is meant
        // to land in. Leaving the pending grant here meant a fighter struck
        // during its vulnerable catch launched away, the exposure ran out in
        // midair, and the protection it never got to use switched on: intangible
        // in the air, nowhere near the edge that granted it.
        //
        // ⭐ ONLY THE UNVESTED HALF. A window that has already opened
        // (`ledge_vulnerable_timer <= 0.0`) is protection the body is legitimately
        // spending, and it survives ledge transitions on purpose.
        if axis.state.ledge_vulnerable_timer > 0.0 {
            axis.state.ledge_vulnerable_timer = 0.0;
            axis.state.ledge_invuln_timer = 0.0;
        }
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

#[cfg(test)]
mod ledge_catch_grant_tests {
    use super::*;
    use crate::ledge_grab::{LedgeContact, LedgeGrabState};

    fn caught(vulnerable: f32, earned: f32) -> (MotionModel, BodyLedgeState) {
        let mut model = MotionModel::AxisSwept(Default::default());
        if let MotionModel::AxisSwept(axis) = &mut model {
            axis.state.ledge_grab = Some(LedgeGrabState::hanging(LedgeContact {
                wall_normal_x: 1.0,
                anchor: Vec2::ZERO,
                climb_target: Vec2::ZERO,
            }));
            axis.state.ledge_vulnerable_timer = vulnerable;
            axis.state.ledge_invuln_timer = earned;
        }
        (model, BodyLedgeState::default())
    }

    fn timers(model: &MotionModel) -> (f32, f32) {
        match model {
            MotionModel::AxisSwept(axis) => (
                axis.state.ledge_vulnerable_timer,
                axis.state.ledge_invuln_timer,
            ),
            _ => unreachable!("the fixture is axis-swept"),
        }
    }

    /// A CATCH INTERRUPTED IN ITS EXPOSED FRAMES TAKES ITS PROTECTION WITH IT.
    ///
    /// ⛔⛔ The catch arms the earned window at once and hides it behind the two
    /// vulnerable frames. Knocked off during those frames, the body used to keep
    /// the pending grant: the exposure expired in midair and the fighter turned
    /// intangible while launched, far from the edge that granted it.
    #[test]
    fn a_ledge_catch_knocked_off_while_exposed_loses_its_pending_invulnerability() {
        let (mut model, mut ledge) = caught(0.033, 0.5);
        assert!(
            knock_off_ledge(&mut model, &mut ledge),
            "the fixture never hung"
        );
        let (vulnerable, invuln) = timers(&model);
        assert_eq!(
            (vulnerable, invuln),
            (0.0, 0.0),
            "a catch interrupted during its exposed frames kept {invuln:.2}s of \
             invulnerability behind {vulnerable:.2}s of exposure — it will switch on \
             in midair, after the hit that took the ledge away"
        );
    }

    /// ⭐ BUT PROTECTION ALREADY RUNNING IS THE BODY'S TO SPEND. A window past its
    /// exposure survives leaving the ledge on purpose — that is what makes a
    /// ledge getup safe. Clearing indiscriminately would have deleted it.
    #[test]
    fn a_vested_ledge_window_survives_leaving_the_ledge() {
        let (mut model, mut ledge) = caught(0.0, 0.5);
        assert!(
            knock_off_ledge(&mut model, &mut ledge),
            "the fixture never hung"
        );
        let (_, invuln) = timers(&model);
        assert!(
            (invuln - 0.5).abs() < 1e-6,
            "a window that had already opened was revoked ({invuln:.2}s left of 0.50) \
             — that is the legitimate protection a getup spends"
        );
    }
}
