use std::collections::BTreeMap;

use bevy::asset::{AssetServer, Handle};
use bevy::prelude::{Commands, Component, Query, Res, ResMut, Resource, With};
use bevy_ecs_ldtk::prelude::LevelSet;

use super::super::{
    default_sandbox_ldtk_path, sandbox_ldtk_asset_path, sandbox_ldtk_path, LdtkLevel, LdtkProject,
    SANDBOX_LDTK_ASSET,
};

#[derive(Resource, Clone, Debug)]
pub struct SandboxLdtkAsset(pub Handle<bevy_ecs_ldtk::assets::LdtkProject>);

pub fn load_ldtk_asset_handle(mut commands: Commands, asset_server: Res<AssetServer>) {
    let asset_path = sandbox_ldtk_asset_path();
    if asset_path == SANDBOX_LDTK_ASSET && sandbox_ldtk_path() != default_sandbox_ldtk_path() {
        eprintln!(
            "LDtk warning: configured map {} is outside the Bevy asset root; \
             Ambition's JSON loader will use it, but the bevy_ecs_ldtk runtime-spine handle \
             falls back to {}",
            sandbox_ldtk_path().display(),
            SANDBOX_LDTK_ASSET
        );
    }
    commands.insert_resource(SandboxLdtkAsset(asset_server.load(asset_path)));
}

#[derive(Component)]
pub struct SandboxLdtkWorldRoot;

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
    mut ldtk_worlds: Query<&mut LevelSet, With<SandboxLdtkWorldRoot>>,
) {
    let active_area = room_set.active_spec().id.clone();
    if !index.needs_level_set_sync(&active_area) {
        return;
    }
    let next_level_set = index.level_set_for(&active_area);
    index.set_active_area(active_area);
    for mut level_set in &mut ldtk_worlds {
        *level_set = next_level_set.clone();
    }
    index.mark_level_set_synced();
}
