//! EFFECTS-stage consumers for `ActorActionMessage`.
//!
//! Per the actor/brain migration mandate (see
//! `dev/journals/brain-pipeline-bypass-audit-2026-05-24.md`):
//! hitboxes, projectiles, SFX, VFX, and recoil should be driven from
//! resolved action messages, not from legacy runtime spawn loops.
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
//!
//! The legacy paths (`EnemyRuntime::update`'s `outputs.projectile_spawns`,
//! `update_player`'s direct hitbox spawn, etc.) are deleted as each
//! consumer here takes over.

use ambition_engine as ae;
use bevy::prelude::*;

use crate::audio::SfxMessage;
use crate::brain::{ActorActionMessage, action_set::ActionRequest};
use crate::content::features::ecs::actors::ActorRuntime;
use crate::content::features::enemies::EnemyArchetype;
use crate::content::features::events::FeatureCombatTuning;
use crate::enemy_projectile::{EnemyProjectileSpawn, EnemyProjectileState};
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
    mut enemy_projectiles: ResMut<EnemyProjectileState>,
    mut sfx: MessageWriter<SfxMessage>,
    mut actors: Query<&mut ActorRuntime>,
) {
    for msg in messages.read() {
        let ActionRequest::Ranged { spec, origin: _, dir } = msg.request else {
            continue;
        };
        let Ok(mut actor) = actors.get_mut(msg.actor) else {
            // Message references an actor that no longer exists
            // (despawned this frame). Skip silently.
            continue;
        };
        let ActorRuntime::Hostile(enemy) = &mut *actor else {
            // Peaceful actor emitting a Ranged action — would happen
            // only via test fixtures or a future "possessed-NPC"
            // path. Not in scope for this consumer.
            continue;
        };
        if !enemy.alive {
            continue;
        }
        let is_pirate_shark = matches!(
            enemy.archetype,
            EnemyArchetype::PirateOnShark | EnemyArchetype::PirateHeavyOnShark
        );
        let (spawn_origin, owner_id) = if is_pirate_shark {
            // PirateOnShark fires from the rider's hand so the
            // projectile looks like it's leaving the gun-sword muzzle.
            // The `lasersword:` prefix on `owner_id` routes the
            // projectile to the lasersword visual in
            // `enemy_projectile/visuals.rs`.
            let hand =
                crate::presentation::rendering::rider_hand_world_pos(enemy.pos, enemy.facing);
            let muzzle = hand + dir.normalize_or_zero() * 18.0;
            (muzzle, format!("lasersword:{}", enemy.id))
        } else {
            (enemy.pos + ae::Vec2::new(0.0, -8.0), enemy.id.clone())
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
        enemy_projectiles.spawn(spawn);
        // Recoil: push the firing actor backward along the negative
        // fire direction.
        let recoil_strength = if is_pirate_shark {
            RANGED_RECOIL_PIRATE
        } else {
            RANGED_RECOIL_DEFAULT
        };
        let kick = dir.normalize_or_zero() * -recoil_strength;
        enemy.vel += kick;
    }
}

/// Read every `ActorActionMessage::Melee` addressed to a hostile
/// actor and start that enemy's melee windup/cooldown via
/// `EnemyRuntime::begin_melee_attack`. The active hitbox lifecycle
/// (windup → active → cooldown) stays on `EnemyRuntime` because the
/// timers are integration-side state per the actor/brain mandate
/// ("Runtimes own state, not policy"). Only the START of the
/// attack — the policy decision — moves through the message stream.
///
/// Damage application during the active window stays in
/// `update_ecs_actors` (which polls `enemy.player_damage(player_body)`
/// each tick); that's a per-tick overlap check, not a discrete
/// spawn, and the runtime's body/attack-timer pair is the right
/// integration surface for it.
pub fn start_enemy_melee_from_brain_actions(
    mut messages: MessageReader<ActorActionMessage>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut actors: Query<&mut ActorRuntime>,
) {
    let combat_tuning = feel_tuning.feature_combat_tuning();
    for msg in messages.read() {
        let ActionRequest::Melee { .. } = msg.request else {
            continue;
        };
        let Ok(mut actor) = actors.get_mut(msg.actor) else {
            continue;
        };
        let ActorRuntime::Hostile(enemy) = &mut *actor else {
            // Peaceful actors never produce Melee messages today
            // (their ActionSet is empty); skip defensively.
            continue;
        };
        // `attacks_player()` and the cooldown gate live inside
        // `begin_melee_attack` so a future "every actor that can
        // attack does so via this consumer" world doesn't have to
        // re-check policy here. The default ActionSet wiring already
        // refuses to emit Melee for peaceful archetypes (their
        // ActionSet.melee is None), so this gate is the safety net,
        // not the primary filter.
        if !enemy.archetype.attacks_player() {
            continue;
        }
        enemy.begin_melee_attack(combat_tuning);
    }
}

/// Helper: combat-tuning lookup. Lives on the test side to make
/// the helper available to the unit tests below without leaking
/// `SandboxFeelTuning` through the public API.
#[cfg(test)]
fn default_combat_tuning() -> FeatureCombatTuning {
    SandboxFeelTuning::default().feature_combat_tuning()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::{ActionSet, RangedActionSpec};
    use crate::content::features::enemies::EnemyRuntime;
    use bevy::prelude::*;

    fn pirate_on_shark_actor(pos: ae::Vec2) -> ActorRuntime {
        let aabb = ae::Aabb::new(pos, ae::Vec2::new(14.0, 23.0));
        let mut enemy = EnemyRuntime::new(
            "shark_a",
            "Burning Flying Shark",
            aabb,
            ae::EnemyBrain::Custom("pirate_on_shark".into()),
            &[],
        );
        enemy.archetype = EnemyArchetype::PirateOnShark;
        ActorRuntime::Hostile(enemy)
    }

    fn build_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.add_message::<SfxMessage>();
        app.init_resource::<EnemyProjectileState>();
        app.add_systems(Update, spawn_enemy_projectiles_from_brain_actions);
        app
    }

    #[test]
    fn ranged_message_spawns_projectile_and_applies_recoil() {
        let mut app = build_app();
        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let actor = app
            .world_mut()
            .spawn((pirate_on_shark_actor(actor_pos),))
            .id();
        let vel_before = match app.world().entity(actor).get::<ActorRuntime>().unwrap() {
            ActorRuntime::Hostile(e) => e.vel,
            _ => panic!("expected Hostile"),
        };
        // Player to the right → +x fire dir.
        let dir = ae::Vec2::new(1.0, 0.0);
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
            actor,
            request: ActionRequest::Ranged {
                spec: RangedActionSpec::Bolt { speed: 500.0, damage: 1 },
                origin: actor_pos,
                dir,
            },
        });
        app.update();
        let projectiles = app.world().resource::<EnemyProjectileState>();
        assert_eq!(
            projectiles.bodies.len(),
            1,
            "exactly one projectile should have spawned"
        );
        // Owner id must reflect lasersword routing for PirateOnShark.
        let owner = &projectiles.bodies[0].owner_id;
        assert!(
            owner.starts_with("lasersword:"),
            "PirateOnShark owner_id should carry the lasersword: prefix; got {owner:?}",
        );
        // Recoil: vel.x reduced by ~RANGED_RECOIL_PIRATE.
        let vel_after = match app.world().entity(actor).get::<ActorRuntime>().unwrap() {
            ActorRuntime::Hostile(e) => e.vel,
            _ => panic!("expected Hostile"),
        };
        let kick = vel_before.x - vel_after.x;
        assert!(
            kick > 300.0,
            "expected pirate recoil > 300 px/s; got {kick} (before={vel_before:?} after={vel_after:?})",
        );
    }

    #[test]
    fn ranged_message_for_non_pirate_uses_body_origin_not_hand() {
        let mut app = build_app();
        let actor_pos = ae::Vec2::new(300.0, 300.0);
        // Use Combatant (a melee archetype) — its spec is irrelevant
        // here; the consumer only branches on archetype for origin
        // and owner_id formatting.
        let aabb = ae::Aabb::new(actor_pos, ae::Vec2::new(14.0, 23.0));
        let mut enemy = EnemyRuntime::new(
            "skitter_a",
            "Skitter",
            aabb,
            ae::EnemyBrain::Custom("small_skitter".into()),
            &[],
        );
        enemy.archetype = EnemyArchetype::SmallSkitter;
        let actor = app
            .world_mut()
            .spawn((ActorRuntime::Hostile(enemy),))
            .id();
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
            actor,
            request: ActionRequest::Ranged {
                spec: RangedActionSpec::Rock { speed: 300.0, damage: 1 },
                origin: actor_pos,
                dir: ae::Vec2::new(1.0, 0.0),
            },
        });
        app.update();
        let projectiles = app.world().resource::<EnemyProjectileState>();
        assert_eq!(projectiles.bodies.len(), 1);
        let owner = &projectiles.bodies[0].owner_id;
        assert!(
            !owner.starts_with("lasersword:"),
            "non-pirate archetype must not get lasersword owner_id; got {owner:?}",
        );
    }

    #[test]
    fn ranged_message_for_dead_actor_is_dropped() {
        let mut app = build_app();
        let actor_pos = ae::Vec2::new(300.0, 300.0);
        let mut actor_runtime = pirate_on_shark_actor(actor_pos);
        if let ActorRuntime::Hostile(ref mut e) = actor_runtime {
            e.alive = false;
        }
        let actor = app.world_mut().spawn((actor_runtime,)).id();
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
            actor,
            request: ActionRequest::Ranged {
                spec: RangedActionSpec::Bolt { speed: 500.0, damage: 1 },
                origin: actor_pos,
                dir: ae::Vec2::new(1.0, 0.0),
            },
        });
        app.update();
        let projectiles = app.world().resource::<EnemyProjectileState>();
        assert!(
            projectiles.bodies.is_empty(),
            "dead actor must not spawn a projectile",
        );
    }

    /// Suppress unused-import noise from the test-only `ActionSet`
    /// reference — kept for callers that grow this module's tests.
    fn _silence_action_set_import(_: ActionSet) {}

    /// `start_enemy_melee_from_brain_actions` pin: the consumer
    /// starts the enemy's melee windup + cooldown when a
    /// `ActorActionMessage::Melee` arrives. Today the same trigger
    /// flowed through `EnemyRuntime::update` directly; the consumer
    /// replaces that gate without changing the windup/cooldown
    /// timings.
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
        let mut enemy = crate::content::features::enemies::EnemyRuntime::new(
            "striker_a",
            "Striker",
            aabb,
            ae::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        enemy.archetype = EnemyArchetype::MediumStriker;
        enemy.attack_cooldown = 0.0;
        let pre_windup = enemy.attack_windup_timer;
        let actor = app.world_mut().spawn((ActorRuntime::Hostile(enemy),)).id();
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
        let actor_runtime = app.world().entity(actor).get::<ActorRuntime>().unwrap();
        let ActorRuntime::Hostile(enemy_after) = actor_runtime else {
            panic!("expected Hostile");
        };
        assert!(
            enemy_after.attack_windup_timer > pre_windup,
            "windup timer should start after the message: was {pre_windup}, now {}",
            enemy_after.attack_windup_timer,
        );
        assert!(
            enemy_after.attack_cooldown > 0.0,
            "cooldown should be primed after the message: got {}",
            enemy_after.attack_cooldown,
        );
        assert!(
            matches!(enemy_after.ai_mode, ae::CharacterAiMode::Telegraph),
            "ai_mode should flip to Telegraph; got {:?}",
            enemy_after.ai_mode,
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
        let mut enemy = crate::content::features::enemies::EnemyRuntime::new(
            "striker_a",
            "Striker",
            aabb,
            ae::EnemyBrain::Custom("medium_striker".into()),
            &[],
        );
        enemy.archetype = EnemyArchetype::MediumStriker;
        // Pre-set cooldown so begin_melee_attack refuses.
        enemy.attack_cooldown = 0.5;
        let pre_windup = enemy.attack_windup_timer;
        let actor = app.world_mut().spawn((ActorRuntime::Hostile(enemy),)).id();
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
        let actor_runtime = app.world().entity(actor).get::<ActorRuntime>().unwrap();
        let ActorRuntime::Hostile(enemy_after) = actor_runtime else {
            panic!("expected Hostile");
        };
        assert_eq!(
            enemy_after.attack_windup_timer, pre_windup,
            "cooldown should prevent the windup from starting",
        );
    }

    /// Silence the test-only helper.
    #[test]
    fn default_combat_tuning_helper_exists() {
        let _ = default_combat_tuning();
    }
}
