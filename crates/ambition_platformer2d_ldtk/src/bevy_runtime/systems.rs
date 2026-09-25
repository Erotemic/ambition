//! The system that rebuilds the runtime-spine index from the ECS.

use std::collections::BTreeMap;

use bevy::prelude::{Query, ResMut};

use ambition_platformer2d_core as ae;

use super::asset::LdtkRuntimeIndex;
use super::components::{AmbitionLdtkEntity, LdtkRuntimeRole};
use super::indices::{LdtkRuntimeSpineEntity, LdtkRuntimeSpineIndex};

/// Rebuild an Ambition runtime-spine index from currently spawned LDtk entities.
///
/// `bevy_ecs_ldtk` owns the entity lifecycle; this system projects those
/// entities into active-area-local Ambition coordinates so gameplay/debug
/// systems can consume plugin output without reparsing the LDtk JSON file.
pub fn rebuild_ldtk_runtime_spine_index(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
    runtime_index: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<LdtkRuntimeIndex>,
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
