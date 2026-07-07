//! LDtk world-composition adapter and validator for the sandbox.
//!
//! Ambition keeps its gameplay model typed in Rust. LDtk is an authoring
//! frontend: this module validates the subset of LDtk entities Ambition
//! currently understands and materializes Ambition runtime rooms directly
//! from LDtk-authored data.
//!
//! ## Submodule layout (post-2026-05-09 split)
//!
//! Submodules are private (`mod`); user-facing types are re-exported
//! from the root of this module.
//!
//! - `project` — JSON deserialization types ([`LdtkProject`],
//!   [`LdtkLevel`], [`LdtkLayerInstance`], [`LdtkEntityInstance`],
//!   [`LdtkFieldInstance`], [`SandboxLdtkProject`]).
//! - `loading` — file-loading policy
//!   ([`LdtkProject::load_default`] (catalog-aware),
//!   [`LdtkProject::load_default_for_dev`] (no-catalog test/headless
//!   helper), `load_static_map`, `load_from_disk_at`, `load_from_path`).
//! - `conversion` — LDtk → Ambition runtime conversion
//!   ([`LdtkProject::to_room_set`], `entity_to_runtime`).
//! - `bevy_runtime` — bevy_ecs_ldtk plugin glue + runtime-spine
//!   indexing.
//! - `hot_reload` — file-watch + transactional reload state.
//! - `intgrid`, `fields`, `surfaces` — IntGrid emission, field
//!   accessors, typed `Surface` parsing.
//! - `tests` (cfg(test) only) — internal tests, split by topic
//!   (`embedded_project`, `intgrid`, `kinematic_paths`, `metadata`,
//!   `surfaces`).

use std::collections::{BTreeMap, BTreeSet};

use ambition_engine_core as ae;

pub mod bevy_runtime;
mod conversion;
mod fields;
mod hot_reload;
mod intgrid;
mod loading;
mod manifest;
mod project;
mod ron_room;
mod surfaces;

pub use bevy_runtime::*;
// The LDtk entity-converter registry (ADR 0009): content registers
// game-specific entity converters at plugin-build time; the engine's
// standard vocabulary enters through the same registry.
pub use conversion::{
    install_ldtk_entity_converters, LdtkEntityConverter, LdtkEntityCtx, RoomEmission,
};
pub use hot_reload::{poll_ldtk_file_changes, LdtkHotReloadState};
// The WorldManifest install seam (JD4): a game declares its LDtk worlds +
// entry room; the engine ships zero worlds and hardcodes no start room.
pub use manifest::{
    install_world_manifest, world_bevy_asset_path, world_manifest, WorldManifest, WorldSource,
};
pub use project::{
    LdtkEntityInstance, LdtkFieldInstance, LdtkLayerInstance, LdtkLevel, LdtkProject,
    SandboxLdtkProject,
};
pub use ron_room::{load_manifest_ron_rooms, room_doc_from_ron, room_doc_to_ron, RonRoomDoc};
pub use surfaces::{
    compile_surface, LdtkSurfaceSpec, SurfaceBreakability, SurfaceCollision, SurfaceCompiled,
    SurfaceContact, SurfaceRespawn,
};

// Field accessors for converter authors (content converters parse their
// authored fields through these) and historical internal callers
// (e.g. `crate::encounter`).
pub use fields::{field_bool, field_f32, field_i32, field_string};

use fields::{
    entity_rect, entity_touches_level_edge, known_entity, pivot_is_top_left, rects_strict_intersect,
};
use intgrid::{AMBITION_LAYER, GRID};
use surfaces::{is_surface_like_identifier, parse_surface_spec};

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
                // writes must run unchanged.
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
}
