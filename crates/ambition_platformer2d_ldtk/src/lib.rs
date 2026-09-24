//! LDtk authoring adapter for the platformer world IR.
//!
//! Gameplay and world concepts stay typed in Rust. LDtk is one backend that
//! validates authored entities and lowers them into
//! `ambition_platformer2d_world`. Format-independent world manifests stay in
//! the world crate.
//!
//! `bevy_ecs_ldtk` integration needs the `ldtk_runtime` feature. Project
//! parsing, validation, conversion, fields, and surfaces do not.

use std::collections::{BTreeMap, BTreeSet};

use ambition_platformer2d_core as ae;

// Only the Bevy runtime adapter requires `bevy_ecs_ldtk`.
#[cfg(feature = "ldtk_runtime")]
pub mod bevy_runtime;
pub mod contract;
mod conversion;
mod fields;
mod intgrid;
mod loading;
mod project;
mod surfaces;

#[cfg(feature = "ldtk_runtime")]
pub use bevy_runtime::*;
// The LDtk entity-converter registry (ADR 0009): content registers game
// converters at plugin-build time; the engine's standard vocabulary uses the
// same registry.
pub use conversion::{
    kinematic_path_lookup_id, LdtkEntityConverter, LdtkEntityCtx, LdtkVocabulary, RoomEmission,
};
// World manifests are format-independent and are not re-exported here;
// callers name `ambition_platformer2d_world` directly.
pub use ambition_platformer2d_world::ron_room::{
    load_ron_rooms, room_doc_from_ron, room_doc_to_ron, RonRoomDoc,
};
pub use project::{
    ActiveLdtkProject, LdtkEntityInstance, LdtkFieldInstance, LdtkLayerInstance, LdtkLevel,
    LdtkProject,
};
pub use surfaces::{
    compile_surface, LdtkSurfaceSpec, SurfaceBreakability, SurfaceCollision, SurfaceCompiled,
    SurfaceContact, SurfaceRespawn,
};

// Field accessors used by entity converters.
pub use fields::{boss_placement_id, field_bool, field_f32, field_i32, field_string};

use fields::{
    edge_exit_step_up_px, entity_rect, entity_touches_level_edge, known_entity, pivot_is_top_left,
    rects_strict_intersect,
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
    pub fn validate(&self, vocabulary: &conversion::LdtkVocabulary) -> LdtkValidationReport {
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
        // Zones that name no target: the landing-pad end of a one-way trip.
        // `(area, zone id, iid)`. See the `LoadingZone` arm.
        let mut landing_pads: Vec<(String, String, String)> = Vec::new();

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
                if !known_entity(&entity.identifier, vocabulary) {
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
                        // A zone with no target is legal: it adds no `RoomLink`, and
                        // `transition_from_zone` fires only on a zone with an outgoing edge. It is the
                        // arrival end of a one-way trip.
                        //
                        // A landing pad must not name a target. The body arrives inside the zone
                        // (`door_arrival` = zone centre, 26px off its floor), so when the transition
                        // cooldown ends the zone fires and sends it back.
                        let has_target_room = field_string(entity, "target_room")
                            .is_some_and(|value| !value.trim().is_empty());
                        let has_target_zone = field_string(entity, "target_zone")
                            .is_some_and(|value| !value.trim().is_empty());
                        if !has_target_room && !has_target_zone {
                            landing_pads.push((
                                active_area.clone(),
                                field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
                                entity.iid.clone(),
                            ));
                        } else if !(has_target_room && has_target_zone) {
                            report.errors.push(format!(
                                "LoadingZone {} names half a target; an exit needs both \
                                 target_room and target_zone, a landing pad needs neither",
                                entity.iid
                            ));
                        }
                        // Parse with `LoadingZoneActivation::from_authored`, the same parser the
                        // converter uses. An unknown value is reported below, not treated as a Door.
                        let authored = field_string(entity, "activation")
                            .unwrap_or_else(|| "Door".to_string());
                        let activation =
                            ambition_platformer2d_world::rooms::LoadingZoneActivation::from_authored(
                                &authored,
                            );
                        if activation.is_none() {
                            report.errors.push(format!(
                                "LoadingZone {} in level '{}' has activation '{authored}', which is not one of {:?}",
                                entity.iid,
                                level.identifier,
                                ambition_platformer2d_world::rooms::LoadingZoneActivation::AUTHORED_SPELLINGS
                            ));
                        }
                        if activation
                            == Some(
                                ambition_platformer2d_world::rooms::LoadingZoneActivation::EdgeExit,
                            )
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
                            // Also check the Collision IntGrid, which is what a body collides with. The
                            // rule above scans only `Solid` entities, but these levels paint floors and
                            // walls into the IntGrid.
                            //
                            // Do not warn on any solid cell inside the zone: the zone's bottom row is
                            // often the floor itself, and that is correct authoring. The question is
                            // whether the ground inside is higher than the ground you walk in from (see
                            // `edge_exit_step_up_px`). It is 0 for every authored EdgeExit, so this can
                            // become an error.
                            let step = edge_exit_step_up_px(level, entity_rect(entity));
                            if step > 0 {
                                report.warnings.push(format!(
                                    "EdgeExit LoadingZone {} in level '{}' sits {step}px above the ground it is entered from; a walking body stalls against the sill and the exit can only be entered by jumping",
                                    entity.iid, level.identifier
                                ));
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
                // Validate Surface-shaped entities by parsing into `LdtkSurfaceSpec` and
                // running the compile path that makes runtime data. This is the single source
                // of truth for field combinations of `Surface` and its legacy aliases.
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
                // Do not warn on empty `realEditorValues`. LDtk 1.5.3 writes that shape for
                // fields that inherit from the entity-def `defaultOverride`, and a file the
                // LDtk editor writes must run unchanged.
            }
        }

        // A landing pad that nothing arrives through is dead geometry, and it looks
        // the same as an exit whose target fields were never filled in.
        if !landing_pads.is_empty() {
            let arrivals: BTreeSet<(String, String)> = self
                .collect_room_links()
                .into_iter()
                .map(|link| (link.to_room, link.to_zone))
                .collect();
            for (area, zone_id, iid) in landing_pads {
                if !arrivals.contains(&(area.clone(), zone_id.clone())) {
                    report.errors.push(format!(
                        "LoadingZone {iid} ('{zone_id}' in area '{area}') names no target and \
                         nothing arrives through it; give it a target_room/target_zone or point \
                         a zone at it"
                    ));
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

    /// Check level `music_track` fields against the audio track ids from
    /// `Platformer2dGameplayDefaults`. Returns one warning per (level, unknown_id)
    /// pair, so all typos show in one startup pass.
    ///
    /// Not part of `validate()`: the LDtk validator must stay self-contained, and
    /// the audio catalog is known only after `Platformer2dGameplayDefaults` loads.
    /// Callers (the visible binary's `init_sandbox_resources`, headless tests)
    /// connect both.
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

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
