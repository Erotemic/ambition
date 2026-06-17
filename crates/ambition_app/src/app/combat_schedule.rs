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

use super::sim_systems::attack_advance_system;
use ambition_gameplay_core::schedule::SandboxSet;
use ambition_gameplay_core::runtime::game_mode::gameplay_allowed;

/// Schedules the `SandboxSet::Combat` system chain.
pub struct CombatSchedulePlugin;

impl Plugin for CombatSchedulePlugin {
    fn build(&self, app: &mut App) {
        // The effect seam: techniques (the shockwave gauntlet, the boss
        // phase-transition slam, …) emit `EffectRequest`; `apply_effects` below
        // drains it. Registered here so the writers never hit an unregistered
        // message.
        app.add_message::<ambition_gameplay_core::effects::EffectRequest>();
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
                ambition_gameplay_core::features::spawn_enemy_projectiles_from_brain_actions
                    .run_if(gameplay_allowed),
                // EFFECTS-stage consumer: reads ActorActionMessage::Melee
                // and starts the enemy's attack windup + cooldown.
                // Replaces the legacy `if frame.melee_pressed` gate inside
                // `EnemyRuntime::update`. The active-edge `Hitbox` spawn
                // happens upstream in `update_ecs_actors` (the runtime is
                // the only place that owns the windup → active transition);
                // `apply_hitbox_damage` below resolves the overlap.
                ambition_gameplay_core::features::start_enemy_melee_from_brain_actions.run_if(gameplay_allowed),
                // EFFECTS-stage consumer: reads
                // `ActorActionMessage::Special { SpecialActionSpec::Special("apple_rain") }`
                // and accumulates per-boss apple-rain spawn cadence.
                // Replaces `BossRuntime::tick_apple_rain` (Task B of the
                // actor/brain follow-up plan). Runs BEFORE
                // `update_enemy_projectiles` so apples spawned this tick
                // advance one step this frame, matching the legacy
                // ordering of `outputs.projectile_spawns` flush →
                // projectile tick.
                ambition_content::bosses::specials::spawn_gnu_apple_rain_from_special_messages
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
                ambition_content::bosses::specials::spawn_overfit_volley_from_special_messages
                    .run_if(gameplay_allowed),
                // Content boss specials nested as one chain element (keeps the
                // outer tuple under Bevy's 20-system limit). Independent of each
                // other; both just need to run before the projectile slot below.
                (
                    ambition_content::bosses::specials::spawn_eye_beam_from_special_messages,
                    ambition_content::bosses::specials::spawn_mode_collapse_converge_from_special_messages,
                    ambition_content::bosses::specials::spawn_gradient_nova_from_special_messages,
                    ambition_content::bosses::specials::spawn_overflow_flood_from_special_messages,
                    ambition_content::bosses::specials::spawn_seismic_stomp_from_special_messages,
                    ambition_content::bosses::specials::spawn_echo_fan_from_special_messages,
                )
                    .run_if(gameplay_allowed),
                ambition_content::bosses::specials::spawn_minima_trap_from_special_messages.run_if(gameplay_allowed),
                ambition_content::bosses::specials::spawn_saddle_point_from_special_messages.run_if(gameplay_allowed),
                ambition_content::bosses::specials::spawn_gradient_cascade_minions_from_special_messages
                    .run_if(gameplay_allowed),
                // Generic effect executor: drains `EffectRequest` (boss OR
                // player emitted) and makes each effect happen — currently the
                // `DamageBox` AOE (shockwave gauntlet + boss phase-transition
                // slam), faction-tagged at the emitter, resolved by
                // `apply_hitbox_damage` below. Runs at the position the bespoke
                // shockwave consumer used, so spawn timing is unchanged.
                // Box + Summon executors, nested into one chained group (keeps
                // the outer tuple within Bevy's 20-system limit). Summon stays
                // lib-side (the enemy roster) so `apply_effects` is substrate-free;
                // same slot as before, so minion spawn timing is unchanged.
                (
                    ambition_gameplay_core::effects::apply_effects.run_if(gameplay_allowed),
                    ambition_gameplay_core::features::apply_summon_effects.run_if(gameplay_allowed),
                )
                    .chain(),
                // Phase 3b enemy-pool spawn consumer: drains SpawnProjectile
                // messages emitted by the EFFECTS-stage fire consumers above
                // (apple rain / overfit volley / eye beam / ranged bolts /
                // sentry / meteor / volley) into EnemyProjectileState.bodies
                // BEFORE the step below, so a body spawned this tick advances
                // one step this frame — identical to the old direct push.
                ambition_gameplay_core::enemy_projectile::apply_projectile_effects
                    .run_if(gameplay_allowed),
                // Unified projectile step (player + enemy, faction-routed). Runs
                // AFTER the enemy spawn consumer (so an enemy body spawned this
                // tick advances one step this frame) and BEFORE the player input +
                // spawn below (so a player shot FIRED this frame first ticks next
                // frame — the old asymmetric spawn timing, preserved).
                ambition_gameplay_core::projectile::step_projectiles.run_if(gameplay_allowed),
                // Player projectile INPUT: charge / Hadouken / fire → SpawnProjectile.
                ambition_gameplay_core::projectile::player_projectile_input,
                // Phase 3b player-pool spawn consumer: materializes player-fired
                // bodies AFTER the step, so the new body first ticks next frame.
                ambition_gameplay_core::projectile::apply_player_spawn_projectile_messages,
                // Hitbox-entity lifecycle for melee strikes (Task A of the
                // actor/brain follow-up plan). `apply_hitbox_damage`
                // resolves overlap → damage event; `tick_and_despawn_hitboxes`
                // advances lifetimes and cleans expired entities.
                ambition_gameplay_core::features::apply_hitbox_damage.run_if(gameplay_allowed),
                ambition_gameplay_core::features::tick_and_despawn_hitboxes,
                ambition_gameplay_core::features::apply_feature_hit_events,
                ambition_content::bosses::tick_cut_rope_boss_arena.run_if(gameplay_allowed),
                ambition_content::bosses::sync_cut_rope_boss_arena_prop_visuals,
                // Mount/rider link bookkeeping. Runs after damage so
                // it observes the alive flag transition for either
                // side; a dead mount releases its rider (gravity on,
                // solo brain restored) and a dead rider clears the
                // mount's MountSlot back-reference.
                ambition_gameplay_core::features::enforce_mount_rider_link,
            )
                .chain()
                .in_set(SandboxSet::Combat),
        );
    }
}
