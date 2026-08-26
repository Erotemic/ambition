//! THE MOUNT PAIR: two linked bodies where one carries the other.
//!
//! ⭐⭐ CARVED OUT OF `ambition_platformer2d_actor_monolith` (D33). What made it
//! carvable was not line count: this module reached the monolith four ways and
//! all four were removed first — the dismount brain rebuild became an answer to
//! the `MountDied` this crate already announces, `ResolvedMotionFrame` turned
//! out to live in `shared_tangle` behind a re-export, `TemporaryControl` moved
//! to `shared_tangle` beside `Mass`, and the twenty-six-column
//! `ActorClusterQueryData` was replaced by the FIVE columns these systems touch.
//!
//! ⛔ THE MONOLITH STILL BUILDS PAIRS AND THAT IS CORRECT. 106 references to
//! these components live in its construction and spawn roads; after the carve
//! they spell `ambition_mount::` instead of `crate::features::` and compile
//! unchanged. An inward edge is a caller naming a domain, not a dependency the
//! domain has.
//!
//! Generic rider/mount relationship between separate actor entities.
//!
//! `RidingOn` links the rider to a `MountSlot`; per-tick coupling publishes the
//! rider pose from the mount while the rider brain may still author actions.
//! Link enforcement handles either side dying: a dead mount releases the rider
//! into its dismounted behavior, while a dead rider leaves the mount independent.
//! Authored pairs are lowered from linked placements into the same engine-owned
//! relation; the engine contains no mount-species special case.

use bevy::prelude::{Commands, Component, Entity, MessageWriter, Query, With, Without};

use ambition_platformer2d_core as ae;
// ⛔ NAMED FROM ITS REAL OWNER. It was `use super::CenteredAabb` while this
// module lived in the monolith, which made a monolith module look like the
// source of a geometry type that `ambition_geometry` defines and `_core`
// re-exports. Same rule the `MountDied` note below states: spell whose type it
// is.
use ae::CenteredAabb;

// `MountDied` — the `(dead-mount, still-mounted)` dissolution
// [`enforce_mount_rider_link`] announces — lives in
// `ambition_platformer2d_shared_tangle::body`, below both of the domains that
// share it: this coupling WRITES it and `ambition_boss_encounter` READS it.
// imported, never re-exported: a `pub use` here would let a caller keep
// spelling it `features::MountDied` and hide whose type it is.
use ambition_platformer2d_shared_tangle::body::MountDied;
// ⛔ AND `Mass` MOVED FOR THE SAME REASON AND BY THE SAME RULE, 2026-08-26. It
// was defined here — a generic physics fact with 27 users outside this module,
// every one of them spelling it `ambition_platformer2d_shared_tangle::body::Mass`, which is a body's
// weight wearing one mechanic's address. It stays in `shared_tangle` on the
// strength of those 27 users.
//
// ⛔⛔ BUT THE SECOND HALF OF THAT ARGUMENT IS GONE, AND SAYING SO IS THE POINT.
// The move was justified here as *"the writer is the character runtime's
// physical baseline and the reader is the mass-weighted centre below"* — and
// that reader was the saddle's centre-of-gravity term, which turned out to be a
// no-op under default gravity and a bug under rotated gravity (see
// `saddle_world_offset`). Deleting it left this crate reading `Mass` NOWHERE, so
// the mount domain is not one of the two domains that share it. The conclusion
// survives its own reasoning by accident, which is worth a sentence rather than
// a quiet edit.
use ambition_platformer2d_core::snapshot::RollbackRegistrar;
use ambition_platformer2d_shared_tangle::body::SpawnBaseline;

/// A mount's *class* — the content-defined category a rider must be
/// allowed to pilot (a shark-rider cannot pilot a mech). The engine
/// enumerates no classes; they are pure content strings (`"shark"`,
/// `"mech"`, `"horse"`), matched against a rider's [`CanPilot`] set.
/// See ADR 0020.
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct MountClass(pub String);

impl MountClass {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// How much of the rider's control intent the mount actually obeys while
/// ridden. The mount's own brain *defers* to the rider through this grant
/// (ADR 0020). The default — and the only variant implemented today — is
/// [`ControlGrant::Total`]: the rider drives fully. Partial/disobedient
/// grants (a skittish horse, an unstable mech that drops or distorts
/// intent) are a reserved seam: add variants here when content needs them.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlGrant {
    /// Rider intent passes straight through — the mount fully obeys.
    #[default]
    Total,
    // Future: Partial { .. }, Skittish { .. }, Locked { .. } — see ADR 0020.
}

/// What happens to the *rider* when its mount dies. Two actors, two health
/// pools: by default a dead mount simply drops its rider unharmed
/// ([`MountDeathImpact::Dismount`]). A mount that should hurt its rider on
/// death — a mech that explodes — authors [`MountDeathImpact::Splash`] with
/// the damage the rider takes (large enough is lethal). See ADR 0020.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MountDeathImpact {
    /// Rider is unharmed and simply dismounts (the default).
    #[default]
    Dismount,
    /// Rider takes this much damage when the mount dies.
    Splash(i32),
}

/// Attached to a mount entity. Specifies where the rider rides
/// relative to the mount's center (sandbox units; y grows downward),
/// the mount's [`MountClass`], the [`ControlGrant`] it extends its
/// rider, and its [`MountDeathImpact`].
#[derive(Component, Clone, Debug)]
pub struct Mountable {
    /// Rider's center offset from the mount's center, in the MOUNT'S OWN
    /// FRAME: `+x` toward the side the mount faces, `+y` toward its feet. For an
    /// aerial mount this is typically `(0, -mount.size.y * 0.5 -
    /// rider.size.y * 0.5 + epsilon)` so the rider sits on the
    /// mount's saddle without their hitboxes overlapping.
    ///
    /// ⭐ FACING-RELATIVE `x` BECAUSE THE SADDLE IS A PLACE ON THE MOUNT. The
    /// constraint already gives the rider the mount's facing, so an offset that
    /// did not mirror would put a rider authored on one shoulder onto the other
    /// the moment the mount turned. See [`saddle_world_offset`].
    pub rider_offset: ae::Vec2,
    /// The mount's class — a rider needs a matching [`CanPilot`] entry.
    pub class: MountClass,
    /// How fully the mount obeys the rider (default `Total`).
    pub control_grant: ControlGrant,
    /// What the rider suffers when this mount dies (default `Dismount`).
    pub death_impact: MountDeathImpact,
}

impl Mountable {
    /// A mount at `rider_offset` with default class / control grant
    /// (`Total`) / death impact (`Dismount`). Callers that author a
    /// specific class or explosion set the fields after.
    pub fn at(rider_offset: ae::Vec2) -> Self {
        Self {
            rider_offset,
            class: MountClass::default(),
            control_grant: ControlGrant::Total,
            death_impact: MountDeathImpact::Dismount,
        }
    }
}

/// Attached to a rider (or would-be rider) entity. The set of mount
/// [`MountClass`]es this actor is allowed to pilot. A shark-rider carries
/// `["shark"]`; it cannot board a `"mech"`-class mount. The engine checks
/// this before establishing a [`RidingOn`] link. See ADR 0020.
#[derive(Component, Clone, Debug, Default)]
pub struct CanPilot {
    pub classes: Vec<MountClass>,
}

impl CanPilot {
    /// Whether a rider carrying this component may pilot `class`.
    pub fn can_pilot(&self, class: &MountClass) -> bool {
        self.classes.contains(class)
    }
}

/// Attached to a mount entity. Holds the rider's `Entity` if one
/// is currently mounted. `None` means the mount is riderless (which
/// is the normal solo state).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct MountSlot {
    pub rider: Option<Entity>,
}

/// Attached to a rider entity. Points at the mount the rider is
/// currently on. The presence of this component is what tells the
/// per-tick sync system to lock the rider's pos to the mount.
///
/// Stays attached even after the mount dies — `sync_riders_to_mounts`
/// checks `mount.alive` each frame and skips the snap for a dead
/// mount. Keeping the link record lets the same-room reset path
/// re-mount the rider without having to look it up by id.
#[derive(Component, Clone, Copy, Debug)]
pub struct RidingOn {
    pub mount: Entity,
}

/// Cache of the rider's MOUNTED brain + action set, attached at
/// composite spawn. Survives mount death (so the rider keeps a
/// record of "what behavior to take if remounted") and is the
/// authority the same-room reset path consults to restore Skirmisher
/// + Bolt firing after the mount comes back alive.
///
/// Without this, a dismounted-then-reset rider would keep their
/// solo melee brain (whatever `enemy_default_brain` returns for the
/// PirateRaider / PirateHeavy archetype) and refuse to fire the
/// gun-sword even while their freshly-respawned shark is alive
/// underneath them.
#[derive(Component, Clone, Debug)]
pub struct MountedBrainCache {
    pub brain: ambition_characters::brain::Brain,
    pub action_set: ambition_characters::brain::ActionSet,
}

/// Tag marker on a rider whose brain is currently in MOUNTED mode
/// (Skirmisher + Bolt). Absent means the rider's brain is its solo
/// archetype default. [`enforce_mount_rider_link`] toggles this
/// marker on alive-transitions of the mount entity.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Mounted;

/// Authored sky-rider collision size. A standalone cove PirateRaider is
/// 44x78 (~125 px tall rendered through the 1.6× pirate sheet
/// collision_scale), but a shark-rider is an authored compact sky variant.
/// Mount state should not change that scale: `sync_riders_to_mounts` snaps the
/// rider to this size while mounted, and the composite spawn path sets
/// `spawn_size` to the same value so the rider keeps it after dismount/reset.
#[derive(Component, Clone, Copy, Debug)]
pub struct MountedSize(pub ae::Vec2);

/// ADR 0020 control routing: the mount defers its locomotion to the rider.
///
/// With `ControlGrant::Total` (the default and only grant today) the mount
/// fully obeys — the RIDER's brain owns the orbit (it is a Skirmisher), and
/// this copies the rider's movement intent onto the mount so the mount body
/// integrates that orbit. The rider's own body movement stays suppressed
/// (`is_mounted`) and it welds to the mount in [`sync_riders_to_mounts`].
/// Attack / fire intent is NOT copied — the rider still fires from the
/// saddle. A riderless mount runs its own brain untouched.
///
/// Runs after `tick_actor_brains` (so the rider's control frame is fresh)
/// and before `integrate_sim_bodies` (so the mount integrates the routed
/// intent). Rider/mount queries are disjoint via `With`/`Without<MountSlot>`.
pub fn steer_mount_from_rider(
    riders: Query<
        (&RidingOn, &ambition_characters::control::ActorControl),
        (With<Mounted>, Without<MountSlot>),
    >,
    mut mounts: Query<
        (&Mountable, &mut ambition_characters::control::ActorControl),
        With<MountSlot>,
    >,
) {
    for (riding, rider_control) in &riders {
        let Ok((mountable, mut mount_control)) = mounts.get_mut(riding.mount) else {
            continue;
        };
        if mountable.control_grant != ControlGrant::Total {
            continue;
        }
        // Total grant → the mount executes the rider's locomotion intent.
        //
        // `locomotion` is CONTROLLED-BODY-LOCAL, so copying it between two
        // bodies is only sound while both resolve the SAME frame. They do
        // today, and not by accident: `sync_riders_to_mounts` zeroes the rider's
        // gravity SCALE and not its direction, so the rider keeps the room's
        // gravity — the same direction the mount resolves `control_down` from.
        // Nothing enforces that, which is why it is written here.
        //
        // the case it would break is named in `actors/update.rs`: *"a surface-walker's frame is its
        // clung surface; everyone else's is gravity at their position."* A surface-walking mount
        // would receive a vector authored in its rider's frame and drive along the wrong axis. ✔
        // not reachable in today's content: the only two authored `mount_class` archetypes are the
        // shark (`is_aerial: true`, so it steers by `velocity_target`, which is WORLD and
        // frame-safe) and the giant ("no motion / no AI — the carried giant just stands"). A new
        // mount with a crawler/adhesive motion model is what makes this live; the fix then is to
        // convert rather than copy, through each body's own basis.
        //
        // `velocity_target` below needs none of this — it is world-space, which
        // is what every OTHER cross-body hand-off in this file already uses.
        let rider_frame = rider_control.0;
        let mount_frame = &mut mount_control.0;
        mount_frame.locomotion = rider_frame.locomotion;
        mount_frame.velocity_target = rider_frame.velocity_target;
        mount_frame.facing = rider_frame.facing;
        // drop-through is NOT hand-copied here, and does not need to be (.2): the rider's
        // descend intent already rides across in `locomotion`, and the jump edge is the mount's
        // own to decide.
    }
}

/// Lock every rider's position / facing / vel / gravity to its
/// mount each tick. Runs after the per-actor brain tick so the
/// rider's brain has had a chance to emit a fire intent against
/// the target from a position close to where it'll actually be
/// after the snap.
///
/// A rider a participant drives (a human piloting the vehicle through possession / the control
/// seam) welds and rides identically to an AI rider; the mount does not care WHO is aboard.
/// Gating on `is_hostile` here would have been exactly the player-centrism the relativity
/// principle forbids — a mount that only obeys enemies.
///
/// The mount queries are disjoint from the rider queries via
/// `With<MountSlot>` / `Without<MountSlot>` so the borrow checker
/// is happy — an entity is either a mount or a rider in this
/// schema, never both. (Even Optimus Prime would be a rider in one
/// composite and a mount in a separate composite; never the same
/// entity playing both roles in one frame.)
pub fn sync_riders_to_mounts(
    mut riders: Query<
        (
            &RidingOn,
            &mut CenteredAabb,
            Option<&MountedSize>,
            // ⭐ THE FOUR COLUMNS THIS SYSTEM ACTUALLY TOUCHES, named one by one
            // instead of borrowed through `ActorClusterQueryData`. That view is
            // twenty-six columns wide and lives in the monolith; every one of
            // these four is owned by `_core` or `ambition_characters`, so the
            // saddle sync now names no monolith type at all.
            //
            // ⛔ IT ALSO WIDENS THE POPULATION, deliberately. The optional view
            // dropped a body that was missing ANY of the twenty-six — a rider
            // lacking, say, `BodyComboTrace` silently got no saddle pin. Nothing
            // about carrying a rider depends on a combo trace.
            &mut ae::BodyKinematics,
            &mut ae::ActorSurfaceState,
            &mut ae::BodyGroundState,
            &ambition_characters::actor::BodyHealth,
        ),
        Without<MountSlot>,
    >,
    mounts: Query<
        (
            &Mountable,
            // The mount's per-tick resolved frame: the saddle offset rotates
            // with the PAIR's reference frame (the rider orbits the mount under
            // a gravity flip instead of floating off the saddle in fixed screen
            // space), and the constraint's frame authority is the carrying body.
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &ae::BodyKinematics,
            &ambition_characters::actor::BodyHealth,
        ),
        With<MountSlot>,
    >,
) {
    for (
        riding,
        mut rider_aabb,
        mounted_size,
        mut rider_kin,
        mut rider_surface,
        mut rider_ground,
        rider_health,
    ) in &mut riders
    {
        let Ok((mountable, mount_frame, mount_kin, mount_health)) =
            mounts.get(riding.mount)
        else {
            continue;
        };
        if !mount_health.alive() || !rider_health.alive() {
            continue;
        }
        // Sky-rider size: keep the authored rider footprint stable while the
        // mount is alive. The same footprint remains after dismount; larger
        // cove pirates are separate authored actor spawns.
        if let Some(size) = mounted_size {
            rider_kin.size = size.0;
        }
        // Snap pose to the mount. Vel zeroed so update_ecs_actors'
        // integrator can't drift the rider off the mount on the
        // next frame; gravity zeroed so a Bevy-side integrator that
        // applies gravity to all hostiles can't pull it down.
        //
        // The saddle offset is authored in the mount's local frame; rotate it into
        // world space by the pair's gravity frame. See `saddle_world_offset`.
        let frame = mount_frame.basis();
        let rider_local =
            saddle_world_offset(mountable.rider_offset, mount_kin.facing, frame);
        // ADR 0020 saddle pin = the external-constraint authority (ADR 0024):
        // the mount owns the rider's pose while mounted.
        ae::movement::constrain_body_pose(
            &mut rider_kin,
            mount_kin.pos + rider_local,
            ae::Vec2::ZERO,
        );
        rider_kin.facing = mount_kin.facing;
        rider_surface.gravity_scale = 0.0;
        // ⛔ `invalidate()` for the same reason a captive gets it: the saddle pin
        // is a discrete pose write every tick, so clearing the flag without the
        // BASELINE leaves the kernel believing the rider was airborne — and a
        // saddle that puts a rider against geometry then re-lands it every tick.
        // Latent on `pirate_sky_lookout` only because its riders are in the sky.
        rider_ground.invalidate();
        // Keep the CenteredAabb mirror in sync so damage / spatial
        // queries on the same tick see the rider where it visually
        // sits. update_ecs_actors writes this from rider.kin.pos at the
        // top of the next tick too, but the same-frame consumers
        // (damage application, projectile origin lookups) need it now.
        rider_aabb.center = rider_kin.pos;
        rider_aabb.half_size = rider_kin.size * 0.5;
    }
}

// ═══ BOARDING AND LEAVING ════════════════════════════════════════════════
//
// ⭐⭐ ADR 0020 DEFERRED THIS AND RESERVED THE SEAM: *"defer the ability to mount
// or board right now, but the authored pairs need to be some state in ldtk that
// indicates the two actors as linked… that can happen later."* Until now the
// only way into a saddle was an authored `mounted_on` reference resolved at
// spawn, and the two places that needed a runtime pair — the player-piloting
// end-to-end test and the monolith's pair fixtures — hand-inserted the three
// components each. Two hand-welds are a convention; a third would have been a
// rule nobody had written down.

/// Why a rider left the saddle. Carried by [`DismountRequested`] so a consumer
/// can tell a ride that ENDED from one that was INTERRUPTED without inspecting
/// the world for clues.
///
/// ⛔ NOT a severity ordering and not exhaustive of "bad things that happen to
/// riders": it names the causes that already exist, and a cause that needs a
/// different consequence adds an arm rather than reusing a near-enough one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DismountReason {
    /// The ride was for a fixed time and the time is up. See [`RideLease`].
    LeaseExpired,
    /// The rider chose to get off — a jump, a dodge, whatever the ruleset
    /// decided means *"put me down"*.
    RiderBailed,
    /// The rider was hit hard enough to be taken off. A rider that merely
    /// flinches stays aboard; this is the launch.
    RiderLaunched,
    /// The rider left play entirely — a ring-out, a stock spent, a death the
    /// ruleset is about to answer with a respawn. ⛔ NOT [`Self::RiderLaunched`]:
    /// a launch is a hit the rider survives and this is not, and a rider can
    /// reach this one without ever tumbling by simply steering out of bounds.
    RiderLeftPlay,
    /// The mount is gone: killed, despawned, or otherwise no longer carrying
    /// anybody.
    MountLost,
}

/// "Take this rider out of its saddle."
///
/// ⭐ A REQUEST RATHER THAN A CALL, because the causes are plural and live in
/// different crates: a lease expiring is this crate's, a bail-out is a
/// ruleset's genre statement about which buttons mean *get off*, and a launch is
/// the damage road's. One system performs the dissolution so the three cannot
/// drift apart about what leaving means.
///
/// ⛔ THE DEATH PATH DOES NOT USE THIS. [`enforce_mount_rider_link`] dissolves a
/// dead pair on its own and deliberately KEEPS `RidingOn` attached so a
/// same-room reset can re-mount without a lookup. Leaving voluntarily is the
/// opposite: the link is gone and nothing is coming back for it.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DismountRequested {
    pub rider: Entity,
    pub reason: DismountReason,
}

/// A ride with a clock on it: this rider comes off when `remaining` reaches
/// zero, whatever it is doing.
///
/// ⭐ SECONDS ON THE SIM CLOCK, like every other gameplay countdown in the tree
/// (`DeathInterlude`, `RespawnGrace`). ⛔ NOT `Empowered::for_seconds`, which
/// looks like the generic timed grant and is ONE component — granting it here
/// would silently overwrite whatever power-up the rider was carrying and
/// ending the ride would take that power-up with it.
///
/// Rides the RIDER rather than the mount: it is a statement about how long this
/// body may be carried, and a mount that outlives its rider (a summoned vehicle
/// waiting for the next one) is a thing the model already allows.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RideLease {
    /// Seconds of sim time left before the rider is put down.
    pub remaining: f32,
}

/// A mount HELD FOR ONE RIDER until they get on.
///
/// ⭐⭐ THIS IS WHAT SPLITS A SUMMON FROM ITS BOARD. The two used to be one
/// exclusive command: construct the mount, flush, weld the rider, install the
/// lease. That was defensible when the mount appeared on top of its summoner,
/// and it is wrong the moment the mount has to TRAVEL to them — which is where
/// this mechanic is going. Boarding on ARRIVAL is the general shape, and a mount
/// that arrives instantly is just the degenerate case of it.
///
/// ⛔⛔ AND IT IS WHAT STOPS THE SECOND ADMIRAL STEALING THE FIRST ONE'S SHARK.
/// Jon: *"in a mirror match, with two admirals, if one summons a shark, the
/// other should not be able to ride it."* A class licence cannot express that —
/// [`CanPilot`] says *"I can ride sharks"*, which is true of both admirals and
/// SHOULD be, because the admiral can ride sharks in Ambition too. The
/// distinction is not about the rider's capability at all; it is that THIS
/// mount is spoken for. So it is recorded on the mount, where the fact lives.
///
/// ⚠ A RESERVATION IS NOT A LINK. The mount is unridden while it holds one:
/// nothing is welded, no lease is running, and every system that filters on
/// [`MountSlot`] or [`RidingOn`] correctly ignores it.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MountReservedFor {
    /// The only body that may board this mount.
    pub rider: Entity,
    /// How long the ride lasts once they do — see [`RideLease`]. Carried here
    /// because the summon that asked for the ride is long gone by the time the
    /// board happens, and a lease installed early would be a clock running on a
    /// ride that has not started.
    pub lease_seconds: f32,
    /// How close the rider must be, in world units, before they get on.
    ///
    /// ⭐ A DISTANCE RATHER THAN A FLAG, so a mount summoned underfoot and a
    /// mount that flies in from off-screen take the same road. The first simply
    /// satisfies it on the tick it appears.
    pub board_within: f32,
    /// Seconds this reservation will wait to be reached before giving up.
    ///
    /// ⛔⛔ WITHOUT IT AN UNREACHED RESERVATION LIVES FOREVER, and that is not a
    /// hypothetical — it shipped. The first version had two outcomes, boarded and
    /// refused, and silently a third: a rider further away than
    /// [`Self::board_within`] was neither boarded NOR refused, so the mount stood
    /// there unclaimed, no [`RideRefused`] was ever written, and the ruleset that
    /// asked for it was never told. A summoner who never becomes a rider never
    /// becomes "held" either, so whatever gate the ruleset hangs on being mounted
    /// stays open and the next summon lands beside the last one.
    ///
    /// ⭐ SO NOT-ARRIVING IS AN OUTCOME, not the absence of one. It ends the same
    /// way a refusal does, which is what makes the doc line above — *"retires the
    /// reservation either way"* — true instead of aspirational.
    pub expires_in: f32,
}

/// Board every reserved mount whose rider is in reach, and retire the
/// reservation either way.
///
/// ⛔ IT DECIDES ONCE. A reservation that comes within reach is spent on that
/// tick — boarded, or [`RideRefused`] and gone. It does not linger and retry,
/// because a mount that kept trying could never FAIL to board, and "the rider
/// was not in a state to get on when it arrived" is a real outcome a ruleset
/// wants to be able to answer.
pub fn board_reserved_mounts(
    mut commands: bevy::prelude::Commands,
    time: bevy::prelude::Res<ambition_time::WorldTime>,
    mut reserved: Query<(
        Entity,
        &mut MountReservedFor,
        &ambition_platformer2d_core::BodyKinematics,
    )>,
    riders: Query<&ambition_platformer2d_core::BodyKinematics>,
    mut refused: MessageWriter<RideRefused>,
) {
    let dt = time.sim_dt();
    // ⛔ THE CLOCK RUNS ON EVERY RESERVATION, before any of the arms below. A
    // countdown that only advanced for reservations that were ALSO in range
    // would never expire the one case it exists for.
    let mut timed_out: Vec<(Entity, Entity)> = Vec::new();
    for (mount, mut reservation, _) in &mut reserved {
        reservation.expires_in -= dt;
        if reservation.expires_in <= 0.0 {
            timed_out.push((mount, reservation.rider));
        }
    }
    timed_out.sort();
    for (mount, rider) in timed_out {
        bevy::log::warn!(
            target: "ambition::mount",
            "reservation EXPIRED unreached: mount={mount:?} rider={rider:?} — \
             the rider never came within boarding range",
        );
        commands.entity(mount).remove::<MountReservedFor>();
        refused.write(RideRefused { rider, mount });
    }
    let reserved = reserved.as_readonly();
    // Sorted: two mounts arriving on one tick must be decided in a stable order,
    // and `Query` iteration order is not guaranteed.
    let mut arrivals: Vec<(Entity, MountReservedFor)> = reserved
        .iter()
        .filter_map(|(mount, reservation, kin)| {
            let rider_kin = riders.get(reservation.rider).ok()?;
            let gap = kin.pos.distance(rider_kin.pos);
            // ⭐ THE WAITING STATE SAYS ITSELF. A reservation that is simply not
            // reached yet used to be invisible — no line, no message, nothing —
            // so "the mount never boarded" and "the mount was refused" produced
            // identical evidence, which is none. `debug!` rather than `info!`
            // because this repeats every tick of an approach.
            if gap > reservation.board_within {
                bevy::log::debug!(
                    target: "ambition::mount",
                    "reservation waiting: mount={mount:?} rider={:?} gap={gap:.1} \
                     board_within={:.1} expires_in={:.2}s",
                    reservation.rider,
                    reservation.board_within,
                    reservation.expires_in,
                );
                return None;
            }
            Some((mount, *reservation))
        })
        .collect();
    arrivals.sort_by_key(|(mount, _)| *mount);
    for (mount, reservation) in arrivals {
        commands.entity(mount).remove::<MountReservedFor>();
        bevy::log::info!(
            target: "ambition::mount",
            "reservation reached: mount={mount:?} rider={:?} lease={}s",
            reservation.rider,
            reservation.lease_seconds,
        );
        commands.queue(move |world: &mut bevy::prelude::World| {
            if board(world, reservation.rider, mount) {
                // ⭐ THE LEASE IS PART OF THE BOARD, never a separate write: a
                // lease on a body that is not riding is one
                // `apply_dismount_requests` will never collect, because it skips
                // a rider with no link.
                world.entity_mut(reservation.rider).insert(RideLease {
                    remaining: reservation.lease_seconds,
                });
                bevy::log::info!(
                    target: "ambition::mount",
                    "boarded: mount={mount:?} rider={:?}",
                    reservation.rider,
                );
            } else {
                // ⛔⛔ THE REFUSAL SAYS WHY, AND IT USED TO SAY NOTHING. Splitting
                // the summon from the board moved this off the construction road,
                // which had a `warn!` naming `CanPilot` as the usual cause — and
                // the replacement was silent, so a player who watched a shark
                // appear and nobody get on had no evidence to hand back. That is
                // the exact report this line exists to answer.
                let class = world.get::<Mountable>(mount).map(|m| m.class.clone());
                let can = world
                    .get::<CanPilot>(reservation.rider)
                    .map(|c| format!("{c:?}"));
                let already_riding = world.get::<RidingOn>(reservation.rider).is_some();
                let seat_taken = world
                    .get::<MountSlot>(mount)
                    .is_some_and(|slot| slot.rider.is_some());
                bevy::log::warn!(
                    target: "ambition::mount",
                    "board REFUSED: mount={mount:?} rider={:?} mount_class={class:?} \
                     rider_can_pilot={can:?} rider_already_riding={already_riding} \
                     saddle_taken={seat_taken}",
                    reservation.rider,
                );
                world.write_message(RideRefused {
                    rider: reservation.rider,
                    mount,
                });
            }
        });
    }
    // Retire a reservation whose rider is gone entirely, so the mount is not
    // held forever for a body that no longer exists.
    let mut orphaned: Vec<(Entity, Entity)> = reserved
        .iter()
        .filter(|(_, reservation, _)| riders.get(reservation.rider).is_err())
        .map(|(mount, reservation, _)| (mount, reservation.rider))
        .collect();
    orphaned.sort();
    for (mount, rider) in orphaned {
        commands.entity(mount).remove::<MountReservedFor>();
        refused.write(RideRefused { rider, mount });
    }
}

/// Weld a rider into a mount's saddle — ADR 0020's board action.
///
/// Refuses, returning `false`, when the pair is not a legal one: the mount is
/// not [`Mountable`], the rider's [`CanPilot`] does not cover the mount's class,
/// either side is already in a link, or the two are the same entity.
///
/// ⛔ THE CLASS CHECK IS THE POINT OF THE FUNCTION. *"A shark-rider cannot board
/// a mech"* is ADR 0020's rule and it only exists if something asks; a caller
/// that welded the components itself would be authorising its own pairing, and
/// then the check would be documentation rather than a rule.
///
/// ⚠ TAKES A `&mut World` because its first caller is inside the summon
/// executor's exclusive command, where the mount it is boarding was created
/// moments earlier and has no `Commands` flush between. A system with `Commands`
/// reaches the same behaviour by queueing this.
///
/// ⛔ `TemporaryControl` IS NOT WRITTEN HERE, and that is deliberate. That
/// component says *which transient controller is MASKING the body's autonomous
/// brain*, and boarding only masks a brain when there is a
/// [`MountedBrainCache`] to swap in — the authored NPC composite. A seated
/// fighter or a possessing human keeps driving its own body from the saddle, so
/// stamping `Mounted` there would claim a brain swap that never happened.
/// `enforce_mount_rider_link` writes it on the arm that does the swap.
pub fn board(world: &mut bevy::prelude::World, rider: Entity, mount: Entity) -> bool {
    if rider == mount {
        return false;
    }
    let Some(class) = world.get::<Mountable>(mount).map(|m| m.class.clone()) else {
        return false;
    };
    // A rider with no `CanPilot` at all pilots nothing. ⛔ the permissive
    // reading — "no statement means no restriction" — is the shape that makes a
    // capability check decorative, and this engine has paid for that before.
    if !world
        .get::<CanPilot>(rider)
        .is_some_and(|can| can.can_pilot(&class))
    {
        return false;
    }
    if world.get::<RidingOn>(rider).is_some() {
        return false;
    }
    if world
        .get::<MountSlot>(mount)
        .is_some_and(|slot| slot.rider.is_some())
    {
        return false;
    }
    // ⛔⛔ A MOUNT HELD FOR SOMEBODY ELSE REFUSES EVERYONE ELSE. Without this a
    // class licence is the whole check, and in a mirror match both admirals hold
    // one — so the second could walk onto the first one's summoned shark. The
    // licence is right and stays: what is wrong is treating "I can ride sharks"
    // as "I can ride THIS shark". See [`MountReservedFor`].
    if world
        .get::<MountReservedFor>(mount)
        .is_some_and(|held| held.rider != rider)
    {
        return false;
    }
    // ⭐ AND THE BODY SAYS IT IS HELD. `PoseOwnedExternally` is the one fact a
    // constrained body owes the domains that cannot see this one — the movement
    // kernel must not integrate its locomotion and a move may forbid itself
    // while it is set. See the marker's own note for why it lives in `_core`.
    world.entity_mut(rider).insert((
        RidingOn { mount },
        Mounted,
        ambition_platformer2d_core::PoseOwnedExternally,
    ));
    world.entity_mut(mount).insert(MountSlot {
        rider: Some(rider),
    });
    true
}

/// Count down every [`RideLease`] and ask for the dismount when one runs out.
///
/// ⛔ IT ASKS RATHER THAN ACTS, so a timed ride and a bail-out leave the saddle
/// by exactly the same road — see [`DismountRequested`].
pub fn tick_ride_leases(
    time: bevy::prelude::Res<ambition_time::WorldTime>,
    mut leases: Query<(Entity, &mut RideLease)>,
    mut dismounts: MessageWriter<DismountRequested>,
) {
    let dt = time.sim_dt();
    // Collected and sorted so two leases expiring on one tick ask in a stable
    // order; `Query` iteration order is not guaranteed and the requests are
    // consumed in write order.
    let mut expired: Vec<Entity> = Vec::new();
    for (rider, mut lease) in &mut leases {
        if lease.remaining <= 0.0 {
            continue;
        }
        lease.remaining -= dt;
        if lease.remaining <= 0.0 {
            lease.remaining = 0.0;
            expired.push(rider);
        }
    }
    expired.sort();
    for rider in expired {
        dismounts.write(DismountRequested {
            rider,
            reason: DismountReason::LeaseExpired,
        });
    }
}

/// Perform the dissolutions [`DismountRequested`] asked for.
///
/// The rider gets its authored body back — the gravity this module zeroed while
/// it was carried, and the size the saddle may have snapped — and the mount's
/// slot is emptied so it can carry somebody else.
///
/// ⛔ THE RIDER'S BRAIN IS NOT REBUILT HERE. A dead mount announces `MountDied`
/// and `rebuild_dismounted_rider_brains` answers it, because that rebuild needs
/// character-runtime facts this crate does not import. A rider that got off a
/// LIVE mount never had its brain masked in the first place unless it carries a
/// [`MountedBrainCache`] — and if it does, `enforce_mount_rider_link` will find
/// it unmounted next tick and re-arm, which is the behaviour an NPC that walked
/// off its own shark should have.
pub fn apply_dismount_requests(
    mut commands: Commands,
    mut requests: bevy::prelude::MessageReader<DismountRequested>,
    mut riders: Query<
        (
            &RidingOn,
            &mut CenteredAabb,
            &mut ae::BodyKinematics,
            &mut ae::ActorSurfaceState,
            &SpawnBaseline,
        ),
        Without<MountSlot>,
    >,
    mut mounts: Query<&mut MountSlot>,
    mut left: MessageWriter<RiderDismounted>,
) {
    for request in requests.read() {
        let Ok((riding, mut aabb, mut kin, mut surface, baseline)) =
            riders.get_mut(request.rider)
        else {
            continue;
        };
        let mount = riding.mount;
        // The authored body, from the record that survived the ride — the live
        // values are exactly the ones the saddle overwrote.
        surface.gravity_scale = baseline.gravity_scale;
        kin.size = baseline.size;
        aabb.center = kin.pos;
        aabb.half_size = kin.size * 0.5;
        commands
            .entity(request.rider)
            .remove::<(
                RidingOn,
                Mounted,
                RideLease,
                ambition_platformer2d_core::PoseOwnedExternally,
            )>();
        if let Ok(mut slot) = mounts.get_mut(mount) {
            // Only if it is still THIS rider's slot: a mount already re-crewed
            // must not be emptied by a stale request.
            if slot.rider == Some(request.rider) {
                slot.rider = None;
            }
        }
        bevy::log::info!(
            target: "ambition::mount",
            "dismounted: rider={:?} mount={mount:?} reason={:?}",
            request.rider,
            request.reason,
        );
        left.write(RiderDismounted {
            rider: request.rider,
            mount,
            reason: request.reason,
        });
    }
}

/// A rider left a LIVE mount's saddle. The twin of `MountDied`, which announces
/// the other way a pair dissolves.
///
/// ⭐ IT EXISTS BECAUSE THE MOUNT NEEDS TO KNOW. A summoned vehicle departs when
/// it loses its rider, and the departure is the ruleset's business rather than
/// this crate's — so the dissolution is announced and whoever authored the mount
/// decides what an empty saddle means.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RiderDismounted {
    pub rider: Entity,
    pub mount: Entity,
    pub reason: DismountReason,
}

/// The set [`enforce_mount_rider_link`] runs in.
///
/// The mount/rider link is re-established here, and the frame's staged victim
/// hits must be handed over BEFORE that happens.
///
/// TWO members, nested inside `CombatSet::Settle`: the link enforcer and the
/// dismount brain rebuild it announces to, chained. The staged-hit handover must
/// precede BOTH, which is why the set covers the pair rather than the enforcer
/// alone. The consumer is itself in `Settle`, so pinning the parent would be a
/// cycle — this is the shape only a nested set can express.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MountRiderLinkEnforced;

/// Dissolve a rider / mount link when either side dies. Runs after
/// the damage pass.
///
/// - Mount dies: rider's gravity flips on (so they fall), and the dissolution is
///   ANNOUNCED with `MountDied`. `rebuild_dismounted_rider_brains` answers it and
///   swaps the brain + action set, so a pirate falling off a dead shark keeps
///   whatever capabilities their held item grants (gun-sword shots today, axe /
///   bow / bomb authored rows later) — this system does not build brains. The
///   [`RidingOn`]
///   component itself STAYS attached — `sync_riders_to_mounts`
///   gates on `mount.alive` and won't snap the rider while the
///   mount is dead. Keeping the link record lets the same-room
///   reset path re-mount the rider once the mount is alive again
///   without having to look it up by id.
/// - Rider dies: the mount keeps running with its own (already
///   standalone) brain. The mount's [`MountSlot`] keeps its
///   `rider` back-reference so the reset path can re-arm the link.
///
/// The dissolution is idempotent, and the `Mounted` MARKER is what makes it so:
/// the dissolve arm requires `(mount dead, rider still marked)`, and it removes
/// the marker, so a second pass over the same dead mount lands on the steady
/// state. ⛔ NOT the rider's brain — this system does not read one. We do not
/// trust the alive TRANSITION to fire once either, because `reset_to_spawn`
/// brings `mount.alive` back to true and a future death means dissolving again.
pub fn enforce_mount_rider_link(
    mut commands: Commands,
    mut mount_died: MessageWriter<MountDied>,
    mut riders: Query<
        (
            Entity,
            &RidingOn,
            &mut CenteredAabb,
            Option<&MountedBrainCache>,
            Option<&Mounted>,
            // The same four columns the saddle sync names, plus the rider's
            // AUTHORED baseline — the body a reset hands back.
            //
            // ⭐ THE LIVE COMPONENTS ARE NOT THAT FACT, which is why it has its
            // own name: `BodyBaseSize` follows the stance, `fly_enabled` is
            // toggled at runtime, and `gravity_scale` is the value THIS MODULE
            // zeroes while the rider is in the saddle. Reading any of them here
            // would hand a grown or a landed rider the wrong body back.
            &mut ae::BodyKinematics,
            &mut ae::ActorSurfaceState,
            &mut ambition_characters::actor::BodyHealth,
            &SpawnBaseline,
        ),
        Without<MountSlot>,
    >,
    mounts: Query<
        (
            Entity,
            Option<&ambition_characters::actor::BodyHealth>,
            Option<&Mountable>,
        ),
        With<MountSlot>,
    >,
    // Stable ids, to record the mount by `SimId` in the rider's temporary-control
    // state (so a snapshot restores the mount link across a rewind).
    sim_ids: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
) {
    // Build a lookup of mount alive-ness + death impact. With two-pirate fights this is O(R+M)
    // per frame and the hashmap stays small.
    use std::collections::HashMap;
    let mut mount_alive: HashMap<Entity, bool> = HashMap::new();
    let mut mount_death_impact: HashMap<Entity, MountDeathImpact> = HashMap::new();
    for (mount_entity, mount_health, mountable) in &mounts {
        let alive = mount_health.is_some_and(|h| h.alive());
        mount_alive.insert(mount_entity, alive);
        mount_death_impact.insert(
            mount_entity,
            mountable.map(|m| m.death_impact).unwrap_or_default(),
        );
    }

    for (
        rider_entity,
        riding,
        mut rider_aabb,
        cache,
        was_mounted,
        mut rider_kin,
        mut rider_surface,
        mut rider_health,
        rider_spawn,
    ) in &mut riders
    {
        if !rider_health.alive() {
            continue;
        }
        let alive = mount_alive.get(&riding.mount).copied().unwrap_or(false);
        match (alive, was_mounted.is_some()) {
            // Mount alive, rider already mounted → steady state. The
            // sync system snaps each frame; nothing to do here.
            (true, true) => {}
            // Mount alive, rider missing the Mounted marker → we
            // either just spawned without the marker (first tick)
            // or the same-room reset path brought the mount back to
            // life. Restore the cached MOUNTED brain + action set
            // and zero gravity. Re-arm idempotently.
            (true, false) => {
                if let Some(cache) = cache {
                    rider_surface.gravity_scale = 0.0;
                    commands.entity(rider_entity).insert((
                        cache.brain.clone(),
                        cache.action_set.clone(),
                        Mounted,
                    ));
                    // Record the mount by stable id for snapshot restore. Only when
                    // the mount carries a `SimId` (otherwise the link isn't
                    // reconstructible); the marker above still tracks live state.
                    if let Ok(mount_id) = sim_ids.get(riding.mount) {
                        commands.entity(rider_entity).insert(
                            ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl::Mounted {
                                mount: mount_id.clone(),
                            },
                        );
                    }
                }
            }
            // Mount dead, rider currently mounted → dissolve. Flip gravity on,
            // keep the rider at its authored sky-rider size, emit `MountDied`,
            // and install the shared explicitly-hostile dismounted rider
            // brain/action-set policy so a PirateRaider / PirateHeavy variant
            // falls and fights without visually scaling up — EXCEPT a boss
            // rider (carries `BossConfig`), whose authored `Brain` is kept.
            (false, true) => {
                // Mount death impact (ADR 0020): by default the rider drops
                // unharmed, but a mount authored to explode splashes lethal-ish
                // damage onto the rider's separate HP pool. Applied once, on the
                // death transition, before the dismount rebuild.
                if let MountDeathImpact::Splash(amount) = mount_death_impact
                    .get(&riding.mount)
                    .copied()
                    .unwrap_or_default()
                {
                    rider_health.damage(amount);
                    // If the splash killed the rider, skip the dismount rebuild —
                    // a dead rider needs no solo brain.
                    if !rider_health.alive() {
                        continue;
                    }
                }
                // ⭐ THE AUTHORED SCALE, READ rather than re-derived from
                // `tuning.is_aerial`. An aerial rider keeps floating; a walker
                // falls. This module is what zeroed the live value, so the
                // baseline is the only place the answer survived.
                rider_surface.gravity_scale = rider_spawn.gravity_scale;
                rider_kin.size = rider_spawn.size;
                // Publish immediately so same-frame presentation / combat sees
                // the rider's grounded pose. This is usually the same size as
                // MountedSize; keeping the write here makes intentional future
                // size overrides explicit and safe.
                rider_aabb.center = rider_kin.pos;
                rider_aabb.half_size = rider_kin.size * 0.5;
                // Announce the dissolution as a body fact (ADR 0020; Q19a). The
                // boss-encounter bridge turns this into a `mount_died` external
                // phase trigger for a mounted boss; other consumers may listen
                // too. Written after the (possibly lethal) splash: a rider the
                // splash killed already `continue`d above, so a `MountDied` here
                // always names a rider that survives to dismount.
                mount_died.write(MountDied {
                    mount: riding.mount,
                    rider: rider_entity,
                });
                // ⭐ THE BRAIN SWAP IS NOT DONE HERE. `MountDied` above is the
                // whole request: `rebuild_dismounted_rider_brains` reads it and
                // rebuilds the rider's solo brain/action-set from its durable
                // kit and the prepared cast — character-runtime facts this
                // module would otherwise have to import in order to dissolve a
                // mount. Same road the boss bridge already takes. A boss rider
                // is skipped THERE, by the same `BossConfig` marker (Q19b).
                commands
                    .entity(rider_entity)
                    .remove::<Mounted>()
                    // Back to autonomous control for snapshot purposes (a boss rider
                    // keeps its authored brain but is no longer mount-controlled).
                    .insert(ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl::Autonomous)
                    // Sprite-binding refresh so the rider's sheet
                    // re-resolves on the next presentation pass.
                    .remove::<ambition_platformer2d_shared_tangle::feature_kind::BoundFeatureKind>(
                    );
            }
            // Mount dead, rider already dissolved → steady state.
            (false, false) => {}
        }
    }
}

// ⛔⛔ THIS CRATE SHIPS WITH NO TESTS OF ITS OWN, AND THAT IS A STATED COST.
// The fifteen arms that cover these rules build real riders through the actor
// monolith's construction road (`ActorClusterSeed`, `ActorDisposition`, the boss
// systems), so they stayed where their fixtures are:
// `ambition_platformer2d_actor_monolith::features::ecs::mount_pair_tests`.
//
// ⭐ THEY STILL TEST THIS CRATE — from the composition, against the real bodies
// the game makes, which is the stronger half of the coverage. What is missing is
// the narrow half: nine of the fifteen are mount RULES and are expressible in the
// nine components these systems read. See D33 for the classification.

/// World position of the rider's hand (where mounted attacks originate). The
/// hand offset is sprite-layout-derived but the SIM needs it to spawn attacks, so
/// it lives here, not in presentation.
const HAND_OFFSET_NORM: ambition_platformer2d_core::Vec2 =
    ambition_platformer2d_core::Vec2::new(0.18, -0.05);
pub fn rider_hand_world_pos(
    rider_pos: ambition_platformer2d_core::Vec2,
    facing: f32,
    rider_height: f32,
) -> ambition_platformer2d_core::Vec2 {
    rider_hand_world_pos_in_frame(
        rider_pos,
        facing,
        rider_height,
        ambition_platformer2d_core::Vec2::new(0.0, 1.0),
    )
}

/// World position of the rider's hand under the actor's acceleration frame.
/// `facing` is local side-facing, so the hand offset is authored in rider-local
/// side/down coordinates and then resolved to world.
pub fn rider_hand_world_pos_in_frame(
    rider_pos: ambition_platformer2d_core::Vec2,
    facing: f32,
    rider_height: f32,
    gravity_dir: ambition_platformer2d_core::Vec2,
) -> ambition_platformer2d_core::Vec2 {
    let facing_sign = if facing >= 0.0 { 1.0 } else { -1.0 };
    let hand_local = ambition_platformer2d_core::Vec2::new(
        HAND_OFFSET_NORM.x * rider_height * facing_sign,
        HAND_OFFSET_NORM.y * rider_height,
    );
    rider_pos + ambition_platformer2d_core::AccelerationFrame::new(gravity_dir).to_world(hand_local)
}

impl bevy::ecs::entity::MapEntities for MountSlot {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        if let Some(entity) = self.rider.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}

impl bevy::ecs::entity::MapEntities for MountReservedFor {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.rider = mapper.get_mapped(self.rider);
    }
}

impl bevy::ecs::entity::MapEntities for RidingOn {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.mount = mapper.get_mapped(self.mount);
    }
}

/// Every piece of mount-pair state the simulation rewinds.
///
/// ⛔⛔ IT LIVED IN THE ACTOR MONOLITH UNTIL 2026-08-26, WHICH IS A CARVE THAT
/// STOPPED HALF WAY. The types, the systems and the saddle constraint all moved
/// here, but `actor_monolith/rollback_registration.rs` — whose own header
/// promises *"the actor runtime names only state defined in this crate"* — went
/// on naming seven `ambition_mount::` components and two of their entity
/// mappings. So adding one mutable field to a mount component meant editing this
/// crate AND remembering a census in a crate that no longer owns the domain, and
/// forgetting the second half is a silent desync rather than a compile error.
///
/// ⭐ THE RUNTIME'S OWN RULE, applied: *"gameplay domains own their concrete
/// declarations … adding state to an existing domain edits only that domain."*
/// This composes beside `ambition_combat`'s and `ambition_characters`'.
///
/// ⛔ THE STABLE NAMES DO NOT MOVE. They are identities on the wire, not
/// addresses — `mount.can_pilot` names the same bytes wherever the registration
/// is written from — so this is a repoint and NOT a schema change, and
/// `GGRS_ROLLBACK_SCHEMA_VERSION` deliberately does not move for it. The owner
/// label does change (it is `CARGO_PKG_NAME`), and the readable baseline omits
/// owner labels for exactly this reason: ownership is organizational.
///
/// ⚠ `Mass`, `SpawnBaseline` and `TemporaryControl` are NOT swept in here. Mount
/// reads them, which is not the same as owning them — all three moved to
/// `shared_tangle` precisely because two domains share them, and each is its own
/// ownership decision rather than something this patch gets to settle by
/// proximity.
///
/// ⛔ THE BOUND IS SPELLED `R: RollbackRegistrar`, WITH THE TRAIT IMPORTED, and
/// that is load-bearing rather than style. `check_absence_contracts.py` finds
/// federated registration sites by that exact substring — a file generic over
/// the registrar is a registration site and nothing else is — so writing the
/// bound as a fully-qualified path hid this whole file from the ratchet, and
/// two stable names reported as having LEFT the wire format while they were
/// still very much in it. The scanner's own comment says it stopped
/// hand-listing crates for exactly this reason; the price is that the marker
/// has to be spelled the way every other domain spells it.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    const OWNER: &str = env!("CARGO_PKG_NAME");

    registrar.rollback_component_clone::<CanPilot>(OWNER, "mount.can_pilot");
    registrar.rollback_component_clone_entity_set::<MountSlot>(OWNER, "mount.slot", |slot| {
        slot.rider.into_iter().collect()
    });
    registrar.rollback_map_entities::<MountSlot>(OWNER, "map.mount_slot");
    registrar.rollback_component_clone::<Mountable>(OWNER, "mount.mountable");
    registrar.rollback_component_clone::<Mounted>(OWNER, "mount.mounted");
    // ⚠ OWNED HERE, NOT BY `_core`, and the split is deliberate: `_core` DEFINES
    // the marker so both domains can name it, and whoever maintains its
    // lifecycle registers it. Today the saddle is the only thing that sets it.
    registrar.rollback_component_clone::<ambition_platformer2d_core::PoseOwnedExternally>(
        OWNER,
        "mount.pose_owned_externally",
    );
    registrar.rollback_component_clone_entity_ref::<RidingOn>(OWNER, "mount.riding_on", |riding| {
        riding.mount
    });
    registrar.rollback_map_entities::<RidingOn>(OWNER, "map.riding_on");
    registrar.rollback_component_clone::<MountedBrainCache>(OWNER, "mount.brain_cache");
    registrar.rollback_component_clone::<MountedSize>(OWNER, "mount.authored_size");
    // The clock on a timed ride. Registered for the same reason every other
    // gameplay countdown is: a rewind that restored "this body is being carried"
    // without restoring how much longer would put the rider down on a different
    // tick than the one being resimulated, and where a rider is put down is a
    // position on the stage.
    // PROBED: the remaining seconds decide which tick the rider is put down, and
    // where a body is put down is a position. A presence-only probe would
    // satisfy the coverage oracle while seeing nothing of the value.
    registrar.rollback_component_clone_probed::<RideLease>(OWNER, "mount.ride_lease", |lease| {
        lease.remaining.to_bits() as u64
    });
    // ⛔ BOTH CHANNELS, because a reader's cursor is `Local` state GGRS never
    // rewinds. An abandoned future's cursor would either re-read a consumed
    // `DismountRequested` — dissolving a link that was re-formed — or skip an
    // unread one, leaving a rider welded to a mount whose lease has run out.
    // Both are positions.
    //
    // ⭐ AND CLEARING IS RIGHT FOR BOTH: each is DERIVED every tick from
    // rollback-registered state (`RideLease` and `RidingOn`), so a resim re-emits
    // them on the tick it emitted them before. Losing the buffered copy is what
    // should happen to a message the simulation will say again.
    // ⛔ AN ENTITY REFERENCE, so it needs the mapping pass as well as the clone —
    // the same pair `RidingOn` gets, for the same reason: a resimulation rebuilds
    // the world's entities and a raw id would point at whoever landed in that
    // slot.
    registrar.rollback_component_clone_entity_ref::<MountReservedFor>(
        OWNER,
        "mount.reserved_for",
        |held| held.rider,
    );
    registrar.rollback_map_entities::<MountReservedFor>(OWNER, "map.mount_reserved_for");
    registrar.clear_message_on_rollback::<DismountRequested>(OWNER, "message.dismount_requested");
    registrar.clear_message_on_rollback::<RiderDismounted>(OWNER, "message.rider_dismounted");
    // ⭐ AND THE REFUSAL FOR THE SAME REASON. It is derived from the summon that
    // asked for it, so a resim that replays the summon replays the refusal.
    registrar.clear_message_on_rollback::<RideRefused>(OWNER, "message.ride_refused");
}

/// A pairing that was ASKED FOR and REFUSED — the mount exists, and nobody is
/// on it.
///
/// ⛔⛔ WITHOUT THIS A REFUSED BOARD LEAVES A BODY NOBODY OWNS. [`board`] returns
/// `false` and the mount simply stands there: it never got a [`MountSlot`], so
/// every system that cleans up after a ride — `depart_when_riderless` included —
/// filters it straight out, and nothing else in the world has a reason to look
/// at it. That is how an admiral with no `CanPilot` ended up standing next to an
/// immortal shark.
///
/// ⭐ IT SAYS REFUSED, NOT DISMOUNTED, and the distinction is load-bearing.
/// [`RiderDismounted`] means a ride ENDED; a consumer may reasonably assume the
/// rider was aboard, was welded, and has state to unwind. Nothing here was ever
/// true of this pair. Reusing that message would have saved a channel and lied.
///
/// ⭐ THE RULESET DECIDES WHAT A REFUSAL COSTS. This says only that it happened.
/// A platform fighter sends the summoned mount away and charges the player for
/// the attempt; a game where you whistle for a horse you have not tamed might
/// leave it standing there, which is the same fact with a different answer.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RideRefused {
    /// Who asked to get on.
    pub rider: Entity,
    /// What they were refused. Still alive, still in the world.
    pub mount: Entity,
}

/// Where the rider sits, relative to the mount's centre, in WORLD units.
///
/// `rider_offset` is authored in the mount's LOCAL frame — `+x` toward the
/// mount's facing side, `+y` toward its feet — so under rotated gravity it has
/// to be rotated into the world before it can be added to a world position.
/// That rotation is the whole of this function.
///
/// ⛔⛔ IT USED TO CARRY A MASS-WEIGHTED CENTRE-OF-GRAVITY TERM, AND THAT TERM
/// WAS A NO-OP WHERE IT WORKED AND A BUG WHERE IT DID NOT:
///
/// ```text
/// let cog_local  = rider_offset * w_rider;                        // LOCAL
/// let rider_local = cog_local + frame.to_world(rider_offset - cog_local);
/// //                ^^^^^^^^^ local     ^^^^^^^^^^^^^^^^^^^^^^^^^ world
/// ```
///
/// Under default gravity `to_world` is the identity, so the two terms collapse
/// to `rider_offset` and the masses cancel exactly — the weighting had never
/// changed a single pixel. Under ROTATED gravity the unrotated `cog_local` term
/// stays pinned to SCREEN axes while the rest of the pair turns, so the rider
/// slides off the saddle by an amount that grows with the mass ratio.
///
/// ⭐⭐ AND THE INTENT IT WAS REACHING FOR CANNOT BE EXPRESSED HERE AT ALL. A
/// pair rotating rigidly about a shared centre of mass moves BOTH bodies about
/// one world-space point. This constraint owns the RIDER's pose and nothing
/// else — the mount's position is written by the mount's own movement — so
/// pivoting only the rider is not that rotation under any mass ratio. A
/// constraint with authority over one body cannot implement a two-body
/// transform, and the honest thing is to say so rather than approximate it.
///
/// ⇒ what this pin means is exactly *"the rider is at the authored mount-local
/// saddle offset"*, and that is what it now computes.
pub fn saddle_world_offset(
    rider_offset: ambition_platformer2d_core::Vec2,
    facing: f32,
    frame: ambition_platformer2d_core::AccelerationFrame,
) -> ambition_platformer2d_core::Vec2 {
    // ⛔⛔ THE MIRROR, AND WITHOUT IT THIS FUNCTION'S OWN DOC WAS A LIE.
    // `AccelerationFrame` knows gravity-relative side and down; it knows nothing
    // about which way the mount is looking. So a saddle authored at `x = +5`
    // stayed on the same GRAVITY-relative side when the mount turned around —
    // while four lines from the call site the rider is given
    // `rider_kin.facing = mount_kin.facing`. A rider that turns with its mount
    // and does not move with it has swapped shoulders, which is not a saddle.
    //
    // ⭐ INVISIBLE TODAY because every authored mount uses `x = 0`, and a
    // rotated-gravity arm that never flips facing cannot see it either. It is
    // fixed now rather than when the first lateral saddle is authored, because
    // by then the wrong answer would be baked into the authored number.
    //
    // ⛔ `signum` OF A FACING THAT MAY BE ZERO IS ZERO, which would collapse the
    // lateral offset instead of mirroring it. A neutral facing keeps the
    // authored side.
    let side = if facing < 0.0 { -1.0 } else { 1.0 };
    frame.to_world(ambition_platformer2d_core::Vec2::new(
        rider_offset.x * side,
        rider_offset.y,
    ))
}

#[cfg(test)]
mod saddle_tests {
    use super::*;
    use ambition_platformer2d_core::{AccelerationFrame, Vec2};

    /// ⛔⛔ THE SADDLE HOLDS UNDER ROTATED GRAVITY, and the offset has BOTH
    /// components on purpose.
    ///
    /// An offset that is purely along one local axis cannot tell a correct
    /// rotation from a partial one: the missing component is where the error
    /// lands. This one is `(3, -11)` — beside and above the mount's centre,
    /// which is what a saddle actually is.
    ///
    /// ⭐ THE INVARIANT IS A LENGTH AND TWO PROJECTIONS. Rotating a vector may
    /// not change how long it is, and the authored components must come back
    /// out along the frame's own side/up — which is the statement "the rider
    /// sits in the saddle" said in a way that does not depend on which way the
    /// world is pointing.
    #[test]
    fn the_saddle_offset_rotates_with_the_pair_and_keeps_its_authored_shape() {
        let offset = Vec2::new(3.0, -11.0);
        // Feet toward world +x: a wall-walker on a right-hand wall.
        let sideways = AccelerationFrame::new(Vec2::new(1.0, 0.0));
        let world = saddle_world_offset(offset, 1.0, sideways);

        assert!(
            (world.length() - offset.length()).abs() < 1e-4,
            "the saddle offset changed LENGTH when the pair rotated, so it is \
             not a rotation: authored {:?}, world {:?}",
            offset,
            world
        );
        assert!(
            (world.dot(sideways.side) - offset.x).abs() < 1e-4,
            "the authored SIDE component did not come back out along the \
             frame's side axis"
        );
        assert!(
            (world.dot(sideways.down) - offset.y).abs() < 1e-4,
            "the authored TOWARD-FEET component did not come back out along the \
             frame's down axis"
        );
    }

    /// ⛔⛔ AND THE SADDLE TURNS WITH THE MOUNT.
    ///
    /// `AccelerationFrame` knows gravity, not facing, so `to_world` alone left a
    /// laterally-authored saddle on the same GRAVITY-relative side when the
    /// mount turned around — while the constraint four lines away hands the
    /// rider `mount_kin.facing`. A rider that turns with its mount and does not
    /// move with it has swapped shoulders.
    ///
    /// ⭐ THE TWO ASSERTIONS ARE THE TWO HALVES: the side component MUST flip
    /// and the toward-feet component MUST NOT. A test that only checked "the
    /// answer changed" would pass for a mirror that flipped both, which is a
    /// rider standing on its head.
    #[test]
    fn a_lateral_saddle_mirrors_with_the_mounts_facing_and_nothing_else_does() {
        let offset = Vec2::new(3.0, -11.0);
        let sideways = AccelerationFrame::new(Vec2::new(1.0, 0.0));
        let right = saddle_world_offset(offset, 1.0, sideways);
        let left = saddle_world_offset(offset, -1.0, sideways);

        assert!(
            (right.dot(sideways.side) + left.dot(sideways.side)).abs() < 1e-4,
            "the saddle's SIDE component did not flip when the mount turned, so a \
             rider authored on one shoulder stays on the gravity-side it started \
             on while facing the other way"
        );
        assert!(
            (right.dot(sideways.down) - left.dot(sideways.down)).abs() < 1e-4,
            "the saddle's TOWARD-FEET component moved when the mount merely \
             turned around — a mount facing left does not put its rider under it"
        );
    }

    /// ⭐ A NEUTRAL FACING KEEPS THE AUTHORED SIDE. `signum(0.0)` is zero, and
    /// multiplying by it would collapse the lateral offset to nothing rather
    /// than mirroring it — a rider snapping to the mount's centre line the
    /// instant its facing passed through zero.
    #[test]
    fn a_neutral_facing_does_not_collapse_the_saddle() {
        let offset = Vec2::new(3.0, -11.0);
        let frame = AccelerationFrame::new(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR);
        assert_eq!(saddle_world_offset(offset, 0.0, frame), offset);
    }

    /// ⭐ THE PREMISE GUARD FOR THE ARM ABOVE, and the measurement that
    /// condemned the mass term: under DEFAULT gravity the rotation is the
    /// identity. So every arm that only ever ran at default gravity — which is
    /// every mount arm in the tree — agreed with the broken math exactly, and
    /// no amount of default-gravity coverage could ever have caught it.
    #[test]
    fn default_gravity_leaves_the_authored_offset_alone() {
        let offset = Vec2::new(3.0, -11.0);
        assert_eq!(
            saddle_world_offset(
                offset,
                1.0,
                AccelerationFrame::new(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR)
            ),
            offset
        );
    }
}
