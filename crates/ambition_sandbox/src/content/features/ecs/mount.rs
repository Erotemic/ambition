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

#[cfg(test)]
mod tests {
    use super::super::FeatureAabb;
    use super::*;
    use crate::content::features::enemies::EnemyRuntime;
    use bevy::prelude::*;

    fn hostile(id: &str, archetype_brain: &str, pos: ae::Vec2, size: ae::Vec2) -> ActorRuntime {
        let aabb = ae::Aabb::new(pos, size * 0.5);
        let mut enemy = EnemyRuntime::new(
            id,
            id,
            aabb,
            crate::actor::EnemyBrain::Custom(archetype_brain.into()),
            &[],
        );
        enemy.size = size;
        enemy.pos = pos;
        enemy.alive = true;
        ActorRuntime::Hostile(enemy)
    }

    fn build_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app
    }

    /// Sync pose snaps rider.pos to mount.pos + Mountable.rider_offset
    /// and zeroes the rider's velocity each tick.
    #[test]
    fn sync_riders_to_mounts_snaps_rider_to_mount_offset() {
        let mut app = build_app();
        app.add_systems(Update, sync_riders_to_mounts);

        let mount_pos = ae::Vec2::new(100.0, 50.0);
        let mount_size = ae::Vec2::new(126.0, 52.0);
        let mount = app
            .world_mut()
            .spawn((
                hostile("mount", "burning_flying_shark", mount_pos, mount_size),
                FeatureAabb::from_center_size(mount_pos, mount_size),
                Mountable {
                    rider_offset: ae::Vec2::new(0.0, -40.0),
                },
                MountSlot { rider: None },
            ))
            .id();

        // Rider's authored position is something arbitrary; the sync
        // system should snap it to the mount's pos + offset on the
        // first tick.
        let rider_start = ae::Vec2::new(999.0, 999.0);
        let rider_size = ae::Vec2::new(44.0, 78.0);
        let rider = app
            .world_mut()
            .spawn((
                hostile("rider", "pirate_raider", rider_start, rider_size),
                FeatureAabb::from_center_size(rider_start, rider_size),
                RidingOn { mount },
            ))
            .id();
        // Pre-poison rider velocity so the assertion that the sync
        // zeroes it isn't a no-op against the default.
        if let Some(mut actor) = app.world_mut().entity_mut(rider).get_mut::<ActorRuntime>() {
            if let ActorRuntime::Hostile(r) = &mut *actor {
                r.vel = ae::Vec2::new(500.0, -200.0);
                r.gravity_scale = 1.0;
            }
        }

        app.update();

        let actor = app.world().entity(rider).get::<ActorRuntime>().unwrap();
        let ActorRuntime::Hostile(r) = actor else {
            panic!("rider should be Hostile")
        };
        assert_eq!(
            r.pos,
            ae::Vec2::new(100.0, 10.0),
            "rider should snap to mount.pos + offset",
        );
        assert_eq!(r.vel, ae::Vec2::ZERO, "rider vel zeroed by sync");
        assert_eq!(r.gravity_scale, 0.0, "rider gravity zeroed by sync");

        let aabb = app.world().entity(rider).get::<FeatureAabb>().unwrap();
        assert_eq!(aabb.center, r.pos, "FeatureAabb mirror updated to synced pos");
    }

    /// Mount's death releases the rider — RidingOn is removed,
    /// gravity flips on, and the rider's brain is re-derived for its
    /// solo archetype (which for a PirateRaider is the Smash brain).
    #[test]
    fn dead_mount_releases_rider_and_restores_solo_brain() {
        let mut app = build_app();
        app.add_systems(Update, enforce_mount_rider_link);

        // Spawn a mount entity flagged dead.
        let mount_pos = ae::Vec2::new(0.0, 0.0);
        let mount_size = ae::Vec2::new(126.0, 52.0);
        let mut mount_actor = hostile("mount", "burning_flying_shark", mount_pos, mount_size);
        if let ActorRuntime::Hostile(m) = &mut mount_actor {
            m.alive = false;
        }
        let mount = app
            .world_mut()
            .spawn((
                mount_actor,
                Mountable {
                    rider_offset: ae::Vec2::new(0.0, -40.0),
                },
                MountSlot { rider: None },
            ))
            .id();

        // Live rider linked to the dead mount.
        let rider_pos = ae::Vec2::new(0.0, -40.0);
        let rider_size = ae::Vec2::new(44.0, 78.0);
        let rider = app
            .world_mut()
            .spawn((
                hostile("rider", "pirate_raider", rider_pos, rider_size),
                FeatureAabb::from_center_size(rider_pos, rider_size),
                RidingOn { mount },
            ))
            .id();
        // Point MountSlot at the rider, mirroring what the spawn
        // helper would do — so the rider-dies path doesn't trigger
        // here (only mount-died should).
        app.world_mut()
            .entity_mut(mount)
            .insert(MountSlot {
                rider: Some(rider),
            });

        app.update();

        // RidingOn should be removed.
        let still_riding = app.world().entity(rider).get::<RidingOn>();
        assert!(
            still_riding.is_none(),
            "RidingOn should be removed when mount dies",
        );
        // Gravity should flip on for a non-aerial archetype.
        let actor = app.world().entity(rider).get::<ActorRuntime>().unwrap();
        let ActorRuntime::Hostile(r) = actor else {
            panic!()
        };
        assert_eq!(r.gravity_scale, 1.0, "PirateRaider rider gets gravity 1.0");
        // Brain should be the solo PirateRaider's (Smash).
        let brain = app.world().entity(rider).get::<crate::brain::Brain>().unwrap();
        assert!(
            matches!(
                brain,
                crate::brain::Brain::StateMachine(
                    crate::brain::StateMachineCfg::Smash { .. }
                ),
            ),
            "after dismount the rider's solo brain (Smash) should be restored",
        );
        // MountSlot should be cleared back-reference (the mount is
        // dead so it'll be despawned by the normal kill path; here
        // we just verify the back-ref unhook).
        let slot = app.world().entity(mount).get::<MountSlot>().unwrap();
        assert!(
            slot.rider.is_none(),
            "MountSlot.rider should be cleared when mount dies",
        );
    }

    /// Rider's death clears the mount's MountSlot.rider back-
    /// reference; the mount itself keeps running unchanged.
    #[test]
    fn dead_rider_clears_mount_slot() {
        let mut app = build_app();
        app.add_systems(Update, enforce_mount_rider_link);

        let mount_pos = ae::Vec2::new(0.0, 0.0);
        let mount_size = ae::Vec2::new(126.0, 52.0);
        let mount = app
            .world_mut()
            .spawn((
                hostile("mount", "burning_flying_shark", mount_pos, mount_size),
                Mountable {
                    rider_offset: ae::Vec2::new(0.0, -40.0),
                },
                MountSlot { rider: None },
            ))
            .id();

        // Dead rider.
        let mut rider_actor = hostile(
            "rider",
            "pirate_raider",
            ae::Vec2::new(0.0, -40.0),
            ae::Vec2::new(44.0, 78.0),
        );
        if let ActorRuntime::Hostile(r) = &mut rider_actor {
            r.alive = false;
        }
        let rider = app
            .world_mut()
            .spawn((
                rider_actor,
                FeatureAabb::from_center_size(ae::Vec2::new(0.0, -40.0), ae::Vec2::new(44.0, 78.0)),
                RidingOn { mount },
            ))
            .id();
        app.world_mut()
            .entity_mut(mount)
            .insert(MountSlot {
                rider: Some(rider),
            });

        app.update();

        let slot = app.world().entity(mount).get::<MountSlot>().unwrap();
        assert!(
            slot.rider.is_none(),
            "dead rider should clear MountSlot.rider",
        );
    }
}
