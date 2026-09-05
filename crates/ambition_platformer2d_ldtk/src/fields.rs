//! LDtk field accessors + value parsers for entity instances.
//!
//! Typed getters off an `LdtkEntityInstance` (`field_string`/`field_f32`/
//! `field_i32`/`field_bool` — first two re-exported `pub` for `crate::encounter`),
//! entity geometry helpers (`entity_rect`, `entity_touches_level_edge`,
//! `pivot_is_top_left`), and string→enum parsers (`parse_points`,
//! `parse_path_mode`, `parse_pickup_kind`, `parse_enemy_brain`/`parse_boss_brain`,
//! `parse_debug_label_kind`). Consumed by sibling `conversion`/`surfaces`.

use serde_json::Value;

use ambition_platformer2d_core as ae;

use super::{LdtkEntityInstance, LdtkFieldInstance, LdtkLevel};

/// True if the CALLER'S vocabulary has a converter for the identifier — the
/// engine's standard nouns plus whatever game extended them, so a
/// game-registered entity passes validation like a built-in one.
///
/// the vocabulary is a parameter because validation's answer DEPENDS on it:
/// `MaryOBlock` is a real entity to Mary-O and an unknown one to the sandbox,
/// and both answers are correct.
pub(super) fn known_entity(
    identifier: &str,
    vocabulary: &super::conversion::LdtkVocabulary,
) -> bool {
    vocabulary.converter_for(identifier).is_some()
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

/// Read an LDtk EntityRef field, returning the referenced entity's
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

pub(super) fn parse_path_mode(value: &str) -> ambition_platformer2d_core::KinematicPathMode {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "once" => ambition_platformer2d_core::KinematicPathMode::Once,
        "loop" => ambition_platformer2d_core::KinematicPathMode::Loop,
        _ => ambition_platformer2d_core::KinematicPathMode::PingPong,
    }
}

pub(super) fn parse_optional_path(
    entity: &LdtkEntityInstance,
) -> Option<ambition_platformer2d_core::KinematicPath> {
    let points = parse_points(&field_string(entity, "path_points").unwrap_or_default());
    if points.len() < 2 {
        return None;
    }
    Some(ambition_platformer2d_core::KinematicPath {
        points,
        speed: field_f32(entity, "path_speed").unwrap_or(100.0),
        mode: parse_path_mode(
            &field_string(entity, "path_mode").unwrap_or_else(|| "PingPong".to_string()),
        ),
        start_offset_seconds: 0.0,
    })
}

pub(super) fn parse_pickup_kind(value: &str) -> ambition_platformer2d_world::rooms::PickupKind {
    if let Some(amount) = value
        .strip_prefix("health:")
        .and_then(|text| text.parse::<i32>().ok())
    {
        ambition_platformer2d_world::rooms::PickupKind::Health { amount }
    } else if let Some(amount) = value
        .strip_prefix("currency:")
        .and_then(|text| text.parse::<i32>().ok())
    {
        ambition_platformer2d_world::rooms::PickupKind::Currency { amount }
    } else if let Some(ability_id) = value.strip_prefix("ability:") {
        ambition_platformer2d_world::rooms::PickupKind::Ability {
            ability_id: ability_id.to_string(),
        }
    } else if let Some(flag) = value.strip_prefix("flag:") {
        ambition_platformer2d_world::rooms::PickupKind::StoryFlag {
            flag: flag.to_string(),
        }
    } else {
        ambition_platformer2d_world::rooms::PickupKind::Custom(value.to_string())
    }
}

/// there is no `Patrol:` prefix here, and its absence is the point. A
/// patrol's path was authored as a reference hidden inside this string field —
/// nothing about the field's name or its `String` type said a reference was in
/// there, so no tool could see it, three resolvers grew private spellings of it,
/// and a mismatch degraded to "the enemy stands still". It is a native LDtk
/// `EntityRef` now (`EnemySpawn.path_ref`), read by
/// [`LdtkEntityCtx::kinematic_path_ref`](crate::LdtkEntityCtx::kinematic_path_ref).
pub(super) fn parse_enemy_brain(
    value: &str,
) -> ambition_entity_catalog::placements::CharacterBrain {
    if let Some(radius) = value
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

pub(super) fn parse_debug_label_kind(
    value: &str,
) -> ambition_platformer2d_world::debug_label::DebugLabelKind {
    match value {
        "Room" => ambition_platformer2d_world::debug_label::DebugLabelKind::Room,
        "LoadingZone" => ambition_platformer2d_world::debug_label::DebugLabelKind::LoadingZone,
        "Hazard" => ambition_platformer2d_world::debug_label::DebugLabelKind::Hazard,
        "Enemy" => ambition_platformer2d_world::debug_label::DebugLabelKind::Enemy,
        "Boss" => ambition_platformer2d_world::debug_label::DebugLabelKind::Boss,
        "Interactable" => ambition_platformer2d_world::debug_label::DebugLabelKind::Interactable,
        "Pickup" => ambition_platformer2d_world::debug_label::DebugLabelKind::Pickup,
        _ => ambition_platformer2d_world::debug_label::DebugLabelKind::Custom,
    }
}

#[cfg(test)]
mod tests;

/// Return how far the ground inside an exit is above the adjacent approach ground.
///
/// For each exit column, the first solid cell at or below the zone top is the
/// walking surface. The worst inside surface is compared with the adjacent
/// approach column. Returns `0` when collision data or a usable ground surface
/// is absent. This tests traversability rather than solid occupancy.
pub(super) fn edge_exit_step_up_px(level: &LdtkLevel, rect: (i32, i32, i32, i32)) -> i32 {
    let Some(layer) = level
        .layer_instances
        .iter()
        .find(|layer| layer.layer_type == "IntGrid" && layer.identifier == "Collision")
    else {
        return 0;
    };
    let grid = layer.grid_size.max(1);
    let (w, h) = (layer.c_wid, layer.c_hei);
    if w <= 0 || h <= 0 || layer.int_grid_csv.len() != (w as usize) * (h as usize) {
        return 0;
    }
    let (x, y, rw, rh) = rect;
    if rw <= 0 || rh <= 0 {
        return 0;
    }
    let c0 = (x.div_euclid(grid)).clamp(0, w - 1);
    let c1 = ((x + rw - 1).div_euclid(grid)).clamp(0, w - 1);
    let r0 = (y.div_euclid(grid)).clamp(0, h - 1);

    // The row a body standing in this column would rest on: the first solid
    // cell at or below the zone's top edge.
    let surface_row = |col: i32| -> Option<i32> {
        (r0..h).find(|row| layer.int_grid_csv[(row * w + col) as usize] != 0)
    };
    // The WORST surface across the zone's span — a body has to cross all of it.
    let Some(inside) = (c0..=c1).filter_map(surface_row).min() else {
        return 0;
    };
    // The column the body walks in FROM. An `EdgeExit` touches a level edge, so
    // the room is on whichever side is not the edge.
    let approach = if c0 == 0 { c1 + 1 } else { c0 - 1 };
    if approach < 0 || approach >= w {
        return 0;
    }
    let Some(outside) = surface_row(approach) else {
        return 0;
    };
    // Inside is higher when its surface row is SMALLER (rows count downward).
    ((outside - inside) * grid).max(0)
}

/// The id an authored `BossSpawn` placement is known by — **the durable
/// progress key**.
///
/// ⭐⭐ ONE DEFINITION, because two readers need it and they must not disagree:
/// `convert_boss_spawn` stamps it onto the placement, and the authored-integrity
/// guard resolves the same set to check that every `boss_cleared("…")` an
/// author typed names a placement that exists.
///
/// Jon's ruling on decision 57 (2026-09-05): *"Boss progress is keyed only by
/// stable authored encounter/placement IDs."* An authored `encounter_id` IS that
/// id; the LDtk iid is the fallback for a placement nobody has named yet.
///
/// ⛔⛔ THE FALLBACK IS A LEGACY ROAD, NOT A DESIGN. An iid is a name no author
/// can know, so a placement gated by dialogue MUST carry an `encounter_id` —
/// that is exactly how `boss_cleared("mockingbird")` came to ask the BEHAVIOUR
/// id of a save keyed `BossSpawn-4308` and could never be true.
pub fn boss_placement_id(entity: &LdtkEntityInstance) -> String {
    field_string(entity, "encounter_id")
        .map(|authored| authored.trim().to_string())
        .filter(|authored| !authored.is_empty())
        .unwrap_or_else(|| entity.iid.clone())
}
