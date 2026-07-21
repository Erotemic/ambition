//! Overflow boundary flood boss-special Technique.
//!
//! Split out of the former 1.8k-line `specials.rs` (2026-06-15) — see
//! [`super`] (`specials/mod.rs`) for the shared module overview.

use bevy::prelude::*;

use ambition_actors::actor::{BodyKinematics, PlayerEntity};
use ambition_actors::features::{ActorTarget, BossClusterRef, FeatureSimEntity};
use ambition_characters::brain::{
    action_set::ActionRequest, ActorActionMessage, BossAttackProfile, BossAttackState,
    SpecialActionSpec,
};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_projectiles::enemy::EnemyProjectileSpawn;
use ambition_vfx::{Effect, EffectRequest};

// ---- Overflow's boundary flood (content-only, open-seam special) ----

/// Content key for Overflow's flood — matches the `Special("overflow_flood")`
/// beats in `boss_profiles.ron`.
pub const OVERFLOW_FLOOD_KEY: &str = "overflow_flood";

const FLOOD_SPACING: f32 = 60.0;
const FLOOD_GAP_HALF: f32 = 78.0;
const FLOOD_MARGIN: f32 = 40.0;
const FLOOD_SPEED: f32 = 60.0;
const FLOOD_GRAVITY: f32 = 520.0;
const FLOOD_DAMAGE: i32 = 1;
const FLOOD_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(12.0, 14.0);
const FLOOD_LIFETIME: f32 = 6.0;
const FLOOD_SPAWN_HEIGHT_ABOVE_BOSS: f32 = 300.0;
const FLOOD_OWNER_PREFIX: &str = "overflow_flood";

/// Per-boss state for the Overflow flood. The telegraph locks the player's x
/// (their safe lane); the strike floods every other column from above. One burst
/// per strike.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct OverflowState {
    pub locked_x: Option<f32>,
    pub fired_this_strike: bool,
}

/// Pure: the world-x columns of the flood — evenly spaced across the playable
/// width at `spacing`, **skipping** any column within `gap_half` of `gap_x` (the
/// one un-overflowed lane the player must hold). Deterministic — the testable
/// core of the Technique.
fn overflow_columns(world_width: f32, spacing: f32, gap_x: f32, gap_half: f32) -> Vec<f32> {
    let spacing = spacing.max(8.0);
    let min_x = FLOOD_MARGIN;
    let max_x = (world_width - FLOOD_MARGIN).max(min_x);
    let mut out = Vec::new();
    let mut x = min_x;
    while x <= max_x {
        if (x - gap_x).abs() > gap_half {
            out.push(x);
        }
        x += spacing;
    }
    out
}

/// Technique: Overflow boundary flood (content-only; open-seam special).
pub fn spawn_overflow_flood_from_special_messages(
    world: ambition::platformer::lifecycle::SessionWorldRef<ambition_engine_core::RoomGeometry>,
    mut effects: MessageWriter<EffectRequest>,
    mut messages: MessageReader<ActorActionMessage>,
    player_query: Query<&BodyKinematics, With<PlayerEntity>>,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &BossAttackState,
            &mut OverflowState,
            Option<&ActorTarget>,
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
            if key == OVERFLOW_FLOOD_KEY {
                firing.insert(msg.actor);
            }
        }
    }
    for (entity, boss_feature, health, attack_state, mut state, actor_target) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        let player_x = actor_target.and_then(|t| {
            t.entity
                .and_then(|e| player_query.get(e).ok())
                .map(|kin| kin.aabb().center().x)
                .or(Some(t.pos.x))
        });
        if !health.alive() {
            state.locked_x = None;
            state.fired_this_strike = false;
            continue;
        }
        let in_telegraph = matches!(
            attack_state.telegraph_profile,
            Some(BossAttackProfile::Special(ref k)) if k == OVERFLOW_FLOOD_KEY
        );
        if in_telegraph {
            if state.locked_x.is_none() {
                state.locked_x = player_x;
            }
            state.fired_this_strike = false;
            continue;
        }
        if !firing.contains(&entity) {
            state.locked_x = None;
            state.fired_this_strike = false;
            continue;
        }
        if state.fired_this_strike {
            continue;
        }
        let gap_x = state.locked_x.or(player_x).unwrap_or(boss.kin.pos.x);
        let spawn_y =
            (boss.kin.pos.y - FLOOD_SPAWN_HEIGHT_ABOVE_BOSS).max(FLOOD_HALF_EXTENT.y + 8.0);
        for x in overflow_columns(world.0.size.x, FLOOD_SPACING, gap_x, FLOOD_GAP_HALF) {
            effects.write(EffectRequest {
                owner: entity,
                effect: Effect::Projectiles {
                    shots: vec![EnemyProjectileSpawn {
                        origin: ae::Vec2::new(x, spawn_y),
                        dir: ae::Vec2::new(0.0, 1.0),
                        speed: FLOOD_SPEED,
                        damage: FLOOD_DAMAGE,
                        max_lifetime: FLOOD_LIFETIME,
                        half_extent: FLOOD_HALF_EXTENT,
                        owner_id: format!("{}:{}", FLOOD_OWNER_PREFIX, boss.config.id),
                        gravity: FLOOD_GRAVITY,
                        visual_id: String::new(),
                        // Straight shot: this ability authors no bounce.
                        bounces: 0,
                        bounce_on_world_contact: false,
                    }],
                },
            });
        }
        state.fired_this_strike = true;
        state.locked_x = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_flood_leaves_exactly_one_safe_lane() {
        let world_width = 1792.0;
        let gap_x = 900.0;
        let gap_half = 78.0;
        let cols = overflow_columns(world_width, 60.0, gap_x, gap_half);
        assert!(!cols.is_empty(), "the flood has columns");
        // No column falls inside the safe lane...
        assert!(
            cols.iter().all(|&x| (x - gap_x).abs() > gap_half),
            "no flood column inside the safe lane",
        );
        // ...but columns exist on BOTH sides of it (you're boxed into the lane).
        assert!(cols.iter().any(|&x| x < gap_x - gap_half));
        assert!(cols.iter().any(|&x| x > gap_x + gap_half));
        // All columns stay within the playable margins.
        assert!(cols.iter().all(|&x| x >= FLOOD_MARGIN - 1e-3));
        assert!(cols.iter().all(|&x| x <= world_width - FLOOD_MARGIN + 1e-3));
    }
}
