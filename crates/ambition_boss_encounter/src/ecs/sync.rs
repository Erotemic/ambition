//! Boss construction snapshot, sprite metrics + render targets, and the
//! spawn-time hurtbox volumes. Sibling of `tick.rs` (the per-frame boss update).

use ambition_sprite_sheet::ActorSpriteMetrics;
// Named imports, not a glob, so the dependencies are visible to grep.
use ambition_combat::components::{ActorDisposition, ActorIdentity};
use ambition_platformer2d_core as ae;
use bevy::prelude::{Component, Entity, Query, Res, With, Without};

use crate::attack_geometry::bounding_aabb;
use ambition_characters::brain::{Brain, StateMachineCfg};
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_sprite_sheet::SheetRegistry;
use bevy::prelude::Commands;

/// Marker that a boss entity has had its sprite metrics applied
/// (once-per-boss derivation gate). Inserted by
/// [`derive_boss_sprite_metrics`] when it walks a new boss.
#[derive(Component, Clone, Copy, Debug)]
pub struct BossSpriteMetricsApplied;

/// A boss's shared actor components at construction: its identity, and its
/// initial disposition. A boss starts Hostile; after that, the general runtime
/// owns `ActorDisposition` (targeting stand-down, release pacification), as for
/// every other actor.
pub fn boss_component_snapshot(boss: crate::BossRef<'_>) -> (ActorIdentity, ActorDisposition) {
    (
        ActorIdentity::new(boss.config.id.clone(), boss.config.name.clone()),
        ActorDisposition::Hostile,
    )
}

/// The sprite-registry target id a boss draws from — its authored
/// `BossBehaviorProfile::sprite_target`, or its `id` when unset (the common
/// case). The sprite generator's `target` does not always match the boss id:
/// clockwork_warden and gradient_sentinel share the generic `"boss"` sheet,
/// GNU-ton draws `"gnu_ton_boss"`, the mockingbird `"mockingbird_boss"`, each
/// authored in `boss_profiles.ron`. The engine names no boss here.
pub fn sprite_target_for_boss(behavior: &crate::pattern::profile::BossBehaviorProfile) -> &str {
    behavior.sprite_target.as_deref().unwrap_or(&behavior.id)
}

/// World-space size of the rendered sprite quad for a boss, given the
/// boss's spawn / collision size and its sprite target.
///
/// The visible sprite is rendered at `max(size) * collision_scale`, where
/// `collision_scale` is per sheet (for example 1.6 for the clockwork /
/// gradient sentinel `BOSS_SHEET`, 1.25 for the mockingbird, 4.5 for
/// GNU-ton). The hurtbox/hitbox math needs this value, not `boss.size`, as
/// the world scale, so the boxes cover the visible body.
///
/// Unknown targets get a 1.0 scale (the sprite renders at `boss.size`), for
/// test fixtures and bosses without a registered sheet.
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

/// Read the sprite registry for each new boss and copy its `body_metrics`
/// into `BossEncounter::sprite_metrics`. Also derive an updated `combat_size`
/// from the bounding box of the body parts, so the boss's collision and soft
/// world-bounds clamp match the visible body, not the LDtk BossSpawn AABB.
///
/// Gated by the `BossSpriteMetricsApplied` marker, so each boss is processed
/// once. Skips bosses whose sprite target is not in the registry (they keep
/// their authored or fallback combat_size).
///
/// When the brain is `BossPattern { cfg, .. }`, this also writes the derived
/// combat_size into `cfg.combat_size`, so the brain's soft world-bounds clamp
/// matches the new envelope.
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
        // Headless or minimal-plugin tests do not init the sprite registry.
        // With no metadata, this is a no-op and the boss keeps its
        // `combat_size`.
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
            // `kin.size` is the collision envelope, so refine it to the
            // sprite-derived combat size too (the render basis stays in
            // `status.render_size`). The shared movement seam then sweeps the
            // real body.
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

/// Compute the rest-pose damageable hurtbox volumes a boss would expose when
/// spawned from an authored `BossSpawn` at `aabb`. Resolves the boss's sprite
/// metrics from the baked sheet registry (no Bevy `App`) and returns
/// world-space AABBs. Used by the headless geometry-debug renderer; live
/// combat uses the ECS path.
///
/// The world scale is the sprite render size, not `boss.size`: the visible
/// sprite renders at `max(boss.size) * collision_scale`, which is larger than
/// the LDtk spawn AABB. The `combat_offset` (`bound.center() - boss.pos`)
/// accounts for a body bbox that is not centered in the sprite frame (for
/// GNU-ton, about 41 px above `boss.pos`).
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
    // AS4b: scale from the sprite render basis, not `kin.size` (now the collision
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

    /// Every `BodyCombat` field declares who writes it on the boss road.
    ///
    /// The boss snapshot returns no `BodyCombat`; it writes derived liveness
    /// in place. A rebuild with a hand-kept list of restored timers once
    /// dropped `landing_lag_timer` on both the boss and actor roads.
    #[allow(dead_code)]
    fn every_body_combat_field_declares_whether_the_boss_sync_writes_it(combat: &BodyCombat) {
        let BodyCombat {
            // Untouched: reaction history the damage path owns, the
            // move-derived super-armor bit, and the authored sandbag flag. The
            // boss sync writes nothing here.
            hit_flash: _,
            struck_recently: _,
            damage_invuln_timer: _,
            hitstun_timer: _,
            recoil_lock_timer: _,
            hitstop_timer: _,
            asdi_owed: _,
            landing_lag_timer: _,
            // A sleep a move applied. Reaction history like the locks above it,
            // and the boss sync has no opinion about it either.
            sleep_timer: _,
            armor: _,
            training_dummy: _,
        } = combat;
    }
}
