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
//! its brain + action set are re-derived from its STANDALONE
//! archetype (so a pirate falling off a dead shark walks toward the
//! player and swings melee, rather than orbit-and-firing a gun-sword
//! it no longer has the platform to wield). When the rider dies the
//! mount keeps running with its own (already-standalone) brain.
//!
//! Any character can be a mount if it carries [`Mountable`] data and
//! any character can be a rider if it has a target to ride. The
//! composite spawn helper [`spawn_mount_rider_pair`] is the only
//! "shark-rider knowledge" in the runtime — everything else is
//! generic.

use bevy::prelude::{Commands, Component, Entity, Query, With, Without};

use super::super::EnemyArchetype;
use super::{ActorRuntime, FeatureAabb};
use crate::engine_core as ae;

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
#[derive(Component, Clone, Copy, Debug)]
pub struct RidingOn {
    pub mount: Entity,
}

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
    mut riders: Query<(&RidingOn, &mut ActorRuntime, &mut FeatureAabb), Without<MountSlot>>,
    mounts: Query<(&ActorRuntime, &Mountable), With<MountSlot>>,
) {
    for (riding, mut rider_actor, mut rider_aabb) in &mut riders {
        let Ok((mount_actor, mountable)) = mounts.get(riding.mount) else {
            continue;
        };
        let ActorRuntime::Hostile(mount) = mount_actor else {
            continue;
        };
        if !mount.alive {
            continue;
        }
        let ActorRuntime::Hostile(rider) = &mut *rider_actor else {
            continue;
        };
        if !rider.alive {
            continue;
        }
        // Snap pose to the mount. Vel zeroed so update_ecs_actors'
        // integrator can't drift the rider off the mount on the
        // next frame; gravity zeroed so a Bevy-side integrator that
        // applies gravity to all hostiles can't pull it down.
        rider.pos.x = mount.pos.x + mountable.rider_offset.x;
        rider.pos.y = mount.pos.y + mountable.rider_offset.y;
        rider.facing = mount.facing;
        rider.vel = ae::Vec2::ZERO;
        rider.gravity_scale = 0.0;
        rider.on_ground = false;
        // Keep the FeatureAabb mirror in sync so damage / spatial
        // queries on the same tick see the rider where it visually
        // sits. update_ecs_actors writes this from rider.pos at the
        // top of the next tick too, but the same-frame consumers
        // (damage application, projectile origin lookups) need it
        // now.
        rider_aabb.center = rider.pos;
        rider_aabb.half_size = rider.size * 0.5;
    }
}

/// Dissolve a rider / mount link when either side dies. Runs after
/// the damage pass.
///
/// - Mount dies: rider's [`RidingOn`] is removed, gravity flips on,
///   and the rider's brain + action set are re-derived for its
///   STANDALONE archetype so a pirate raider falling off a dead
///   shark walks at the player swinging melee instead of orbit-and-
///   firing a gun-sword that no longer has a platform.
/// - Rider dies: the mount's [`MountSlot`] clears. The mount's own
///   brain (already its solo brain — the shark is just a Burning
///   Flying Shark) keeps running unchanged.
pub fn enforce_mount_rider_link(
    mut commands: Commands,
    mut riders: Query<(Entity, &RidingOn, &mut ActorRuntime), Without<MountSlot>>,
    mut mounts: Query<(Entity, &mut MountSlot, &ActorRuntime), With<MountSlot>>,
) {
    // First: dead mounts release their riders. Iterate mounts so we
    // can clear MountSlot.rider authoritatively from the mount side.
    let mut released_riders: Vec<Entity> = Vec::new();
    for (_mount_entity, mut slot, mount_actor) in &mut mounts {
        let mount_dead = match mount_actor {
            ActorRuntime::Hostile(m) => !m.alive,
            _ => true,
        };
        if !mount_dead {
            continue;
        }
        if let Some(rider) = slot.rider.take() {
            released_riders.push(rider);
        }
    }
    // Apply rider-side cleanup for released riders.
    for rider_entity in released_riders {
        let Ok((_, _, mut rider_actor)) = riders.get_mut(rider_entity) else {
            continue;
        };
        commands.entity(rider_entity).remove::<RidingOn>();
        let ActorRuntime::Hostile(rider) = &mut *rider_actor else {
            continue;
        };
        rider.gravity_scale = if rider.archetype.is_aerial() {
            0.0
        } else {
            1.0
        };
        // Re-derive solo brain + action set. The rider's archetype
        // is whatever the standalone form was (e.g. PirateRaider);
        // `enemy_default_brain` / `enemy_default_action_set` read
        // the spec table and return the right melee / Smash config
        // for that archetype.
        let new_brain = super::spawn::enemy_default_brain(rider);
        let new_action_set = super::spawn::enemy_default_action_set(rider);
        commands
            .entity(rider_entity)
            .insert((new_brain, new_action_set))
            // Sprite-binding refresh — same trick the legacy fused
            // dismount used. The rider entity might already have
            // the right BoundFeatureKind; removing it forces a
            // re-resolve on the next presentation pass and is
            // cheap.
            .remove::<crate::presentation::rendering::BoundFeatureKind>();
    }
    // Second: dead riders clear MountSlot on their mounts. Iterate
    // mounts again to update the slot reference. (Dead-rider
    // entities themselves are despawned by the standard kill path;
    // we just unhook the back-reference.)
    let mut dead_rider_entities: Vec<Entity> = Vec::new();
    for (rider_entity, _riding, rider_actor) in &riders {
        let dead = match rider_actor {
            ActorRuntime::Hostile(r) => !r.alive,
            _ => true,
        };
        if dead {
            dead_rider_entities.push(rider_entity);
        }
    }
    for (_mount_entity, mut slot, _mount_actor) in &mut mounts {
        if let Some(rider) = slot.rider {
            if dead_rider_entities.contains(&rider) {
                slot.rider = None;
            }
        }
    }
}

/// Sandbox-units offset for a pirate-on-shark composite. The rider
/// sits directly above the mount's body with a small overlap so the
/// pirate visually rests on the shark's saddle rather than floating.
///
/// `mount_size` and `rider_size` are the standalone authored sizes;
/// the offset uses the half-heights so the rider's bottom edge sits
/// at the mount's top edge plus 8 px of overlap (matches the legacy
/// fused `rider_aabb` placement).
pub fn pirate_on_shark_rider_offset(mount_size: ae::Vec2, rider_size: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new(
        0.0,
        -(mount_size.y * 0.5) - (rider_size.y * 0.5) + 8.0,
    )
}

/// Predicate used by composite-spawn callers to recognize the
/// authored "X on Shark" archetypes. The runtime no longer stores
/// these archetypes on an entity — they're a spawn-time tag that
/// fans out to a mount + rider pair.
pub fn is_composite_spawn(archetype: EnemyArchetype) -> bool {
    matches!(
        archetype,
        EnemyArchetype::PirateOnShark | EnemyArchetype::PirateHeavyOnShark
    )
}
