//! LDtk world-composition adapter and validator for the sandbox.
//!
//! Ambition keeps its gameplay model typed in Rust. LDtk is an authoring
//! frontend: this module validates the subset of LDtk entities Ambition
//! currently understands, registers those entities with `bevy_ecs_ldtk`, and
//! now materializes Ambition runtime rooms directly from LDtk-authored data.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use bevy::asset::{AssetServer, Handle};
use bevy::prelude::{
    Added, App, Bundle, Commands, Component, Entity, Name, Plugin, Query, Res, ResMut, Resource,
    Time, With,
};
use bevy_ecs_ldtk::prelude::{
    EntityInstance as PluginEntityInstance, LdtkEntity, LdtkEntityAppExt, LdtkFields, LevelSet,
};
use serde::Deserialize;
use serde_json::Value;

use ambition_engine as ae;

use crate::rooms::{LoadingZone, LoadingZoneActivation, RoomLink, RoomSet, RoomSpec};

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

    fn replace_if_changed(&mut self, mut next: Self) {
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

    fn replace_if_changed(&mut self, mut next: Self) {
        next.solids.sort_by(|a, b| a.iid.cmp(&b.iid));
        if self.active_area != next.active_area || self.solids != next.solids {
            next.revision = self.revision.saturating_add(1);
            *self = next;
        }
    }
}

/// Collision behavior contributed by an LDtk-authored `Surface`.
///
/// `Surface` is the authoring-time primitive: designers place a single
/// rectangular entity and tweak its `collision`, `breakability`, `contact`,
/// and `respawn` fields rather than swapping between a zoo of one-purpose
/// entities. The compile step translates this into typed engine
/// `Block`/`Breakable`/contact data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SurfaceCollision {
    /// Pure trigger volume; bodies pass through.
    #[default]
    None,
    /// Hard wall on both axes (legacy `Solid`).
    Solid,
    /// One-way landing: solid only when crossed from above (legacy `OneWayPlatform`).
    OneWayUp,
    /// Soft blink wall: solid until the player has the matching blink upgrade.
    BlinkSoft,
    /// Hard blink wall: solid until the player has the stronger blink upgrade.
    BlinkHard,
}

/// Whether and how a `Surface` can be destroyed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SurfaceBreakability {
    #[default]
    Indestructible,
    BreakOnHit,
    BreakOnStand,
    BreakOnHitOrStand,
}

/// Side-effect applied to bodies that touch a `Surface`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SurfaceContact {
    #[default]
    None,
    /// Damage / hazard reset (legacy `HazardBlock`).
    Damage { amount: i32 },
    /// Refreshes pogo / movement resources (legacy `PogoOrb`).
    PogoRefresh,
    /// Applies a fixed impulse on contact (legacy `ReboundPad`).
    Rebound { impulse: ae::Vec2 },
}

/// When a destroyed `Surface` returns.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SurfaceRespawn {
    #[default]
    Never,
    OnRoomReload,
    AfterSeconds(f32),
}

/// Typed intermediate representation for a single LDtk `Surface` (or legacy
/// alias such as `Solid`, `OneWayPlatform`, `BlinkWall`, `HazardBlock`,
/// `PogoOrb`, `ReboundPad`, `Breakable`).
///
/// This is the authoring-side data parsed straight out of LDtk JSON. The
/// compile step (`compile_surface`) lowers it into engine-native runtime
/// pieces (`ae::Block`, `ae::RoomObject`) so collision/contact systems never
/// have to reparse strings or JSON.
#[derive(Clone, Debug, PartialEq)]
pub struct LdtkSurfaceSpec {
    /// LDtk-stable instance id.
    pub iid: String,
    /// Display name (defaults to identifier when not provided).
    pub name: String,
    /// Top-left in active-area-local Ambition coordinates (post-offset).
    pub min: ae::Vec2,
    /// Width and height in pixels.
    pub size: ae::Vec2,
    pub collision: SurfaceCollision,
    pub breakability: SurfaceBreakability,
    pub contact: SurfaceContact,
    pub respawn: SurfaceRespawn,
    /// Hit points for breakable surfaces. Ignored when `Indestructible`.
    pub max_hp: i32,
}

impl LdtkSurfaceSpec {
    /// Build an indestructible solid wall with no contact behavior. Convenient
    /// for tests and migration shims.
    pub fn solid_wall(
        iid: impl Into<String>,
        name: impl Into<String>,
        min: ae::Vec2,
        size: ae::Vec2,
    ) -> Self {
        Self {
            iid: iid.into(),
            name: name.into(),
            min,
            size,
            collision: SurfaceCollision::Solid,
            breakability: SurfaceBreakability::Indestructible,
            contact: SurfaceContact::None,
            respawn: SurfaceRespawn::Never,
            max_hp: 0,
        }
    }
}

/// Result of compiling a single `LdtkSurfaceSpec` into runtime engine data.
#[derive(Clone, Debug, Default)]
pub struct SurfaceCompiled {
    pub blocks: Vec<ae::Block>,
    pub objects: Vec<ae::RoomObject>,
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
        // Plugin-spawned LDtk entities that resolve to a `Solid` collision
        // surface get the typed `LdtkSolid` component so the
        // `LdtkRuntimeSolidIndex` collision authority can pick them up
        // without reparsing identifiers. We currently emit it for the
        // legacy `Solid` identifier; native `Surface` entities are picked up
        // here too by a runtime field check (collision string == "Solid").
        let is_solid_collision = ambition_entity.identifier == "Solid"
            || (ambition_entity.identifier == "Surface"
                && instance
                    .get_string_field("collision")
                    .map(|value| value.as_str() == "Solid")
                    .unwrap_or(false));
        if is_solid_collision {
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
    // `Surface` is the preferred future authoring primitive for rectangular
    // gameplay geometry / collision-like volumes. The identifiers listed below
    // it (`Solid`, `OneWayPlatform`, `BlinkWall`, `HazardBlock`, `PogoOrb`,
    // `ReboundPad`, `Breakable`) are kept as legacy aliases that compile down
    // to the same `LdtkSurfaceSpec` representation.
    "Surface",
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
    "Breakable",
    "EnemySpawn",
    "BossSpawn",
    "DebugLabel",
    "CameraZone",
    "StitchedBoundary",
];

/// LDtk identifiers handled by the `Surface` compile pipeline.
///
/// `Surface` is the canonical authoring primitive for rectangular collision-like
/// gameplay geometry; the others are legacy aliases retained while existing
/// LDtk projects migrate. Adding to this list also registers the identifier
/// for `LdtkSurfaceSpec` parsing in `entity_to_runtime`.
pub const SURFACE_LIKE_IDENTIFIERS: &[&str] = &[
    "Surface",
    "Solid",
    "OneWayPlatform",
    "BlinkWall",
    "HazardBlock",
    "PogoOrb",
    "ReboundPad",
    "Breakable",
];

/// True if `identifier` should be parsed as an `LdtkSurfaceSpec`.
pub fn is_surface_like_identifier(identifier: &str) -> bool {
    SURFACE_LIKE_IDENTIFIERS.contains(&identifier)
}

pub const SANDBOX_LDTK_ASSET: &str = "ambition/worlds/sandbox.ldtk";

pub fn sandbox_ldtk_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(SANDBOX_LDTK_ASSET)
}

pub fn sandbox_ldtk_modified_time() -> Result<SystemTime, String> {
    let path = sandbox_ldtk_path();
    fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| {
            format!(
                "could not read LDtk modified time for {}: {error}",
                path.display()
            )
        })
}

#[derive(Resource, Clone, Debug)]
pub struct LdtkHotReloadState {
    pub pending: bool,
    pub auto_apply: bool,
    pub poll_timer: f32,
    pub last_modified: Option<SystemTime>,
    pub last_status: String,
    pub last_errors: Vec<String>,
    pub applied_count: u32,
}

impl Default for LdtkHotReloadState {
    fn default() -> Self {
        Self {
            pending: false,
            auto_apply: false,
            poll_timer: 0.0,
            last_modified: None,
            last_status: "LDtk hot reload idle".to_string(),
            last_errors: Vec::new(),
            applied_count: 0,
        }
    }
}

impl LdtkHotReloadState {
    pub fn from_current_file() -> Self {
        let mut state = Self::default();
        match sandbox_ldtk_modified_time() {
            Ok(modified) => {
                state.last_modified = Some(modified);
                state.last_status = if cfg!(feature = "dev_hot_reload") {
                    "LDtk hot reload watching; press F11 to apply, F12 toggles auto-apply"
                        .to_string()
                } else {
                    "LDtk hot reload polling; run with --features dev_hot_reload for Bevy file watching too".to_string()
                };
            }
            Err(error) => {
                state.last_status = error;
            }
        }
        state
    }

    pub fn mark_pending(&mut self, modified: SystemTime) {
        self.last_modified = Some(modified);
        self.pending = true;
        self.last_errors.clear();
        self.last_status = "LDtk change detected; press F11 to apply".to_string();
    }

    pub fn mark_applied(&mut self, room: &str) {
        self.pending = false;
        self.applied_count = self.applied_count.saturating_add(1);
        self.last_errors.clear();
        self.last_status = format!("LDtk reload applied to '{room}' (#{})", self.applied_count);
    }

    pub fn mark_failed(&mut self, errors: Vec<String>) {
        self.pending = false;
        self.last_errors = errors;
        let first = self
            .last_errors
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown LDtk reload failure".to_string());
        self.last_status = format!("LDtk reload rejected: {first}");
    }
}

pub fn poll_ldtk_file_changes(time: Res<Time>, mut state: ResMut<LdtkHotReloadState>) {
    state.poll_timer -= time.delta_secs();
    if state.poll_timer > 0.0 {
        return;
    }
    state.poll_timer = 0.35;
    let Ok(modified) = sandbox_ldtk_modified_time() else {
        return;
    };
    let changed = state
        .last_modified
        .map(|last| modified > last)
        .unwrap_or(false);
    if changed {
        state.mark_pending(modified);
    } else if state.last_modified.is_none() {
        state.last_modified = Some(modified);
    }
}

const AMBITION_LAYER: &str = "Ambition";
const GRID: i32 = 16;

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

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkProject {
    #[serde(rename = "jsonVersion")]
    pub json_version: String,
    #[serde(default)]
    pub levels: Vec<LdtkLevel>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkLevel {
    pub identifier: String,
    pub iid: String,
    #[serde(rename = "worldX")]
    pub world_x: i32,
    #[serde(rename = "worldY")]
    pub world_y: i32,
    #[serde(rename = "pxWid")]
    pub px_wid: i32,
    #[serde(rename = "pxHei")]
    pub px_hei: i32,
    #[serde(default, rename = "fieldInstances")]
    pub field_instances: Vec<LdtkFieldInstance>,
    #[serde(default, rename = "layerInstances")]
    pub layer_instances: Vec<LdtkLayerInstance>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkLayerInstance {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(default, rename = "entityInstances")]
    pub entity_instances: Vec<LdtkEntityInstance>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkEntityInstance {
    pub iid: String,
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(default, rename = "__pivot")]
    pub pivot: Vec<f32>,
    pub px: [i32; 2],
    pub width: i32,
    pub height: i32,
    #[serde(default, rename = "fieldInstances")]
    pub field_instances: Vec<LdtkFieldInstance>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkFieldInstance {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(rename = "__value")]
    pub value: Value,
    #[serde(default, rename = "realEditorValues")]
    pub real_editor_values: Vec<Value>,
}

#[derive(Clone, Debug, Default)]
pub struct LdtkValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl LdtkValidationReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn print_to_stderr(&self) {
        for warning in &self.warnings {
            eprintln!("LDtk validation warning: {warning}");
        }
        for error in &self.errors {
            eprintln!("LDtk validation error: {error}");
        }
    }
}

impl LdtkProject {
    pub fn load_embedded() -> Self {
        serde_json::from_str(include_str!("../assets/ambition/worlds/sandbox.ldtk"))
            .expect("embedded assets/ambition/worlds/sandbox.ldtk should parse")
    }

    pub fn load_from_disk() -> Result<Self, String> {
        let path = sandbox_ldtk_path();
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("could not read LDtk project {}: {error}", path.display()))?;
        serde_json::from_str(&text)
            .map_err(|error| format!("could not parse LDtk project {}: {error}", path.display()))
    }

    pub fn validate(&self) -> LdtkValidationReport {
        let mut report = LdtkValidationReport::default();
        if self.json_version.trim().is_empty() {
            report
                .errors
                .push("project jsonVersion is empty".to_string());
        }
        if self.levels.is_empty() {
            report.errors.push("project has no levels".to_string());
            return report;
        }

        let mut level_ids = BTreeSet::new();
        let mut player_starts_by_area: BTreeMap<String, usize> = BTreeMap::new();
        let mut level_count_by_area: BTreeMap<String, usize> = BTreeMap::new();

        for level in &self.levels {
            if !level_ids.insert(level.identifier.clone()) {
                report.errors.push(format!(
                    "duplicate LDtk level identifier '{}'",
                    level.identifier
                ));
            }
            if level.px_wid <= 0 || level.px_hei <= 0 {
                report.errors.push(format!(
                    "level '{}' has non-positive dimensions {}x{}",
                    level.identifier, level.px_wid, level.px_hei
                ));
            }
            if level.world_x % GRID != 0 || level.world_y % GRID != 0 {
                report.warnings.push(format!(
                    "level '{}' world origin ({}, {}) is not aligned to {}px grid",
                    level.identifier, level.world_x, level.world_y, GRID
                ));
            }
            let active_area = level.active_area();
            if level
                .raw_active_area()
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                report.errors.push(format!(
                    "level '{}' has a blank activeArea level field; LDtk editor round-trips must preserve this field",
                    level.identifier
                ));
            }
            *level_count_by_area.entry(active_area.clone()).or_default() += 1;

            let Some(layer) = level.ambition_layer() else {
                report.errors.push(format!(
                    "level '{}' is missing '{AMBITION_LAYER}' entity layer",
                    level.identifier
                ));
                continue;
            };

            let solids = layer
                .entity_instances
                .iter()
                .filter(|entity| entity.identifier == "Solid")
                .collect::<Vec<_>>();

            for entity in &layer.entity_instances {
                if !known_entity(&entity.identifier) {
                    report.errors.push(format!(
                        "level '{}' has unsupported Ambition entity '{}' ({})",
                        level.identifier, entity.identifier, entity.iid
                    ));
                }
                if entity.width <= 0 || entity.height <= 0 {
                    report.errors.push(format!(
                        "level '{}' entity '{}' ({}) has non-positive dimensions {}x{}",
                        level.identifier,
                        entity.identifier,
                        entity.iid,
                        entity.width,
                        entity.height
                    ));
                }
                if entity.px[0] < 0
                    || entity.px[1] < 0
                    || entity.px[0] + entity.width > level.px_wid
                    || entity.px[1] + entity.height > level.px_hei
                {
                    report.errors.push(format!(
                        "level '{}' entity '{}' ({}) is outside level bounds",
                        level.identifier, entity.identifier, entity.iid
                    ));
                }
                if !pivot_is_top_left(entity) {
                    report.errors.push(format!(
                        "level '{}' entity '{}' ({}) must use top-left pivot [0, 0] for Ambition conversion",
                        level.identifier, entity.identifier, entity.iid
                    ));
                }
                match entity.identifier.as_str() {
                    "PlayerStart" => {
                        *player_starts_by_area
                            .entry(active_area.clone())
                            .or_default() += 1;
                    }
                    "LoadingZone" => {
                        if field_string(entity, "id").is_none() {
                            report.errors.push(format!(
                                "LoadingZone {} is missing string field 'id'",
                                entity.iid
                            ));
                        }
                        if field_string(entity, "target_room").is_none()
                            || field_string(entity, "target_zone").is_none()
                        {
                            report.errors.push(format!(
                                "LoadingZone {} requires target_room and target_zone fields",
                                entity.iid
                            ));
                        }
                        if field_string(entity, "activation").unwrap_or_else(|| "Door".to_string())
                            == "EdgeExit"
                        {
                            if !entity_touches_level_edge(entity, level) {
                                report.errors.push(format!(
                                    "EdgeExit LoadingZone {} in level '{}' must touch a level edge",
                                    entity.iid, level.identifier
                                ));
                            }
                            for solid in &solids {
                                if rects_strict_intersect(entity_rect(entity), entity_rect(solid)) {
                                    report.errors.push(format!(
                                        "EdgeExit LoadingZone {} in level '{}' overlaps solid {} ({}); split the wall or move the zone so the exit is physically reachable",
                                        entity.iid, level.identifier, solid.identifier, solid.iid
                                    ));
                                }
                            }
                        }
                    }
                    "DebugLabel" => {
                        if field_string(entity, "text").is_none() {
                            report
                                .errors
                                .push(format!("DebugLabel {} requires text field", entity.iid));
                        }
                    }
                    _ => {}
                }
                // Surface-shaped entities are validated by parsing into the
                // typed `LdtkSurfaceSpec` and running the same compile path
                // that produces runtime data. This is the single source of
                // truth for collision/breakability/contact/respawn field
                // combinations across the canonical `Surface` and its legacy
                // identifier aliases.
                if is_surface_like_identifier(&entity.identifier)
                    && entity.width > 0
                    && entity.height > 0
                {
                    let placeholder_min = ae::Vec2::ZERO;
                    let placeholder_size =
                        ae::Vec2::new(entity.width as f32, entity.height as f32);
                    let name = field_string(entity, "name")
                        .unwrap_or_else(|| entity.identifier.clone());
                    match parse_surface_spec(entity, placeholder_min, placeholder_size, name)
                        .and_then(|spec| compile_surface(&spec))
                    {
                        Ok(_) => {}
                        Err(error) => report.errors.push(format!(
                            "{} {}: {error}",
                            entity.identifier, entity.iid
                        )),
                    }
                }
                for field in &entity.field_instances {
                    if field.value.is_null() {
                        continue;
                    }
                    if field.real_editor_values.is_empty() {
                        report.warnings.push(format!(
                            "{} {} field '{}' has __value but empty realEditorValues; this is what LDtk writes for unedited fields that inherit a defaultOverride. Run `python3 tools/repair_ambition_ldtk.py --in-place <ldtk>` to normalize before committing.",
                            entity.identifier, entity.iid, field.identifier
                        ));
                    }
                }
            }
        }

        for (area, count) in player_starts_by_area {
            if count != 1 {
                report.errors.push(format!(
                    "active area '{area}' has {count} PlayerStart entities; expected exactly 1"
                ));
            }
        }
        for area in level_count_by_area.keys() {
            if !self.area_has_player_start(area) {
                report
                    .errors
                    .push(format!("active area '{area}' has no PlayerStart"));
            }
        }

        report
    }

    /// Build the sandbox runtime room set from LDtk.
    ///
    /// This is a direct LDtk-native runtime builder. LDtk does not
    /// round-trip through a RON-shaped world manifest before it becomes
    /// playable data. `RoomSet` remains the runtime graph, but LDtk
    /// materializes `RoomSpec`, `ae::World`, loading zones, and graph links
    /// directly here.
    pub fn to_room_set(&self) -> Result<RoomSet, Vec<String>> {
        let report = self.validate();
        if !report.is_ok() {
            return Err(report.errors);
        }

        let mut area_levels: BTreeMap<String, Vec<&LdtkLevel>> = BTreeMap::new();
        for level in &self.levels {
            area_levels
                .entry(level.active_area())
                .or_default()
                .push(level);
        }

        let start_room = if area_levels.contains_key("central_hub_complex") {
            "central_hub_complex".to_string()
        } else {
            area_levels
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| "central_hub_complex".to_string())
        };

        let links = self.collect_room_links();
        let mut rooms = Vec::new();
        for (area_id, levels) in area_levels {
            rooms.push(self.compose_runtime_area(&area_id, &levels)?);
        }
        Ok(RoomSet::from_parts(start_room, rooms, links))
    }

    fn collect_room_links(&self) -> Vec<RoomLink> {
        let mut links = Vec::new();
        for level in &self.levels {
            let from_room = level.active_area();
            let Some(layer) = level.ambition_layer() else {
                continue;
            };
            for entity in &layer.entity_instances {
                if entity.identifier != "LoadingZone" {
                    continue;
                }
                let Some(target_room) = field_string(entity, "target_room") else {
                    continue;
                };
                let Some(target_zone) = field_string(entity, "target_zone") else {
                    continue;
                };
                links.push(RoomLink {
                    from_room: from_room.clone(),
                    from_zone: field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
                    to_room: target_room,
                    to_zone: target_zone,
                    bidirectional: field_bool(entity, "bidirectional").unwrap_or(false),
                });
            }
        }
        links
    }

    fn compose_runtime_area(
        &self,
        area_id: &str,
        levels: &[&LdtkLevel],
    ) -> Result<RoomSpec, Vec<String>> {
        let mut errors = Vec::new();
        let min_x = levels.iter().map(|level| level.world_x).min().unwrap_or(0) as f32;
        let min_y = levels.iter().map(|level| level.world_y).min().unwrap_or(0) as f32;
        let max_x = levels
            .iter()
            .map(|level| level.world_x + level.px_wid)
            .max()
            .unwrap_or(0) as f32;
        let max_y = levels
            .iter()
            .map(|level| level.world_y + level.px_hei)
            .max()
            .unwrap_or(0) as f32;
        let mut spawn = None;
        let mut blocks = Vec::new();
        let mut loading_zones = Vec::new();
        let mut objects = Vec::new();
        for level in levels {
            // AMBITION_REVIEW(spatial): LDtk world coordinates are flattened into
            // active-area-local Ambition coordinates here. Wall openings, edge
            // exits, transition arrivals, and camera bounds all depend on this
            // convention staying stable.
            let offset = ae::Vec2::new(level.world_x as f32 - min_x, level.world_y as f32 - min_y);
            let Some(layer) = level.ambition_layer() else {
                errors.push(format!(
                    "level '{}' missing Ambition layer",
                    level.identifier
                ));
                continue;
            };
            for entity in &layer.entity_instances {
                match entity_to_runtime(entity, offset) {
                    Ok(emission) => {
                        if emission.ignored {
                            continue;
                        }
                        if let Some(value) = emission.spawn {
                            spawn = Some(value);
                        }
                        blocks.extend(emission.blocks);
                        loading_zones.extend(emission.zones);
                        objects.extend(emission.objects);
                    }
                    Err(error) => {
                        errors.push(format!("{} {}: {error}", entity.identifier, entity.iid))
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(RoomSpec {
            id: area_id.to_string(),
            world: ae::World {
                name: format!("Ambition: {}", area_id.replace('_', " ")),
                size: ae::Vec2::new(max_x - min_x, max_y - min_y),
                spawn: spawn.unwrap_or_else(|| ae::Vec2::new(96.0, 96.0)),
                blocks,
                objects,
            },
            loading_zones,
        })
    }

    fn area_has_player_start(&self, area: &str) -> bool {
        self.levels.iter().any(|level| {
            level.active_area() == area
                && level
                    .ambition_layer()
                    .map(|layer| {
                        layer
                            .entity_instances
                            .iter()
                            .any(|entity| entity.identifier == "PlayerStart")
                    })
                    .unwrap_or(false)
        })
    }
}

impl LdtkLevel {
    fn raw_active_area(&self) -> Option<String> {
        self.field_string("activeArea")
    }

    fn active_area(&self) -> String {
        self.raw_active_area()
            .map(|area| area.trim().to_string())
            .filter(|area| !area.is_empty())
            .unwrap_or_else(|| self.identifier.clone())
    }

    fn ambition_layer(&self) -> Option<&LdtkLayerInstance> {
        self.layer_instances
            .iter()
            .find(|layer| layer.identifier == AMBITION_LAYER)
    }

    fn field_string(&self, name: &str) -> Option<String> {
        field_value(&self.field_instances, name).and_then(value_to_string)
    }
}

/// Aggregated runtime emission for one LDtk entity instance.
///
/// LDtk entities historically mapped 1:1 to a single emitted runtime piece.
/// With `Surface`, a single LDtk entity can compile into multiple emissions
/// (e.g. a Block for static collision plus an Object for breakable lifetime),
/// so the conversion API yields a struct rather than a one-of enum.
#[derive(Clone, Debug, Default)]
struct RuntimeEntityEmission {
    spawn: Option<ae::Vec2>,
    blocks: Vec<ae::Block>,
    zones: Vec<LoadingZone>,
    objects: Vec<ae::RoomObject>,
    ignored: bool,
}

impl RuntimeEntityEmission {
    fn ignored() -> Self {
        Self {
            ignored: true,
            ..Self::default()
        }
    }

    fn spawn(value: ae::Vec2) -> Self {
        Self {
            spawn: Some(value),
            ..Self::default()
        }
    }

    fn zone(zone: LoadingZone) -> Self {
        Self {
            zones: vec![zone],
            ..Self::default()
        }
    }

    fn object(object: ae::RoomObject) -> Self {
        Self {
            objects: vec![object],
            ..Self::default()
        }
    }

    fn from_compiled(compiled: SurfaceCompiled) -> Self {
        Self {
            blocks: compiled.blocks,
            objects: compiled.objects,
            ..Self::default()
        }
    }
}

fn entity_min_size(entity: &LdtkEntityInstance, offset: ae::Vec2) -> (ae::Vec2, ae::Vec2) {
    (
        ae::Vec2::new(entity.px[0] as f32, entity.px[1] as f32) + offset,
        ae::Vec2::new(entity.width as f32, entity.height as f32),
    )
}

fn object_aabb(min: ae::Vec2, size: ae::Vec2) -> ae::Aabb {
    ae::aabb_from_min_size(min, size)
}

fn runtime_room_object(
    entity: &LdtkEntityInstance,
    name: String,
    min: ae::Vec2,
    size: ae::Vec2,
    kind: ae::RoomObjectKind,
) -> ae::RoomObject {
    let aabb = object_aabb(min, size);
    ae::RoomObject::new(entity.iid.clone(), name, aabb, kind)
}

fn entity_to_runtime(
    entity: &LdtkEntityInstance,
    offset: ae::Vec2,
) -> Result<RuntimeEntityEmission, String> {
    let (min, size) = entity_min_size(entity, offset);
    let name = field_string(entity, "name").unwrap_or_else(|| entity.identifier.clone());

    // Surface-shaped identifiers (canonical `Surface` plus legacy aliases) all
    // share a typed parse → compile pipeline. Old JSON paths for `Solid`,
    // `OneWayPlatform`, `BlinkWall`, `HazardBlock`, `PogoOrb`, `ReboundPad`,
    // and `Breakable` are now routed through `LdtkSurfaceSpec` so future
    // collision/contact systems consume one typed runtime IR.
    if is_surface_like_identifier(&entity.identifier) {
        let spec = parse_surface_spec(entity, min, size, name)?;
        let compiled = compile_surface(&spec)?;
        return Ok(RuntimeEntityEmission::from_compiled(compiled));
    }

    match entity.identifier.as_str() {
        "PlayerStart" => Ok(RuntimeEntityEmission::spawn(min + size * 0.5)),
        "LoadingZone" => Ok(RuntimeEntityEmission::zone(LoadingZone {
            id: field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
            name,
            activation: match field_string(entity, "activation")
                .unwrap_or_else(|| "Door".to_string())
                .as_str()
            {
                "EdgeExit" => LoadingZoneActivation::EdgeExit,
                _ => LoadingZoneActivation::Door,
            },
            aabb: object_aabb(min, size),
        })),
        "DamageVolume" => {
            let aabb = object_aabb(min, size);
            let mut volume = ae::DamageVolume::new(
                entity.iid.clone(),
                aabb,
                field_i32(entity, "damage").unwrap_or(1),
            );
            volume.motion = parse_optional_path(entity);
            Ok(RuntimeEntityEmission::object(ae::RoomObject::new(
                entity.iid.clone(),
                name,
                aabb,
                ae::RoomObjectKind::DamageVolume(volume),
            )))
        }
        "KinematicPath" => {
            let points = parse_points(&field_string(entity, "points").unwrap_or_default());
            if points.len() < 2 {
                return Err("KinematicPath requires at least two points".to_string());
            }
            let path = ae::KinematicPath {
                points,
                speed: field_f32(entity, "speed").unwrap_or(100.0),
                mode: parse_path_mode(
                    &field_string(entity, "mode").unwrap_or_else(|| "PingPong".to_string()),
                ),
                start_offset_seconds: 0.0,
            };
            Ok(RuntimeEntityEmission::object(runtime_room_object(
                entity,
                name,
                min,
                size,
                ae::RoomObjectKind::KinematicPath(path),
            )))
        }
        "NpcSpawn" => {
            let interactable = ae::Interactable::new(
                entity.iid.clone(),
                field_string(entity, "prompt").unwrap_or_else(|| "Talk".to_string()),
                object_aabb(min, size),
                ae::InteractionKind::Npc {
                    dialogue_id: field_string(entity, "dialogue_id"),
                },
            );
            Ok(RuntimeEntityEmission::object(runtime_room_object(
                entity,
                name,
                min,
                size,
                ae::RoomObjectKind::Interactable(interactable),
            )))
        }
        "PickupSpawn" => {
            let pickup = ae::Pickup::new(
                entity.iid.clone(),
                parse_pickup_kind(
                    &field_string(entity, "kind").unwrap_or_else(|| "health:1".to_string()),
                ),
            );
            Ok(RuntimeEntityEmission::object(runtime_room_object(
                entity,
                name,
                min,
                size,
                ae::RoomObjectKind::Pickup(pickup),
            )))
        }
        "ChestSpawn" => {
            let chest = ae::Chest::new(
                entity.iid.clone(),
                field_string(entity, "reward").map(|value| parse_pickup_kind(&value)),
            );
            Ok(RuntimeEntityEmission::object(runtime_room_object(
                entity,
                name,
                min,
                size,
                ae::RoomObjectKind::Chest(chest),
            )))
        }
        "EnemySpawn" => Ok(RuntimeEntityEmission::object(runtime_room_object(
            entity,
            name,
            min,
            size,
            ae::RoomObjectKind::EnemySpawn(parse_enemy_brain(
                &field_string(entity, "brain").unwrap_or_else(|| "Passive".to_string()),
            )),
        ))),
        "BossSpawn" => Ok(RuntimeEntityEmission::object(runtime_room_object(
            entity,
            name,
            min,
            size,
            ae::RoomObjectKind::BossSpawn(parse_boss_brain(
                &field_string(entity, "brain").unwrap_or_else(|| "Dormant".to_string()),
            )),
        ))),
        "DebugLabel" => {
            let pos = min + size * 0.5;
            let aabb = ae::Aabb::new(pos, ae::Vec2::splat(1.0));
            let label = ae::DebugLabel::new(
                field_string(entity, "text").unwrap_or_else(|| entity.identifier.clone()),
                pos,
                parse_debug_label_kind(
                    &field_string(entity, "category").unwrap_or_else(|| "Custom".to_string()),
                ),
            );
            Ok(RuntimeEntityEmission::object(ae::RoomObject::new(
                entity.iid.clone(),
                name,
                aabb,
                ae::RoomObjectKind::DebugLabel(label),
            )))
        }
        "CameraZone" | "StitchedBoundary" => Ok(RuntimeEntityEmission::ignored()),
        _ => Err(format!(
            "unsupported entity identifier '{}'",
            entity.identifier
        )),
    }
}

/// Build an `LdtkSurfaceSpec` from a Surface-shaped LDtk entity.
///
/// Identifier-based dispatch:
/// - `Surface`: parse fields directly (the canonical authoring path).
/// - `Solid`/`OneWayPlatform`/`BlinkWall`/`HazardBlock`/`PogoOrb`/`ReboundPad`/`Breakable`:
///   legacy aliases — fields are remapped onto the Surface model so the same
///   compile path produces the same runtime data the old per-identifier
///   branches did.
fn parse_surface_spec(
    entity: &LdtkEntityInstance,
    min: ae::Vec2,
    size: ae::Vec2,
    name: String,
) -> Result<LdtkSurfaceSpec, String> {
    let mut spec = LdtkSurfaceSpec {
        iid: entity.iid.clone(),
        name,
        min,
        size,
        collision: SurfaceCollision::None,
        breakability: SurfaceBreakability::Indestructible,
        contact: SurfaceContact::None,
        respawn: SurfaceRespawn::Never,
        max_hp: 0,
    };

    match entity.identifier.as_str() {
        "Surface" => {
            spec.collision = parse_surface_collision(
                &field_string(entity, "collision").unwrap_or_else(|| "None".to_string()),
            )?;
            spec.breakability = parse_surface_breakability(
                &field_string(entity, "breakability")
                    .unwrap_or_else(|| "Indestructible".to_string()),
            )?;
            spec.contact = parse_surface_contact(entity)?;
            spec.respawn = parse_surface_respawn(entity)?;
            spec.max_hp = field_i32(entity, "hp")
                .or_else(|| field_i32(entity, "max_hp"))
                .unwrap_or(if matches!(spec.breakability, SurfaceBreakability::Indestructible) {
                    0
                } else {
                    3
                });
        }
        "Solid" => {
            spec.collision = SurfaceCollision::Solid;
        }
        "OneWayPlatform" => {
            spec.collision = SurfaceCollision::OneWayUp;
        }
        "BlinkWall" => {
            spec.collision = match field_string(entity, "tier")
                .unwrap_or_else(|| "Soft".to_string())
                .as_str()
            {
                "Soft" => SurfaceCollision::BlinkSoft,
                "Hard" => SurfaceCollision::BlinkHard,
                other => return Err(format!("invalid BlinkWall tier '{other}'")),
            };
        }
        "HazardBlock" => {
            spec.collision = SurfaceCollision::None;
            spec.contact = SurfaceContact::Damage {
                amount: field_i32(entity, "damage").unwrap_or(1),
            };
        }
        "PogoOrb" => {
            spec.collision = SurfaceCollision::None;
            spec.contact = SurfaceContact::PogoRefresh;
        }
        "ReboundPad" => {
            let impulse_x = field_f32(entity, "impulseX")
                .ok_or_else(|| "missing impulseX".to_string())?;
            let impulse_y = field_f32(entity, "impulseY")
                .ok_or_else(|| "missing impulseY".to_string())?;
            spec.collision = SurfaceCollision::None;
            spec.contact = SurfaceContact::Rebound {
                impulse: ae::Vec2::new(impulse_x, impulse_y),
            };
        }
        "Breakable" => {
            spec.collision = if field_bool(entity, "solid").unwrap_or(false) {
                SurfaceCollision::Solid
            } else {
                SurfaceCollision::None
            };
            spec.breakability = match field_string(entity, "trigger")
                .unwrap_or_else(|| "OnHit".to_string())
                .as_str()
            {
                "OnHit" => SurfaceBreakability::BreakOnHit,
                "OnStand" => SurfaceBreakability::BreakOnStand,
                "Either" => SurfaceBreakability::BreakOnHitOrStand,
                other => return Err(format!("invalid Breakable trigger '{other}'")),
            };
            spec.respawn = parse_surface_respawn_legacy(
                field_string(entity, "respawn").as_deref().unwrap_or("Never"),
            )?;
            spec.max_hp = field_i32(entity, "max_hp").unwrap_or(3);
        }
        other => {
            return Err(format!(
                "parse_surface_spec called for non-surface identifier '{other}'"
            ));
        }
    }

    Ok(spec)
}

fn parse_surface_collision(value: &str) -> Result<SurfaceCollision, String> {
    match value.trim() {
        "None" => Ok(SurfaceCollision::None),
        "Solid" => Ok(SurfaceCollision::Solid),
        "OneWayUp" => Ok(SurfaceCollision::OneWayUp),
        "BlinkSoft" => Ok(SurfaceCollision::BlinkSoft),
        "BlinkHard" => Ok(SurfaceCollision::BlinkHard),
        other => Err(format!("invalid Surface collision '{other}'")),
    }
}

fn parse_surface_breakability(value: &str) -> Result<SurfaceBreakability, String> {
    match value.trim() {
        "Indestructible" => Ok(SurfaceBreakability::Indestructible),
        "BreakOnHit" => Ok(SurfaceBreakability::BreakOnHit),
        "BreakOnStand" => Ok(SurfaceBreakability::BreakOnStand),
        "BreakOnHitOrStand" => Ok(SurfaceBreakability::BreakOnHitOrStand),
        other => Err(format!("invalid Surface breakability '{other}'")),
    }
}

fn parse_surface_contact(entity: &LdtkEntityInstance) -> Result<SurfaceContact, String> {
    let raw = field_string(entity, "contact").unwrap_or_else(|| "None".to_string());
    match raw.trim() {
        "None" => Ok(SurfaceContact::None),
        "Damage" => Ok(SurfaceContact::Damage {
            amount: field_i32(entity, "damage").unwrap_or(1),
        }),
        "PogoRefresh" => Ok(SurfaceContact::PogoRefresh),
        "Rebound" => {
            let impulse_x = field_f32(entity, "rebound_x")
                .or_else(|| field_f32(entity, "impulseX"))
                .ok_or_else(|| "Rebound contact requires rebound_x".to_string())?;
            let impulse_y = field_f32(entity, "rebound_y")
                .or_else(|| field_f32(entity, "impulseY"))
                .ok_or_else(|| "Rebound contact requires rebound_y".to_string())?;
            Ok(SurfaceContact::Rebound {
                impulse: ae::Vec2::new(impulse_x, impulse_y),
            })
        }
        other => Err(format!("invalid Surface contact '{other}'")),
    }
}

fn parse_surface_respawn(entity: &LdtkEntityInstance) -> Result<SurfaceRespawn, String> {
    let raw = field_string(entity, "respawn").unwrap_or_else(|| "Never".to_string());
    match raw.trim() {
        "Never" => Ok(SurfaceRespawn::Never),
        "OnRoomReload" => Ok(SurfaceRespawn::OnRoomReload),
        "AfterSeconds" => {
            let seconds = field_f32(entity, "respawn_seconds")
                .ok_or_else(|| "AfterSeconds respawn requires respawn_seconds".to_string())?;
            if !(seconds > 0.0) {
                return Err("respawn_seconds must be > 0".to_string());
            }
            Ok(SurfaceRespawn::AfterSeconds(seconds))
        }
        other => Err(format!("invalid Surface respawn '{other}'")),
    }
}

/// Legacy `Breakable.respawn` field accepted both compact strings (`"Never"`,
/// `"OnRoomReload"`, `"Persistent"`) and a `"AfterSeconds:<n>"` shorthand.
/// Surface only supports the engine-native respawn policies, so `"Persistent"`
/// is mapped to `Never` for now (it lives in the engine but isn't used by the
/// Surface compile path yet).
fn parse_surface_respawn_legacy(value: &str) -> Result<SurfaceRespawn, String> {
    let trimmed = value.trim();
    if let Some(seconds) = trimmed
        .strip_prefix("AfterSeconds:")
        .and_then(|text| text.parse::<f32>().ok())
    {
        if !(seconds > 0.0) {
            return Err(format!("invalid respawn '{trimmed}'"));
        }
        return Ok(SurfaceRespawn::AfterSeconds(seconds));
    }
    match trimmed {
        "Never" | "Persistent" | "" => Ok(SurfaceRespawn::Never),
        "OnRoomReload" => Ok(SurfaceRespawn::OnRoomReload),
        other => Err(format!("invalid respawn '{other}'")),
    }
}

/// Lower a typed `LdtkSurfaceSpec` into engine runtime data.
///
/// Combinations supported today:
///
/// - `Indestructible` + collision (or static contact) → a single `ae::Block`.
/// - Any breakable collision/`None` contact → a `RoomObjectKind::Breakable`,
///   whose engine `BreakableCollision` mirrors the authored `SurfaceCollision`.
///
/// Combinations that are not yet wired (e.g. breakable + damage contact, or
/// breakable + blink wall) return descriptive errors so authors hit a clear
/// validation message rather than silent gameplay drift.
pub fn compile_surface(spec: &LdtkSurfaceSpec) -> Result<SurfaceCompiled, String> {
    if spec.size.x <= 0.0 || spec.size.y <= 0.0 {
        return Err(format!(
            "Surface {} has non-positive size {}x{}",
            spec.iid, spec.size.x, spec.size.y
        ));
    }

    let mut blocks = Vec::new();
    let mut objects = Vec::new();

    match spec.breakability {
        SurfaceBreakability::Indestructible => {
            if let Some(block) = compile_static_surface_block(spec)? {
                blocks.push(block);
            }
        }
        breakable_kind => {
            if !matches!(spec.contact, SurfaceContact::None) {
                return Err(format!(
                    "Surface {} combines breakability with contact; not yet supported",
                    spec.iid
                ));
            }
            let collision = match spec.collision {
                SurfaceCollision::None => ae::BreakableCollision::None,
                SurfaceCollision::Solid => ae::BreakableCollision::Solid,
                SurfaceCollision::OneWayUp => ae::BreakableCollision::OneWayUp,
                SurfaceCollision::BlinkSoft | SurfaceCollision::BlinkHard => {
                    return Err(format!(
                        "Surface {} cannot mix BlinkWall collision with breakability yet",
                        spec.iid
                    ));
                }
            };
            if matches!(breakable_kind, SurfaceBreakability::BreakOnStand)
                && !collision.blocks_movement()
            {
                return Err(format!(
                    "Surface {} BreakOnStand requires non-None collision",
                    spec.iid
                ));
            }
            let max_hp = spec.max_hp.max(1);
            let mut breakable = ae::Breakable::new(spec.iid.clone(), max_hp);
            breakable.collision = collision;
            breakable.trigger = match breakable_kind {
                SurfaceBreakability::BreakOnHit => ae::BreakableTrigger::OnHit,
                SurfaceBreakability::BreakOnStand => ae::BreakableTrigger::OnStand,
                SurfaceBreakability::BreakOnHitOrStand => ae::BreakableTrigger::Either,
                SurfaceBreakability::Indestructible => unreachable!(),
            };
            breakable.respawn = match spec.respawn {
                SurfaceRespawn::Never => ae::RespawnPolicy::Never,
                SurfaceRespawn::OnRoomReload => ae::RespawnPolicy::OnRoomReload,
                SurfaceRespawn::AfterSeconds(seconds) => ae::RespawnPolicy::AfterSeconds(seconds),
            };
            objects.push(ae::RoomObject::new(
                spec.iid.clone(),
                spec.name.clone(),
                ae::aabb_from_min_size(spec.min, spec.size),
                ae::RoomObjectKind::Breakable(breakable),
            ));
        }
    }

    Ok(SurfaceCompiled { blocks, objects })
}

fn compile_static_surface_block(spec: &LdtkSurfaceSpec) -> Result<Option<ae::Block>, String> {
    let name = spec.name.clone();
    let min = spec.min;
    let size = spec.size;
    match (spec.collision, spec.contact) {
        (SurfaceCollision::None, SurfaceContact::None) => Ok(None),
        (SurfaceCollision::Solid, SurfaceContact::None) => {
            Ok(Some(ae::Block::solid(name, min, size)))
        }
        (SurfaceCollision::OneWayUp, SurfaceContact::None) => {
            Ok(Some(ae::Block::one_way(name, min, size)))
        }
        (SurfaceCollision::BlinkSoft, SurfaceContact::None) => Ok(Some(ae::Block::blink_wall(
            name,
            min,
            size,
            ae::BlinkWallTier::Soft,
        ))),
        (SurfaceCollision::BlinkHard, SurfaceContact::None) => Ok(Some(ae::Block::blink_wall(
            name,
            min,
            size,
            ae::BlinkWallTier::Hard,
        ))),
        // Damage contact maps to the legacy hazard reset block; per-amount
        // damage tuning today flows through `RoomObjectKind::DamageVolume`,
        // so for now Surface damage parity stays at the BlockKind::Hazard
        // level. TODO: emit a `DamageVolume` object when amount != 1.
        (SurfaceCollision::None, SurfaceContact::Damage { .. }) => {
            Ok(Some(ae::Block::hazard(name, min, size)))
        }
        (SurfaceCollision::None, SurfaceContact::PogoRefresh) => {
            let radius = size.x.min(size.y) * 0.5;
            Ok(Some(ae::Block::pogo_orb(name, min + size * 0.5, radius)))
        }
        (SurfaceCollision::None, SurfaceContact::Rebound { impulse }) => {
            Ok(Some(ae::Block::rebound(name, min, size, impulse)))
        }
        (collision, contact) => Err(format!(
            "Surface {} has unsupported collision/contact combination ({:?} + {:?})",
            spec.iid, collision, contact
        )),
    }
}

fn known_entity(identifier: &str) -> bool {
    AMBITION_LDTK_ENTITY_IDENTIFIERS.contains(&identifier)
}

fn pivot_is_top_left(entity: &LdtkEntityInstance) -> bool {
    if entity.pivot.len() != 2 {
        return true;
    }
    entity.pivot[0].abs() <= 1.0e-6 && entity.pivot[1].abs() <= 1.0e-6
}

fn entity_rect(entity: &LdtkEntityInstance) -> (i32, i32, i32, i32) {
    (entity.px[0], entity.px[1], entity.width, entity.height)
}

fn rects_strict_intersect(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

fn entity_touches_level_edge(entity: &LdtkEntityInstance, level: &LdtkLevel) -> bool {
    entity.px[0] <= 0
        || entity.px[1] <= 0
        || entity.px[0] + entity.width >= level.px_wid
        || entity.px[1] + entity.height >= level.px_hei
}

fn field_value<'a>(fields: &'a [LdtkFieldInstance], name: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find(|field| field.identifier == name)
        .map(|field| &field.value)
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn field_string(entity: &LdtkEntityInstance, name: &str) -> Option<String> {
    field_value(&entity.field_instances, name).and_then(value_to_string)
}

fn field_f32(entity: &LdtkEntityInstance, name: &str) -> Option<f32> {
    field_value(&entity.field_instances, name).and_then(|value| match value {
        Value::Number(number) => number.as_f64().map(|value| value as f32),
        Value::String(text) => text.parse::<f32>().ok(),
        _ => None,
    })
}

fn field_i32(entity: &LdtkEntityInstance, name: &str) -> Option<i32> {
    field_value(&entity.field_instances, name).and_then(|value| match value {
        Value::Number(number) => number.as_i64().map(|value| value as i32),
        Value::String(text) => text.parse::<i32>().ok(),
        _ => None,
    })
}

fn field_bool(entity: &LdtkEntityInstance, name: &str) -> Option<bool> {
    field_value(&entity.field_instances, name).and_then(|value| match value {
        Value::Bool(value) => Some(*value),
        Value::String(text) => text.parse::<bool>().ok(),
        _ => None,
    })
}

fn parse_points(value: &str) -> Vec<ae::Vec2> {
    value
        .split(';')
        .filter_map(|pair| {
            let mut parts = pair.split(',').map(str::trim);
            let x = parts.next()?.parse::<f32>().ok()?;
            let y = parts.next()?.parse::<f32>().ok()?;
            Some(ae::Vec2::new(x, y))
        })
        .collect()
}

fn parse_path_mode(value: &str) -> ae::KinematicPathMode {
    match value {
        "Once" => ae::KinematicPathMode::Once,
        "Loop" => ae::KinematicPathMode::Loop,
        _ => ae::KinematicPathMode::PingPong,
    }
}

fn parse_optional_path(entity: &LdtkEntityInstance) -> Option<ae::KinematicPath> {
    let points = parse_points(&field_string(entity, "path_points").unwrap_or_default());
    if points.len() < 2 {
        return None;
    }
    Some(ae::KinematicPath {
        points,
        speed: field_f32(entity, "path_speed").unwrap_or(100.0),
        mode: parse_path_mode(
            &field_string(entity, "path_mode").unwrap_or_else(|| "PingPong".to_string()),
        ),
        start_offset_seconds: 0.0,
    })
}

fn parse_pickup_kind(value: &str) -> ae::PickupKind {
    if let Some(amount) = value
        .strip_prefix("health:")
        .and_then(|text| text.parse::<i32>().ok())
    {
        ae::PickupKind::Health { amount }
    } else if let Some(amount) = value
        .strip_prefix("currency:")
        .and_then(|text| text.parse::<i32>().ok())
    {
        ae::PickupKind::Currency { amount }
    } else if let Some(ability_id) = value.strip_prefix("ability:") {
        ae::PickupKind::Ability {
            ability_id: ability_id.to_string(),
        }
    } else if let Some(flag) = value.strip_prefix("flag:") {
        ae::PickupKind::StoryFlag {
            flag: flag.to_string(),
        }
    } else {
        ae::PickupKind::Custom(value.to_string())
    }
}

fn parse_enemy_brain(value: &str) -> ae::EnemyBrain {
    if let Some(path_id) = value.strip_prefix("Patrol:") {
        ae::EnemyBrain::Patrol {
            path_id: Some(path_id.to_string()),
        }
    } else if let Some(radius) = value
        .strip_prefix("Guard:")
        .and_then(|text| text.parse::<f32>().ok())
    {
        ae::EnemyBrain::Guard {
            leash_radius: radius,
        }
    } else {
        match value {
            "Passive" => ae::EnemyBrain::Passive,
            other => ae::EnemyBrain::Custom(other.to_string()),
        }
    }
}

fn parse_boss_brain(value: &str) -> ae::BossBrain {
    if let Some(script_id) = value.strip_prefix("PhaseScript:") {
        ae::BossBrain::PhaseScript {
            script_id: script_id.to_string(),
        }
    } else {
        match value {
            "Dormant" => ae::BossBrain::Dormant,
            other => ae::BossBrain::Custom(other.to_string()),
        }
    }
}

fn parse_debug_label_kind(value: &str) -> ae::DebugLabelKind {
    match value {
        "Room" => ae::DebugLabelKind::Room,
        "LoadingZone" => ae::DebugLabelKind::LoadingZone,
        "Hazard" => ae::DebugLabelKind::Hazard,
        "Enemy" => ae::DebugLabelKind::Enemy,
        "Boss" => ae::DebugLabelKind::Boss,
        "Interactable" => ae::DebugLabelKind::Interactable,
        "Pickup" => ae::DebugLabelKind::Pickup,
        _ => ae::DebugLabelKind::Custom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ae::AabbExt;

    fn make_entity(
        identifier: &str,
        size: [i32; 2],
        fields: &[(&str, Value)],
    ) -> LdtkEntityInstance {
        LdtkEntityInstance {
            iid: format!("{identifier}-test"),
            identifier: identifier.to_string(),
            pivot: vec![0.0, 0.0],
            px: [0, 0],
            width: size[0],
            height: size[1],
            field_instances: fields
                .iter()
                .map(|(name, value)| LdtkFieldInstance {
                    identifier: name.to_string(),
                    value: value.clone(),
                    real_editor_values: vec![Value::Null],
                })
                .collect(),
        }
    }

    fn compile_identifier(
        identifier: &str,
        size: [i32; 2],
        fields: &[(&str, Value)],
    ) -> SurfaceCompiled {
        let entity = make_entity(identifier, size, fields);
        let spec = parse_surface_spec(
            &entity,
            ae::Vec2::ZERO,
            ae::Vec2::new(size[0] as f32, size[1] as f32),
            identifier.to_string(),
        )
        .expect("surface spec parses");
        compile_surface(&spec).expect("surface compiles")
    }

    #[test]
    fn embedded_ldtk_validates() {
        let project = LdtkProject::load_embedded();
        let report = project.validate();
        assert!(report.errors.is_empty(), "{:#?}", report.errors);
    }

    #[test]
    fn embedded_ldtk_composes_central_hub_complex() {
        let project = LdtkProject::load_embedded();
        let room_set = project.to_room_set().expect("embedded LDtk should compose");
        assert!(
            room_set.rooms.len() > 1,
            "old sandbox rooms should be represented as LDtk active areas"
        );
        let room = room_set
            .rooms
            .iter()
            .find(|room| room.id == "central_hub_complex")
            .expect("central hub active area exists");
        assert!(
            room.world.size.y > 1000.0,
            "basement should extend below hub"
        );
        assert!(
            !room
                .world
                .objects
                .iter()
                .any(|object| matches!(&object.kind, ae::RoomObjectKind::BossSpawn(_))),
            "boss belongs in the boss lab, not the stitched hub basement"
        );
        let boss_room = room_set
            .rooms
            .iter()
            .find(|room| room.id == "basement_boss")
            .expect("boss lab room exists");
        assert!(boss_room.world.objects.iter().any(|object| matches!(
            &object.kind,
            ae::RoomObjectKind::BossSpawn(_)
        ) && object
            .name
            .contains("clockwork warden")));
    }

    #[test]
    fn solid_is_a_promoted_runtime_role() {
        let role = LdtkRuntimeRole::from_identifier("Solid");
        assert_eq!(role, LdtkRuntimeRole::Solid);
        assert!(role.promoted(), "Solid is a Step 1 promoted runtime role");
        let summary = LdtkRuntimeSpineIndex::default().promoted_summary();
        assert!(
            summary.contains("solids"),
            "promoted summary surfaces solid count: {summary}"
        );
    }

    #[test]
    fn solid_index_replaces_only_when_changed() {
        let mut index = LdtkRuntimeSolidIndex::default();
        let solid_a = LdtkRuntimeSolid {
            iid: "solid-a".to_string(),
            min: ae::Vec2::ZERO,
            size: ae::Vec2::new(64.0, 16.0),
        };
        let solid_b = LdtkRuntimeSolid {
            iid: "solid-b".to_string(),
            min: ae::Vec2::new(64.0, 0.0),
            size: ae::Vec2::new(64.0, 16.0),
        };
        index.replace_if_changed(LdtkRuntimeSolidIndex {
            active_area: "central_hub_complex".to_string(),
            solids: vec![solid_b.clone(), solid_a.clone()],
            revision: 0,
        });
        assert_eq!(index.count(), 2);
        assert_eq!(
            index.solids[0].iid, "solid-a",
            "solids are sorted by iid for stable diffs"
        );
        assert_eq!(index.revision, 1);

        let before = index.revision;
        index.replace_if_changed(LdtkRuntimeSolidIndex {
            active_area: "central_hub_complex".to_string(),
            solids: vec![solid_a, solid_b],
            revision: index.revision,
        });
        assert_eq!(
            index.revision, before,
            "no-op replace must not bump revision"
        );
    }

    #[test]
    fn surface_solid_compiles_to_solid_block() {
        let compiled = compile_identifier(
            "Surface",
            [128, 32],
            &[("collision", Value::String("Solid".into()))],
        );
        assert_eq!(compiled.objects.len(), 0);
        assert_eq!(compiled.blocks.len(), 1);
        assert!(matches!(
            compiled.blocks[0].kind,
            ae::BlockKind::Solid
        ));
    }

    #[test]
    fn surface_one_way_compiles_to_one_way_block() {
        let compiled = compile_identifier(
            "Surface",
            [96, 16],
            &[("collision", Value::String("OneWayUp".into()))],
        );
        assert_eq!(compiled.blocks.len(), 1);
        assert!(matches!(
            compiled.blocks[0].kind,
            ae::BlockKind::OneWay
        ));
    }

    #[test]
    fn surface_breakable_solid_wall_emits_breakable_with_solid_collision() {
        let compiled = compile_identifier(
            "Surface",
            [48, 48],
            &[
                ("collision", Value::String("Solid".into())),
                ("breakability", Value::String("BreakOnHit".into())),
                ("hp", Value::Number(serde_json::Number::from(2))),
            ],
        );
        assert!(compiled.blocks.is_empty());
        assert_eq!(compiled.objects.len(), 1);
        match &compiled.objects[0].kind {
            ae::RoomObjectKind::Breakable(breakable) => {
                assert_eq!(breakable.collision, ae::BreakableCollision::Solid);
                assert_eq!(breakable.trigger, ae::BreakableTrigger::OnHit);
                assert_eq!(breakable.health.max, 2);
            }
            other => panic!("expected Breakable, got {other:?}"),
        }
    }

    #[test]
    fn surface_breakable_one_way_emits_breakable_with_one_way_collision() {
        let compiled = compile_identifier(
            "Surface",
            [80, 16],
            &[
                ("collision", Value::String("OneWayUp".into())),
                ("breakability", Value::String("BreakOnStand".into())),
            ],
        );
        assert_eq!(compiled.objects.len(), 1);
        match &compiled.objects[0].kind {
            ae::RoomObjectKind::Breakable(breakable) => {
                assert_eq!(breakable.collision, ae::BreakableCollision::OneWayUp);
                assert_eq!(breakable.trigger, ae::BreakableTrigger::OnStand);
            }
            other => panic!("expected Breakable, got {other:?}"),
        }
    }

    #[test]
    fn surface_damage_contact_compiles_to_hazard_block() {
        let compiled = compile_identifier(
            "Surface",
            [64, 16],
            &[
                ("collision", Value::String("None".into())),
                ("contact", Value::String("Damage".into())),
            ],
        );
        assert_eq!(compiled.blocks.len(), 1);
        assert!(matches!(
            compiled.blocks[0].kind,
            ae::BlockKind::Hazard
        ));
    }

    #[test]
    fn surface_break_on_stand_with_no_collision_is_rejected() {
        let entity = make_entity(
            "Surface",
            [32, 32],
            &[
                ("collision", Value::String("None".into())),
                ("breakability", Value::String("BreakOnStand".into())),
            ],
        );
        let spec = parse_surface_spec(
            &entity,
            ae::Vec2::ZERO,
            ae::Vec2::new(32.0, 32.0),
            "test".into(),
        )
        .expect("parses");
        let err = compile_surface(&spec).expect_err("BreakOnStand requires collision");
        assert!(
            err.contains("BreakOnStand requires non-None collision"),
            "{err}"
        );
    }

    #[test]
    fn legacy_solid_alias_matches_native_surface_solid() {
        let legacy = compile_identifier("Solid", [64, 16], &[]);
        let native = compile_identifier(
            "Surface",
            [64, 16],
            &[("collision", Value::String("Solid".into()))],
        );
        assert_eq!(legacy.blocks.len(), 1);
        assert_eq!(native.blocks.len(), 1);
        // Both should be solid blocks with the same kind/aabb shape.
        assert!(matches!(legacy.blocks[0].kind, ae::BlockKind::Solid));
        assert!(matches!(native.blocks[0].kind, ae::BlockKind::Solid));
        assert_eq!(legacy.blocks[0].aabb.center(), native.blocks[0].aabb.center());
        assert_eq!(
            legacy.blocks[0].aabb.half_size(),
            native.blocks[0].aabb.half_size()
        );
    }

    #[test]
    fn legacy_breakable_alias_maps_solid_field_to_collision() {
        let compiled = compile_identifier(
            "Breakable",
            [32, 32],
            &[
                ("solid", Value::Bool(true)),
                ("max_hp", Value::Number(serde_json::Number::from(4))),
                ("trigger", Value::String("OnHit".into())),
            ],
        );
        assert_eq!(compiled.objects.len(), 1);
        match &compiled.objects[0].kind {
            ae::RoomObjectKind::Breakable(breakable) => {
                assert_eq!(breakable.collision, ae::BreakableCollision::Solid);
                assert_eq!(breakable.health.max, 4);
            }
            other => panic!("expected Breakable, got {other:?}"),
        }
    }

    #[test]
    fn legacy_blink_wall_alias_uses_tier_field() {
        let soft = compile_identifier(
            "BlinkWall",
            [32, 32],
            &[("tier", Value::String("Soft".into()))],
        );
        let hard = compile_identifier(
            "BlinkWall",
            [32, 32],
            &[("tier", Value::String("Hard".into()))],
        );
        assert!(matches!(
            soft.blocks[0].kind,
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Soft
            }
        ));
        assert!(matches!(
            hard.blocks[0].kind,
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard
            }
        ));
    }

    #[test]
    fn legacy_rebound_pad_requires_impulse_fields() {
        let entity = make_entity("ReboundPad", [16, 16], &[]);
        let err = parse_surface_spec(
            &entity,
            ae::Vec2::ZERO,
            ae::Vec2::new(16.0, 16.0),
            "rp".into(),
        )
        .expect_err("missing impulses");
        assert!(err.contains("missing impulseX"), "{err}");
    }

    #[test]
    fn surface_is_a_known_entity_identifier() {
        assert!(known_entity("Surface"));
        assert!(is_surface_like_identifier("Surface"));
        // legacy aliases still route through the surface pipeline
        for id in [
            "Solid",
            "OneWayPlatform",
            "BlinkWall",
            "HazardBlock",
            "PogoOrb",
            "ReboundPad",
            "Breakable",
        ] {
            assert!(is_surface_like_identifier(id), "{id}");
        }
        // non-surface entities are not surface-like
        for id in ["PlayerStart", "LoadingZone", "DebugLabel", "NpcSpawn"] {
            assert!(!is_surface_like_identifier(id), "{id}");
        }
    }
}
