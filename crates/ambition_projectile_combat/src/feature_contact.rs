//! Sampled hurt-material contacts for bosses and breakables.

use ambition_breakables::BreakableFeature;
use ambition_platformer2d_core::AabbExt;
use bevy::prelude::{Entity, Query, With};

use ambition_boss_encounter::BossConfig;

use ambition_combat::components::{CenteredAabb, DamageableVolumes, FeatureId};
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;

/// Whether this breakable can receive a projectile contact at all.
fn breakable_is_eligible(feature: &BreakableFeature) -> bool {
    !feature.broken()
        && feature.state().trigger.allows_hit()
        && !feature.state().pogo_refresh
}

/// Select the first eligible breakable contact from the physical body sweep.
pub fn projectile_reaches_breakable(
    source: &ambition_platformer2d_core::BodyKinematics,
    delta: ambition_platformer2d_core::Vec2,
    geometry: Option<&ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    may_touch: impl Fn(Entity) -> bool,
    breakables: &Query<
        (Entity, &FeatureId, &CenteredAabb, &BreakableFeature),
        With<FeatureSimEntity>,
    >,
) -> Option<FeatureContact> {
    breakables
        .iter()
        .filter(|(entity, _, _, feature)| may_touch(*entity) && breakable_is_eligible(feature))
        .filter_map(|(entity, id, aabb, _)| {
            DamageableVolumes::world_box(aabb.aabb())
                .swept_body_contact(entity, None, geometry, room, source, delta).ok().flatten()
                .map(|hit| FeatureContact {
                contact: hit,
                target_center: aabb.aabb().center(),
                target: entity,
                target_id: id.as_str().to_string(),
            })
        })
        .min_by(FeatureContact::order_for_caller)
}

/// A fixed recipient and its accepted physical contact.
#[derive(Clone, Debug)]
pub struct FeatureContact {
    /// The native contact, including time, physical point and target path.
    pub contact: ambition_combat::components::BodySweepContact,
    /// The target's root position for deterministic tie ordering.
    pub target_center: ambition_platformer2d_core::Vec2,
    /// Which feature was reached.
    pub target: Entity,
    /// The authored identity, for breaking an exact tie.
    ///
    /// ⛔⛔ **A TIE MAY NOT BE DECIDED BY QUERY ORDER.** Two features a shot
    /// reaches at the same instant — overlapping crates, two parts of one
    /// encounter — compare equal on time, and `min_by` over an equal key hands
    /// the answer back to archetype order, which a rollback resimulation does
    /// not reproduce. The body branch already tie-breaks on position and then on
    /// `SimId` for exactly this reason; an entity index is not stable across a
    /// rewind and an authored id is.
    pub target_id: String,
}

impl FeatureContact {
    /// The total deterministic order the protocol specifies: time of impact,
    /// then the target's position, then its authored identity.
    pub fn order_for_caller(&self, other: &Self) -> std::cmp::Ordering {
        self.contact.hit.toi
            .total_cmp(&other.contact.hit.toi)
            .then(self.target_center.x.total_cmp(&other.target_center.x))
            .then(self.target_center.y.total_cmp(&other.target_center.y))
            .then_with(|| self.target_id.cmp(&other.target_id))
    }
}

/// Select the first live boss contact from its published hurt material.
pub fn projectile_reaches_boss(
    source: &ambition_platformer2d_core::BodyKinematics,
    delta: ambition_platformer2d_core::Vec2,
    geometry: Option<&ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    may_touch: impl Fn(Entity) -> bool,
    bosses: &Query<
        (
            Entity,
            &FeatureId,
            &CenteredAabb,
            &ambition_characters::actor::BodyHealth,
            &DamageableVolumes,
            Option<&ambition_platformer2d_core::BodyKinematics>,
        ),
        (With<FeatureSimEntity>, With<BossConfig>),
    >,
) -> Option<FeatureContact> {
    bosses
        .iter()
        .filter(|(entity, _, _, health, damageable, _)| {
            may_touch(*entity)
                && health.alive()
                && damageable.published()
        })
        .filter_map(|(entity, id, aabb, _, damageable, kin)| {
            damageable.swept_body_contact(entity, kin, geometry, room, source, delta).ok().flatten()
            .map(|hit| FeatureContact {
                contact: hit,
                target_center: aabb.aabb().center(),
                target: entity,
                target_id: id.as_str().to_string(),
            })
        })
        .min_by(FeatureContact::order_for_caller)
}
#[cfg(test)]
mod tests {
    use super::*;
    use ambition_boss_encounter::behavior::BossBehaviorProfileExt;
    use ambition_combat::components::DamageableVolumes;
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
    fn strike_box() -> Aabb {
        Aabb::new(Vec2::new(90.0, 84.0), Vec2::new(4.0, 4.0))
    }

    /// ⛔⛔ **ABSENT AND EMPTY MEAN OPPOSITE THINGS, and this is the rule every
    /// damage family owes whatever its geometry.**
    ///
    /// ⚠ IT MOVED HERE FROM A PREDICATE THAT HAD NO PRODUCTION CALLER.
    /// `ecs_hit_event_hits_actor` was reachable, tested, and unreached — so the
    /// four states were pinned against a function nobody ran. They are asked of
    /// `strike_reaches_victim` now, which is what every consumer of the rule
    /// actually calls, including the swept sibling in this module.
    ///
    /// All four states are pinned because the point is a RULE, not a patch:
    /// absent and unpublished must keep falling back to the coarse box
    /// (requiring the component, or reading an unpublished empty list as
    /// intangible, would silently turn a hit test into a no-op), published
    /// non-empty answers from the SILHOUETTE, and published-empty is an authored
    /// invulnerable window that offers no target at all.
    #[test]
    fn absent_unpublished_published_and_intangible_are_four_different_answers() {
        use ambition_combat::hitbox::strike_reaches_victim;
        let strike = ambition_platformer2d_core::CombatVolume::aabb(strike_box());
        let coarse = CenteredAabb::new(BODY_CENTER, BODY_HALF);

        assert!(
            strike_reaches_victim(&strike, None, &coarse, None),
            "no component: the coarse box is the only available answer"
        );
        assert!(
            strike_reaches_victim(&strike, Some(&DamageableVolumes::default()), &coarse, None),
            "unpublished: no publisher has spoken for this body yet, so the coarse \
             box still answers — an empty list is not yet an authored `nowhere`"
        );
        assert!(
            !strike_reaches_victim(
                &strike,
                Some(&DamageableVolumes::world_box(published_head())),
                &coarse,
                None,
            ),
            "published a silhouette DISJOINT from the strike: a body that has \
             spoken answers from its parts, not from the envelope it happens to \
             sit in"
        );
        let mut intangible = DamageableVolumes::default();
        intangible.clear();
        assert!(
            !strike_reaches_victim(&strike, Some(&intangible), &coarse, None),
            "published EMPTY: an authored invulnerable window offers no target at \
             all, and a road that says `hit` here despawns the bolt and fires the \
             hit trace for damage that never lands"
        );
    }

    /// One live boss carrying `volumes`, asked where along a shot's travel it is
    /// first reached.
    fn shot_reaches_boss(volumes: DamageableVolumes, delta: Vec2) -> bool {
        let mut world = World::new();
        world.spawn((
            FeatureSimEntity,
            FeatureId::new("gnu_ton"),
            CenteredAabb::new(BODY_CENTER, BODY_HALF),
            // The predicate reads no field of this; `BossConfig` is the query
            // FILTER that separates a boss from an ordinary body. The profile is
            // the shared test catalog's generic fallback, so the fixture states
            // no boss content of its own.
            BossConfig {
                id: "gnu_ton".into(),
                name: "GNU-ton".into(),
                spawn: Vec2::ZERO,
                practice: false,
                wakes_after: None,
                brain: ambition_entity_catalog::placements::BossBrain::Dormant,
                behavior: ambition_boss_encounter::pattern::profile::BossBehaviorProfile::generic(
                    ambition_boss_encounter::test_boss_catalog(),
                    "gnu_ton",
                ),
                seed: ambition_boss_encounter::BossSeed::resolved(ambition_boss_encounter::test_boss_catalog(), "gnu_ton", "GNU-ton", 18,
                ),
            },
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(9)),
            volumes,
        ));
        let start = strike_box().center();
        let half = strike_box().half_size();
        world
            .run_system_once(
                move |bosses: Query<
                    (
                        Entity,
                        &FeatureId,
                        &CenteredAabb,
                        &ambition_characters::actor::BodyHealth,
                        &DamageableVolumes,
                        Option<&ambition_platformer2d_core::BodyKinematics>,
                    ),
                    (With<FeatureSimEntity>, With<BossConfig>),
                >| {
                    projectile_reaches_boss(&ambition_platformer2d_core::BodyKinematics {
                        pos: start, canonical_size: half * 2.0, ..Default::default()
                    }, delta, None, None, |_| true, &bosses).is_some()
                },
            )
            .expect("the boss contact ran")
    }

    /// ⛔⛔ **A BOSS IS REACHED WHERE ITS PUBLISHER SAYS, AND NOWHERE ELSE.**
    ///
    /// Before A2a the damage road derived boss geometry from the catalog and the
    /// live attack/animation state — one of three derivations of one fact — so
    /// `refresh_boss_damageable_volumes`'s authored-hurtbox override reached none
    /// of them and a published-EMPTY boss still offered a target.
    ///
    /// ⚠ THE UNPUBLISHED ROW IS THE OPPOSITE OF AN ORDINARY BODY'S, deliberately.
    /// A body falls back to its coarse envelope; a boss has no fallback at all,
    /// because a multi-part boss's gross box covers metres of creature that
    /// cannot be hurt. "Nobody has spoken for this boss yet" is NO CONTACT, which
    /// is also what the contact protocol requires of a not-yet-ready authored
    /// publisher: it may not be an excuse to expose an unintended coarse hurtbox
    /// for one frame.
    #[test]
    fn a_boss_is_reached_only_through_its_published_volumes() {
        let still = Vec2::ZERO;
        assert!(
            !shot_reaches_boss(DamageableVolumes::default(), still),
            "unpublished: no publisher has spoken for this boss, and a boss has no \
             coarse fallback. Answering `hit` invents a hull nobody authored — on \
             the first eligible tick, which is exactly the frame the contact \
             protocol forbids it on"
        );
        let mut intangible = DamageableVolumes::default();
        intangible.clear();
        assert!(
            !shot_reaches_boss(intangible, still),
            "published EMPTY: an authored invulnerable window offers no target at \
             all"
        );
        assert!(
            !shot_reaches_boss(DamageableVolumes::world_box(published_head()), still),
            "published a silhouette DISJOINT from the shot: a boss answers from \
             its published parts, not from its coarse envelope"
        );
        assert!(
            shot_reaches_boss(DamageableVolumes::world_box(strike_box()), still),
            "published a part the shot overlaps: the contact must still be found, \
             or the three rows above are satisfied by a road that refuses \
             everything"
        );
        // ⭐ AND IT IS SWEPT, which the discrete predicate this replaces was not.
        // A part 40 px away is missed by an overlap test and reached by a leg that
        // crosses it.
        let far = Aabb::new(strike_box().center() + Vec2::new(40.0, 0.0), Vec2::splat(4.0),
        );
        assert!(
            !shot_reaches_boss(DamageableVolumes::world_box(far), still),
            "a stationary shot reached a part 40 px away"
        );
        assert!(
            shot_reaches_boss(DamageableVolumes::world_box(far), Vec2::new(64.0, 0.0)),
            "a shot whose leg crosses the part did not reach it, so this road is \
             still an endpoint test wearing a swept signature"
        );
    }
}
