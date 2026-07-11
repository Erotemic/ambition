//! LDtk field accessors + value parsers for entity instances.
//!
//! Typed getters off an `LdtkEntityInstance` (`field_string`/`field_f32`/
//! `field_i32`/`field_bool` — first two re-exported `pub` for `crate::encounter`),
//! entity geometry helpers (`entity_rect`, `entity_touches_level_edge`,
//! `pivot_is_top_left`), and string→enum parsers (`parse_points`,
//! `parse_path_mode`, `parse_pickup_kind`, `parse_enemy_brain`/`parse_boss_brain`,
//! `parse_debug_label_kind`). Consumed by sibling `conversion`/`surfaces`.

use serde_json::Value;

use ambition_engine_core as ae;

use super::{LdtkEntityInstance, LdtkFieldInstance, LdtkLevel};

/// True if the identifier has a registered converter — the engine's standard
/// vocabulary plus any content-installed converters (ADR 0009), so a
/// game-registered entity passes validation like a built-in one.
pub(super) fn known_entity(identifier: &str) -> bool {
    super::conversion::converter_for(identifier).is_some()
}

pub(super) fn pivot_is_top_left(entity: &LdtkEntityInstance) -> bool {
    if entity.pivot.len() != 2 {
        return true;
    }
    entity.pivot[0].abs() <= 1.0e-6 && entity.pivot[1].abs() <= 1.0e-6
}

pub(super) fn entity_rect(entity: &LdtkEntityInstance) -> (i32, i32, i32, i32) {
    (entity.px[0], entity.px[1], entity.width, entity.height)
}

pub(super) fn rects_strict_intersect(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

pub(super) fn entity_touches_level_edge(entity: &LdtkEntityInstance, level: &LdtkLevel) -> bool {
    entity.px[0] <= 0
        || entity.px[1] <= 0
        || entity.px[0] + entity.width >= level.px_wid
        || entity.px[1] + entity.height >= level.px_hei
}

pub(super) fn field_value<'a>(fields: &'a [LdtkFieldInstance], name: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find(|field| field.identifier == name)
        .map(|field| &field.value)
}

pub(super) fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

pub fn field_string(entity: &LdtkEntityInstance, name: &str) -> Option<String> {
    field_value(&entity.field_instances, name).and_then(value_to_string)
}

pub fn field_f32(entity: &LdtkEntityInstance, name: &str) -> Option<f32> {
    field_value(&entity.field_instances, name).and_then(|value| match value {
        Value::Number(number) => number.as_f64().map(|value| value as f32),
        Value::String(text) => text.parse::<f32>().ok(),
        _ => None,
    })
}

pub fn field_i32(entity: &LdtkEntityInstance, name: &str) -> Option<i32> {
    field_value(&entity.field_instances, name).and_then(|value| match value {
        Value::Number(number) => number.as_i64().map(|value| value as i32),
        Value::String(text) => text.parse::<i32>().ok(),
        _ => None,
    })
}

pub fn field_bool(entity: &LdtkEntityInstance, name: &str) -> Option<bool> {
    field_value(&entity.field_instances, name).and_then(|value| match value {
        Value::Bool(value) => Some(*value),
        Value::String(text) => text.parse::<bool>().ok(),
        _ => None,
    })
}

/// Read an LDtk **EntityRef** field, returning the referenced entity's
/// `iid`. LDtk stores an entity-reference field's `__value` as an object
/// `{ "entityIid": "...", "layerIid": "...", "levelIid": "...",
/// "worldIid": "..." }` (or `null` when unset). This returns the
/// `entityIid` so the loader can resolve the referenced entity after
/// both instances have spawned — the primitive behind ADR 0020's
/// two-linked-entities mount authoring (a rider's `mounted_on` ref).
pub fn field_entity_ref(entity: &LdtkEntityInstance, name: &str) -> Option<String> {
    match field_value(&entity.field_instances, name)? {
        // The canonical LDtk shape: an object carrying `entityIid`.
        Value::Object(map) => map.get("entityIid").and_then(value_to_string),
        // Some exporters flatten a ref to the bare iid string.
        Value::String(iid) if !iid.is_empty() => Some(iid.clone()),
        _ => None,
    }
}

pub(super) fn parse_points(value: &str) -> Vec<ae::Vec2> {
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

pub(super) fn parse_path_mode(value: &str) -> ambition_engine_core::KinematicPathMode {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "once" => ambition_engine_core::KinematicPathMode::Once,
        "loop" => ambition_engine_core::KinematicPathMode::Loop,
        _ => ambition_engine_core::KinematicPathMode::PingPong,
    }
}

pub(super) fn parse_optional_path(
    entity: &LdtkEntityInstance,
) -> Option<ambition_engine_core::KinematicPath> {
    let points = parse_points(&field_string(entity, "path_points").unwrap_or_default());
    if points.len() < 2 {
        return None;
    }
    Some(ambition_engine_core::KinematicPath {
        points,
        speed: field_f32(entity, "path_speed").unwrap_or(100.0),
        mode: parse_path_mode(
            &field_string(entity, "path_mode").unwrap_or_else(|| "PingPong".to_string()),
        ),
        start_offset_seconds: 0.0,
    })
}

pub(super) fn parse_pickup_kind(value: &str) -> ambition_world::rooms::PickupKindSpec {
    if let Some(amount) = value
        .strip_prefix("health:")
        .and_then(|text| text.parse::<i32>().ok())
    {
        ambition_world::rooms::PickupKindSpec::Health { amount }
    } else if let Some(amount) = value
        .strip_prefix("currency:")
        .and_then(|text| text.parse::<i32>().ok())
    {
        ambition_world::rooms::PickupKindSpec::Currency { amount }
    } else if let Some(ability_id) = value.strip_prefix("ability:") {
        ambition_world::rooms::PickupKindSpec::Ability {
            ability_id: ability_id.to_string(),
        }
    } else if let Some(flag) = value.strip_prefix("flag:") {
        ambition_world::rooms::PickupKindSpec::StoryFlag {
            flag: flag.to_string(),
        }
    } else {
        ambition_world::rooms::PickupKindSpec::Custom(value.to_string())
    }
}

pub(super) fn parse_enemy_brain(
    value: &str,
) -> ambition_entity_catalog::placements::CharacterBrain {
    if let Some(path_id) = value.strip_prefix("Patrol:") {
        ambition_entity_catalog::placements::CharacterBrain::Patrol {
            path_id: Some(path_id.to_string()),
        }
    } else if let Some(radius) = value
        .strip_prefix("Guard:")
        .and_then(|text| text.parse::<f32>().ok())
    {
        ambition_entity_catalog::placements::CharacterBrain::Guard {
            leash_radius: radius,
        }
    } else {
        match value {
            "Passive" => ambition_entity_catalog::placements::CharacterBrain::Passive,
            other => ambition_entity_catalog::placements::CharacterBrain::Custom(other.to_string()),
        }
    }
}

pub(super) fn parse_boss_brain(value: &str) -> ambition_entity_catalog::placements::BossBrain {
    if let Some(script_id) = value.strip_prefix("PhaseScript:") {
        ambition_entity_catalog::placements::BossBrain::PhaseScript {
            script_id: script_id.to_string(),
        }
    } else {
        match value {
            "Dormant" => ambition_entity_catalog::placements::BossBrain::Dormant,
            other => ambition_entity_catalog::placements::BossBrain::Custom(other.to_string()),
        }
    }
}

pub(super) fn parse_debug_label_kind(value: &str) -> ambition_world::debug_label::DebugLabelKind {
    match value {
        "Room" => ambition_world::debug_label::DebugLabelKind::Room,
        "LoadingZone" => ambition_world::debug_label::DebugLabelKind::LoadingZone,
        "Hazard" => ambition_world::debug_label::DebugLabelKind::Hazard,
        "Enemy" => ambition_world::debug_label::DebugLabelKind::Enemy,
        "Boss" => ambition_world::debug_label::DebugLabelKind::Boss,
        "Interactable" => ambition_world::debug_label::DebugLabelKind::Interactable,
        "Pickup" => ambition_world::debug_label::DebugLabelKind::Pickup,
        _ => ambition_world::debug_label::DebugLabelKind::Custom,
    }
}

#[cfg(test)]
mod tests;
