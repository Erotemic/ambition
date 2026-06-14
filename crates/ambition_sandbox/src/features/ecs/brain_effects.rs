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

use crate::engine_core as ae;
use bevy::prelude::*;

use crate::audio::SfxMessage;
use crate::brain::{action_set::ActionRequest, ActorActionMessage};
use crate::enemy_projectile::EnemyProjectileSpawn;
use crate::features::ecs::actors::ActorRuntime;
use crate::time::feel::SandboxFeelTuning;

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

/// Read every `ActorActionMessage::Ranged` and spawn the matching
/// enemy projectile. Applies recoil to the firing actor's velocity.
///
/// Only handles **hostile** actors today — player projectiles still
/// flow through the legacy `update_player` path. Player migration is
/// the next slice in the mandate.
pub fn spawn_enemy_projectiles_from_brain_actions(
    mut messages: MessageReader<ActorActionMessage>,
    mut effects: MessageWriter<crate::effects::EffectRequest>,
    mut sfx: MessageWriter<SfxMessage>,
    mut actors: Query<(
        &ActorRuntime,
        Option<super::enemy_clusters::EnemyClusterQueryData>,
    )>,
    held_items: Query<&super::HeldItem>,
    // A possessed actor fires player-faction shots (the faction-aware pool then
    // routes them at the enemies, not the player) — `crate::abilities::traversal::possession`.
    possessed: Query<(), bevy::prelude::With<crate::abilities::traversal::possession::Possessed>>,
) {
    for msg in messages.read() {
        let ActionRequest::Ranged {
            spec,
            origin: _,
            dir,
        } = msg.request
        else {
            continue;
        };
        let Ok((actor, clusters)) = actors.get_mut(msg.actor) else {
            // Message references an actor that no longer exists
            // (despawned this frame). Skip silently.
            continue;
        };
        if !matches!(actor, ActorRuntime::Enemy) {
            // Peaceful actor emitting a Ranged action — would happen
            // only via test fixtures or a future "possessed-NPC"
            // path. Not in scope for this consumer.
            continue;
        }
        let Some(mut cq) = clusters else {
            continue;
        };
        let enemy = cq.as_enemy_mut();
        if !enemy.status.alive {
            continue;
        }
        // Held-item muzzle: a gun-sword shot should originate at the actor's
        // hand whether the pirate is still mounted or has fallen off the shark.
        // Future items can extend this routing by id without changing the brain.
        let held_item_id = held_items.get(msg.actor).ok().map(|item| item.id());
        let uses_gun_sword = held_item_id == Some("gun_sword");
        let (spawn_origin, owner_id) = if uses_gun_sword {
            let hand = crate::presentation::rendering::rider_hand_world_pos(
                enemy.kin.pos,
                enemy.kin.facing,
                enemy.kin.size.y,
            );
            let muzzle = hand + dir.normalize_or_zero() * 18.0;
            (muzzle, format!("lasersword:{}", enemy.config.id))
        } else {
            (
                enemy.kin.pos + ae::Vec2::new(0.0, -8.0),
                enemy.config.id.clone(),
            )
        };
        let spawn = EnemyProjectileSpawn {
            origin: spawn_origin,
            dir,
            speed: spec.speed(),
            damage: spec.damage(),
            max_lifetime: PROJECTILE_MAX_LIFETIME,
            half_extent: PROJECTILE_HALF_EXTENT,
            owner_id: owner_id.clone(),
            gravity: 0.0,
        };
        if owner_id.starts_with("lasersword:") {
            sfx.write(SfxMessage::Play {
                id: ambition_sfx::SfxId::from_static("weapon.lasersword.fire"),
                pos: spawn.origin,
            });
        }
        let shot_faction = if possessed.get(msg.actor).is_ok() {
            crate::projectile::ProjectileFaction::Player
        } else {
            crate::projectile::ProjectileFaction::Enemy
        };
        effects.write(crate::effects::EffectRequest {
            owner: msg.actor,
            effect: crate::effects::Effect::Projectiles {
                faction: shot_faction,
                shots: vec![spawn],
            },
        });
        // Recoil: push the firing actor backward along the negative
        // fire direction.
        let recoil_strength = if uses_gun_sword {
            RANGED_RECOIL_PIRATE
        } else {
            RANGED_RECOIL_DEFAULT
        };
        let kick = dir.normalize_or_zero() * -recoil_strength;
        enemy.kin.vel += kick;
    }
}

/// Read every `ActorActionMessage::Melee` addressed to a hostile
/// actor and start that enemy's melee windup/cooldown. Only the
/// START of the attack — the policy decision — moves through the
/// message stream; timers remain integration-side state.
///
/// Damage application during the active window flows through the
/// `Hitbox` entity lifecycle (see
/// `content/features/ecs/hitbox.rs`): `update_ecs_actors` spawns
/// the strike's hitbox on the windup → active edge, and
/// `apply_hitbox_damage` resolves the overlap once per strike.
pub fn start_enemy_melee_from_brain_actions(
    mut messages: MessageReader<ActorActionMessage>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut actors: Query<(
        &ActorRuntime,
        Option<super::enemy_clusters::EnemyClusterQueryData>,
    )>,
) {
    let combat_tuning = feel_tuning.feature_combat_tuning();
    for msg in messages.read() {
        let ActionRequest::Melee { attack_axis, .. } = msg.request else {
            continue;
        };
        let Ok((actor, clusters)) = actors.get_mut(msg.actor) else {
            continue;
        };
        if !matches!(actor, ActorRuntime::Enemy) {
            // Peaceful actors never produce Melee messages today
            // (their ActionSet is empty); skip defensively.
            continue;
        }
        let Some(mut cq) = clusters else {
            continue;
        };
        let mut enemy = cq.as_enemy_mut();
        // The ActionSet → ActorActionMessage seam is the attack-policy gate:
        // if a hostile actor produced a Melee message, it owns a melee verb
        // for this state even when its authored archetype is normally peaceful
        // (e.g. a PirateHeavy after her shark mount dies). Keep only the
        // runtime cooldown/alive gate inside begin_melee_attack.
        // Thread the brain's attack axis through to the runtime so
        // the windup → active edge spawns the hitbox in the same
        // direction the brain committed to (forward / up / down / back).
        enemy.begin_melee_attack(combat_tuning, attack_axis);
    }
}

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
    use crate::features::ecs::hitbox::{Hitbox, HitboxAnchor};
    use crate::brain::{ActionSet, RangedActionSpec};
    use crate::enemy_projectile::test_support::enemy_projectile_bodies;
    use crate::enemy_projectile::EnemyProjectileState;
    use crate::features::ecs::enemy_clusters::EnemyClusterSeed;
    use crate::projectile::ProjectileSeqCounter;

    /// Build a rider-shaped hostile actor: standalone PirateRaider
    /// archetype on the runtime side, but the caller is expected to
    /// attach a [`crate::features::RidingOn`] component to the
    /// spawned entity so the ranged-projectile handler routes the
    /// fire through the lasersword path.
    type EnemyClusterBundle = (
        super::super::enemy_clusters::BodyKinematics,
        super::super::enemy_clusters::EnemyStatus,
        super::super::enemy_clusters::EnemyConfig,
        super::super::enemy_clusters::ActorMotionPath,
        crate::features::ActorSurfaceState,
        crate::features::ActorAttackState,
        crate::mechanics::combat::CombatCapabilities,
    );

    /// Spawnable (marker + clusters) bundle for an enemy test fixture.
    fn enemy_actor(enemy: EnemyClusterSeed) -> (ActorRuntime, EnemyClusterBundle) {
        (ActorRuntime::Enemy, enemy.into_components())
    }

    fn pirate_rider_actor(pos: ae::Vec2) -> (ActorRuntime, EnemyClusterBundle) {
        let aabb = ae::Aabb::new(pos, ae::Vec2::new(14.0, 23.0));
        let enemy = EnemyClusterSeed::new(
            "rider_a",
            "Pirate Raider",
            aabb,
            crate::actor::EnemyBrain::Custom("pirate_raider".into()),
            &[],
        );
        enemy_actor(enemy)
    }

    fn build_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.add_message::<SfxMessage>();
        app.add_message::<crate::effects::EffectRequest>();
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
        let enemy = EnemyClusterSeed::new(
            "skitter_a",
            "Skitter",
            aabb,
            crate::actor::EnemyBrain::Custom("small_skitter".into()),
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

    #[test]
    fn ranged_message_for_dead_actor_is_dropped() {
        let mut app = build_app();
        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let mut actor_runtime = pirate_rider_actor(actor_pos);
        // .1 = cluster bundle, .1.1 = EnemyStatus.
        actor_runtime.1 .1.alive = false;
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

    /// `start_enemy_melee_from_brain_actions` pin: the consumer
    /// starts the enemy's melee windup + cooldown when a
    /// `ActorActionMessage::Melee` arrives without changing the
    /// windup/cooldown timings.
    #[test]
    fn melee_message_starts_enemy_windup_and_cooldown() {
        use crate::brain::{MeleeActionSpec, SwipeSpec};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<SandboxFeelTuning>();
        app.add_systems(Update, start_enemy_melee_from_brain_actions);

        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(20.0, 24.0));
        let mut enemy = EnemyClusterSeed::new(
            "striker_a",
            "Striker",
            aabb,
            crate::actor::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        enemy.attack.cooldown = 0.0;
        let pre_windup = enemy.attack.windup_timer;
        let actor = app.world_mut().spawn(enemy_actor(enemy)).id();
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Melee {
                    spec: MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
                    origin: actor_pos,
                    facing: 1.0,
                    attack_axis: ae::Vec2::new(1.0, 0.0),
                },
            });
        app.update();
        let attack = *app
            .world()
            .get::<crate::features::ActorAttackState>(actor)
            .unwrap();
        let status = *app
            .world()
            .get::<crate::features::EnemyStatus>(actor)
            .unwrap();
        assert!(
            attack.windup_timer > pre_windup,
            "windup timer should start after the message: was {pre_windup}, now {}",
            attack.windup_timer,
        );
        assert!(
            attack.cooldown > 0.0,
            "cooldown should be primed after the message: got {}",
            attack.cooldown,
        );
        assert!(
            matches!(status.ai_mode, crate::actor::ai::CharacterAiMode::Telegraph),
            "ai_mode should flip to Telegraph; got {:?}",
            status.ai_mode,
        );
    }

    /// A mounted PirateHeavy becomes explicitly hostile after her shark dies even
    /// though the standalone PirateHeavy archetype is authored peaceful. The
    /// mount-dissolve path installs a melee ActionSet; once that ActionSet emits a
    /// Melee message, the effects consumer must honor it instead of re-checking
    /// `EnemyArchetype::attacks_player()`.
    #[test]
    fn melee_message_can_start_windup_for_dismounted_pirate_heavy() {
        use crate::brain::{LungeSpec, MeleeActionSpec};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<SandboxFeelTuning>();
        app.add_systems(Update, start_enemy_melee_from_brain_actions);

        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(36.0, 55.0));
        let mut enemy = EnemyClusterSeed::new(
            "iron_mary_dismounted",
            "Iron Mary",
            aabb,
            crate::actor::EnemyBrain::Custom("pirate_heavy".into()),
            &[],
        );
        // The "pirate_heavy" brain resolved to the PirateHeavy spec: peaceful
        // by default, with the cove-crew provoke override that forces an
        // aggressive MeleeBrute when struck.
        assert!(
            !enemy.config.tuning.attacks_player,
            "standalone PirateHeavy is normally peaceful"
        );
        assert_eq!(
            enemy.spec.brain_spec().provoke_forced_brute_min_aggro,
            Some(500.0)
        );
        enemy.attack.cooldown = 0.0;
        let actor = app.world_mut().spawn(enemy_actor(enemy)).id();

        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Melee {
                    spec: MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT),
                    origin: actor_pos,
                    facing: -1.0,
                    attack_axis: ae::Vec2::new(-1.0, 0.0),
                },
            });

        app.update();

        let attack = *app
            .world()
            .get::<crate::features::ActorAttackState>(actor)
            .unwrap();
        let status = *app
            .world()
            .get::<crate::features::EnemyStatus>(actor)
            .unwrap();
        assert!(
            attack.windup_timer > 0.0,
            "explicit melee message should start dismounted PirateHeavy windup"
        );
        assert!(
            matches!(status.ai_mode, crate::actor::ai::CharacterAiMode::Telegraph),
            "dismounted PirateHeavy should telegraph her melee attack"
        );
    }

    /// Cooldown still gates the consumer — a Melee message arriving
    /// while the enemy is mid-cooldown is a no-op. Mirrors the
    /// pre-migration legacy gate.
    #[test]
    fn melee_message_during_cooldown_is_dropped() {
        use crate::brain::{MeleeActionSpec, SwipeSpec};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<SandboxFeelTuning>();
        app.add_systems(Update, start_enemy_melee_from_brain_actions);

        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(20.0, 24.0));
        let mut enemy = EnemyClusterSeed::new(
            "striker_a",
            "Striker",
            aabb,
            crate::actor::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        // Pre-set cooldown so begin_melee_attack refuses.
        enemy.attack.cooldown = 0.5;
        let pre_windup = enemy.attack.windup_timer;
        let actor = app.world_mut().spawn(enemy_actor(enemy)).id();
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Melee {
                    spec: MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
                    origin: actor_pos,
                    facing: 1.0,
                    attack_axis: ae::Vec2::new(1.0, 0.0),
                },
            });
        app.update();
        let attack = *app
            .world()
            .get::<crate::features::ActorAttackState>(actor)
            .unwrap();
        assert_eq!(
            attack.windup_timer, pre_windup,
            "cooldown should prevent the windup from starting",
        );
    }

    /// Silence the test-only helper.
    #[test]
    fn default_combat_tuning_helper_exists() {
        let _ = default_combat_tuning();
    }

}
