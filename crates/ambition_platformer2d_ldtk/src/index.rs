//! The LDtk project's area table, read from the pure parser's project.
//!
//! It needs no `bevy_ecs_ldtk`: a simulation reads the areas' levels and
//! bounds from it, and only the painted bundles' `LevelSet` (the runtime's
//! `level_set_for`) is the plugin's.

use std::collections::BTreeMap;

use bevy::prelude::Component;

use super::{LdtkLevel, LdtkProject};

#[derive(Clone, Copy, Debug)]
pub struct LdtkAreaBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl LdtkAreaBounds {
    pub(crate) fn from_level(level: &LdtkLevel) -> Self {
        Self {
            min_x: level.world_x,
            min_y: level.world_y,
            max_x: level.world_x + level.px_wid,
            max_y: level.world_y + level.px_hei,
        }
    }

    pub(crate) fn include_level(&mut self, level: &LdtkLevel) {
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

    pub fn area_bounds(&self, area: &str) -> Option<LdtkAreaBounds> {
        self.area_bounds.get(area).copied()
    }
}

