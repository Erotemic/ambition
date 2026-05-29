//! Hitbox-entity lifecycle: spawn → overlap-check → despawn.
//!
//! Per the actor/brain follow-up plan
//! (`dev/journals/actor-brain-migration-followups-plan.md`, Task A):
//! enemy melee strikes were resolved by per-tick polling inside
//! `update_ecs_actors` (calling `enemy.player_damage(player_body)`
//! every frame the attack_timer was hot). That bypass made melee
//! the only attack family that didn't flow through the actor/brain
//! → ActorActionMessage → EFFECTS-consumer seam.
//!
//! This module replaces the poll with explicit entities:
//!
//! - `update_ecs_actors` detects the windup → active edge and
//!   spawns one `(Hitbox, HitboxLifetime, HitboxHits)` entity per
//!   strike using the strike's per-archetype AABB.
//! - `apply_hitbox_damage` (this module) tests overlap against the
//!   target faction's hurtboxes each tick, emits the matching
//!   damage event, and inserts hit targets into `HitboxHits` so a
//!   long active window can't double-hit the same target.
//! - `tick_and_despawn_hitboxes` (this module) advances every
//!   hitbox's lifetime and despawns expired ones.
//!
//! `HitboxAnchor::FollowOwner` re-resolves the hitbox AABB each
//! tick from the owner entity's position, so a moving attacker's
//! swing tracks the actor without a per-frame component update.
//! `HitboxAnchor::World` (Task B groundwork) is a fixed
//! world-space rectangle for hazards / boss specials.

use bevy::prelude::{Commands, Component, Entity, MessageWriter, Query, Res, With};
use std::collections::HashSet;

use crate::engine_core as ae;
use crate::engine_core::AabbExt;

use super::super::components::ActorFaction;
use super::super::events::{HitEvent, HitKnockback, HitMode, HitSource, HitTarget};
use super::super::util::midpoint;
use crate::audio::SfxMessage;
use crate::presentation::fx::{ParticleKind, VfxMessage};
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use crate::WorldTime;

/// One in-flight strike's damage volume. Spawned on the windup →
/// active edge of an attack; despawned when its `HitboxLifetime`
/// expires.
#[derive(Component, Clone, Debug)]
pub struct Hitbox {
    /// Entity that spawned the hitbox (skip self-hits, look up the
    /// follow anchor's world position each tick).
    pub owner: Entity,
    /// Whose attack is this? Picks the target query in
    /// `apply_hitbox_damage`.
    pub source: ActorFaction,
    /// FollowOwner re-resolves the AABB each tick from the owner's
    /// authoritative position. World is fixed world-space.
    pub anchor: HitboxAnchor,
    pub half_extent: ae::Vec2,
    pub damage: i32,
    pub knockback_strength: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum HitboxAnchor {
    /// Melee swing — the hitbox tracks the owner's `pos` each tick
    /// with a per-strike local offset baked at spawn time. Facing
    /// is encoded in `local_offset.x`'s sign so a flipped attacker
    /// doesn't need a per-frame re-spawn.
    FollowOwner { local_offset: ae::Vec2 },
    /// Arena hazard / boss special — fixed world-space rectangle.
    /// Used by Task B's apple-rain / spotlight effect consumers.
    #[allow(dead_code)]
    World { center: ae::Vec2 },
}

#[derive(Component, Clone, Copy, Debug)]
pub struct HitboxLifetime {
    pub remaining_s: f32,
}

/// Hit-once set: targets the hitbox already damaged this strike.
/// Stops a long active window from re-hitting a stationary target
/// every frame (the old polled path leaned on player iframes for
/// this; the explicit set removes the assumption).
#[derive(Component, Default, Debug)]
pub struct HitboxHits {
    pub hit: HashSet<Entity>,
}

impl Hitbox {
    /// Re-resolve this hitbox's world-space AABB. Computed every
    /// tick rather than mirrored on the entity so a moving owner
    /// doesn't need a per-frame component update.
    pub fn world_aabb(&self, owner_pos: ae::Vec2) -> ae::Aabb {
        let center = match self.anchor {
            HitboxAnchor::FollowOwner { local_offset } => owner_pos + local_offset,
            HitboxAnchor::World { center } => center,
        };
        // `Aabb::new` (Bevy `Aabb2d`) takes `(center, half_size)`.
        ae::Aabb::new(center, self.half_extent)
    }
}

/// Apply each live hitbox's damage to the right faction's targets.
///
/// Enemy / Boss hitboxes hit the player and emit `HitEvent` with a
/// victim-side `HitSource`. Player / Npc / Neutral hitboxes are
/// routed through other paths (player slash flows as
/// `HitSource::PlayerSlash`); this system is the catch-all for
/// hostile melee.
pub fn apply_hitbox_damage(
    mut hitboxes: Query<(Entity, &Hitbox, &mut HitboxHits)>,
    owners: Query<&super::super::components::FeatureAabb>,
    // Iterate every player so a multi-player build hits each
    // overlapping player independently. Single-player behavior is
    // preserved because the iterator has exactly one entity today.
    player_query: Query<
        (
            Entity,
            &crate::player::PlayerKinematics,
            &crate::player::PlayerOffense,
            &crate::player::PlayerDodgeState,
            &crate::player::PlayerShieldState,
            &crate::player::PlayerCombatState,
        ),
        bevy::prelude::With<crate::player::PlayerEntity>,
    >,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut hit_events: MessageWriter<HitEvent>,
) {
    for (_hitbox_entity, hitbox, mut hits) in &mut hitboxes {
        let owner_pos = match owners.get(hitbox.owner) {
            Ok(aabb) => aabb.center,
            // Owner despawned this frame — leave the hitbox as a
            // ghost; `tick_and_despawn_hitboxes` will clean it up
            // when its lifetime expires. Don't apply damage from
            // an owner-less hitbox; the source position can't be
            // resolved sensibly.
            Err(_) => continue,
        };
        let world_aabb = hitbox.world_aabb(owner_pos);

        match hitbox.source {
            ActorFaction::Enemy | ActorFaction::Boss => {
                // Iterate every player and emit one HitEvent per
                // overlapping vulnerable player. `HitboxHits`
                // tracks which players this hitbox has already
                // damaged so a long active window doesn't double-
                // tap a stationary player.
                for (player_entity, kin, offense, dodge, shield, combat) in &player_query {
                    let player_body = kin.aabb();
                    let dodge_rolling = dodge.roll_timer > 0.0;
                    let player_vulnerable = !offense.invincible
                        && !dodge_rolling
                        && !shield.parrying()
                        && combat.vulnerable();
                    if !player_vulnerable {
                        continue;
                    }
                    if hits.hit.contains(&player_entity) {
                        continue;
                    }
                    if !world_aabb.strict_intersects(player_body) {
                        continue;
                    }
                    let impact = midpoint(player_body.center(), world_aabb.center());
                    let knockback_dir = if player_body.center().x >= owner_pos.x {
                        1.0
                    } else {
                        -1.0
                    };
                    let source_kind = if matches!(hitbox.source, ActorFaction::Boss) {
                        HitSource::BossAttack
                    } else {
                        HitSource::EnemyAttack
                    };
                    sfx.write(SfxMessage::Play {
                        id: ambition_sfx::ids::PLAYER_DAMAGE,
                        pos: impact,
                    });
                    vfx.write(VfxMessage::Impact { pos: impact });
                    vfx.write(VfxMessage::Burst {
                        pos: impact,
                        count: 14,
                        speed: 300.0,
                        color: [1.0, 0.34, 0.28, 0.88],
                        kind: ParticleKind::Shard,
                    });
                    debris.write(DebrisBurstMessage {
                        pos: impact,
                        cue: PhysicsDebrisCue::Impact,
                    });
                    hit_events.write(HitEvent {
                        volume: world_aabb,
                        damage: hitbox.damage.max(1),
                        source: source_kind,
                        // Enemy / boss hitboxes know their owner — the
                        // entity that spawned the hitbox is the
                        // attacker. Read on the player side to
                        // attribute hitstun to the right attacker.
                        attacker: Some(hitbox.owner),
                        // Stamp the victim so the player-damage
                        // reader doesn't fall back to primary.
                        target: HitTarget::Player(player_entity),
                        mode: HitMode::Knockback,
                        knockback: Some(HitKnockback {
                            dir: knockback_dir,
                            strength: hitbox.knockback_strength.max(0.0),
                            source_pos: owner_pos,
                            impact_pos: impact,
                        }),
                        ignored_targets: Vec::new(),
                    });
                    hits.hit.insert(player_entity);
                }
            }
            // Player / Npc / Neutral hitboxes: player slash damage
            // still flows through the legacy HitEvent path
            // (see `attack_advance_system`); peaceful NPC / neutral
            // factions don't spawn hitboxes today. This branch is a
            // no-op so future migrations can add cases without a
            // schema change.
            ActorFaction::Player | ActorFaction::Npc | ActorFaction::Neutral => {}
        }
    }
}

/// Advance every hitbox's lifetime by `world_time.sim_dt()` and
/// despawn the ones that hit zero. Sim-clock so bullet-time freezes
/// in-flight hitboxes alongside the rest of combat (ADR 0010).
pub fn tick_and_despawn_hitboxes(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    mut hitboxes: Query<(Entity, &mut HitboxLifetime), With<Hitbox>>,
) {
    let dt = world_time.sim_dt();
    for (entity, mut lifetime) in &mut hitboxes {
        lifetime.remaining_s -= dt;
        if lifetime.remaining_s <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawn helper: emit a fresh hitbox entity for a melee strike. The
/// caller picks the local offset / half-extent / damage / faction
/// based on the strike's archetype + facing.
pub fn spawn_melee_hitbox(
    commands: &mut Commands,
    owner: Entity,
    source: ActorFaction,
    local_offset: ae::Vec2,
    half_extent: ae::Vec2,
    damage: i32,
    knockback_strength: f32,
    active_s: f32,
) -> Entity {
    commands
        .spawn((
            Hitbox {
                owner,
                source,
                anchor: HitboxAnchor::FollowOwner { local_offset },
                half_extent,
                damage,
                knockback_strength,
            },
            HitboxLifetime {
                remaining_s: active_s.max(0.0),
            },
            HitboxHits::default(),
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    fn dummy_entity() -> Entity {
        Entity::from_raw_u32(42).expect("nonzero raw entity index")
    }

    /// FollowOwner anchor re-resolves position each tick: moving
    /// the owner moves the hitbox without per-frame component update.
    #[test]
    fn follow_owner_hitbox_aabb_tracks_owner_position() {
        let hitbox = Hitbox {
            owner: dummy_entity(),
            source: ActorFaction::Enemy,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::new(-20.0, 0.0),
            },
            half_extent: ae::Vec2::new(10.0, 10.0),
            damage: 1,
            knockback_strength: 0.0,
        };
        let aabb_a = hitbox.world_aabb(ae::Vec2::new(100.0, 100.0));
        let aabb_b = hitbox.world_aabb(ae::Vec2::new(200.0, 100.0));
        assert_eq!(aabb_a.center(), ae::Vec2::new(80.0, 100.0));
        assert_eq!(aabb_b.center(), ae::Vec2::new(180.0, 100.0));
        // Half-extent translates into a full-size AABB; the local
        // offset doesn't change shape.
        assert_eq!(aabb_a.half_size(), ae::Vec2::new(10.0, 10.0));
    }

    /// World anchor is a fixed world rectangle regardless of owner.
    #[test]
    fn world_anchor_hitbox_ignores_owner_position() {
        let hitbox = Hitbox {
            owner: dummy_entity(),
            source: ActorFaction::Boss,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::new(500.0, 600.0),
            },
            half_extent: ae::Vec2::new(40.0, 40.0),
            damage: 1,
            knockback_strength: 0.0,
        };
        let aabb_a = hitbox.world_aabb(ae::Vec2::new(0.0, 0.0));
        let aabb_b = hitbox.world_aabb(ae::Vec2::new(9999.0, 9999.0));
        assert_eq!(aabb_a.center(), ae::Vec2::new(500.0, 600.0));
        assert_eq!(aabb_b.center(), ae::Vec2::new(500.0, 600.0));
    }

    /// `tick_and_despawn_hitboxes` advances `remaining_s` by
    /// `world_time.sim_dt()` and despawns when it hits zero. A
    /// short-lifetime hitbox should not survive a single tick at
    /// the default 1/60s sim dt.
    fn make_app_with_sim_dt(sim_dt: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<WorldTime>();
        // WorldTime::default() leaves scaled_dt = 0, which would
        // freeze every gameplay timer; bump it so the despawn
        // assertions actually advance the lifetime.
        let mut world_time = app.world_mut().resource_mut::<WorldTime>();
        world_time.scaled_dt = sim_dt;
        world_time.raw_dt = sim_dt;
        app
    }

    #[test]
    fn tick_and_despawn_drops_expired_hitboxes() {
        let mut app = make_app_with_sim_dt(0.05);
        app.add_systems(Update, tick_and_despawn_hitboxes);
        let hitbox = app
            .world_mut()
            .spawn((
                Hitbox {
                    owner: dummy_entity(),
                    source: ActorFaction::Enemy,
                    anchor: HitboxAnchor::FollowOwner {
                        local_offset: ae::Vec2::ZERO,
                    },
                    half_extent: ae::Vec2::new(10.0, 10.0),
                    damage: 1,
                    knockback_strength: 0.0,
                },
                HitboxLifetime { remaining_s: 0.01 },
                HitboxHits::default(),
            ))
            .id();
        // 50ms sim_dt burns through the 10ms lifetime in one tick.
        app.update();
        assert!(
            app.world().get_entity(hitbox).is_err(),
            "hitbox entity should be despawned after lifetime expired",
        );
    }

    /// A hitbox with `remaining_s` larger than one tick should
    /// stay alive after a single update.
    #[test]
    fn tick_and_despawn_keeps_live_hitboxes() {
        let mut app = make_app_with_sim_dt(0.05);
        app.add_systems(Update, tick_and_despawn_hitboxes);
        let hitbox = app
            .world_mut()
            .spawn((
                Hitbox {
                    owner: dummy_entity(),
                    source: ActorFaction::Enemy,
                    anchor: HitboxAnchor::FollowOwner {
                        local_offset: ae::Vec2::ZERO,
                    },
                    half_extent: ae::Vec2::new(10.0, 10.0),
                    damage: 1,
                    knockback_strength: 0.0,
                },
                HitboxLifetime { remaining_s: 5.0 },
                HitboxHits::default(),
            ))
            .id();
        app.update();
        assert!(
            app.world().get_entity(hitbox).is_ok(),
            "hitbox with multi-second lifetime should survive a single tick",
        );
    }

    /// `spawn_melee_hitbox` populates a freshly-spawned entity with
    /// the three expected components — pinned so a future
    /// `Bundle`-ification of the spawn doesn't drop the `HitboxHits`
    /// hit-once tracker by accident.
    ///
    /// The helper takes `&mut Commands`; drive it through a one-off
    /// system so Bevy provides a real Commands handle (and flushes
    /// the queue automatically when the system finishes).
    #[test]
    fn spawn_melee_hitbox_attaches_full_component_set() {
        #[derive(Resource, Default)]
        struct SpawnedHitbox(Option<Entity>);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<SpawnedHitbox>();
        let owner = dummy_entity();
        app.add_systems(
            Update,
            move |mut commands: Commands, mut store: ResMut<SpawnedHitbox>| {
                if store.0.is_some() {
                    return;
                }
                let entity = spawn_melee_hitbox(
                    &mut commands,
                    owner,
                    ActorFaction::Enemy,
                    ae::Vec2::new(-20.0, 0.0),
                    ae::Vec2::new(20.0, 14.0),
                    3,
                    1.5,
                    0.42,
                );
                store.0 = Some(entity);
            },
        );
        app.update();
        let spawned = app
            .world()
            .resource::<SpawnedHitbox>()
            .0
            .expect("spawn_melee_hitbox should return an Entity");
        let entity = app.world().entity(spawned);
        let hitbox = entity.get::<Hitbox>().expect("Hitbox missing");
        assert_eq!(hitbox.damage, 3);
        assert!((hitbox.knockback_strength - 1.5).abs() < f32::EPSILON);
        match hitbox.anchor {
            HitboxAnchor::FollowOwner { local_offset } => {
                assert_eq!(local_offset, ae::Vec2::new(-20.0, 0.0));
            }
            _ => panic!("expected FollowOwner anchor"),
        }
        let lifetime = entity.get::<HitboxLifetime>().expect("Lifetime missing");
        assert!((lifetime.remaining_s - 0.42).abs() < f32::EPSILON);
        assert!(
            entity.get::<HitboxHits>().is_some(),
            "HitboxHits hit-once tracker should be attached by default",
        );
    }
}
