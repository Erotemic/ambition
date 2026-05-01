//! LDtk world-composition adapter and validator for the sandbox.
//!
//! Ambition keeps its gameplay model typed in Rust. LDtk is an authoring
//! frontend: this module validates the subset of LDtk entities Ambition
//! currently understands, then flattens placed LDtk levels into one or more
//! continuous `RoomManifestSpec` active areas.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::Value;

use crate::data::{
    BlinkWallTierSpec, BlockSpec, BossBrainSpec, DebugLabelKindSpec, EnemyBrainSpec,
    InteractionKindSpec, LoadingZoneActivationSpec, LoadingZoneSpec, PickupKindSpec,
    RespawnPolicySpec, KinematicPathModeSpec, KinematicPathSpec, RoomLinkSpec, RoomManifestSpec, RoomObjectSpec, RoomSpecData, ShellSpec,
};

const AMBITION_LAYER: &str = "Ambition";
const GRID: i32 = 16;

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

    pub fn validate(&self) -> LdtkValidationReport {
        let mut report = LdtkValidationReport::default();
        if self.json_version.trim().is_empty() {
            report.errors.push("project jsonVersion is empty".to_string());
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
                report.errors.push(format!("duplicate LDtk level identifier '{}'", level.identifier));
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
            *level_count_by_area.entry(active_area.clone()).or_default() += 1;

            let Some(layer) = level.ambition_layer() else {
                report.errors.push(format!("level '{}' is missing '{AMBITION_LAYER}' entity layer", level.identifier));
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
                        level.identifier, entity.identifier, entity.iid, entity.width, entity.height
                    ));
                }
                if entity.px[0] < 0 || entity.px[1] < 0 || entity.px[0] + entity.width > level.px_wid || entity.px[1] + entity.height > level.px_hei {
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
                        *player_starts_by_area.entry(active_area.clone()).or_default() += 1;
                    }
                    "LoadingZone" => {
                        if field_string(entity, "id").is_none() {
                            report.errors.push(format!("LoadingZone {} is missing string field 'id'", entity.iid));
                        }
                        if field_string(entity, "target_room").is_none() || field_string(entity, "target_zone").is_none() {
                            report.errors.push(format!(
                                "LoadingZone {} requires target_room and target_zone fields",
                                entity.iid
                            ));
                        }
                        if field_string(entity, "activation").unwrap_or_else(|| "Door".to_string()) == "EdgeExit" {
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
                    "BlinkWall" => {
                        let tier = field_string(entity, "tier").unwrap_or_else(|| "Soft".to_string());
                        if !matches!(tier.as_str(), "Soft" | "Hard") {
                            report.errors.push(format!("BlinkWall {} has invalid tier '{tier}'", entity.iid));
                        }
                    }
                    "ReboundPad" => {
                        if field_f32(entity, "impulseX").is_none() || field_f32(entity, "impulseY").is_none() {
                            report.errors.push(format!("ReboundPad {} requires impulseX and impulseY fields", entity.iid));
                        }
                    }
                    "DebugLabel" => {
                        if field_string(entity, "text").is_none() {
                            report.errors.push(format!("DebugLabel {} requires text field", entity.iid));
                        }
                    }
                    _ => {}
                }
            }
        }

        for (area, count) in player_starts_by_area {
            if count != 1 {
                report.errors.push(format!("active area '{area}' has {count} PlayerStart entities; expected exactly 1"));
            }
        }
        for area in level_count_by_area.keys() {
            if !self.area_has_player_start(area) {
                report.errors.push(format!("active area '{area}' has no PlayerStart"));
            }
        }

        report
    }

    pub fn to_room_manifest(&self) -> Result<RoomManifestSpec, Vec<String>> {
        let report = self.validate();
        if !report.is_ok() {
            return Err(report.errors);
        }

        let mut area_levels: BTreeMap<String, Vec<&LdtkLevel>> = BTreeMap::new();
        for level in &self.levels {
            area_levels.entry(level.active_area()).or_default().push(level);
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
        let links = self.collect_links();
        let mut rooms = Vec::new();
        for (area_id, levels) in area_levels {
            rooms.push(self.compose_area(&area_id, &levels)?);
        }
        Ok(RoomManifestSpec { start_room, rooms, links })
    }

    fn collect_links(&self) -> Vec<RoomLinkSpec> {
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
                links.push(RoomLinkSpec {
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

    fn compose_area(&self, area_id: &str, levels: &[&LdtkLevel]) -> Result<RoomSpecData, Vec<String>> {
        let mut errors = Vec::new();
        let min_x = levels.iter().map(|level| level.world_x).min().unwrap_or(0) as f32;
        let min_y = levels.iter().map(|level| level.world_y).min().unwrap_or(0) as f32;
        let max_x = levels.iter().map(|level| level.world_x + level.px_wid).max().unwrap_or(0) as f32;
        let max_y = levels.iter().map(|level| level.world_y + level.px_hei).max().unwrap_or(0) as f32;
        let mut spawn = None;
        let mut blocks = Vec::new();
        let mut zones = Vec::new();
        let mut objects = Vec::new();
        for level in levels {
            // AMBITION_REVIEW(spatial): LDtk world coordinates are flattened into
            // active-area-local Ambition coordinates here. Wall openings, edge
            // exits, transition arrivals, and camera bounds all depend on this
            // convention staying stable.
            let offset = [level.world_x as f32 - min_x, level.world_y as f32 - min_y];
            let Some(layer) = level.ambition_layer() else {
                errors.push(format!("level '{}' missing Ambition layer", level.identifier));
                continue;
            };
            for entity in &layer.entity_instances {
                match entity_to_spec(entity, offset) {
                    EntityConversion::Spawn(value) => spawn = Some(value),
                    EntityConversion::Block(block) => blocks.push(block),
                    EntityConversion::Zone(zone) => zones.push(zone),
                    EntityConversion::Object(object) => objects.push(object),
                    EntityConversion::Ignored => {}
                    EntityConversion::Error(error) => errors.push(format!("{} {}: {error}", entity.identifier, entity.iid)),
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(RoomSpecData {
            id: area_id.to_string(),
            name: format!("Ambition: {}", area_id.replace('_', " ")),
            size: [max_x - min_x, max_y - min_y],
            spawn: spawn.unwrap_or([96.0, 96.0]),
            shell: ShellSpec { enabled: false, openings: Vec::new() },
            blocks,
            zones,
            objects,
        })
    }

    fn area_has_player_start(&self, area: &str) -> bool {
        self.levels.iter().any(|level| {
            level.active_area() == area
                && level
                    .ambition_layer()
                    .map(|layer| layer.entity_instances.iter().any(|entity| entity.identifier == "PlayerStart"))
                    .unwrap_or(false)
        })
    }
}

impl LdtkLevel {
    fn active_area(&self) -> String {
        self.field_string("activeArea")
            .unwrap_or_else(|| self.identifier.clone())
    }

    fn ambition_layer(&self) -> Option<&LdtkLayerInstance> {
        self.layer_instances.iter().find(|layer| layer.identifier == AMBITION_LAYER)
    }

    fn field_string(&self, name: &str) -> Option<String> {
        field_value(&self.field_instances, name).and_then(value_to_string)
    }
}

enum EntityConversion {
    Spawn([f32; 2]),
    Block(BlockSpec),
    Zone(LoadingZoneSpec),
    Object(RoomObjectSpec),
    Ignored,
    Error(String),
}

fn entity_to_spec(entity: &LdtkEntityInstance, offset: [f32; 2]) -> EntityConversion {
    let min = [entity.px[0] as f32 + offset[0], entity.px[1] as f32 + offset[1]];
    let size = [entity.width as f32, entity.height as f32];
    let name = field_string(entity, "name").unwrap_or_else(|| entity.identifier.clone());
    match entity.identifier.as_str() {
        "PlayerStart" => EntityConversion::Spawn([min[0] + size[0] * 0.5, min[1] + size[1] * 0.5]),
        "Solid" => EntityConversion::Block(BlockSpec::Solid { name, min, size }),
        "OneWayPlatform" => EntityConversion::Block(BlockSpec::OneWay { name, min, size }),
        "BlinkWall" => {
            let tier = match field_string(entity, "tier").unwrap_or_else(|| "Soft".to_string()).as_str() {
                "Soft" => BlinkWallTierSpec::Soft,
                "Hard" => BlinkWallTierSpec::Hard,
                other => return EntityConversion::Error(format!("invalid BlinkWall tier '{other}'")),
            };
            EntityConversion::Block(BlockSpec::BlinkWall { name, min, size, tier })
        }
        "HazardBlock" => EntityConversion::Block(BlockSpec::Hazard { name, min, size }),
        "PogoOrb" => {
            let radius = size[0].min(size[1]) * 0.5;
            EntityConversion::Block(BlockSpec::PogoOrb {
                name,
                center: [min[0] + size[0] * 0.5, min[1] + size[1] * 0.5],
                radius,
            })
        }
        "ReboundPad" => {
            let Some(impulse_x) = field_f32(entity, "impulseX") else {
                return EntityConversion::Error("missing impulseX".to_string());
            };
            let Some(impulse_y) = field_f32(entity, "impulseY") else {
                return EntityConversion::Error("missing impulseY".to_string());
            };
            EntityConversion::Block(BlockSpec::Rebound { name, min, size, impulse: [impulse_x, impulse_y] })
        }
        "LoadingZone" => EntityConversion::Zone(LoadingZoneSpec {
            id: field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
            name,
            activation: match field_string(entity, "activation").unwrap_or_else(|| "Door".to_string()).as_str() {
                "EdgeExit" => LoadingZoneActivationSpec::EdgeExit,
                _ => LoadingZoneActivationSpec::Door,
            },
            min,
            size,
        }),
        "DamageVolume" => EntityConversion::Object(RoomObjectSpec::DamageVolume {
            id: entity.iid.clone(),
            name,
            min,
            size,
            damage: field_i32(entity, "damage").unwrap_or(1),
            path: parse_optional_path(entity),
        }),
        "KinematicPath" => {
            let points = parse_points(&field_string(entity, "points").unwrap_or_default());
            if points.len() < 2 {
                return EntityConversion::Error("KinematicPath requires at least two points".to_string());
            }
            EntityConversion::Object(RoomObjectSpec::KinematicPath {
                id: entity.iid.clone(),
                name,
                min,
                size,
                points,
                speed: field_f32(entity, "speed").unwrap_or(100.0),
                mode: parse_path_mode(&field_string(entity, "mode").unwrap_or_else(|| "PingPong".to_string())),
            })
        },
        "NpcSpawn" => EntityConversion::Object(RoomObjectSpec::Interactable {
            id: entity.iid.clone(),
            name,
            prompt: field_string(entity, "prompt").unwrap_or_else(|| "Talk".to_string()),
            min,
            size,
            kind: InteractionKindSpec::Npc { dialogue_id: field_string(entity, "dialogue_id") },
        }),
        "PickupSpawn" => EntityConversion::Object(RoomObjectSpec::Pickup {
            id: entity.iid.clone(),
            name,
            min,
            size,
            kind: parse_pickup_kind(&field_string(entity, "kind").unwrap_or_else(|| "health:1".to_string())),
        }),
        "ChestSpawn" => EntityConversion::Object(RoomObjectSpec::Chest {
            id: entity.iid.clone(),
            name,
            min,
            size,
            reward: field_string(entity, "reward").map(|value| parse_pickup_kind(&value)),
        }),
        "Breakable" => EntityConversion::Object(RoomObjectSpec::Breakable {
            id: entity.iid.clone(),
            name,
            min,
            size,
            max_hp: field_i32(entity, "max_hp").unwrap_or(3),
            respawn: parse_respawn(&field_string(entity, "respawn").unwrap_or_else(|| "Never".to_string())),
            solid: field_bool(entity, "solid").unwrap_or(false),
        }),
        "EnemySpawn" => EntityConversion::Object(RoomObjectSpec::EnemySpawn {
            id: entity.iid.clone(),
            name,
            min,
            size,
            brain: parse_enemy_brain(&field_string(entity, "brain").unwrap_or_else(|| "Passive".to_string())),
        }),
        "BossSpawn" => EntityConversion::Object(RoomObjectSpec::BossSpawn {
            id: entity.iid.clone(),
            name,
            min,
            size,
            brain: parse_boss_brain(&field_string(entity, "brain").unwrap_or_else(|| "Dormant".to_string())),
        }),
        "DebugLabel" => EntityConversion::Object(RoomObjectSpec::DebugLabel {
            id: entity.iid.clone(),
            name,
            position: [min[0] + size[0] * 0.5, min[1] + size[1] * 0.5],
            text: field_string(entity, "text").unwrap_or_else(|| entity.identifier.clone()),
            category: parse_debug_label_kind(&field_string(entity, "category").unwrap_or_else(|| "Custom".to_string())),
        }),
        "CameraZone" | "StitchedBoundary" => EntityConversion::Ignored,
        _ => EntityConversion::Error(format!("unsupported entity identifier '{}'", entity.identifier)),
    }
}

fn known_entity(identifier: &str) -> bool {
    matches!(
        identifier,
        "PlayerStart"
            | "Solid"
            | "OneWayPlatform"
            | "BlinkWall"
            | "HazardBlock"
            | "PogoOrb"
            | "ReboundPad"
            | "LoadingZone"
            | "DamageVolume"
            | "KinematicPath"
            | "NpcSpawn"
            | "PickupSpawn"
            | "ChestSpawn"
            | "Breakable"
            | "EnemySpawn"
            | "BossSpawn"
            | "DebugLabel"
            | "CameraZone"
            | "StitchedBoundary"
    )
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
    fields.iter().find(|field| field.identifier == name).map(|field| &field.value)
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

fn parse_points(value: &str) -> Vec<[f32; 2]> {
    value
        .split(';')
        .filter_map(|pair| {
            let mut parts = pair.split(',').map(str::trim);
            let x = parts.next()?.parse::<f32>().ok()?;
            let y = parts.next()?.parse::<f32>().ok()?;
            Some([x, y])
        })
        .collect()
}

fn parse_path_mode(value: &str) -> KinematicPathModeSpec {
    match value {
        "Once" => KinematicPathModeSpec::Once,
        "Loop" => KinematicPathModeSpec::Loop,
        _ => KinematicPathModeSpec::PingPong,
    }
}

fn parse_optional_path(entity: &LdtkEntityInstance) -> Option<KinematicPathSpec> {
    let points = parse_points(&field_string(entity, "path_points").unwrap_or_default());
    if points.len() < 2 {
        return None;
    }
    Some(KinematicPathSpec {
        points,
        speed: field_f32(entity, "path_speed").unwrap_or(100.0),
        mode: parse_path_mode(&field_string(entity, "path_mode").unwrap_or_else(|| "PingPong".to_string())),
    })
}

fn parse_respawn(value: &str) -> Option<RespawnPolicySpec> {
    if let Some(seconds) = value.strip_prefix("AfterSeconds:").and_then(|text| text.parse::<f32>().ok()) {
        Some(RespawnPolicySpec::AfterSeconds(seconds))
    } else {
        match value {
            "Never" => Some(RespawnPolicySpec::Never),
            "OnRoomReload" => Some(RespawnPolicySpec::OnRoomReload),
            "Persistent" => Some(RespawnPolicySpec::Persistent),
            "None" | "" => None,
            _ => Some(RespawnPolicySpec::Never),
        }
    }
}

fn parse_pickup_kind(value: &str) -> PickupKindSpec {
    if let Some(amount) = value.strip_prefix("health:").and_then(|text| text.parse::<i32>().ok()) {
        PickupKindSpec::Health { amount }
    } else if let Some(amount) = value.strip_prefix("currency:").and_then(|text| text.parse::<i32>().ok()) {
        PickupKindSpec::Currency { amount }
    } else if let Some(ability_id) = value.strip_prefix("ability:") {
        PickupKindSpec::Ability { ability_id: ability_id.to_string() }
    } else if let Some(flag) = value.strip_prefix("flag:") {
        PickupKindSpec::StoryFlag { flag: flag.to_string() }
    } else {
        PickupKindSpec::Custom(value.to_string())
    }
}

fn parse_enemy_brain(value: &str) -> EnemyBrainSpec {
    if let Some(path_id) = value.strip_prefix("Patrol:") {
        EnemyBrainSpec::Patrol { path_id: Some(path_id.to_string()) }
    } else if let Some(radius) = value.strip_prefix("Guard:").and_then(|text| text.parse::<f32>().ok()) {
        EnemyBrainSpec::Guard { leash_radius: radius }
    } else {
        match value {
            "Passive" => EnemyBrainSpec::Passive,
            other => EnemyBrainSpec::Custom(other.to_string()),
        }
    }
}

fn parse_boss_brain(value: &str) -> BossBrainSpec {
    if let Some(script_id) = value.strip_prefix("PhaseScript:") {
        BossBrainSpec::PhaseScript { script_id: script_id.to_string() }
    } else {
        match value {
            "Dormant" => BossBrainSpec::Dormant,
            other => BossBrainSpec::Custom(other.to_string()),
        }
    }
}

fn parse_debug_label_kind(value: &str) -> DebugLabelKindSpec {
    match value {
        "Room" => DebugLabelKindSpec::Room,
        "LoadingZone" => DebugLabelKindSpec::LoadingZone,
        "Hazard" => DebugLabelKindSpec::Hazard,
        "Enemy" => DebugLabelKindSpec::Enemy,
        "Boss" => DebugLabelKindSpec::Boss,
        "Interactable" => DebugLabelKindSpec::Interactable,
        "Pickup" => DebugLabelKindSpec::Pickup,
        _ => DebugLabelKindSpec::Custom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_ldtk_validates() {
        let project = LdtkProject::load_embedded();
        let report = project.validate();
        assert!(report.errors.is_empty(), "{:#?}", report.errors);
    }

    #[test]
    fn embedded_ldtk_composes_central_hub_complex() {
        let project = LdtkProject::load_embedded();
        let manifest = project.to_room_manifest().expect("embedded LDtk should compose");
        assert_eq!(manifest.start_room, "central_hub_complex");
        assert!(manifest.rooms.len() > 1, "old sandbox rooms should be represented as LDtk active areas");
        assert!(manifest.links.iter().any(|link| link.from_room == "central_hub_complex" && link.from_zone == "boss_door" && link.to_room == "basement_boss"));
        let room = manifest.rooms.iter().find(|room| room.id == "central_hub_complex").expect("central hub active area exists");
        assert!(room.size[1] > 1000.0, "basement should extend below hub");
        assert!(!room.objects.iter().any(|object| matches!(object, RoomObjectSpec::BossSpawn { .. })), "boss belongs in the boss lab, not the stitched hub basement");
        let boss_room = manifest.rooms.iter().find(|room| room.id == "basement_boss").expect("boss lab room exists");
        assert!(boss_room.objects.iter().any(|object| matches!(object, RoomObjectSpec::BossSpawn { name, .. } if name.contains("clockwork warden"))));
    }
}
