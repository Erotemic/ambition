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

use super::super::EnemyArchetype;
use super::brain_builders::dismounted_rider_brain_and_action_set;
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
            &mut FeatureAabb,
            Option<&MountedSize>,
            Option<super::enemy_clusters::EnemyClusterQueryData>,
        ),
        Without<MountSlot>,
    >,
    mounts: Query<
        (
            &ActorRuntime,
            &Mountable,
            Option<super::enemy_clusters::EnemyClusterQueryData>,
        ),
        With<MountSlot>,
    >,
) {
    for (riding, rider_actor, mut rider_aabb, mounted_size, rider_clusters) in &mut riders {
        let Ok((mount_actor, mountable, mount_clusters)) = mounts.get(riding.mount) else {
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
        rider.kin.pos.x = mount_c.kin.pos.x + mountable.rider_offset.x;
        rider.kin.pos.y = mount_c.kin.pos.y + mountable.rider_offset.y;
        rider.kin.facing = mount_c.kin.facing;
        rider.kin.vel = ae::Vec2::ZERO;
        rider.surface.gravity_scale = 0.0;
        rider.surface.on_ground = false;
        // Keep the FeatureAabb mirror in sync so damage / spatial
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
            &mut FeatureAabb,
            Option<&MountedBrainCache>,
            Option<&Mounted>,
            Option<&super::HeldItem>,
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
                // The brain builders only read id + archetype; take the
                // rider's `EnemyConfig` cluster directly.
                let proxy = super::enemy_clusters::EnemyClusterScratch::new(
                    rider.config.id.clone(),
                    rider.config.name.clone(),
                    rider.aabb(),
                    rider.config.brain.clone(),
                    &[],
                )
                .config;
                let (new_brain, new_action_set) =
                    dismounted_rider_brain_and_action_set(&proxy, held_item.map(|item| &item.spec));
                commands
                    .entity(rider_entity)
                    .insert((new_brain, new_action_set))
                    .remove::<Mounted>()
                    // Sprite-binding refresh so the rider's sheet
                    // re-resolves on the next presentation pass.
                    .remove::<crate::presentation::rendering::BoundFeatureKind>();
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
    use bevy::prelude::*;

    type EnemyClusterBundle = (
        super::super::enemy_clusters::BodyKinematics,
        super::super::enemy_clusters::EnemyStatus,
        super::super::enemy_clusters::EnemyConfig,
        super::super::enemy_clusters::ActorMotionPath,
        crate::features::ActorSurfaceState,
        crate::features::ActorAttackState,
        crate::mechanics::combat::CombatCapabilities,
    );

    fn hostile(
        id: &str,
        archetype_brain: &str,
        pos: ae::Vec2,
        size: ae::Vec2,
    ) -> (ActorRuntime, EnemyClusterBundle) {
        let aabb = ae::Aabb::new(pos, size * 0.5);
        let mut enemy = super::super::enemy_clusters::EnemyClusterScratch::new(
            id,
            id,
            aabb,
            crate::actor::EnemyBrain::Custom(archetype_brain.into()),
            &[],
        );
        enemy.kin.size = size;
        enemy.kin.pos = pos;
        enemy.status.alive = true;
        (ActorRuntime::Enemy, enemy.into_components())
    }

    /// Read an entity's enemy kinematics/status/surface from its
    /// cluster components for test assertions.
    fn rider_kin(
        world: &bevy::prelude::World,
        e: bevy::prelude::Entity,
    ) -> super::super::enemy_clusters::BodyKinematics {
        *world
            .entity(e)
            .get::<super::super::enemy_clusters::BodyKinematics>()
            .expect("enemy entity has BodyKinematics")
    }

    fn rider_surface(
        world: &bevy::prelude::World,
        e: bevy::prelude::Entity,
    ) -> crate::features::ActorSurfaceState {
        *world
            .entity(e)
            .get::<crate::features::ActorSurfaceState>()
            .expect("enemy entity has ActorSurfaceState")
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
        app.world_mut()
            .get_mut::<crate::features::BodyKinematics>(rider)
            .unwrap()
            .vel = ae::Vec2::new(500.0, -200.0);
        app.world_mut()
            .get_mut::<crate::features::ActorSurfaceState>(rider)
            .unwrap()
            .gravity_scale = 1.0;

        app.update();

        let k = rider_kin(app.world(), rider);
        let s = rider_surface(app.world(), rider);
        assert_eq!(
            k.pos,
            ae::Vec2::new(100.0, 10.0),
            "rider should snap to mount.pos + offset",
        );
        assert_eq!(k.vel, ae::Vec2::ZERO, "rider vel zeroed by sync");
        assert_eq!(s.gravity_scale, 0.0, "rider gravity zeroed by sync");

        let aabb = app.world().entity(rider).get::<FeatureAabb>().unwrap();
        assert_eq!(
            aabb.center, k.pos,
            "FeatureAabb mirror updated to synced pos"
        );
    }

    /// Helper: spawn a mount + rider pair wired the same way the
    /// composite-spawn helper does, but using a placeholder mounted
    /// brain (Skirmisher with explicit cfg) so the cache check has
    /// something concrete to compare against.
    fn spawn_pair(app: &mut App, mount_alive: bool, rider_alive: bool) -> (Entity, Entity) {
        use crate::brain::{
            ActionSet, Brain, RangedActionSpec, SkirmisherCfg, SkirmisherState, StateMachineCfg,
        };
        let mounted_brain = Brain::StateMachine(StateMachineCfg::Skirmisher {
            cfg: SkirmisherCfg {
                aggressiveness: 1.0,
                aggro_radius: 1200.0,
                standoff_px: 385.0,
                strafe_speed: 230.0,
                fire_cooldown_s: 1.5,
                orbit_drift_rad_s: 0.6,
            },
            state: SkirmisherState::default(),
        });
        let mounted_action_set = ActionSet {
            ranged: Some(RangedActionSpec::Bolt {
                speed: 500.0,
                damage: 2,
            }),
            ..Default::default()
        };
        let mount_pos = ae::Vec2::new(0.0, 0.0);
        let mount_size = ae::Vec2::new(126.0, 52.0);
        let mut mount_actor = hostile("mount", "burning_flying_shark", mount_pos, mount_size);
        // .1 = EnemyClusterBundle, .1.1 = EnemyStatus.
        mount_actor.1 .1.alive = mount_alive;
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

        let rider_pos = ae::Vec2::new(0.0, -40.0);
        let rider_size = ae::Vec2::new(44.0, 78.0);
        let mut rider_actor = hostile("rider", "pirate_raider", rider_pos, rider_size);
        // .1.1 = EnemyStatus, .1.4 = ActorSurfaceState.
        rider_actor.1 .1.alive = rider_alive;
        rider_actor.1 .4.gravity_scale = 0.0;
        let rider = app
            .world_mut()
            .spawn((
                rider_actor,
                FeatureAabb::from_center_size(rider_pos, rider_size),
                mounted_brain.clone(),
                mounted_action_set.clone(),
                MountedBrainCache {
                    brain: mounted_brain,
                    action_set: mounted_action_set,
                },
                Mounted,
                RidingOn { mount },
            ))
            .id();
        app.world_mut()
            .entity_mut(mount)
            .insert(MountSlot { rider: Some(rider) });
        (mount, rider)
    }

    /// Mount's death dissolves the link: rider's gravity flips on,
    /// brain swaps to the solo PirateRaider Smash, and the Mounted
    /// marker is removed. RidingOn + MountSlot stay attached so the
    /// same-room reset path can re-arm the link without an id
    /// lookup.
    #[test]
    fn dead_mount_dissolves_link_keeping_records() {
        let mut app = build_app();
        app.add_systems(Update, enforce_mount_rider_link);
        let (mount, rider) = spawn_pair(&mut app, /*mount_alive*/ false, true);

        app.update();

        assert!(
            app.world().entity(rider).get::<RidingOn>().is_some(),
            "RidingOn stays attached so reset can re-arm without id lookup",
        );
        assert!(
            app.world().entity(rider).get::<Mounted>().is_none(),
            "Mounted marker removed on dissolve",
        );
        assert_eq!(
            rider_surface(app.world(), rider).gravity_scale,
            1.0,
            "PirateRaider rider gets gravity 1.0"
        );
        let brain = app
            .world()
            .entity(rider)
            .get::<crate::brain::Brain>()
            .unwrap();
        assert!(
            matches!(
                brain,
                crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::MeleeBrute { .. }),
            ),
            "after dismount the rider should be MeleeBrute (explicit chase + swipe)",
        );
        let slot = app.world().entity(mount).get::<MountSlot>().unwrap();
        assert!(
            slot.rider.is_some(),
            "MountSlot.rider stays populated so reset can re-arm",
        );
    }

    /// Same-room reset re-arms the link: starting from a dissolved
    /// state (mount dead, rider with solo brain), once the mount's
    /// `alive` flag is set back to true the enforcer restores the
    /// MOUNTED brain + Mounted marker + zero gravity on the rider.
    #[test]
    fn reviving_mount_re_arms_rider_to_mounted_brain() {
        let mut app = build_app();
        app.add_systems(Update, enforce_mount_rider_link);
        let (mount, rider) = spawn_pair(&mut app, /*mount_alive*/ false, true);

        // First tick: dissolve.
        app.update();
        assert!(app.world().entity(rider).get::<Mounted>().is_none());

        // Simulate the same-room reset: flip mount.alive back to
        // true (reset_to_spawn would do this). The enforcer should
        // re-arm the link on the next tick.
        app.world_mut()
            .get_mut::<crate::features::EnemyStatus>(mount)
            .unwrap()
            .alive = true;
        app.update();

        assert!(
            app.world().entity(rider).get::<Mounted>().is_some(),
            "Mounted marker should be re-added on revive",
        );
        assert_eq!(
            rider_surface(app.world(), rider).gravity_scale,
            0.0,
            "rider gravity should be zeroed back to mounted state",
        );
        let brain = app
            .world()
            .entity(rider)
            .get::<crate::brain::Brain>()
            .unwrap();
        assert!(
            matches!(
                brain,
                crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::Skirmisher { .. }),
            ),
            "after revive the rider's mounted brain (Skirmisher) should be restored",
        );
    }

    /// Dead rider leaves the link records in place — the mount keeps
    /// its MountSlot back-reference (no re-arming needed since the
    /// rider's just dead, not transitioning). Mount stays alive.
    #[test]
    fn dead_rider_does_not_disturb_mount_records() {
        let mut app = build_app();
        app.add_systems(Update, enforce_mount_rider_link);
        let (mount, _rider) = spawn_pair(
            &mut app, /*mount_alive*/ true, /*rider_alive*/ false,
        );

        app.update();

        let slot = app.world().entity(mount).get::<MountSlot>().unwrap();
        assert!(
            slot.rider.is_some(),
            "MountSlot.rider stays populated even with dead rider",
        );
        assert!(
            app.world()
                .entity(mount)
                .get::<crate::features::EnemyStatus>()
                .unwrap()
                .alive,
            "mount stays alive when rider dies"
        );
    }
}
