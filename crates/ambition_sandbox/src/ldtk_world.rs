//! LDtk world-composition adapter and validator for the sandbox.
//!
//! Ambition keeps its gameplay model typed in Rust. LDtk is an authoring
//! frontend: this module validates the subset of LDtk entities Ambition
//! currently understands and materializes Ambition runtime rooms directly
//! from LDtk-authored data.
//!
//! Step C of `docs/path_forward.md` calls for splitting this file. The
//! bevy_ecs_ldtk plugin glue + runtime-spine indexing live in the
//! [`bevy_runtime`] submodule; this top-level file owns the JSON parser,
//! validator, surface compiler, hot reload state, and tests. Re-exports
//! below keep the historical `crate::ldtk_world::Foo` import paths
//! working across the rest of the crate.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use serde::Deserialize;
use serde_json::Value;

use ambition_engine as ae;

use crate::rooms::{LoadingZone, LoadingZoneActivation, RoomLink, RoomSet, RoomSpec};

pub mod bevy_runtime;
mod fields;
mod hot_reload;
mod intgrid;
mod surfaces;
#[cfg(test)]
mod tests;

pub use bevy_runtime::*;
pub use hot_reload::{
    configured_ldtk_path, default_sandbox_ldtk_path, poll_ldtk_file_changes,
    sandbox_ldtk_asset_path, sandbox_ldtk_modified_time, sandbox_ldtk_path, LdtkHotReloadState,
    AMBITION_LDTK_ENV, SANDBOX_LDTK_ASSET,
};
pub use surfaces::{
    compile_surface, LdtkSurfaceSpec, SurfaceBreakability, SurfaceCollision, SurfaceCompiled,
    SurfaceContact, SurfaceRespawn,
};

use fields::{
    entity_rect, entity_touches_level_edge, field_bool, field_i32, field_value, known_entity,
    parse_boss_brain, parse_debug_label_kind, parse_enemy_brain, parse_optional_path,
    parse_path_mode, parse_pickup_kind, parse_points, pivot_is_top_left, rects_strict_intersect,
    value_to_string,
};
pub(crate) use fields::{field_f32, field_string};
use intgrid::*;
use surfaces::{is_surface_like_identifier, parse_surface_spec};

#[derive(Clone, Debug, Deserialize)]
pub struct LdtkProject {
    #[serde(rename = "jsonVersion")]
    pub json_version: String,
    #[serde(default)]
    pub levels: Vec<LdtkLevel>,
}

/// Bevy resource wrapper so other systems (encounter loader) can read
/// the parsed LDtk project without re-parsing the file. Inserted in
/// `init_sandbox_resources`; refreshed by hot reload.
#[derive(bevy::prelude::Resource, Clone, Debug)]
pub struct SandboxLdtkProject(pub LdtkProject);

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
    #[serde(rename = "__type", default)]
    pub layer_type: String,
    #[serde(rename = "__cWid", default)]
    pub c_wid: i32,
    #[serde(rename = "__cHei", default)]
    pub c_hei: i32,
    #[serde(rename = "__gridSize", default = "default_grid_size")]
    pub grid_size: i32,
    #[serde(default, rename = "entityInstances")]
    pub entity_instances: Vec<LdtkEntityInstance>,
    /// IntGrid cell values, row-major (`y * c_wid + x`), `0` = empty.
    /// Only populated for layers whose `__type == "IntGrid"`.
    #[serde(default, rename = "intGridCsv")]
    pub int_grid_csv: Vec<i32>,
}

fn default_grid_size() -> i32 {
    16
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
    /// Load the sandbox map using the normal runtime policy.
    ///
    /// Desktop builds default to the external checked-in asset path so LDtk edits
    /// and modded maps do not require recompiling Rust. Android `static_map`
    /// builds default to the embedded map unless a user explicitly passes
    /// `--ldtk`, `--map`, or `AMBITION_LDTK`; the source-tree path is not a
    /// meaningful filesystem location inside the APK.
    pub fn load_default() -> Result<Self, String> {
        #[cfg(all(target_os = "android", feature = "static_map"))]
        if configured_ldtk_path().is_none() {
            return Self::load_static_map();
        }

        let path = sandbox_ldtk_path();
        match Self::load_from_path(&path) {
            Ok(project) => Ok(project),
            Err(error) => {
                #[cfg(feature = "static_map")]
                {
                    eprintln!(
                        "LDtk warning: {error}; falling back to statically packed sandbox.ldtk"
                    );
                    Self::load_static_map().map_err(|fallback_error| {
                        format!(
                            "{error}; statically packed sandbox.ldtk also failed: {fallback_error}"
                        )
                    })
                }
                #[cfg(not(feature = "static_map"))]
                {
                    Err(format!(
                        "{error}. No statically packed fallback is available in this build; \
                         restore the LDtk asset or rebuild with `--features static_map`."
                    ))
                }
            }
        }
    }

    #[cfg(feature = "static_map")]
    pub fn load_static_map() -> Result<Self, String> {
        serde_json::from_str(include_str!("../assets/ambition/worlds/sandbox.ldtk"))
            .map_err(|error| format!("could not parse statically packed sandbox.ldtk: {error}"))
    }

    pub fn load_from_disk() -> Result<Self, String> {
        Self::load_from_path(sandbox_ldtk_path())
    }

    pub fn load_from_path(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
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
                    "DebugLabel" if field_string(entity, "text").is_none() => {
                        report
                            .errors
                            .push(format!("DebugLabel {} requires text field", entity.iid));
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
                    let placeholder_size = ae::Vec2::new(entity.width as f32, entity.height as f32);
                    let name =
                        field_string(entity, "name").unwrap_or_else(|| entity.identifier.clone());
                    match parse_surface_spec(entity, placeholder_min, placeholder_size, name)
                        .and_then(|spec| compile_surface(&spec))
                    {
                        Ok(_) => {}
                        Err(error) => report
                            .errors
                            .push(format!("{} {}: {error}", entity.identifier, entity.iid)),
                    }
                }
                // Note: we deliberately do NOT warn on empty `realEditorValues`
                // here. LDtk 1.5.3 emits that shape natively for fields that
                // inherit their value from the entity-def `defaultOverride`,
                // so flagging it would treat the editor's own output as a
                // problem and break the contract that a file the LDtk editor
                // writes must run unchanged. The historical
                // `tools/repair_ambition_ldtk.py` script remains available for
                // anyone who wants to canonicalize the JSON for diffs, but
                // it is not required for runtime correctness.
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

    /// Cross-validate level `music_track` fields against the catalog of
    /// audio-side track ids loaded from `SandboxDataSpec`. Returns one
    /// warning per (level, unknown_id) pair so the user can see all
    /// typos in a single startup pass instead of debugging room-by-room.
    ///
    /// Lives here (not on `validate()`) because the LDtk validator must
    /// stay self-contained — the audio catalog is only known once
    /// `SandboxDataSpec` is loaded. Callers (visible binary's
    /// `init_sandbox_resources`, headless tests) wire both halves.
    pub fn music_track_warnings<'a, I>(&self, valid_track_ids: I) -> Vec<String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let valid: BTreeSet<&str> = valid_track_ids.into_iter().collect();
        let mut warnings = Vec::new();
        for level in &self.levels {
            let Some(track) = level.field_string("music_track") else {
                continue;
            };
            let trimmed = track.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !valid.contains(trimmed) {
                warnings.push(format!(
                    "level '{}' references unknown music_track '{}' — add it to the audio music_tracks catalog or fix the typo",
                    level.identifier, trimmed
                ));
            }
        }
        warnings
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
        let mut water_regions = Vec::new();
        let mut climbable_regions = Vec::new();
        let mut moving_platform: Option<crate::platforms::MovingPlatformState> = None;
        let mut metadata = crate::rooms::RoomMetadata::default();
        for level in levels {
            // First-non-empty wins so author intent is predictable when
            // an active area spans multiple levels (e.g. central hub +
            // basement). The level order here is the LDtk-file order.
            metadata.merge(level.level_metadata());
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
                        water_regions.extend(emission.water_regions);
                        if moving_platform.is_none() {
                            moving_platform = emission.moving_platform;
                        }
                    }
                    Err(error) => {
                        errors.push(format!("{} {}: {error}", entity.identifier, entity.iid))
                    }
                }
            }

            // IntGrid `Collision` layer: greedy-merge runs of same-value
            // cells into rectangles before emitting engine blocks. Per-cell
            // blocks introduced perceptible friction during ground-walk
            // because every 16px boundary became a potential snag against
            // the bespoke sweep logic (path_forward step D); merging
            // collapses a typical floor of N cells into one block while
            // keeping the IntGrid as the authoring representation. See
            // `tools/ldtk_intgrid_migration.py` for the value mapping.
            if let Some(layer) = level.collision_layer() {
                match emit_collision_blocks_from_intgrid(layer, offset) {
                    Ok(layer_blocks) => blocks.extend(layer_blocks),
                    Err(message) => {
                        errors.push(format!("level '{}' Collision: {message}", level.identifier))
                    }
                }
            }

            // IntGrid `Water` layer: each cell becomes a swimmable
            // region. Source-agnostic with entity `WaterVolume`; both
            // populate `World::water_regions`.
            if let Some(layer) = level.water_layer() {
                match emit_water_regions_from_intgrid(layer, offset) {
                    Ok(layer_regions) => water_regions.extend(layer_regions),
                    Err(message) => {
                        errors.push(format!("level '{}' Water: {message}", level.identifier))
                    }
                }
            }

            // IntGrid `Climbable` layer: each cell becomes a ladder /
            // vine / climbable wall region. Same source-agnostic
            // contract as Water — engine queries via
            // `World::climbable_at` regardless of authoring source.
            if let Some(layer) = level.climbable_layer() {
                match emit_climbable_regions_from_intgrid(layer, offset) {
                    Ok(layer_regions) => climbable_regions.extend(layer_regions),
                    Err(message) => {
                        errors.push(format!("level '{}' Climbable: {message}", level.identifier))
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
                water_regions,
                climbable_regions,
            },
            loading_zones,
            metadata,
            moving_platform,
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

    pub fn active_area(&self) -> String {
        self.raw_active_area()
            .map(|area| area.trim().to_string())
            .filter(|area| !area.is_empty())
            .unwrap_or_else(|| self.identifier.clone())
    }

    /// Read the optional biome metadata level fields. Empty/None values
    /// stay None so the active-area-merge in `compose_runtime_area`
    /// only takes the first non-empty value per active area.
    pub fn level_metadata(&self) -> crate::rooms::RoomMetadata {
        let take = |name: &str| {
            self.field_string(name)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        crate::rooms::RoomMetadata {
            biome: take("biome"),
            music_track: take("music_track"),
            ambient_profile: take("ambient_profile"),
            visual_theme: take("visual_theme"),
        }
    }

    pub fn ambition_layer(&self) -> Option<&LdtkLayerInstance> {
        self.layer_instances
            .iter()
            .find(|layer| layer.identifier == AMBITION_LAYER)
    }

    fn collision_layer(&self) -> Option<&LdtkLayerInstance> {
        self.layer_instances
            .iter()
            .find(|layer| layer.identifier == COLLISION_LAYER)
    }

    fn water_layer(&self) -> Option<&LdtkLayerInstance> {
        self.layer_instances
            .iter()
            .find(|layer| layer.identifier == WATER_LAYER)
    }

    fn climbable_layer(&self) -> Option<&LdtkLayerInstance> {
        self.layer_instances
            .iter()
            .find(|layer| layer.identifier == CLIMBABLE_LAYER)
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
    water_regions: Vec<ae::WaterRegion>,
    /// LDtk-authored moving platform. Today the sandbox runtime stores
    /// a single `MovingPlatformState`; if multiple `MovingPlatform`
    /// entities are placed in the same area, only the first is used.
    moving_platform: Option<crate::platforms::MovingPlatformState>,
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

    fn water_region(region: ae::WaterRegion) -> Self {
        Self {
            water_regions: vec![region],
            ..Self::default()
        }
    }

    fn moving_platform(state: crate::platforms::MovingPlatformState) -> Self {
        Self {
            moving_platform: Some(state),
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
                    // Optional `patrol_radius` field on NpcSpawn. 0
                    // (or unset) → static NPC; >0 → paces around
                    // spawn within that half-range.
                    patrol_radius: field_f32(entity, "patrol_radius").unwrap_or(0.0),
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
        "WaterVolume" => {
            // Entity-authored water: source-agnostic, lands in the
            // same `World::water_regions` list IntGrid Water cells
            // populate. Reserved for irregular pools the per-cell
            // IntGrid layer can't shape.
            let mut spec = ae::WaterVolumeSpec::default();
            if let Some(value) = field_f32(entity, "gravity_scale") {
                spec.gravity_scale = value;
            }
            if let Some(value) = field_f32(entity, "drag") {
                spec.drag = value;
            }
            if let Some(value) = field_f32(entity, "max_fall_speed") {
                spec.max_fall_speed = value;
            }
            if let Some(value) = field_f32(entity, "swim_up_impulse") {
                spec.swim_up_impulse = value;
            }
            // Entity water defaults to Clear. The IntGrid Water
            // layer is the canonical authoring path for distinct
            // kinds; if a future entity field needs Murky, add a
            // `kind` field via `register_ldtk_entity_def.py` and
            // route it here.
            Ok(RuntimeEntityEmission::water_region(ae::WaterRegion::new(
                object_aabb(min, size),
                ae::WaterKind::Clear,
                spec,
            )))
        }
        "MovingPlatform" => {
            // LDtk entity bounds → starting AABB. The platform sweeps
            // horizontally by `sweep_dx` from the start position, at
            // `speed` px/s, ping-ponging at the bounds. Defaults match
            // the legacy `time_reference` platform so an authored
            // entity with no overrides reproduces the previous feel.
            let start_pos = min + size * 0.5;
            let sweep_dx = field_f32(entity, "sweep_dx").unwrap_or(240.0);
            let speed = field_f32(entity, "speed").unwrap_or(130.0);
            Ok(RuntimeEntityEmission::moving_platform(
                crate::platforms::MovingPlatformState::from_authored(
                    start_pos, size, sweep_dx, speed,
                ),
            ))
        }
        "CameraZone" | "StitchedBoundary" => Ok(RuntimeEntityEmission::ignored()),
        // EncounterTrigger entities are read by `crate::encounter::load_encounter_specs_from_ldtk`
        // directly off the `LdtkProject` because the encounter spec
        // wants level-relative coordinates and field combinations
        // (camera_zoom, target_encounter id) that don't fit the
        // generic `RoomObject` shape. Skipping here keeps composition
        // free of encounter-specific routing.
        "EncounterTrigger" => Ok(RuntimeEntityEmission::ignored()),
        // LockWall is a marker for an encounter-spawned Solid; the
        // encounter system reads it off the project directly.
        "LockWall" => Ok(RuntimeEntityEmission::ignored()),
        // Switches are interactables routed through `FeatureRuntime`.
        // The id / target_encounter / action fields are encoded into
        // an `InteractionKind::Custom` payload so the switch handler
        // can decide what to do without growing a new `RoomObjectKind`
        // variant in the engine for every action type.
        "Switch" => {
            let id = field_string(entity, "id").unwrap_or_else(|| entity.iid.clone());
            let action = field_string(entity, "action").unwrap_or_else(|| "ResetEncounter".into());
            let target = field_string(entity, "target_encounter").unwrap_or_default();
            // Custom payload format: "switch:<id>:<action>:<target>"
            // FeatureRuntime parses it back into typed fields.
            let custom = format!("switch:{id}:{action}:{target}");
            let interactable = ae::Interactable::new(
                id.clone(),
                field_string(entity, "prompt").unwrap_or_else(|| "Activate".into()),
                object_aabb(min, size),
                ae::InteractionKind::Custom(custom),
            );
            // Use the LDtk field `id` for the RoomObject id so the
            // SwitchRuntime's id matches the SwitchActivation payload's
            // `id`. (`runtime_room_object` defaults to entity.iid like
            // "Switch-4072"; that would mismatch and
            // `FeatureRuntime::set_switch_on` would silently no-op,
            // which is the bug that left the switch stuck red.)
            let aabb = object_aabb(min, size);
            Ok(RuntimeEntityEmission::object(ae::RoomObject::new(
                id,
                name,
                aabb,
                ae::RoomObjectKind::Interactable(interactable),
            )))
        }
        _ => Err(format!(
            "unsupported entity identifier '{}'",
            entity.identifier
        )),
    }
}
