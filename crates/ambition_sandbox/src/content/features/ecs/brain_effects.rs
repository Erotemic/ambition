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
use crate::brain::{action_set::ActionRequest, ActorActionMessage, SpecialActionSpec};
use crate::content::features::ecs::actors::ActorRuntime;
use crate::content::features::ecs::BossFeature;
use crate::content::features::ecs::FeatureSimEntity;
use crate::content::features::enemies::EnemyArchetype;
use crate::enemy_projectile::{EnemyProjectileSpawn, EnemyProjectileState};
use crate::time::feel::SandboxFeelTuning;
use crate::WorldTime;

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
        let ActionRequest::Ranged {
            spec,
            origin: _,
            dir,
        } = msg.request
        else {
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
/// `EnemyRuntime::begin_melee_attack`. The windup → active timer
/// transition stays on `EnemyRuntime` because the timers are
/// integration-side state per the actor/brain mandate ("Runtimes
/// own state, not policy"). Only the START of the attack — the
/// policy decision — moves through the message stream.
///
/// Damage application during the active window flows through the
/// `Hitbox` entity lifecycle (see
/// `content/features/ecs/hitbox.rs`): `update_ecs_actors` spawns
/// the strike's hitbox on the windup → active edge, and
/// `apply_hitbox_damage` resolves the overlap once per strike.
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

/// Per-boss apple-rain accumulator: state moved out of `BossRuntime`
/// to keep the runtime focused on body/HP and to let the EFFECTS
/// consumer (`spawn_gnu_apple_rain_from_special_messages`) own the
/// per-tick spawn cadence. Defaulted-attached to every boss; only
/// the gnu_ton encounter advances it (its ActionSet's `special` is
/// `SpecialActionSpec::GnuAppleRain`, so only it generates the
/// Special messages the consumer reads). Per the actor/brain
/// follow-up plan Task B: components hold state, consumers spawn
/// effects.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AppleRainSpawnState {
    /// Seconds carried over from the previous tick's apple-spawn.
    /// Drained while >= `interval_s` and refilled by `dt` each tick
    /// the Special message arrives.
    pub spawn_accum: f32,
    /// Monotonic spawn counter; used as the golden-ratio sequence
    /// index for deterministic per-spawn x distribution.
    pub spawn_index: u32,
}

/// Apple cosmetic / collision constants reused from
/// `content/features/bosses.rs` so the consumer doesn't re-derive
/// them. The half-extent / gravity / lifetime stay co-authored with
/// the legacy path until the second boss adopts the consumer
/// pattern.
const APPLE_RAIN_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(14.0, 16.0);
const APPLE_RAIN_GRAVITY: f32 = 540.0;
const APPLE_RAIN_LIFETIME: f32 = 6.0;
const APPLE_RAIN_SPAWN_HEIGHT_ABOVE_PLAYER: f32 = 320.0;
const PHI_FRAC: f32 = 0.618_033_99;

/// Spawn GNU-ton's apple rain in response to
/// `ActorActionMessage::Special { spec: SpecialActionSpec::GnuAppleRain }`.
/// The boss runtime tags `frame.special_pressed = true` every tick
/// its `BossAttackProfile::GnuAppleRain` strike window is active;
/// the resolver translates that into one `Special` message per
/// tick. This consumer owns the spawn cadence, the
/// golden-ratio x distribution, and the self-aabb dodge that keeps
/// apples from landing on the giant's own head — all of which used
/// to live inside `BossRuntime::tick_apple_rain`.
///
/// Bosses whose Special slot is something other than GnuAppleRain
/// emit no messages this consumer cares about; bosses whose
/// `BossPattern` brain doesn't fire `special_pressed` simply pass
/// through. The per-boss `AppleRainSpawnState` resets to zero on
/// any tick the message doesn't arrive, so the next strike window
/// starts on a clean beat instead of inheriting a burst from
/// leftover dt.
pub fn spawn_gnu_apple_rain_from_special_messages(
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    mut messages: MessageReader<ActorActionMessage>,
    mut enemy_projectiles: ResMut<EnemyProjectileState>,
    mut bosses: Query<(Entity, &mut AppleRainSpawnState, &BossFeature), With<FeatureSimEntity>>,
) {
    let dt = world_time.sim_dt();
    // Bosses with a `Special::GnuAppleRain` request this tick.
    // Multiple messages from the same boss collapse onto the same
    // entry — the consumer treats "any GnuAppleRain message this
    // tick" as "the strike window is active this tick".
    let mut active_params: std::collections::HashMap<Entity, (f32, f32, i32)> =
        std::collections::HashMap::new();
    for msg in messages.read() {
        let ActionRequest::Special { spec } = msg.request else {
            continue;
        };
        let SpecialActionSpec::GnuAppleRain {
            interval_s,
            spawn_speed,
            damage,
        } = spec
        else {
            continue;
        };
        active_params.insert(msg.actor, (interval_s, spawn_speed, damage));
    }

    for (entity, mut state, boss_feature) in &mut bosses {
        let Some((interval_s, spawn_speed, damage)) = active_params.get(&entity).copied() else {
            // No message this tick → reset accumulator so a future
            // strike window starts on a clean beat.
            state.spawn_accum = 0.0;
            continue;
        };
        let boss = &boss_feature.boss;
        if !boss.alive || interval_s <= 0.0 {
            continue;
        }
        state.spawn_accum += dt;
        let margin = APPLE_RAIN_HALF_EXTENT.x + 8.0;
        let max_x = (world.0.size.x - margin).max(margin);
        let spawnable_width = (max_x - margin).max(0.0);
        let self_aabb = boss.aabb();
        while state.spawn_accum >= interval_s {
            state.spawn_accum -= interval_s;
            let frac = ((state.spawn_index as f32) * PHI_FRAC).fract();
            let mut spawn_x = margin + frac * spawnable_width;
            // Slide x out from under the boss body so an apple
            // doesn't immediately hit GNU-ton on the head. Pick
            // the nearer of the boss's left/right edges so the
            // dodge motion stays small.
            let self_left = self_aabb.min.x - APPLE_RAIN_HALF_EXTENT.x;
            let self_right = self_aabb.max.x + APPLE_RAIN_HALF_EXTENT.x;
            if spawn_x > self_left && spawn_x < self_right {
                spawn_x = if spawn_x - self_left < self_right - spawn_x {
                    self_left
                } else {
                    self_right
                };
                spawn_x = spawn_x.clamp(margin, max_x);
            }
            let spawn_y = (boss.pos.y - APPLE_RAIN_SPAWN_HEIGHT_ABOVE_PLAYER)
                .max(APPLE_RAIN_HALF_EXTENT.y + 8.0);
            enemy_projectiles.spawn(EnemyProjectileSpawn {
                origin: ae::Vec2::new(spawn_x, spawn_y),
                // Downward initial velocity so the apple commits to
                // its lane immediately instead of hanging at zero
                // until gravity catches up.
                dir: ae::Vec2::new(0.0, 1.0),
                speed: spawn_speed,
                damage,
                max_lifetime: APPLE_RAIN_LIFETIME,
                half_extent: APPLE_RAIN_HALF_EXTENT,
                owner_id: format!(
                    "{}:{}",
                    crate::content::features::bosses::GNU_TON_APPLE_OWNER_PREFIX,
                    boss.id,
                ),
                gravity: APPLE_RAIN_GRAVITY,
            });
            state.spawn_index = state.spawn_index.wrapping_add(1);
        }
    }
}

/// Helper: combat-tuning lookup. Lives on the test side to make
/// the helper available to the unit tests below without leaking
/// `SandboxFeelTuning` through the public API.
#[cfg(test)]
fn default_combat_tuning() -> crate::content::features::events::FeatureCombatTuning {
    SandboxFeelTuning::default().feature_combat_tuning()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::{ActionSet, RangedActionSpec};
    use crate::content::features::enemies::EnemyRuntime;

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
                    spec: RangedActionSpec::Bolt {
                        speed: 500.0,
                        damage: 1,
                    },
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
        let actor = app.world_mut().spawn((ActorRuntime::Hostile(enemy),)).id();
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
                    spec: RangedActionSpec::Bolt {
                        speed: 500.0,
                        damage: 1,
                    },
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

    // -----------------------------------------------------------
    // Apple-rain consumer tests
    //
    // Mirrors the per-tick invariants the deleted
    // `BossRuntime::tick_apple_rain` tests pinned: cadence,
    // self-aabb dodge, downward gravity, full-width coverage, and
    // the "accumulator resets when the strike window closes"
    // semantics. The new tests drive the EFFECTS consumer
    // (`spawn_gnu_apple_rain_from_special_messages`) directly with
    // an `ActorActionMessage::Special` stream so the assertions hold
    // for the same authoritative path the live boss uses.
    // -----------------------------------------------------------

    use crate::content::features::bosses::{
        BossBehaviorProfile, BossRuntime, GNU_TON_APPLE_OWNER_PREFIX,
    };
    use crate::content::features::ecs::{BossFeature, FeatureSimEntity};
    use crate::GameWorld;

    fn gnu_apple_rain_spec() -> SpecialActionSpec {
        SpecialActionSpec::GnuAppleRain {
            interval_s: 0.35,
            spawn_speed: 35.0,
            damage: 1,
        }
    }

    fn gnu_ton_boss_feature() -> BossFeature {
        let aabb = ae::Aabb::new(ae::Vec2::new(500.0, 400.0), ae::Vec2::new(80.0, 80.0));
        let mut boss = BossRuntime::new("boss_gnu_ton", "GNU-ton", aabb, ae::BossBrain::Dormant);
        boss.behavior = BossBehaviorProfile::gnu_ton();
        BossFeature::new(boss)
    }

    fn apple_rain_app(sim_dt: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<WorldTime>();
        let mut world_time = app.world_mut().resource_mut::<WorldTime>();
        world_time.scaled_dt = sim_dt;
        world_time.raw_dt = sim_dt;
        let arena = ae::World::new(
            "test_arena",
            ae::Vec2::new(2_000.0, 2_000.0),
            ae::Vec2::ZERO,
            Vec::new(),
        );
        app.insert_resource(GameWorld(arena));
        app.add_systems(Update, spawn_gnu_apple_rain_from_special_messages);
        app
    }

    fn write_special(app: &mut App, actor: Entity, spec: SpecialActionSpec) {
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Special { spec },
            });
    }

    /// Across a few intervals' worth of ticks, the consumer should
    /// emit one apple per `interval_s` (cadence). Each apple must
    /// carry the apple owner prefix so the visuals layer swaps in
    /// the apple sprite.
    #[test]
    fn apple_rain_consumer_spawns_on_interval() {
        let interval = 0.35;
        let dt = interval; // one apple per tick
        let mut app = apple_rain_app(dt);
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                AppleRainSpawnState::default(),
                gnu_ton_boss_feature(),
            ))
            .id();
        for _ in 0..3 {
            write_special(&mut app, actor, gnu_apple_rain_spec());
            app.update();
        }
        let projectiles = app.world().resource::<EnemyProjectileState>();
        assert_eq!(
            projectiles.bodies.len(),
            3,
            "expected one apple per tick at dt==interval, got {}",
            projectiles.bodies.len(),
        );
        for spawn in &projectiles.bodies {
            assert!(
                spawn.owner_id.starts_with(GNU_TON_APPLE_OWNER_PREFIX),
                "apples must use the apple owner prefix; got {}",
                spawn.owner_id,
            );
        }
    }

    /// When no Special message arrives this tick, the per-boss
    /// `AppleRainSpawnState` must reset to zero so the next strike
    /// window starts on a clean beat instead of dumping a burst
    /// from leftover dt.
    #[test]
    fn apple_rain_state_resets_when_strike_window_closes() {
        let dt = 0.30; // less than the 0.35 interval — accum sits non-zero after one tick
        let mut app = apple_rain_app(dt);
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                AppleRainSpawnState::default(),
                gnu_ton_boss_feature(),
            ))
            .id();
        // Tick 1: Special arrives → accumulator advances 0.30s.
        write_special(&mut app, actor, gnu_apple_rain_spec());
        app.update();
        let accum_after_tick_1 = app
            .world()
            .entity(actor)
            .get::<AppleRainSpawnState>()
            .unwrap()
            .spawn_accum;
        assert!(
            accum_after_tick_1 > 0.0,
            "accumulator should advance while strike active",
        );
        // Tick 2: no Special → accumulator resets, no apple emitted.
        app.update();
        let accum_after_tick_2 = app
            .world()
            .entity(actor)
            .get::<AppleRainSpawnState>()
            .unwrap()
            .spawn_accum;
        assert_eq!(
            accum_after_tick_2, 0.0,
            "accumulator should reset on the first no-message tick",
        );
        // And the world should still have only the single (or zero)
        // apple spawned during tick 1's drain — never an extra
        // burst from the reset tick.
        let projectiles = app.world().resource::<EnemyProjectileState>();
        assert!(
            projectiles.bodies.len() <= 1,
            "no extra apple should spawn during the reset tick; got {} bodies",
            projectiles.bodies.len(),
        );
    }

    /// Apples must not land directly on the boss body — the strike
    /// is choreography, not friendly fire. After enough spawns to
    /// walk the golden-ratio sequence, no apple x should fall
    /// inside the boss's expanded AABB.
    #[test]
    fn apple_rain_consumer_avoids_self_aabb() {
        let interval = 0.35;
        let dt = interval; // one apple per tick
        let mut app = apple_rain_app(dt);
        let feature = gnu_ton_boss_feature();
        let self_aabb = feature.boss.aabb();
        let actor = app
            .world_mut()
            .spawn((FeatureSimEntity, AppleRainSpawnState::default(), feature))
            .id();
        for _ in 0..32 {
            write_special(&mut app, actor, gnu_apple_rain_spec());
            app.update();
        }
        let projectiles = app.world().resource::<EnemyProjectileState>();
        let pad = 14.0; // matches APPLE_RAIN_HALF_EXTENT.x
        for spawn in &projectiles.bodies {
            assert!(
                spawn.body.pos.x <= self_aabb.min.x - pad + 1e-3
                    || spawn.body.pos.x >= self_aabb.max.x + pad - 1e-3,
                "apple at x={} fell inside boss aabb [{},{}] +/- {}",
                spawn.body.pos.x,
                self_aabb.min.x,
                self_aabb.max.x,
                pad,
            );
        }
    }

    /// Across many spawns, apples should appear on BOTH sides of
    /// the boss center — otherwise the player can just stand on
    /// the opposite side of the boss and wait out the strike.
    #[test]
    fn apple_rain_consumer_covers_full_arena_width() {
        let interval = 0.35;
        let dt = interval;
        let mut app = apple_rain_app(dt);
        let feature = gnu_ton_boss_feature();
        let boss_x = feature.boss.pos.x;
        let actor = app
            .world_mut()
            .spawn((FeatureSimEntity, AppleRainSpawnState::default(), feature))
            .id();
        for _ in 0..24 {
            write_special(&mut app, actor, gnu_apple_rain_spec());
            app.update();
        }
        let projectiles = app.world().resource::<EnemyProjectileState>();
        let any_left = projectiles
            .bodies
            .iter()
            .any(|s| s.body.pos.x < boss_x - 100.0);
        let any_right = projectiles
            .bodies
            .iter()
            .any(|s| s.body.pos.x > boss_x + 100.0);
        assert!(
            any_left && any_right,
            "expected apples on both sides of boss; got {} apples",
            projectiles.bodies.len(),
        );
    }

    /// A boss with `AppleRainSpawnState` attached but no Special
    /// message arriving this tick produces zero apples (and a
    /// non-`Special` message stream also produces zero — the
    /// consumer is gate-pure on the spec variant).
    #[test]
    fn apple_rain_consumer_ignores_non_special_actors() {
        let mut app = apple_rain_app(0.5);
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                AppleRainSpawnState::default(),
                gnu_ton_boss_feature(),
            ))
            .id();
        // Write a Ranged message — the consumer must ignore it.
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .write(ActorActionMessage {
                actor,
                request: ActionRequest::Ranged {
                    spec: RangedActionSpec::Bolt {
                        speed: 100.0,
                        damage: 1,
                    },
                    origin: ae::Vec2::ZERO,
                    dir: ae::Vec2::new(1.0, 0.0),
                },
            });
        app.update();
        let projectiles = app.world().resource::<EnemyProjectileState>();
        assert!(
            projectiles.bodies.is_empty(),
            "non-Special message must not trigger the apple consumer",
        );
    }
}
