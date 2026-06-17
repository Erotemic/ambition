//! LDtk asset handles + per-area `LevelSet` selection (bevy_ecs_ldtk side).
//!
//! Holds the loaded-project handles (`SandboxLdtkAsset`/`IntroLdtkAsset`/
//! `CutRopeLdtkAsset`), world-root markers, and `LdtkRuntimeIndex` — the
//! area→level-IID/`LevelSet`/bounds map that drives streaming. Systems:
//! `load_ldtk_asset_handle` (kick the load) and `sync_ldtk_level_set` (swap the
//! visible `LevelSet` when the active area changes).

use std::collections::BTreeMap;

use bevy::asset::{AssetServer, Handle};
use bevy::prelude::{Commands, Component, Query, Res, ResMut, Resource, Transform, Vec3, With};
use bevy_ecs_ldtk::prelude::LevelSet;

use crate::config::WORLD_Z_BLOCK;

use super::super::{sandbox_ldtk_asset_path, LdtkLevel, LdtkProject};

#[derive(Resource, Clone, Debug)]
pub struct SandboxLdtkAsset(pub Handle<bevy_ecs_ldtk::assets::LdtkProject>);

/// Bevy asset handle for the secondary intro LDtk file. Loaded
/// alongside the main sandbox project so `bevy_ecs_ldtk` can render
/// the intro's painted tile layers — its asset loader is independent
/// of Ambition's merged JSON loader (`LdtkProject::load_default`),
/// so each file needs its own bundle.
#[derive(Resource, Clone, Debug)]
pub struct IntroLdtkAsset(pub Handle<bevy_ecs_ldtk::assets::LdtkProject>);

/// Bevy asset handle for the cut-rope boss LDtk side world.
#[derive(Resource, Clone, Debug)]
pub struct CutRopeLdtkAsset(pub Handle<bevy_ecs_ldtk::assets::LdtkProject>);

pub fn load_ldtk_asset_handle(mut commands: Commands, asset_server: Res<AssetServer>) {
    // `bevy_ecs_ldtk`'s asset loader is always rooted at the default
    // Bevy asset source — modded LDtk paths still feed Ambition's JSON
    // loader (via the SandboxAssetCatalog), but the `LdtkWorldBundle`
    // handle here is fixed to the canonical asset-path constant.
    let asset_path = sandbox_ldtk_asset_path();
    commands.insert_resource(SandboxLdtkAsset(asset_server.load(asset_path)));
    // Secondary intro LDtk asset. Co-located in the same Bevy asset
    // directory; bevy_ecs_ldtk loads it independently so each file's
    // LdtkWorldBundle can render its own levels.
    commands.insert_resource(IntroLdtkAsset(
        asset_server.load("ambition/worlds/intro.ldtk"),
    ));
    commands.insert_resource(CutRopeLdtkAsset(
        asset_server.load("ambition/worlds/you_have_to_cut_the_rope.ldtk"),
    ));
}

/// Marker for the main sandbox LDtk world root entity. The sandbox
/// bundle's `LevelSet` carries the iids of the active area IFF the
/// area belongs to `sandbox.ldtk`; otherwise the set is empty and
/// nothing renders.
#[derive(Component)]
pub struct SandboxLdtkWorldRoot;

/// Marker for the secondary intro LDtk world root entity. Same
/// LevelSet logic as [`SandboxLdtkWorldRoot`] — the intro bundle's
/// set is non-empty only while the active area is in `intro.ldtk`.
#[derive(Component)]
pub struct IntroLdtkWorldRoot;

/// Marker for the cut-rope boss LDtk world root entity.
#[derive(Component)]
pub struct CutRopeLdtkWorldRoot;

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

#[derive(Resource, Clone, Debug)]
pub struct LdtkRuntimeIndex {
    active_area: String,
    area_level_iids: BTreeMap<String, Vec<String>>,
    area_bounds: BTreeMap<String, LdtkAreaBounds>,
    revision: u64,
    synced_revision: u64,
}

impl LdtkRuntimeIndex {
    pub fn from_project(project: &LdtkProject, start_area: impl Into<String>) -> Self {
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
            active_area: start_area.into(),
            area_level_iids,
            area_bounds,
            revision: 1,
            synced_revision: 0,
        }
    }

    pub fn active_area(&self) -> &str {
        &self.active_area
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

    pub fn active_area_origin(&self) -> [i32; 2] {
        self.area_bounds(self.active_area())
            .map(|bounds| [bounds.min_x, bounds.min_y])
            .unwrap_or([0, 0])
    }

    pub fn set_active_area(&mut self, area: impl Into<String>) {
        self.active_area = area.into();
    }

    pub fn replace_from_project(&mut self, project: &LdtkProject, active_area: impl Into<String>) {
        let replacement = Self::from_project(project, active_area);
        self.active_area = replacement.active_area;
        self.area_level_iids = replacement.area_level_iids;
        self.area_bounds = replacement.area_bounds;
        self.revision = self.revision.saturating_add(1);
        self.synced_revision = self.synced_revision.min(self.revision.saturating_sub(1));
    }

    pub fn needs_level_set_sync(&self, area: &str) -> bool {
        self.active_area() != area || self.synced_revision != self.revision
    }

    pub fn mark_level_set_synced(&mut self) {
        self.synced_revision = self.revision;
    }
}

pub fn sync_ldtk_level_set(
    room_set: Res<crate::rooms::RoomSet>,
    mut index: ResMut<LdtkRuntimeIndex>,
    mut ldtk_worlds: Query<
        &mut LevelSet,
        bevy::prelude::Or<(
            With<SandboxLdtkWorldRoot>,
            With<IntroLdtkWorldRoot>,
            With<CutRopeLdtkWorldRoot>,
        )>,
    >,
) {
    let active_area = room_set.active_spec().id.clone();
    if !index.needs_level_set_sync(&active_area) {
        return;
    }
    let next_level_set = index.level_set_for(&active_area);
    index.set_active_area(active_area);
    // Both bundles get the same LevelSet. bevy_ecs_ldtk only spawns
    // levels whose iids exist in the bundle's loaded asset, so the
    // sandbox bundle renders when the active area is in sandbox.ldtk
    // and the intro bundle renders when the active area is in
    // intro.ldtk — no cross-talk because iids are unique per file.
    for mut level_set in &mut ldtk_worlds {
        *level_set = next_level_set.clone();
    }
    index.mark_level_set_synced();
}

/// Position the `LdtkWorldBundle` root entity so the rendered LDtk
/// Tiles layer aligns with Ambition's centered active-area frame.
///
/// **Coordinate reconciliation, ADR 0015 §Coordinate-frame
/// reconciliation:** `bevy_ecs_ldtk` renders Tiles in raw LDtk
/// world-pixel space — each level sits at its own world origin and
/// every tile inside is at level-local px coords. With
/// `LevelSpawnBehavior::UseZeroTranslation` (the default + our
/// setting) the active level sits at the bundle's origin and tiles
/// render upward + rightward from (0,0) in Bevy's Y-up.
///
/// Ambition's renderer (`world_to_bevy`) centers each active area
/// at the Bevy camera origin: an `ae::Vec2(0,0)` (engine top-left)
/// becomes `(-world.size.x/2, +world.size.y/2)`. The bottom-left
/// of the room becomes `(-world.size.x/2, -world.size.y/2)`.
///
/// To make bevy_ecs_ldtk's tile origin (the level's bottom-left)
/// match Ambition's bottom-left, translate the entire
/// `LdtkWorldBundle` root by that offset. Z is set just behind
/// `WORLD_Z_BLOCK` so Ambition's existing block visuals draw on
/// top of (or alongside) the tile background.
///
/// AMBITION_REVIEW(spatial): this is the single seam where LDtk
/// world coords meet Ambition's centered frame. Re-check any time
/// the level layout changes (room dimensions, `world_to_bevy`,
/// LdtkSettings::level_spawn_behavior).
pub fn sync_ldtk_world_transform(
    room_set: Res<crate::rooms::RoomSet>,
    mut ldtk_worlds: Query<
        &mut Transform,
        bevy::prelude::Or<(
            With<SandboxLdtkWorldRoot>,
            With<IntroLdtkWorldRoot>,
            With<CutRopeLdtkWorldRoot>,
        )>,
    >,
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
