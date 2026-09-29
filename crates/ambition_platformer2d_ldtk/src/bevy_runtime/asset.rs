//! LDtk asset handles + per-area `LevelSet` selection (bevy_ecs_ldtk side).
//!
//! Holds the loaded-project handles (`ActiveLdtkProject`/`IntroLdtkAsset`/
//! `CutRopeLdtkAsset`), world-root markers, and `LdtkRuntimeIndex` — the
//! area→level-IID/`LevelSet`/bounds map that drives streaming. Systems:
//! `load_ldtk_asset_handle` (kick the load) and `sync_ldtk_level_set` (swap the
//! visible `LevelSet` when the active area changes).

use std::collections::BTreeMap;

use bevy::asset::{AssetServer, Handle};
use bevy::prelude::{
    Commands, Component, DetectChangesMut, Query, Res, Resource, Transform, Vec3, With,
};
use bevy_ecs_ldtk::prelude::LevelSet;

use ambition_platformer2d_core::config::WORLD_Z_BLOCK;

use super::super::{LdtkLevel, LdtkProject};
use ambition_platformer2d_world::world_manifest::{world_bevy_asset_path, WorldManifest};

/// Loaded bevy_ecs_ldtk project handles, one per prepared
/// [`WorldManifest`] row (index-aligned; 0 = primary).
/// `bevy_ecs_ldtk`'s asset loader is per-file and independent of
/// Ambition's merged JSON loader, so every world file gets its own
/// handle + `LdtkWorldBundle` to render its painted tile layers.
#[derive(Resource, Clone, Debug, Default)]
pub struct LdtkWorldAssets(pub Vec<Handle<bevy_ecs_ldtk::assets::LdtkProject>>);

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

/// Marker for every LDtk world-root entity (one per manifest row). Each
/// bundle's `LevelSet` carries the active area's iids; only the bundle
/// whose loaded asset contains those iids spawns levels (iids are unique
/// per file), so the shared sync below can write the SAME set to all.
#[derive(Component)]
pub struct LdtkWorldRoot;

#[derive(Clone, Copy, Debug)]
pub struct LdtkAreaBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl LdtkAreaBounds {
    fn from_level(level: &LdtkLevel) -> Self {
        Self {
            min_x: level.world_x,
            min_y: level.world_y,
            max_x: level.world_x + level.px_wid,
            max_y: level.world_y + level.px_hei,
        }
    }

    fn include_level(&mut self, level: &LdtkLevel) {
        self.min_x = self.min_x.min(level.world_x);
        self.min_y = self.min_y.min(level.world_y);
        self.max_x = self.max_x.max(level.world_x + level.px_wid);
        self.max_y = self.max_y.max(level.world_y + level.px_hei);
    }
}

/// The LDtk project's area table: the levels each area holds and the area's
/// bounds in project pixels.
///
/// It is prepared content. It is built once from the project and does not know
/// which area is active: `RoomSet` holds that fact, and the LDtk bundles'
/// `LevelSet` holds which area they show. It is not rollback state, because
/// no simulation system writes it: only a content authority does, at
/// activation and at a hot reload, and the hot reload rebases the local
/// rollback session in the same step. No rewind crosses a change.
#[derive(Component, Clone, Debug, Default)]
pub struct LdtkRuntimeIndex {
    area_level_iids: BTreeMap<String, Vec<String>>,
    area_bounds: BTreeMap<String, LdtkAreaBounds>,
}

impl LdtkRuntimeIndex {
    pub fn from_project(project: &LdtkProject) -> Self {
        let mut area_level_iids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut area_bounds: BTreeMap<String, LdtkAreaBounds> = BTreeMap::new();
        for level in &project.levels {
            let active_area = level.active_area();
            area_level_iids
                .entry(active_area.clone())
                .or_default()
                .push(level.iid.clone());
            area_bounds
                .entry(active_area)
                .and_modify(|bounds| bounds.include_level(level))
                .or_insert_with(|| LdtkAreaBounds::from_level(level));
        }
        Self {
            area_level_iids,
            area_bounds,
        }
    }

    pub fn level_iids_for(&self, area: &str) -> Vec<String> {
        self.area_level_iids.get(area).cloned().unwrap_or_default()
    }

    pub fn level_set_for(&self, area: &str) -> LevelSet {
        LevelSet::from_iids(self.level_iids_for(area))
    }

    pub fn area_bounds(&self, area: &str) -> Option<LdtkAreaBounds> {
        self.area_bounds.get(area).copied()
    }
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

pub fn sync_ldtk_level_set(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
    index: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<LdtkRuntimeIndex>,
    mut ldtk_worlds: Query<&mut LevelSet, With<LdtkWorldRoot>>,
) {
    // The bundle's own `LevelSet` is the record of what it shows. A compare by
    // value finds both a room change and a reloaded project, so no second
    // cursor is necessary. `set_if_neq` writes only on a difference, and
    // `bevy_ecs_ldtk` respawns levels only on a write.
    //
    // Both bundles get the same LevelSet. bevy_ecs_ldtk only spawns
    // levels whose iids exist in the bundle's loaded asset, so the
    // sandbox bundle renders when the active area is in sandbox.ldtk
    // and the intro bundle renders when the active area is in
    // intro.ldtk — no cross-talk because iids are unique per file.
    let shown = index.level_set_for(&room_set.active_spec().id);
    for mut level_set in &mut ldtk_worlds {
        level_set.set_if_neq(shown.clone());
    }
}

/// Align the LDtk bundle's bottom-left tile origin with Ambition's centered
/// active-area frame. `bevy_ecs_ldtk` uses level-local pixel coordinates from
/// the bundle origin; Ambition maps the room bottom-left to
/// `(-world.size.x/2, -world.size.y/2)`. The bundle therefore receives that XY
/// offset and sits just behind `WORLD_Z_BLOCK`.
///
/// Keep this seam consistent with room dimensions, `world_to_bevy`, and
/// `LdtkSettings::level_spawn_behavior`.
pub fn sync_ldtk_world_transform(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
    mut ldtk_worlds: Query<&mut Transform, With<LdtkWorldRoot>>,
) {
    let active_world = room_set.active_world();
    let target = Vec3::new(
        -active_world.size.x * 0.5,
        -active_world.size.y * 0.5,
        // Render tile background slightly in FRONT of Ambition's
        // colored block quads (WORLD_Z_BLOCK = 0.0) so the painted
        // tileset visual hides the debug rectangles where it has
        // content. Stay well behind WORLD_Z_PLAYER (20.0) so the
        // player sprite stays on top.
        WORLD_Z_BLOCK + 0.5,
    );
    for mut tf in &mut ldtk_worlds {
        if (tf.translation - target).length_squared() > 1e-6 {
            tf.translation = target;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce as _;
    use bevy::prelude::World;
    use bevy_ecs_ldtk::prelude::LevelSet;

    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, session_world_component_mut,
    };
    use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};

    use super::{sync_ldtk_level_set, LdtkRuntimeIndex, LdtkWorldRoot};
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

    fn room(id: &str) -> RoomSpec {
        let world = ae::World::new(id, ae::Vec2::new(64.0, 64.0), ae::Vec2::ZERO, Vec::new());
        RoomSpec::new(id, world)
    }

    fn shown(world: &mut World) -> LevelSet {
        world
            .query_filtered::<&LevelSet, bevy::prelude::With<LdtkWorldRoot>>()
            .single(world)
            .expect("one LDtk bundle")
            .clone()
    }

    /// The bundle shows the levels of the room `RoomSet` names, and follows
    /// both a room change and a reloaded project. The index holds no active
    /// area and no revision; the `LevelSet` value is the only cursor.
    #[test]
    fn the_bundle_shows_the_active_rooms_levels_and_follows_a_change_or_a_reload() {
        let mut world = World::new();
        let root = insert_session_world_component(
            &mut world,
            RoomSet::from_parts_or_panic("a", vec![room("a"), room("b")], Vec::new()),
        );
        world.entity_mut(root).insert(LdtkRuntimeIndex::from_project(&project(&[
            ("a1", "a"),
            ("a2", "a"),
            ("b1", "b"),
        ])));
        world.spawn((LdtkWorldRoot, LevelSet::default()));

        // Control: before the sync runs, the bundle shows nothing.
        assert_eq!(shown(&mut world), LevelSet::default());

        world.run_system_once(sync_ldtk_level_set).unwrap();
        assert_eq!(shown(&mut world), LevelSet::from_iids(["a1", "a2"]));

        session_world_component_mut::<RoomSet>(&mut world)
            .expect("the session has a room set")
            .set_active_by_id("b")
            .expect("the fixture authors room b");
        world.run_system_once(sync_ldtk_level_set).unwrap();
        assert_eq!(
            shown(&mut world),
            LevelSet::from_iids(["b1"]),
            "the bundle still shows the room the player left"
        );

        // A reload that moves a level into the active area, with the room unchanged.
        world.entity_mut(root).insert(LdtkRuntimeIndex::from_project(&project(&[
            ("a1", "a"),
            ("b1", "b"),
            ("b2", "b"),
        ])));
        world.run_system_once(sync_ldtk_level_set).unwrap();
        assert_eq!(
            shown(&mut world),
            LevelSet::from_iids(["b1", "b2"]),
            "a reloaded project did not reach the bundle"
        );
    }
}
