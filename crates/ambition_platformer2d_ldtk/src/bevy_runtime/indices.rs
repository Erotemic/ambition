//! Resource indices summarizing plugin-spawned LDtk entities.
//!
//! `LdtkRuntimeSpineIndex` lists the promoted entities of the active area, and
//! `LdtkRuntimeSpineStats` counts the spawns. The debug overlay and the headless
//! summary read them. Populated by sibling `systems`.

use std::collections::BTreeMap;

use bevy::prelude::Resource;

use ambition_platformer2d_core as ae;

use super::components::LdtkRuntimeRole;

#[derive(Resource, Default, Clone, Debug)]
pub struct LdtkRuntimeSpineStats {
    pub spawned_entities: usize,
    pub revision: u64,
    pub last_entity: String,
    pub sample_entity: String,
}

/// Runtime-spine view of a plugin-spawned LDtk entity in active-area-local
/// Ambition coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct LdtkRuntimeSpineEntity {
    pub iid: String,
    pub identifier: String,
    pub role: LdtkRuntimeRole,
    pub min: ae::Vec2,
    pub size: ae::Vec2,
}

impl LdtkRuntimeSpineEntity {
    pub fn aabb(&self) -> ae::Aabb {
        ae::aabb_from_min_size(self.min, self.size)
    }
}

/// Rebuilt every frame from plugin-spawned LDtk entities.
///
/// This is the first place where direct `bevy_ecs_ldtk` output becomes an
/// Ambition runtime resource. For now it is used for debug/authoring overlays
/// and HUD health checks; future patches should let promoted categories drive
/// gameplay systems directly from this spine instead of the JSON adapter.
#[derive(Resource, Default, Clone, Debug)]
pub struct LdtkRuntimeSpineIndex {
    pub active_area: String,
    pub entities: Vec<LdtkRuntimeSpineEntity>,
    pub promoted_counts: BTreeMap<LdtkRuntimeRole, usize>,
    pub revision: u64,
}

impl LdtkRuntimeSpineIndex {
    pub fn promoted_summary(&self) -> String {
        let mut parts = Vec::new();
        for role in [
            LdtkRuntimeRole::PlayerStart,
            LdtkRuntimeRole::LoadingZone,
            LdtkRuntimeRole::DebugLabel,
            LdtkRuntimeRole::CameraZone,
            LdtkRuntimeRole::Solid,
            LdtkRuntimeRole::OneWayPlatform,
            LdtkRuntimeRole::DamageVolume,
        ] {
            let count = self.promoted_counts.get(&role).copied().unwrap_or(0);
            parts.push(format!("{} {}", count, role.label()));
        }
        parts.join(", ")
    }

    pub(crate) fn replace_if_changed(&mut self, mut next: Self) {
        next.entities.sort_by(|a, b| a.iid.cmp(&b.iid));
        if self.active_area != next.active_area || self.entities != next.entities {
            next.revision = self.revision.saturating_add(1);
            *self = next;
        }
    }
}
