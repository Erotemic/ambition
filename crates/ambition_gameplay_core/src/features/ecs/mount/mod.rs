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
//! any character can be a rider if it has a target to ride. The
//! composite spawn helper [`spawn_mount_rider_pair`] is the only
//! "shark-rider knowledge" in the runtime — everything else is
//! generic.

use bevy::prelude::{Commands, Component, Entity, Query, With, Without};

use super::brain_builders::dismounted_rider_brain_and_action_set;
use super::{ActorRuntime, CenteredAabb};
use crate::engine_core as ae;

/// Physical mass of an actor, used to weight a mount+rider pair's center of
/// gravity. A heavy mount (the shark) keeps the COG near itself so the lighter
/// rider orbits it when the pair rolls under a gravity flip. Authored from the
/// archetype RON (`EnemyArchetypeSpec::mass`), defaulting to 1.0. Lives here with
/// the mount coupling for now; promote to a shared physics location if other
/// systems start consuming it.
#[derive(Component, Clone, Copy, Debug)]
pub struct Mass(pub f32);

impl Default for Mass {
    fn default() -> Self {
        Mass(1.0)
    }
}

/// Attached to a mount entity. Specifies where the rider rides
/// relative to the mount's center (sandbox units; y grows downward).
#[derive(Component, Clone, Copy, Debug)]
pub struct Mountable {
    /// Rider's center offset from the mount's center. For an
    /// aerial mount this is typically `(0, -mount.size.y * 0.5 -
    /// rider.size.y * 0.5 + epsilon)` so the rider sits on the
    /// mount's saddle without their hitboxes overlapping.
    pub rider_offset: ae::Vec2,
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
    pub brain: crate::brain::Brain,
    pub action_set: crate::brain::ActionSet,
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

/// Lock every rider's position / facing / vel / gravity to its
/// mount each tick. Runs after the per-actor brain tick so the
/// rider's brain has had a chance to emit a fire intent against
/// the target from a position close to where it'll actually be
/// after the snap.
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
            &ActorRuntime,
            &mut CenteredAabb,
            Option<&MountedSize>,
            Option<&Mass>,
            Option<super::enemy_clusters::EnemyClusterQueryData>,
        ),
        Without<MountSlot>,
    >,
    mounts: Query<
        (
            &ActorRuntime,
            &Mountable,
            Option<&Mass>,
            Option<super::enemy_clusters::EnemyClusterQueryData>,
        ),
        With<MountSlot>,
    >,
    // Per-position gravity so the saddle offset rotates with the pair's reference
    // frame (the rider orbits the mount under a gravity flip instead of floating
    // off the saddle in fixed screen space).
    gravity: crate::physics::GravityCtx,
) {
    for (riding, rider_actor, mut rider_aabb, mounted_size, rider_mass, rider_clusters) in
        &mut riders
    {
        let Ok((mount_actor, mountable, mount_mass, mount_clusters)) = mounts.get(riding.mount)
        else {
            continue;
        };
        if !matches!(mount_actor, ActorRuntime::Enemy) {
            continue;
        }
        let Some(mount_c) = mount_clusters else {
            continue;
        };
        if !mount_c.status.alive {
            continue;
        }
        if !matches!(rider_actor, ActorRuntime::Enemy) {
            continue;
        }
        let Some(mut rider_cq) = rider_clusters else {
            continue;
        };
        let rider = rider_cq.as_enemy_mut();
        if !rider.status.alive {
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
        let frame = ae::AccelerationFrame::new(gravity.dir_at(mount_c.kin.pos));
        let mass_mount = mount_mass.copied().unwrap_or_default().0.max(0.0001);
        let mass_rider = rider_mass.copied().unwrap_or_default().0.max(0.0001);
        let w_rider = mass_rider / (mass_mount + mass_rider);
        // COG relative to the mount center (mount at 0, rider at `rider_offset`).
        let cog_local = mountable.rider_offset * w_rider;
        let rider_local = cog_local + frame.to_world(mountable.rider_offset - cog_local);
        rider.kin.pos = mount_c.kin.pos + rider_local;
        rider.kin.facing = mount_c.kin.facing;
        rider.kin.vel = ae::Vec2::ZERO;
        rider.surface.gravity_scale = 0.0;
        rider.surface.on_ground = false;
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
    mut riders: Query<
        (
            Entity,
            &RidingOn,
            &ActorRuntime,
            &mut CenteredAabb,
            Option<&MountedBrainCache>,
            Option<&Mounted>,
            Option<&super::HeldItem>,
            Option<&super::CombatKit>,
            Option<super::enemy_clusters::EnemyClusterQueryData>,
        ),
        Without<MountSlot>,
    >,
    mounts: Query<
        (
            Entity,
            &ActorRuntime,
            Option<&super::enemy_clusters::EnemyStatus>,
        ),
        With<MountSlot>,
    >,
) {
    // Build a lookup of mount alive-ness. With two-pirate fights
    // this is O(R+M) per frame and the hashmap stays small.
    use std::collections::HashMap;
    let mut mount_alive: HashMap<Entity, bool> = HashMap::new();
    for (mount_entity, mount_actor, mount_status) in &mounts {
        let alive =
            matches!(mount_actor, ActorRuntime::Enemy) && mount_status.is_some_and(|s| s.alive);
        mount_alive.insert(mount_entity, alive);
    }

    for (
        rider_entity,
        riding,
        rider_actor,
        mut rider_aabb,
        cache,
        was_mounted,
        held_item,
        combat_kit,
        rider_clusters,
    ) in &mut riders
    {
        if !matches!(rider_actor, ActorRuntime::Enemy) {
            continue;
        }
        let Some(mut rider_cq) = rider_clusters else {
            continue;
        };
        let rider = rider_cq.as_enemy_mut();
        if !rider.status.alive {
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
                }
            }
            // Mount dead, rider currently mounted → dissolve. Flip gravity on,
            // keep the rider at its authored sky-rider size, and install the
            // shared explicitly-hostile dismounted rider brain/action-set policy
            // so a PirateRaider / PirateHeavy variant falls and fights without
            // visually scaling up.
            (false, true) => {
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
                // Rebuild from the rider's DURABLE stored combat kit (the
                // same data the archetype projected at spawn) so dismount
                // never re-reads the roster enum. A rider always carries a
                // CombatKit; fall back to an empty kit defensively.
                let rider_kit = combat_kit.cloned().unwrap_or_default();
                let (new_brain, new_action_set) = dismounted_rider_brain_and_action_set(
                    rider.config,
                    &rider_kit,
                    held_item.map(|item| &item.spec),
                );
                commands
                    .entity(rider_entity)
                    .insert((new_brain, new_action_set))
                    .remove::<Mounted>()
                    // Sprite-binding refresh so the rider's sheet
                    // re-resolves on the next presentation pass.
                    .remove::<crate::mechanics::combat::BoundFeatureKind>();
            }
            // Mount dead, rider already dissolved → steady state.
            (false, false) => {}
        }
    }
}

/// Sandbox-units offset for a pirate-on-shark composite. The rider
/// sits directly above the mount's body with a small overlap so the
/// pirate visually rests on the shark's saddle rather than floating.
///
/// `mount_size` is the mount body size and `rider_size` is the authored
/// sky-rider size; the offset uses the half-heights so the rider's bottom edge sits
/// at the mount's top edge plus 8 px of overlap (matches the legacy
/// fused `rider_aabb` placement).
pub fn pirate_on_shark_rider_offset(mount_size: ae::Vec2, rider_size: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new(0.0, -(mount_size.y * 0.5) - (rider_size.y * 0.5) + 8.0)
}

#[cfg(test)]
mod tests;

/// World position of the rider's hand (where mounted attacks originate). The
/// hand offset is sprite-layout-derived but the SIM needs it to spawn attacks, so
/// it lives here, not in presentation.
const HAND_OFFSET_NORM: crate::engine_core::Vec2 = crate::engine_core::Vec2::new(0.18, -0.05);
pub fn rider_hand_world_pos(
    rider_pos: crate::engine_core::Vec2,
    facing: f32,
    rider_height: f32,
) -> crate::engine_core::Vec2 {
    let facing_sign = if facing >= 0.0 { 1.0 } else { -1.0 };
    let hand_local_x = HAND_OFFSET_NORM.x * rider_height * facing_sign;
    let hand_local_y = HAND_OFFSET_NORM.y * rider_height;
    crate::engine_core::Vec2::new(rider_pos.x + hand_local_x, rider_pos.y + hand_local_y)
}
