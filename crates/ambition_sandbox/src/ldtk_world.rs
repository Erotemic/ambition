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
use std::path::PathBuf;
use std::time::SystemTime;

use bevy::prelude::{Res, ResMut, Resource, Time};
use serde::Deserialize;
use serde_json::Value;

use ambition_engine as ae;

use crate::rooms::{LoadingZone, LoadingZoneActivation, RoomLink, RoomSet, RoomSpec};

pub mod bevy_runtime;
pub use bevy_runtime::*;

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

/// LDtk identifiers that lower into the typed runtime "surface" conversion
/// pipeline.
///
/// The LDtk editor keeps these visually/semantically distinct so designers
/// pick the right primitive (Solid, OneWayPlatform, BlinkWall, HazardBlock,
/// PogoOrb, ReboundPad, Breakable). Internally the parser collapses them to
/// the same typed `LdtkSurfaceSpec` so collision/contact/breakability code
/// has a single conversion path. There is intentionally no canonical
/// generic `Surface` authoring entity; the editor stays differentiated.
const SURFACE_LIKE_IDENTIFIERS: &[&str] = &[
    "Solid",
    "OneWayPlatform",
    "BlinkWall",
    "HazardBlock",
    "PogoOrb",
    "ReboundPad",
    "BreakablePlatform",
    "BreakablePogoOrb",
];

/// True if `identifier` lowers into `LdtkSurfaceSpec` via `parse_surface_spec`.
fn is_surface_like_identifier(identifier: &str) -> bool {
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
const COLLISION_LAYER: &str = "Collision";
const WATER_LAYER: &str = "Water";
const GRID: i32 = 16;

/// IntGrid Water layer values. Distinct from Collision values because
/// they live on a separate layer (see `WATER_LAYER`).
const WATER_INT_GRID_CLEAR: i32 = 1;
const WATER_INT_GRID_MURKY: i32 = 2;

// IntGrid value → engine block kind. Mirrors `tools/ldtk_intgrid_migration.py`;
// the migration script is the source of truth for which value means what, but
// any new value here that isn't covered there will fail validation at compose
// time so authors can't silently introduce mismatched mappings.
const INT_GRID_SOLID: i32 = 1;
const INT_GRID_ONE_WAY: i32 = 2;
const INT_GRID_BLINK_SOFT: i32 = 3;
const INT_GRID_BLINK_HARD: i32 = 4;
const INT_GRID_HAZARD: i32 = 5;

fn int_grid_value_to_block(value: i32, min: ae::Vec2, size: ae::Vec2) -> Result<ae::Block, String> {
    match value {
        INT_GRID_SOLID => Ok(ae::Block::solid("ldtk solid", min, size)),
        INT_GRID_ONE_WAY => Ok(ae::Block::one_way("ldtk one-way", min, size)),
        INT_GRID_BLINK_SOFT => Ok(ae::Block::blink_wall(
            "ldtk blink-soft",
            min,
            size,
            ae::BlinkWallTier::Soft,
        )),
        INT_GRID_BLINK_HARD => Ok(ae::Block::blink_wall(
            "ldtk blink-hard",
            min,
            size,
            ae::BlinkWallTier::Hard,
        )),
        // Hazard tile: damages the player on contact. Static-only —
        // moving / per-volume-tuned hazards stay on the
        // `RoomObjectKind::DamageVolume` entity path because IntGrid
        // can't carry per-cell motion paths or damage amounts.
        INT_GRID_HAZARD => Ok(ae::Block::hazard("ldtk hazard", min, size)),
        other => Err(format!("unknown IntGrid value {other}")),
    }
}

/// Two-pass rectangle merge over the IntGrid:
///   1. Per-row horizontal coalesce: each row collapses adjacent
///      same-value cells into a single run.
///   2. Per-column vertical merge: adjacent rows that produced the
///      *exact same span* (same x extent, same value) are stacked into
///      one taller block.
///
/// This correctly handles:
///   - Long horizontal floors (pass 1 merges them; pass 2 finds nothing
///     more to do) → one block. Floor-walk friction fix preserved.
///   - Vertical walls of N-cell-wide cells stacked vertically (pass 1
///     produces N identical 1-tall blocks; pass 2 stacks them into one
///     N×H block) → one block. Wall-slide grinding fix.
///   - Staircase / diagonal patterns: pass 1 produces blocks of varying
///     widths per row (1, 2, 3, …); pass 2 finds no two adjacent rows
///     with the same span so nothing merges. Staircases stay per-row
///     visually (matches the editor's rendering). Regression fix from
///     the earlier greedy-row-major bug.
///
/// Invariant: every cell ends up covered by exactly one rectangle.
fn merge_intgrid_rects(
    layer: &LdtkLayerInstance,
    offset: ae::Vec2,
) -> Result<Vec<(i32, ae::Vec2, ae::Vec2)>, String> {
    let cw = layer.c_wid;
    let ch = layer.c_hei;
    let grid = layer.grid_size as f32;
    if cw <= 0 || ch <= 0 || layer.int_grid_csv.is_empty() {
        return Ok(Vec::new());
    }
    let expected = (cw as usize) * (ch as usize);
    if layer.int_grid_csv.len() != expected {
        return Err(format!(
            "intGridCsv length {} does not match cWid*cHei = {}*{} = {expected}",
            layer.int_grid_csv.len(),
            cw,
            ch
        ));
    }
    let cells = &layer.int_grid_csv;
    let cw_usize = cw as usize;
    let ch_usize = ch as usize;

    // Pass 1: produce per-row runs as (cx, x_end, cy, value).
    let mut runs: Vec<(usize, usize, usize, i32)> = Vec::new();
    for cy in 0..ch_usize {
        let mut cx = 0;
        while cx < cw_usize {
            let value = cells[cy * cw_usize + cx];
            if value == 0 {
                cx += 1;
                continue;
            }
            let mut x_end = cx + 1;
            while x_end < cw_usize && cells[cy * cw_usize + x_end] == value {
                x_end += 1;
            }
            runs.push((cx, x_end, cy, value));
            cx = x_end;
        }
    }

    // Pass 2: stack runs vertically when the next-row run has the same
    // [cx, x_end) span and value.
    let mut consumed = vec![false; runs.len()];
    let mut by_row_cx: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::with_capacity(runs.len());
    for (i, &(cx, _, cy, _)) in runs.iter().enumerate() {
        by_row_cx.insert((cy, cx), i);
    }

    let mut rects = Vec::new();
    for i in 0..runs.len() {
        if consumed[i] {
            continue;
        }
        let (cx, x_end, cy, value) = runs[i];
        let mut y_end = cy + 1;
        while y_end < ch_usize {
            let Some(&next_idx) = by_row_cx.get(&(y_end, cx)) else {
                break;
            };
            let (n_cx, n_x_end, _, n_value) = runs[next_idx];
            if consumed[next_idx] || n_cx != cx || n_x_end != x_end || n_value != value {
                break;
            }
            consumed[next_idx] = true;
            y_end += 1;
        }
        let min = ae::Vec2::new(cx as f32 * grid, cy as f32 * grid) + offset;
        let size = ae::Vec2::new((x_end - cx) as f32 * grid, (y_end - cy) as f32 * grid);
        rects.push((value, min, size));
    }
    Ok(rects)
}

fn emit_collision_blocks_from_intgrid(
    layer: &LdtkLayerInstance,
    offset: ae::Vec2,
) -> Result<Vec<ae::Block>, String> {
    let rects = merge_intgrid_rects(layer, offset)?;
    let mut blocks = Vec::with_capacity(rects.len());
    for (value, min, size) in rects {
        let block = int_grid_value_to_block(value, min, size)
            .map_err(|message| format!("rect value={value} {size:?}: {message}"))?;
        blocks.push(block);
    }
    Ok(blocks)
}

/// Lower a Water IntGrid layer to source-agnostic `WaterRegion`
/// rectangles. Cells with value 1 emit `WaterKind::Clear`; value 2
/// emits `WaterKind::Murky`. Per-region tuning falls back to
/// `WaterVolumeSpec::default()`; per-volume tuning is the entity
/// path's job (rare, irregular pools).
fn emit_water_regions_from_intgrid(
    layer: &LdtkLayerInstance,
    offset: ae::Vec2,
) -> Result<Vec<ae::WaterRegion>, String> {
    let rects = merge_intgrid_rects(layer, offset)?;
    let mut regions = Vec::with_capacity(rects.len());
    for (value, min, size) in rects {
        let kind = match value {
            WATER_INT_GRID_CLEAR => ae::WaterKind::Clear,
            WATER_INT_GRID_MURKY => ae::WaterKind::Murky,
            other => return Err(format!("unknown Water IntGrid value {other}")),
        };
        regions.push(ae::WaterRegion::new(
            ae::aabb_from_min_size(min, size),
            kind,
            ae::WaterVolumeSpec::default(),
        ));
    }
    Ok(regions)
}

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
            },
            loading_zones,
            metadata,
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
            let impulse_x =
                field_f32(entity, "impulseX").ok_or_else(|| "missing impulseX".to_string())?;
            let impulse_y =
                field_f32(entity, "impulseY").ok_or_else(|| "missing impulseY".to_string())?;
            spec.collision = SurfaceCollision::None;
            spec.contact = SurfaceContact::Rebound {
                impulse: ae::Vec2::new(impulse_x, impulse_y),
            };
        }
        "BreakablePlatform" => {
            // Constrained breakable: `collision` must be Solid or OneWayUp
            // (the LDtk enum has no None option), so the historically
            // incoherent OnStand+None combo is unrepresentable in the
            // editor — no degrade path needed.
            spec.collision = match field_string(entity, "collision").as_deref() {
                Some("Solid") | None => SurfaceCollision::Solid,
                Some("OneWayUp") => SurfaceCollision::OneWayUp,
                Some(other) => {
                    return Err(format!("invalid BreakablePlatform collision '{other}'"));
                }
            };
            spec.breakability = match field_string(entity, "trigger")
                .as_deref()
                .unwrap_or("OnHit")
            {
                "OnHit" => SurfaceBreakability::BreakOnHit,
                "OnStand" => SurfaceBreakability::BreakOnStand,
                "Either" => SurfaceBreakability::BreakOnHitOrStand,
                other => return Err(format!("invalid BreakablePlatform trigger '{other}'")),
            };
            spec.respawn = parse_breakable_respawn(entity)?;
            spec.max_hp = field_i32(entity, "max_hp").unwrap_or(3);
        }
        "BreakablePogoOrb" => {
            // Pogo-orb-with-health. No body collision; while intact the
            // collision world gets a `BlockKind::PogoOrb` block emitted
            // by `world_with_sandbox_solids`, and successful pogo bounces
            // damage the orb until it breaks.
            spec.collision = SurfaceCollision::None;
            spec.breakability = SurfaceBreakability::BreakOnHit;
            spec.contact = SurfaceContact::PogoRefresh;
            spec.respawn = parse_breakable_respawn(entity)?;
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

/// Parse the `Breakable.respawn` field plus its companion `respawn_seconds`.
///
/// Accepted forms:
/// - `"Never"` (default), `"OnRoomReload"`
/// - `"AfterSeconds"` paired with a positive `respawn_seconds` float field
/// - legacy inline `"AfterSeconds:<n>"` shorthand (still accepted for older
///   instances saved before `respawn_seconds` was added)
/// - legacy `"Persistent"`, mapped to `Never`
fn parse_breakable_respawn(entity: &LdtkEntityInstance) -> Result<SurfaceRespawn, String> {
    let raw = field_string(entity, "respawn").unwrap_or_else(|| "Never".to_string());
    let trimmed = raw.trim();
    if let Some(seconds) = trimmed
        .strip_prefix("AfterSeconds:")
        .and_then(|text| text.parse::<f32>().ok())
    {
        if !(seconds > 0.0) {
            return Err(format!(
                "AfterSeconds respawn requires positive seconds, got {seconds}"
            ));
        }
        return Ok(SurfaceRespawn::AfterSeconds(seconds));
    }
    match trimmed {
        "Never" | "Persistent" | "" => Ok(SurfaceRespawn::Never),
        "OnRoomReload" => Ok(SurfaceRespawn::OnRoomReload),
        "AfterSeconds" => {
            let seconds = field_f32(entity, "respawn_seconds")
                .ok_or_else(|| "AfterSeconds respawn requires respawn_seconds".to_string())?;
            if !(seconds > 0.0) {
                return Err(format!(
                    "AfterSeconds respawn requires positive respawn_seconds, got {seconds}"
                ));
            }
            Ok(SurfaceRespawn::AfterSeconds(seconds))
        }
        other => Err(format!("invalid Breakable respawn '{other}'")),
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
            // Allow exactly one breakable+contact combo: BreakablePogoOrb,
            // which is BreakOnHit with collision=None and PogoRefresh contact.
            // The runtime emits a `BlockKind::PogoOrb` block in
            // `world_with_sandbox_solids` while the orb is intact, and the
            // sandbox damages the orb on each pogo bounce. Other
            // breakable+contact combos remain unsupported.
            let pogo_orb_combo = matches!(spec.contact, SurfaceContact::PogoRefresh)
                && matches!(spec.collision, SurfaceCollision::None)
                && matches!(breakable_kind, SurfaceBreakability::BreakOnHit);
            if !matches!(spec.contact, SurfaceContact::None) && !pogo_orb_combo {
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
            breakable.pogo_refresh = pogo_orb_combo;
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

pub(crate) fn field_string(entity: &LdtkEntityInstance, name: &str) -> Option<String> {
    field_value(&entity.field_instances, name).and_then(value_to_string)
}

pub(crate) fn field_f32(entity: &LdtkEntityInstance, name: &str) -> Option<f32> {
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

    /// Audit: no static-collision entity types should appear in the
    /// embedded LDtk file. Solid / OneWayPlatform / BlinkWall /
    /// HazardBlock all belong on the IntGrid Collision layer (per
    /// `tools/ldtk_intgrid_migration.py`); leaving them as entities
    /// is the bug the user noticed — entities don't tile, so a
    /// "Solid floor" entity stretches its single texture over a
    /// long footprint and smears.
    ///
    /// `DamageVolume` deliberately stays as an entity because it
    /// can carry motion paths and per-volume damage; the audit
    /// excludes it.
    ///
    /// If a future authoring patch reintroduces any of these
    /// entity types, this test fails and the author has to either
    /// re-run the migration or convert the spec to use IntGrid
    /// rectangles directly.
    #[test]
    fn no_static_collision_entities_in_embedded_ldtk() {
        let project = LdtkProject::load_embedded();
        const BANNED: &[&str] = &["Solid", "OneWayPlatform", "BlinkWall", "HazardBlock"];
        let mut violations: Vec<String> = Vec::new();
        for level in &project.levels {
            for layer in &level.layer_instances {
                for entity in &layer.entity_instances {
                    if BANNED.contains(&entity.identifier.as_str()) {
                        violations.push(format!(
                            "{}::{} (iid={})",
                            level.identifier, entity.identifier, entity.iid
                        ));
                    }
                }
            }
        }
        assert!(
            violations.is_empty(),
            "found {} static-collision entit{} that should be IntGrid tiles \
             (run `python tools/ldtk_intgrid_migration.py` to migrate): {:#?}",
            violations.len(),
            if violations.len() == 1 { "y" } else { "ies" },
            violations,
        );
    }

    /// IntGrid value 5 (Hazard) must round-trip through the
    /// `int_grid_value_to_block` mapping into a `BlockKind::Hazard`
    /// block. Pinning the conversion so a future renumbering can't
    /// silently drop hazard cells from the runtime collision world.
    #[test]
    fn int_grid_hazard_value_maps_to_hazard_block() {
        let block = int_grid_value_to_block(
            5,
            ae::Vec2::ZERO,
            ae::Vec2::new(16.0, 16.0),
        )
        .expect("value 5 must map to a block");
        assert!(matches!(block.kind, ae::BlockKind::Hazard));
        assert_eq!(block.name, "ldtk hazard");
    }

    #[test]
    fn level_metadata_reads_optional_biome_fields() {
        // Build a synthetic level whose fieldInstances declare every
        // optional metadata field. The reader should pick all four up
        // and produce a RoomMetadata with each Some(...).
        use serde_json::Value;
        fn field(name: &str, value: &str) -> LdtkFieldInstance {
            LdtkFieldInstance {
                identifier: name.into(),
                value: Value::String(value.into()),
                real_editor_values: vec![],
            }
        }
        let level = LdtkLevel {
            iid: "level-iid".into(),
            identifier: "metadata_level".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 256,
            px_hei: 256,
            field_instances: vec![
                field("activeArea", "metadata_area"),
                field("biome", "cave"),
                field("music_track", "loop_a"),
                field("ambient_profile", "damp"),
                field("visual_theme", "blue"),
            ],
            layer_instances: Vec::new(),
        };
        let meta = level.level_metadata();
        assert_eq!(meta.biome.as_deref(), Some("cave"));
        assert_eq!(meta.music_track.as_deref(), Some("loop_a"));
        assert_eq!(meta.ambient_profile.as_deref(), Some("damp"));
        assert_eq!(meta.visual_theme.as_deref(), Some("blue"));
    }

    #[test]
    fn level_metadata_skips_blank_strings() {
        use serde_json::Value;
        fn field(name: &str, value: &str) -> LdtkFieldInstance {
            LdtkFieldInstance {
                identifier: name.into(),
                value: Value::String(value.into()),
                real_editor_values: vec![],
            }
        }
        let level = LdtkLevel {
            iid: "level-iid".into(),
            identifier: "blank_level".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 256,
            px_hei: 256,
            field_instances: vec![
                field("activeArea", "blank_area"),
                field("biome", "   "),
                field("music_track", ""),
            ],
            layer_instances: Vec::new(),
        };
        let meta = level.level_metadata();
        assert!(meta.biome.is_none(), "whitespace-only must be treated as None");
        assert!(meta.music_track.is_none());
    }

    #[test]
    fn room_metadata_merge_first_non_empty_wins() {
        use crate::rooms::RoomMetadata;
        let mut a = RoomMetadata {
            biome: Some("hub".into()),
            music_track: None,
            ambient_profile: None,
            visual_theme: None,
        };
        let b = RoomMetadata {
            biome: Some("basement".into()),
            music_track: Some("dark_loop".into()),
            ambient_profile: Some("bass".into()),
            visual_theme: None,
        };
        a.merge(b);
        assert_eq!(a.biome.as_deref(), Some("hub"), "first non-empty wins");
        assert_eq!(
            a.music_track.as_deref(),
            Some("dark_loop"),
            "later levels fill in missing fields"
        );
        assert_eq!(a.ambient_profile.as_deref(), Some("bass"));
        assert_eq!(a.visual_theme, None);
    }

    /// Pin the biome-metadata seam end-to-end: every gameplay active
    /// area in the embedded LDtk should compose with a non-empty
    /// `biome` so the runtime resource (`ActiveRoomMetadata`) and
    /// the room-music plumbing have something to read. Regression
    /// guard for the "RoomSpec::metadata is always default" failure
    /// mode where the seam compiles but the LDtk side never set a
    /// value.
    #[test]
    fn embedded_ldtk_active_areas_have_biome_metadata() {
        let project = LdtkProject::load_embedded();
        let room_set = project.to_room_set().expect("embedded LDtk should compose");
        let mut missing: Vec<&str> = Vec::new();
        for room in &room_set.rooms {
            if room.metadata.biome.is_none() {
                missing.push(room.id.as_str());
            }
        }
        assert!(
            missing.is_empty(),
            "every embedded LDtk active area should declare a biome; missing: {missing:?}"
        );
    }

    /// `mob_lab` is the canonical "non-default music_track" example
    /// in the embedded LDtk. The room metadata flowing through to
    /// `RoomSpec::metadata.music_track` is what lets the runtime
    /// `RoomMusicRequest` swap the track when the player enters the
    /// area.
    #[test]
    fn embedded_ldtk_mob_lab_carries_music_track() {
        let project = LdtkProject::load_embedded();
        let room_set = project.to_room_set().expect("embedded LDtk should compose");
        let mob = room_set
            .rooms
            .iter()
            .find(|r| r.id == "mob_lab")
            .expect("mob_lab active area exists");
        assert_eq!(mob.metadata.biome.as_deref(), Some("mob_arena"));
        assert_eq!(
            mob.metadata.music_track.as_deref(),
            Some("pulse_drift_voyage"),
            "mob_lab should declare its non-default music track via the LDtk level field"
        );
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
    fn central_hub_collision_layer_lowers_to_engine_blocks() {
        let project = LdtkProject::load_embedded();
        let room_set = project.to_room_set().expect("embedded LDtk should compose");
        let hub = room_set
            .rooms
            .iter()
            .find(|room| room.id == "central_hub_complex")
            .expect("central hub active area exists");
        let solid_blocks = hub
            .world
            .blocks
            .iter()
            .filter(|b| matches!(b.kind, ae::BlockKind::Solid))
            .count();
        let one_way_blocks = hub
            .world
            .blocks
            .iter()
            .filter(|b| matches!(b.kind, ae::BlockKind::OneWay))
            .count();
        let blink_blocks = hub
            .world
            .blocks
            .iter()
            .filter(|b| matches!(b.kind, ae::BlockKind::BlinkWall { .. }))
            .count();
        // Step E migration painted Solid + OneWayPlatform + BlinkWall in
        // central_hub_main as IntGrid cells; the rect-merge collapses
        // adjacent same-value runs into single blocks. Each kind should
        // still produce at least one block, and the total stays well
        // below the unmerged 1004-cell count to confirm merging actually
        // ran.
        assert!(
            solid_blocks >= 1,
            "expected at least one solid IntGrid block in central hub; got {solid_blocks}"
        );
        assert!(
            one_way_blocks >= 1,
            "expected at least one OneWay IntGrid block in central hub; got {one_way_blocks}"
        );
        assert!(
            blink_blocks >= 1,
            "expected at least one BlinkWall IntGrid block in central hub; got {blink_blocks}"
        );
        let total = solid_blocks + one_way_blocks + blink_blocks;
        eprintln!(
            "central_hub_complex IntGrid blocks after merge: solid={solid_blocks} one_way={one_way_blocks} blink={blink_blocks} total={total}"
        );
        assert!(
            total < 200,
            "expected rect-merged collision count well below the 1004 unmerged cells; got {total}"
        );
    }

    #[test]
    fn intgrid_rect_merge_collapses_a_horizontal_run() {
        // 5x1 row of value=1 cells should produce a single 5*16-wide block.
        let layer = LdtkLayerInstance {
            identifier: "Collision".to_string(),
            layer_type: "IntGrid".to_string(),
            c_wid: 5,
            c_hei: 1,
            grid_size: 16,
            entity_instances: Vec::new(),
            int_grid_csv: vec![1; 5],
        };
        let blocks =
            emit_collision_blocks_from_intgrid(&layer, ae::Vec2::ZERO).expect("merge succeeds");
        assert_eq!(blocks.len(), 1, "horizontal run should merge to one block");
        let block = &blocks[0];
        assert!(matches!(block.kind, ae::BlockKind::Solid));
        let size = ae::AabbExt::half_size(block.aabb) * 2.0;
        assert!(
            (size.x - 80.0).abs() < 0.001,
            "merged width = 5 cells * 16px"
        );
        assert!((size.y - 16.0).abs() < 0.001, "merged height = 1 cell");
    }

    #[test]
    fn intgrid_rect_merge_does_not_collapse_columns_into_vertical_bars() {
        // A staircase pattern is the regression case: greedy vertical
        // merge previously collapsed each diagonal step into a tall
        // 1-wide bar, which rendered as vertical walls instead of the
        // staircase the editor shows. Horizontal-only merge keeps each
        // cell's row the way the artist painted it — so a 3-step
        // staircase produces 6 blocks (1 + 2 + 3 cells across), one per
        // run, not three vertical bars.
        let layer = LdtkLayerInstance {
            identifier: "Collision".to_string(),
            layer_type: "IntGrid".to_string(),
            c_wid: 3,
            c_hei: 3,
            grid_size: 16,
            entity_instances: Vec::new(),
            int_grid_csv: vec![
                0, 0, 1, // row 0
                0, 1, 1, // row 1
                1, 1, 1, // row 2
            ],
        };
        let blocks =
            emit_collision_blocks_from_intgrid(&layer, ae::Vec2::ZERO).expect("merge succeeds");
        assert_eq!(
            blocks.len(),
            3,
            "staircase should produce one block per row, not collapsed verticals"
        );
        let widths: Vec<i32> = blocks
            .iter()
            .map(|b| (ae::AabbExt::half_size(b.aabb).x * 2.0 / 16.0).round() as i32)
            .collect();
        assert_eq!(widths, vec![1, 2, 3]);
    }

    #[test]
    fn intgrid_rect_merge_separates_distinct_values() {
        // Row [Solid, Solid, OneWay, Solid] should produce 3 blocks: a
        // 2-cell solid, a 1-cell one-way, and a 1-cell solid.
        let layer = LdtkLayerInstance {
            identifier: "Collision".to_string(),
            layer_type: "IntGrid".to_string(),
            c_wid: 4,
            c_hei: 1,
            grid_size: 16,
            entity_instances: Vec::new(),
            int_grid_csv: vec![
                INT_GRID_SOLID,
                INT_GRID_SOLID,
                INT_GRID_ONE_WAY,
                INT_GRID_SOLID,
            ],
        };
        let blocks =
            emit_collision_blocks_from_intgrid(&layer, ae::Vec2::ZERO).expect("merge succeeds");
        assert_eq!(blocks.len(), 3);
        assert!(matches!(blocks[0].kind, ae::BlockKind::Solid));
        assert!(matches!(blocks[1].kind, ae::BlockKind::OneWay));
        assert!(matches!(blocks[2].kind, ae::BlockKind::Solid));
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
    fn one_way_platform_compiles_to_one_way_block() {
        let compiled = compile_identifier("OneWayPlatform", [96, 16], &[]);
        assert_eq!(compiled.blocks.len(), 1);
        assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::OneWay));
    }

    #[test]
    fn solid_compiles_to_solid_block() {
        let compiled = compile_identifier("Solid", [128, 32], &[]);
        assert_eq!(compiled.objects.len(), 0);
        assert_eq!(compiled.blocks.len(), 1);
        assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::Solid));
    }

    #[test]
    fn hazard_block_compiles_to_hazard_block() {
        let compiled = compile_identifier("HazardBlock", [64, 16], &[]);
        assert_eq!(compiled.blocks.len(), 1);
        assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::Hazard));
    }

    #[test]
    fn pogo_orb_compiles_to_pogo_orb_block() {
        let compiled = compile_identifier("PogoOrb", [32, 32], &[]);
        assert_eq!(compiled.blocks.len(), 1);
        assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::PogoOrb));
    }

    #[test]
    fn rebound_pad_compiles_to_rebound_block() {
        let compiled = compile_identifier(
            "ReboundPad",
            [32, 16],
            &[
                ("impulseX", Value::Number(serde_json::Number::from(0))),
                ("impulseY", Value::Number(serde_json::Number::from(-600))),
            ],
        );
        assert_eq!(compiled.blocks.len(), 1);
        assert!(matches!(
            compiled.blocks[0].kind,
            ae::BlockKind::Rebound { .. }
        ));
    }

    #[test]
    fn blink_wall_uses_tier_field() {
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
    fn rebound_pad_requires_impulse_fields() {
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

    /// `BreakablePlatform` with `collision=Solid` lowers to a Breakable
    /// runtime object with hard collision while intact.
    #[test]
    fn breakable_platform_solid_compiles_with_solid_collision() {
        let compiled = compile_identifier(
            "BreakablePlatform",
            [48, 48],
            &[
                ("collision", Value::String("Solid".into())),
                ("trigger", Value::String("OnHit".into())),
                ("max_hp", Value::Number(serde_json::Number::from(2))),
            ],
        );
        assert!(compiled.blocks.is_empty());
        assert_eq!(compiled.objects.len(), 1);
        match &compiled.objects[0].kind {
            ae::RoomObjectKind::Breakable(breakable) => {
                assert_eq!(breakable.collision, ae::BreakableCollision::Solid);
                assert_eq!(breakable.trigger, ae::BreakableTrigger::OnHit);
                assert_eq!(breakable.health.max, 2);
                assert!(!breakable.pogo_refresh);
            }
            other => panic!("expected Breakable, got {other:?}"),
        }
    }

    /// `BreakablePlatform` with `collision=OneWayUp` lowers to a Breakable
    /// runtime object that lands as a one-way platform.
    #[test]
    fn breakable_platform_one_way_up_compiles() {
        let compiled = compile_identifier(
            "BreakablePlatform",
            [80, 16],
            &[
                ("collision", Value::String("OneWayUp".into())),
                ("trigger", Value::String("OnStand".into())),
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

    /// `BreakablePlatform` rejects unknown collision values. The LDtk enum
    /// has only Solid|OneWayUp, so the previous OnStand+None combo is
    /// unrepresentable in the editor and we don't even need a degrade path.
    #[test]
    fn breakable_platform_rejects_unknown_collision() {
        let entity = make_entity(
            "BreakablePlatform",
            [32, 32],
            &[("collision", Value::String("None".into()))],
        );
        let err = parse_surface_spec(
            &entity,
            ae::Vec2::ZERO,
            ae::Vec2::new(32.0, 32.0),
            "p".into(),
        )
        .expect_err("None is not a valid BreakablePlatform collision");
        assert!(err.contains("BreakablePlatform"), "{err}");
    }

    /// Engine compile path stays strict: a hand-crafted incoherent combo
    /// (BreakOnStand with collision=None) is still rejected, even though
    /// the LDtk adapter can no longer produce one for BreakablePlatform.
    #[test]
    fn engine_compile_still_rejects_on_stand_without_collision() {
        let bad_spec = LdtkSurfaceSpec {
            iid: "test".into(),
            name: "test".into(),
            min: ae::Vec2::ZERO,
            size: ae::Vec2::new(32.0, 32.0),
            collision: SurfaceCollision::None,
            breakability: SurfaceBreakability::BreakOnStand,
            contact: SurfaceContact::None,
            respawn: SurfaceRespawn::Never,
            max_hp: 3,
        };
        let err = compile_surface(&bad_spec).expect_err("BreakOnStand requires collision");
        assert!(
            err.contains("BreakOnStand requires non-None collision"),
            "{err}"
        );
    }

    /// `respawn = AfterSeconds` requires a positive `respawn_seconds` field.
    #[test]
    fn breakable_platform_after_seconds_requires_positive_respawn_seconds() {
        let missing_field = make_entity(
            "BreakablePlatform",
            [32, 32],
            &[
                ("collision", Value::String("Solid".into())),
                ("trigger", Value::String("OnHit".into())),
                ("respawn", Value::String("AfterSeconds".into())),
            ],
        );
        let err = parse_surface_spec(
            &missing_field,
            ae::Vec2::ZERO,
            ae::Vec2::new(32.0, 32.0),
            "p".into(),
        )
        .expect_err("AfterSeconds without respawn_seconds is rejected");
        assert!(err.contains("respawn_seconds"), "{err}");

        let zero_seconds = make_entity(
            "BreakablePlatform",
            [32, 32],
            &[
                ("collision", Value::String("Solid".into())),
                ("trigger", Value::String("OnHit".into())),
                ("respawn", Value::String("AfterSeconds".into())),
                (
                    "respawn_seconds",
                    Value::Number(serde_json::Number::from(0)),
                ),
            ],
        );
        let err = parse_surface_spec(
            &zero_seconds,
            ae::Vec2::ZERO,
            ae::Vec2::new(32.0, 32.0),
            "p".into(),
        )
        .expect_err("respawn_seconds must be positive");
        assert!(err.contains("positive"), "{err}");
    }

    /// `BreakablePogoOrb` lowers to a Breakable with the `pogo_refresh`
    /// flag set, so the gameplay loop emits a PogoOrb collision-world
    /// block while intact and routes pogo bounces back as damage.
    #[test]
    fn breakable_pogo_orb_compiles_with_pogo_flag() {
        let compiled = compile_identifier(
            "BreakablePogoOrb",
            [36, 36],
            &[("max_hp", Value::Number(serde_json::Number::from(4)))],
        );
        assert!(compiled.blocks.is_empty());
        assert_eq!(compiled.objects.len(), 1);
        match &compiled.objects[0].kind {
            ae::RoomObjectKind::Breakable(breakable) => {
                assert!(breakable.pogo_refresh);
                assert_eq!(breakable.collision, ae::BreakableCollision::None);
                assert_eq!(breakable.trigger, ae::BreakableTrigger::OnHit);
                assert_eq!(breakable.health.max, 4);
            }
            other => panic!("expected Breakable, got {other:?}"),
        }
    }

    #[test]
    fn no_surface_authoring_primitive_is_registered() {
        // The LDtk editor stays differentiated; there should be no canonical
        // generic Surface entity registered or routed through the parser.
        assert!(
            !known_entity("Surface"),
            "Surface must not be a registered LDtk entity"
        );
        assert!(
            !is_surface_like_identifier("Surface"),
            "Surface must not route through the typed surface conversion path"
        );
        // The legacy generic `Breakable` is gone; only the narrow types
        // remain.
        assert!(!known_entity("Breakable"), "legacy Breakable was removed");
        assert!(
            !is_surface_like_identifier("Breakable"),
            "legacy Breakable parser branch was removed"
        );
        // Differentiated identifiers DO still route through the typed
        // conversion path.
        for id in [
            "Solid",
            "OneWayPlatform",
            "BlinkWall",
            "HazardBlock",
            "PogoOrb",
            "ReboundPad",
            "BreakablePlatform",
            "BreakablePogoOrb",
        ] {
            assert!(is_surface_like_identifier(id), "{id}");
        }
        for id in ["PlayerStart", "LoadingZone", "DebugLabel", "NpcSpawn"] {
            assert!(!is_surface_like_identifier(id), "{id}");
        }
    }
}
