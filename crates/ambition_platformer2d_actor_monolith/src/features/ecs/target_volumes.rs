//! Per-frame damageable and pogo-target volume derivation.
//!
//! This module keeps the rule "damageable implies pogoable by default" in body
//! data. Family-specific systems publish current damageable volumes; the generic
//! derivation mirrors them into pogo affordance geometry unless a feature opts
//! out. World collision geometry is a separate explicit contribution and never
//! inferred merely because a body can be struck.

use super::*;

/// Publish the damageable volumes of **every live body**, in one rule.
///
/// Peaceful NPCs and hostile enemies intentionally share this path: both are
/// valid strike targets, and by default both become pogo targets through
/// [`derive_pogo_target_volumes`]. Hostility should affect AI and damage dealt,
/// not whether a body can be hit.
///
/// **So does the player.** This system was gated `With<FeatureSimEntity>` — a
/// marker the primary player does not carry — which meant a player could author
/// hurtboxes and never publish them, and made "hittable" a property of which
/// spawn path built the body. Jon's ruling: *it is a smell that something would
/// work for an enemy but not a player; they should be unified.* The gate is gone,
/// and the predicate for participating is now the honest one — **carrying
/// [`DamageableVolumes`] is what makes a body a damage target**, whatever spawned
/// it.
///
/// # Registered TWICE, deliberately
///
/// The same function runs in two phases, because two consumers need the answer
/// at two different moments and there must not be two *rules*:
///
/// * in `WorldPrep`, so [`derive_pogo_target_volumes`] and the feature-world
///   collision overlay (rebuilt in the same set) see this frame's targets;
/// * again after `PlayerSimulation`, before `Platformer2dSimulationPhaseMonolith::Combat`, so damage
///   resolves against **post-movement** positions. A body's `CenteredAabb` is
///   written by its integrator — the player's in `PlayerSimulation`, an actor's
///   in `WorldPrep` — so publishing only in `WorldPrep` would hand the damage
///   path a player box one frame stale. That is the same defect class as the
///   Mary-O contact bug: a classifier must read the positions the contact pass
///   reads.
///
/// Two invocations of one rule is a refresh. Two rules writing one component is
/// the clobber-by-ordering bug the boss exclusion below exists to prevent — keep
/// it that way.
pub fn refresh_body_damageable_volumes(
    mut bodies: Query<
        (
            &CenteredAabb,
            Option<&ambition_characters::actor::BodyHealth>,
            // §7.10: a body that AUTHORED hurtboxes publishes those instead of its
            // coarse envelope. Exactly the seam the boss path already uses for its
            // head/hand volumes -- an authored silhouette beats a bounding
            // rectangle, and now any character can have one, not just a boss.
            Option<&crate::character_runtime::ResolvedHurtboxes>,
            Option<&crate::actor::BodyKinematics>,
            &mut DamageableVolumes,
        ),
        // Two families that publish their OWN volumes by a different rule, and
        // would be clobbered by the coarse box: a boss's active head/hand
        // hurtboxes (the GNU-ton seam) and a breakable's intact/broken gate.
        // Everything else -- player, enemy, npc, sandbag, a possessed anything --
        // resolves here, through ONE rule.
        (
            Without<super::boss_clusters::BossConfig>,
            Without<BreakableFeature>,
        ),
    >,
) {
    for (aabb, health, hurtboxes, kin, mut damageable) in &mut bodies {
        // Structural tangibility gate (Jon 2026-07-22): a live body — peaceful or
        // hostile — is a valid body-strike / pogo target; a dead one is an
        // intangible corpse and publishes no volume (so you cannot pogo off a
        // corpse). Disposition governs AI and damage dealt TO the player, not
        // whether the player can refresh a downslash from the body.
        if crate::combat::util::body_is_corpse(health) {
            damageable.clear();
            continue;
        }
        match authored_world_volumes(hurtboxes, kin) {
            Some(volumes) => damageable.publish(volumes),
            None => damageable.set_single(aabb.aabb()),
        }
    }
}

/// A body's authored silhouette in world space, if it authored one at all.
///
/// `Some(vec![])` is a real authored answer meaning "invulnerable during this
/// window", so a caller must NOT treat it as "nothing authored" and fall through
/// to a coarse box — that fallthrough would silently delete an authored
/// invulnerability. `None` means no doc, which is what the fallback is for.
///
/// Boxes, and honestly so: the authored hurtbox TIMELINE
/// (`ambition_entity_catalog`'s hurtbox contracts) is authored as rectangles,
/// so this producer publishes `CombatVolume::Aabb` and the cheap overlap path
/// applies. The component can carry hulls — the sprite-metadata producer
/// (`damageable_volumes`) publishes them when a sheet authors a `poly` — and
/// widening the timeline schema to hulls is its own step, not a silent
/// reinterpretation of rows that were written as rects.
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

/// Publish strike-damageable boss volumes from the same authored hurtbox path
/// used by actual boss damage application.
///
/// This is the GNU-ton-critical seam: his coarse spawn/render AABB is a giant
/// composite envelope, but `damageable_volumes` returns the active head/hand
/// hurtboxes from `BossAttackState` + sprite frame metadata. The downstream
/// pogo derivation therefore exposes the thing the player can actually damage,
/// not the composite body's bounding rectangle.
pub fn refresh_boss_damageable_volumes(
    boss_catalog: Res<crate::boss_encounter::BossCatalog>,
    mut bosses: Query<(
        super::boss_clusters::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::brain::BossAttackState,
        Option<&crate::features::BossAnimationFrameSample>,
        // A boss is a character too: if one AUTHORS a `HurtboxDoc`, that doc
        // wins over the frame-sampled parts below. Without this branch a boss
        // was the one family that could not use the authored path, which is
        // backwards — the authored path exists BECAUSE bosses needed it.
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
