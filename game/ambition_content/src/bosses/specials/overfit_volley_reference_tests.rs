//! The NATIVE overfit volley, kept as the reference trace of its procedural
//! module (`ambition_content_modules::overfit_volley`). Test-only: the game
//! runs the module. `module_parity_tests` holds the module to this system.

use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_characters::brain::{
    ActorActionMessage, BossAttackProfile, BossAttackState,
};
use ambition_platformer2d::actor::FeatureSimEntity;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};
use ambition_time::WorldTime;

const OVERFIT_VOLLEY_KEY: &str = "overfit_volley";
const OVERFIT_VOLLEY_SAMPLE_INTERVAL_S: f32 = 0.30;
const OVERFIT_VOLLEY_SAMPLE_COUNT: u8 = 5;
const OVERFIT_VOLLEY_SHOT_SPEED: f32 = 360.0;
const OVERFIT_VOLLEY_SHOT_DAMAGE: i32 = 1;

/// Per-boss state for the `overfit_volley` technique. Sampled positions
/// are memorized during the telegraph window; the strike edge fires one
/// bolt at every sample.
#[derive(Component, Clone, Debug, Default)]
pub struct OverfitVolleyState {
    /// Player positions sampled during the active telegraph.
    pub samples: Vec<ae::Vec2>,
    /// Seconds since the last sample. Drains when `>= sample_interval_s`.
    pub sample_accum: f32,
    /// Tracks the per-strike "have we fired yet?" gate. Reset when
    /// the strike window closes (telegraph or active drops the
    /// `overfit_volley` profile).
    pub fired_this_strike: bool,
    /// Tracks the previous tick's "in-attack" status so the seed
    /// sample only happens once per telegraph (not every tick the
    /// state machine reports telegraph_profile).
    pub had_seed_sample: bool,
}

const OVERFIT_VOLLEY_BOLT_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(8.0, 8.0);
const OVERFIT_VOLLEY_BOLT_LIFETIME: f32 = 2.4;

/// EFFECTS consumer: `overfit_volley` position-sampling bolt barrage.
///
/// Reads two things per tick:
///
/// 1. `BossAttackState.telegraph_profile` — when set to the
///    `overfit_volley` profile, the consumer samples the player's position at
///    every `OVERFIT_VOLLEY_SAMPLE_INTERVAL_S` and pushes onto
///    `OverfitVolleyState.samples` (capped at `OVERFIT_VOLLEY_SAMPLE_COUNT`).
/// 2. `ActorActionMessage::Special { spec: Special("overfit_volley") }` —
///    arrives every tick the strike is active; the consumer fires
///    one bolt per memorized sample on the first such message
///    (gated by `fired_this_strike`).
///
/// When neither telegraph nor strike is active for this profile, the
/// state resets to a clean slate so the next strike window starts
/// from zero.
pub fn spawn_overfit_volley_from_special_messages(
    world_time: Res<WorldTime>,
    mut projectiles: MessageWriter<ProjectileSpawnRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    // Per-actor target: each boss carries an `ActorTarget` populated
    // upstream by `select_actor_targets` (nearest-player resolution).
    // Reading the target's player kinematics by Entity makes this
    // system multi-player ready — single-player behavior is preserved
    // because there's only one player today.
    player_query: Query<
        &ambition_platformer2d_core::BodyKinematics,
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &BossAttackState,
            &mut OverfitVolleyState,
            Option<&ambition_combat::components::ActorTarget>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();

    let firing = super::actors_firing(&mut messages, OVERFIT_VOLLEY_KEY);

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
        } else if firing.contains_key(&entity) {
            let (shot_speed, damage) = (OVERFIT_VOLLEY_SHOT_SPEED, OVERFIT_VOLLEY_SHOT_DAMAGE);
            if !state.fired_this_strike {
                let origin = boss.kin.pos + boss.config.behavior.projectile_origin_offset;
                for sample_pos in state.samples.iter() {
                    let delta = *sample_pos - origin;
                    let dir = delta.normalize_or_zero();
                    if dir.length_squared() < 1e-4 {
                        continue;
                    }
                    projectiles.write(ProjectileSpawnRequest::open(
                        entity,
                        ProjectileSpawn {
                            origin,
                            dir,
                            speed: shot_speed,
                            damage,
                            max_lifetime: OVERFIT_VOLLEY_BOLT_LIFETIME,
                            half_extent: OVERFIT_VOLLEY_BOLT_HALF_EXTENT,
                            gravity: 0.0,
                            visual_id: String::new(),
                            // Straight shot: this ability authors no bounce.
                            bounces: 0,
                            bounce_on_world_contact: false,
                            splash_half_extent: 0.0,
                            boomerang_return_s: None,
                        },
                        ProjectileStart::StepThisTick,
                    )
                    .fired_by_move_if_any(firing.get(&entity).copied().flatten()),
                    );
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
