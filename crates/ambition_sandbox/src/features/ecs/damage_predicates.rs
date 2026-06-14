//! Read-only hit-test predicates, split out of `damage.rs` (Refactor 6: shrink
//! the incremental rebuild unit). The `ecs_hit_event_hits_*` functions let the
//! projectile / attack systems pre-check whether a queued `HitEvent` will land
//! before kicking off cues — the same overlap rules `apply_feature_hit_events`
//! applies (including multi-part boss hurtboxes). A straight code move.

use bevy::prelude::{Query, With, Without};

use super::{
    ActorCombatState, ActorDisposition, BossConfig, BreakableFeature, FeatureAabb, FeatureId,
    FeatureSimEntity, HitEvent,
};
use crate::engine_core::AabbExt;

/// True if `ignored_targets` contains the `"{prefix}:{id}"` disposition key,
/// WITHOUT allocating that key. The old code `format!("enemy:{id}")`-ed a fresh
/// `String` for every actor on every hit event just to compare it; this matches
/// the same keys by slicing each ignored entry instead — removing per-hit
/// allocator churn that compounds when RL steps the sim millions of times, and
/// folding the six copies of the pattern into one helper.
pub(super) fn target_is_ignored(ignored_targets: &[String], prefix: &str, id: &str) -> bool {
    ignored_targets.iter().any(|ignored| {
        ignored
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix(':'))
            == Some(id)
    })
}

/// Read-only hit test used by systems that need immediate projectile / attack
/// feedback while damage application is still drained through
/// typed Bevy messages.
pub fn ecs_hit_event_hits_breakable(
    event: &HitEvent,
    breakables: &Query<(&FeatureId, &FeatureAabb, &BreakableFeature), With<FeatureSimEntity>>,
) -> bool {
    breakables.iter().any(|(id, aabb, feature)| {
        !target_is_ignored(&event.ignored_targets, "breakable", id.as_str())
            && !feature.broken()
            && feature.breakable.trigger.allows_hit()
            && !feature.breakable.pogo_refresh
            && event.volume.strict_intersects(aabb.aabb())
    })
}

pub fn ecs_hit_event_hits_actor(
    event: &HitEvent,
    actors: &Query<
        (
            &FeatureId,
            &FeatureAabb,
            &ActorDisposition,
            &ActorCombatState,
        ),
        (With<FeatureSimEntity>, Without<BossConfig>),
    >,
) -> bool {
    actors.iter().any(|(id, aabb, disposition, combat)| {
        let prefix = match *disposition {
            ActorDisposition::Peaceful => "npc",
            ActorDisposition::Hostile => "enemy",
        };
        !target_is_ignored(&event.ignored_targets, prefix, id.as_str())
            && combat.alive
            && event.volume.strict_intersects(aabb.aabb())
    })
}

pub fn ecs_hit_event_hits_boss(
    event: &HitEvent,
    bosses: &Query<
        (
            &FeatureId,
            &FeatureAabb,
            super::boss_clusters::BossClusterRef,
            &crate::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<FeatureSimEntity>,
    >,
) -> bool {
    // Check against `damageable_volumes` so the hit-check matches
    // what `apply_feature_hit_events` will actually apply damage
    // to. Multi-part bosses (e.g. GNU-ton) have a gross
    // `FeatureAabb` covering the whole creature but only the head
    // is actually damageable — checking against the gross AABB
    // would over-trigger projectile termination on the body without
    // ever applying damage. `damageable_volumes` reads the brain's
    // `BossAttackState` to decide head-descent vs rest position, and
    // the live `BossAnimationFrameSample` (same component
    // `apply_boss_hit` consumes) so the projectile's hit/terminate
    // check locks to the exact rendered frame instead of an
    // elapsed-time estimate — otherwise the projectile could
    // register a hit a few frames off from where the head is drawn
    // and where damage actually lands.
    bosses
        .iter()
        .any(|(id, _aabb, feature, attack_state, animation_frame)| {
            if target_is_ignored(&event.ignored_targets, "boss", id.as_str()) {
                return false;
            }
            if !feature.status.alive {
                return false;
            }
            crate::features::damageable_volumes(
                &crate::features::BossVolumeContext::from_ref(feature.as_boss_ref(), attack_state)
                    .with_animation_frame(animation_frame),
            )
            .iter()
            .any(|part| event.volume.strict_intersects(*part))
        })
}
