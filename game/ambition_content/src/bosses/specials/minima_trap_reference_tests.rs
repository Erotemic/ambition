//! The NATIVE minima trap, kept as the reference trace of its procedural
//! module (`ambition_content_modules::minima_trap`). Test-only: the game runs
//! the module. `module_parity_tests` holds the module to this system.

use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_characters::brain::ActorActionMessage;
use ambition_platformer2d::actor::FeatureSimEntity;
use ambition_platformer2d_core::{self as ae, AabbExt};

const MINIMA_TRAP_KEY: &str = "minima_trap";
const MINIMA_TRAP_HAZARD_DURATION_S: f32 = 5.0;
const MINIMA_TRAP_DAMAGE: i32 = 2;
const MINIMA_TRAP_HALF_EXTENT_X: f32 = 56.0;
const MINIMA_TRAP_HALF_EXTENT_Y: f32 = 24.0;

/// `minima_trap` is a one-shot per-strike action (spawn pit hitbox +
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

const MINIMA_TRAP_OWNER_PREFIX: &str = "gradient_sentinel_minima";
const MINIMA_TRAP_KNOCKBACK: f32 = 1.4;
/// The character the trap summons: the pacifist crawler.
///
/// The summon road resolves the prepared cast first and builds from the
/// definition. An id that resolves nothing falls back to the generic
/// `combatant` body silently, so this must name a real character.
const MINIMA_TRAP_MINION_CHARACTER: &str = "npc_puppy_slug";
const MINIMA_TRAP_MINION_HALF_SIZE: ae::Vec2 = ae::Vec2::new(24.0, 11.0);
/// Horizontal offset (px) from the pit center where the minion
/// spawns. Pushed toward the boss side so the player sees the
/// slug appear *next* to the pit instead of *under* them. 90 px
/// is well outside both the pit's 56-px half-extent and the
/// player's body so the slug never overlaps the player on the
/// frame it appears.
const MINIMA_TRAP_MINION_SPAWN_OFFSET_PX: f32 = 90.0;

/// EFFECTS consumer: `minima_trap` pit + optional puppy_slug.
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
    // `select_actor_targets`); the multi-player-ready pattern.
    player_query: Query<
        &ambition_platformer2d_core::BodyKinematics,
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
    mut bosses: Query<
        (
            Entity,
            BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &mut MinimaTrapState,
            Option<&ambition_combat::components::ActorTarget>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let firing = super::actors_firing(&mut messages, MINIMA_TRAP_KEY);

    for (entity, boss_feature, health, mut state, actor_target) in &mut bosses {
        let boss = boss_feature.as_boss_ref();
        let player_pos = actor_target.and_then(|t| {
            t.entity
                .and_then(|e| player_query.get(e).ok())
                .map(|kin| kin.aabb().center())
                .or(Some(t.pos))
        });
        if !firing.contains_key(&entity) {
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
                faction: ambition_vfx::HitSide::Boss,
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
            // Do not spawn the slug on top of the player. Offset it horizontally
            // toward the boss, so it appears on the boss side of the pit and the player
            // has time to retreat. Half the offset covers the slug's half-width plus a
            // read margin.
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
                    pos: minion_pos,
                    half_size: MINIMA_TRAP_MINION_HALF_SIZE,
                    character_id: MINIMA_TRAP_MINION_CHARACTER.to_string(),
                    encounter_id,
                    faction: ambition_vfx::HitSide::Enemy,
                    // A boss drops a minion; nobody rides it.
                    ridden_by_summoner: None,
                    // The sentinel's minions keep the vitals their character authors.
                    health: None,
                    // A boss minion keeps its character's hazard.
                    keeps_contact_damage: true,
                }),
            });
        }

        state.fired_this_strike = true;
        state.spawn_index = state.spawn_index.wrapping_add(1);
    }
}
