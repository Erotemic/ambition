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

use crate::audio::SfxMessage;
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
    mut effects: MessageWriter<crate::effects::EffectRequest>,
    mut sfx: MessageWriter<SfxMessage>,
    mut actors: Query<(
        &super::super::components::ActorDisposition,
        Option<super::actor_clusters::ActorClusterQueryData>,
    )>,
    held_items: Query<&super::HeldItem>,
    // A possessed actor fires player-faction shots (the faction-aware pool then
    // routes them at the enemies, not the player) — `crate::abilities::traversal::possession`.
    possessed: Query<(), bevy::prelude::With<crate::abilities::traversal::possession::Possessed>>,
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
        let Ok((disposition, clusters)) = actors.get_mut(msg.actor) else {
            // Message references an actor that no longer exists
            // (despawned this frame). Skip silently.
            continue;
        };
        if disposition.is_peaceful() {
            // Peaceful actor emitting a Ranged action — would happen
            // only via test fixtures or a future "possessed-NPC"
            // path. Not in scope for this consumer.
            continue;
        }
        let Some(mut cq) = clusters else {
            continue;
        };
        let enemy = cq.as_actor_mut();
        if !enemy.status.alive {
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
        let kick = world_dir * -recoil_strength;
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
/// `features/ecs/hitbox.rs`): `update_ecs_actors` spawns
/// the strike's hitbox on the windup → active edge, and
/// `apply_hitbox_damage` resolves the overlap once per strike.
pub fn start_enemy_melee_from_brain_actions(
    mut messages: MessageReader<ActorActionMessage>,
    mut actors: Query<(
        &super::super::components::ActorDisposition,
        Option<super::actor_clusters::ActorClusterQueryData>,
    )>,
) {
    for msg in messages.read() {
        let ActionRequest::Melee { attack_axis, .. } = msg.request else {
            continue;
        };
        let Ok((disposition, clusters)) = actors.get_mut(msg.actor) else {
            continue;
        };
        if disposition.is_peaceful() {
            // Peaceful actors never produce Melee messages today
            // (their ActionSet is empty); skip defensively.
            continue;
        }
        let Some(mut cq) = clusters else {
            continue;
        };
        let mut enemy = cq.as_actor_mut();
        // The ActionSet → ActorActionMessage seam is the attack-policy gate:
        // if a hostile actor produced a Melee message, it owns a melee verb
        // for this state even when its authored archetype is normally peaceful
        // (e.g. a PirateHeavy after her shark mount dies). Keep only the
        // runtime cooldown/alive gate inside begin_melee_attack.
        // Thread the brain's attack axis through to the runtime so
        // the windup → active edge spawns the hitbox in the same
        // direction the brain committed to (forward / up / down / back).
        enemy.begin_melee_attack(attack_axis);
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
            ambition_characters::actor::EnemyBrain::Custom("pirate_raider".into()),
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
        let enemy = ActorClusterSeed::new(
            "skitter_a",
            "Skitter",
            aabb,
            ambition_characters::actor::EnemyBrain::Custom("small_skitter".into()),
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
            ambition_characters::actor::EnemyBrain::Custom("cellular_automaton_fighter".into()),
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
            ambition_characters::actor::EnemyBrain::Custom("small_skitter".into()),
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
        // .1 = cluster bundle, .1.1 = ActorStatus.
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

    /// `start_enemy_melee_from_brain_actions` pin: the consumer
    /// starts the enemy's melee windup + cooldown when a
    /// `ActorActionMessage::Melee` arrives without changing the
    /// windup/cooldown timings.
    #[test]
    fn melee_message_starts_enemy_windup_and_cooldown() {
        use ambition_characters::brain::{MeleeActionSpec, SwipeSpec};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<SandboxFeelTuning>();
        app.add_systems(Update, start_enemy_melee_from_brain_actions);

        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(20.0, 24.0));
        let mut enemy = ActorClusterSeed::new(
            "striker_a",
            "Striker",
            aabb,
            ambition_characters::actor::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        enemy.attack.cooldown = 0.0;
        assert!(!enemy.attack.is_winding_up());
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
        let attack = app
            .world()
            .get::<crate::features::BodyMelee>(actor)
            .unwrap();
        let status = *app
            .world()
            .get::<crate::features::ActorStatus>(actor)
            .unwrap();
        assert!(
            attack.is_winding_up(),
            "a swing should be winding up after the message: swing armed = {}",
            attack.swing.is_some(),
        );
        assert!(
            attack.cooldown > 0.0,
            "cooldown should be primed after the message: got {}",
            attack.cooldown,
        );
        assert!(
            matches!(
                status.ai_mode,
                ambition_characters::actor::ai::CharacterAiMode::Telegraph
            ),
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
        use ambition_characters::brain::{LungeSpec, MeleeActionSpec};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<SandboxFeelTuning>();
        app.add_systems(Update, start_enemy_melee_from_brain_actions);

        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(36.0, 55.0));
        let mut enemy = ActorClusterSeed::new(
            "iron_mary_dismounted",
            "Iron Mary",
            aabb,
            ambition_characters::actor::EnemyBrain::Custom("pirate_heavy".into()),
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

        let attack = app
            .world()
            .get::<crate::features::BodyMelee>(actor)
            .unwrap();
        let status = *app
            .world()
            .get::<crate::features::ActorStatus>(actor)
            .unwrap();
        assert!(
            attack.is_winding_up(),
            "explicit melee message should start dismounted PirateHeavy windup"
        );
        assert!(
            matches!(
                status.ai_mode,
                ambition_characters::actor::ai::CharacterAiMode::Telegraph
            ),
            "dismounted PirateHeavy should telegraph her melee attack"
        );
    }

    /// Cooldown still gates the consumer — a Melee message arriving
    /// while the enemy is mid-cooldown is a no-op. Mirrors the
    /// pre-migration legacy gate.
    #[test]
    fn melee_message_during_cooldown_is_dropped() {
        use ambition_characters::brain::{MeleeActionSpec, SwipeSpec};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<SandboxFeelTuning>();
        app.add_systems(Update, start_enemy_melee_from_brain_actions);

        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(20.0, 24.0));
        let mut enemy = ActorClusterSeed::new(
            "striker_a",
            "Striker",
            aabb,
            ambition_characters::actor::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        // Pre-set cooldown so begin_melee_attack refuses.
        enemy.attack.cooldown = 0.5;
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
        let attack = app
            .world()
            .get::<crate::features::BodyMelee>(actor)
            .unwrap();
        assert!(
            !attack.is_winding_up() && attack.swing.is_none(),
            "cooldown should prevent the swing from starting",
        );
    }

    /// Silence the test-only helper.
    #[test]
    fn default_combat_tuning_helper_exists() {
        let _ = default_combat_tuning();
    }
}
