//! Bevy + bevy_ecs_ldtk plugin glue and runtime-spine indexing for the
//! sandbox's LDtk integration.
//!
//! This submodule isolates everything that needs `bevy_ecs_ldtk` types
//! (PluginEntityInstance, LevelSet, LdtkEntity, asset Handle&lt;LdtkProject&gt;)
//! from the pure-Rust LDtk JSON parser / validator / surface compiler in
//! `super`. Step C of `docs/path_forward.md` calls for splitting
//! `ldtk_world.rs`; this is the bevy_ecs_ldtk-using half of that split.
//! Once the runtime-spine roadmap (`memory project_ldtk_roadmap`) is
//! complete enough for the JSON adapter to retire, this becomes the
//! collision authority too.

use std::collections::BTreeMap;

use bevy::asset::{AssetServer, Handle};
use bevy::prelude::{
    Added, App, Bundle, Commands, Component, Entity, Name, Plugin, Query, Res, ResMut, Resource,
    With,
};
use bevy_ecs_ldtk::prelude::{
    EntityInstance as PluginEntityInstance, LdtkEntity, LdtkEntityAppExt, LevelSet,
};

use ambition_engine as ae;

use super::{LdtkLevel, LdtkProject, SANDBOX_LDTK_ASSET};

/// Lightweight bundle registered for every Ambition-authored LDtk entity.
///
/// This makes `bevy_ecs_ldtk` the owner of LDtk entity lifecycle/identity
/// without letting the plugin render its default unregistered-entity
/// placeholders. Ambition systems then consume the spawned `EntityInstance`
/// component and attach gameplay semantics deliberately.
#[derive(Bundle, LdtkEntity, Default)]
pub struct AmbitionLdtkMarkerBundle {
    #[from_entity_instance]
    pub entity_instance: PluginEntityInstance,
    pub marker: AmbitionLdtkMarker,
}

#[derive(Component, Default, Clone, Copy, Debug)]
pub struct AmbitionLdtkMarker;

#[derive(Component, Clone, Debug)]
pub struct AmbitionLdtkEntity {
    pub iid: String,
    pub identifier: String,
    pub px: [i32; 2],
    pub size: [i32; 2],
    pub world: Option<[i32; 2]>,
}

impl AmbitionLdtkEntity {
    pub fn summary(&self) -> String {
        let world = self
            .world
            .map(|world| format!(" world=({}, {})", world[0], world[1]))
            .unwrap_or_default();
        format!(
            "{} {} px=({}, {}) size={}x{}{}",
            self.identifier, self.iid, self.px[0], self.px[1], self.size[0], self.size[1], world
        )
    }
}

#[derive(Resource, Default, Clone, Debug)]
pub struct LdtkRuntimeSpineStats {
    pub spawned_entities: usize,
    pub revision: u64,
    pub last_entity: String,
    pub sample_entity: String,
}

/// Ambition-facing role for a plugin-spawned LDtk entity.
///
/// These are deliberately narrower than the full LDtk identifier set. The
/// first promoted runtime-spine categories are the low-risk entities that
/// should be observable directly from `bevy_ecs_ldtk` before we migrate
/// collision and gameplay-heavy objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LdtkRuntimeRole {
    PlayerStart,
    LoadingZone,
    DebugLabel,
    CameraZone,
    Solid,
    Other,
}

impl LdtkRuntimeRole {
    pub fn from_identifier(identifier: &str) -> Self {
        match identifier {
            "PlayerStart" => Self::PlayerStart,
            "LoadingZone" => Self::LoadingZone,
            "DebugLabel" => Self::DebugLabel,
            "CameraZone" => Self::CameraZone,
            "Solid" => Self::Solid,
            _ => Self::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::PlayerStart => "player starts",
            Self::LoadingZone => "loading zones",
            Self::DebugLabel => "debug labels",
            Self::CameraZone => "camera zones",
            Self::Solid => "solids",
            Self::Other => "other",
        }
    }

    pub fn promoted(self) -> bool {
        !matches!(self, Self::Other)
    }
}

/// Typed Ambition collision component attached to plugin-spawned `Solid`
/// entities.
///
/// The first collision-heavy LDtk category to leave the JSON-only adapter path:
/// while `compose_runtime_area` still produces `ae::Block::solid()` entries for
/// the runtime collision world, every spawned `Solid` LDtk entity now also
/// carries this typed component so future systems can query ECS-side without
/// reparsing the LDtk file. Once the raw-LDtk-vs-runtime overlay (Step 2 of the
/// LDtk roadmap) verifies parity, the JSON path can be retired and these
/// components become collision authority.
#[derive(Component, Clone, Debug, Default)]
pub struct LdtkSolid {
    /// Top-left corner in LDtk-level-local pixel coordinates.
    pub level_px: [i32; 2],
    /// Width and height in pixels.
    pub size: [i32; 2],
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
pub struct AmbitionLdtkRegistrationPlugin;

impl Plugin for AmbitionLdtkRegistrationPlugin {
    fn build(&self, app: &mut App) {
        for identifier in AMBITION_LDTK_ENTITY_IDENTIFIERS {
            app.register_ldtk_entity::<AmbitionLdtkMarkerBundle>(identifier);
        }
    }
}

pub fn sync_plugin_spawned_ambition_entities(
    mut commands: Commands,
    mut stats: ResMut<LdtkRuntimeSpineStats>,
    query: Query<(Entity, &PluginEntityInstance), Added<PluginEntityInstance>>,
) {
    for (entity, instance) in &query {
        stats.spawned_entities = stats.spawned_entities.saturating_add(1);
        stats.revision = stats.revision.saturating_add(1);
        let ambition_entity = AmbitionLdtkEntity {
            iid: instance.iid.clone(),
            identifier: instance.identifier.clone(),
            px: [instance.px.x, instance.px.y],
            size: [instance.width, instance.height],
            world: instance.world_x.zip(instance.world_y).map(|(x, y)| [x, y]),
        };
        stats.last_entity = format!("{} {}", ambition_entity.identifier, ambition_entity.iid);
        stats.sample_entity = ambition_entity.summary();

        // Attach typed Ambition components for promoted collision-heavy LDtk
        // categories. The generic `AmbitionLdtkEntity` always lands; typed
        // sibling components let downstream systems query specifically without
        // identifier-string matching.
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((
            Name::new(format!(
                "LDtk {} {}",
                ambition_entity.identifier, ambition_entity.iid
            )),
            ambition_entity.clone(),
        ));
        // Plugin-spawned `Solid` LDtk entities get the typed `LdtkSolid`
        // component so the `LdtkRuntimeSolidIndex` collision authority can
        // pick them up without reparsing identifiers.
        if ambition_entity.identifier == "Solid" {
            entity_commands.insert(LdtkSolid {
                level_px: ambition_entity.px,
                size: ambition_entity.size,
            });
        }
    }
}

/// Rebuild the active-area-local index of promoted LDtk `Solid` entities.
///
/// Mirrors `rebuild_ldtk_runtime_spine_index` but only collects entities that
/// carry the typed `LdtkSolid` component, so future collision authority can
/// query a tight collision-only view without iterating the broader spine.
pub fn rebuild_ldtk_runtime_solid_index(
    room_set: Res<crate::rooms::RoomSet>,
    runtime_index: Res<LdtkRuntimeIndex>,
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

/// Rebuild an Ambition runtime-spine index from currently spawned LDtk entities.
///
/// `bevy_ecs_ldtk` owns the entity lifecycle; this system projects those
/// entities into active-area-local Ambition coordinates so gameplay/debug
/// systems can consume plugin output without reparsing the LDtk JSON file.
pub fn rebuild_ldtk_runtime_spine_index(
    room_set: Res<crate::rooms::RoomSet>,
    runtime_index: Res<LdtkRuntimeIndex>,
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
pub const AMBITION_LDTK_ENTITY_IDENTIFIERS: &[&str] = &[
    "PlayerStart",
    "Solid",
    "OneWayPlatform",
    "BlinkWall",
    "HazardBlock",
    "PogoOrb",
    "ReboundPad",
    "LoadingZone",
    "DamageVolume",
    "KinematicPath",
    "NpcSpawn",
    "PickupSpawn",
    "ChestSpawn",
    "BreakablePlatform",
    "BreakablePogoOrb",
    "EnemySpawn",
    "BossSpawn",
    "DebugLabel",
    "CameraZone",
    "StitchedBoundary",
];

#[derive(Resource, Clone, Debug)]
pub struct SandboxLdtkAsset(pub Handle<bevy_ecs_ldtk::assets::LdtkProject>);

pub fn load_ldtk_asset_handle(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SandboxLdtkAsset(asset_server.load(SANDBOX_LDTK_ASSET)));
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

