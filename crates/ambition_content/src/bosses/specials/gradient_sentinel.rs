//! Gradient Sentinel kit (apple rain, overfit volley, minima trap, saddle point, gradient cascade) boss-special Technique.
//!
//! Split out of the former 1.8k-line `specials.rs` (2026-06-15) — see
//! [`super`] (`specials/mod.rs`) for the shared module overview.

use super::*;

// ===================================================================
// Migrated boss-special Techniques (from ambition_gameplay_core brain_effects).
// Each owns its key + per-boss state + params + behavior; the engine
// names none of them. Tuning consts (APPLE_RAIN_*, OVERFIT_VOLLEY_*, …)
// still live in ambition_gameplay_core::features::bosses for now (just numbers).
// ===================================================================

const APPLE_RAIN_KEY: &str = "apple_rain";
const OVERFIT_VOLLEY_KEY: &str = "overfit_volley";
const MINIMA_TRAP_KEY: &str = "minima_trap";
const SADDLE_POINT_KEY: &str = "saddle_point";
const GRADIENT_CASCADE_KEY: &str = "gradient_cascade";

// Boss-special tuning — content-owned (moved off the engine lib with the
// techniques; the engine's `features::bosses` no longer holds boss-special
// numbers). Values are tuned for the gradient-sentinel arena.
const APPLE_RAIN_INTERVAL: f32 = 0.35;
const APPLE_RAIN_SPAWN_SPEED: f32 = 35.0;
const APPLE_RAIN_DAMAGE: i32 = 1;
const OVERFIT_VOLLEY_SAMPLE_INTERVAL_S: f32 = 0.30;
const OVERFIT_VOLLEY_SAMPLE_COUNT: u8 = 5;
const OVERFIT_VOLLEY_SHOT_SPEED: f32 = 360.0;
const OVERFIT_VOLLEY_SHOT_DAMAGE: i32 = 1;
const MINIMA_TRAP_HAZARD_DURATION_S: f32 = 5.0;
const MINIMA_TRAP_DAMAGE: i32 = 2;
const MINIMA_TRAP_HALF_EXTENT_X: f32 = 56.0;
const MINIMA_TRAP_HALF_EXTENT_Y: f32 = 24.0;
const SADDLE_POINT_ARM_LENGTH: f32 = 220.0;
const SADDLE_POINT_ARM_THICKNESS: f32 = 36.0;
const SADDLE_POINT_AXIS_PERIOD_S: f32 = 1.2;
const SADDLE_POINT_DAMAGE: i32 = 2;
const GRADIENT_CASCADE_MINION_COUNT: u8 = 2;

/// Per-boss apple-rain accumulator: state moved out of `BossRuntime`
/// to keep the runtime focused on body/HP and to let the EFFECTS
/// consumer (`spawn_gnu_apple_rain_from_special_messages`) own the
/// per-tick spawn cadence. Defaulted-attached to every boss; only
/// the gnu_ton encounter advances it (its ActionSet's `special` is
/// `SpecialActionSpec::Special("apple_rain")`, so only it generates the
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
/// `ActorActionMessage::Special { spec: SpecialActionSpec::Special("apple_rain") }`.
/// The boss runtime tags `frame.special_pressed = true` every tick
/// its `BossAttackProfile::Special("apple_rain")` strike window is active;
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
    world: Res<ambition_engine_core::RoomGeometry>,
    mut messages: MessageReader<ActorActionMessage>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut bosses: Query<
        (
            Entity,
            &mut AppleRainSpawnState,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    // Apple-rain tuning is content-owned (lib consts for now; move with the
    // technique). The brain fires one `Special("apple_rain")` message per tick
    // the strike window is active.
    let (interval_s, spawn_speed, damage) = (
        APPLE_RAIN_INTERVAL,
        APPLE_RAIN_SPAWN_SPEED,
        APPLE_RAIN_DAMAGE,
    );
    // Bosses with an `apple_rain` Special this tick. Multiple messages from one
    // boss collapse to the same entry — "any message this tick" = "strike
    // window active this tick".
    let mut firing: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
            ..
        } = &msg.request
        {
            if key == APPLE_RAIN_KEY {
                firing.insert(msg.actor);
            }
        }
    }

    for (entity, mut state, boss_feature, health) in &mut bosses {
        if !firing.contains(&entity) {
            // No message this tick → reset accumulator so a future
            // strike window starts on a clean beat.
            state.spawn_accum = 0.0;
            continue;
        };
        let boss = boss_feature.as_boss_ref();
        if !health.alive() || interval_s <= 0.0 {
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
            effects.write(ambition_vfx::EffectRequest {
                owner: entity,
                effect: ambition_vfx::Effect::Projectiles {
                    shots: vec![EnemyProjectileSpawn {
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
                            ambition_gameplay_core::features::bosses::GNU_TON_APPLE_OWNER_PREFIX,
                            boss.config.id,
                        ),
                        gravity: APPLE_RAIN_GRAVITY,
                        // The apple-rain fruit renders as the generated apple
                        // sprite (kept upright vs gravity) — keyed by kind, not
                        // by the owner-id substring the visuals layer once read.
                        visual_tag: ambition_gameplay_core::projectile::ProjectileVisualKind::Apple
                            .to_tag(),
                    }],
                },
            });
            state.spawn_index = state.spawn_index.wrapping_add(1);
        }
    }
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
// `boss_special_for_profile` (see `ambition_gameplay_core::features::bosses`).

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
/// `ambition_gameplay_core::features::bosses` — these are local mirrors.
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
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    // Per-actor target: each boss carries an `ActorTarget` populated
    // upstream by `select_actor_targets` (nearest-player resolution).
    // Reading the target's player kinematics by Entity makes this
    // system multi-player ready — single-player behavior is preserved
    // because there's only one player today.
    player_query: Query<
        &ambition_gameplay_core::actor::BodyKinematics,
        With<ambition_gameplay_core::actor::PlayerEntity>,
    >,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &BossAttackState,
            &mut OverfitVolleyState,
            Option<&ambition_gameplay_core::features::ActorTarget>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();

    let mut firing: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
            ..
        } = &msg.request
        {
            if key == OVERFIT_VOLLEY_KEY {
                firing.insert(msg.actor);
            }
        }
    }

    for (entity, boss_feature, health, attack_state, mut state, actor_target) in &mut bosses {
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
        if !health.alive() {
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
            Some(BossAttackProfile::Special(ref k)) if k == OVERFIT_VOLLEY_KEY
        );

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
        } else if firing.contains(&entity) {
            let (shot_speed, damage) = (OVERFIT_VOLLEY_SHOT_SPEED, OVERFIT_VOLLEY_SHOT_DAMAGE);
            if !state.fired_this_strike {
                let origin = boss.kin.pos + boss.config.behavior.projectile_origin_offset;
                for sample_pos in state.samples.iter() {
                    let delta = *sample_pos - origin;
                    let dir = delta.normalize_or_zero();
                    if dir.length_squared() < 1e-4 {
                        continue;
                    }
                    effects.write(ambition_vfx::EffectRequest {
                        owner: entity,
                        effect: ambition_vfx::Effect::Projectiles {
                            shots: vec![EnemyProjectileSpawn {
                                origin,
                                dir,
                                speed: shot_speed,
                                damage,
                                max_lifetime: OVERFIT_VOLLEY_BOLT_LIFETIME,
                                half_extent: OVERFIT_VOLLEY_BOLT_HALF_EXTENT,
                                owner_id: format!(
                                    "{}:{}",
                                    OVERFIT_VOLLEY_OWNER_PREFIX, boss.config.id
                                ),
                                gravity: 0.0,
                                visual_tag: 0,
                            }],
                        },
                    });
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
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    // Per-boss target via `ActorTarget` (populated by
    // `select_actor_targets`); same multi-player-ready pattern as
    // the overfit-volley consumer above.
    player_query: Query<
        &ambition_gameplay_core::actor::BodyKinematics,
        With<ambition_gameplay_core::actor::PlayerEntity>,
    >,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &mut MinimaTrapState,
            Option<&ambition_gameplay_core::features::ActorTarget>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let mut firing: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
            ..
        } = &msg.request
        {
            if key == MINIMA_TRAP_KEY {
                firing.insert(msg.actor);
            }
        }
    }

    for (entity, boss_feature, health, mut state, actor_target) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        let player_pos = actor_target.and_then(|t| {
            t.entity
                .and_then(|e| player_query.get(e).ok())
                .map(|kin| kin.aabb().center())
                .or(Some(t.pos))
        });
        if !firing.contains(&entity) {
            // Strike window closed — reset the fired gate so the next
            // strike re-spawns the pit.
            state.fired_this_strike = false;
            continue;
        };
        if !health.alive() {
            continue;
        }
        if state.fired_this_strike {
            continue;
        }
        let (hazard_duration_s, damage, hx, hy, spawn_minion) = (
            MINIMA_TRAP_HAZARD_DURATION_S,
            MINIMA_TRAP_DAMAGE,
            MINIMA_TRAP_HALF_EXTENT_X,
            MINIMA_TRAP_HALF_EXTENT_Y,
            true,
        );
        let pit_center = player_pos.unwrap_or(boss.kin.pos);

        effects.write(ambition_vfx::EffectRequest {
            owner: entity,
            effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                center: pit_center,
                faction: ActorFaction::Boss,
                half_extent: ae::Vec2::new(hx, hy),
                damage,
                knockback: MINIMA_TRAP_KNOCKBACK,
                lifetime_s: hazard_duration_s.max(0.05),
                name: None,
            }),
        });

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
            effects.write(ambition_vfx::EffectRequest {
                owner: entity,
                effect: ambition_vfx::Effect::Summon(ambition_vfx::SummonSpec {
                    id: minion_id,
                    name: "Puppy Slug".to_string(),
                    pos: minion_pos,
                    half_size: MINIMA_TRAP_MINION_HALF_SIZE,
                    archetype_id: MINIMA_TRAP_MINION_ARCHETYPE.to_string(),
                    encounter_id,
                    faction: ambition_gameplay_core::features::ActorFaction::Enemy,
                }),
            });
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
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &mut SaddlePointState,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();

    let mut firing: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
            ..
        } = &msg.request
        {
            if key == SADDLE_POINT_KEY {
                firing.insert(msg.actor);
            }
        }
    }

    for (entity, boss_feature, health, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains(&entity) {
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
        if !health.alive() {
            if let Some(h) = state.horizontal_hitbox.take() {
                commands.entity(h).despawn();
            }
            if let Some(h) = state.vertical_hitbox.take() {
                commands.entity(h).despawn();
            }
            continue;
        }
        let (arm_length, arm_thickness, axis_period_s, damage) = (
            SADDLE_POINT_ARM_LENGTH,
            SADDLE_POINT_ARM_THICKNESS,
            SADDLE_POINT_AXIS_PERIOD_S,
            SADDLE_POINT_DAMAGE,
        );
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
            //
            // This one calls the executor DIRECTLY (not via `Effect::DamageBox`)
            // on purpose: the rotating cross tracks each arm's `Entity` to
            // despawn it on toggle, and the fire-and-forget `EffectRequest` seam
            // can't hand the spawned entity back. Effects you need a handle to
            // use the spawn helper directly; fire-and-forget ones emit a request.
            ambition_vfx::spawn_damage_box(
                commands,
                entity,
                ActorFaction::Boss,
                boss.kin.pos,
                ambition_vfx::DamageBox {
                    half_extent: ae::Vec2::new(he_x, he_y),
                    shape: None,
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
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &mut GradientCascadeState,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let minion_count = GRADIENT_CASCADE_MINION_COUNT;
    let mut firing: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    for msg in messages.read() {
        if let ActionRequest::Special {
            spec: SpecialActionSpec::Special(key),
            ..
        } = &msg.request
        {
            if key == GRADIENT_CASCADE_KEY {
                firing.insert(msg.actor);
            }
        }
    }

    for (entity, boss_feature, health, mut state) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        if !firing.contains(&entity) {
            // Strike closed — reset gate.
            state.fired_this_strike = false;
            continue;
        };
        if !health.alive() {
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
            effects.write(ambition_vfx::EffectRequest {
                owner: entity,
                effect: ambition_vfx::Effect::Summon(ambition_vfx::SummonSpec {
                    id: minion_id,
                    name: "Slop Lurker".to_string(),
                    pos: spawn_pos,
                    half_size: GRADIENT_CASCADE_MINION_HALF_SIZE,
                    archetype_id: GRADIENT_CASCADE_MINION_ARCHETYPE.to_string(),
                    encounter_id: encounter_id.clone(),
                    faction: ambition_gameplay_core::features::ActorFaction::Enemy,
                }),
            });
        }
        state.fired_this_strike = true;
        state.spawn_index = state.spawn_index.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

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
            assert!(
                x >= margin - 1e-3 && x <= max_x + 1e-3,
                "apple {i} x={x} outside [{margin}, {max_x}]"
            );
            assert!(
                x <= self_left + 1e-3 || x >= self_right - 1e-3,
                "apple {i} x={x} inside boss keep-out ({self_left}..{self_right})"
            );
            xs.push(x);
        }
        let mid = world_width / 2.0;
        assert!(
            xs.iter().any(|&x| x < mid) && xs.iter().any(|&x| x > mid),
            "apple spawns should spread across both halves: {xs:?}"
        );
    }
}
