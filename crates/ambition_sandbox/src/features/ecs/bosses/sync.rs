//! Boss component sync: mirror runtime boss state into ECS components, derive
//! sprite metrics + render targets, and the spawn-time hurtbox volumes.

use super::super::*;

use crate::brain::{BossAttackState, Brain, StateMachineCfg};
use crate::engine_core::AabbExt;
use crate::features::bosses::BossSpriteMetrics;
use crate::features::bounding_aabb;
use ambition_sprite_sheet::SheetRegistry;
use bevy::prelude::Commands;

/// Marker that a boss entity has had its sprite metrics applied
/// (once-per-boss derivation gate). Inserted by
/// [`derive_boss_sprite_metrics`] when it walks a new boss.
#[derive(Component, Clone, Copy, Debug)]
pub struct BossSpriteMetricsApplied;

/// Build the shared actor combat read-model snapshot for a boss.
///
/// Bosses still own encounter-specific state through [`BossFeature`] and the
/// boss encounter registry, but their generic combat shape is now exposed
/// through the same `ActorIdentity` / `ActorHealth` / `ActorCombatState` /
/// `ActorIntent` components used by NPCs and enemies. This keeps future
/// faction, targeting, HUD, and held-item work from needing to pattern-match
/// directly on `BossFeature` for ordinary combat facts.
pub(crate) fn boss_component_snapshot(
    boss: super::super::boss_clusters::BossRef<'_>,
    attack_state: &BossAttackState,
) -> (
    ActorIdentity,
    ActorDisposition,
    ActorHealth,
    ActorCombatState,
    ActorIntent,
    ActorCooldowns,
) {
    let mode = if !boss.status.alive {
        crate::actor::ai::CharacterAiMode::Dead
    } else if attack_state.active_profile.is_some() {
        crate::actor::ai::CharacterAiMode::Attack
    } else if attack_state.telegraph_profile.is_some() {
        crate::actor::ai::CharacterAiMode::Telegraph
    } else {
        crate::actor::ai::CharacterAiMode::Chase
    };
    (
        ActorIdentity::new(boss.config.id.clone(), boss.config.name.clone()),
        ActorDisposition::Hostile,
        ActorHealth::new(boss.status.health),
        ActorCombatState::hostile(
            boss.status.alive,
            boss.status.hit_flash,
            attack_state.telegraph_remaining,
            attack_state.active_remaining,
            false,
        ),
        ActorIntent::new(mode),
        ActorCooldowns::default(),
    )
}

/// Keep boss shared-actor read models synced from the boss runtime and brain
/// attack state. Boss integration remains in [`update_ecs_bosses`]; this system
/// only mirrors generic combat facts into components shared with NPC/enemy
/// actors.
pub fn sync_boss_actor_components(
    mut bosses: Query<
        (
            super::super::boss_clusters::BossClusterRef,
            &BossAttackState,
            &crate::brain::ActionSet,
            &mut CombatKit,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &mut ActorHealth,
            &mut ActorCombatState,
            &mut ActorIntent,
            &mut ActorCooldowns,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for (
        feature,
        attack_state,
        action_set,
        mut combat_kit,
        mut identity,
        mut disposition,
        mut health,
        mut combat,
        mut intent,
        mut cooldowns,
    ) in &mut bosses
    {
        let (
            next_identity,
            next_disposition,
            next_health,
            next_combat,
            next_intent,
            next_cooldowns,
        ) = boss_component_snapshot(feature.as_boss_ref(), attack_state);
        *combat_kit = CombatKit::from_action_set(action_set);
        *identity = next_identity;
        *disposition = next_disposition;
        *health = next_health;
        *combat = next_combat;
        *intent = next_intent;
        *cooldowns = next_cooldowns;
    }
}

/// Map a boss `BossBehaviorProfile::id` to its sprite-registry
/// target id. The sprite generator's `target` field doesn't always
/// match the boss internal id — clockwork_warden / gradient_sentinel
/// both share the generic `"boss"` sheet. Future RON-authored
/// boss specs can carry their own sprite target string; for now
/// this match is the single mapping point.
pub fn sprite_target_for_boss(behavior_id: &str) -> &str {
    match behavior_id {
        "clockwork_warden" | "gradient_sentinel" => "boss",
        // GNU-ton's hand-tuned `gnu_ton_part_aabb` math was migrated
        // into the standard `body_metrics` pipeline (the
        // `gnu_ton_boss_spritesheet.ron`'s `animations` block) on
        // 2026-05-26. Map the behavior id to that sheet target so
        // `derive_boss_sprite_metrics` picks up the per-animation
        // hitboxes / hurtboxes.
        "gnu_ton" => "gnu_ton_boss",
        other => other,
    }
}

/// World-space size of the rendered sprite quad for a boss, given the
/// boss's spawn / collision size and its sprite target.
///
/// The visible sprite is rendered at `max(size) * collision_scale`,
/// where `collision_scale` is per-sheet (1.6 for the clockwork /
/// gradient sentinel `BOSS_SHEET`, 1.25 for the mockingbird sheet,
/// 4.5 for GNU-ton). The hurtbox / hitbox math needs THIS value
/// (not `boss.size`) as the world scale so the cyan / red / yellow
/// boxes cover the visible body. Otherwise the boxes end up half
/// the size of what the player sees.
///
/// Unknown targets get a 1.0 scale fallback (sprite renders at
/// `boss.size`) — that's the safe "no sprite spec known" case used
/// by test fixtures and bosses without a registered sheet.
pub fn sprite_render_size_for(target: &str, boss_size: ae::Vec2) -> ae::Vec2 {
    use crate::boss_encounter::sprites;
    let spec = match target {
        "boss" => Some(sprites::BOSS_SHEET),
        "mockingbird" => Some(sprites::MOCKINGBIRD_SHEET),
        "smirking_behemoth_boss" => Some(sprites::SMIRKING_BEHEMOTH_SHEET),
        // `gnu_ton_boss` is the actual sheet target ID emitted by the
        // gnu_ton spritesheet RON. `gnu_ton_body` / `gnu_ton_hands` /
        // `gnu_ton` (legacy aliases) stay mapped for compatibility.
        "gnu_ton_boss" | "gnu_ton" | "gnu_ton_body" | "gnu_ton_hands" => {
            Some(sprites::GNU_TON_SHEET)
        }
        _ => None,
    };
    let Some(spec) = spec else {
        return boss_size;
    };
    let bevy_size = bevy::math::Vec2::new(boss_size.x, boss_size.y);
    let render = spec.render_size(bevy_size);
    ae::Vec2::new(render.x, render.y)
}

/// Read the sprite registry for each freshly-spawned boss and copy
/// its `body_metrics` into `BossRuntime::sprite_metrics`. Also
/// derives an updated `combat_size` from the bounding box of the
/// body parts so the boss's collision + soft world-bounds clamp
/// scales with the visible sprite body instead of the LDtk
/// BossSpawn AABB.
///
/// Gated by the `BossSpriteMetricsApplied` marker so each boss is
/// processed exactly once. Skips bosses whose sprite target isn't
/// in the registry (the boss keeps its authored / fallback
/// combat_size).
///
/// When the boss's brain is `BossPattern { cfg, .. }`, the system
/// also writes the derived combat_size into `cfg.combat_size` so
/// the brain's soft world-bounds clamp matches the new physical
/// envelope (otherwise the brain would still clamp against the
/// stale 64×80 spawn AABB).
pub fn derive_boss_sprite_metrics(
    mut commands: Commands,
    registry: Option<Res<SheetRegistry>>,
    mut bosses: Query<
        (
            Entity,
            super::super::boss_clusters::BossClusterQueryData,
            Option<&mut Brain>,
        ),
        (With<FeatureSimEntity>, Without<BossSpriteMetricsApplied>),
    >,
) {
    let Some(registry) = registry else {
        // Headless / minimal-plugin tests don't init the sprite
        // registry. With no metadata available, the derivation is a
        // no-op — boss keeps its hardcoded `combat_size`.
        return;
    };
    if registry.is_empty() {
        // Registry hasn't loaded yet — retry next frame. Don't
        // insert the gate marker so the next tick re-attempts.
        return;
    }
    for (entity, mut feature, brain_opt) in &mut bosses {
        let Some((snapshot, derived_combat_size)) =
            boss_sprite_metrics_from_registry(feature.as_boss_ref(), &registry)
        else {
            // No metadata for this boss — leave defaults alone.
            commands.entity(entity).insert(BossSpriteMetricsApplied);
            continue;
        };
        feature.status.sprite_metrics = Some(snapshot);
        if let Some(derived) = derived_combat_size {
            feature.config.behavior.combat_size = Some(derived);
            // Mirror into the brain cfg so the soft world-bounds
            // clamp uses the new value too.
            if let Some(mut brain) = brain_opt {
                if let Brain::StateMachine(StateMachineCfg::BossPattern { cfg, .. }) = &mut *brain {
                    cfg.combat_size = derived;
                }
            }
        }
        commands.entity(entity).insert(BossSpriteMetricsApplied);
    }
}

/// Pure derivation of a boss's sprite metrics + updated combat size from
/// the sheet registry. Extracted from [`derive_boss_sprite_metrics`] so
/// headless tools and tests can compute boss hurtbox geometry without the
/// ECS system (which additionally writes the derived size into the boss
/// brain cfg). Returns `None` when the boss's sprite target has no body
/// metrics; otherwise `(metrics, Some(derived_combat_size))` where the
/// combat size is `None` if there were no body parts to bound.
///
/// Uses the SPRITE RENDER SIZE (not `boss.size`) as the world-scale base —
/// the visible sprite renders at `max(boss.size) * collision_scale`, which
/// is bigger than the LDtk spawn AABB. The `combat_offset`
/// (`bound.center() - boss.pos`) captures that the body bbox isn't
/// necessarily centered in the sprite frame, so `boss.aabb()` lines up
/// with the visible body (GNU-ton's is ~41 px above `boss.pos`).
/// Compute the rest-pose damageable hurtbox volumes a boss would expose
/// when spawned from an authored `BossSpawn` at `aabb`. Resolves the
/// boss's sprite metrics from the baked sheet registry (no Bevy `App`)
/// and returns world-space AABBs. Exposed for the headless geometry-debug
/// renderer so boss combat geometry can be verified in a room without
/// launching the game; live combat uses the ECS path.
pub fn boss_spawn_hurtboxes(
    id: &str,
    name: &str,
    aabb: ae::Aabb,
    brain: crate::actor::BossBrain,
) -> Vec<ae::Aabb> {
    let registry = crate::character_sprites::baked_sheet_registry();
    let mut boss = super::super::boss_clusters::BossClusterScratch::new(id, name, aabb, brain);
    if let Some((metrics, _)) = boss_sprite_metrics_from_registry(boss.as_ref(), &registry) {
        boss.status.sprite_metrics = Some(metrics);
    }
    let attack_state = crate::brain::BossAttackState::default();
    crate::features::damageable_volumes(&crate::features::BossVolumeContext::from_ref(
        boss.as_ref(),
        &attack_state,
    ))
}

pub(crate) fn boss_sprite_metrics_from_registry(
    boss: super::super::boss_clusters::BossRef<'_>,
    registry: &SheetRegistry,
) -> Option<(BossSpriteMetrics, Option<ae::Vec2>)> {
    let target = sprite_target_for_boss(&boss.config.behavior.id);
    let (metrics, frame_w, frame_h) = registry.body_metrics(target)?;
    let sprite_render_size = sprite_render_size_for(target, boss.kin.size);
    let mut snapshot = BossSpriteMetrics {
        frame_width: frame_w,
        frame_height: frame_h,
        body_pixel_bbox: metrics.body_pixel_bbox,
        body_pixel_parts: metrics.body_pixel_parts.clone(),
        sprite_render_size,
        combat_offset: ae::Vec2::ZERO,
        animations: metrics.animations.clone(),
    };
    let body_aabbs = crate::features::world_space_body_aabbs_from_parts(
        &snapshot.body_pixel_parts,
        snapshot.body_pixel_bbox,
        frame_w,
        frame_h,
        boss.kin.pos,
        sprite_render_size,
    );
    let derived = bounding_aabb(&body_aabbs);
    if let Some(bound) = derived {
        snapshot.combat_offset = bound.center() - boss.kin.pos;
    }
    Some((snapshot, derived.map(|b| b.half_size() * 2.0)))
}
