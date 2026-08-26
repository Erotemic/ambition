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
// weight wearing one mechanic's address. The writer is the character runtime's
// physical baseline and the reader is the mass-weighted centre below; two
// domains, so it sits under both. Imported, never re-exported.
use ambition_platformer2d_shared_tangle::body::Mass;
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
    /// Rider's center offset from the mount's center. For an
    /// aerial mount this is typically `(0, -mount.size.y * 0.5 -
    /// rider.size.y * 0.5 + epsilon)` so the rider sits on the
    /// mount's saddle without their hitboxes overlapping.
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
            Option<&Mass>,
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
            Option<&Mass>,
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
        rider_mass,
        mut rider_kin,
        mut rider_surface,
        mut rider_ground,
        rider_health,
    ) in &mut riders
    {
        let Ok((mountable, mount_mass, mount_frame, mount_kin, mount_health)) =
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
        // Rotate-as-a-unit: the saddle offset is authored in the mount's local frame, so rotate
        // it into world space by the pair's gravity frame and pivot the rider around the
        // mass-weighted center of gravity.
        let frame = mount_frame.basis();
        let mass_mount = mount_mass.copied().unwrap_or_default().0.max(0.0001);
        let mass_rider = rider_mass.copied().unwrap_or_default().0.max(0.0001);
        let w_rider = mass_rider / (mass_mount + mass_rider);
        // COG relative to the mount center (mount at 0, rider at `rider_offset`).
        let cog_local = mountable.rider_offset * w_rider;
        let rider_local = cog_local + frame.to_world(mountable.rider_offset - cog_local);
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

impl bevy::ecs::entity::MapEntities for RidingOn {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.mount = mapper.get_mapped(self.mount);
    }
}
