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
use crate::engine_core::AabbExt;
use bevy::prelude::*;

use crate::audio::SfxMessage;
use crate::brain::{
    action_set::ActionRequest, ActorActionMessage, BossAttackProfile, BossAttackState,
    SpecialActionSpec,
};
use crate::enemy_projectile::EnemyProjectileSpawn;
use crate::features::components::ActorFaction;
use crate::features::ecs::actors::ActorRuntime;
use crate::features::ecs::boss_clusters::BossClusterRef;
// World-anchored damage boxes (pit-trap, rotating-cross) spawn via
// `crate::effects::spawn_damage_box`; the `Hitbox` types are only needed by
// this module's tests now (imported there).
use crate::features::ecs::FeatureSimEntity;
use crate::projectile::SpawnProjectile;
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
    mut spawn_projectiles: MessageWriter<SpawnProjectile>,
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
        spawn_projectiles.write(SpawnProjectile::enemy(spawn, shot_faction));
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

/// Per-boss apple-rain accumulator: state moved out of `BossRuntime`
/// to keep the runtime focused on body/HP and to let the EFFECTS
/// consumer (`spawn_gnu_apple_rain_from_special_messages`) own the
/// per-tick spawn cadence. Defaulted-attached to every boss; only
/// the gnu_ton encounter advances it (its ActionSet's `special` is
/// `SpecialActionSpec::DebrisRain`, so only it generates the
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

/// Horizontal spawn lane (world x) for the `spawn_index`-th GNU-ton
/// apple. Apples spread across the playable width by a golden-ratio
/// sequence — even coverage without an obvious left-to-right sweep —
/// then slide out from under the boss body so an apple never spawns
/// already overlapping the boss head; it picks the nearer boss edge to
/// keep the dodge small. Pure so the distribution + dodge are
/// unit-testable independently of the message/projectile plumbing.
fn apple_rain_spawn_x(spawn_index: u32, world_width: f32, boss_aabb: ae::Aabb) -> f32 {
    let margin = APPLE_RAIN_HALF_EXTENT.x + 8.0;
    let max_x = (world_width - margin).max(margin);
    let spawnable_width = (max_x - margin).max(0.0);
    let frac = ((spawn_index as f32) * PHI_FRAC).fract();
    let mut spawn_x = margin + frac * spawnable_width;
    let self_left = boss_aabb.min.x - APPLE_RAIN_HALF_EXTENT.x;
    let self_right = boss_aabb.max.x + APPLE_RAIN_HALF_EXTENT.x;
    if spawn_x > self_left && spawn_x < self_right {
        spawn_x = if spawn_x - self_left < self_right - spawn_x {
            self_left
        } else {
            self_right
        };
        spawn_x = spawn_x.clamp(margin, max_x);
    }
    spawn_x
}

/// Spawn GNU-ton's apple rain in response to
/// `ActorActionMessage::Special { spec: SpecialActionSpec::DebrisRain }`.
/// The boss runtime tags `frame.special_pressed = true` every tick
/// its `BossAttackProfile::DebrisRain` strike window is active;
/// the resolver translates that into one `Special` message per
/// tick. This consumer owns the spawn cadence, the
/// golden-ratio x distribution, and the self-aabb dodge that keeps
/// apples from landing on the giant's own head — all of which used
/// to live inside `BossRuntime::tick_apple_rain`.
///
/// Bosses whose Special slot is something other than DebrisRain
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
    mut spawn_projectiles: MessageWriter<SpawnProjectile>,
    mut bosses: Query<(Entity, &mut AppleRainSpawnState, BossClusterRef), With<FeatureSimEntity>>,
) {
    let dt = world_time.sim_dt();
    // Bosses with a `Special::DebrisRain` request this tick.
    // Multiple messages from the same boss collapse onto the same
    // entry — the consumer treats "any DebrisRain message this
    // tick" as "the strike window is active this tick".
    let mut active_params: std::collections::HashMap<Entity, (f32, f32, i32)> =
        std::collections::HashMap::new();
    for msg in messages.read() {
        let ActionRequest::Special { spec } = msg.request else {
            continue;
        };
        let SpecialActionSpec::DebrisRain {
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
        let boss = boss_feature.as_boss_ref();
        if !boss.status.alive || interval_s <= 0.0 {
            continue;
        }
        state.spawn_accum += dt;
        let self_aabb = boss.aabb();
        while state.spawn_accum >= interval_s {
            state.spawn_accum -= interval_s;
            // Golden-ratio spread across the playable width, slid out
            // from under the boss body. See `apple_rain_spawn_x`.
            let spawn_x = apple_rain_spawn_x(state.spawn_index, world.0.size.x, self_aabb);
            let spawn_y = (boss.kin.pos.y - APPLE_RAIN_SPAWN_HEIGHT_ABOVE_PLAYER)
                .max(APPLE_RAIN_HALF_EXTENT.y + 8.0);
            spawn_projectiles.write(SpawnProjectile::enemy(
                EnemyProjectileSpawn {
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
                        crate::features::bosses::GNU_TON_APPLE_OWNER_PREFIX,
                        boss.config.id,
                    ),
                    gravity: APPLE_RAIN_GRAVITY,
                },
                crate::projectile::ProjectileFaction::Enemy,
            ));
            state.spawn_index = state.spawn_index.wrapping_add(1);
        }
    }
}

/// Helper: combat-tuning lookup. Lives on the test side to make
/// the helper available to the unit tests below without leaking
/// `SandboxFeelTuning` through the public API.
#[cfg(test)]
fn default_combat_tuning() -> crate::features::events::FeatureCombatTuning {
    SandboxFeelTuning::default().feature_combat_tuning()
}

// =================================================================
// Gradient Sentinel specials — state components + EFFECTS consumers
// =================================================================
//
// The Gradient Sentinel boss carries four distinct specials that
// don't fit the single `ActionSet::special` slot. Each special has
// its own per-boss state component and EFFECTS consumer:
//
//   MemorizedVolley     → sample player positions during telegraph,
//                       fire bolts at all samples on strike edge.
//   PitTrap        → spawn a World-anchored pit hitbox at player
//                       pos on strike edge + a puppy_slug minion.
//   RotatingCross       → spawn 2 World hitboxes around the boss (one
//                       horizontal arm, one vertical) and rotate which
//                       arm has damage live over the strike duration.
//   MinionCascade   → spawn N "slop" minions (small_lurker) at the
//                       top of the arena on strike edge.
//
// All four follow the AppleRain consumer pattern: per-boss state
// component, read `ActorActionMessage::Special { spec }` matched to
// the variant, advance/reset state from the message stream + the
// boss's live `BossAttackState`. The brain emits the Special
// messages directly from `tick_boss_brains_system` via
// `boss_special_for_profile` (see `crate::features::bosses`).

/// Per-boss state for MemorizedVolley. Sampled positions are
/// memorized during the telegraph window; the strike edge fires one
/// bolt at every sample.
#[derive(Component, Clone, Debug, Default)]
pub struct OverfitVolleyState {
    /// Player positions sampled during the active telegraph.
    pub samples: Vec<ae::Vec2>,
    /// Seconds since the last sample. Drains when `>= sample_interval_s`.
    pub sample_accum: f32,
    /// Tracks the per-strike "have we fired yet?" gate. Reset when
    /// the strike window closes (telegraph or active drops the
    /// MemorizedVolley profile).
    pub fired_this_strike: bool,
    /// Tracks the previous tick's "in-attack" status so the seed
    /// sample only happens once per telegraph (not every tick the
    /// state machine reports telegraph_profile).
    pub had_seed_sample: bool,
}

/// Per-boss state for the Smirking Behemoth eye beam. The telegraph
/// locks an approximate target point and the strike spawns a single
/// line of fast projectile boxes toward it.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct EyeBeamState {
    pub locked_target: Option<ae::Vec2>,
    pub fired_this_strike: bool,
}

/// PitTrap is a one-shot per-strike action (spawn pit hitbox +
/// minion at strike edge). State is just the "fired" gate — the pit
/// hitbox + minion are independent entities once spawned, so no
/// further per-boss state is needed.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct MinimaTrapState {
    pub fired_this_strike: bool,
    /// Per-spawn counter so each pit + minion entity gets a unique id
    /// (so the inspector / save sync doesn't collide).
    pub spawn_index: u32,
}

/// Per-boss state for RotatingCross. Tracks which axis (horizontal arm
/// or vertical arm) is currently the damaging one + how much time
/// is left in this axis before the toggle.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SaddlePointState {
    /// Whether the strike is currently active. Reset on no-message
    /// ticks so we re-spawn the hitbox entities on the next strike.
    pub strike_active: bool,
    /// Which axis is "live" — true = horizontal arm, false = vertical.
    pub axis_horizontal: bool,
    /// Seconds left in the current axis before toggling.
    pub axis_remaining_s: f32,
    /// Hitbox entities for each arm; tracked so the rotation can
    /// despawn/replace them on toggle. `None` between strikes.
    pub horizontal_hitbox: Option<Entity>,
    pub vertical_hitbox: Option<Entity>,
}

/// MinionCascade is one-shot per strike (spawn N minions at strike
/// edge). State is just the "fired" gate plus a spawn counter for
/// unique minion ids.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GradientCascadeState {
    pub fired_this_strike: bool,
    pub spawn_index: u32,
}

/// MemorizedVolley constants reused from the spec but baked here so
/// the consumer doesn't need to round-trip through the spec on every
/// telegraph tick (the spec only arrives via the strike-tick
/// message; sampling happens during telegraph too). Tuning lives in
/// `crate::features::bosses` — these are local mirrors.
const OVERFIT_VOLLEY_BOLT_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(8.0, 8.0);
const OVERFIT_VOLLEY_BOLT_LIFETIME: f32 = 2.4;
const OVERFIT_VOLLEY_OWNER_PREFIX: &str = "gradient_sentinel_overfit";

/// EFFECTS consumer: MemorizedVolley position-sampling bolt barrage.
///
/// Reads two things per tick:
///
/// 1. `BossAttackState.telegraph_profile` — when set to
///    `MemorizedVolley`, the consumer samples the player's position at
///    every `OVERFIT_VOLLEY_SAMPLE_INTERVAL_S` and pushes onto
///    `OverfitVolleyState.samples` (capped at `OVERFIT_VOLLEY_SAMPLE_COUNT`).
/// 2. `ActorActionMessage::Special { spec: MemorizedVolley { .. } }` —
///    arrives every tick the strike is active; the consumer fires
///    one bolt per memorized sample on the first such message
///    (gated by `fired_this_strike`).
///
/// When neither telegraph nor strike is active for this profile, the
/// state resets to a clean slate so the next strike window starts
/// from zero.
pub fn spawn_overfit_volley_from_special_messages(
    world_time: Res<WorldTime>,
    mut spawn_projectiles: MessageWriter<SpawnProjectile>,
    mut messages: MessageReader<ActorActionMessage>,
    // Per-actor target: each boss carries an `ActorTarget` populated
    // upstream by `select_actor_targets` (nearest-player resolution).
    // Reading the target's player kinematics by Entity makes this
    // system multi-player ready — single-player behavior is preserved
    // because there's only one player today.
    player_query: Query<&crate::player::BodyKinematics, With<crate::player::PlayerEntity>>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &BossAttackState,
            &mut OverfitVolleyState,
            Option<&super::super::components::ActorTarget>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    use crate::features::bosses::{OVERFIT_VOLLEY_SAMPLE_COUNT, OVERFIT_VOLLEY_SAMPLE_INTERVAL_S};
    let dt = world_time.sim_dt();

    let mut active_strike_params: std::collections::HashMap<Entity, (f32, i32)> =
        std::collections::HashMap::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec:
                SpecialActionSpec::MemorizedVolley {
                    shot_speed, damage, ..
                },
        } = msg.request
        {
            active_strike_params.insert(msg.actor, (shot_speed, damage));
        }
    }

    for (entity, boss_feature, attack_state, mut state, actor_target) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        // Per-boss target: read kinematics for the player this boss
        // is tracking. Falls back to `actor_target.pos` (set by
        // `select_actor_targets` even when the player entity is None)
        // when present; tests that spawn bosses without an
        // `ActorTarget` exercise the fully-absent path with no
        // sample fallback (consumer gates on Some).
        let player_pos = actor_target.and_then(|t| {
            t.entity
                .and_then(|e| player_query.get(e).ok())
                .map(|kin| kin.aabb().center())
                .or(Some(t.pos))
        });
        if !boss.status.alive {
            // Dead boss: clear samples so a respawned-then-attacking
            // boss doesn't inherit stale memory.
            state.samples.clear();
            state.sample_accum = 0.0;
            state.fired_this_strike = false;
            state.had_seed_sample = false;
            continue;
        }

        let in_telegraph = matches!(
            attack_state.telegraph_profile,
            Some(BossAttackProfile::MemorizedVolley)
        );
        let strike_params = active_strike_params.get(&entity).copied();

        if in_telegraph {
            // Seed an initial sample on the first telegraph tick so
            // even a static player gets at least one bolt.
            if !state.had_seed_sample {
                if let Some(pos) = player_pos {
                    state.samples.push(pos);
                }
                state.had_seed_sample = true;
                state.sample_accum = 0.0;
            }
            state.sample_accum += dt;
            while state.sample_accum >= OVERFIT_VOLLEY_SAMPLE_INTERVAL_S {
                state.sample_accum -= OVERFIT_VOLLEY_SAMPLE_INTERVAL_S;
                if state.samples.len() < OVERFIT_VOLLEY_SAMPLE_COUNT as usize {
                    if let Some(pos) = player_pos {
                        state.samples.push(pos);
                    }
                }
            }
            // Strike hasn't fired yet — keep the gate open.
            state.fired_this_strike = false;
        } else if let Some((shot_speed, damage)) = strike_params {
            if !state.fired_this_strike {
                let origin = boss.kin.pos + boss.config.behavior.projectile_origin_offset;
                for sample_pos in state.samples.iter() {
                    let delta = *sample_pos - origin;
                    let dir = delta.normalize_or_zero();
                    if dir.length_squared() < 1e-4 {
                        continue;
                    }
                    spawn_projectiles.write(SpawnProjectile::enemy(
                        EnemyProjectileSpawn {
                            origin,
                            dir,
                            speed: shot_speed,
                            damage,
                            max_lifetime: OVERFIT_VOLLEY_BOLT_LIFETIME,
                            half_extent: OVERFIT_VOLLEY_BOLT_HALF_EXTENT,
                            owner_id: format!("{}:{}", OVERFIT_VOLLEY_OWNER_PREFIX, boss.config.id),
                            gravity: 0.0,
                        },
                        crate::projectile::ProjectileFaction::Enemy,
                    ));
                }
                state.fired_this_strike = true;
                state.samples.clear();
                state.had_seed_sample = false;
            }
        } else {
            // Not telegraphing and not striking — reset for next cycle.
            state.samples.clear();
            state.sample_accum = 0.0;
            state.fired_this_strike = false;
            state.had_seed_sample = false;
        }
    }
}

const EYE_BEAM_OWNER_PREFIX: &str = "smirking_behemoth_eye_beam";

/// EFFECTS consumer: Smirking Behemoth eye beam.
///
/// During the `LockOnBeam` telegraph the boss locks the currently tracked
/// player position. On the first strike tick it emits a short line of
/// fast bubble-laser projectile boxes from the eye toward that locked
/// point. This deliberately does **not** reuse MemorizedVolley's sample
/// barrage, because the cut-rope boss needs one readable beam rather
/// than a cloud of slow memorized shots.
pub fn spawn_eye_beam_from_special_messages(
    mut spawn_projectiles: MessageWriter<SpawnProjectile>,
    mut messages: MessageReader<ActorActionMessage>,
    player_query: Query<&crate::player::BodyKinematics, With<crate::player::PlayerEntity>>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &BossAttackState,
            &mut EyeBeamState,
            Option<&super::super::components::ActorTarget>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let mut active_strike_params: std::collections::HashMap<
        Entity,
        (f32, i32, u8, f32, f32, f32, f32),
    > = std::collections::HashMap::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec:
                SpecialActionSpec::LockOnBeam {
                    shot_speed,
                    damage,
                    box_count,
                    box_spacing,
                    half_extent_x,
                    half_extent_y,
                    lifetime_s,
                },
        } = msg.request
        {
            active_strike_params.insert(
                msg.actor,
                (
                    shot_speed,
                    damage,
                    box_count,
                    box_spacing,
                    half_extent_x,
                    half_extent_y,
                    lifetime_s,
                ),
            );
        }
    }

    for (entity, boss_feature, attack_state, mut state, actor_target) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        let player_pos = actor_target.and_then(|t| {
            t.entity
                .and_then(|e| player_query.get(e).ok())
                .map(|kin| kin.aabb().center())
                .or(Some(t.pos))
        });
        if !boss.status.alive {
            state.locked_target = None;
            state.fired_this_strike = false;
            continue;
        }

        let in_telegraph = matches!(
            attack_state.telegraph_profile,
            Some(BossAttackProfile::LockOnBeam)
        );
        let strike_params = active_strike_params.get(&entity).copied();
        if in_telegraph {
            if state.locked_target.is_none() {
                state.locked_target = player_pos;
            }
            state.fired_this_strike = false;
            continue;
        }

        let Some((shot_speed, damage, box_count, box_spacing, half_x, half_y, lifetime_s)) =
            strike_params
        else {
            state.locked_target = None;
            state.fired_this_strike = false;
            continue;
        };
        if state.fired_this_strike {
            continue;
        }
        let target = state.locked_target.or(player_pos).unwrap_or(boss.kin.pos);
        let offset = ae::Vec2::new(
            boss.config.behavior.projectile_origin_offset.x * boss.kin.facing.signum(),
            boss.config.behavior.projectile_origin_offset.y,
        );
        let origin = boss.kin.pos + offset;
        let delta = target - origin;
        let dir = if delta.length_squared() < 1e-4 {
            ae::Vec2::new(boss.kin.facing.signum(), 0.0)
        } else {
            delta.normalize()
        };
        let count = box_count.max(1);
        let spacing = box_spacing.max(1.0);
        for i in 0..count {
            let beam_origin = origin + dir * spacing * f32::from(i);
            spawn_projectiles.write(SpawnProjectile::enemy(
                EnemyProjectileSpawn {
                    origin: beam_origin,
                    dir,
                    speed: shot_speed.max(1.0),
                    damage,
                    max_lifetime: lifetime_s.max(0.05),
                    half_extent: ae::Vec2::new(half_x.max(1.0), half_y.max(1.0)),
                    owner_id: format!("{}:{}", EYE_BEAM_OWNER_PREFIX, boss.config.id),
                    gravity: 0.0,
                },
                crate::projectile::ProjectileFaction::Enemy,
            ));
        }
        state.fired_this_strike = true;
        state.locked_target = None;
    }
}

const MINIMA_TRAP_OWNER_PREFIX: &str = "gradient_sentinel_minima";
const MINIMA_TRAP_KNOCKBACK: f32 = 1.4;
/// Minion archetype id spawned by the trap (puppy_slug — pacifist
/// crawler).
const MINIMA_TRAP_MINION_ARCHETYPE: &str = "puppy_slug";
const MINIMA_TRAP_MINION_HALF_SIZE: ae::Vec2 = ae::Vec2::new(24.0, 11.0);
/// Horizontal offset (px) from the pit center where the minion
/// spawns. Pushed toward the boss side so the player sees the
/// slug appear *next* to the pit instead of *under* them. 90 px
/// is well outside both the pit's 56-px half-extent and the
/// player's body so the slug never overlaps the player on the
/// frame it appears.
const MINIMA_TRAP_MINION_SPAWN_OFFSET_PX: f32 = 90.0;

/// EFFECTS consumer: PitTrap pit + optional puppy_slug.
///
/// On the first Special message of a strike (gated by
/// `MinimaTrapState.fired_this_strike`):
/// - Spawn a World-anchored hitbox at the player's current position
///   with `half_extent_x/y` and `hazard_duration_s` lifetime.
/// - Optionally spawn a puppy_slug minion at the same position so
///   the player has a moving threat to deal with alongside the pit.
///
/// The hitbox is a regular `Hitbox` entity, so it flows through the
/// standard `apply_hitbox_damage` → `HitEvent` path. The
/// once-per-strike `HitboxHits` set ensures the player takes at
/// most one hit per pit lifetime.
pub fn spawn_minima_trap_from_special_messages(
    mut commands: Commands,
    mut messages: MessageReader<ActorActionMessage>,
    // Per-boss target via `ActorTarget` (populated by
    // `select_actor_targets`); same multi-player-ready pattern as
    // the overfit-volley consumer above.
    player_query: Query<&crate::player::BodyKinematics, With<crate::player::PlayerEntity>>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &mut MinimaTrapState,
            Option<&super::super::components::ActorTarget>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let mut active_strike_params: std::collections::HashMap<Entity, (f32, i32, f32, f32, bool)> =
        std::collections::HashMap::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec:
                SpecialActionSpec::PitTrap {
                    hazard_duration_s,
                    damage,
                    half_extent_x,
                    half_extent_y,
                    spawn_minion,
                },
        } = msg.request
        {
            active_strike_params.insert(
                msg.actor,
                (
                    hazard_duration_s,
                    damage,
                    half_extent_x,
                    half_extent_y,
                    spawn_minion,
                ),
            );
        }
    }

    for (entity, boss_feature, mut state, actor_target) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        let player_pos = actor_target.and_then(|t| {
            t.entity
                .and_then(|e| player_query.get(e).ok())
                .map(|kin| kin.aabb().center())
                .or(Some(t.pos))
        });
        let Some(params) = active_strike_params.get(&entity).copied() else {
            // Strike window closed — reset the fired gate so the next
            // strike re-spawns the pit.
            state.fired_this_strike = false;
            continue;
        };
        if !boss.status.alive {
            continue;
        }
        if state.fired_this_strike {
            continue;
        }
        let (hazard_duration_s, damage, hx, hy, spawn_minion) = params;
        let pit_center = player_pos.unwrap_or(boss.kin.pos);

        crate::effects::spawn_damage_box(
            &mut commands,
            entity,
            ActorFaction::Boss,
            pit_center,
            crate::effects::DamageBox {
                half_extent: ae::Vec2::new(hx, hy),
                damage,
                knockback: MINIMA_TRAP_KNOCKBACK,
                lifetime_s: hazard_duration_s.max(0.05),
                name: None,
            },
        );

        if spawn_minion {
            let minion_id = format!(
                "{}_minion:{}:{}",
                MINIMA_TRAP_OWNER_PREFIX, boss.config.id, state.spawn_index
            );
            // Encounter id = boss's canonical behavior id (resolved
            // at spawn from the brain's `PhaseScript:` payload).
            // Using `boss.config.behavior.id` instead of
            // `encounter_id_from_name(boss.config.name)` handles the
            // case where an LDtk BossSpawn carries a flavor name like
            // "System Boss" — the minion's encounter scope still
            // matches the parent encounter even though name != id.
            let encounter_id = boss.config.behavior.id.clone();
            // Don't spawn the slug right on top of the player —
            // the user reported the slug appearing under them with
            // no dodge window. Offset the slug horizontally toward
            // the BOSS so the player sees it appear from the
            // boss-side of the pit and has time to retreat. Half
            // the spawn offset distance is the slug's half-width
            // plus a comfortable read margin.
            let player_to_boss = boss.kin.pos - pit_center;
            let toward_boss_x = if player_to_boss.x.abs() < f32::EPSILON {
                // Player directly aligned with boss — spawn left
                // of the pit as a deterministic fallback so the
                // slug never appears AT the pit center.
                -1.0
            } else {
                player_to_boss.x.signum()
            };
            let minion_offset_px = MINIMA_TRAP_MINION_SPAWN_OFFSET_PX;
            let minion_pos = ae::Vec2::new(
                pit_center.x + toward_boss_x * minion_offset_px,
                pit_center.y,
            );
            crate::features::ecs::spawn::spawn_runtime_minion(
                &mut commands,
                minion_id,
                "Puppy Slug",
                minion_pos,
                MINIMA_TRAP_MINION_HALF_SIZE,
                MINIMA_TRAP_MINION_ARCHETYPE,
                encounter_id,
                crate::features::ActorFaction::Enemy,
                crate::features::ActorAggression::hostile_to_player(),
            );
        }

        state.fired_this_strike = true;
        state.spawn_index = state.spawn_index.wrapping_add(1);
    }
}

const SADDLE_POINT_KNOCKBACK: f32 = 1.6;

/// EFFECTS consumer: RotatingCross rotating cross hazard.
///
/// On the first Special message of a strike, spawns two World-anchored
/// hitbox entities centered on the boss — one horizontal arm, one
/// vertical arm. Only the "live" axis carries non-zero damage; the
/// inactive axis is despawned. Every `axis_period_s` seconds the
/// active axis toggles: the live hitbox is despawned and the other
/// arm is spawned in its place. This creates a readable "stand on
/// the safe axis" puzzle for the player.
///
/// The boss may move during the strike (AnchorSway profile), so the
/// hitboxes use the boss entity as their anchor base by being
/// re-spawned at the boss position each toggle. (A future
/// `HitboxAnchor::FollowOwner` with a per-arm long offset would
/// move the cross with the boss in real time — out of scope here.)
pub fn spawn_saddle_point_from_special_messages(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    mut messages: MessageReader<ActorActionMessage>,
    mut bosses: Query<(Entity, BossClusterRef, &mut SaddlePointState), With<FeatureSimEntity>>,
) {
    let dt = world_time.sim_dt();

    let mut active_strike_params: std::collections::HashMap<Entity, (f32, f32, f32, i32)> =
        std::collections::HashMap::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec:
                SpecialActionSpec::RotatingCross {
                    arm_length,
                    arm_thickness,
                    axis_period_s,
                    damage,
                },
        } = msg.request
        {
            active_strike_params.insert(
                msg.actor,
                (arm_length, arm_thickness, axis_period_s, damage),
            );
        }
    }

    for (entity, boss_feature, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        let Some((arm_length, arm_thickness, axis_period_s, damage)) =
            active_strike_params.get(&entity).copied()
        else {
            // Strike closed — despawn any lingering hitboxes and
            // reset state so the next strike starts clean.
            if let Some(h) = state.horizontal_hitbox.take() {
                commands.entity(h).despawn();
            }
            if let Some(h) = state.vertical_hitbox.take() {
                commands.entity(h).despawn();
            }
            state.strike_active = false;
            state.axis_remaining_s = 0.0;
            continue;
        };
        if !boss.status.alive {
            if let Some(h) = state.horizontal_hitbox.take() {
                commands.entity(h).despawn();
            }
            if let Some(h) = state.vertical_hitbox.take() {
                commands.entity(h).despawn();
            }
            continue;
        }
        let period = axis_period_s.max(0.05);

        // Strike start (or re-start after a between-strike gap):
        // spawn the initial active hitbox + reset rotation timer.
        // The boss may move during the strike (AnchorSway), so each
        // toggle re-spawns at the *current* boss center.
        let spawn_axis_hitbox = |commands: &mut Commands, axis_horizontal: bool| -> Entity {
            let (he_x, he_y) = if axis_horizontal {
                (arm_length, arm_thickness)
            } else {
                (arm_thickness, arm_length)
            };
            // Lifetime > axis_period_s so the hitbox doesn't expire
            // mid-axis. We despawn it on toggle or strike end.
            crate::effects::spawn_damage_box(
                commands,
                entity,
                ActorFaction::Boss,
                boss.kin.pos,
                crate::effects::DamageBox {
                    half_extent: ae::Vec2::new(he_x, he_y),
                    damage,
                    knockback: SADDLE_POINT_KNOCKBACK,
                    lifetime_s: period * 2.0,
                    name: None,
                },
            )
        };

        if !state.strike_active {
            // First tick of the strike — clear any leftovers and
            // spawn the first axis.
            if let Some(h) = state.horizontal_hitbox.take() {
                commands.entity(h).despawn();
            }
            if let Some(h) = state.vertical_hitbox.take() {
                commands.entity(h).despawn();
            }
            // Start on the horizontal axis (matches the visual
            // expectation: cross forms, horizontal arm lights up
            // first, then alternates).
            state.axis_horizontal = true;
            state.horizontal_hitbox = Some(spawn_axis_hitbox(&mut commands, true));
            state.vertical_hitbox = None;
            state.axis_remaining_s = period;
            state.strike_active = true;
            continue;
        }

        // Continuing strike — advance axis timer; toggle on expiry.
        state.axis_remaining_s = (state.axis_remaining_s - dt).max(0.0);
        if state.axis_remaining_s <= 0.0 {
            state.axis_horizontal = !state.axis_horizontal;
            // Despawn previous axis, spawn the new one.
            if let Some(h) = state.horizontal_hitbox.take() {
                commands.entity(h).despawn();
            }
            if let Some(h) = state.vertical_hitbox.take() {
                commands.entity(h).despawn();
            }
            if state.axis_horizontal {
                state.horizontal_hitbox = Some(spawn_axis_hitbox(&mut commands, true));
            } else {
                state.vertical_hitbox = Some(spawn_axis_hitbox(&mut commands, false));
            }
            state.axis_remaining_s = period;
        }
    }
}

const GRADIENT_CASCADE_MINION_ARCHETYPE: &str = "small_lurker";
const GRADIENT_CASCADE_MINION_HALF_SIZE: ae::Vec2 = ae::Vec2::new(15.0, 20.0);
/// Vertical y where slop minions spawn (top of the arena, just below
/// the ceiling). The arena ceiling sits at y=32; minions spawn at
/// y=80 so they're visibly inside the play space rather than clipping
/// the ceiling overlay.
const GRADIENT_CASCADE_SPAWN_Y: f32 = 80.0;
/// Horizontal spread (px from arena center) for spawning N minions.
const GRADIENT_CASCADE_X_SPREAD: f32 = 220.0;

/// Even horizontal offset (px from the boss x) for the `i`-th of
/// `count` gradient-cascade minions, spread across
/// `[-X_SPREAD, +X_SPREAD]`. A lone minion drops on the boss x; N≥2
/// place the first and last at the spread edges with even spacing
/// between. Pure so the spacing is unit-testable.
fn gradient_cascade_minion_x_offset(i: i32, count: i32) -> f32 {
    let t = if count <= 1 {
        0.5
    } else {
        i as f32 / (count - 1) as f32
    };
    (t - 0.5) * 2.0 * GRADIENT_CASCADE_X_SPREAD
}

/// EFFECTS consumer: MinionCascade — spawn N "slop" minions at the
/// top of the arena.
///
/// One-shot per strike. Spawns `minion_count` `small_lurker`
/// minions in a horizontal spread at `GRADIENT_CASCADE_SPAWN_Y`,
/// centered on the boss x. Gravity carries them down toward the
/// player; their default `MeleeBrute` brain chases on contact.
pub fn spawn_gradient_cascade_minions_from_special_messages(
    mut commands: Commands,
    mut messages: MessageReader<ActorActionMessage>,
    mut bosses: Query<(Entity, BossClusterRef, &mut GradientCascadeState), With<FeatureSimEntity>>,
) {
    let mut active_strike_params: std::collections::HashMap<Entity, u8> =
        std::collections::HashMap::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::MinionCascade { minion_count },
        } = msg.request
        {
            active_strike_params.insert(msg.actor, minion_count);
        }
    }

    for (entity, boss_feature, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        let Some(minion_count) = active_strike_params.get(&entity).copied() else {
            // Strike closed — reset gate.
            state.fired_this_strike = false;
            continue;
        };
        if !boss.status.alive {
            continue;
        }
        if state.fired_this_strike {
            continue;
        }
        let count = minion_count.max(1) as i32;
        // Spread N minions evenly across [-X_SPREAD, +X_SPREAD] around
        // the boss x.
        // Encounter id = boss's canonical behavior id (see the
        // PitTrap consumer above for the name-vs-id rationale).
        let encounter_id = boss.config.behavior.id.clone();
        for i in 0..count {
            let x_off = gradient_cascade_minion_x_offset(i, count);
            let spawn_pos = ae::Vec2::new(boss.kin.pos.x + x_off, GRADIENT_CASCADE_SPAWN_Y);
            let minion_id = format!(
                "gradient_sentinel_cascade:{}:{}:{}",
                boss.config.id, state.spawn_index, i
            );
            crate::features::ecs::spawn::spawn_runtime_minion(
                &mut commands,
                minion_id,
                "Slop Lurker",
                spawn_pos,
                GRADIENT_CASCADE_MINION_HALF_SIZE,
                GRADIENT_CASCADE_MINION_ARCHETYPE,
                encounter_id.clone(),
                crate::features::ActorFaction::Enemy,
                crate::features::ActorAggression::hostile_to_player(),
            );
        }
        state.fired_this_strike = true;
        state.spawn_index = state.spawn_index.wrapping_add(1);
    }
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

    #[test]
    fn gradient_cascade_minion_offsets_spread_symmetrically() {
        // A lone minion drops on the boss x.
        assert_eq!(gradient_cascade_minion_x_offset(0, 1), 0.0);
        // Two minions land on the spread edges.
        assert_eq!(
            gradient_cascade_minion_x_offset(0, 2),
            -GRADIENT_CASCADE_X_SPREAD
        );
        assert_eq!(
            gradient_cascade_minion_x_offset(1, 2),
            GRADIENT_CASCADE_X_SPREAD
        );
        // An odd count puts the middle minion on the boss x and the
        // ends symmetric about it.
        let n = 5;
        assert_eq!(gradient_cascade_minion_x_offset(2, n), 0.0);
        let first = gradient_cascade_minion_x_offset(0, n);
        let last = gradient_cascade_minion_x_offset(n - 1, n);
        assert!((first + last).abs() < 1e-3, "ends should be symmetric");
        assert_eq!(first, -GRADIENT_CASCADE_X_SPREAD);
        // Offsets increase monotonically and stay within the spread.
        let mut prev = f32::NEG_INFINITY;
        for i in 0..n {
            let x = gradient_cascade_minion_x_offset(i, n);
            assert!(x > prev, "offsets should be strictly increasing");
            assert!(x.abs() <= GRADIENT_CASCADE_X_SPREAD + 1e-3);
            prev = x;
        }
    }

    #[test]
    fn apple_rain_spawn_x_stays_in_bounds_spreads_and_dodges_the_boss() {
        let world_width = 1792.0;
        let margin = APPLE_RAIN_HALF_EXTENT.x + 8.0;
        let max_x = world_width - margin;
        // Boss body envelope in the arena center.
        let boss = ae::Aabb::new(ae::Vec2::new(896.0, 640.0), ae::Vec2::new(110.0, 110.0));
        let self_left = boss.min.x - APPLE_RAIN_HALF_EXTENT.x;
        let self_right = boss.max.x + APPLE_RAIN_HALF_EXTENT.x;

        let mut xs = Vec::new();
        for i in 0..64u32 {
            let x = apple_rain_spawn_x(i, world_width, boss);
            // (1) Always inside the playable margins.
            assert!(
                x >= margin - 1e-3 && x <= max_x + 1e-3,
                "apple {i} x={x} outside [{margin}, {max_x}]"
            );
            // (2) Never spawns strictly inside the boss-body keep-out
            // band (it slides to the nearer edge).
            assert!(
                x <= self_left + 1e-3 || x >= self_right - 1e-3,
                "apple {i} x={x} inside boss keep-out ({self_left}..{self_right})"
            );
            xs.push(x);
        }
        // (3) The golden-ratio sequence covers both halves of the
        // arena rather than clustering on one side.
        let mid = world_width / 2.0;
        assert!(
            xs.iter().any(|&x| x < mid) && xs.iter().any(|&x| x > mid),
            "apple spawns should spread across both halves: {xs:?}"
        );
    }

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
        app.add_message::<SpawnProjectile>();
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
        // Phase 3b: the consumer emits SpawnProjectile; chain the enemy-pool
        // applier so the projectile entity spawns within the update.
        app.add_systems(
            Update,
            (
                spawn_enemy_projectiles_from_brain_actions,
                crate::enemy_projectile::apply_enemy_spawn_projectile_messages,
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

    use crate::features::bosses::{BossBehaviorProfile, GNU_TON_APPLE_OWNER_PREFIX};
    use crate::features::ecs::boss_clusters::BodyKinematics;
    use crate::features::ecs::boss_clusters::{BossClusterScratch, BossConfig, BossStatus};
    use crate::features::ecs::FeatureSimEntity;
    use crate::GameWorld;

    fn gnu_apple_rain_spec() -> SpecialActionSpec {
        SpecialActionSpec::DebrisRain {
            interval_s: 0.35,
            spawn_speed: 35.0,
            damage: 1,
        }
    }

    fn gnu_ton_boss_feature() -> (BodyKinematics, BossConfig, BossStatus) {
        let aabb = ae::Aabb::new(ae::Vec2::new(500.0, 400.0), ae::Vec2::new(80.0, 80.0));
        let mut scratch = BossClusterScratch::new(
            "boss_gnu_ton",
            "GNU-ton",
            aabb,
            crate::actor::BossBrain::Dormant,
        );
        scratch.config.behavior = BossBehaviorProfile::gnu_ton();
        scratch.into_components()
    }

    fn apple_rain_app(sim_dt: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
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
        app.add_message::<SpawnProjectile>();
        app.add_systems(
            Update,
            (
                spawn_gnu_apple_rain_from_special_messages,
                crate::enemy_projectile::apply_enemy_spawn_projectile_messages,
            )
                .chain(),
        );
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
        let projectiles = enemy_projectile_bodies(&mut app);
        assert_eq!(
            projectiles.len(),
            3,
            "expected one apple per tick at dt==interval, got {}",
            projectiles.len(),
        );
        for spawn in &projectiles {
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
        let projectiles = enemy_projectile_bodies(&mut app);
        assert!(
            projectiles.len() <= 1,
            "no extra apple should spawn during the reset tick; got {} bodies",
            projectiles.len(),
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
        let self_aabb = crate::features::BossRef {
            kin: &feature.0,
            config: &feature.1,
            status: &feature.2,
        }
        .aabb();
        let actor = app
            .world_mut()
            .spawn((FeatureSimEntity, AppleRainSpawnState::default(), feature))
            .id();
        for _ in 0..32 {
            write_special(&mut app, actor, gnu_apple_rain_spec());
            app.update();
        }
        let projectiles = enemy_projectile_bodies(&mut app);
        let pad = 14.0; // matches APPLE_RAIN_HALF_EXTENT.x
        for spawn in &projectiles {
            assert!(
                spawn.body.kin.pos.x <= self_aabb.min.x - pad + 1e-3
                    || spawn.body.kin.pos.x >= self_aabb.max.x + pad - 1e-3,
                "apple at x={} fell inside boss aabb [{},{}] +/- {}",
                spawn.body.kin.pos.x,
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
        let boss_x = feature.0.pos.x;
        let actor = app
            .world_mut()
            .spawn((FeatureSimEntity, AppleRainSpawnState::default(), feature))
            .id();
        for _ in 0..24 {
            write_special(&mut app, actor, gnu_apple_rain_spec());
            app.update();
        }
        let projectiles = enemy_projectile_bodies(&mut app);
        let any_left = projectiles
            .iter()
            .any(|s| s.body.kin.pos.x < boss_x - 100.0);
        let any_right = projectiles
            .iter()
            .any(|s| s.body.kin.pos.x > boss_x + 100.0);
        assert!(
            any_left && any_right,
            "expected apples on both sides of boss; got {} apples",
            projectiles.len(),
        );
    }

    // -----------------------------------------------------------
    // Gradient Sentinel consumer tests
    //
    // Each new EFFECTS consumer (MemorizedVolley, PitTrap,
    // RotatingCross, MinionCascade) gets a small App-driven test
    // that fires a Special message and asserts the consumer
    // produced the right downstream effect. Mirrors the apple-rain
    // pattern: a build_app helper that adds the message channel +
    // the consumer, then write a message and assert.
    //
    // None of these are full end-to-end tests — they exercise the
    // consumer in isolation with mock state. Full schedule
    // integration is covered by the boss_pattern + scripted_pattern
    // tests in their own modules.
    // -----------------------------------------------------------

    fn gradient_sentinel_boss_feature() -> (BodyKinematics, BossConfig, BossStatus) {
        let aabb = ae::Aabb::new(ae::Vec2::new(640.0, 696.0), ae::Vec2::new(64.0, 80.0));
        let mut scratch = BossClusterScratch::new(
            "boss_gradient_sentinel",
            "Gradient Sentinel",
            aabb,
            crate::actor::BossBrain::Dormant,
        );
        scratch.config.behavior = BossBehaviorProfile::clockwork_warden();
        scratch.into_components()
    }

    fn overfit_volley_spec() -> SpecialActionSpec {
        SpecialActionSpec::MemorizedVolley {
            sample_interval_s: 0.30,
            sample_count: 5,
            shot_speed: 360.0,
            damage: 1,
        }
    }

    /// Sanity: MemorizedVolley consumer with a seeded sample fires
    /// projectiles on the strike tick.
    ///
    /// Building a real player entity for this test is heavyweight
    /// (drags the entire player module in). Instead we seed
    /// samples directly into `OverfitVolleyState` to simulate the
    /// telegraph having already happened, then drive one strike
    /// tick and assert projectiles spawned. This pins the
    /// "fire-bolts-at-samples" half of the consumer; the
    /// "sample-during-telegraph" half is exercised by the
    /// scripted_pattern tests + manual playtest.
    #[test]
    fn overfit_volley_consumer_fires_bolts_at_seeded_samples_on_strike() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
        app.init_resource::<WorldTime>();
        // We can't easily build a real player entity without
        // dragging in PlayerPlugin. Reproduce the bolts via a
        // narrower fixture system that doesn't query the player —
        // we exercise the bolts-from-seeded-samples branch by
        // calling spawn_overfit_volley_from_special_messages with
        // a Query that returns no player. The seed-on-telegraph
        // branch then never fires (because in_telegraph is false
        // when telegraph_profile is None and player_pos is also
        // None), so we pre-populate state.samples directly.
        let mut world_time = app.world_mut().resource_mut::<WorldTime>();
        world_time.scaled_dt = 1.0 / 60.0;
        world_time.raw_dt = 1.0 / 60.0;
        app.add_message::<SpawnProjectile>();
        app.add_systems(
            Update,
            (
                spawn_overfit_volley_from_special_messages,
                crate::enemy_projectile::apply_enemy_spawn_projectile_messages,
            )
                .chain(),
        );
        // Boss: alive, no telegraph (so consumer takes the strike
        // branch), with two pre-seeded samples.
        let mut state = OverfitVolleyState::default();
        state.samples.push(ae::Vec2::new(720.0, 696.0)); // right of boss
        state.samples.push(ae::Vec2::new(560.0, 696.0)); // left of boss
        state.had_seed_sample = true;
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                BossAttackState::default(),
                state,
                gradient_sentinel_boss_feature(),
            ))
            .id();
        write_special(&mut app, actor, overfit_volley_spec());
        app.update();
        let projectiles = enemy_projectile_bodies(&mut app);
        assert_eq!(
            projectiles.len(),
            2,
            "expected one bolt per seeded sample (2), got {}",
            projectiles.len(),
        );
        // Owner id must carry the overfit prefix so a future
        // visuals routing can pick a custom sprite.
        for spawn in &projectiles {
            assert!(
                spawn.owner_id.starts_with("gradient_sentinel_overfit"),
                "expected overfit owner_id, got {}",
                spawn.owner_id,
            );
        }
        // State should reset (samples cleared, fired_this_strike=true).
        let state = app
            .world()
            .entity(actor)
            .get::<OverfitVolleyState>()
            .unwrap();
        assert!(state.fired_this_strike);
        assert!(state.samples.is_empty());
    }

    /// MemorizedVolley consumer fires AT MOST once per strike — a
    /// second Special message in the same strike window is a no-op.
    #[test]
    fn overfit_volley_consumer_fires_once_per_strike_then_holds() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<ProjectileSeqCounter>();
        app.init_resource::<WorldTime>();
        let mut wt = app.world_mut().resource_mut::<WorldTime>();
        wt.scaled_dt = 1.0 / 60.0;
        wt.raw_dt = 1.0 / 60.0;
        app.add_message::<SpawnProjectile>();
        app.add_systems(
            Update,
            (
                spawn_overfit_volley_from_special_messages,
                crate::enemy_projectile::apply_enemy_spawn_projectile_messages,
            )
                .chain(),
        );
        let mut state = OverfitVolleyState::default();
        state.samples.push(ae::Vec2::new(720.0, 696.0));
        state.had_seed_sample = true;
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                BossAttackState::default(),
                state,
                gradient_sentinel_boss_feature(),
            ))
            .id();
        for _ in 0..4 {
            write_special(&mut app, actor, overfit_volley_spec());
            app.update();
        }
        let projectiles = enemy_projectile_bodies(&mut app);
        assert_eq!(
            projectiles.len(),
            1,
            "expected exactly one bolt despite 4 strike ticks, got {}",
            projectiles.len(),
        );
    }

    /// PitTrap consumer spawns a World-anchored hitbox at a
    /// position when the Special message arrives. We can't easily
    /// fetch the player position without a full player entity, but
    /// the consumer falls back to the boss position when no player
    /// is found — that's the path this test exercises.
    #[test]
    fn minima_trap_consumer_spawns_world_hitbox_on_strike() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<WorldTime>();
        let mut wt = app.world_mut().resource_mut::<WorldTime>();
        wt.scaled_dt = 1.0 / 60.0;
        wt.raw_dt = 1.0 / 60.0;
        app.add_systems(Update, spawn_minima_trap_from_special_messages);
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                MinimaTrapState::default(),
                gradient_sentinel_boss_feature(),
            ))
            .id();
        write_special(
            &mut app,
            actor,
            SpecialActionSpec::PitTrap {
                hazard_duration_s: 5.0,
                damage: 2,
                half_extent_x: 56.0,
                half_extent_y: 24.0,
                spawn_minion: false, // avoid the runtime minion spawn machinery
            },
        );
        app.update();
        // Count hitboxes — should be exactly one new World-anchored.
        let mut hitboxes = app.world_mut().query::<&Hitbox>();
        let world_hitboxes: Vec<_> = hitboxes
            .iter(app.world())
            .filter(|h| matches!(h.anchor, HitboxAnchor::World { .. }))
            .collect();
        assert_eq!(
            world_hitboxes.len(),
            1,
            "expected exactly one PitTrap hitbox, got {}",
            world_hitboxes.len(),
        );
        let trap = &world_hitboxes[0];
        assert!(matches!(trap.source, ActorFaction::Boss));
        assert_eq!(trap.damage, 2);
        assert_eq!(trap.half_extent, ae::Vec2::new(56.0, 24.0));
        // State should record fired_this_strike.
        let state = app.world().entity(actor).get::<MinimaTrapState>().unwrap();
        assert!(state.fired_this_strike);
    }

    /// PitTrap fires at most once per strike — repeated Special
    /// messages produce only the one hitbox.
    #[test]
    fn minima_trap_consumer_does_not_re_fire_during_strike() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<WorldTime>();
        let mut wt = app.world_mut().resource_mut::<WorldTime>();
        wt.scaled_dt = 1.0 / 60.0;
        wt.raw_dt = 1.0 / 60.0;
        app.add_systems(Update, spawn_minima_trap_from_special_messages);
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                MinimaTrapState::default(),
                gradient_sentinel_boss_feature(),
            ))
            .id();
        let spec = SpecialActionSpec::PitTrap {
            hazard_duration_s: 5.0,
            damage: 2,
            half_extent_x: 56.0,
            half_extent_y: 24.0,
            spawn_minion: false,
        };
        for _ in 0..3 {
            write_special(&mut app, actor, spec);
            app.update();
        }
        let mut hitboxes = app.world_mut().query::<&Hitbox>();
        let world_hitboxes: Vec<_> = hitboxes
            .iter(app.world())
            .filter(|h| matches!(h.anchor, HitboxAnchor::World { .. }))
            .collect();
        assert_eq!(
            world_hitboxes.len(),
            1,
            "expected exactly one hitbox across 3 strike ticks, got {}",
            world_hitboxes.len(),
        );
    }

    /// RotatingCross consumer spawns a single hitbox on the first
    /// strike tick (the horizontal arm) and a second on the toggle
    /// edge (when axis_period_s elapses).
    #[test]
    fn saddle_point_consumer_spawns_initial_arm_then_toggles_axes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<WorldTime>();
        let mut wt = app.world_mut().resource_mut::<WorldTime>();
        wt.scaled_dt = 0.5; // half-second per tick → toggle after 3 ticks at period=1.2
        wt.raw_dt = 0.5;
        app.add_systems(Update, spawn_saddle_point_from_special_messages);
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                SaddlePointState::default(),
                gradient_sentinel_boss_feature(),
            ))
            .id();
        let spec = SpecialActionSpec::RotatingCross {
            arm_length: 220.0,
            arm_thickness: 36.0,
            axis_period_s: 1.2,
            damage: 2,
        };

        // Tick 1: strike starts, horizontal arm spawned.
        write_special(&mut app, actor, spec);
        app.update();
        let state = app.world().entity(actor).get::<SaddlePointState>().unwrap();
        assert!(state.strike_active);
        assert!(state.axis_horizontal);
        assert!(state.horizontal_hitbox.is_some());
        assert!(state.vertical_hitbox.is_none());

        // Walk past the axis period — toggles to vertical.
        for _ in 0..4 {
            write_special(&mut app, actor, spec);
            app.update();
        }
        let state = app.world().entity(actor).get::<SaddlePointState>().unwrap();
        assert!(
            !state.axis_horizontal,
            "axis should have toggled to vertical after period elapsed",
        );
        assert!(state.vertical_hitbox.is_some());
        assert!(state.horizontal_hitbox.is_none());
    }

    /// When the strike ends (no Special message arrives), the
    /// RotatingCross consumer despawns its hitboxes + resets state so
    /// the next strike starts clean.
    #[test]
    fn saddle_point_consumer_despawns_on_strike_end() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<WorldTime>();
        let mut wt = app.world_mut().resource_mut::<WorldTime>();
        wt.scaled_dt = 0.1;
        wt.raw_dt = 0.1;
        app.add_systems(Update, spawn_saddle_point_from_special_messages);
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                SaddlePointState::default(),
                gradient_sentinel_boss_feature(),
            ))
            .id();
        let spec = SpecialActionSpec::RotatingCross {
            arm_length: 220.0,
            arm_thickness: 36.0,
            axis_period_s: 1.2,
            damage: 2,
        };

        // Start strike.
        write_special(&mut app, actor, spec);
        app.update();
        let state = app.world().entity(actor).get::<SaddlePointState>().unwrap();
        assert!(state.strike_active);

        // No message → strike closed.
        app.update();
        let state = app.world().entity(actor).get::<SaddlePointState>().unwrap();
        assert!(!state.strike_active);
        assert!(state.horizontal_hitbox.is_none());
        assert!(state.vertical_hitbox.is_none());
    }

    /// PitTrap with spawn_minion=true should attach an
    /// EncounterMob marker to the spawned puppy_slug so
    /// `spawn_dynamic_feature_visuals` (the per-frame visual
    /// discovery system) actually attaches a sprite next frame.
    /// Without this marker the minion would exist in ECS but never
    /// render.
    #[test]
    fn minima_trap_spawned_minion_carries_encounter_mob_marker() {
        use crate::features::components::EncounterMob;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<WorldTime>();
        let mut wt = app.world_mut().resource_mut::<WorldTime>();
        wt.scaled_dt = 1.0 / 60.0;
        wt.raw_dt = 1.0 / 60.0;
        app.add_systems(Update, spawn_minima_trap_from_special_messages);
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                MinimaTrapState::default(),
                gradient_sentinel_boss_feature(),
            ))
            .id();
        let pre_count = app
            .world_mut()
            .query::<&EncounterMob>()
            .iter(app.world())
            .count();
        write_special(
            &mut app,
            actor,
            SpecialActionSpec::PitTrap {
                hazard_duration_s: 5.0,
                damage: 2,
                half_extent_x: 56.0,
                half_extent_y: 24.0,
                spawn_minion: true, // <- spawn the minion this time
            },
        );
        app.update();
        let post_count = app
            .world_mut()
            .query::<&EncounterMob>()
            .iter(app.world())
            .count();
        assert_eq!(
            post_count - pre_count,
            1,
            "expected one new EncounterMob (the puppy_slug minion), got {}",
            post_count - pre_count,
        );
    }

    /// MinionCascade-spawned minions must also carry the
    /// EncounterMob marker for the same reason as PitTrap.
    /// Without it the cascade adds spawn but never render — the
    /// bug the user reported as "I don't see it spawning any
    /// lurker slop enemies."
    #[test]
    fn gradient_cascade_spawned_minions_carry_encounter_mob_marker() {
        use crate::features::components::EncounterMob;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<WorldTime>();
        let mut wt = app.world_mut().resource_mut::<WorldTime>();
        wt.scaled_dt = 1.0 / 60.0;
        wt.raw_dt = 1.0 / 60.0;
        app.add_systems(Update, spawn_gradient_cascade_minions_from_special_messages);
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                GradientCascadeState::default(),
                gradient_sentinel_boss_feature(),
            ))
            .id();
        let pre_count = app
            .world_mut()
            .query::<&EncounterMob>()
            .iter(app.world())
            .count();
        write_special(
            &mut app,
            actor,
            SpecialActionSpec::MinionCascade { minion_count: 3 },
        );
        app.update();
        let post_count = app
            .world_mut()
            .query::<&EncounterMob>()
            .iter(app.world())
            .count();
        assert_eq!(
            post_count - pre_count,
            3,
            "expected three new EncounterMob entities, got {}",
            post_count - pre_count,
        );
    }

    /// MinionCascade consumer spawns minion entities at strike
    /// start. Mocks the runtime spawn machinery by checking the
    /// Bevy entity count goes up by `minion_count`.
    #[test]
    fn gradient_cascade_consumer_spawns_minion_entities_on_strike_edge() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ActorActionMessage>();
        app.init_resource::<WorldTime>();
        let mut wt = app.world_mut().resource_mut::<WorldTime>();
        wt.scaled_dt = 1.0 / 60.0;
        wt.raw_dt = 1.0 / 60.0;
        app.add_systems(Update, spawn_gradient_cascade_minions_from_special_messages);

        // Count actors pre-spawn.
        let actor = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                GradientCascadeState::default(),
                gradient_sentinel_boss_feature(),
            ))
            .id();
        let pre_count = app
            .world_mut()
            .query::<&ActorRuntime>()
            .iter(app.world())
            .count();

        write_special(
            &mut app,
            actor,
            SpecialActionSpec::MinionCascade { minion_count: 3 },
        );
        app.update();

        let post_count = app
            .world_mut()
            .query::<&ActorRuntime>()
            .iter(app.world())
            .count();
        assert_eq!(
            post_count - pre_count,
            3,
            "expected 3 new minion ActorRuntime entities, got {}",
            post_count - pre_count,
        );
        let state = app
            .world()
            .entity(actor)
            .get::<GradientCascadeState>()
            .unwrap();
        assert!(state.fired_this_strike);
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
        assert!(
            enemy_projectile_bodies(&mut app).is_empty(),
            "non-Special message must not trigger the apple consumer",
        );
    }
}
