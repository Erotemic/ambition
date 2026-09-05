//! LDtk authoring adapter for the platformer world IR.
//!
//! Gameplay/world concepts remain typed in Rust; LDtk is one backend that validates
//! authored entities and lowers them into `ambition_platformer2d_world`. Format-
//! independent world manifests stay in the world crate.
//!
//! `bevy_ecs_ldtk` integration is available only with the `ldtk_runtime` feature;
//! pure project parsing, validation, conversion, fields, and surfaces do not require
//! that runtime feature.

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
// The LDtk entity-converter registry (ADR 0009): content registers
// game-specific entity converters at plugin-build time; the engine's
// standard vocabulary enters through the same registry.
pub use conversion::{
    kinematic_path_lookup_id, LdtkEntityConverter, LdtkEntityCtx, LdtkVocabulary, RoomEmission,
};
// World manifests are format-independent and are intentionally not re-exported
// from the LDtk adapter; callers name `ambition_platformer2d_world` directly.
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
        // Zones that name NO target: the landing pad half of a one-way trip.
        // `(area, zone id, iid)` — see the `LoadingZone` arm for why they are
        // legal and what still has to hold for them.
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
                        // Conversion has always had both shapes — a zone with no target contributes
                        // no `RoomLink`, and `transition_from_zone` only fires on a zone with an
                        // outgoing edge — so the arrival end of a one-way trip was expressible at
                        // runtime and unauthorable in a file.
                        //
                        //  a landing pad that names a target is a BOUNCE.
                        // The body arrives standing inside the zone it arrived
                        // through (`door_arrival` = zone centre, 26px off its
                        // floor), so the moment the transition cooldown lapses
                        // that zone fires and sends it straight back.
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
                        //  ONE parse, not a second copy of the token set.
                        // This read `== "EdgeExit"` while the converter matched
                        // its own list — two spellings of one vocabulary, free
                        // to disagree. An unrecognised value is reported below
                        // rather than silently becoming a Door.
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
                            //  AND THE COLLISION GRID, which is what a body
                            // actually collides with. The rule above scans
                            // entities named `Solid`; these levels paint their
                            // floors and walls into the Collision IntGrid, so the
                            // reachability rule could not fire on the case it was
                            // written for. Five of twenty-four authored EdgeExits
                            //
                            //  AND THE FIRST REPLACEMENT ASKED A PROXY
                            // TOO. It counted solid cells inside the zone and
                            // warned on any, which flagged five of twenty-four
                            // exits — and three of those five were correct
                            // authoring: their bottom row is solid because that
                            // row IS THE FLOOR, unbroken across the level, and a
                            // zone stopping above the floor could never be
                            // touched by a body standing on it.
                            //
                            //  the question is whether the ground INSIDE is
                            // higher than the ground you walk in from. See
                            // `edge_exit_step_up_px`. It answers 0 for every
                            // authored EdgeExit now that the hub's two sills are
                            // cleared, so this is ready to be an error.
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

        //  the typo the blanket rule used to catch, kept. A landing pad
        // nothing arrives through is dead geometry, and an exit whose target
        // fields were never filled in reads exactly like one.
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

    /// Cross-validate level `music_track` fields against the catalog of
    /// audio-side track ids loaded from `Platformer2dGameplayDefaults`. Returns one
    /// warning per (level, unknown_id) pair so the user can see all
    /// typos in a single startup pass instead of debugging room-by-room.
    ///
    /// Lives here (not on `validate()`) because the LDtk validator must
    /// stay self-contained — the audio catalog is only known once
    /// `Platformer2dGameplayDefaults` is loaded. Callers (visible binary's
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

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
