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
/// - Mount dies: rider's gravity flips on (so they fall), and their
///   brain + action set are re-derived for the rider's STANDALONE
///   archetype so a pirate raider falling off a dead shark walks
///   at the player swinging melee instead of orbit-and-firing a
///   gun-sword that no longer has a platform. The [`RidingOn`]
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
            &mut ActorRuntime,
            Option<&MountedBrainCache>,
            Option<&Mounted>,
        ),
        Without<MountSlot>,
    >,
    mounts: Query<(Entity, &ActorRuntime), With<MountSlot>>,
) {
    // Build a lookup of mount alive-ness. With two-pirate fights
    // this is O(R+M) per frame and the hashmap stays small.
    use std::collections::HashMap;
    let mut mount_alive: HashMap<Entity, bool> = HashMap::new();
    for (mount_entity, mount_actor) in &mounts {
        let alive = matches!(mount_actor, ActorRuntime::Hostile(m) if m.alive);
        mount_alive.insert(mount_entity, alive);
    }

    for (rider_entity, riding, mut rider_actor, cache, was_mounted) in &mut riders {
        let ActorRuntime::Hostile(rider) = &mut *rider_actor else {
            continue;
        };
        if !rider.alive {
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
                    rider.gravity_scale = 0.0;
                    commands.entity(rider_entity).insert((
                        cache.brain.clone(),
                        cache.action_set.clone(),
                        Mounted,
                    ));
                }
            }
            // Mount dead, rider currently mounted → dissolve. Flip
            // gravity on, install the rider's solo brain + action
            // set so a PirateRaider falls and walks at the player
            // swinging melee.
            //
            // Aggressiveness is forced ON after the rebuild — a
            // PirateHeavy variant (peaceful Cove crew when
            // standalone) should keep fighting after the shark dies
            // under her.
            (false, true) => {
                rider.gravity_scale = if rider.archetype.is_aerial() {
                    0.0
                } else {
                    1.0
                };
                let mut new_brain = super::spawn::enemy_default_brain(rider);
                force_hostile(&mut new_brain);
                let new_action_set = super::spawn::enemy_default_action_set(rider);
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

/// Set every aggressiveness-carrying brain cfg's aggressiveness to
/// 1.0 in place. Used by [`enforce_mount_rider_link`] to keep a
/// dismounted rider hostile when their archetype's default
/// aggression is 0 (PirateHeavy is authored as peaceful Cove crew).
fn force_hostile(brain: &mut crate::brain::Brain) {
    use crate::brain::{Brain, StateMachineCfg};
    let Brain::StateMachine(cfg) = brain else {
        return;
    };
    match cfg {
        StateMachineCfg::Patrol { cfg, .. } => cfg.aggressiveness = 1.0,
        StateMachineCfg::Wanderer { cfg, .. } => cfg.aggressiveness = 1.0,
        StateMachineCfg::MeleeBrute { cfg, .. } => cfg.aggressiveness = 1.0,
        StateMachineCfg::Skirmisher { cfg, .. } => cfg.aggressiveness = 1.0,
        StateMachineCfg::Sniper { cfg, .. } => cfg.aggressiveness = 1.0,
        // StandStill / Smash / BossPattern carry no aggressiveness
        // field; the brain template itself is hostile-by-construction
        // (Smash) or scripted (BossPattern).
        _ => {}
    }
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
        if let ActorRuntime::Hostile(m) = &mut mount_actor {
            m.alive = mount_alive;
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

        let rider_pos = ae::Vec2::new(0.0, -40.0);
        let rider_size = ae::Vec2::new(44.0, 78.0);
        let mut rider_actor = hostile("rider", "pirate_raider", rider_pos, rider_size);
        if let ActorRuntime::Hostile(r) = &mut rider_actor {
            r.alive = rider_alive;
            r.gravity_scale = 0.0;
        }
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
            .insert(MountSlot {
                rider: Some(rider),
            });
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
        let actor = app.world().entity(rider).get::<ActorRuntime>().unwrap();
        let ActorRuntime::Hostile(r) = actor else {
            panic!()
        };
        assert_eq!(r.gravity_scale, 1.0, "PirateRaider rider gets gravity 1.0");
        let brain = app.world().entity(rider).get::<crate::brain::Brain>().unwrap();
        assert!(
            matches!(
                brain,
                crate::brain::Brain::StateMachine(
                    crate::brain::StateMachineCfg::Smash { .. }
                ),
            ),
            "after dismount the rider's solo brain (Smash) should be installed",
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
        if let Some(mut actor) = app.world_mut().entity_mut(mount).get_mut::<ActorRuntime>() {
            if let ActorRuntime::Hostile(m) = &mut *actor {
                m.alive = true;
            }
        }
        app.update();

        assert!(
            app.world().entity(rider).get::<Mounted>().is_some(),
            "Mounted marker should be re-added on revive",
        );
        let actor = app.world().entity(rider).get::<ActorRuntime>().unwrap();
        let ActorRuntime::Hostile(r) = actor else {
            panic!()
        };
        assert_eq!(
            r.gravity_scale, 0.0,
            "rider gravity should be zeroed back to mounted state",
        );
        let brain = app.world().entity(rider).get::<crate::brain::Brain>().unwrap();
        assert!(
            matches!(
                brain,
                crate::brain::Brain::StateMachine(
                    crate::brain::StateMachineCfg::Skirmisher { .. }
                ),
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
        let (mount, _rider) =
            spawn_pair(&mut app, /*mount_alive*/ true, /*rider_alive*/ false);

        app.update();

        let slot = app.world().entity(mount).get::<MountSlot>().unwrap();
        assert!(
            slot.rider.is_some(),
            "MountSlot.rider stays populated even with dead rider",
        );
        let mount_actor = app.world().entity(mount).get::<ActorRuntime>().unwrap();
        let ActorRuntime::Hostile(m) = mount_actor else {
            panic!()
        };
        assert!(m.alive, "mount stays alive when rider dies");
    }
}
