//! Body-generic combat geometry for observers.
//!
//! This read-model answers the two questions a combat debugger needs without
//! asking who controls a body: **where can this body be struck?** and **where
//! are live strikes right now?**  The extraction mirrors the combat resolver's
//! geometry rule: an unpublished/missing `DamageableVolumes` falls back to the
//! coarse body box, a published empty list is intangible, and a published list
//! is used verbatim.

use ambition_combat::components::{CenteredAabb, DamageableVolumes};
use ambition_combat::hitbox::{Hitbox, HitboxAnchor};
use ambition_platformer2d_core as ae;
use bevy::prelude::{Query, ResMut, Resource, With};

/// One combat body's collision envelope and effective damageable silhouette.
#[derive(Clone, Debug, PartialEq)]
pub struct CombatBodyGeometryView {
    pub collision: ae::Aabb,
    pub hurtboxes: Vec<ae::CombatVolume>,
}

/// Exact live strike geometry, already resolved into world space.
#[derive(Clone, Debug, PartialEq)]
pub struct CombatStrikeGeometryView {
    pub volume: ae::CombatVolume,
    /// The live strike entity, so an observer can tie a per-strike visual to
    /// the volume that owns it and retire the visual when the strike ends.
    ///
    /// ⭐ **an identity, not a handle to reach back through.** An observer may
    /// compare it and key on it; ⛔ it must not use it to `get::<Hitbox>()` and
    /// read the authoritative component, which is the coupling this row exists
    /// to remove.
    pub strike: bevy::prelude::Entity,
    /// The body whose strike this is.
    pub owner: bevy::prelude::Entity,
    /// Is this strike anchored to its owner's BODY (a character's move), as
    /// opposed to fixed in the world (an arena hazard, a wielded AOE)?
    ///
    /// The distinction presentation actually needs: only a body-tracking strike
    /// stands in for somebody's attack.
    pub anchored_to_body: bool,
    /// **The owner position `volume` was resolved against.**
    ///
    /// Presentation draws bodies at the PRESENTED pose, not the simulated one,
    /// and a stand-in placed on the sim pose shudders against the body it is
    /// supposed to be attached to. Publishing the anchor lets an observer
    /// re-place the same geometry at the drawn pose with one translation —
    /// `presented - owner_anchor` — instead of reaching back into the
    /// authoritative `Hitbox` to recompute it.
    pub owner_anchor: ae::Vec2,
}

/// Presentation-facing snapshot of authoritative combat geometry.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct CombatGeometryView {
    pub bodies: Vec<CombatBodyGeometryView>,
    pub strikes: Vec<CombatStrikeGeometryView>,
}

fn effective_hurtboxes(
    collision: ae::Aabb,
    damageable: Option<&DamageableVolumes>,
) -> Vec<ae::CombatVolume> {
    match damageable {
        Some(published) if published.intangible() => Vec::new(),
        Some(published) if published.published() => published.volumes.clone(),
        _ => vec![ae::CombatVolume::aabb(collision)],
    }
}

/// Rebuild the combat-geometry observation from current simulation truth.
///
/// `BodyCombat` is the participation predicate: human-controlled fighters,
/// brain-controlled fighters, possessed bodies, and bosses all qualify through
/// the same component. No `PrimaryPlayerOnly`, controller, or faction marker is
/// consulted.
pub fn rebuild_combat_geometry_view(
    bodies: Query<
        (&CenteredAabb, Option<&DamageableVolumes>),
        With<ambition_characters::actor::BodyCombat>,
    >,
    hitboxes: Query<(bevy::prelude::Entity, &Hitbox)>,
    owner_boxes: Query<&CenteredAabb>,
    owner_kinematics: Query<&ae::BodyKinematics>,
    mut view: ResMut<CombatGeometryView>,
) {
    view.bodies.clear();
    view.strikes.clear();

    for (aabb, damageable) in &bodies {
        let collision = aabb.aabb();
        view.bodies.push(CombatBodyGeometryView {
            collision,
            hurtboxes: effective_hurtboxes(collision, damageable),
        });
    }

    for (strike, hitbox) in &hitboxes {
        let owner_pos = match hitbox.anchor {
            HitboxAnchor::World { .. } => Some(ae::Vec2::ZERO),
            HitboxAnchor::FollowOwner { .. } => owner_boxes
                .get(hitbox.owner)
                .map(|aabb| aabb.center)
                .or_else(|_| owner_kinematics.get(hitbox.owner).map(|kin| kin.pos))
                .ok(),
        };
        let Some(owner_pos) = owner_pos else {
            continue;
        };
        view.strikes.push(CombatStrikeGeometryView {
            volume: hitbox.world_volume(owner_pos),
            strike,
            owner: hitbox.owner,
            anchored_to_body: matches!(hitbox.anchor, HitboxAnchor::FollowOwner { .. }),
            owner_anchor: owner_pos,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat::hitbox::{HitSide, HitboxKnockback};
    use ambition_platformer2d_core::AabbExt;
    use bevy::prelude::*;

    #[test]
    fn combat_geometry_needs_no_privileged_primary_body() {
        let mut app = App::new();
        app.init_resource::<CombatGeometryView>();
        app.add_systems(Update, rebuild_combat_geometry_view);

        let body_center = ae::Vec2::new(120.0, 80.0);
        let collision = ae::Aabb::new(body_center, ae::Vec2::new(12.0, 18.0));
        let authored_hurt = ae::Aabb::new(
            body_center + ae::Vec2::new(2.0, -3.0),
            ae::Vec2::new(7.0, 11.0),
        );
        let owner = app
            .world_mut()
            .spawn((
                CenteredAabb::from_aabb(collision),
                DamageableVolumes::single(authored_hurt),
                ambition_characters::actor::BodyCombat::default(),
            ))
            .id();
        app.world_mut().spawn(Hitbox {
            owner,
            source: HitSide::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::new(20.0, 0.0),
            },
            half_extent: ae::Vec2::new(5.0, 6.0),
            shape: None,
            facing: 1.0,
            damage: 2,
            knockback: HitboxKnockback::LaunchSpeed {
                base: 100.0,
                growth: 2.0,
            },
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
            strike_sfx: None,
        });

        app.update();

        let view = app.world().resource::<CombatGeometryView>();
        assert_eq!(view.bodies.len(), 1);
        assert_eq!(view.bodies[0].collision, collision);
        assert_eq!(
            view.bodies[0].hurtboxes,
            vec![ae::CombatVolume::aabb(authored_hurt)]
        );
        assert_eq!(view.strikes.len(), 1);
        assert_eq!(
            view.strikes[0].volume.bounds().center(),
            body_center + ae::Vec2::new(20.0, 0.0)
        );
    }

    #[test]
    fn world_anchored_strike_does_not_need_a_live_owner() {
        let mut app = App::new();
        app.init_resource::<CombatGeometryView>();
        app.add_systems(Update, rebuild_combat_geometry_view);

        let owner = app.world_mut().spawn_empty().id();
        app.world_mut().despawn(owner);
        let center = ae::Vec2::new(310.0, 170.0);
        app.world_mut().spawn(Hitbox {
            owner,
            source: HitSide::Boss,
            anchor: HitboxAnchor::World { center },
            half_extent: ae::Vec2::new(16.0, 9.0),
            shape: None,
            facing: 1.0,
            damage: 1,
            knockback: HitboxKnockback::FeelScale(1.0),
            launch_dir: None,
            frame_down: ae::Vec2::new(0.0, 1.0),
            strike_sfx: None,
        });

        app.update();

        let view = app.world().resource::<CombatGeometryView>();
        assert_eq!(view.strikes.len(), 1);
        assert_eq!(view.strikes[0].volume.bounds().center(), center);
    }

    #[test]
    fn combat_geometry_preserves_intangible_and_fallback_states() {
        let collision = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(10.0, 12.0));
        assert_eq!(
            effective_hurtboxes(collision, None),
            vec![ae::CombatVolume::aabb(collision)]
        );
        let unpublished = DamageableVolumes::default();
        assert_eq!(
            effective_hurtboxes(collision, Some(&unpublished)),
            vec![ae::CombatVolume::aabb(collision)]
        );
        let mut intangible = DamageableVolumes::default();
        intangible.clear();
        assert!(effective_hurtboxes(collision, Some(&intangible)).is_empty());
    }
}
