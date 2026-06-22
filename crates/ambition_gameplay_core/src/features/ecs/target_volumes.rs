//! Per-frame damageable and pogo-target volume derivation.
//!
//! This module keeps the gameplay rule "damageable implies pogoable by
//! default" in ECS data instead of burying it in the sandbox-world overlay.
//! Family-specific systems publish their current damageable volumes; the
//! generic derivation system then mirrors them into pogo target volumes unless
//! a feature explicitly opts out.

use super::*;

/// Publish player-damageable actor volumes for all live actors.
///
/// Peaceful NPCs and hostile enemies intentionally share this path: both are
/// valid player-strike targets, and by default both become pogo targets through
/// [`derive_pogo_target_volumes`]. Hostility should affect AI and damage dealt
/// to the player, not whether the player can refresh a downslash from the
/// actor's body.
pub fn refresh_actor_damageable_volumes(
    mut actors: Query<
        (
            &CenteredAabb,
            &ActorDisposition,
            Option<&super::enemy_clusters::EnemyStatus>,
            &mut DamageableVolumes,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for (aabb, disposition, status, mut damageable) in &mut actors {
        // Peaceful actors are always a valid player-strike target; hostile actors
        // only while alive.
        if disposition.is_peaceful() || status.is_some_and(|s| s.alive) {
            damageable.set_single(aabb.aabb());
        } else {
            damageable.clear();
        }
    }
}

/// Publish player-damageable boss volumes from the same authored hurtbox path
/// used by actual boss damage application.
///
/// This is the GNU-ton-critical seam: his coarse spawn/render AABB is a giant
/// composite envelope, but `damageable_volumes` returns the active head/hand
/// hurtboxes from `BossAttackState` + sprite frame metadata. The downstream
/// pogo derivation therefore exposes the thing the player can actually damage,
/// not the composite body's bounding rectangle.
pub fn refresh_boss_damageable_volumes(
    mut bosses: Query<
        (
            super::boss_clusters::BossClusterRef,
            &crate::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
            &mut DamageableVolumes,
        ),
        With<FeatureSimEntity>,
    >,
) {
    for (feature, attack_state, animation_frame, mut damageable) in &mut bosses {
        let boss = feature.as_boss_ref();
        if !boss.status.alive {
            damageable.clear();
            continue;
        }
        let ctx = crate::features::BossVolumeContext::from_ref(boss, attack_state)
            .with_animation_frame(animation_frame);
        damageable.volumes = crate::features::damageable_volumes(&ctx);
    }
}

/// Publish damageable breakable volumes for intact hit-reactive breakables.
///
/// Breakable pogo-orbs remain damageable even though their actual damage is
/// resolved by the dedicated `HitSource::PogoBounce` path. Regular OnHit/Either
/// breakables participate in the default damageable => pogoable rule; pure
/// stand-to-crumble platforms keep their legacy `PogoTargetContributor` marker
/// instead of pretending to be player-damage targets.
pub fn refresh_breakable_damageable_volumes(
    mut breakables: Query<
        (&CenteredAabb, &BreakableFeature, &mut DamageableVolumes),
        With<FeatureSimEntity>,
    >,
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
                pogo.volumes.clear();
                pogo.volumes.extend(damageable.volumes.iter().copied());
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
            DamageableVolumes {
                volumes: vec![aabb],
            },
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
            DamageableVolumes {
                volumes: vec![aabb],
            },
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
