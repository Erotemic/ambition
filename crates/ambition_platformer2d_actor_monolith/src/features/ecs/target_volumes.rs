//! Per-frame damageable and pogo-target volume derivation.
//!
//! This module keeps the rule "damageable implies pogoable by default" in body
//! data. Family-specific systems publish current damageable volumes; the generic
//! derivation mirrors them into pogo affordance geometry unless a feature opts
//! out. World collision geometry is a separate explicit contribution and never
//! inferred merely because a body can be struck.

use super::*;

/// Publish current damageable volumes for every ordinary live body.
///
/// Disposition does not affect hittability: player, peaceful NPC, and hostile
/// bodies use the same rule. This system runs in `WorldPrep` for pogo/world
/// consumers and again after player movement so combat sees post-movement
/// geometry. Both phases invoke this one publishing rule.
pub fn refresh_body_damageable_volumes(
    mut bodies: Query<
        (
            &CenteredAabb,
            Option<&ambition_characters::actor::BodyHealth>,
            // Authored hurtboxes override the coarse body envelope.
            Option<&crate::character_runtime::ResolvedHurtboxes>,
            Option<&crate::actor::BodyKinematics>,
            &mut DamageableVolumes,
        ),
        // Bosses and breakables publish their own family-specific volumes.
        (
            Without<ambition_boss_encounter::BossConfig>,
            Without<BreakableFeature>,
        ),
    >,
) {
    for (aabb, health, hurtboxes, kin, mut damageable) in &mut bodies {
        // Dead bodies publish no target volume; disposition does not affect tangibility.
        if ambition_combat::util::body_is_corpse(health) {
            damageable.clear();
            continue;
        }
        match authored_world_volumes(hurtboxes, kin) {
            Some(volumes) => damageable.publish(volumes),
            None => damageable.set_single(aabb.aabb()),
        }
    }
}

/// A body's authored hurtbox silhouette in world space.
///
/// `None` means no authored hurtboxes and permits coarse fallback.
/// `Some(vec![])` is an explicit intangible window and must remain empty. Timeline
/// hurtboxes are authored as rectangles, so this path publishes AABB volumes.
fn authored_world_volumes(
    hurtboxes: Option<&crate::character_runtime::ResolvedHurtboxes>,
    kin: Option<&crate::actor::BodyKinematics>,
) -> Option<Vec<ambition_platformer2d_core::CombatVolume>> {
    let (resolved, kin) = hurtboxes.zip(kin)?;
    let volumes = resolved.world_volumes(kin.aabb().center(), kin.facing)?;
    Some(
        volumes
            .into_iter()
            .map(|v| ambition_platformer2d_core::CombatVolume::aabb(v.aabb()))
            .collect(),
    )
}

/// Publish boss damageable volumes from authored hurtboxes or active boss parts,
/// never the composite body's coarse envelope.
pub fn refresh_boss_damageable_volumes(
    boss_catalog: Res<ambition_boss_encounter::BossCatalog>,
    mut bosses: Query<(
        ambition_boss_encounter::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::brain::BossAttackState,
        Option<&crate::features::BossAnimationFrameSample>,
        // Authored character hurtboxes override boss-part sampling.
        Option<&crate::character_runtime::ResolvedHurtboxes>,
        Option<&crate::actor::BodyKinematics>,
        &mut DamageableVolumes,
    )>,
) {
    for (feature, health, attack_state, animation_frame, hurtboxes, kin, mut damageable) in
        &mut bosses
    {
        let boss = feature.as_boss_ref();
        if !health.alive() {
            damageable.clear();
            continue;
        }
        if let Some(volumes) = authored_world_volumes(hurtboxes, kin) {
            damageable.publish(volumes);
            continue;
        }
        let ctx = crate::features::BossVolumeContext::from_ref(&boss_catalog, boss, attack_state)
            .with_animation_frame(animation_frame);
        damageable.publish(crate::features::damageable_volumes(&ctx));
    }
}

/// Publish damageable breakable volumes for intact hit-reactive breakables.
///
/// Breakable pogo-orbs remain damageable even though their actual damage is
/// resolved by the dedicated `HitSource::Pogo` path. Regular OnHit/Either
/// breakables participate in the default damageable => pogoable rule; pure
/// stand-to-crumble platforms opt into world rebound geometry through
/// `PogoTargetContributor` instead of pretending to be body-damage targets.
pub fn refresh_breakable_damageable_volumes(
    mut breakables: Query<(&CenteredAabb, &BreakableFeature, &mut DamageableVolumes)>,
) {
    for (aabb, feature, mut damageable) in &mut breakables {
        if feature.broken() {
            damageable.clear();
            continue;
        }
        if feature.breakable.trigger.allows_hit() || feature.breakable.pogo_refresh {
            damageable.set_single(aabb.aabb());
        } else {
            damageable.clear();
        }
    }
}

/// Derive pogo target volumes from damageable volumes by default.
///
/// `PogoPolicy::Custom` is deliberately a no-op so another system can own the
/// current `PogoTargetVolumes` for a feature family without fighting this
/// generic derivation pass.
pub fn derive_pogo_target_volumes(
    mut targets: Query<(&DamageableVolumes, &PogoPolicy, &mut PogoTargetVolumes)>,
) {
    for (damageable, policy, mut pogo) in &mut targets {
        match *policy {
            PogoPolicy::FromDamageable => {
                // Coarsened ON PURPOSE for pogo affordance geometry. Damage keeps
                // the original `CombatVolume`s; this projection is for proximity
                // and for features that explicitly opt into world-surface boxes.
                pogo.volumes.clear();
                pogo.volumes.extend(damageable.bounds());
            }
            PogoPolicy::Custom => {}
            PogoPolicy::Disabled => pogo.volumes.clear(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{App, Update};

    #[test]
    fn derive_pogo_target_volumes_copies_damageable_by_default() {
        let mut app = App::new();
        let aabb = ae::Aabb::new(ae::Vec2::new(4.0, 5.0), ae::Vec2::new(2.0, 3.0));
        app.world_mut().spawn((
            DamageableVolumes::single(aabb),
            PogoPolicy::FromDamageable,
            PogoTargetVolumes::default(),
        ));
        app.add_systems(Update, derive_pogo_target_volumes);
        app.update();

        let mut q = app.world_mut().query::<&PogoTargetVolumes>();
        let pogo = q.single(app.world()).expect("one pogo target");
        assert_eq!(pogo.volumes, vec![aabb]);
    }

    #[test]
    fn derive_pogo_target_volumes_respects_disabled_policy() {
        let mut app = App::new();
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(8.0, 8.0));
        app.world_mut().spawn((
            DamageableVolumes::single(aabb),
            PogoPolicy::Disabled,
            PogoTargetVolumes {
                volumes: vec![aabb],
            },
        ));
        app.add_systems(Update, derive_pogo_target_volumes);
        app.update();

        let mut q = app.world_mut().query::<&PogoTargetVolumes>();
        let pogo = q.single(app.world()).expect("one pogo target");
        assert!(pogo.volumes.is_empty());
    }
}
