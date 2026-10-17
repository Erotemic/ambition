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
    live_room: ambition_platformer2d_world::rooms::SoleLiveRoomSpec,
    runtime_index: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<LdtkRuntimeIndex>,
    mut spine_index: ResMut<LdtkRuntimeSpineIndex>,
    query: Query<&AmbitionLdtkEntity>,
    changed: Query<(), bevy::prelude::Changed<AmbitionLdtkEntity>>,
    mut removed: bevy::prelude::RemovedComponents<AmbitionLdtkEntity>,
) {
    // The index follows from the live room, the runtime index and the LDtk
    // entities. A tick where none of them changed, and nothing else wrote the
    // index, has nothing to rebuild.
    use bevy::prelude::DetectChanges as _;
    let entities_moved = !changed.is_empty() | (removed.read().count() > 0);
    if !entities_moved && !live_room.is_changed() && !runtime_index.is_changed() && !spine_index.is_changed() {
        return;
    }
    let active_area = live_room.spec().id.clone();
    let origin = runtime_index
        .area_bounds(&active_area)
        .map(|bounds| [bounds.min_x, bounds.min_y])
        .unwrap_or([0, 0]);

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

#[cfg(test)]
mod tests {
    use bevy::ecs::change_detection::DetectChangesMut as _;
    use bevy::prelude::*;

    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};

    use super::super::asset::LdtkRuntimeIndex;
    use super::super::components::AmbitionLdtkEntity;
    use super::super::indices::LdtkRuntimeSpineIndex;
    use super::rebuild_ldtk_runtime_spine_index;

    fn entity(iid: &str, x: i32) -> AmbitionLdtkEntity {
        AmbitionLdtkEntity { iid: iid.to_owned(), identifier: "DebugLabel".to_owned(), px: [x, 0], size: [8, 8], world: None }
    }

    /// The index is rebuilt only when what it reads changed. An entity edited
    /// past change detection is not seen, which shows the tick skipped; an
    /// ordinary edit, a new entity and a removed one are each in the index on
    /// the next tick.
    #[test]
    fn the_spine_index_follows_its_entities_and_skips_a_quiet_tick() {
        let mut app = App::new();
        let root = ambition_platformer2d_world::rooms::insert_room_set(
            app.world_mut(),
            RoomSet::from_parts_or_panic(
                "a",
                vec![RoomSpec::new("a", ae::World::new("a", ae::Vec2::new(64.0, 64.0), ae::Vec2::ZERO, Vec::new()))],
                Vec::new(),
            ),
        );
        app.world_mut().entity_mut(root).insert(LdtkRuntimeIndex::default());
        app.init_resource::<LdtkRuntimeSpineIndex>();
        app.add_systems(Update, rebuild_ldtk_runtime_spine_index);
        let target = app.world_mut().spawn(entity("one", 10)).id();
        app.update();
        let x_of = |app: &App, iid: &str| {
            app.world().resource::<LdtkRuntimeSpineIndex>().entities.iter().find(|e| e.iid == iid).map(|e| e.min.x)
        };
        assert_eq!(x_of(&app, "one"), Some(10.0), "premise: the first tick indexes the entity");

        app.world_mut().get_mut::<AmbitionLdtkEntity>(target).unwrap().bypass_change_detection().px[0] = 20;
        app.update();
        assert_eq!(x_of(&app, "one"), Some(10.0), "a tick where nothing changed rebuilt the index");

        app.world_mut().get_mut::<AmbitionLdtkEntity>(target).unwrap().px[0] = 30;
        app.update();
        assert_eq!(x_of(&app, "one"), Some(30.0), "an edited entity was not indexed again");

        let added = app.world_mut().spawn(entity("two", 5)).id();
        app.update();
        assert_eq!(x_of(&app, "two"), Some(5.0), "a new entity was not indexed");
        app.world_mut().despawn(added);
        app.update();
        assert_eq!(x_of(&app, "two"), None, "a removed entity stayed in the index");
    }
}
