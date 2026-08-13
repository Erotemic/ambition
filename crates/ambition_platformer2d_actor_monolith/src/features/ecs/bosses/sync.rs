//! Boss component sync: mirror boss cluster state onto the generic actor
//! read-model components, derive sprite metrics + render targets, and build the
//! spawn-time hurtbox volumes. Sibling of `tick.rs` (the per-frame boss update).

use super::super::*;

use crate::features::bosses::ActorSpriteMetrics;
use crate::features::bounding_aabb;
use ambition_characters::brain::{BossAttackState, Brain, StateMachineCfg};
use ambition_platformer2d_core::AabbExt;
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
/// through the same `ActorIdentity` / `BodyHealth` / `BodyCombat`
/// components used by NPCs and enemies. This keeps future
/// faction, targeting, HUD, and held-item work from needing to pattern-match
/// directly on `BossFeature` for ordinary combat facts.
pub fn boss_component_snapshot(
    boss: super::super::boss_clusters::BossRef<'_>,
    attack_state: &BossAttackState,
    // The boss's HP authority (§A1) — liveness is `health.alive()`, never a
    // boss-state shadow flag.
    health: &BodyHealth,
    // The body's current `BodyCombat`: the reaction timers (hit_flash,
    // post-hit i-frame, the §A2 stagger set) are AUTHORITATIVE state written
    // by the damage path — carry them across the presentation rebuild, the
    // same rule as `sync_actor_components_from_cluster`.
    prev_combat: &BodyCombat,
) -> (
    ActorIdentity,
    ActorDisposition,
    BodyCombat,
) {
    let alive = health.alive();
    let mut combat = BodyCombat::hostile(
        alive,
        prev_combat.hit_flash,
        attack_state.telegraph_remaining,
        attack_state.active_remaining,
        false,
    );
    combat.damage_invuln_timer = prev_combat.damage_invuln_timer;
    combat.hitstun_timer = prev_combat.hitstun_timer;
    combat.recoil_lock_timer = prev_combat.recoil_lock_timer;
    combat.hitstop_timer = prev_combat.hitstop_timer;
    (
        ActorIdentity::new(boss.config.id.clone(), boss.config.name.clone()),
        ActorDisposition::Hostile,
        combat,
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
            &ambition_characters::brain::ActionSet,
            &mut CombatKit,
            &mut ActorIdentity,
            &mut ActorDisposition,
            &BodyHealth,
            &mut BodyCombat,
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
        health,
        mut combat,
    ) in &mut bosses
    {
        // `health` is the boss's HP AUTHORITY now (§A1) — read, never rebuilt.
        let (next_identity, next_disposition, next_combat) =
            boss_component_snapshot(feature.as_boss_ref(), attack_state, &health, &combat);
        *combat_kit = CombatKit::from_action_set(action_set);
        *identity = next_identity;
        *disposition = next_disposition;
        *combat = next_combat;
    }
}

/// The sprite-registry target id a boss draws from — its authored
/// `BossBehaviorProfile::sprite_target`, or its `id` when unset (the common
/// case). The sprite generator's `target` doesn't always match the boss id:
/// clockwork_warden / gradient_sentinel share the generic `"boss"` sheet,
/// GNU-ton draws `"gnu_ton_boss"`, the mockingbird `"mockingbird_boss"` — each
/// authored in `boss_profiles.ron`. The engine names no boss here.
pub fn sprite_target_for_boss(
    behavior: &crate::boss_encounter::behavior::BossBehaviorProfile,
) -> &str {
    behavior.sprite_target.as_deref().unwrap_or(&behavior.id)
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
pub fn sprite_render_size_for(
    catalog: &crate::boss_encounter::BossCatalog,
    behavior: &crate::boss_encounter::behavior::BossBehaviorProfile,
    boss_size: ae::Vec2,
) -> ae::Vec2 {
    let spec = catalog.sheet_for_behavior(behavior);
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
    boss_catalog: Res<crate::boss_encounter::BossCatalog>,
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
            boss_sprite_metrics_from_registry(&boss_catalog, feature.as_boss_ref(), &registry)
        else {
            // No metadata for this boss — leave defaults alone.
            commands.entity(entity).insert(BossSpriteMetricsApplied);
            continue;
        };
        feature.status.sprite_metrics = Some(snapshot);
        if let Some(derived) = derived_combat_size {
            feature.config.behavior.combat_size = Some(derived);
            // AS4b: `kin.size` IS the collision envelope, so refine it to the
            // sprite-derived combat size too (the render basis stays put in
            // `status.render_size`). This keeps the shared movement seam sweeping the
            // real body once the boss integrates through the flight limb (AS4c).
            feature.kin.size = derived;
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
    boss_catalog: &crate::boss_encounter::BossCatalog,
    id: &str,
    name: &str,
    aabb: ae::Aabb,
    brain: ambition_entity_catalog::placements::BossBrain,
) -> Vec<ae::CombatVolume> {
    let registry = ambition_sprite_sheet::baked_sheet_registry();
    let mut boss =
        super::super::boss_clusters::BossClusterScratch::new(boss_catalog, id, name, aabb, brain);
    if let Some((metrics, _)) =
        boss_sprite_metrics_from_registry(boss_catalog, boss.as_ref(), &registry)
    {
        boss.status.sprite_metrics = Some(metrics);
    }
    let attack_state = ambition_characters::brain::BossAttackState::default();
    crate::features::damageable_volumes(&crate::features::BossVolumeContext::from_ref(
        boss_catalog,
        boss.as_ref(),
        &attack_state,
    ))
}

pub(crate) fn boss_sprite_metrics_from_registry(
    boss_catalog: &crate::boss_encounter::BossCatalog,
    boss: super::super::boss_clusters::BossRef<'_>,
    registry: &SheetRegistry,
) -> Option<(ActorSpriteMetrics, Option<ae::Vec2>)> {
    let target = sprite_target_for_boss(&boss.config.behavior);
    let (metrics, frame_w, frame_h) = registry.body_metrics(target)?;
    // AS4b: scale from the sprite render BASIS, not `kin.size` (now the collision
    // envelope) — so the derived world metrics are unchanged by the size flip.
    let sprite_render_size =
        sprite_render_size_for(boss_catalog, &boss.config.behavior, boss.status.render_size);
    let mut snapshot = ActorSpriteMetrics {
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

#[cfg(test)]
mod boss_combat_rebuild_contract {
    use super::*;

    /// **THE BOSS ROAD'S CARRY LIST, DECLARED FIELD BY FIELD** — ledger D108,
    /// second site.
    ///
    /// [`boss_component_snapshot`] rebuilds `BodyCombat` and restores the
    /// reaction timers by hand, and its own comment says it follows *"the same
    /// rule as `sync_actor_components_from_cluster`"*. **It does — including
    /// that function's omission.** Both carry five timers and neither carries
    /// `landing_lag_timer`, because this list was written by reading that one.
    ///
    /// ⛔ **a citation is only as correct as the thing it cites.** The comment
    /// was accurate and the rule it named was wrong, so the error propagated
    /// intact to a second road.
    ///
    /// ⇒ same remedy, same reason: adding a field to `BodyCombat` is now a
    /// compile error here until somebody says whether a boss keeps it.
    #[allow(dead_code)]
    fn every_body_combat_field_declares_whether_a_boss_keeps_it(combat: &BodyCombat) {
        let BodyCombat {
            // ── CARRIED ACROSS (5) — authoritative reaction state written by
            // the damage path; the presentation rebuild must not cancel it.
            hit_flash: _,
            damage_invuln_timer: _,
            hitstun_timer: _,
            recoil_lock_timer: _,
            hitstop_timer: _,

            // ── REBUILT (6) — derived from the boss's own attack state and HP
            // authority, which is the point of the refresh.
            alive: _,
            attacking: _,
            strike_count: _,
            attack_windup_timer: _,
            attack_timer: _,
            training_dummy: _,

            // ── ⛔ DROPPED (1) — NOT a decision. See D108.
            //
            // Bosses run the shared moveset runtime, so a boss that lands out of
            // an authored aerial has the same lag written and erased here that a
            // CPU fighter does.
            landing_lag_timer: _,
        } = combat;
    }
}
