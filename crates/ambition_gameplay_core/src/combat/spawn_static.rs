//! Static authored room-feature spawn helpers.
//!
//! These functions stay family-specific so adding an authored static
//! feature remains "add a RoomSpec Vec + add one loop in spawn.rs".

use super::*;
use crate::platformer_runtime::prelude::SpawnScopedExt;
use bevy::prelude::Name;

pub(crate) fn spawn_hazard(
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
    commands.spawn_room_scoped((
        Name::new(format!("Feature hazard: {}", authored.name)),
        FeatureSimEntity,
        RoomVisual,
        FeatureId::new(authored.id.clone()),
        FeatureName::new(authored.name.clone()),
        CenteredAabb::from_center_size(hazard.pos, hazard.size),
        HazardFeature::new(hazard),
    ));
}

pub(crate) fn spawn_pickup(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Pickup>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    commands.spawn_room_scoped((
        Name::new(format!("Feature pickup: {}", authored.name)),
        PickupBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            authored.payload.clone(),
        ),
    ));
}

pub(crate) fn spawn_ground_item(commands: &mut Commands, spec: &crate::rooms::GroundItemSpec) {
    // Resolve the held-item registry id -> HeldItemSpec. An unregistered or
    // feature-gated id is skipped (the item simply doesn't appear) -- the same
    // tolerance the retired `spawn_debug_ground_items_once` table had.
    let Some(held) = crate::brain::held_item_by_id(&spec.held_item) else {
        return;
    };
    commands.spawn_room_scoped((
        Name::new(format!("Ground item: {}", spec.name)),
        crate::items::pickup::GroundItem {
            spec: held,
            pos: spec.pos,
            vel: crate::engine_core::Vec2::ZERO,
            half_extent: spec.half_extent,
        },
    ));
}

#[cfg(feature = "portal")]
pub(crate) fn spawn_portal_gun_spawn(
    commands: &mut Commands,
    spec: &crate::rooms::PortalGunSpawnSpec,
) {
    commands.spawn_room_scoped((
        Name::new(format!("Portal gun pickup: {}", spec.name)),
        crate::portal::PortalGunPickup {
            pos: spec.pos,
            half_extent: spec.half_extent,
            // World-placed pickups spawn already armed (a just-dropped one delays).
            arm_timer: 0.0,
        },
    ));
}

#[cfg(feature = "portal")]
pub(crate) fn spawn_portal(commands: &mut Commands, spec: &crate::rooms::PortalSpec) {
    // Authored static portal: the same `Portal` component the gun fires, but
    // pre-placed and color-paired. Room-scoped so a transition despawns it and
    // the loader re-spawns it; never gun-owned, so it persists without a gun.
    // Opening size: authored along-surface half-length if given, else default.
    let half_extent = match spec.half_length {
        Some(h) => crate::portal::portal_half_extent_with_length(spec.normal, h),
        None => crate::portal::portal_half_extent(spec.normal),
    };
    let mut entity = commands.spawn_room_scoped((
        Name::new(format!("Portal ({}): {}", spec.color.name(), spec.name)),
        crate::portal::PlacedPortal {
            // Link-authored portals get a provisional channel; `resolve_portal_links`
            // assigns the real paired channel each frame. Color-authored keep
            // their legacy complementary channel.
            channel: spec.color.channel(),
            pos: spec.pos,
            normal: spec.normal,
            half_extent,
        },
    ));
    if let Some(link) = &spec.link {
        entity.insert(crate::portal::PortalLink(crate::portal::link_hash(link)));
    }
}

pub(crate) fn spawn_shrine(commands: &mut Commands, spec: &crate::rooms::ShrineSpec) {
    commands.spawn_room_scoped((
        Name::new(format!("Heal/save shrine: {}", spec.name)),
        crate::shrine::HealShrine {
            pos: spec.pos,
            half_extent: spec.half_extent,
        },
    ));
}

pub(crate) fn spawn_gravity_zone(commands: &mut Commands, spec: &crate::rooms::GravityZoneSpec) {
    let mut entity = commands.spawn_room_scoped((
        Name::new(format!("Gravity zone: {}", spec.name)),
        crate::physics::GravityZone {
            aabb: crate::engine_core::Aabb::new(spec.center, spec.half_extent),
            dir: spec.dir,
        },
    ));
    // A non-zero amplitude makes the column slide horizontally (the sliding
    // gravity demo); a static column omits the OscillatingZone.
    if spec.oscillate_amplitude > 0.0 {
        entity.insert(crate::physics::OscillatingZone {
            base_center: spec.center,
            half: spec.half_extent,
            amplitude_x: spec.oscillate_amplitude,
            freq: spec.oscillate_freq,
            phase: 0.0,
        });
    }
}

pub(crate) fn spawn_chest(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Chest>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    commands.spawn_room_scoped((
        Name::new(format!("Feature chest: {}", authored.name)),
        ChestBundle::new(
            &authored.id,
            &authored.name,
            feature_aabb,
            authored.payload.clone(),
        ),
    ));
}

pub(crate) fn spawn_breakable(
    commands: &mut Commands,
    authored: &crate::rooms::Authored<crate::interaction::Breakable>,
) {
    let feature_aabb = CenteredAabb::from_aabb(authored.aabb);
    let breakable = &authored.payload;
    let mut entity = commands.spawn_room_scoped((
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
