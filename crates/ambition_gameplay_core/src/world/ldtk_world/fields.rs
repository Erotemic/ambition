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

use super::{LdtkEntityInstance, LdtkFieldInstance, LdtkLevel, AMBITION_LDTK_ENTITY_IDENTIFIERS};

pub(super) fn known_entity(identifier: &str) -> bool {
    AMBITION_LDTK_ENTITY_IDENTIFIERS.contains(&identifier)
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

pub(super) fn field_i32(entity: &LdtkEntityInstance, name: &str) -> Option<i32> {
    field_value(&entity.field_instances, name).and_then(|value| match value {
        Value::Number(number) => number.as_i64().map(|value| value as i32),
        Value::String(text) => text.parse::<i32>().ok(),
        _ => None,
    })
}

pub(super) fn field_bool(entity: &LdtkEntityInstance, name: &str) -> Option<bool> {
    field_value(&entity.field_instances, name).and_then(|value| match value {
        Value::Bool(value) => Some(*value),
        Value::String(text) => text.parse::<bool>().ok(),
        _ => None,
    })
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

pub(super) fn parse_path_mode(value: &str) -> ambition_characters::actor::KinematicPathMode {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "once" => ambition_characters::actor::KinematicPathMode::Once,
        "loop" => ambition_characters::actor::KinematicPathMode::Loop,
        _ => ambition_characters::actor::KinematicPathMode::PingPong,
    }
}

pub(super) fn parse_optional_path(
    entity: &LdtkEntityInstance,
) -> Option<ambition_characters::actor::KinematicPath> {
    let points = parse_points(&field_string(entity, "path_points").unwrap_or_default());
    if points.len() < 2 {
        return None;
    }
    Some(ambition_characters::actor::KinematicPath {
        points,
        speed: field_f32(entity, "path_speed").unwrap_or(100.0),
        mode: parse_path_mode(
            &field_string(entity, "path_mode").unwrap_or_else(|| "PingPong".to_string()),
        ),
        start_offset_seconds: 0.0,
    })
}

pub(super) fn parse_pickup_kind(value: &str) -> ambition_interaction::PickupKind {
    if let Some(amount) = value
        .strip_prefix("health:")
        .and_then(|text| text.parse::<i32>().ok())
    {
        ambition_interaction::PickupKind::Health { amount }
    } else if let Some(amount) = value
        .strip_prefix("currency:")
        .and_then(|text| text.parse::<i32>().ok())
    {
        ambition_interaction::PickupKind::Currency { amount }
    } else if let Some(ability_id) = value.strip_prefix("ability:") {
        ambition_interaction::PickupKind::Ability {
            ability_id: ability_id.to_string(),
        }
    } else if let Some(flag) = value.strip_prefix("flag:") {
        ambition_interaction::PickupKind::StoryFlag {
            flag: flag.to_string(),
        }
    } else {
        ambition_interaction::PickupKind::Custom(value.to_string())
    }
}

pub(super) fn parse_enemy_brain(value: &str) -> ambition_characters::actor::EnemyBrain {
    if let Some(path_id) = value.strip_prefix("Patrol:") {
        ambition_characters::actor::EnemyBrain::Patrol {
            path_id: Some(path_id.to_string()),
        }
    } else if let Some(radius) = value
        .strip_prefix("Guard:")
        .and_then(|text| text.parse::<f32>().ok())
    {
        ambition_characters::actor::EnemyBrain::Guard {
            leash_radius: radius,
        }
    } else {
        match value {
            "Passive" => ambition_characters::actor::EnemyBrain::Passive,
            other => ambition_characters::actor::EnemyBrain::Custom(other.to_string()),
        }
    }
}

pub(super) fn parse_boss_brain(value: &str) -> ambition_characters::actor::BossBrain {
    if let Some(script_id) = value.strip_prefix("PhaseScript:") {
        ambition_characters::actor::BossBrain::PhaseScript {
            script_id: script_id.to_string(),
        }
    } else {
        match value {
            "Dormant" => ambition_characters::actor::BossBrain::Dormant,
            other => ambition_characters::actor::BossBrain::Custom(other.to_string()),
        }
    }
}

pub(super) fn parse_debug_label_kind(value: &str) -> crate::debug_label::DebugLabelKind {
    match value {
        "Room" => crate::debug_label::DebugLabelKind::Room,
        "LoadingZone" => crate::debug_label::DebugLabelKind::LoadingZone,
        "Hazard" => crate::debug_label::DebugLabelKind::Hazard,
        "Enemy" => crate::debug_label::DebugLabelKind::Enemy,
        "Boss" => crate::debug_label::DebugLabelKind::Boss,
        "Interactable" => crate::debug_label::DebugLabelKind::Interactable,
        "Pickup" => crate::debug_label::DebugLabelKind::Pickup,
        _ => crate::debug_label::DebugLabelKind::Custom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::{BossBrain, EnemyBrain, KinematicPathMode};
    use ambition_interaction::PickupKind;

    #[test]
    fn parse_points_reads_semicolon_pairs_and_skips_malformed() {
        let pts = parse_points("10,20; 30,40 ;bad;50,60");
        assert_eq!(
            pts,
            vec![
                ae::Vec2::new(10.0, 20.0),
                ae::Vec2::new(30.0, 40.0),
                ae::Vec2::new(50.0, 60.0),
            ]
        );
        assert!(parse_points("").is_empty());
    }

    #[test]
    fn parse_path_mode_is_case_and_dash_insensitive_with_pingpong_default() {
        assert!(matches!(parse_path_mode("Once"), KinematicPathMode::Once));
        assert!(matches!(parse_path_mode("LOOP"), KinematicPathMode::Loop));
        assert!(matches!(
            parse_path_mode("ping-pong"),
            KinematicPathMode::PingPong
        ));
        assert!(matches!(
            parse_path_mode("???"),
            KinematicPathMode::PingPong
        ));
    }

    #[test]
    fn parse_pickup_kind_dispatches_each_prefix() {
        assert_eq!(
            parse_pickup_kind("health:5"),
            PickupKind::Health { amount: 5 }
        );
        assert_eq!(
            parse_pickup_kind("currency:50"),
            PickupKind::Currency { amount: 50 }
        );
        assert_eq!(
            parse_pickup_kind("ability:dash"),
            PickupKind::Ability {
                ability_id: "dash".into()
            }
        );
        assert_eq!(
            parse_pickup_kind("flag:seen_alice"),
            PickupKind::StoryFlag {
                flag: "seen_alice".into()
            }
        );
        assert_eq!(
            parse_pickup_kind("mystery"),
            PickupKind::Custom("mystery".into())
        );
        // A malformed amount falls through to Custom rather than panicking.
        assert_eq!(
            parse_pickup_kind("health:notanumber"),
            PickupKind::Custom("health:notanumber".into())
        );
    }

    #[test]
    fn parse_enemy_brain_dispatches_prefixes_and_falls_back_to_custom() {
        assert!(matches!(
            parse_enemy_brain("Patrol:loop_a"),
            EnemyBrain::Patrol { path_id: Some(p) } if p == "loop_a"
        ));
        assert!(matches!(
            parse_enemy_brain("Guard:120"),
            EnemyBrain::Guard { leash_radius } if (leash_radius - 120.0).abs() < 1e-3
        ));
        assert!(matches!(parse_enemy_brain("Passive"), EnemyBrain::Passive));
        assert!(matches!(
            parse_enemy_brain("Goblin"),
            EnemyBrain::Custom(s) if s == "Goblin"
        ));
    }

    #[test]
    fn parse_boss_brain_dispatches_phasescript_and_falls_back_to_custom() {
        assert!(matches!(
            parse_boss_brain("PhaseScript:gnu_ton"),
            BossBrain::PhaseScript { script_id } if script_id == "gnu_ton"
        ));
        assert!(matches!(parse_boss_brain("Dormant"), BossBrain::Dormant));
        assert!(matches!(
            parse_boss_brain("Mystery"),
            BossBrain::Custom(s) if s == "Mystery"
        ));
    }
}
