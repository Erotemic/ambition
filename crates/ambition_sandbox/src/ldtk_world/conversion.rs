//! LDtk → Ambition runtime conversion.
//!
//! Materializes the typed [`crate::rooms::RoomSet`] graph from a
//! validated [`super::project::LdtkProject`]. Per-entity routing
//! (`entity_to_runtime`) plus IntGrid → block / water / climbable
//! emission live here.

use std::collections::BTreeMap;

use ambition_engine as ae;

use super::fields::{
    field_bool, field_f32, field_i32, field_string, parse_boss_brain, parse_debug_label_kind,
    parse_enemy_brain, parse_optional_path, parse_path_mode, parse_pickup_kind, parse_points,
};
use super::intgrid::{
    emit_climbable_regions_from_intgrid, emit_collision_blocks_from_intgrid,
    emit_water_regions_from_intgrid,
};
use super::project::{LdtkEntityInstance, LdtkLevel, LdtkProject};
use super::surfaces::{
    compile_surface, is_surface_like_identifier, parse_surface_spec, SurfaceCompiled,
};
use crate::rooms::{LoadingZone, LoadingZoneActivation, RoomLink, RoomSet, RoomSpec};

impl LdtkProject {
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
        let mut moving_platforms: Vec<crate::platforms::MovingPlatformState> = Vec::new();
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
                        moving_platforms.extend(emission.moving_platforms);
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
            // keeping the IntGrid as the authoring representation.
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
            // vine / climbable wall region.
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
            moving_platforms,
        })
    }

    pub(super) fn area_has_player_start(&self, area: &str) -> bool {
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

/// Aggregated runtime emission for one LDtk entity instance.
///
/// LDtk entities historically mapped 1:1 to a single emitted runtime piece.
/// With `Surface`, a single LDtk entity can compile into multiple emissions
/// (e.g. a Block for static collision plus an Object for breakable lifetime),
/// so the conversion API yields a struct rather than a one-of enum.
#[derive(Clone, Debug, Default)]
pub(super) struct RuntimeEntityEmission {
    pub(super) spawn: Option<ae::Vec2>,
    pub(super) blocks: Vec<ae::Block>,
    pub(super) zones: Vec<LoadingZone>,
    pub(super) objects: Vec<ae::RoomObject>,
    pub(super) water_regions: Vec<ae::WaterRegion>,
    /// LDtk-authored moving platforms emitted by this entity.
    ///
    /// Most entities emit zero platforms; `MovingPlatform` emits one. The room
    /// composer concatenates these so active areas can own multiple authored
    /// moving solids.
    pub(super) moving_platforms: Vec<crate::platforms::MovingPlatformState>,
    pub(super) ignored: bool,
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
            moving_platforms: vec![state],
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

pub(super) fn entity_to_runtime(
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
