//! Per-frame damageable and pogo-target volume derivation.
//!
//! This module keeps the rule "damageable implies pogoable by default" in body
//! data. Family-specific systems publish current damageable volumes; the generic
//! derivation mirrors them into pogo affordance geometry unless a feature opts
//! out. World collision geometry is a separate explicit contribution and never
//! inferred merely because a body can be struck.

use super::*;
use ambition_breakables::BreakableFeature;
use ambition_combat::components::{CenteredAabb, DamageableVolumes, PogoPolicy, PogoTargetVolumes};

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
            (
                bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
                Option<&ambition_platformer2d_core::DepthPlane>,
                bevy::prelude::Has<ambition_platformer2d_core::Unmirrored>,
            ),
            // Authored hurtboxes override the coarse body envelope.
            Option<&ambition_combat::hurtbox_resolution::ResolvedHurtboxes>,
            Option<&ambition_platformer2d_core::BodyKinematics>,
            &mut DamageableVolumes,
            // A body that fades between its attacks (`ambition_combat::fade`).
            (
                Option<&ambition_combat::actor_tuning::ActorPolicy>,
                ambition_combat::moveset::MeleeSwingQuery,
                Option<&ambition_characters::actor::BodyCombat>,
            ),
        ),
        // Bosses and breakables publish their own family-specific volumes.
        (
            Without<ambition_boss_encounter::BossConfig>,
            Without<BreakableFeature>,
        ),
    >,
) {
    for (aabb, health, (out_of_play, plane, unmirrored), hurtboxes, kin, mut damageable, (policy, swing, combat),
    ) in &mut bodies {
        // Dead, out-of-play and backdrop bodies publish no target volume;
        // disposition does not affect tangibility. Nor does a faded body.
        if ambition_combat::util::body_is_untouchable(health, out_of_play, plane)
            || ambition_combat::fade::faded(policy, swing.swing().as_ref(), combat)
        {
            damageable.clear();
            continue;
        }
        match authored_world_volumes(hurtboxes, kin, unmirrored) {
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
    hurtboxes: Option<&ambition_combat::hurtbox_resolution::ResolvedHurtboxes>,
    kin: Option<&ambition_platformer2d_core::BodyKinematics>,
    unmirrored: bool,
) -> Option<Vec<ambition_platformer2d_core::CombatVolume>> {
    let (resolved, kin) = hurtboxes.zip(kin)?;
    let side = ambition_platformer2d_core::mirror_side(kin.facing, unmirrored);
    let volumes = resolved.world_volumes(kin.aabb().center(), side)?;
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
    // The boss catalog of the running session, and not that of the App: a
    // reload publishes the App's catalog before its session is activated, and
    // a resimulated frame must not read it. `None`: no session runs, see
    // `SessionMechanics`.
    generation: Option<Res<ambition_platformer2d_construction::mechanics::SessionMechanics>>,
    mut bosses: Query<(
        ambition_boss_encounter::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &ambition_characters::brain::BossAttackState,
        Option<&ambition_boss_encounter::attack_geometry::BossAnimationFrameSample>,
        // Authored character hurtboxes override boss-part sampling.
        Option<&ambition_combat::hurtbox_resolution::ResolvedHurtboxes>,
        Option<&ambition_platformer2d_core::BodyKinematics>,
        &mut DamageableVolumes,
        (
            bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
            Option<&ambition_platformer2d_core::DepthPlane>,
        ),
    )>,
) {
    let Some(generation) = generation else {
        return;
    };
    let boss_catalog = &generation.bosses;
    for (
        feature,
        health,
        attack_state,
        animation_frame,
        hurtboxes,
        kin,
        mut damageable,
        (out_of_play, plane),
    ) in &mut bosses
    {
        let boss = feature.as_boss_ref();
        // The same participation rule every body answers: a boss out of play or
        // behind the playable plane publishes nothing, not only a dead one.
        if ambition_combat::util::body_is_untouchable(Some(health), out_of_play, plane) {
            damageable.clear();
            continue;
        }
        // A successor that sleeps until its predecessor falls is not in the
        // fight yet: it publishes nothing, so blows and shots go through it.
        // The sleeping Inugami lies in Wolfenstein's body, and each shot at
        // him stopped on it and was swallowed (Jon, 2026-10-21: "the robot's
        // projectile doesn't seem to do damage anymore").
        if boss.config.wakes_after.is_some()
            && boss.status.encounter_phase() == ambition_boss_encounter::BossEncounterPhase::Dormant
        {
            damageable.clear();
            continue;
        }
        if let Some(volumes) = authored_world_volumes(hurtboxes, kin, feature.unmirrored) {
            damageable.publish(volumes);
            continue;
        }
        let ctx = ambition_boss_encounter::attack_geometry::BossVolumeContext::from_ref(
            boss_catalog,
            boss,
            attack_state,
        )
        .with_animation_frame(animation_frame);
        damageable.publish(ambition_combat::body_geometry::damageable_volumes(&ctx));
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
