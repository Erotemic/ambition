//! Read-only preflight hit predicates for projectile/attack feedback.
//!
//! A positive preflight may terminate a strike, so each predicate must match the
//! tangibility gate used by the corresponding damage applier. Actors use
//! `DamageableVolumes`, bosses test active authored part volumes, and breakables
//! mirror their broken/trigger/pogo gates. Actor and breakable precision remains
//! coarse-AABB by current gameplay policy; bosses require part-level precision.

use bevy::prelude::{Query, With, Without};

use ambition_boss_encounter::BossConfig;

use ambition_combat::components::{
    ActorDisposition, BreakableFeature, CenteredAabb, DamageableVolumes, FeatureId,
};
use ambition_combat::events::HitEvent;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;

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
    breakables: &Query<(&FeatureId, &CenteredAabb, &BreakableFeature), With<FeatureSimEntity>>,
) -> bool {
    breakables.iter().any(|(id, aabb, feature)| {
        !target_is_ignored(&event.ignored_targets, "breakable", id.as_str())
            && !feature.broken()
            && feature.breakable.trigger.allows_hit()
            && !feature.breakable.pogo_refresh
            && event.volume.intersects_aabb(aabb.aabb())
    })
}

/// Absent and empty mean OPPOSITE things here, which is the whole point of
/// [`DamageableVolumes::intangible`].
pub fn ecs_hit_event_hits_actor(
    event: &HitEvent,
    actors: &Query<
        (
            &FeatureId,
            &CenteredAabb,
            &ActorDisposition,
            // AC3.1.A: the liveness AUTHORITY. A damage gate is liveness-critical gameplay.
            &ambition_characters::actor::BodyHealth,
            Option<&DamageableVolumes>,
        ),
        (With<FeatureSimEntity>, Without<BossConfig>),
    >,
) -> bool {
    actors
        .iter()
        .any(|(id, aabb, disposition, health, volumes)| {
            let prefix = match *disposition {
                ActorDisposition::Peaceful => "npc",
                ActorDisposition::Hostile => "enemy",
            };
            !target_is_ignored(&event.ignored_targets, prefix, id.as_str())
            && health.alive()
            // Published, and published NOTHING: an authored invulnerable window
            // offers no target at all, so `apply_feature_hit_events` applies
            // nothing — it asks the SAME question as this predicate's first arm
            // through `strike_reaches_victim`. Saying `hit` here would despawn the
            // bolt and fire the hit trace for damage that never lands. The corpse
            // case already agreed (the publisher clears AND `alive` goes false);
            // this is the live-but-intangible state the two disagreed on.
            //
            // this is the intangibility half ONLY. A tangible body is still
            // tested against its coarse box below, not against the volumes it
            // published — see the module doc.
            && !volumes.is_some_and(DamageableVolumes::intangible)
            && event.volume.intersects_aabb(aabb.aabb())
        })
}

pub fn ecs_hit_event_hits_boss(
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    event: &HitEvent,
    bosses: &Query<
        (
            &FeatureId,
            &CenteredAabb,
            ambition_boss_encounter::BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &ambition_characters::brain::BossAttackState,
            Option<&ambition_boss_encounter::attack_geometry::BossAnimationFrameSample>,
        ),
        With<FeatureSimEntity>,
    >,
) -> bool {
    // Check against `damageable_volumes` so the hit-check matches
    // what `apply_feature_hit_events` will actually apply damage
    // to. Multi-part bosses (e.g. GNU-ton) have a gross
    // `CenteredAabb` covering the whole creature but only the head
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
    bosses.iter().any(
        |(id, _aabb, feature, health, attack_state, animation_frame)| {
            if target_is_ignored(&event.ignored_targets, "boss", id.as_str()) {
                return false;
            }
            if !health.alive() {
                return false;
            }
            ambition_combat::body_geometry::damageable_volumes(
                &ambition_boss_encounter::attack_geometry::BossVolumeContext::from_ref(
                    boss_catalog,
                    feature.as_boss_ref(),
                    attack_state,
                )
                .with_animation_frame(animation_frame),
            )
            .iter()
            .any(|part| event.volume.intersects(part))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::{Aabb, Vec2};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::World;

    /// The victim's coarse collision box.
    const BODY_CENTER: Vec2 = Vec2::new(100.0, 100.0);
    const BODY_HALF: Vec2 = Vec2::new(16.0, 24.0);

    /// A published silhouette high on the body — deliberately DISJOINT from the
    /// strike below, so the "published, non-empty" row can tell a coarse-box
    /// answer apart from a silhouette answer.
    fn published_head() -> Aabb {
        Aabb::new(Vec2::new(100.0, 116.0), Vec2::new(6.0, 6.0))
    }

    /// A strike low on the body: inside the coarse box, outside `published_head`.
    fn strike_event() -> HitEvent {
        HitEvent {
            strike_sfx: None,
            volume: Aabb::new(Vec2::new(90.0, 84.0), Vec2::new(4.0, 4.0)).into(),
            damage: 1,
            source: ambition_combat::events::HitSource::Projectile,
            attacker: None,
            target: ambition_combat::events::HitTarget::Volume,
            mode: ambition_combat::events::HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        }
    }

    /// One live, non-boss actor body carrying `volumes` (or nothing at all), asked
    /// the question `step_projectiles` asks before it despawns a bolt.
    fn strike_hits_body(volumes: Option<DamageableVolumes>) -> bool {
        let mut world = World::new();
        let mut body = world.spawn((
            FeatureSimEntity,
            FeatureId::new("mite"),
            CenteredAabb::new(BODY_CENTER, BODY_HALF),
            ActorDisposition::Hostile,
            // AC3.1.A: a LIVE body is one with health, not one with a mirror bit.
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(3)),
        ));
        if let Some(volumes) = volumes {
            body.insert(volumes);
        }
        let event = strike_event();
        world
            .run_system_once(
                move |actors: Query<
                    (
                        &FeatureId,
                        &CenteredAabb,
                        &ActorDisposition,
                        &ambition_characters::actor::BodyHealth,
                        Option<&DamageableVolumes>,
                    ),
                    (With<FeatureSimEntity>, Without<BossConfig>),
                >| { ecs_hit_event_hits_actor(&event, &actors) },
            )
            .expect("the hit predicate ran")
    }

    /// A published-EMPTY body is intangible, and this predicate exists to predict
    /// the applier — which refuses it through `strike_reaches_victim`'s first arm.
    ///
    /// All four `DamageableVolumes` states are pinned because the point is a RULE,
    /// not a patch: absent and unpublished must keep falling back to the coarse
    /// box (requiring the component, or reading an unpublished empty list as
    /// intangible, would silently turn this hit test into a no-op), and
    /// published-non-empty must also answer from the coarse box; authored rectangles are a
    /// separate precision policy and are deliberately not used here.
    #[test]
    fn the_actor_hit_test_refuses_a_body_that_published_no_hurtbox() {
        assert!(
            strike_hits_body(None),
            "no component: the coarse box is the only available answer"
        );
        assert!(
            strike_hits_body(Some(DamageableVolumes::default())),
            "unpublished: no publisher has spoken for this body yet, so the coarse \
             box still answers — an empty list is not yet an authored `nowhere`"
        );
        assert!(
            strike_hits_body(Some(DamageableVolumes::single(published_head()))),
            "published silhouette: this predicate still answers from the COARSE \
             box, so a strike that misses the silhouette but overlaps the box \
             reads as a hit. That is the precision half, and it is not this fix"
        );
        let mut intangible = DamageableVolumes::default();
        intangible.clear();
        assert!(
            !strike_hits_body(Some(intangible)),
            "published EMPTY: an authored invulnerable window offers no target at \
             all, and `apply_feature_hit_events` will apply nothing — a predictor \
             that says `hit` here despawns the bolt and fires the hit trace for \
             damage that never lands"
        );
    }
}
