//! EFFECTS-stage consumers for `ActorActionMessage`.
//!
//! Hitboxes, projectiles, SFX, VFX, and recoil are driven from resolved
//! action messages rather than from per-actor integration loops.
//!
//! This module owns the consumer Bevy systems that read
//! `MessageReader<ActorActionMessage>` and produce effects. Each
//! system is one variant of `ActionRequest`; the upstream
//! `emit_brain_action_messages` resolver translates the actor's
//! `ActorControl` frame + `ActionSet` into the per-request stream
//! these systems consume.
//!
//! Schedule:
//! - `emit_brain_action_messages` runs first
//! - these systems run after, reading the same message stream
//! - the `BrainActionCounter` observer is unaffected (it counts but
//!   doesn't consume)

use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_sfx::SfxMessage;
use crate::enemy_projectile::EnemyProjectileSpawn;
#[cfg(test)]
use crate::time::feel::SandboxFeelTuning;
use ambition_characters::brain::{action_set::ActionRequest, ActorActionMessage};

/// Recoil applied to the firing enemy along the negative fire
/// direction. Per-archetype because PirateOnShark visibly knocks
/// back the rider+shark combo.
const RANGED_RECOIL_PIRATE: f32 = 380.0;
const RANGED_RECOIL_DEFAULT: f32 = 60.0;

/// Projectile envelope shared by every ranged enemy. Future
/// per-archetype overrides (slower arrows, gravity-arc rocks)
/// will move this into an `ActionSet`-derived parameter.
const PROJECTILE_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(10.0, 8.0);
const PROJECTILE_MAX_LIFETIME: f32 = 2.4;

/// Body-side ranged refire interval (s) — the floor on every ranged-capable
/// body's fire rate (invariant I3). The controller (AI brain, possessing human,
/// or future RL policy) may attempt `fire` every tick; the body accepts a shot
/// at most once per this interval. This was previously a *brain*-side cadence
/// (`SmashState::ranged_cooldown_remaining`), which leaked the physical limit
/// into the controller — a human could spam past it. It now lives on the body.
/// Per-archetype tempos will move this onto an `ActionSet`-derived parameter,
/// like the projectile envelope above.
const RANGED_REFIRE_S: f32 = 1.1;

/// Read every `ActorActionMessage::Ranged` and spawn the matching
/// enemy projectile. Applies recoil to the firing actor's velocity.
///
/// Only handles **hostile** actors today — player projectiles still
/// flow through the legacy `update_player` path. Player migration is
/// the next slice in the mandate.
pub fn spawn_enemy_projectiles_from_brain_actions(
    mut messages: MessageReader<ActorActionMessage>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut sfx: MessageWriter<SfxMessage>,
    mut actors: Query<Option<super::actor_clusters::ActorClusterQueryData>>,
    held_items: Query<&super::HeldItem>,
) {
    for msg in messages.read() {
        let ActionRequest::Ranged {
            spec,
            origin,
            dir,
            dir_policy,
        } = msg.request
        else {
            continue;
        };
        let Ok(clusters) = actors.get_mut(msg.actor) else {
            // Message references an actor that no longer exists
            // (despawned this frame). Skip silently.
            continue;
        };
        // Capability, not AI policy: the actor fires because it OWNS a ranged
        // `ActionSet` slot (the upstream resolver only emits `Ranged` for a body
        // whose `ActionSet.ranged.is_some()`). A player possessing a peaceful NPC
        // fires its authored weapon; an autonomous peaceful NPC has no ranged
        // slot, so it emits nothing. Disposition (attack-or-not while autonomous)
        // is the BRAIN's business, not this effect consumer's.
        let Some(mut cq) = clusters else {
            continue;
        };
        let enemy = cq.as_actor_mut();
        if !enemy.health.alive() {
            continue;
        }
        // Body-side fire-rate enforcement (invariant I3): the controller attempts
        // a shot every time it emits `fire`; the body accepts it only when the
        // ranged weapon is off cooldown, re-arming on each accepted shot. A
        // blocked attempt simply spawns nothing this tick. This is the single
        // place the weapon rate is enforced, identical for an AI spam controller,
        // a tactical brain, and a possessing human.
        if !enemy.attack.try_fire_ranged(RANGED_REFIRE_S).accepted() {
            continue;
        }
        // Held-item muzzle: a gun-sword shot should originate at the actor's
        // hand whether the pirate is still mounted or has fallen off the shark.
        // Future items can extend this routing by id without changing the brain.
        let held_item_id = held_items.get(msg.actor).ok().map(|item| item.id());
        let uses_gun_sword = held_item_id == Some("gun_sword");
        // The projectile's APPEARANCE is chosen by KIND, set here at the fire
        // site: a gun-sword discharge is a spinning lasersword; otherwise the
        // archetype's authored ranged visual (e.g. the PCA's Conway glider),
        // defaulting to the generic hostile shot. The render layer reads this
        // kind — never the owner-id string.
        let visual_kind = if uses_gun_sword {
            crate::projectile::ProjectileVisualKind::Lasersword
        } else {
            enemy.config.tuning.ranged_visual
        };
        let gravity_dir = -enemy
            .surface
            .surface_normal
            .normalize_or(ae::Vec2::new(0.0, -1.0));
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let request = ambition_characters::actor::control::ActorFireRequest {
            dir,
            dir_policy,
            speed: spec.speed(),
        };
        let world_dir = request.dir_to_world(frame).normalize_or_zero();
        // owner_id is the firing actor's id ONLY — used for self / friendly-fire
        // filtering and traces. It no longer encodes the projectile's look
        // (that's `visual_kind`), so a gun-sword shot carries the plain actor id
        // while still originating at the hand muzzle.
        let owner_id = enemy.config.id.clone();
        let spawn_origin = if uses_gun_sword {
            let hand = crate::features::rider_hand_world_pos_in_frame(
                enemy.kin.pos,
                enemy.kin.facing,
                enemy.kin.size.y,
                gravity_dir,
            );
            hand + world_dir * 18.0
        } else {
            origin + frame.to_world(ae::Vec2::new(0.0, -8.0))
        };
        let spawn = EnemyProjectileSpawn {
            origin: spawn_origin,
            dir: world_dir,
            speed: spec.speed(),
            damage: spec.damage(),
            max_lifetime: PROJECTILE_MAX_LIFETIME,
            half_extent: PROJECTILE_HALF_EXTENT,
            owner_id: owner_id.clone(),
            gravity: 0.0,
            visual_tag: visual_kind.to_tag(),
        };
        if uses_gun_sword {
            sfx.write(SfxMessage::Play {
                id: ambition_sfx::SfxId::from_static("weapon.lasersword.fire"),
                pos: spawn.origin,
            });
        }
        effects.write(ambition_vfx::EffectRequest {
            owner: msg.actor,
            effect: ambition_vfx::Effect::Projectiles { shots: vec![spawn] },
        });
        // Recoil: push the firing actor backward along the negative
        // fire direction.
        let recoil_strength = if uses_gun_sword {
            RANGED_RECOIL_PIRATE
        } else {
            RANGED_RECOIL_DEFAULT
        };
        let kick = world_dir * -recoil_strength;
        enemy.kin.vel += kick;
    }
}

// Melee START is no longer an actor-specific consumer. `ActorActionMessage::Melee`
// is turned into a swing by the body-generic `combat::attack::start_body_melee`
// phase (which runs for EVERY body — player, possessed actor, autonomous hostile),
// and the active-edge strike is spawned by `combat::attack::advance_body_melee`.
// The old `start_enemy_melee_from_brain_actions` / `ActorMut::begin_melee_attack`
// actor-only pair is deleted — one melee lifecycle, not a player driver plus an
// actor driver.

/// Helper: combat-tuning lookup. Lives on the test side to make
/// the helper available to the unit tests below without leaking
/// `SandboxFeelTuning` through the public API.
#[cfg(test)]
fn default_combat_tuning() -> crate::features::events::FeatureCombatTuning {
    SandboxFeelTuning::default().feature_combat_tuning()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy_projectile::test_support::enemy_projectile_bodies;
    use crate::enemy_projectile::EnemyProjectileState;
    use crate::features::ecs::actor_clusters::ActorClusterSeed;
    use crate::projectile::ProjectileSeqCounter;
    use ambition_characters::brain::{ActionSet, RangedActionSpec};

    /// Build a rider-shaped hostile actor: standalone PirateRaider
    /// archetype on the runtime side, but the caller is expected to
    /// attach a [`crate::features::RidingOn`] component to the
    /// spawned entity so the ranged-projectile handler routes the
    /// fire through the lasersword path.
    type ActorClusterBundle = (
        super::super::actor_clusters::BodyKinematics,
        super::super::actor_clusters::ActorStatus,
        crate::actor::BodyHealth,
        super::super::actor_clusters::ActorConfig,
        super::super::actor_clusters::ActorMotionPath,
        crate::features::ActorSurfaceState,
        crate::features::BodyMelee,
        crate::actor::AncillaryMovementBundle,
        crate::combat::CombatCapabilities,
    );

    /// Spawnable (disposition + clusters) bundle for an enemy test fixture.
    fn enemy_actor(
        enemy: ActorClusterSeed,
    ) -> (crate::features::ActorDisposition, ActorClusterBundle) {
        (
            crate::features::ActorDisposition::Hostile,
            enemy.into_components(),
        )
    }

    fn pirate_rider_actor(
        pos: ae::Vec2,
    ) -> (crate::features::ActorDisposition, ActorClusterBundle) {
        let aabb = ae::Aabb::new(pos, ae::Vec2::new(14.0, 23.0));
        let enemy = ActorClusterSeed::new(
            "rider_a",
            "Pirate Raider",
            aabb,
            ambition_characters::actor::CharacterBrain::Custom("pirate_raider".into()),
            &[],
        );
        enemy_actor(enemy)
    }

    fn build_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.add_message::<SfxMessage>();
        app.add_message::<ambition_vfx::EffectRequest>();
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
        // Phase 3b: the consumer emits SpawnProjectile; chain the enemy-pool
        // applier so the projectile entity spawns within the update.
        app.add_systems(
            Update,
            (
                spawn_enemy_projectiles_from_brain_actions,
                crate::enemy_projectile::apply_projectile_effects,
            )
                .chain(),
        );
        app
    }

    #[test]
    fn ranged_message_for_non_pirate_uses_body_origin_not_hand() {
        let mut app = build_app();
        let actor_pos = ae::Vec2::new(300.0, 300.0);
        // Use Combatant (a melee archetype) — its spec is irrelevant
        // here; the consumer only branches on archetype for origin
        // and owner_id formatting.
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(14.0, 23.0));
        let enemy = ActorClusterSeed::new(
            "skitter_a",
            "Skitter",
            aabb,
            ambition_characters::actor::CharacterBrain::Custom("small_skitter".into()),
            &[],
        );
        let actor = app.world_mut().spawn(enemy_actor(enemy)).id();
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Ranged {
                    spec: RangedActionSpec::Rock {
                        speed: 300.0,
                        damage: 1,
                    },
                    origin: actor_pos,
                    dir: ae::Vec2::new(1.0, 0.0),
                    dir_policy: ae::GameplayFramePolicy::WorldSpace,
                },
            });
        app.update();
        let projectiles = enemy_projectile_bodies(&mut app);
        assert_eq!(projectiles.len(), 1);
        let owner = &projectiles[0].owner_id;
        assert!(
            !owner.starts_with("lasersword:"),
            "non-pirate archetype must not get lasersword owner_id; got {owner:?}",
        );
    }

    /// The ranged-fire consumer stamps the firing actor's authored ranged
    /// visual onto the spawned projectile (by KIND, not owner_id). A
    /// `cellular_automaton_fighter` authored `ranged_visual: Glider` fires a
    /// Glider-kind shot.
    #[test]
    fn ranged_shot_carries_archetype_authored_visual_kind() {
        let mut app = build_app();
        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(14.0, 23.0));
        let enemy = ActorClusterSeed::new(
            "pca_test",
            "Perfect Cell-ular Automaton",
            aabb,
            ambition_characters::actor::CharacterBrain::Custom("cellular_automaton_fighter".into()),
            &[],
        );
        let mut bundle = enemy_actor(enemy);
        // Author the ranged visual as the runtime archetype projection would.
        bundle.1 .3.tuning.ranged_visual = crate::projectile::ProjectileVisualKind::Glider;
        let actor = app.world_mut().spawn(bundle).id();
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Ranged {
                    spec: RangedActionSpec::Rock {
                        speed: 300.0,
                        damage: 1,
                    },
                    origin: actor_pos,
                    dir: ae::Vec2::new(1.0, 0.0),
                    dir_policy: ae::GameplayFramePolicy::WorldSpace,
                },
            });
        app.update();
        let mut q = app
            .world_mut()
            .query::<&crate::projectile::ProjectileVisualKind>();
        let kinds: Vec<_> = q.iter(app.world()).copied().collect();
        assert_eq!(
            kinds,
            vec![crate::projectile::ProjectileVisualKind::Glider],
            "the PCA's authored ranged_visual must ride onto the spawned shot"
        );
    }

    #[test]
    fn ranged_message_converts_local_direction_at_consumer_frame() {
        let mut app = build_app();
        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(14.0, 23.0));
        let enemy = ActorClusterSeed::new(
            "side_gravity_shooter",
            "Skitter",
            aabb,
            ambition_characters::actor::CharacterBrain::Custom("small_skitter".into()),
            &[],
        );
        let mut actor_bundle = enemy_actor(enemy);
        // surface_normal points away from the support; gravity_dir is its
        // negative. Here local down is world +X, so local side/right maps to
        // world -Y under the arbitrary AccelerationFrame transform.
        actor_bundle.1 .5.surface_normal = ae::Vec2::new(-1.0, 0.0);
        let actor = app.world_mut().spawn(actor_bundle).id();
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Ranged {
                    spec: RangedActionSpec::Rock {
                        speed: 300.0,
                        damage: 1,
                    },
                    origin: actor_pos,
                    dir: ae::Vec2::new(1.0, 0.0),
                    dir_policy: ae::GameplayFramePolicy::ControlledBodyLocal,
                },
            });
        app.update();
        let projectiles = enemy_projectile_bodies(&mut app);
        assert_eq!(projectiles.len(), 1);
        let dir = projectiles[0].body.kin.vel.normalize_or_zero();
        assert!(
            dir.y < -0.99 && dir.x.abs() < 0.01,
            "local side/right under +X down should fire world -Y, got {dir:?}"
        );
    }

    #[test]
    fn ranged_message_for_dead_actor_is_dropped() {
        let mut app = build_app();
        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let mut actor_runtime = pirate_rider_actor(actor_pos);
        // .1 = cluster bundle; BodyHealth (liveness authority) is at .1.2.
        actor_runtime.1 .2.health.current = 0;
        let actor = app.world_mut().spawn(actor_runtime).id();
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Ranged {
                    spec: RangedActionSpec::Bolt {
                        speed: 500.0,
                        damage: 1,
                    },
                    origin: actor_pos,
                    dir: ae::Vec2::new(1.0, 0.0),
                    dir_policy: ae::GameplayFramePolicy::WorldSpace,
                },
            });
        app.update();
        assert!(
            enemy_projectile_bodies(&mut app).is_empty(),
            "dead actor must not spawn a projectile",
        );
    }

    /// Suppress unused-import noise from the test-only `ActionSet`
    /// reference — kept for callers that grow this module's tests.
    fn _silence_action_set_import(_: ActionSet) {}

    // The melee-START unit pins that used to live here
    // (`melee_message_starts_enemy_windup_and_cooldown`,
    // `melee_message_can_start_windup_for_dismounted_pirate_heavy`,
    // `melee_message_during_cooldown_is_dropped`) exercised the deleted
    // actor-only `start_enemy_melee_from_brain_actions`. The unified
    // `combat::attack::start_body_melee` phase now owns melee-start for every body;
    // it is pinned through the REAL schedule by
    // `ambition_app/tests/enemy_attacks_player.rs` (actor melee lands on the player),
    // `possession_end_to_end.rs` (possessed actor melee), and the body-generic
    // `unified_melee.rs` tests (player + peaceful-NPC-with-kit + hostile actor all
    // enter the SAME lifecycle from `ActorActionMessage::Melee`).

    /// Silence the test-only helper.
    #[test]
    fn default_combat_tuning_helper_exists() {
        let _ = default_combat_tuning();
    }
}
