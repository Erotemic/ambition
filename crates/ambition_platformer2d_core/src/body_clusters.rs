//! Authoritative **body** cluster types — the movement aggregate every actor
//! carries, the player included (NOT player-specific).
//!
//! Each Bevy `Component` carries one tightly related slice of body state
//! (kinematics, ground contact, dash charges, …). Together they form the
//! authoritative movement aggregate every body — player, enemy, NPC, boss —
//! shares. The clusters hold SHARED facts: contact facts the collision
//! doctrine writes and preserved resources/cooldowns. Policy-PRIVATE maneuver
//! state lives inside the body's `MotionModel` variant (ADR 0024 — see
//! [`crate::movement::AxisManeuverState`]).
//!
//! [`BodyClustersMut`] is a struct-of-`&mut` view assembled from a
//! `Query<BodyClusterQueryData, …>::as_clusters_mut()` call; every
//! engine entry point in `crate::movement` takes one.
//! Tests that need a non-ECS scratchpad construct
//! [`BodyClusterScratch::new_with_abilities`] and re-borrow via
//! `BodyClusterScratch::as_mut`.

use crate::abilities::AbilitySet;
use crate::movement::ComboMark;
use crate::player_state::{BodyMode, ResourceMeter};
use crate::world::{ClimbableContact, WaterContact};
use crate::Vec2;

/// Mutable cluster references aggregated for the engine
/// `update_player_*_with_clusters` entry points.
///
/// Holding the 18 cluster refs in a struct keeps the entry-point
/// signatures from accumulating 20+ positional parameters and lets
/// sandbox callers build the view from a Bevy query without going
/// through a separate bridge module.
pub struct BodyClustersMut<'a> {
    pub abilities: &'a BodyAbilities,
    pub kinematics: &'a mut BodyKinematics,
    /// The §3.1 per-tick motion record, written by the simulation phase
    /// itself (see [`SweepSample`]). `Option` so legacy bodies, scratch
    /// tests, and non-pipeline movers opt in incrementally — an absent
    /// sample means swept readers fall back to their historical
    /// `vel·dt` approximation.
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

/// Position, velocity, AABB size, and facing direction of a body.
///
/// The foundational body state every controllable body in the platformer
/// shares: the player, enemies/NPCs, and bosses all carry one. It replaces
/// the three historical parallel types (`PlayerKinematics`,
/// `ActorKinematics`, `BossKinematics`) so any code that operates on "a body"
/// (orientation, transit, vortex, brain effects, …) holds ONE query instead
/// of branching across three.
///
/// The player shares this unified component with enemies / NPCs / bosses. The
/// player-only "base / standing body size" lives separately on
/// [`BodyBaseSize`] so the shared component stays minimal.
///
/// ## Query-conflict discipline
///
/// Because player, enemy, and boss entities all carry `BodyKinematics`, any
/// single system that holds more than one `&mut BodyKinematics` query (or a
/// `&mut` query alongside another that can alias the same entity) must make the
/// queries provably disjoint with marker filters (player / enemy / boss are
/// mutually exclusive archetypes). Handle the conflict with filters, never by
/// re-splitting the component.
///
/// Bosses float and never integrate `vel` themselves (the brain emits a fresh
/// `desired_vel` each tick for `integrate_body`), so a boss simply leaves
/// `vel` at [`Vec2::ZERO`].
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

/// The canonical per-tick MOTION RECORD (collision doctrine §3.1) — the
/// simulation phase's own integration segment, written by the kernel at
/// the end of `update_body_simulation_with_clusters` (both endpoints
/// captured INSIDE the kernel). Because `prev` is the position at
/// sim-phase ENTRY, every teleport in the engine — blink (control
/// phase), the player respawn wrapper (after sim), portal transfer /
/// room transition / scripted warps (other systems) — is excluded from
/// the record BY CONSTRUCTION: a position change outside the sim phase
/// simply never becomes path. No reset protocol exists or is needed.
///
/// Swept readers (hazard touch, CC6's relative portal sweep) consume
/// `prev → curr`; bodies without the component (legacy spawns, scratch
/// tests, movers not yet writing it) fall back to the historical
/// `vel·dt` approximation at the read site.
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
            contact_initialized: false,
        }
    }

    /// Invalidate contact facts after a discrete pose change. The stale value
    /// is cleared for readers outside the kernel; the next movement step
    /// establishes a new baseline from world geometry.
    pub fn invalidate(&mut self) {
        self.on_ground = false;
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
/// are body-mode (climbing) mechanics state owned outside the movement policy.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyJumpState {
    pub air_jumps_available: u8,
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
    /// **How long this carry is still OWED to the body**, in seconds.
    ///
    /// The floor exists because momentum the world imparted must not bleed away
    /// under the hands-off air stop assist. But that argument has an expiry: it
    /// is momentum you were given *while you could not act*, so it stops being
    /// owed the moment you can. Without a bound, one knockback makes a body
    /// coast at its launch speed for the rest of its airtime and the tight
    /// stop-on-release feel is gone for everyone who was ever hit (queue F0e).
    ///
    /// Zero means the ordinary rule: `carried_run` bleeds at
    /// `MovementTuning::carried_decay` like it always did. A portal fling leaves
    /// this at zero deliberately — a fling is a change of reference frame, not a
    /// reaction you are locked out of, and it has always decayed that way.
    pub carried_hold: f32,
    /// **A world-space launch an external reaction imparted, not yet handed to
    /// the motion model.** `Vec2::ZERO` between hits, which is almost always.
    ///
    /// ⭐ **why this exists at all**: writing a launch into
    /// `BodyKinematics::vel` is only authoritative for a model that OWNS `vel`.
    /// A surface-momentum body that is `Riding` does not — its `vel` is
    /// documented as *"DERIVED (published for observers)"*, computed from the
    /// scalar `v_t` along the tangent — so a knockback written there was
    /// republished away on the next step and Sanic took hits without moving.
    /// The impulse has to reach the model, and only the model can decide what a
    /// launch MEANS to it (leave the surface, or override the run).
    ///
    /// ⚠ **drained by [`step_motion`](crate::movement::step_motion), the single
    /// movement gateway, and by nothing else.** Putting the drain there rather
    /// than asking each writer to also call the model is deliberate: this repo
    /// has repeatedly been bitten by an authority that needs a follow-up call,
    /// and a launch that silently did nothing is exactly that failure.
    ///
    /// ⚠ **it rides on `BodyFlightState` because that cluster is ALREADY the
    /// world-imparted momentum channel** (see `carried_run` above, written by
    /// the portal adapter and by knockback) and is already rollback-registered
    /// as `body.flight`. A pending launch that did not rewind would be a phantom
    /// hit on the resimulated timeline; here it rewinds with everything else and
    /// needs no new registration.
    ///
    /// ⚠ **`Vec2::ZERO` is the empty state rather than an `Option`**, for the
    /// snapshot's sake: the encoder has a `vec2` primitive and no `Option<Vec2>`
    /// one, and inventing an option encoding to express "no launch" would put a
    /// new shape in the wire format for a case that already has a harmless
    /// spelling. A launch of exactly zero is a no-op either way — the kernel's
    /// own `apply_pad_impulse` has always early-returned on
    /// `length_squared() <= 1e-6` for the same reason.
    pub pending_launch: crate::Vec2,
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
}

/// Shield/parry cluster.
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyShieldState {
    pub active: bool,
    pub parry_window_timer: f32,
}

impl BodyShieldState {
    pub fn parrying(self) -> bool {
        self.active && self.parry_window_timer > 0.0
    }
}

/// **"This body starts again."** — the half of a reset the engine cannot perform.
///
/// [`reset_body_clusters`] clears every cluster the ENGINE owns, and a ruleset
/// that calls it can honestly say the movement, ability, shield and dodge state
/// are gone. It cannot say the body is clean, because a character provider may
/// attach authoritative state of its own — Sanic's ball-dash charge and his
/// `Rolling` marker, Mary-O's spark cadence — and a generic rule knows neither
/// the types nor what resetting them means. The versus round boundary claimed
/// "the whole movement/ability cluster set", which was true, and read as "the
/// fighter starts clean", which was not (GPT 5.6, 2026-07-27).
///
/// So the reset ANNOUNCES itself and each provider answers for its own state.
/// An entity event rather than a message: it fires at the command flush inside
/// the tick that reset the body, so no provider has to find a schedule slot
/// between "the round restarted" and "my systems run again" — the ordering bug
/// this would otherwise hand to every character author in turn.
///
/// Observing it costs a provider one line and is inert for any body that lacks
/// its components:
///
/// ```ignore
/// app.add_observer(|restart: On<BodyRestarted>, mut dashes: Query<&mut BallDash>| {
///     if let Ok(mut dash) = dashes.get_mut(restart.entity) {
///         *dash = BallDash::default();
///     }
/// });
/// ```
///
/// ## Every reset, not just the polite ones
///
/// This shipped as a versus-only event: `begin_round` triggered it by hand and
/// the seven other production callers of [`reset_body_clusters`] — death
/// respawn, safe respawn, room arrival, sandbox reset, lifecycle commit — did
/// not, so ordinary play recreated exactly the leak the event was added to
/// close. Sanic could respawn holding a ball-dash charge; the observers existed
/// and nothing invoked them (GPT 5.6, 2026-07-28).
///
/// A doc comment cannot make seven call sites remember. So the announcement is
/// DERIVED: [`reset_body_clusters`] raises [`BodyLifetime::restart_pending`] on
/// state it already owns, and [`announce_body_restarts`] turns that into this
/// trigger once per tick. A caller that has never heard of this type announces
/// correctly, and a caller added next year does too.
#[derive(bevy_ecs::event::EntityEvent, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyRestarted {
    /// The body starting again.
    pub entity: bevy_ecs::entity::Entity,
}

/// Turn every pending restart into a [`BodyRestarted`] trigger, once.
///
/// Registered at the FRONT of the sim tick, so a reset performed anywhere —
/// mid-tick in a combat phase, or outside the sim schedule entirely by a room
/// load — is announced before any system acts on that body again. The flag is
/// cleared here and nowhere else, so the announcement happens exactly once per
/// reset however many phases the reset passed through.
///
/// Ordinary `Commands`, so the observers run at the next command flush rather
/// than reentrantly inside this query.
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
/// ⚠ **it is a PARAMETER because it used to be `DEFAULT_TUNING.air_jumps`**, and
/// that made this function quietly wrong for anybody running non-default tuning.
/// The sandbox reset knew, and carried a follow-up call:
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
/// nothing saying so. **An authority that requires a follow-up call is not an
/// authority** — it is a two-step ritual, and the second step is the one people
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
    *clusters.jump = BodyJumpState {
        air_jumps_available: air_jumps,
        ladder_jump_boost: 0.0,
        ladder_drop_through_timer: 0.0,
        ladder_drop_through_hold_lock: false,
    };
    *clusters.dash = BodyDashState {
        charges_available: dash_charges,
        ..Default::default()
    };
    *clusters.flight = BodyFlightState::default();
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
        // The announcement is DERIVED from the reset, not asked of the caller.
        // Seven production call sites reset a body — death, safe respawn, room
        // arrival, sandbox reset, lifecycle commit, a versus round — and exactly
        // one of them remembered to tell the providers, which is the natural
        // outcome of a contract that lives in a doc comment (GPT 5.6,
        // 2026-07-28). A flag on state this function already owns cannot be
        // forgotten by a caller that does not know it exists.
        restart_pending: true,
        ..Default::default()
    };
    clusters.combo_trace.combo.clear();
    clusters.combo_trace.combo.push(ComboMark {
        op: MovementOp::Reset,
        age: 0.0,
    });
}

/// Refresh the dash charge count and air-jump count from the active
/// `BodyAbilities` + the caller's authored base air-jump count.
pub fn refresh_movement_resources_clusters(
    abilities: &BodyAbilities,
    dash: &mut BodyDashState,
    jump: &mut BodyJumpState,
    base_air_jumps: u8,
) {
    dash.charges_available = abilities.abilities.dash_charge_count();
    jump.air_jumps_available = abilities.abilities.air_jump_count(base_air_jumps);
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

/// ECS-owned COMBAT action buffer (attack / pogo / projectile press windows).
/// The MOVEMENT buffers (jump / dash / blink) are axis-policy maneuver state
/// ([`crate::movement::AxisManeuverState::buffer_jump`] and siblings).
#[derive(bevy_ecs::component::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyActionBuffer {
    pub attack: f32,
    pub pogo: f32,
    pub projectile: f32,
}

impl BodyActionBuffer {
    pub fn tick(&mut self, dt: f32) {
        for slot in [&mut self.attack, &mut self.pogo, &mut self.projectile] {
            *slot = (*slot - dt).max(0.0);
        }
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
            jump: BodyJumpState {
                air_jumps_available: air_jumps,
                ladder_jump_boost: 0.0,
                ladder_drop_through_timer: 0.0,
                ladder_drop_through_hold_lock: false,
            },
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
    /// `base_size` is identity-derived — a worn form, a mount, a boss phase —
    /// and only the authority owning that identity may write it. This used to
    /// hardcode `default_player_body_size()` into both fields, which silently
    /// unmade any identity-driven size on every reset.
    ///
    /// The bug that found this: a grown Super Mary-O who fell in a pit came
    /// back with a SMALL collider while still wearing the cap and still
    /// presenting the tall sprite. Her own `sync_grown_form` could not repair
    /// it, because it decides "am I in sync" by comparing worn equipment
    /// against `WornCharacter` — both of which the reset left tall — and never
    /// looks at the collider. Hitbox and identity disagreed, invisibly, until
    /// the next hit landed on a body half the size of the one on screen.
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

    /// **A reset restores the LIVE air-jump count, not the engine default.**
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

    /// **Any reset announces itself.** The seven production callers of
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
