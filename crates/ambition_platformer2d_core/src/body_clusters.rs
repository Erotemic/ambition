//! Authoritative movement-state components shared by every actor body.
//!
//! Cluster components hold shared kinematics, contact, resources, and cooldowns;
//! policy-private maneuver state lives in the body's `MotionModel`.
//! [`BodyClustersMut`] collects mutable cluster references for movement entry
//! points, while [`BodyClusterScratch`] provides the same shape for tests.

use crate::abilities::AbilitySet;
use crate::movement::ComboMark;
use crate::player_state::{BodyMode, ResourceMeter};
use crate::world::{ClimbableContact, WaterContact};
use crate::Vec2;

/// Mutable body-cluster view consumed by movement entry points.
pub struct BodyClustersMut<'a> {
    pub abilities: &'a BodyAbilities,
    pub kinematics: &'a mut BodyKinematics,
    /// Per-tick motion record written by simulation. When absent, swept
    /// readers fall back to the `vel * dt` approximation.
    pub sweep: Option<&'a mut SweepSample>,
    pub base_size: &'a mut BodyBaseSize,
    pub ground: &'a mut BodyGroundState,
    pub wall: &'a mut BodyWallState,
    pub jump: &'a mut BodyJumpState,
    pub dash: &'a mut BodyDashState,
    pub flight: &'a mut BodyFlightState,
    pub blink: &'a mut BodyBlinkState,
    pub ledge: &'a mut BodyLedgeState,
    pub dodge: &'a mut BodyDodgeState,
    pub shield: &'a mut BodyShieldState,
    pub body_mode: &'a mut BodyModeState,
    pub env_contact: &'a mut BodyEnvironmentContact,
    pub mana: &'a mut BodyMana,
    pub offense: &'a mut BodyOffense,
    pub action_buffer: &'a mut BodyActionBuffer,
    pub lifetime: &'a mut BodyLifetime,
    pub combo_trace: &'a mut BodyComboTrace,
}

/// Bevy query data that matches [`BodyClustersMut`]. Use in a system
/// signature as `Query<BodyClusterQueryData, ...>` and call
/// [`BodyClusterQueryDataItem::as_clusters_mut`] to borrow the view.
#[derive(bevy_ecs::query::QueryData)]
#[query_data(mutable)]
pub struct BodyClusterQueryData {
    pub abilities: &'static BodyAbilities,
    pub kinematics: &'static mut BodyKinematics,
    pub sweep: Option<&'static mut SweepSample>,
    pub base_size: &'static mut BodyBaseSize,
    pub ground: &'static mut BodyGroundState,
    pub wall: &'static mut BodyWallState,
    pub jump: &'static mut BodyJumpState,
    pub dash: &'static mut BodyDashState,
    pub flight: &'static mut BodyFlightState,
    pub blink: &'static mut BodyBlinkState,
    pub ledge: &'static mut BodyLedgeState,
    pub dodge: &'static mut BodyDodgeState,
    pub shield: &'static mut BodyShieldState,
    pub body_mode: &'static mut BodyModeState,
    pub env_contact: &'static mut BodyEnvironmentContact,
    pub mana: &'static mut BodyMana,
    pub offense: &'static mut BodyOffense,
    pub action_buffer: &'static mut BodyActionBuffer,
    pub lifetime: &'static mut BodyLifetime,
    pub combo_trace: &'static mut BodyComboTrace,
}

impl<'w, 's> BodyClusterQueryDataItem<'w, 's> {
    /// Borrow the query item as a [`BodyClustersMut`].
    pub fn as_clusters_mut<'a>(&'a mut self) -> BodyClustersMut<'a>
    where
        'w: 'a,
        's: 'a,
    {
        BodyClustersMut {
            abilities: &*self.abilities,
            kinematics: &mut *self.kinematics,
            sweep: self.sweep.as_deref_mut(),
            base_size: &mut *self.base_size,
            ground: &mut *self.ground,
            wall: &mut *self.wall,
            jump: &mut *self.jump,
            dash: &mut *self.dash,
            flight: &mut *self.flight,
            blink: &mut *self.blink,
            ledge: &mut *self.ledge,
            dodge: &mut *self.dodge,
            shield: &mut *self.shield,
            body_mode: &mut *self.body_mode,
            env_contact: &mut *self.env_contact,
            mana: &mut *self.mana,
            offense: &mut *self.offense,
            action_buffer: &mut *self.action_buffer,
            lifetime: &mut *self.lifetime,
            combo_trace: &mut *self.combo_trace,
        }
    }
}

/// Active ability set for this player.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default)]
pub struct BodyAbilities {
    pub abilities: AbilitySet,
}

impl BodyAbilities {
    pub fn new(abilities: AbilitySet) -> Self {
        Self { abilities }
    }
}

/// The body's INTRINSIC capability set — the union of the grant bundles it was
/// authored with, captured at spawn and held constant.
///
/// [`BodyAbilities`] is the *effective* set (what the movement kernel actually
/// reads). This is the *base* it derives from: `effective = base ∩ session_mask`
/// (∪ gear/upgrades once those land). Keeping the base separate is what lets a
/// session-level restriction (the dev editable mask, a story lockout) gate a
/// verb OFF without destroying the character's authored identity — mask it back
/// open and the base is still there. Without this, the only place a body's
/// intrinsic kit lived was the effective set, so anything that wrote the
/// effective set (the F3 dev sync) erased the authored kit permanently.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default)]
pub struct AbilityBase {
    pub abilities: AbilitySet,
}

impl AbilityBase {
    pub fn new(abilities: AbilitySet) -> Self {
        Self { abilities }
    }
}

/// The body's AUTHORED movement feel — a per-character override of the
/// axis-swept tuning (jump arc, air-jump count, …).
///
/// The tuning analogue of [`AbilityBase`], and the third sibling of a
/// character's per-body overrides (abilities, momentum, tuning). Its PRESENCE is
/// the signal that this body's feel is authored: the player integrator refreshes
/// the axis policy's live parameters from THIS instead of the global F3 dev
/// tuning, so a body that carries it (a demo protagonist with a distinct jump)
/// keeps its feel instead of tracking the shared inspector sliders — exactly as
/// a `SurfaceMomentum` body's params already escape that refresh. A body WITHOUT
/// it (the sandbox protagonist) still tracks the F3 editable live, so the dev
/// tuning workflow is unchanged. The value is a full [`MovementTuning`] so the
/// projection to `AxisSweptParams` stays the one existing path.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug)]
pub struct AuthoredMovementTuning(pub crate::movement::MovementTuning);

/// Position, velocity, AABB size, and facing shared by every actor body.
///
/// Systems with multiple mutable `BodyKinematics` queries must make them
/// provably disjoint with marker filters rather than introducing parallel body
/// components. Boss brains emit `desired_vel`; their stored `vel` remains
/// [`Vec2::ZERO`].
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct BodyKinematics {
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub facing: f32,
}

impl Default for BodyKinematics {
    /// Player-flavored default (the only `::default()` callers are player
    /// spawn helpers): a default-sized body at the origin, at rest, facing
    /// right. Matches the pre-unification `PlayerKinematics::default`.
    fn default() -> Self {
        let body = crate::movement::default_player_body_size();
        Self {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            size: body,
            facing: 1.0,
        }
    }
}

impl BodyKinematics {
    /// The body's world-space AABB (centered on `pos`, half-extents `size/2`).
    pub fn aabb(self) -> crate::Aabb {
        crate::Aabb::new(self.pos, self.size * 0.5)
    }

    /// The body's AABB ORIENTED to its gravity/acceleration frame: width<->height
    /// swap under sideways gravity (the body lies along the wall), so the collision
    /// footprint matches the gravity-rotated sprite. Identity under down/up gravity,
    /// so vertical-gravity play is byte-identical to [`Self::aabb`].
    pub fn aabb_oriented(self, gravity_dir: crate::Vec2) -> crate::Aabb {
        let half = crate::AccelerationFrame::new(gravity_dir).to_world_half(self.size * 0.5);
        crate::Aabb::new(self.pos, half)
    }
}

/// Canonical per-tick integration segment, captured entirely inside the
/// simulation kernel from phase entry (`prev`) to phase exit (`curr`). Teleports
/// outside the simulation phase therefore never become swept motion.
///
/// TODO(compat-remove): require `SweepSample` on swept movers and remove reader
/// fallbacks that reconstruct motion from `vel * dt`.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct SweepSample {
    /// Position at simulation-phase entry (the TRUE segment start).
    pub prev: Vec2,
    /// Position at simulation-phase exit. May differ from the body's
    /// CURRENT `pos` on frames where a later system teleported it — the
    /// sample is the traveled path, not the endpoint.
    pub curr: Vec2,
    /// Velocity at `prev` (the motion that produced the path).
    pub vel: Vec2,
    /// Body proxy at the time of the step: AABB half-extents.
    pub half: Vec2,
}

impl SweepSample {
    /// The segment's displacement (`curr − prev`).
    pub fn delta(&self) -> Vec2 {
        self.curr - self.prev
    }
}

/// The player's authored *standing* body size — the baseline the morph /
/// crouch / slide stances and the sprite-scale math read from. Player-only;
/// it is deliberately NOT part of the shared [`BodyKinematics`] (enemies and
/// bosses have no stance-baseline concept), so it rides in its own component
/// alongside the rest of the player clusters.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct BodyBaseSize {
    pub base_size: Vec2,
}

impl Default for BodyBaseSize {
    fn default() -> Self {
        Self {
            base_size: crate::movement::default_player_body_size(),
        }
    }
}

/// Ground CONTACT fact, written by the shared collision doctrine. The
/// coyote / drop-through / rebound grace timers are axis-policy maneuver
/// state and live inside the model variant
/// ([`crate::movement::AxisManeuverState`], ADR 0024).
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct BodyGroundState {
    pub on_ground: bool,
    /// The body's HEAD is against something — the other end of the same
    /// gravity axis `on_ground` reports, sampled by the same pass.
    ///
    /// Persisted rather than read from the step's contacts because the floor
    /// game runs in the CONTROL phase, one phase before the contacts exist: a
    /// ceiling tech has to be decidable at the top of the tick, exactly like
    /// the wall tech decides from `BodyWallState::on_wall`. One frame of
    /// latency costs a surface a body is pinned against nothing.
    pub head_contact: bool,
    /// Whether `on_ground` is a real contact sample for the body's current
    /// pose. Freshly constructed and discretely transited bodies need one
    /// gravity-relative support probe before ordinary movement can interpret a
    /// `false -> true` change as a landing.
    pub contact_initialized: bool,
}

impl BodyGroundState {
    /// Ground state for a newly constructed or discontinuously repositioned
    /// body. The movement kernel samples support at the current pose before
    /// processing control or integration.
    pub const fn uninitialized() -> Self {
        Self {
            on_ground: false,
            head_contact: false,
            contact_initialized: false,
        }
    }

    /// Invalidate contact facts after a discrete pose change. The stale value
    /// is cleared for readers outside the kernel; the next movement step
    /// establishes a new baseline from world geometry.
    pub fn invalidate(&mut self) {
        self.on_ground = false;
        // The head contact is a contact fact too, and a discrete pose change
        // invalidates both ends of the axis or neither.
        self.head_contact = false;
        self.contact_initialized = false;
    }
}

impl Default for BodyGroundState {
    /// A known-airborne value for scratch tests and explicit state fixtures.
    /// Runtime spawn bundles deliberately replace this with
    /// [`Self::uninitialized`].
    fn default() -> Self {
        Self {
            on_ground: false,
            head_contact: false,
            contact_initialized: true,
        }
    }
}

/// Wall CONTACT facts, written by the shared collision doctrine. The
/// cling/climb engagement and the pre-wall momentum window are axis-policy
/// maneuver state and live inside the model variant
/// ([`crate::movement::AxisManeuverState`]).
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyWallState {
    pub on_wall: bool,
    pub wall_normal_x: f32,
}

/// Jump-cluster state. The jump buffer is axis-policy maneuver state
/// ([`crate::movement::AxisManeuverState::buffer_jump`]). This component owns
/// `air_jumps_available` — a PRESERVED body resource, not maneuver state —
/// plus the transient ladder-jump boost / ladder drop-through timers, which
/// AN ACTOR'S SURFACE-CLING GEOMETRY — the surface-walker's normal, and how
/// much gravity it feels.
///
/// the rollback identity does not move with it. The wire name is the
/// stable id `actor.surface_state`, registered by whoever owns the component's
/// registration; no baseline records a crate path, so relocating the definition
/// is not a schema change.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct ActorSurfaceState {
    /// Outward-pointing unit normal of the surface the actor is currently
    /// clinging to. Used by surface-walking archetypes (`PuppySlug`) to crawl
    /// floors, walls, and ceilings; every other archetype pins this at `(0, -1)`
    /// (floor) and ignores it. Engine y grows downward, so floor → (0, -1),
    /// right wall → (-1, 0), ceiling → (0, 1), left wall → (1, 0).
    pub surface_normal: crate::Vec2,
    /// 0.0 = ignores gravity (flying); 1.0 = full gravity.
    pub gravity_scale: f32,
}

/// are body-mode (climbing) mechanics state owned outside the movement policy.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyJumpState {
    pub air_jumps_available: u8,
    /// Recovery uses left before this body has to land or catch something.
    ///
    /// ⛔⛔ A PLATFORM FIGHTER WITHOUT THIS HAS NO BOTTOM BLASTZONE. Nothing
    /// limited a repeated recovery special: no cooldown, no cost, no per-airtime
    /// budget — only `MoveGates::grounded`, which does not distinguish the second
    /// use from the first. A fighter could press Up-B forever and could only be
    /// killed by a launch that outran its own recovery.
    ///
    /// ⭐ AN INTEGER, NOT A FLAG. The genre's default is one, and a heavy or a
    /// winged fighter authoring two is an ordinary tuning statement rather than a
    /// second mechanic — see `AxisLocomotion::recovery_charges`.
    pub recovery_charges: u8,
    /// THIS BODY SPENT ITS LAST RECOVERY AND HAS NOT ANSWERED FOR IT YET — the
    /// helpless EPISODE, as opposed to the resource that opened it.
    ///
    /// ⭐⭐ HELPLESSNESS IS AN EPISODE, NOT A COUNT. It was derived as
    /// `recovery_charges == 0`, which cannot be ENDED by anything short of a
    /// refresh — and a refresh is landing-shaped, which being hit deliberately is
    /// not. So an accepted hit that hands the air dodge back (*"a launched
    /// fighter that could not dodge would have no answer to the follow-up"*) gave
    /// a fighter a dodge it was still forbidden to use.
    ///
    /// ```text
    /// spend the last recovery    arm this
    /// recovery move playing      episode suppressed (the move is the answer)
    /// move ends airborne         helpless becomes effective
    /// flinching Strike           CLEARED — and the CHARGE COMES BACK WITH IT
    /// land / ledge / respawn     cleared with the ordinary refresh
    /// ```
    ///
    /// ⛔⛔ AND THE FLINCH IS THE CONDITION, not "an accepted hit". Jon,
    /// 2026-08-26: *"the rule for refreshing the up-b should be getting
    /// FLINCHED… that hit clears helplessness. Once hitstun ends, you can act
    /// again, INCLUDING using your up-B again."* So a damage-only tick, a hit
    /// an armored fighter ate, and a `HitReaction::Windbox` all leave both the
    /// episode and the charge alone; only a flinching `Strike` lifts them. This
    /// doc used to say a hit *"gives nothing back"* and that the charge was a
    /// different rule *"this must not undo"* — written about the AIR DODGE half
    /// and generalised too far.
    ///
    /// ⛔ THE DOUBLE JUMP IS STILL NOT GIVEN BACK, which is what keeps the two
    /// recovery resources independent: after a flinch the up-B is available
    /// again, the air dodge comes back through its own hit-refresh rule, and the
    /// double jump stays spent.
    pub post_recovery_helpless: bool,
    /// A head is under this body's feet, and the next jump press belongs to
    /// it. Written by the pair pass BEFORE the kernel runs and consumed by
    /// [`crate::movement`]'s jump chain AHEAD of the air jump, so one press has
    /// one meaning: a footstool costs no air jump, and a body with none left can
    /// still take one — which is the genre's rule and was not true when the
    /// bounce merely overwrote the velocity afterwards.
    pub footstool_claimed: bool,
    pub ladder_jump_boost: f32,
    pub ladder_drop_through_timer: f32,
    pub ladder_drop_through_hold_lock: bool,
}

/// Dash-cluster RESOURCES: the charge count and recharge cooldown, preserved
/// across policy switches. The buffered press and the active-dash countdown
/// are axis maneuver state ([`crate::movement::AxisManeuverState`]).
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyDashState {
    pub charges_available: u8,
    pub cooldown: f32,
}

/// Flight ability mode + the airborne carried-momentum channel. The glide /
/// fast-fall flags and hover-bob phase are axis maneuver state
/// ([`crate::movement::AxisManeuverState`]).
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyFlightState {
    pub fly_enabled: bool,
    /// Signed run-axis velocity CARRIED by the body from the world (a portal
    /// fling, knockback, wind) — the floor the hands-off air stop assist
    /// decays toward instead of zero, so imparted momentum is conserved while
    /// ordinary jump drift keeps the tight stop-on-release feel. Clamped each
    /// frame to the actual run velocity (opposing input, walls, and landing
    /// all shrink it naturally) and bled by `MovementTuning::carried_decay`.
    /// World-imparted (written by the portal adapter and by knockback) —
    /// SHARED, not policy-private.
    pub carried_run: f32,
    /// Seconds for which world-imparted carried momentum is protected from the
    /// normal airborne stop-assist decay. `0.0` uses ordinary `carried_decay`;
    /// portal frame changes intentionally leave it at zero.
    pub carried_hold: f32,
    /// World-space reaction launch waiting for the motion model to consume it.
    ///
    /// Writers cannot assign `BodyKinematics::vel` directly because some motion
    /// models publish velocity from their own authoritative coordinates.
    /// `step_motion` drains this field and interprets the launch for the active
    /// model. It lives in rollback-registered `BodyFlightState`, and `Vec2::ZERO`
    /// is the no-op/empty representation.
    pub pending_launch: crate::Vec2,
    /// IS THE PENDING LAUNCH A PUSH RATHER THAN A HIT?
    ///
    /// ⭐⭐ THE FLOOR GAME IS DECIDED FROM SPEED ALONE WITHOUT THIS. The one
    /// gateway every launch passes through asks `jab_lock` and
    /// `launch_into_tumble` from `launch.length()`, so a WEAK GUST could pin a
    /// prone body where it lies and a STRONG one could send it tumbling — both
    /// against a volume whose authored contract is *"moves you and leaves you in
    /// control"*.
    ///
    /// ⛔ THE KIND WAS GENUINELY ABSENT, unlike the parry and guard halves of the
    /// same review item, where `flinchless` was already riding the knockback to
    /// the site. `pending_launch` is a bare `Vec2`, so the gateway had nothing to
    /// read — which is why this one costs wire format and those did not.
    ///
    /// ⛔ STAGE THE PAIR WITH [`Self::stage_launch`] AND DRAIN IT WITH
    /// [`Self::take_launch`]. A caller that writes `pending_launch` directly gets
    /// `false`, which is what an ordinary knockback means, and is why the many
    /// fixtures that set the vector by hand still say what they meant.
    pub pending_launch_flinchless: bool,
}

/// A launch waiting at the gateway, and what KIND of thing it is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PendingLaunch {
    pub velocity: crate::Vec2,
    /// A push, not a hit: it moves the body and leaves it in control, so it
    /// neither pins a prone body nor starts a tumble.
    pub flinchless: bool,
}

impl PendingLaunch {
    /// Nothing is waiting. `Vec2::ZERO` is the empty state, so the common path
    /// is one comparison.
    pub fn is_empty(&self) -> bool {
        self.velocity.length_squared() <= 1.0e-6
    }
}

impl BodyFlightState {
    /// Hand a launch to the gateway, with the kind it is.
    ///
    /// ⭐ ONE CALL FOR THE PAIR, so a writer cannot set the velocity and forget
    /// the kind — which is the whole hazard of a flag beside a vector.
    pub fn stage_launch(&mut self, velocity: crate::Vec2, flinchless: bool) {
        self.pending_launch = velocity;
        self.pending_launch_flinchless = flinchless;
    }

    /// READ the waiting launch without spending it.
    ///
    /// ⛔ FOR ONE CALLER AND ONE REASON: a body whose pose another authority owns
    /// must still learn what the hit MEANT (does it tumble?) on the tick it
    /// lands, while the TRAVEL waits for a tick the body can actually move. See
    /// `MotionStepContext::pose_owned_externally`. Anything that means to spend
    /// the launch calls [`Self::take_launch`].
    pub fn pending_launch_state(&self) -> PendingLaunch {
        PendingLaunch {
            velocity: self.pending_launch,
            flinchless: self.pending_launch_flinchless,
        }
    }

    /// Take the waiting launch, clearing both halves together.
    pub fn take_launch(&mut self) -> PendingLaunch {
        PendingLaunch {
            velocity: std::mem::replace(&mut self.pending_launch, crate::Vec2::ZERO),
            flinchless: std::mem::replace(&mut self.pending_launch_flinchless, false),
        }
    }
}

/// Blink RESOURCE: the recharge cooldown, preserved across policy switches.
/// The hold-to-aim lifecycle, aim offset, and post-blink grace timer are axis
/// maneuver state ([`crate::movement::AxisManeuverState`]).
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyBlinkState {
    pub cooldown: f32,
}

/// Ledge re-grab cooldown (a time fact, shared with combat's knock-off rule).
/// The hang / pull-up state itself is axis maneuver state
/// ([`crate::movement::AxisManeuverState::ledge_grab`]); combat knocks a body
/// off a ledge through the typed [`crate::movement::knock_off_ledge`] op.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyLedgeState {
    pub release_cooldown: f32,
}

/// Re-grab lockout applied when a hit knocks the player off a ledge, so they
/// fall with the knockback instead of instantly re-latching.
pub const LEDGE_KNOCK_OFF_COOLDOWN: f32 = 0.35;

/// Dodge RESOURCE: the cooldown. The active i-frame roll timer is axis
/// maneuver state ([`crate::movement::AxisManeuverState::dodge_roll_timer`]).
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyDodgeState {
    pub cooldown: f32,
    /// The air dodge has been spent in this airtime.
    ///
    /// Cleared where every other aerial resource is — on ground contact and on a ledge grab —
    /// rather than by a timer, because "one per trip through the air" is the rule players learn.
    pub air_dodge_spent: bool,
    /// DODGE STALING — how many evades this body has spent recently.
    ///
    /// ⭐⭐ THE GENRE'S ANSWER TO A SPAMMED ROLL, and it is why staling belongs
    /// on the DODGE rather than on the move-staling queue: an evade is not a
    /// move, it has no id, and the thing that wears out is the OPTION rather
    /// than any particular one of them. A body that has rolled four times in a
    /// row should find the fifth roll worse — which is exactly what stops
    /// rolling being the answer to everything.
    ///
    /// Counted up by each evade and bled off by [`Self::stale_decay`], so a
    /// fighter who stops rolling gets the option back. `0` is a fresh body and
    /// is what every body carries in a game that declares no staling.
    pub evades_recent: u8,
    /// Seconds left before one recent evade is forgiven.
    ///
    /// ⛔ A DECAY, NOT A WINDOW. A fixed window would forgive the whole count at
    /// once, so a fighter could roll four times, wait, and be fully fresh; the
    /// count comes off one at a time so recovering costs proportionally.
    pub stale_decay: f32,
}

/// How long a caught parry stays readable, in seconds of the body's own time.
///
/// A READ window, not a mechanic: nothing about hit eligibility, shield cost or
/// timing consults it, so the number only has to outlive a frame. It is a
/// constant rather than a `ShieldTuning` row for exactly that reason — a game
/// that wants a different parry FEEL tunes `parry_window_time`, which is the
/// value with consequences.
pub const PARRY_CAUGHT_READ_TIME: f32 = 0.20;

/// Shield/parry cluster.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyShieldState {
    pub active: bool,
    pub parry_window_timer: f32,
    /// Integrity SPENT, not integrity left — so the derived `Default` (a
    /// fresh, undamaged guard) is right for every body, including the bodies
    /// whose [`crate::ShieldTuning`] leaves the shield unlimited.
    pub depleted: f32,
    /// Dizzy after a break: the guard cannot be raised and the body has no
    /// control authority while this is positive.
    pub break_timer: f32,
    /// Shieldstun: the defender owes this after blocking, and it is what makes
    /// a blocked hit cost the blocker tempo instead of nothing.
    pub stun_timer: f32,
    /// How long THIS break was, stamped when the guard shatters, so
    /// [`Self::break_phase`] can say how far through it the body is.
    ///
    /// not a derive memo. The phase needs
    /// `break_timer / ShieldTuning::break_stun_time`, and the tuning is not
    /// available where the phase is READ — both presentation sites hold this
    /// component and no tuning, and reaching for `MotionModel::shield_tuning()`
    /// from the view would put presentation back inside policy internals. So
    /// the authority for the denominator is written ONCE at the break, and a
    /// rewind restores the same answer rather than recomputing it against
    /// whatever tuning is live later.
    ///
    /// `0.0` when no break is running, which is what makes the phase `None`.
    pub break_total: f32,
    /// A parry CAUGHT a strike, and this is what is left of the beat.
    ///
    /// The distinction [`Self::parrying`] cannot make. That one answers whether
    /// the window is OPEN, which is true of every raised guard for a few ticks
    /// and says nothing about whether anything was caught — so a cue driven off
    /// it fires on every shield raise, which is the wrong beat. This is armed
    /// only where a strike was actually swallowed by the window, by
    /// [`Self::catch_parry`], which is the ONE site every route calls.
    ///
    /// A DURATION rather than a flag because a parry is over in one tick and a
    /// reader is not: presentation samples state, and a bool set and cleared
    /// inside a sim tick can be missed entirely. Same shape as
    /// `BodyCombat::hit_flash` for the same reason.
    pub parry_caught_timer: f32,
    /// This guard was SPENT by an out-of-shield action and may not be raised
    /// again until the button comes up.
    ///
    /// Without it a held button re-raises the guard on the very next tick —
    /// and re-arms the parry window with it — so acting out of shield would
    /// cost nothing and hand back a fresh perfect-shield every time. The lock
    /// is what makes an out-of-shield option a spend.
    pub release_locked: bool,
    /// Hard commitment owed for lowering the guard by itself — shield-drop lag
    /// ([`crate::ShieldTuning::drop_lag`]).
    ///
    /// A separate clock from [`Self::stun_timer`] beside it, and deliberately:
    /// shieldstun is what a BLOCKED HIT charges and this is what LETTING GO
    /// costs. One timer for two causes is the mistake the ledge's borrowed
    /// dodge timer already taught this tree once.
    pub drop_lag_timer: f32,
    /// WHERE THE RAISED GUARD SITS, as a signed fraction of the body's
    /// half-height along gravity — positive toward the feet. `0.0` is a guard
    /// centred on the body, which is every guard whose
    /// [`crate::ShieldTuning::tilt_range`] is zero.
    ///
    /// RESOLVED ONCE per tick and read by two consumers that must not disagree:
    /// the coverage rule that decides whether a poke gets through, and the view
    /// that draws the bubble. Deriving it separately at each would let the
    /// picture show a guard the hit test does not use.
    ///
    /// Zero whenever the guard is DOWN, so a lowered shield has no stale lean
    /// waiting for it when it comes back up.
    pub shield_tilt: f32,
}

impl BodyShieldState {
    /// Is this body inside its perfect-shield window?
    ///
    /// the timer ALONE, and dropping `active` was the point. Under
    /// Ultimate's release-timed parry ([`crate::ParryTiming::OnRelease`]) the
    /// window is live while the guard is DOWN, so an `active &&` term would make
    /// that setting unreachable. The term is not lost: `resolve_shield` is the
    /// one place the timer is armed, and only a guard that was UP can be
    /// released — so "a shield past its window does not parry" still holds,
    /// because such a shield has a zero timer.
    pub fn parrying(self) -> bool {
        self.parry_window_timer > 0.0
    }

    /// This shield's parry window just caught a strike.
    pub fn parry_caught(self) -> bool {
        self.parry_caught_timer > 0.0
    }

    /// Spend this guard on an out-of-shield action: it comes down, its parry
    /// window closes with it, and it stays down until the button is released.
    ///
    /// ⭐ the ONE place a guard is spent by acting. An action that lowered
    /// `active` without taking the window and the lock would hand the body a
    /// free parry on the next tick.
    pub fn spend_on_action(&mut self) {
        self.active = false;
        self.parry_window_timer = 0.0;
        self.release_locked = true;
    }

    /// Record that this shield's parry window CAUGHT a strike.
    ///
    /// ⭐ THE one arming site. Every route a parry can happen on — a melee
    /// strike the damage gate refused, a shot the guard turned around — calls
    /// this rather than each publishing its own signal, so a consumer reads one
    /// fact and a route added later is visible for free.
    pub fn catch_parry(&mut self) {
        self.parry_caught_timer = PARRY_CAUGHT_READ_TIME;
    }

    /// The guard was broken and the body is still paying for it.
    pub fn broken(self) -> bool {
        self.break_timer > 0.0
    }

    /// How far through the break this body is: `0.0` the instant the guard
    /// shatters, approaching `1.0` as it recovers. `None` when no break runs.
    ///
    /// the genre draws a break as a SEQUENCE — launched, falling, collapsed,
    /// recovering — and the sim publishes one countdown. This is the whole
    /// distinction, derived rather than stored: four beats out of one timer and
    /// the length it started at.
    pub fn break_phase(self) -> Option<f32> {
        (self.break_timer > 0.0 && self.break_total > 0.0)
            .then(|| 1.0 - (self.break_timer / self.break_total).clamp(0.0, 1.0))
    }

    /// `1.0` fresh, `0.0` about to break — and `1.0` for a body whose shield is
    /// not a resource, so a presentation reader needs no special case.
    pub fn integrity_fraction(self, tuning: crate::ShieldTuning) -> f32 {
        if !tuning.is_resource() {
            return 1.0;
        }
        (1.0 - self.depleted / tuning.max_health).clamp(0.0, 1.0)
    }
}

/// Break the guard: spend it to the last point, drop it, and start the dizzy.
pub fn break_shield(shield: &mut BodyShieldState, tuning: crate::ShieldTuning) {
    shield.depleted = tuning.max_health;
    shield.break_timer = tuning.break_stun_time;
    shield.break_total = tuning.break_stun_time;
    shield.active = false;
    shield.parry_window_timer = 0.0;
}

/// Spend the guard on a blocked hit, breaking it if that empties it.
///
/// Called from the damage resolver's block branch so the block and the cost it
/// carries are one step, not a decision plus a follow-up nobody owes.
pub fn spend_shield_on_block(
    shield: &mut BodyShieldState,
    tuning: crate::ShieldTuning,
    damage: i32,
) {
    if !tuning.is_resource() {
        return;
    }
    let damage = damage.max(0) as f32;
    shield.stun_timer = shield.stun_timer.max(damage * tuning.stun_per_damage);
    shield.depleted += damage * tuning.damage_scale;
    if shield.depleted >= tuning.max_health {
        break_shield(shield, tuning);
    }
}

/// The shield resource's whole clock: the dizzy, the drain while held, and
/// the regeneration while down.
///
/// A break restores full integrity when the dizzy ends, so recovery is one
/// event rather than a slow crawl back from zero.
/// Returns `true` on the ONE tick a break becomes visible, whoever caused it —
/// the drain below or a blocked hit — so the presentation has a single emit site
/// instead of one per cause. The test is that the dizzy has not started counting
/// yet, and only this function counts it.
pub fn tick_shield_resource(
    shield: &mut BodyShieldState,
    tuning: crate::ShieldTuning,
    dt: f32,
) -> bool {
    if !tuning.is_resource() {
        return false;
    }
    shield.stun_timer = (shield.stun_timer - dt).max(0.0);
    if shield.active {
        shield.depleted += tuning.drain_per_second * dt;
        if shield.depleted >= tuning.max_health {
            break_shield(shield, tuning);
        }
    } else if shield.break_timer <= 0.0 {
        shield.depleted = (shield.depleted - tuning.regen_per_second * dt).max(0.0);
    }
    if shield.break_timer <= 0.0 {
        return false;
    }
    let freshly_broken = shield.break_timer >= tuning.break_stun_time;
    shield.break_timer = (shield.break_timer - dt).max(0.0);
    if shield.break_timer <= 0.0 {
        shield.depleted = 0.0;
    }
    freshly_broken
}

/// Announces that a body has restarted.
///
/// [`reset_body_clusters`] clears engine-owned state. Providers observe this
/// event to reset authoritative state they own. The event is derived from
/// [`BodyLifetime::restart_pending`] so every reset path announces once.
#[derive(bevy_ecs::event::EntityEvent, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyRestarted {
    /// The body starting again.
    pub entity: bevy_ecs::entity::Entity,
}

/// Turn pending restarts into [`BodyRestarted`] triggers.
///
/// This runs at the front of the sim tick and clears each pending flag exactly
/// once. Observers run at the next command flush rather than reentrantly.
pub fn announce_body_restarts(
    mut commands: bevy_ecs::system::Commands,
    mut bodies: bevy_ecs::system::Query<(bevy_ecs::entity::Entity, &mut BodyLifetime)>,
) {
    for (entity, mut lifetime) in &mut bodies {
        if !lifetime.restart_pending {
            continue;
        }
        lifetime.restart_pending = false;
        commands.trigger(BodyRestarted { entity });
    }
}

/// Reset a live player back to spawn while preserving the
/// `BodyAbilities` and incrementing the lifetime reset counter. The
/// combo trace is wiped and a fresh `MovementOp::Reset` mark is pushed.
///
/// The pose snap is a discrete TRANSIT ([`crate::movement::transit_body`], the
/// ADR 0024 authority): it also reconciles model-private attachment, so a
/// riding momentum body or an attached crawler cannot carry a stale surface
/// identity into the destination room/spawn. A body reset is a full respawn,
/// so the axis policy's private maneuver state is reset wholesale too.
///
/// `air_jumps_default` is the mid-air jump count to restore when the body's own
/// `BodyAbilities` does not name one.
///
/// ```ignore
/// ae::reset_body_clusters(..);
/// // reset_body_clusters uses DEFAULT_TUNING for the post-reset dash/jump
/// // refresh; redo with the live tuning so a F3 live-tuning session sees its
/// // overridden air_jumps / dash_charge_count immediately after a reset.
/// ae::refresh_movement_resources_clusters(..);
/// ```
///
/// Every other caller did not, and got default air jumps after a reset with
/// nothing saying so. An authority that requires a follow-up call is not an
/// authority — it is a two-step ritual, and the second step is the one people
/// forget. Now the question is asked at the call site, where the answer is.
pub fn reset_body_clusters(
    model: &mut crate::movement::MotionModel,
    clusters: &mut BodyClustersMut<'_>,
    spawn: Vec2,
    air_jumps_default: u8,
) {
    use crate::movement::{ComboMark, MovementOp};

    let new_resets = clusters.lifetime.resets + 1;
    let abilities = clusters.abilities.abilities;
    // A reset restores the body to its BASE size; it does not redefine what the
    // base IS. `base_size` is IDENTITY-derived — a worn form, a mount, a boss
    // phase — and only the authority owning that identity may write it. Hardcoding
    // the default here silently unmade any identity-driven size on every reset:
    // a grown Super Mary-O who fell in a pit came back with a SMALL collider while
    // still wearing the cap and still presenting the tall sprite, and her own
    // `sync_grown_form` could not repair it because it compares equipment against
    // `WornCharacter` (which the reset left tall) rather than against the collider.
    // Hitbox and identity disagreed, invisibly, until the next hit.
    //
    // A default-constructed body's `base_size` is already the default player size,
    // so nothing that never sets an identity size changes behavior.
    let body = clusters.base_size.base_size;
    let dash_charges = abilities.dash_charge_count();
    let air_jumps = abilities.air_jump_count(air_jumps_default);

    clusters.kinematics.size = body;
    clusters.kinematics.facing = 1.0;
    crate::movement::transit_body(
        model,
        clusters,
        spawn,
        crate::movement::TransitVelocity::Zero,
    );
    if let crate::movement::MotionModel::AxisSwept(axis) = model {
        axis.state = crate::movement::AxisManeuverState::default();
    }
    // `base_size` is deliberately NOT written: see the note above.
    *clusters.ground = BodyGroundState::uninitialized();
    *clusters.wall = BodyWallState::default();
    *clusters.jump = BodyJumpState::fresh(air_jumps);
    *clusters.dash = BodyDashState {
        charges_available: dash_charges,
        ..Default::default()
    };
    *clusters.flight = BodyFlightState {
        fly_enabled: abilities.fly && !abilities.fly_toggle,
        ..BodyFlightState::default()
    };
    *clusters.blink = BodyBlinkState::default();
    *clusters.ledge = BodyLedgeState::default();
    *clusters.dodge = BodyDodgeState::default();
    *clusters.shield = BodyShieldState::default();
    *clusters.body_mode = BodyModeState::default();
    *clusters.env_contact = BodyEnvironmentContact::default();
    *clusters.mana = BodyMana::default();
    *clusters.offense = BodyOffense::default();
    *clusters.action_buffer = BodyActionBuffer::default();
    *clusters.lifetime = BodyLifetime {
        resets: new_resets,
        // The announcement is DERIVED from the reset, not asked of the caller. A flag on state
        // this function already owns cannot be forgotten by a caller that does not know it
        // exists.
        restart_pending: true,
        ..Default::default()
    };
    clusters.combo_trace.combo.clear();
    clusters.combo_trace.combo.push(ComboMark {
        op: MovementOp::Reset,
        age: 0.0,
    });
}

/// Refresh every AERIAL resource — dash charges, air jumps, and the air dodge —
/// from the active `BodyAbilities` + the caller's authored base air-jump count.
///
/// ⛔⛔ THIS IS THE **LANDING-CLASS** REFRESH, AND IT IS NOT THE ONLY CAUSE.
/// "Restores on landing" is a rule about a class of resource, which is why all
/// three sit here — but a cause that resets the body WITHOUT re-seating it
/// resets less. An ordinary hit gives back the air dodge AND the recovery — a
/// flinch is what lifts freefall, and lifting the punishment without returning
/// the thing punished for is neither rule — but a spent double jump stays spent
/// through an edge-guard, and the traversal dash is Ambition's own capability
/// that no hit reaction recharges. A caller that
/// reaches for this function is claiming its cause is landing-shaped — catching
/// the ledge is, being hit is not.
pub fn refresh_movement_resources_clusters(
    abilities: &BodyAbilities,
    dash: &mut BodyDashState,
    jump: &mut BodyJumpState,
    dodge: &mut BodyDodgeState,
    base_air_jumps: u8,
    recovery: RecoveryRefresh,
) {
    dash.charges_available = abilities.abilities.dash_charge_count();
    jump.air_jumps_available = abilities.abilities.air_jump_count(base_air_jumps);
    if recovery == RecoveryRefresh::Answered {
        jump.recovery_charges = DEFAULT_RECOVERY_CHARGES;
        // The episode ends with the resource that opened it — this helper is the
        // landing-shaped refresh, and being re-seated is exactly what answers for a
        // spent recovery.
        jump.post_recovery_helpless = false;
    }
    dodge.air_dodge_spent = false;
}

/// May this refresh answer for a spent recovery?
///
/// ⭐⭐ BECAUSE "GROUNDED" IS ASKED EVERY TICK AND "LANDED" IS AN EVENT, and the
/// refresh above is written for the second while two of its callers run on the
/// first. That is harmless for the aerial resources — a body standing on the
/// floor cannot spend an air jump — and it is NOT harmless for the recovery,
/// which is the one resource a fighter spends WHILE STILL GROUNDED. Measured on
/// the pirate: `call_the_shark` spends the charge on the press frame and the
/// grounded refresh hands it straight back on the next one, so the fighter
/// boards the shark carrying a full recovery and the rule that a flinch
/// refreshes it cannot be tested at all.
///
/// ⛔ THE FIX IS NOT "STOP REFRESHING WHILE GROUNDED". A grounded up-B that gets
/// stuffed and stays on the floor still wants ordinary landing-class semantics;
/// the genre lets a standing fighter press it again. What must not happen is the
/// floor answering for a recovery whose move is still running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryRefresh {
    /// This cause re-seated the body: landing, catching the ledge, a respawn, a
    /// rebound. The charge comes back and the helpless episode ends.
    Answered,
    /// This cause is a grounded TICK during a recovery the body has already
    /// committed. The aerial resources refresh; the recovery does not, because
    /// the move that spent it has not finished happening yet.
    Withheld,
}

/// Recovery uses a body gets back when it is re-seated — landing, catching the
/// ledge, being grabbed, a respawn.
///
/// ⭐ ONE, which is the genre's answer, and a CONSTANT until a fighter wants a
/// different number. `air_jumps` is authored per body because bodies genuinely
/// differ about it; nothing in the tree authors a second recovery yet, and
/// adding the field before there is a customer would be a knob nobody turns.
/// Promote it to `AxisLocomotion` beside `air_jumps` when one appears — the
/// budget is already an integer precisely so that costs nothing.
pub const DEFAULT_RECOVERY_CHARGES: u8 = 1;

impl BodyJumpState {
    /// A body that is STARTING — a fresh spawn or a reset — with its air budget
    /// full.
    ///
    /// ⛔⛔ `Default` IS NOT THIS, and must not become it. Default is the SPENT
    /// state: zero air jumps, zero recoveries, which is what a test wants when it
    /// says "this body has nothing left". Two construction paths spelled the
    /// fresh state as `air_jumps_available: n, ..Default::default()` and so
    /// silently gave every fresh body ZERO recoveries — a stock-respawned fighter
    /// came back in the air unable to use the special that was supposed to save
    /// it, because `reset_body_clusters` is followed by no landing-class refresh.
    ///
    /// ⇒ ONE initializer, and the cause model reads:
    ///
    /// ```text
    /// fresh body / respawn  full initial budget   (here)
    /// landing · ledge · capture  refresh           (refresh_movement_resources_clusters)
    /// flinching Strike      the air dodge AND the recovery  (apply_body_hit_reaction)
    /// damage-only · armored · windbox   nothing
    /// ```
    pub fn fresh(air_jumps_available: u8) -> Self {
        Self {
            air_jumps_available,
            recovery_charges: DEFAULT_RECOVERY_CHARGES,
            post_recovery_helpless: false,
            footstool_claimed: false,
            ladder_jump_boost: 0.0,
            ladder_drop_through_timer: 0.0,
            ladder_drop_through_hold_lock: false,
        }
    }
}

/// Authoritative body-shape stance.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyModeState {
    pub body_mode: BodyMode,
}

/// Per-frame world-contact cluster: water + climbable region overlap.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyEnvironmentContact {
    pub water: Option<WaterContact>,
    pub climbable: Option<ClimbableContact>,
}

/// Generic spendable meter the player draws on for charge attacks /
/// special abilities.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct BodyMana {
    pub meter: ResourceMeter,
}

impl Default for BodyMana {
    fn default() -> Self {
        Self {
            meter: ResourceMeter::new(100.0, 0.0, 0.0),
        }
    }
}

/// Offensive scaling knobs.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyOffense {
    pub damage_multiplier: i32,
}

impl Default for BodyOffense {
    fn default() -> Self {
        Self {
            damage_multiplier: 1,
        }
    }
}

/// ECS-owned COMBAT action buffer: seconds of the owner's proper time each
/// pressed combat verb stays SPENDABLE after its press edge. The MOVEMENT
/// buffers (jump / dash / blink) are axis-policy maneuver state
/// ([`crate::movement::AxisManeuverState::buffer_jump`] and siblings).
///
/// A slot is armed by the press edge and spent by the action authority that
/// ACCEPTS the action — the buffer proposes the press again on every tick of
/// its window, and the authority (a move's cancel window, a shield release, a
/// landing) still decides. A window that expires unspent is a press that was
/// genuinely too early, not a press that was dropped by an off-by-one frame.
///
/// ⛔ this is a semantic verb buffer, never raw device input: every controller
/// — human, brain, replay, RRL policy — reaches it through the same resolved
/// control state, so leniency is one mechanic rather than a player-only road.
///
/// The ATTACK slot times a press whose MEANING (direction, tilt-vs-smash,
/// posture) is `AttackGestureState::buffered_press` in `ambition_characters`:
/// this crate owns the clock, the gesture interpreter owns the intent it
/// produced, and the two are cleared together. The other three verbs are bare
/// edges and need no rider.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyActionBuffer {
    pub attack: f32,
    pub pogo: f32,
    pub grab: f32,
    pub special: f32,
}

impl BodyActionBuffer {
    /// Every slot in declaration order — the exhaustive destructure that keeps
    /// [`Self::tick`] and every "is anything buffered" question from silently
    /// skipping a slot added later.
    fn slots(&mut self) -> [&mut f32; 4] {
        let Self {
            attack,
            pogo,
            grab,
            special,
        } = self;
        [attack, pogo, grab, special]
    }

    /// Decay every window by `dt` seconds of the owner's proper time.
    pub fn tick(&mut self, dt: f32) {
        for slot in self.slots() {
            *slot = (*slot - dt).max(0.0);
        }
    }

    /// No verb is waiting. Compared against the default rather than field by
    /// field, so a slot added later is covered without editing this.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Forget the press the ATTACK/POGO edges jointly proposed.
    ///
    /// Both, because they resolve to one move: a body pressing the dedicated
    /// pogo and the attack on the same tick starts a single timeline, and
    /// leaving the other slot armed would start a second one behind it.
    pub fn spend_attack(&mut self) {
        self.attack = 0.0;
        self.pogo = 0.0;
    }
}

/// Lifetime + diagnostic counters.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyLifetime {
    pub time_alive: f32,
    pub resets: u32,
    pub max_speed: f32,
    /// This body was reset and the providers have not been told yet.
    ///
    /// Set by [`reset_body_clusters`], cleared by [`announce_body_restarts`],
    /// which turns it into a [`BodyRestarted`] trigger. It rides here rather
    /// than in a component of its own because this one is already snapshotted
    /// and restored: a resimulation that replays the reset replays the
    /// announcement with it, which a separate unregistered marker could not do.
    pub restart_pending: bool,
}

/// Symbolic operation trace ("J o D o D"), preserved across the
/// engine-Player tick scratchpad so the HUD combo readout doesn't
/// blank every frame.
#[derive(bevy_ecs::component::Component, Clone, Debug, Default)]
pub struct BodyComboTrace {
    pub combo: Vec<ComboMark>,
}

impl BodyComboTrace {
    pub fn symbols(&self) -> String {
        if self.combo.is_empty() {
            return "-".to_string();
        }
        self.combo
            .iter()
            .map(|m| m.op.symbol())
            .collect::<Vec<_>>()
            .join(" o ")
    }
}

/// Owned bag of all 18 player cluster components PLUS the body's
/// [`MotionModel`], used by unit tests and the non-ECS call sites that need
/// to assemble a whole body without a Bevy entity (a body without a policy is
/// not a body). Construct via [`BodyClusterScratch::new_with_abilities`] and
/// re-borrow the cluster view via [`BodyClusterScratch::as_mut`], or split
/// model + clusters via [`BodyClusterScratch::parts`].
#[derive(Clone, Debug)]
pub struct BodyClusterScratch {
    /// The movement policy, held ALONGSIDE the clusters (in ECS it is its own
    /// component). Persistent across steps so model-private maneuver state
    /// (ADR 0024) survives multi-tick scratch tests exactly as it does on a
    /// live entity.
    pub model: crate::movement::MotionModel,
    pub abilities: BodyAbilities,
    pub kinematics: BodyKinematics,
    pub base_size: BodyBaseSize,
    pub ground: BodyGroundState,
    pub wall: BodyWallState,
    pub jump: BodyJumpState,
    pub dash: BodyDashState,
    pub flight: BodyFlightState,
    pub blink: BodyBlinkState,
    pub ledge: BodyLedgeState,
    pub dodge: BodyDodgeState,
    pub shield: BodyShieldState,
    pub body_mode: BodyModeState,
    pub env_contact: BodyEnvironmentContact,
    pub mana: BodyMana,
    pub offense: BodyOffense,
    pub action_buffer: BodyActionBuffer,
    pub lifetime: BodyLifetime,
    pub combo_trace: BodyComboTrace,
}

impl BodyClusterScratch {
    /// Build a `BodyClusterScratch` for a fresh player at `spawn`
    /// with the given `AbilitySet` — same defaults as
    /// `Player::new_with_abilities` but without materializing the
    /// monolithic `Player` aggregate.
    pub fn new_with_abilities(spawn: Vec2, abilities: crate::abilities::AbilitySet) -> Self {
        use crate::movement::{default_player_body_size, DEFAULT_TUNING};
        let body = default_player_body_size();
        let dash_charges = abilities.dash_charge_count();
        let air_jumps = abilities.air_jump_count(DEFAULT_TUNING.air_jumps);
        Self {
            model: crate::movement::MotionModel::default(),
            abilities: BodyAbilities { abilities },
            kinematics: BodyKinematics {
                pos: spawn,
                vel: Vec2::ZERO,
                size: body,
                facing: 1.0,
            },
            base_size: BodyBaseSize { base_size: body },
            ground: BodyGroundState::default(),
            wall: BodyWallState::default(),
            jump: BodyJumpState::fresh(air_jumps),
            dash: BodyDashState {
                charges_available: dash_charges,
                cooldown: 0.0,
            },
            flight: BodyFlightState::default(),
            blink: BodyBlinkState::default(),
            ledge: BodyLedgeState::default(),
            dodge: BodyDodgeState::default(),
            shield: BodyShieldState::default(),
            body_mode: BodyModeState::default(),
            env_contact: BodyEnvironmentContact::default(),
            mana: BodyMana {
                meter: ResourceMeter::new(100.0, 0.0, 0.0),
            },
            offense: BodyOffense {
                damage_multiplier: 1,
            },
            action_buffer: BodyActionBuffer::default(),
            lifetime: BodyLifetime::default(),
            combo_trace: BodyComboTrace::default(),
        }
    }

    /// Born moving. [`Self::new_with_abilities`] constructs a body at rest;
    /// a caller assembling a hypothetical — a recovery probe reconstructing "the
    /// body as this line left it", a fixture starting mid-fall — states its
    /// velocity here instead of reaching into `kinematics` afterwards.
    ///
    /// this is a CONSTRUCTION seam, not an authority. ADR 0024's velocity
    /// authority governs a simulated body being changed under a resolved frame;
    /// a `BodyClusterScratch` is an owned bag with no entity, no frame and no
    /// integrator, so there is no authority to route through — only the question
    /// of whether the initial state is said at construction or patched on after.
    /// `engine.velocity-writes-are-authority-only` matches the receiver's NAME
    /// and so cannot tell those apart; see that policy's rationale.
    #[must_use]
    pub fn with_velocity(mut self, vel: Vec2) -> Self {
        self.kinematics = BodyKinematics {
            vel,
            ..self.kinematics
        };
        self
    }

    /// Split-borrow the scratch body into its policy and its cluster view,
    /// mirroring the ECS shape (the model is a separate component from the
    /// clusters) so scratch callers can hand both to [`crate::step_motion`].
    pub fn parts(&mut self) -> (&mut crate::movement::MotionModel, BodyClustersMut<'_>) {
        let clusters = BodyClustersMut {
            abilities: &self.abilities,
            kinematics: &mut self.kinematics,
            // Scratch is the non-ECS test scratchpad; it carries no sample
            // (tests that observe the sample set `clusters.sweep` on the
            // borrowed view directly).
            sweep: None,
            base_size: &mut self.base_size,
            ground: &mut self.ground,
            wall: &mut self.wall,
            jump: &mut self.jump,
            dash: &mut self.dash,
            flight: &mut self.flight,
            blink: &mut self.blink,
            ledge: &mut self.ledge,
            dodge: &mut self.dodge,
            shield: &mut self.shield,
            body_mode: &mut self.body_mode,
            env_contact: &mut self.env_contact,
            mana: &mut self.mana,
            offense: &mut self.offense,
            action_buffer: &mut self.action_buffer,
            lifetime: &mut self.lifetime,
            combo_trace: &mut self.combo_trace,
        };
        (&mut self.model, clusters)
    }

    /// The axis-swept policy's private maneuver state (panics if the scratch
    /// body runs a different policy). Test ergonomics for asserting/arranging
    /// model-private facts.
    pub fn axis(&self) -> &crate::movement::AxisManeuverState {
        match &self.model {
            crate::movement::MotionModel::AxisSwept(axis) => &axis.state,
            other => panic!("scratch body is not axis-swept: {other:?}"),
        }
    }

    /// Mutable flavor of [`Self::axis`].
    pub fn axis_mut(&mut self) -> &mut crate::movement::AxisManeuverState {
        match &mut self.model {
            crate::movement::MotionModel::AxisSwept(axis) => &mut axis.state,
            other => panic!("scratch body is not axis-swept: {other:?}"),
        }
    }

    pub fn as_mut(&mut self) -> BodyClustersMut<'_> {
        BodyClustersMut {
            abilities: &self.abilities,
            kinematics: &mut self.kinematics,
            // Scratch is the non-ECS test scratchpad; it carries no sample
            // (tests that observe the sample set `clusters.sweep` on the
            // borrowed view directly).
            sweep: None,
            base_size: &mut self.base_size,
            ground: &mut self.ground,
            wall: &mut self.wall,
            jump: &mut self.jump,
            dash: &mut self.dash,
            flight: &mut self.flight,
            blink: &mut self.blink,
            ledge: &mut self.ledge,
            dodge: &mut self.dodge,
            shield: &mut self.shield,
            body_mode: &mut self.body_mode,
            env_contact: &mut self.env_contact,
            mana: &mut self.mana,
            offense: &mut self.offense,
            action_buffer: &mut self.action_buffer,
            lifetime: &mut self.lifetime,
            combo_trace: &mut self.combo_trace,
        }
    }
}

#[cfg(test)]
mod reset_tests {
    use super::*;

    /// A reset RESTORES the body to its base size; it does not REDEFINE what
    /// the base is.
    ///
    /// `base_size` is identity-derived — a worn form, a mount, a boss phase — and only the
    /// authority owning that identity may write it.
    ///
    /// Her own `sync_grown_form` could not repair it, because it decides "am I in sync" by
    /// comparing worn equipment against `WornCharacter` — both of which the reset left tall — and
    /// never looks at the collider.
    #[test]
    fn a_reset_restores_the_bodys_own_base_size_not_the_global_default() {
        let grown = Vec2::new(30.0, 72.0);
        let mut scratch = BodyClusterScratch::new_with_abilities(
            Vec2::new(400.0, 400.0),
            crate::abilities::AbilitySet::default(),
        );
        // An identity authority (Mary-O's `sync_grown_form`) grew her.
        scratch.kinematics.size = grown;
        scratch.base_size.base_size = grown;
        // ...and something transient shrank the live collider, as a crouch would.
        scratch.kinematics.size = Vec2::new(30.0, 40.0);

        let spawn = Vec2::new(64.0, 352.0);
        let (model, mut clusters) = scratch.parts();
        reset_body_clusters(
            model,
            &mut clusters,
            spawn,
            crate::movement::DEFAULT_TUNING.air_jumps,
        );

        assert_eq!(
            scratch.base_size.base_size, grown,
            "a reset must not redefine the body's identity size"
        );
        assert_eq!(
            scratch.kinematics.size, grown,
            "and it restores the collider TO that identity size, undoing the \
             transient crouch — not to the global player default"
        );
    }

    /// The other half: a body that never took an identity size still resets to
    /// the ordinary player default, so nothing that does not grow changes.
    #[test]
    fn a_body_with_no_identity_size_still_resets_to_the_player_default() {
        let default = crate::movement::default_player_body_size();
        let mut scratch = BodyClusterScratch::new_with_abilities(
            Vec2::new(400.0, 400.0),
            crate::abilities::AbilitySet::default(),
        );
        scratch.kinematics.size = Vec2::new(30.0, 40.0);

        let spawn = Vec2::new(64.0, 352.0);
        let (model, mut clusters) = scratch.parts();
        reset_body_clusters(
            model,
            &mut clusters,
            spawn,
            crate::movement::DEFAULT_TUNING.air_jumps,
        );

        assert_eq!(scratch.kinematics.size, default);
        assert_eq!(scratch.base_size.base_size, default);
    }

    /// A reset restores the LIVE air-jump count, not the engine default.
    ///
    /// This was `DEFAULT_TUNING.air_jumps` inside the function, so a body under
    /// non-default tuning came back from every reset with default jumps. Four of
    /// five call sites knew and followed with
    /// `refresh_movement_resources_clusters`; one did not, and nothing said so.
    /// An authority that requires a follow-up call is not an authority.
    #[test]
    fn a_reset_restores_the_air_jumps_the_caller_names() {
        // `air_jump_count` returns 0 for a body that cannot double-jump at all,
        // so the tuning only speaks through an ability set that grants one — a
        // fixture without it would pass under the bug and prove nothing.
        let mut abilities = crate::abilities::AbilitySet::default();
        abilities.double_jump = true;
        let mut scratch = BodyClusterScratch::new_with_abilities(Vec2::ZERO, abilities);
        let generous = crate::movement::DEFAULT_TUNING.air_jumps + 3;
        let (model, mut clusters) = scratch.parts();
        reset_body_clusters(model, &mut clusters, Vec2::ZERO, generous);
        assert_eq!(
            scratch.jump.air_jumps_available, generous,
            "the reset restored the engine default over the tuning the caller \
             named, which is the bug that made every call site carry a follow-up"
        );
    }

    /// Any reset announces itself. The seven production callers of
    /// `reset_body_clusters` do not know `BodyRestarted` exists, and must not
    /// have to: the flag is raised by the reset, not by the caller.
    #[test]
    fn resetting_a_body_leaves_a_restart_to_announce() {
        let mut scratch = BodyClusterScratch::new_with_abilities(
            Vec2::ZERO,
            crate::abilities::AbilitySet::default(),
        );
        assert!(!scratch.lifetime.restart_pending);
        let (model, mut clusters) = scratch.parts();
        reset_body_clusters(
            model,
            &mut clusters,
            Vec2::new(10.0, 20.0),
            crate::movement::DEFAULT_TUNING.air_jumps,
        );
        assert!(scratch.lifetime.restart_pending);
    }

    /// ...and the pending flag becomes exactly one trigger, then stops.
    #[test]
    fn a_pending_restart_is_announced_once() {
        use bevy_ecs::prelude::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let mut world = World::new();
        let seen = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&seen);
        world.add_observer(move |_: On<BodyRestarted>| {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        let body = world
            .spawn(BodyLifetime {
                restart_pending: true,
                ..Default::default()
            })
            .id();
        // A second body with nothing pending: the announcement is per-body, and
        // a sweep that told every body it had restarted would be worse than one
        // that told nobody.
        world.spawn(BodyLifetime::default());

        let mut system = bevy_ecs::system::IntoSystem::into_system(announce_body_restarts);
        system.initialize(&mut world);
        system.run((), &mut world).expect("the announcer runs");
        world.flush();
        assert_eq!(seen.load(Ordering::Relaxed), 1);
        assert!(!world.get::<BodyLifetime>(body).unwrap().restart_pending);

        system.run((), &mut world).expect("the announcer runs");
        world.flush();
        assert_eq!(
            seen.load(Ordering::Relaxed),
            1,
            "the flag was not cleared, so every later tick re-announces a \
             restart that already happened"
        );
    }
}

#[cfg(test)]
mod shield_resource_tests {
    use super::*;
    use crate::ShieldTuning;

    const FIGHTER: ShieldTuning = ShieldTuning::PLATFORM_FIGHTER;
    const DT: f32 = 1.0 / 60.0;

    fn guarding() -> BodyShieldState {
        BodyShieldState {
            active: true,
            ..Default::default()
        }
    }

    /// HOLDING THE GUARD SPENDS IT, AND SPENDING IT ALL BREAKS IT.
    #[test]
    fn a_held_guard_drains_until_it_breaks() {
        let mut shield = guarding();
        let mut seconds = 0.0;
        while !shield.broken() && seconds < 30.0 {
            tick_shield_resource(&mut shield, FIGHTER, DT);
            seconds += DT;
        }
        assert!(
            shield.broken(),
            "a guard held for 30s never broke — it is not a resource"
        );
        assert!(
            seconds > 1.0,
            "the guard broke in {seconds}s, which is a flicker rather than a resource"
        );
        assert!(!shield.active, "a broken guard was still up");
        // The breaking tick spends its own frame of the dizzy, so the armed
        // value is one tick short of the authored one rather than equal to it.
        assert!(
            (FIGHTER.break_stun_time - shield.break_timer).abs() <= DT * 1.5,
            "the dizzy was armed at {} against an authored {}",
            shield.break_timer,
            FIGHTER.break_stun_time
        );
    }

    /// A BROKEN GUARD CANNOT COME BACK UP, AND COMES BACK WHOLE.
    #[test]
    fn a_break_locks_the_guard_out_and_then_restores_it_fully() {
        let mut shield = guarding();
        break_shield(&mut shield, FIGHTER);
        assert_eq!(shield.integrity_fraction(FIGHTER), 0.0);

        let mut elapsed = 0.0;
        while shield.broken() {
            // The dizzy is the ONE thing that runs while broken: no regeneration
            // crawls the guard back before the body may act.
            assert_eq!(shield.depleted, FIGHTER.max_health);
            tick_shield_resource(&mut shield, FIGHTER, DT);
            elapsed += DT;
        }
        assert!(
            (elapsed - FIGHTER.break_stun_time).abs() < 0.05,
            "the dizzy ran {elapsed}s against an authored {}s",
            FIGHTER.break_stun_time
        );
        assert_eq!(
            shield.integrity_fraction(FIGHTER),
            1.0,
            "the guard came back damaged, so a break is a permanent tax"
        );
    }

    /// A BLOCKED HIT COSTS INTEGRITY, AND A BIG ENOUGH ONE BREAKS THE GUARD.
    #[test]
    fn blocking_spends_the_guard_in_proportion_to_the_damage() {
        let mut small = guarding();
        spend_shield_on_block(&mut small, FIGHTER, 5);
        let mut large = guarding();
        spend_shield_on_block(&mut large, FIGHTER, 20);
        assert!(
            large.depleted > small.depleted,
            "a 20-damage hit cost the guard no more than a 5-damage one"
        );
        assert!(!small.broken() && !large.broken());

        let mut poked = guarding();
        spend_shield_on_block(&mut poked, FIGHTER, FIGHTER.max_health as i32 + 1);
        assert!(
            poked.broken(),
            "a hit bigger than the whole guard did not break it"
        );
    }

    /// A BODY WITH NO SHIELD RESOURCE IS UNTOUCHED BY ALL OF IT.
    ///
    /// the poison this pins: every body in the game carries a
    /// `BodyShieldState` and almost none authors a [`ShieldTuning`], so a rule
    /// that ran on the DEFAULT tuning would break every exploration guard in the
    /// game on its first frame.
    #[test]
    fn an_unlimited_guard_never_drains_spends_or_breaks() {
        let mut shield = guarding();
        for _ in 0..600 {
            tick_shield_resource(&mut shield, ShieldTuning::OFF, DT);
        }
        spend_shield_on_block(&mut shield, ShieldTuning::OFF, 999);
        assert_eq!(shield.depleted, 0.0);
        assert!(!shield.broken());
        assert!(shield.active, "an unlimited guard was dropped");
        assert_eq!(shield.integrity_fraction(ShieldTuning::OFF), 1.0);
    }

    /// LETTING GO REGENERATES, AND ONLY UP TO WHOLE.
    #[test]
    fn a_dropped_guard_regenerates_and_stops_at_full() {
        let mut shield = guarding();
        spend_shield_on_block(&mut shield, FIGHTER, 20);
        let hurt = shield.depleted;
        shield.active = false;
        for _ in 0..30 {
            tick_shield_resource(&mut shield, FIGHTER, DT);
        }
        assert!(shield.depleted < hurt, "a dropped guard did not recover");
        for _ in 0..600 {
            tick_shield_resource(&mut shield, FIGHTER, DT);
        }
        assert_eq!(shield.depleted, 0.0, "regeneration overshot past whole");
    }
}

#[cfg(test)]
mod shieldstun_tests {
    use super::*;
    use crate::ShieldTuning;

    /// BLOCKING COSTS THE BLOCKER TIME, IN PROPORTION TO WHAT IT BLOCKED.
    ///
    /// without this a guard is free: the defender eats a smash attack and acts
    /// on the next frame, so there is nothing to punish and no reason to swing.
    #[test]
    fn a_blocked_hit_charges_shieldstun_and_it_runs_out() {
        let t = ShieldTuning::PLATFORM_FIGHTER;
        let mut light = BodyShieldState {
            active: true,
            ..Default::default()
        };
        spend_shield_on_block(&mut light, t, 4);
        let mut heavy = BodyShieldState {
            active: true,
            ..Default::default()
        };
        spend_shield_on_block(&mut heavy, t, 18);
        assert!(
            light.stun_timer > 0.0,
            "a blocked hit charged no shieldstun"
        );
        assert!(
            heavy.stun_timer > light.stun_timer,
            "an 18-damage hit cost the blocker no more time than a 4-damage one"
        );

        let mut elapsed = 0.0;
        while heavy.stun_timer > 0.0 && elapsed < 5.0 {
            tick_shield_resource(&mut heavy, t, 1.0 / 60.0);
            elapsed += 1.0 / 60.0;
        }
        assert_eq!(heavy.stun_timer, 0.0, "shieldstun never expired");
    }

    /// AND A BODY WITH NO SHIELD RESOURCE STILL BLOCKS FOR FREE.
    #[test]
    fn an_unlimited_guard_owes_no_shieldstun() {
        let mut shield = BodyShieldState {
            active: true,
            ..Default::default()
        };
        spend_shield_on_block(&mut shield, ShieldTuning::OFF, 30);
        assert_eq!(shield.stun_timer, 0.0);
    }
}

#[cfg(test)]
mod fresh_budget_tests {
    use super::*;

    /// ⛔⛔ `Default` IS THE SPENT STATE, AND A FRESH BODY IS NOT SPENT.
    ///
    /// Both fresh-construction paths spelled the jump cluster as
    /// `air_jumps_available: n, ..Default::default()`, which quietly gave every
    /// fresh body ZERO recovery charges. It did not show up anywhere that lands:
    /// the landing-class refresh fills the budget, so any body that touched the
    /// floor was immediately correct. It shows up on the one road that does NOT
    /// land — a stock respawn puts the fighter in the AIR and runs no refresh
    /// after the reset, so the returning fighter's recovery special was refused
    /// before it had used one.
    ///
    /// ⭐ BOTH PATHS, because they were two copies of the same mistake and
    /// fixing one leaves the other. And `Default` is asserted to still be spent:
    /// that is what a test means when it says "this body has nothing left", and
    /// making `Default` full would fix this by breaking that.
    #[test]
    fn a_fresh_body_starts_with_its_recovery_and_default_stays_spent() {
        assert!(
            DEFAULT_RECOVERY_CHARGES > 0,
            "poison: with a zero default every assertion here holds trivially"
        );
        assert_eq!(
            BodyJumpState::default().recovery_charges,
            0,
            "`Default` stopped meaning SPENT — a fixture that wanted an empty \
             budget now silently gets a full one"
        );
        assert_eq!(
            BodyJumpState::fresh(2).recovery_charges,
            DEFAULT_RECOVERY_CHARGES
        );
        assert_eq!(BodyJumpState::fresh(2).air_jumps_available, 2);

        // The scratch path: a body built for a test or a fixture is a body that
        // has just been created, not one that has spent everything.
        let scratch =
            BodyClusterScratch::new_with_abilities(Vec2::ZERO, crate::AbilitySet::sandbox_all());
        assert_eq!(
            scratch.jump.recovery_charges, DEFAULT_RECOVERY_CHARGES,
            "a freshly constructed body has no recovery"
        );
    }

    /// ⭐⭐ THE AERIAL RESOURCES ARE ANSWERED BY THE FLOOR; THE RECOVERY IS
    /// ANSWERED BY LANDING — and the difference only shows for a resource a
    /// fighter can spend WHILE STANDING ON IT.
    ///
    /// ⛔ THE WITHHELD ARM IS NOT "REFRESH NOTHING". If it were, this could be
    /// implemented by skipping the call, and a grounded fighter mid-recovery
    /// would also keep a spent air jump and a spent air dodge — neither of which
    /// this rule is about. Both halves are asserted in the same arm precisely so
    /// a future "just don't call it" simplification fails here.
    #[test]
    fn a_withheld_refresh_answers_for_the_air_and_not_for_the_recovery() {
        let abilities = BodyAbilities {
            abilities: crate::AbilitySet::sandbox_all(),
        };
        let spent = |recovery: RecoveryRefresh| {
            let mut dash = BodyDashState {
                charges_available: 0,
                ..Default::default()
            };
            let mut jump = BodyJumpState {
                air_jumps_available: 0,
                recovery_charges: 0,
                post_recovery_helpless: true,
                ..Default::default()
            };
            let mut dodge = BodyDodgeState {
                air_dodge_spent: true,
                ..Default::default()
            };
            refresh_movement_resources_clusters(
                &abilities, &mut dash, &mut jump, &mut dodge, 2, recovery,
            );
            (dash.charges_available, jump, dodge.air_dodge_spent)
        };

        let (dash_answered, jump_answered, dodge_answered) = spent(RecoveryRefresh::Answered);
        assert_eq!(jump_answered.recovery_charges, DEFAULT_RECOVERY_CHARGES);
        assert!(!jump_answered.post_recovery_helpless);
        assert_eq!(jump_answered.air_jumps_available, 2);
        assert!(!dodge_answered);
        assert!(
            dash_answered > 0,
            "poison: this body's kit grants no dash charges, so the withheld arm              below would agree with the answered one for the wrong reason"
        );

        let (dash_withheld, jump_withheld, dodge_withheld) = spent(RecoveryRefresh::Withheld);
        assert_eq!(
            jump_withheld.recovery_charges, 0,
            "the floor answered for a recovery whose move has not finished"
        );
        assert!(
            jump_withheld.post_recovery_helpless,
            "the episode ended without the resource that opened it coming back"
        );
        assert_eq!(
            (dash_withheld, jump_withheld.air_jumps_available, dodge_withheld),
            (dash_answered, 2, false),
            "withholding the recovery also withheld the AERIAL resources — this              rule is about the one thing a fighter spends with its feet down"
        );
    }
}
