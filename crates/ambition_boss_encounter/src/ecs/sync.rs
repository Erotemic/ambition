//! Boss component sync: mirror boss cluster state onto the generic actor
//! read-model components, derive sprite metrics + render targets, and build the
//! spawn-time hurtbox volumes. Sibling of `tick.rs` (the per-frame boss update).

use ambition_sprite_sheet::ActorSpriteMetrics;
// ⭐ NAMED, NOT GLOBBED. This was `use super::super::*`, a glob over the
// whole `features/ecs` module — a channel a `crate::` grep cannot see, and
// the reason a carve estimate needs more than an import count. Measured by
// deleting it: everything it actually supplied was bevy's prelude and
// `WorldTime`, and NO monolith vocabulary at all.
use ambition_combat::components::{ActorDisposition, ActorIdentity, CombatKit};
use ambition_platformer2d_core as ae;
use bevy::prelude::{Component, Entity, Query, Res, With, Without};

use crate::attack_geometry::bounding_aabb;
use ambition_characters::brain::{BossAttackState, Brain, StateMachineCfg};
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_sprite_sheet::SheetRegistry;
use bevy::prelude::Commands;

/// Marker that a boss entity has had its sprite metrics applied
/// (once-per-boss derivation gate). Inserted by
/// [`derive_boss_sprite_metrics`] when it walks a new boss.
#[derive(Component, Clone, Copy, Debug)]
pub struct BossSpriteMetricsApplied;

/// Build the shared actor combat read-model snapshot for a boss.
///
/// Bosses still own encounter-specific state through [`BossFeature`] and the boss encounter
/// registry, but their generic combat shape is now exposed through the same `ActorIdentity` /
/// `BodyHealth` / `BodyCombat` components used by NPCs and enemies. This keeps future faction,
/// targeting, HUD, and held-item work from needing to pattern-match directly on `BossFeature` for
/// ordinary combat facts. IT NO LONGER RETURNS A `BodyCombat` (AC3.2).
///
/// a citation is only as correct as the thing it cites. The comment here
/// said "the same rule as `sync_actor_components_from_cluster`" and it was
/// accurate; the rule it named was wrong.
///
/// The liveness the caller now writes in place is the last derived fact this
/// produced, and AC3.1.A deletes even that.
pub fn boss_component_snapshot(boss: crate::BossRef<'_>) -> (ActorIdentity, ActorDisposition) {
    (
        ActorIdentity::new(boss.config.id.clone(), boss.config.name.clone()),
        ActorDisposition::Hostile,
    )
}

/// Keep boss shared-actor read models synced from the boss runtime and brain
/// attack state. Boss integration remains in [`update_ecs_bosses`]; this system
/// only mirrors generic combat facts into components shared with NPC/enemy
/// actors.
pub fn sync_boss_actor_components(
    mut bosses: Query<
        (
            crate::BossClusterRef,
            &BossAttackState,
            &ambition_characters::brain::ActionSet,
            &mut CombatKit,
            &mut ActorIdentity,
            &mut ActorDisposition,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for (feature, _attack_state, action_set, mut combat_kit, mut identity, mut disposition) in
        &mut bosses
    {
        // AC3.1.A: this loop no longer touches `BodyCombat` or `BodyHealth` at all.
        let (next_identity, next_disposition) = boss_component_snapshot(feature.as_boss_ref());
        *combat_kit = CombatKit::from_action_set(action_set);
        *identity = next_identity;
        *disposition = next_disposition;
    }
}

/// The sprite-registry target id a boss draws from — its authored
/// `BossBehaviorProfile::sprite_target`, or its `id` when unset (the common
/// case). The sprite generator's `target` doesn't always match the boss id:
/// clockwork_warden / gradient_sentinel share the generic `"boss"` sheet,
/// GNU-ton draws `"gnu_ton_boss"`, the mockingbird `"mockingbird_boss"` — each
/// authored in `boss_profiles.ron`. The engine names no boss here.
pub fn sprite_target_for_boss(behavior: &crate::pattern::profile::BossBehaviorProfile) -> &str {
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
    catalog: &crate::BossCatalog,
    behavior: &crate::pattern::profile::BossBehaviorProfile,
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
    boss_catalog: Res<crate::BossCatalog>,
    registry: Option<Res<SheetRegistry>>,
    mut bosses: Query<
        (Entity, crate::BossClusterQueryData, Option<&mut Brain>),
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
    boss_catalog: &crate::BossCatalog,
    id: &str,
    name: &str,
    aabb: ae::Aabb,
    brain: ambition_entity_catalog::placements::BossBrain,
) -> Vec<ae::CombatVolume> {
    let registry = ambition_sprite_sheet::baked_sheet_registry();
    let mut boss = crate::BossClusterScratch::new(boss_catalog, id, name, aabb, brain);
    if let Some((metrics, _)) =
        boss_sprite_metrics_from_registry(boss_catalog, boss.as_ref(), &registry)
    {
        boss.status.sprite_metrics = Some(metrics);
    }
    let attack_state = ambition_characters::brain::BossAttackState::default();
    crate::attack_geometry::damageable_volumes(
        &crate::attack_geometry::BossVolumeContext::from_ref(
            boss_catalog,
            boss.as_ref(),
            &attack_state,
        ),
    )
}

pub(crate) fn boss_sprite_metrics_from_registry(
    boss_catalog: &crate::BossCatalog,
    boss: crate::BossRef<'_>,
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
    let body_aabbs = crate::attack_geometry::world_space_body_aabbs_from_parts(
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
    use ambition_characters::actor::BodyCombat;

    /// EVERY `BodyCombat` FIELD DECLARES WHO WRITES IT ON THE BOSS ROAD.
    ///
    /// the original of this guard recorded a propagated error.
    /// `boss_component_snapshot` rebuilt `BodyCombat` and restored a list of
    /// timers *"the same rule as `sync_actor_components_from_cluster`"* — an
    /// accurate citation of a wrong rule, so both roads forgot
    /// `landing_lag_timer` and a boss landing out of an authored aerial had its
    /// lag erased on the next frame.
    ///
    ///  AC3.2 removed the rebuild from both roads at once. The boss snapshot no
    /// longer returns a `BodyCombat` at all; it writes derived liveness in place.
    #[allow(dead_code)]
    fn every_body_combat_field_declares_whether_the_boss_sync_writes_it(combat: &BodyCombat) {
        let BodyCombat {
            // ── UNTOUCHED (8) — reaction history the damage path owns, the
            // move-derived super-armor bit, and the authored sandbag flag. The
            // boss sync writes NOTHING here now.
            hit_flash: _,
            damage_invuln_timer: _,
            hitstun_timer: _,
            recoil_lock_timer: _,
            hitstop_timer: _,
            asdi_owed: _,
            landing_lag_timer: _,
            armored: _,
            training_dummy: _,
        } = combat;
    }
}
