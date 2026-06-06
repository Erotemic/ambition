//! Combat-phase schedule plugin.
//!
//! EFFECTS-stage brain-action consumers (enemy melee/ranged spawns, boss
//! special-attack spawns), projectile + hitbox + feature-hit resolution,
//! the cut-rope boss-arena tick, and mount/rider link bookkeeping all run
//! here in `SandboxSet::Combat`.
//!
//! Extracted from `app/plugins.rs` (ecs-cleanup-plan #8) so the top-level
//! simulation orchestration reads as a list of named domain plugins rather
//! than one monolithic scheduler.

use bevy::prelude::*;

use super::schedule::SandboxSet;
use super::sim_systems::attack_advance_system;
use crate::runtime::game_mode::gameplay_allowed;

/// Schedules the `SandboxSet::Combat` system chain.
pub struct CombatSchedulePlugin;

impl Plugin for CombatSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                attack_advance_system.run_if(gameplay_allowed),
                // EFFECTS-stage consumer: reads ActorActionMessage::Ranged
                // emitted upstream by `emit_brain_action_messages`
                // (PlayerInput set) and spawns enemy projectiles. Runs
                // BEFORE `update_enemy_projectiles` so projectiles spawned
                // this tick already advance one step this frame, matching
                // the pre-migration latency.
                crate::features::spawn_enemy_projectiles_from_brain_actions
                    .run_if(gameplay_allowed),
                // EFFECTS-stage consumer: reads ActorActionMessage::Melee
                // and starts the enemy's attack windup + cooldown.
                // Replaces the legacy `if frame.melee_pressed` gate inside
                // `EnemyRuntime::update`. The active-edge `Hitbox` spawn
                // happens upstream in `update_ecs_actors` (the runtime is
                // the only place that owns the windup → active transition);
                // `apply_hitbox_damage` below resolves the overlap.
                crate::features::start_enemy_melee_from_brain_actions.run_if(gameplay_allowed),
                // EFFECTS-stage consumer: reads
                // `ActorActionMessage::Special { SpecialActionSpec::GnuAppleRain }`
                // and accumulates per-boss apple-rain spawn cadence.
                // Replaces `BossRuntime::tick_apple_rain` (Task B of the
                // actor/brain follow-up plan). Runs BEFORE
                // `update_enemy_projectiles` so apples spawned this tick
                // advance one step this frame, matching the legacy
                // ordering of `outputs.projectile_spawns` flush →
                // projectile tick.
                crate::features::spawn_gnu_apple_rain_from_special_messages
                    .run_if(gameplay_allowed),
                // Gradient Sentinel special consumers (one per
                // SpecialActionSpec variant): position-sampling bolt
                // barrage, pit + puppy_slug spawn, rotating cross hazard,
                // and slop-minion descent. All four are written directly
                // by `tick_boss_brains_system` via `boss_special_for_profile`
                // and follow the apple-rain consumer pattern (per-boss
                // state component, reset on no-message tick). Run before
                // `update_enemy_projectiles` for the bolt barrage so it
                // advances this frame.
                crate::features::spawn_overfit_volley_from_special_messages
                    .run_if(gameplay_allowed),
                crate::features::spawn_eye_beam_from_special_messages.run_if(gameplay_allowed),
                crate::features::spawn_minima_trap_from_special_messages.run_if(gameplay_allowed),
                crate::features::spawn_saddle_point_from_special_messages.run_if(gameplay_allowed),
                crate::features::spawn_gradient_cascade_minions_from_special_messages
                    .run_if(gameplay_allowed),
                // Actor-generic consumer: a ShockwaveSlam Special (boss OR
                // player-emitted) spawns a faction-tagged World-anchored AOE
                // hitbox at the emitter, resolved by `apply_hitbox_damage`
                // below. The player end (the shockwave gauntlet) is the first
                // "player wields a boss attack" slice.
                crate::shockwave::spawn_shockwave_from_special_messages.run_if(gameplay_allowed),
                crate::projectile::update_projectiles,
                crate::enemy_projectile::update_enemy_projectiles.run_if(gameplay_allowed),
                // Hitbox-entity lifecycle for melee strikes (Task A of the
                // actor/brain follow-up plan). `apply_hitbox_damage`
                // resolves overlap → damage event; `tick_and_despawn_hitboxes`
                // advances lifetimes and cleans expired entities.
                crate::features::apply_hitbox_damage.run_if(gameplay_allowed),
                crate::features::tick_and_despawn_hitboxes,
                crate::features::apply_feature_hit_events,
                crate::ambition_content::bosses::tick_cut_rope_boss_arena.run_if(gameplay_allowed),
                crate::ambition_content::bosses::sync_cut_rope_boss_arena_prop_visuals,
                // Mount/rider link bookkeeping. Runs after damage so
                // it observes the alive flag transition for either
                // side; a dead mount releases its rider (gravity on,
                // solo brain restored) and a dead rider clears the
                // mount's MountSlot back-reference.
                crate::features::enforce_mount_rider_link,
            )
                .chain()
                .in_set(SandboxSet::Combat),
        );
    }
}
