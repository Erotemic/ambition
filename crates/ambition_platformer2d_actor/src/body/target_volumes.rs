//! Publish damageable regions for live bodies.

use super::*;
use ambition_breakables::BreakableFeature;
use ambition_combat::components::{CenteredAabb, DamageableVolumes};

/// Publish current damageable volumes for every ordinary live body.
///
/// Disposition does not affect hittability: player, peaceful NPC, and hostile
/// bodies use the same rule. Publish before world and item contacts, then
/// update the sample after movement for combat.
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
            || policy.is_some_and(|policy| policy.0.tactics.fades
                && ambition_combat::fade::faded(Some(policy), swing.swing().as_ref(), combat))
        {
            DamageableVolumes::clear_if_needed(&mut damageable);
            continue;
        }
        match authored_body_volumes(hurtboxes, kin, unmirrored) {
            Some(volumes) => damageable.replace_body_volumes(volumes),
            None => DamageableVolumes::refresh_box(&mut damageable, kin, aabb),
        }
    }
}

/// A body's authored hurtbox silhouette in world space.
///
/// `None` permits the physical body-box fallback.
/// `Some(vec![])` is an explicit intangible window and must remain empty.
/// Keep the authored shape in canonical body coordinates.
fn authored_body_volumes<'a>(
    hurtboxes: Option<&'a ambition_combat::hurtbox_resolution::ResolvedHurtboxes>,
    kin: Option<&ambition_platformer2d_core::BodyKinematics>,
    unmirrored: bool,
) -> Option<impl Iterator<Item = ambition_platformer2d_core::CombatVolume> + 'a> {
    let (resolved, kin) = hurtboxes.zip(kin)?;
    let side = ambition_platformer2d_core::mirror_side(kin.facing, unmirrored);
    resolved.canonical_volumes(side)
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
        if let Some(volumes) = authored_body_volumes(hurtboxes, kin, feature.unmirrored) {
            damageable.replace_body_volumes(volumes);
            continue;
        }
        let ctx = ambition_boss_encounter::attack_geometry::BossVolumeContext::from_ref(
            boss_catalog,
            boss,
            attack_state,
        )
        .with_animation_frame(animation_frame);
        damageable.publish_body(ambition_combat::body_geometry::canonical_damageable_volumes(&ctx));
    }
}
