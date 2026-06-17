//! Resource indices summarizing plugin-spawned LDtk collision entities.
//!
//! Per-kind AABB lists rebuilt from the ECS each frame — `LdtkRuntimeSolidIndex`,
//! `LdtkRuntimeOneWayIndex`, `LdtkRuntimeDamageIndex` (plus the spine
//! roll-up `LdtkRuntimeSpineIndex`/`LdtkRuntimeSpineStats`) — so collision/parity
//! code can read the runtime-spine geometry without re-querying. Populated by
//! sibling `systems`; cross-checked against the JSON world in sibling `parity`.

use std::collections::BTreeMap;

use bevy::prelude::Resource;

use crate::engine_core as ae;

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

/// Active-area-local view of one promoted LDtk `Solid` entity.
#[derive(Clone, Debug, PartialEq)]
pub struct LdtkRuntimeSolid {
    pub iid: String,
    /// Top-left corner in active-area-local Ambition coordinates.
    pub min: ae::Vec2,
    pub size: ae::Vec2,
}

impl LdtkRuntimeSolid {
    pub fn aabb(&self) -> ae::Aabb {
        ae::aabb_from_min_size(self.min, self.size)
    }
}

/// Rebuilt every frame from plugin-spawned `Solid` LDtk entities carrying the
/// typed `LdtkSolid` component.
///
/// This is the parallel ECS view of solid collision authored in LDtk. The
/// runtime collision world (`ae::World::blocks`) is still populated by the
/// JSON adapter for now; once the raw-LDtk-vs-runtime overlay (Step 2 of the
/// LDtk roadmap) verifies parity, this index becomes the collision authority
/// and the JSON path retires.
#[derive(Resource, Default, Clone, Debug)]
pub struct LdtkRuntimeSolidIndex {
    pub active_area: String,
    pub solids: Vec<LdtkRuntimeSolid>,
    pub revision: u64,
}

impl LdtkRuntimeSolidIndex {
    pub fn count(&self) -> usize {
        self.solids.len()
    }

    pub(crate) fn replace_if_changed(&mut self, mut next: Self) {
        next.solids.sort_by(|a, b| a.iid.cmp(&b.iid));
        if self.active_area != next.active_area || self.solids != next.solids {
            next.revision = self.revision.saturating_add(1);
            *self = next;
        }
    }
}

/// Active-area-local view of one promoted LDtk `OneWayPlatform`.
#[derive(Clone, Debug, PartialEq)]
pub struct LdtkRuntimeOneWayPlatform {
    pub iid: String,
    pub min: ae::Vec2,
    pub size: ae::Vec2,
}

impl LdtkRuntimeOneWayPlatform {
    pub fn aabb(&self) -> ae::Aabb {
        ae::aabb_from_min_size(self.min, self.size)
    }
}

/// Rebuilt every frame from plugin-spawned `OneWayPlatform` entities.
/// Parallel ECS view of one-way platform collision authored in LDtk;
/// the JSON-derived `ae::World::blocks` is still the collision
/// authority pending the parity overlay.
#[derive(Resource, Default, Clone, Debug)]
pub struct LdtkRuntimeOneWayIndex {
    pub active_area: String,
    pub platforms: Vec<LdtkRuntimeOneWayPlatform>,
    pub revision: u64,
}

impl LdtkRuntimeOneWayIndex {
    pub fn count(&self) -> usize {
        self.platforms.len()
    }

    pub(crate) fn replace_if_changed(&mut self, mut next: Self) {
        next.platforms.sort_by(|a, b| a.iid.cmp(&b.iid));
        if self.active_area != next.active_area || self.platforms != next.platforms {
            next.revision = self.revision.saturating_add(1);
            *self = next;
        }
    }
}

/// Active-area-local view of one promoted LDtk `DamageVolume`.
#[derive(Clone, Debug, PartialEq)]
pub struct LdtkRuntimeDamageVolume {
    pub iid: String,
    pub min: ae::Vec2,
    pub size: ae::Vec2,
    pub damage: i32,
}

impl LdtkRuntimeDamageVolume {
    pub fn aabb(&self) -> ae::Aabb {
        ae::aabb_from_min_size(self.min, self.size)
    }
}

/// Rebuilt every frame from plugin-spawned `DamageVolume` /
/// `HazardBlock` entities. Parallel ECS view of damage authored in
/// LDtk; the JSON-derived blocks are still the gameplay authority.
#[derive(Resource, Default, Clone, Debug)]
pub struct LdtkRuntimeDamageIndex {
    pub active_area: String,
    pub volumes: Vec<LdtkRuntimeDamageVolume>,
    pub revision: u64,
}

impl LdtkRuntimeDamageIndex {
    pub fn count(&self) -> usize {
        self.volumes.len()
    }

    pub(crate) fn replace_if_changed(&mut self, mut next: Self) {
        next.volumes.sort_by(|a, b| a.iid.cmp(&b.iid));
        if self.active_area != next.active_area || self.volumes != next.volumes {
            next.revision = self.revision.saturating_add(1);
            *self = next;
        }
    }
}
