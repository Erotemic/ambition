//! Static authored room-feature spawn helpers.
//!
//! These functions stay family-specific so adding an authored static
//! feature remains "add a RoomSpec Vec + add one loop in spawn.rs".

use super::*;
use bevy::prelude::Name;

pub(super) fn spawn_hazard(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::combat::DamageVolume>,
    paths: &[(String, crate::actor::KinematicPath)],
) {
    let hazard = HazardRuntime::new_with_paths(
        authored.id.clone(),
        authored.name.clone(),
        authored.aabb,
        authored.payload.clone(),
        paths,
    );
    commands.spawn((
        Name::new(format!("Feature hazard: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        FeatureAabb::from_center_size(hazard.pos, hazard.size),
        HazardFeature::new(hazard),
    ));
}

pub(super) fn spawn_pickup(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Pickup>,
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    commands.spawn((
        Name::new(format!("Feature pickup: {}", authored.name)),
        PickupBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            authored.payload.clone(),
        ),
    ));
}

pub(super) fn spawn_chest(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Chest>,
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    commands.spawn((
        Name::new(format!("Feature chest: {}", authored.name)),
        ChestBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            authored.payload.clone(),
        ),
    ));
}

pub(super) fn spawn_breakable(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Breakable>,
) {
    let feature_aabb = FeatureAabb::from_aabb(authored.aabb);
    let breakable = &authored.payload;
    let mut entity = commands.spawn((
        Name::new(format!("Feature breakable: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        feature_aabb,
        BreakableFeature::new(breakable.clone()),
        DamageableVolumes::default(),
        PogoPolicy::FromDamageable,
        PogoTargetVolumes::default(),
        StandTimer(0.0),
    ));
    if breakable.collision.blocks_movement() {
        entity.insert(SandboxSolidContributor);
    }
    if breakable.pogo_refresh
        || (breakable.collision.blocks_movement() && breakable.trigger.allows_stand())
    {
        entity.insert(PogoTargetContributor);
    }
}
