//! Systems that rebuild the runtime-spine collision indices from the ECS.
//!
//! Each `rebuild_ldtk_runtime_*_index` system scans the typed `LdtkSolid`/
//! `LdtkOneWayPlatform`/`LdtkDamageVolume` components (sibling `components`)
//! and refills the matching resource in sibling `indices`. Pure index
//! maintenance — consumed downstream by collision queries and sibling `parity`.

use std::collections::BTreeMap;

use bevy::prelude::{Query, ResMut, With};

use ambition_engine_core as ae;

use super::asset::LdtkRuntimeIndex;
use super::components::{
    AmbitionLdtkEntity, LdtkDamageVolume, LdtkOneWayPlatform, LdtkRuntimeRole, LdtkSolid,
};
use super::indices::{
    LdtkRuntimeDamageIndex, LdtkRuntimeDamageVolume, LdtkRuntimeOneWayIndex,
    LdtkRuntimeOneWayPlatform, LdtkRuntimeSolid, LdtkRuntimeSolidIndex, LdtkRuntimeSpineEntity,
    LdtkRuntimeSpineIndex,
};

/// Rebuild the active-area-local index of promoted LDtk `Solid` entities.
///
/// Mirrors `rebuild_ldtk_runtime_spine_index` but only collects entities that
/// carry the typed `LdtkSolid` component, so future collision authority can
/// query a tight collision-only view without iterating the broader spine.
pub fn rebuild_ldtk_runtime_solid_index(
    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<ambition_world::rooms::RoomSet>,
    runtime_index: ambition_platformer_primitives::lifecycle::SessionWorldRef<LdtkRuntimeIndex>,
    mut solid_index: ResMut<LdtkRuntimeSolidIndex>,
    query: Query<&AmbitionLdtkEntity, With<LdtkSolid>>,
) {
    let active_area = room_set.active_spec().id.clone();
    let origin = runtime_index
        .area_bounds(&active_area)
        .map(|bounds| [bounds.min_x, bounds.min_y])
        .unwrap_or_else(|| runtime_index.active_area_origin());

    let mut next = LdtkRuntimeSolidIndex {
        active_area,
        solids: Vec::new(),
        revision: solid_index.revision,
    };

    for entity in &query {
        let raw_min = entity.world.unwrap_or(entity.px);
        // AMBITION_REVIEW(spatial): solid `min` is projected from LDtk world
        // pixels into active-area-local Ambition coordinates by subtracting
        // the area origin. This must stay consistent with the spine-index
        // projection and with `compose_runtime_area`'s offset math, otherwise
        // ECS-side collision will drift from the JSON-derived `world.blocks`.
        let min = ae::Vec2::new(
            (raw_min[0] - origin[0]) as f32,
            (raw_min[1] - origin[1]) as f32,
        );
        let size = ae::Vec2::new(entity.size[0] as f32, entity.size[1] as f32);
        next.solids.push(LdtkRuntimeSolid {
            iid: entity.iid.clone(),
            min,
            size,
        });
    }

    solid_index.replace_if_changed(next);
}

/// Rebuild the active-area-local index of promoted LDtk `OneWayPlatform`
/// entities. Mirror of `rebuild_ldtk_runtime_solid_index` for the
/// promoted one-way category. The JSON adapter still owns runtime
/// collision authority pending the parity overlay.
pub fn rebuild_ldtk_runtime_one_way_index(
    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<ambition_world::rooms::RoomSet>,
    runtime_index: ambition_platformer_primitives::lifecycle::SessionWorldRef<LdtkRuntimeIndex>,
    mut one_way_index: ResMut<LdtkRuntimeOneWayIndex>,
    query: Query<&AmbitionLdtkEntity, With<LdtkOneWayPlatform>>,
) {
    let active_area = room_set.active_spec().id.clone();
    let origin = runtime_index
        .area_bounds(&active_area)
        .map(|bounds| [bounds.min_x, bounds.min_y])
        .unwrap_or_else(|| runtime_index.active_area_origin());

    let mut next = LdtkRuntimeOneWayIndex {
        active_area,
        platforms: Vec::new(),
        revision: one_way_index.revision,
    };

    for entity in &query {
        let raw_min = entity.world.unwrap_or(entity.px);
        // AMBITION_REVIEW(spatial): one-way `min` is projected from LDtk
        // world pixels into active-area-local Ambition coords. Must stay
        // consistent with `LdtkRuntimeSolidIndex` projection — they share
        // the same coordinate frame.
        let min = ae::Vec2::new(
            (raw_min[0] - origin[0]) as f32,
            (raw_min[1] - origin[1]) as f32,
        );
        let size = ae::Vec2::new(entity.size[0] as f32, entity.size[1] as f32);
        next.platforms.push(LdtkRuntimeOneWayPlatform {
            iid: entity.iid.clone(),
            min,
            size,
        });
    }

    one_way_index.replace_if_changed(next);
}

/// Rebuild the active-area-local index of promoted LDtk `DamageVolume`
/// (and legacy `HazardBlock`) entities.
pub fn rebuild_ldtk_runtime_damage_index(
    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<ambition_world::rooms::RoomSet>,
    runtime_index: ambition_platformer_primitives::lifecycle::SessionWorldRef<LdtkRuntimeIndex>,
    mut damage_index: ResMut<LdtkRuntimeDamageIndex>,
    query: Query<(&AmbitionLdtkEntity, &LdtkDamageVolume)>,
) {
    let active_area = room_set.active_spec().id.clone();
    let origin = runtime_index
        .area_bounds(&active_area)
        .map(|bounds| [bounds.min_x, bounds.min_y])
        .unwrap_or_else(|| runtime_index.active_area_origin());

    let mut next = LdtkRuntimeDamageIndex {
        active_area,
        volumes: Vec::new(),
        revision: damage_index.revision,
    };

    for (entity, damage) in &query {
        let raw_min = entity.world.unwrap_or(entity.px);
        let min = ae::Vec2::new(
            (raw_min[0] - origin[0]) as f32,
            (raw_min[1] - origin[1]) as f32,
        );
        let size = ae::Vec2::new(entity.size[0] as f32, entity.size[1] as f32);
        next.volumes.push(LdtkRuntimeDamageVolume {
            iid: entity.iid.clone(),
            min,
            size,
            damage: damage.damage,
        });
    }

    damage_index.replace_if_changed(next);
}

/// Rebuild an Ambition runtime-spine index from currently spawned LDtk entities.
///
/// `bevy_ecs_ldtk` owns the entity lifecycle; this system projects those
/// entities into active-area-local Ambition coordinates so gameplay/debug
/// systems can consume plugin output without reparsing the LDtk JSON file.
pub fn rebuild_ldtk_runtime_spine_index(
    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<ambition_world::rooms::RoomSet>,
    runtime_index: ambition_platformer_primitives::lifecycle::SessionWorldRef<LdtkRuntimeIndex>,
    mut spine_index: ResMut<LdtkRuntimeSpineIndex>,
    query: Query<&AmbitionLdtkEntity>,
) {
    let active_area = room_set.active_spec().id.clone();
    let origin = runtime_index
        .area_bounds(&active_area)
        .map(|bounds| [bounds.min_x, bounds.min_y])
        .unwrap_or_else(|| runtime_index.active_area_origin());

    let mut next = LdtkRuntimeSpineIndex {
        active_area,
        entities: Vec::new(),
        promoted_counts: BTreeMap::new(),
        revision: spine_index.revision,
    };

    for entity in &query {
        let role = LdtkRuntimeRole::from_identifier(&entity.identifier);
        if role.promoted() {
            *next.promoted_counts.entry(role).or_default() += 1;
        }
        let raw_min = entity.world.unwrap_or(entity.px);
        let min = ae::Vec2::new(
            (raw_min[0] - origin[0]) as f32,
            (raw_min[1] - origin[1]) as f32,
        );
        let size = ae::Vec2::new(entity.size[0] as f32, entity.size[1] as f32);
        next.entities.push(LdtkRuntimeSpineEntity {
            iid: entity.iid.clone(),
            identifier: entity.identifier.clone(),
            role,
            min,
            size,
        });
    }

    spine_index.replace_if_changed(next);
}
