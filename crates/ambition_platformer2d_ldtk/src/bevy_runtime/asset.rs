//! LDtk asset handles + per-area `LevelSet` selection (bevy_ecs_ldtk side).
//!
//! Holds the loaded-project handles (`ActiveLdtkProject`/`IntroLdtkAsset`/
//! `CutRopeLdtkAsset`), world-root markers, and `LdtkRuntimeIndex` — the
//! area→level-IID/`LevelSet`/bounds map that drives streaming. Systems:
//! `load_ldtk_asset_handle` (kick the load) and
//! `present_ldtk_levels_per_live_room` (each live room's bundles and the
//! `LevelSet` each shows).

use std::collections::BTreeMap;

use bevy::asset::{AssetServer, Assets, Handle};
use bevy::prelude::{
    Commands, Component, DetectChanges, DetectChangesMut, Query, Res, ResMut, Resource, Transform,
    Vec3, With,
};
use bevy_ecs_ldtk::assets::LdtkProject;
use bevy_ecs_ldtk::ldtk::{LayerInstance, Type, raw_level_accessor::RawLevelAccessor};
use bevy_ecs_ldtk::prelude::{IntGridRendering, LdtkSettings, LevelSet};

use ambition_platformer2d_core::config::WORLD_Z_BLOCK;

pub use super::super::index::{LdtkAreaBounds, LdtkRuntimeIndex};
use ambition_platformer2d_world::world_manifest::{world_bevy_asset_path, WorldManifest};

/// Loaded bevy_ecs_ldtk project handles, one per prepared
/// [`WorldManifest`] row (index-aligned; 0 = primary).
/// `bevy_ecs_ldtk`'s asset loader is per-file and independent of
/// Ambition's merged JSON loader, so every world file gets its own
/// handle + `LdtkWorldBundle` to render its painted tile layers.
#[derive(Resource, Clone, Debug, Default)]
pub struct LdtkWorldAssets(pub Vec<Handle<LdtkProject>>);

pub fn load_ldtk_asset_handle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    manifest: Res<WorldManifest>,
) {
    let handles = manifest
        .worlds
        .iter()
        .map(|source| asset_server.load(world_bevy_asset_path(source)))
        .collect();
    commands.insert_resource(LdtkWorldAssets(handles));
}

/// Keep unpainted grids out of the presentation ECS. The room IR reads their cells.
pub(super) fn select_painted_layers(
    projects: Res<Assets<LdtkProject>>,
    settings: ResMut<LdtkSettings>,
) {
    if !projects.is_changed() {
        return;
    }
    let excluded = if settings.int_grid_rendering == IntGridRendering::Invisible
        && projects.iter().all(|(_, project)| !project.json_data().external_levels)
    {
        let layers = projects.iter().flat_map(|(_, project)| {
            project.iter_raw_levels().flat_map(|level| level.layer_instances.iter().flatten())
        });
        unpainted_grid_names(layers)
    } else {
        Vec::new()
    };
    settings
        .map_unchanged(|settings| &mut settings.exclusions.layer_identifiers)
        .set_if_neq(excluded);
}

fn unpainted_grid_names<'a>(layers: impl Iterator<Item = &'a LayerInstance>) -> Vec<String> {
    let mut excluded_by_name = BTreeMap::new();
    for layer in layers {
        let unpainted_grid = layer.layer_instance_type == Type::IntGrid
            && layer.tileset_def_uid.is_none()
            && layer.auto_layer_tiles.is_empty()
            && layer.grid_tiles.is_empty();
        // The adapter excludes names across all its projects.
        *excluded_by_name.entry(layer.identifier.as_str()).or_insert(true) &= unpainted_grid;
    }
    excluded_by_name
        .into_iter()
        .filter(|(_, exclude)| *exclude)
        .map(|(name, _)| name.to_owned())
        .collect()
}

/// Marker for every LDtk world-root entity (one per manifest row and live
/// room). Each bundle's `LevelSet` carries its room's iids; only the bundle
/// whose loaded asset contains those iids spawns levels (iids are unique per
/// file), so a room's bundles all get the SAME set.
#[derive(Component)]
pub struct LdtkWorldRoot;

/// The levels `index` shows for `area`, as the `bevy_ecs_ldtk` bundle's set.
pub fn level_set_for(index: &LdtkRuntimeIndex, area: &str) -> LevelSet {
    LevelSet::from_iids(index.level_iids_for(area))
}

/// Run condition: did an authoring format install an LDtk world into the
/// live session?
///
/// The index is optional session state now, so its ABSENCE is the honest
/// statement that this game has no LDtk world — and every system below reads it
/// through a `SessionWorldRef`, which is a `Single` and would otherwise fail
/// its parameter validation once a tick in all five RON-authored games. Gating
/// on the component means the whole spine simply does not run there, rather
/// than running against an empty index and finding nothing.
pub fn ldtk_world_installed(
    roots: Query<
        (),
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
            With<LdtkRuntimeIndex>,
        ),
    >,
) -> bool {
    !roots.is_empty()
}

/// Each live room's painted levels (view half V4b): one bundle per prepared
/// world file, stamped with the live room, showing that room's levels.
///
/// One bundle set showed the sole live room, so while two rooms were live it
/// showed neither (the sole-room read does not answer then), and with one room
/// a second player's view could not have its own. Now each live room has its
/// own bundles. The stamp is what the view half reads: the room's render band
/// (V3) draws them only in the views that frame that room, and the room's
/// retirement takes them (`RoomScopedEntity`). The bundles are the memo: a
/// live room with no bundle stamped with it gets them, so a room opened beside
/// another, and a room that replaces another, are drawn.
///
/// The `LevelSet` value is the record of what a bundle shows. A compare by
/// value finds a reloaded project, and `bevy_ecs_ldtk` respawns levels only on
/// a write. Every bundle of a room gets the same set: a bundle spawns only the
/// levels its own file holds (iids are unique per file).
#[allow(clippy::type_complexity)]
pub fn present_ldtk_levels_per_live_room(
    mut commands: Commands,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    index: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<LdtkRuntimeIndex>,
    assets: Option<Res<LdtkWorldAssets>>,
    active_session: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>>,
    mut presented: Query<
        (
            &ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance,
            &mut LevelSet,
            &mut Transform,
        ),
        With<LdtkWorldRoot>,
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    let Some(session_scope) =
        ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::for_optional_active_session(
            active_session.as_deref(),
        )
    else {
        return;
    };
    for (room, definition) in rooms.live_rooms() {
        let spec = rooms.rooms().spec(definition);
        let shown = level_set_for(&index, &spec.id);
        let target = ldtk_world_translation(&spec.world);
        let mut drawn = false;
        for (stamp, mut level_set, mut transform) in &mut presented {
            if stamp.0 != room {
                continue;
            }
            drawn = true;
            level_set.set_if_neq(shown.clone());
            if (transform.translation - target).length_squared() > 1e-6 {
                transform.translation = target;
            }
        }
        if drawn {
            continue;
        }
        let scope = session_scope.in_room(Some(room));
        for handle in &assets.0 {
            let mut root = commands.spawn((
                bevy_ecs_ldtk::prelude::LdtkWorldBundle {
                    ldtk_handle: handle.clone().into(),
                    level_set: shown.clone(),
                    transform: Transform::from_translation(target),
                    ..Default::default()
                },
                LdtkWorldRoot,
                ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity,
                bevy::prelude::Name::new(format!("LDtk levels of {}", spec.id)),
            ));
            scope.apply_to(&mut root);
        }
    }
}

/// Where a room's LDtk bundles sit: `bevy_ecs_ldtk` places levels from the
/// bundle's origin in level-local pixels, and Ambition centres the room, so
/// the bundle takes the room's bottom-left (`-size / 2`). In front of the
/// block quads (`WORLD_Z_BLOCK`), so the painted tiles hide the debug
/// rectangles where they have content, and well behind the player
/// (`WORLD_Z_PLAYER`). (ADR 0015, coordinate-frame reconciliation; keep it
/// consistent with `world_to_bevy` and `LdtkSettings::level_spawn_behavior`.)
pub fn ldtk_world_translation(world: &ambition_platformer2d_core::World) -> Vec3 {
    Vec3::new(-world.size.x * 0.5, -world.size.y * 0.5, WORLD_Z_BLOCK + 0.5)
}

#[cfg(test)]
mod tests {
    use bevy_ecs_ldtk::ldtk::{LayerInstance, Type};
    use bevy::ecs::system::RunSystemOnce as _;
    use bevy::prelude::World;
    use bevy_ecs_ldtk::prelude::LevelSet;

    use ambition_platformer2d_core as ae;

    #[test]
    fn a_painted_or_entity_layer_keeps_its_name_in_the_renderer() {
        let grid = |name: &str| LayerInstance {
            identifier: name.into(),
            layer_instance_type: Type::IntGrid,
            int_grid_csv: vec![3, 0, 3],
            ..Default::default()
        };
        let layers = [
            grid("Collision"),
            grid("Ground"),
            LayerInstance { tileset_def_uid: Some(7), ..grid("Ground") },
            LayerInstance { layer_instance_type: Type::Entities, ..grid("Props") },
            grid("Props"),
        ];
        assert_eq!(super::unpainted_grid_names(layers.iter()), ["Collision"]);
        assert_eq!(layers[0].int_grid_csv, [3, 0, 3]);
    }
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
    use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};

    use super::{present_ldtk_levels_per_live_room, LdtkRuntimeIndex, LdtkWorldAssets, LdtkWorldRoot};
    use crate::LdtkProject;

    /// A project whose levels fall in the named areas, by `activeArea`.
    fn project(levels: &[(&str, &str)]) -> LdtkProject {
        let levels: Vec<serde_json::Value> = levels
            .iter()
            .map(|(iid, area)| {
                serde_json::json!({
                    "identifier": iid, "iid": iid,
                    "worldX": 0, "worldY": 0, "pxWid": 16, "pxHei": 16,
                    "fieldInstances": [{"__identifier": "activeArea", "__value": area}],
                })
            })
            .collect();
        serde_json::from_value(serde_json::json!({"jsonVersion": "1.5.3", "levels": levels}))
            .expect("the fixture project parses")
    }

    fn room(id: &str, width: f32) -> RoomSpec {
        let world = ae::World::new(id, ae::Vec2::new(width, 64.0), ae::Vec2::ZERO, Vec::new());
        RoomSpec::new(id, world)
    }

    /// Per live room: the level set and the x of each bundle stamped with it.
    fn shown(world: &mut World) -> Vec<(LiveRoomInstance, LevelSet, i32)> {
        let mut rows: Vec<_> = world
            .query_filtered::<(&InRoomInstance, &LevelSet, &bevy::prelude::Transform), bevy::prelude::With<LdtkWorldRoot>>()
            .iter(world)
            .map(|(room, set, transform)| (room.0, set.clone(), transform.translation.x as i32))
            .collect();
        rows.sort_by_key(|(room, ..)| *room);
        rows
    }

    /// A world with rooms `a` (640 wide) and `b` (320 wide), `a` live, one
    /// world file, and the index `levels`.
    fn session(levels: &[(&str, &str)]) -> World {
        let mut world = World::new();
        let root = ambition_platformer2d_world::rooms::insert_room_set(
            &mut world,
            RoomSet::from_parts_or_panic("a", vec![room("a", 640.0), room("b", 320.0)], Vec::new()),
        );
        world.entity_mut(root).insert(LdtkRuntimeIndex::from_project(&project(levels)));
        world.insert_resource(LdtkWorldAssets(vec![Default::default()]));
        world
    }

    fn open_b_beside_a(world: &mut World) {
        let b = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<RoomSet>(world)
            .expect("the room set")
            .definition_by_id("b")
            .expect("the fixture authors b");
        world
            .spawn(ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(
                ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(0),
            ))
            .insert((LiveRoomInstance::ACTIVATION.next(), b));
    }

    /// Each live room gets its own bundle, stamped with it, showing its own
    /// levels at its own origin; a second pass adds nothing; and a reload
    /// reaches the bundle of the room it changes. The control is one live
    /// room: one bundle. Before, one bundle showed the sole live room, and
    /// with two rooms live nothing answered for either.
    #[test]
    fn each_live_room_shows_its_own_ldtk_levels() {
        let a = LiveRoomInstance::ACTIVATION;
        let b = a.next();
        let mut world = session(&[("a1", "a"), ("a2", "a"), ("b1", "b")]);
        world.run_system_once(present_ldtk_levels_per_live_room).unwrap();
        assert_eq!(
            shown(&mut world),
            vec![(a, LevelSet::from_iids(["a1", "a2"]), -320)],
            "control: one live room, one bundle"
        );

        open_b_beside_a(&mut world);
        world.run_system_once(present_ldtk_levels_per_live_room).unwrap();
        world.run_system_once(present_ldtk_levels_per_live_room).unwrap();
        assert_eq!(
            shown(&mut world),
            vec![
                (a, LevelSet::from_iids(["a1", "a2"]), -320),
                (b, LevelSet::from_iids(["b1"]), -160),
            ],
            "each live room has one bundle of its own levels, at its own origin"
        );
        // Its room's retirement takes it: the sweep takes a room-scoped entity
        // stamped with the retiring room.
        let unscoped = world
            .query_filtered::<(), (
                bevy::prelude::With<LdtkWorldRoot>,
                bevy::prelude::Without<ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity>,
            )>()
            .iter(&world)
            .count();
        assert_eq!(unscoped, 0, "a bundle that is not room-scoped outlives its room");

        // A reload that moves a level into `b`.
        let root = ambition_platformer2d_shared_tangle::lifecycle::session_world_entity(&world).expect("the session");
        world.entity_mut(root).insert(LdtkRuntimeIndex::from_project(&project(&[
            ("a1", "a"),
            ("a2", "a"),
            ("b1", "b"),
            ("b2", "b"),
        ])));
        world.run_system_once(present_ldtk_levels_per_live_room).unwrap();
        assert_eq!(
            shown(&mut world)[1].1,
            LevelSet::from_iids(["b1", "b2"]),
            "a reloaded project did not reach the room it changes"
        );
    }
}
