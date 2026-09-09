//! Read-only preflight hit predicates for projectile/attack feedback.
//!
//! A positive preflight may terminate a strike, so each predicate must match the
//! tangibility gate used by the corresponding damage applier. Actors and BOSSES
//! both read `DamageableVolumes`; breakables mirror their broken/trigger/pogo
//! gates. Actor and breakable precision remains coarse-AABB by current gameplay
//! policy; a boss reads the published parts, which is part-level precision
//! without a second derivation.
//!
//! ⛔⛔ **A2a: THE BOSS ARM USED TO DERIVE ITS OWN GEOMETRY.** It built a
//! `BossVolumeContext` from the catalog and the live attack/animation state —
//! the same derivation `apply_boss_hit` did twice more — so one fact had three
//! authorities and the publisher's authored-hurtbox override reached none of
//! them. It reads the publication now; the publication moved into the window
//! after `Playback` where those live values are settled.

use ambition_platformer2d_core::AabbExt;
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
        breakable_is_eligible(&event.ignored_targets, id, feature)
            && event.volume.intersects_aabb(aabb.aabb())
    })
}

/// Whether this breakable can receive a projectile/attack contact at all.
///
/// One eligibility rule, read by the discrete predicate above and the swept one
/// below — the two used to spell the same four conditions separately, which is
/// how a swept road comes to admit a target the discrete road refuses.
fn breakable_is_eligible(
    ignored_targets: &[String],
    id: &FeatureId,
    feature: &BreakableFeature,
) -> bool {
    !target_is_ignored(ignored_targets, "breakable", id.as_str())
        && !feature.broken()
        && feature.breakable.trigger.allows_hit()
        && !feature.breakable.pogo_refresh
}

/// The earliest point along a projectile's travel at which it reaches an
/// eligible breakable, with that breakable's centre.
///
/// ⛔⛔ **THE SWEPT SIBLING, AND THE DISCRETE ONE IS NOT A SUBSTITUTE.** A shot
/// crossing a crate between two samples has no endpoint that overlaps it, so the
/// discrete predicate says the crate was never touched — the same defect the
/// ordinary body branch had. The centre travels with the answer because the
/// caller needs it: `time_of_impact` leaves the shot's box TANGENT, and every
/// downstream overlap test is `strict_intersects`.
pub fn projectile_reaches_breakable(
    start: ambition_platformer2d_core::Vec2,
    half: ambition_platformer2d_core::Vec2,
    delta: ambition_platformer2d_core::Vec2,
    ignored_targets: &[String],
    breakables: &Query<(&FeatureId, &CenteredAabb, &BreakableFeature), With<FeatureSimEntity>>,
) -> Option<(f32, ambition_platformer2d_core::Vec2)> {
    breakables
        .iter()
        .filter(|(id, _, feature)| breakable_is_eligible(ignored_targets, id, feature))
        .filter_map(|(_, aabb, _)| {
            swept_box_reaches(start, half, delta, aabb.aabb())
                .map(|time| (time, aabb.aabb().center()))
        })
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
}

/// Earliest normalized time in `[0, 1]` at which a box swept from `start` by
/// `delta` reaches `target`, with parity for the already-overlapping case.
fn swept_box_reaches(
    start: ambition_platformer2d_core::Vec2,
    half: ambition_platformer2d_core::Vec2,
    delta: ambition_platformer2d_core::Vec2,
    target: ambition_platformer2d_core::Aabb,
) -> Option<f32> {
    let start_box = ambition_platformer2d_core::Aabb::new(start, half);
    if start_box.strict_intersects(target) {
        return Some(0.0);
    }
    start_box.sweep_hit(delta, target).map(|hit| hit.time_of_impact)
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
    event: &HitEvent,
    bosses: &Query<
        (
            &FeatureId,
            &CenteredAabb,
            &ambition_characters::actor::BodyHealth,
            &DamageableVolumes,
        ),
        (With<FeatureSimEntity>, With<BossConfig>),
    >,
) -> bool {
    // ⛔⛔ **IT READS THE PUBLICATION NOW, AND THE RECOMPUTATION IT REPLACED WAS
    // THE DEFECT.** This used to build a `BossVolumeContext` from the catalog,
    // the live `BossAttackState` and the live `BossAnimationFrameSample` and
    // derive the parts itself — one of THREE places deriving one fact. None of
    // the three consulted `refresh_boss_damageable_volumes`, so a boss with
    // authored hurtboxes was struck on its generated hull, and an authored
    // EMPTY override (an invulnerable window) offered a target anyway.
    //
    // ⭐ The reason it could not read the publication before is that the
    // publication ran in `WorldPrep`, a phase ahead of the boss brain, and
    // described the previous frame. It is republished after `Playback` now, in
    // the window this recomputation was reaching for.
    //
    // ⚠ NOT-YET-PUBLISHED MEANS NO CONTACT, deliberately. A boss has no coarse
    // fallback — `refresh_boss_damageable_volumes` never publishes the composite
    // envelope — so an unpublished boss is one whose geometry nobody has spoken
    // for, and inventing a hull for it is exactly the first-frame coarse hurtbox
    // the contract forbids.
    bosses.iter().any(|(id, _aabb, health, damageable)| {
        !target_is_ignored(&event.ignored_targets, "boss", id.as_str())
            && health.alive()
            && damageable.published()
            && damageable
                .volumes
                .iter()
                .any(|part| event.volume.intersects(part))
    })
}

/// The earliest point along a projectile's travel at which it reaches a live
/// boss's published geometry, with the part's centre.
///
/// The swept sibling of [`ecs_hit_event_hits_boss`], reusing the ONE swept
/// victim-geometry rule so a boss and an ordinary body answer the same way about
/// published parts, coarse fallback (none, for a boss) and intangibility.
pub fn projectile_reaches_boss(
    start: ambition_platformer2d_core::Vec2,
    half: ambition_platformer2d_core::Vec2,
    delta: ambition_platformer2d_core::Vec2,
    ignored_targets: &[String],
    bosses: &Query<
        (
            &FeatureId,
            &CenteredAabb,
            &ambition_characters::actor::BodyHealth,
            &DamageableVolumes,
        ),
        (With<FeatureSimEntity>, With<BossConfig>),
    >,
) -> Option<(f32, ambition_platformer2d_core::Vec2)> {
    bosses
        .iter()
        .filter(|(id, _, health, damageable)| {
            !target_is_ignored(ignored_targets, "boss", id.as_str())
                && health.alive()
                && damageable.published()
        })
        .filter_map(|(_, aabb, _, damageable)| {
            ambition_combat::hitbox::swept_strike_reaches_victim(
                start,
                half,
                delta,
                Some(damageable),
                aabb,
            )
            .map(|time| (time, aabb.aabb().center()))
        })
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_boss_encounter::behavior::BossBehaviorProfileExt;
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

    /// One live boss carrying `volumes`, asked the question the projectile
    /// stepper asks before it despawns a bolt.
    fn strike_hits_boss(volumes: DamageableVolumes) -> bool {
        let mut world = World::new();
        world.spawn((
            FeatureSimEntity,
            FeatureId::new("gnu_ton"),
            CenteredAabb::new(BODY_CENTER, BODY_HALF),
            // The predicate reads no field of this; `BossConfig` is here as the
            // query FILTER that separates a boss from an ordinary body. The
            // profile comes from an empty catalog's generic fallback so the
            // fixture states no boss content of its own.
            BossConfig {
                id: "gnu_ton".into(),
                name: "GNU-ton".into(),
                spawn: Vec2::ZERO,
                brain: ambition_entity_catalog::placements::BossBrain::Dormant,
                behavior: ambition_boss_encounter::pattern::profile::BossBehaviorProfile::generic(
                    ambition_boss_encounter::test_boss_catalog(),
                    "gnu_ton",
                ),
            },
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(9)),
            volumes,
        ));
        let event = strike_event();
        world
            .run_system_once(
                move |bosses: Query<
                    (
                        &FeatureId,
                        &CenteredAabb,
                        &ambition_characters::actor::BodyHealth,
                        &DamageableVolumes,
                    ),
                    (With<FeatureSimEntity>, With<BossConfig>),
                >| { ecs_hit_event_hits_boss(&event, &bosses) },
            )
            .expect("the boss hit predicate ran")
    }

    /// ⛔⛔ **A BOSS IS HIT WHERE ITS PUBLISHER SAYS, AND NOWHERE ELSE.**
    ///
    /// Before A2a this predicate derived the geometry itself from the catalog and
    /// the live attack/animation state — one of three derivations of one fact —
    /// so `refresh_boss_damageable_volumes`'s authored-hurtbox override reached
    /// none of them and a published-EMPTY boss still offered a target.
    ///
    /// ⚠ THE UNPUBLISHED ROW IS THE OPPOSITE OF THE BODY'S, deliberately. An
    /// ordinary body falls back to its coarse envelope; a boss has no fallback at
    /// all — `refresh_boss_damageable_volumes` never publishes the composite
    /// envelope, because a multi-part boss's gross box covers metres of creature
    /// that cannot be hurt. So "nobody has spoken for this boss yet" is NO
    /// CONTACT, which is also what the contact protocol requires of a
    /// not-yet-ready authored publisher: it may not be an excuse to expose an
    /// unintended coarse hurtbox for one frame.
    #[test]
    fn the_boss_hit_test_answers_only_from_the_published_volumes() {
        assert!(
            !strike_hits_boss(DamageableVolumes::default()),
            "unpublished: no publisher has spoken for this boss, and a boss has no \
             coarse fallback. Answering `hit` here invents a hull nobody authored \
             — and on the first eligible tick, which is exactly the frame the \
             contact protocol forbids it on"
        );
        let mut intangible = DamageableVolumes::default();
        intangible.clear();
        assert!(
            !strike_hits_boss(intangible),
            "published EMPTY: an authored invulnerable window offers no target at \
             all. This was the live defect — the derivation ignored the \
             publication, so an intangible boss was still struck on its generated \
             hull"
        );
        assert!(
            !strike_hits_boss(DamageableVolumes::single(published_head())),
            "published a silhouette DISJOINT from the strike: a boss answers from \
             its published parts, not from its coarse envelope, so a strike inside \
             the envelope and outside every part is a miss"
        );
        assert!(
            strike_hits_boss(DamageableVolumes::single(ae_strike_box())),
            "published a part the strike overlaps: the predicate must still say \
             HIT, or the three rows above are satisfied by a predicate that \
             refuses everything"
        );
    }

    /// The strike's own volume as a published part, for the anti-vacuity row.
    fn ae_strike_box() -> Aabb {
        Aabb::new(Vec2::new(90.0, 84.0), Vec2::new(4.0, 4.0))
    }
}
