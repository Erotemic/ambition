//! Generic rider / mount relationship between two ECS actor entities.
//!
//! Replaces the legacy "fused archetype" model (`PirateOnShark` /
//! `PirateHeavyOnShark` as single entities with a second HP pool +
//! second hitbox). Mount and rider are now SEPARATE entities; a
//! [`RidingOn`] component on the rider points at the mount entity,
//! and [`MountSlot`] on the mount holds the rider's `Entity` back so
//! either side can resolve the link.
//!
//! Per-tick coupling: [`sync_riders_to_mounts`] snaps the rider's
//! position / facing to the mount's position + the mount's
//! [`Mountable::rider_offset`]. The rider's brain still runs (it
//! computes a fire intent toward the target from the snapped
//! position); the snap each frame nullifies its movement intent.
//!
//! Dissolution: [`enforce_mount_rider_link`] runs after the damage
//! pass. When the mount dies the rider's gravity flips back on and
//! its brain + action set are swapped through the shared dismounted
//! rider builder (so a pirate falling off a dead shark walks toward
//! the player and swings melee, rather than orbit-and-firing a
//! gun-sword it no longer has the platform to wield). When the rider
//! dies the mount keeps running with its own brain.
//!
//! Any character can be a mount if it carries [`Mountable`] data and
//! any character can be a rider if it has a target to ride. Authored
//! pairs come from two linked LDtk `EnemySpawn`s (a rider with a
//! `mounted_on` entity-ref); [`resolve_pending_mount_links`] installs
//! the runtime link. There is no "shark-rider knowledge" in the engine
//! — the whole relationship is data (ADR 0020).

use bevy::prelude::{
    Commands, Component, Entity, Message, MessageWriter, Query, Res, ResMut, Resource, With,
    Without,
};

use super::brain_builders::dismounted_rider_brain_and_action_set;
use super::CenteredAabb;
use ambition_engine_core as ae;

/// Emitted the frame a mount dies and its rider dismounts (the
/// `(dead-mount, still-mounted)` dissolution in [`enforce_mount_rider_link`]).
/// Carries both entities so a consumer can react to either side.
///
/// This is a body FACT crossing out of the mount coupling — deliberately NOT
/// routed through the `EncounterGate` script bus (that channel is
/// script-vocabulary). The boss-encounter bridge subscribes to turn it into a
/// `mount_died` external phase trigger — the boss whose mount died fights on
/// foot in an authored mini-phase (ADR 0020; Q19). Any other system may
/// subscribe to the same message later without touching this one.
#[derive(Message, Clone, Copy, Debug)]
pub struct MountDied {
    pub mount: Entity,
    pub rider: Entity,
}

/// Physical mass of an actor, used to weight a mount+rider pair's center of
/// gravity. A heavy mount (the shark) keeps the COG near itself so the lighter
/// rider orbits it when the pair rolls under a gravity flip. Authored from the
/// archetype RON (`CharacterArchetypeSpec::mass`), defaulting to 1.0. Lives here with
/// the mount coupling for now; promote to a shared physics location if other
/// systems start consuming it.
#[derive(Component, Clone, Copy, Debug)]
pub struct Mass(pub f32);

impl Default for Mass {
    fn default() -> Self {
        Mass(1.0)
    }
}

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

/// Room-authored mount links awaiting resolution (ADR 0020). Populated by
/// the registry-aware room staging path from `RoomSpec.mount_links` as `(rider_id,
/// mount_id)` [`crate::combat::components::FeatureId`] pairs; drained by
/// [`resolve_pending_mount_links`] once both actors exist. A pair whose
/// entities have not spawned yet is retained for the next frame.
#[derive(Resource, Default, Clone, Debug)]
pub struct PendingMountLinks(pub Vec<(String, String)>);

/// Resolve authored mount links into live `RidingOn`/`MountSlot` connections
/// (ADR 0020). Matches each `(rider_id, mount_id)` pair by `FeatureId`, checks
/// the rider's [`CanPilot`] against the mount's [`Mountable::class`], and
/// installs the link (`RidingOn` + `Mounted` on the rider, `MountSlot.rider`
/// on the mount). An incompatible pair (a rider that cannot pilot that class,
/// or a "mount" with no [`Mountable`]) is dropped with no link; a pair whose
/// entities have not spawned yet is retried next frame. Runs before
/// [`sync_riders_to_mounts`] so a freshly-linked rider welds the same frame.
pub fn resolve_pending_mount_links(
    mut commands: Commands,
    pending: Option<ResMut<PendingMountLinks>>,
    ids: Query<(Entity, &crate::combat::components::FeatureId)>,
    riders: Query<&CanPilot>,
    mounts: Query<&Mountable>,
) {
    let Some(mut pending) = pending else {
        return;
    };
    if pending.0.is_empty() {
        return;
    }
    use std::collections::HashMap;
    let mut by_id: HashMap<&str, Entity> = HashMap::new();
    for (entity, fid) in &ids {
        by_id.insert(fid.as_str(), entity);
    }
    let links = std::mem::take(&mut pending.0);
    let mut unresolved = Vec::new();
    for (rider_id, mount_id) in links {
        let (Some(&rider), Some(&mount)) =
            (by_id.get(rider_id.as_str()), by_id.get(mount_id.as_str()))
        else {
            // One or both actors have not spawned yet — retry next frame.
            unresolved.push((rider_id, mount_id));
            continue;
        };
        // Pilot-compatibility: the rider must be allowed to pilot this mount's
        // class. A missing `CanPilot`, a class mismatch, or a non-mount target
        // drops the link (no silent illegal mount).
        let Ok(mountable) = mounts.get(mount) else {
            continue;
        };
        let allowed = riders
            .get(rider)
            .map(|cp| cp.can_pilot(&mountable.class))
            .unwrap_or(false);
        if !allowed {
            continue;
        }
        commands.entity(rider).insert((RidingOn { mount }, Mounted));
        commands
            .entity(mount)
            .insert(MountSlot { rider: Some(rider) });
    }
    pending.0 = unresolved;
}

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
        (&RidingOn, &ambition_characters::brain::ActorControl),
        (With<Mounted>, Without<MountSlot>),
    >,
    mut mounts: Query<(&Mountable, &mut ambition_characters::brain::ActorControl), With<MountSlot>>,
) {
    for (riding, rider_control) in &riders {
        let Ok((mountable, mut mount_control)) = mounts.get_mut(riding.mount) else {
            continue;
        };
        if mountable.control_grant != ControlGrant::Total {
            continue;
        }
        // Total grant → the mount executes the rider's locomotion intent.
        let rider_frame = rider_control.0;
        let mount_frame = &mut mount_control.0;
        mount_frame.locomotion = rider_frame.locomotion;
        mount_frame.velocity_target = rider_frame.velocity_target;
        mount_frame.facing = rider_frame.facing;
        mount_frame.drop_through = rider_frame.drop_through;
    }
}

/// Lock every rider's position / facing / vel / gravity to its
/// mount each tick. Runs after the per-actor brain tick so the
/// rider's brain has had a chance to emit a fire intent against
/// the target from a position close to where it'll actually be
/// after the snap.
///
/// **Controller-agnostic coupling (M5, ADR 0020 §4):** the pair welds on the
/// STRUCTURAL facts — both bodies are alive and carry their mount-role
/// components — never on disposition. A rider driven by `Brain::Player` (a human
/// piloting the vehicle through possession / the control seam) welds and rides
/// identically to an AI rider; the mount does not care WHO is aboard. Gating on
/// `is_hostile` here would have been exactly the player-centrism the relativity
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
            Option<super::actor_clusters::ActorClusterQueryData>,
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
            &crate::physics::ResolvedMotionFrame,
            Option<super::actor_clusters::ActorClusterQueryData>,
        ),
        With<MountSlot>,
    >,
) {
    for (riding, mut rider_aabb, mounted_size, rider_mass, rider_clusters) in &mut riders {
        let Ok((mountable, mount_mass, mount_frame, mount_clusters)) = mounts.get(riding.mount)
        else {
            continue;
        };
        let Some(mount_c) = mount_clusters else {
            continue;
        };
        if !mount_c.health.alive() {
            continue;
        }
        let Some(mut rider_cq) = rider_clusters else {
            continue;
        };
        let rider = rider_cq.as_actor_mut();
        if !rider.health.alive() {
            continue;
        }
        // Sky-rider size: keep the authored rider footprint stable while the
        // mount is alive. The same footprint remains after dismount; larger
        // cove pirates are separate authored actor spawns.
        if let Some(size) = mounted_size {
            rider.kin.size = size.0;
        }
        // Snap pose to the mount. Vel zeroed so update_ecs_actors'
        // integrator can't drift the rider off the mount on the
        // next frame; gravity zeroed so a Bevy-side integrator that
        // applies gravity to all hostiles can't pull it down.
        //
        // Rotate-as-a-unit: the saddle offset is authored in the mount's local
        // frame, so rotate it into world space by the pair's gravity frame and
        // pivot the rider around the mass-weighted center of gravity. A heavy
        // mount (large `Mass`) keeps the COG near itself, so the lighter rider
        // orbits it on a gravity flip; vertical gravity is identity
        // (`to_world` == I, COG term cancels), so this is byte-identical to the
        // old fixed-offset snap.
        let frame = mount_frame.basis();
        let mass_mount = mount_mass.copied().unwrap_or_default().0.max(0.0001);
        let mass_rider = rider_mass.copied().unwrap_or_default().0.max(0.0001);
        let w_rider = mass_rider / (mass_mount + mass_rider);
        // COG relative to the mount center (mount at 0, rider at `rider_offset`).
        let cog_local = mountable.rider_offset * w_rider;
        let rider_local = cog_local + frame.to_world(mountable.rider_offset - cog_local);
        // ADR 0020 saddle pin = the external-constraint authority (ADR 0024):
        // the mount owns the rider's pose while mounted.
        ae::movement::constrain_body_pose(rider.kin, mount_c.kin.pos + rider_local, ae::Vec2::ZERO);
        rider.kin.facing = mount_c.kin.facing;
        rider.surface.gravity_scale = 0.0;
        rider.ground.on_ground = false;
        // Keep the CenteredAabb mirror in sync so damage / spatial
        // queries on the same tick see the rider where it visually
        // sits. update_ecs_actors writes this from rider.kin.pos at the
        // top of the next tick too, but the same-frame consumers
        // (damage application, projectile origin lookups) need it now.
        rider_aabb.center = rider.kin.pos;
        rider_aabb.half_size = rider.kin.size * 0.5;
    }
}

/// Dissolve a rider / mount link when either side dies. Runs after
/// the damage pass.
///
/// - Mount dies: rider's gravity flips on (so they fall), and their
///   brain + action set are swapped through the shared dismounted
///   rider builder so a pirate falling off a dead shark keeps whatever
///   capabilities their held item grants (gun-sword shots today, axe / bow /
///   bomb authored rows later). The [`RidingOn`]
///   component itself STAYS attached — `sync_riders_to_mounts`
///   gates on `mount.alive` and won't snap the rider while the
///   mount is dead. Keeping the link record lets the same-room
///   reset path re-mount the rider once the mount is alive again
///   without having to look it up by id.
/// - Rider dies: the mount keeps running with its own (already
///   standalone) brain. The mount's [`MountSlot`] keeps its
///   `rider` back-reference so the reset path can re-arm the link.
///
/// The dissolution is idempotent — applying it twice to the same
/// dead-mount situation is a no-op because the second pass sees
/// the rider's brain is already the solo brain. The fired hook
/// is the (transitively-tracked) alive transition, but we don't
/// trust that to fire only once because reset_to_spawn brings
/// `mount.alive` back to true and a future death would mean
/// re-applying the dissolve.
pub fn enforce_mount_rider_link(
    mut commands: Commands,
    roster: Res<crate::features::CharacterRoster>,
    mut mount_died: MessageWriter<MountDied>,
    mut riders: Query<
        (
            Entity,
            &RidingOn,
            &mut CenteredAabb,
            Option<&MountedBrainCache>,
            Option<&Mounted>,
            Option<&super::HeldItem>,
            Option<&super::CombatKit>,
            // A rider whose identity is AUTHORED, not derived from its kit (it
            // carries `BossConfig`), keeps its `Brain` untouched on dismount —
            // no new flag, the component IS the marker (ADR 0020; Q19b).
            Option<&crate::features::BossConfig>,
            Option<super::actor_clusters::ActorClusterQueryData>,
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
    sim_ids: Query<&ambition_platformer_primitives::sim_id::SimId>,
) {
    // Build a lookup of mount alive-ness + death impact. With two-pirate
    // fights this is O(R+M) per frame and the hashmap stays small. Liveness is
    // the STRUCTURAL fact (the mount's HP pool), never disposition — a
    // player-piloted mount dissolves on death the same as an enemy one (M5).
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
        held_item,
        combat_kit,
        boss_config,
        rider_clusters,
    ) in &mut riders
    {
        let Some(mut rider_cq) = rider_clusters else {
            continue;
        };
        let rider = rider_cq.as_actor_mut();
        if !rider.health.alive() {
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
                    rider.surface.gravity_scale = 0.0;
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
                            crate::features::TemporaryControl::Mounted {
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
                    rider.health.damage(amount);
                    // If the splash killed the rider, skip the dismount rebuild —
                    // a dead rider needs no solo brain.
                    if !rider.health.alive() {
                        continue;
                    }
                }
                rider.surface.gravity_scale = if rider.config.tuning.is_aerial {
                    0.0
                } else {
                    1.0
                };
                rider.kin.size = rider.config.spawn.size;
                // Publish immediately so same-frame presentation / combat sees
                // the rider's grounded pose. This is usually the same size as
                // MountedSize; keeping the write here makes intentional future
                // size overrides explicit and safe.
                rider_aabb.center = rider.kin.pos;
                rider_aabb.half_size = rider.kin.size * 0.5;
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
                // Brain swap: rebuild the solo brain/action-set from the rider's
                // DURABLE stored kit — UNLESS the rider's identity is authored (it
                // carries `BossConfig`). A boss's behavior is not derived from a
                // kit, so re-deriving it on dismount would be wrong; it lands on
                // foot still running its authored `Brain`/`BossPattern` (Q19b).
                if boss_config.is_none() {
                    // A rider always carries a CombatKit; fall back defensively.
                    let rider_kit = combat_kit.cloned().unwrap_or_default();
                    let (new_brain, new_action_set) = dismounted_rider_brain_and_action_set(
                        &roster,
                        rider.config,
                        &rider_kit,
                        held_item.map(|item| &item.spec),
                    );
                    commands
                        .entity(rider_entity)
                        .insert((new_brain, new_action_set));
                }
                commands
                    .entity(rider_entity)
                    .remove::<Mounted>()
                    // Back to autonomous control for snapshot purposes (a boss rider
                    // keeps its authored brain but is no longer mount-controlled).
                    .insert(crate::features::TemporaryControl::Autonomous)
                    // Sprite-binding refresh so the rider's sheet
                    // re-resolves on the next presentation pass.
                    .remove::<ambition_platformer_primitives::feature_kind::BoundFeatureKind>();
            }
            // Mount dead, rider already dissolved → steady state.
            (false, false) => {}
        }
    }
}

#[cfg(test)]
mod tests;

/// World position of the rider's hand (where mounted attacks originate). The
/// hand offset is sprite-layout-derived but the SIM needs it to spawn attacks, so
/// it lives here, not in presentation.
const HAND_OFFSET_NORM: ambition_engine_core::Vec2 = ambition_engine_core::Vec2::new(0.18, -0.05);
pub fn rider_hand_world_pos(
    rider_pos: ambition_engine_core::Vec2,
    facing: f32,
    rider_height: f32,
) -> ambition_engine_core::Vec2 {
    rider_hand_world_pos_in_frame(
        rider_pos,
        facing,
        rider_height,
        ambition_engine_core::Vec2::new(0.0, 1.0),
    )
}

/// World position of the rider's hand under the actor's acceleration frame.
/// `facing` is local side-facing, so the hand offset is authored in rider-local
/// side/down coordinates and then resolved to world.
pub fn rider_hand_world_pos_in_frame(
    rider_pos: ambition_engine_core::Vec2,
    facing: f32,
    rider_height: f32,
    gravity_dir: ambition_engine_core::Vec2,
) -> ambition_engine_core::Vec2 {
    let facing_sign = if facing >= 0.0 { 1.0 } else { -1.0 };
    let hand_local = ambition_engine_core::Vec2::new(
        HAND_OFFSET_NORM.x * rider_height * facing_sign,
        HAND_OFFSET_NORM.y * rider_height,
    );
    rider_pos + ambition_engine_core::AccelerationFrame::new(gravity_dir).to_world(hand_local)
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
