//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_engine_core::KinematicPathMode;
use ambition_entity_catalog::placements::{BossBrain, CharacterBrain};
use ambition_world::rooms::PickupKindSpec as PickupKind;

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

fn entity_with_field(name: &str, value: Value) -> LdtkEntityInstance {
    LdtkEntityInstance {
        iid: "self".into(),
        identifier: "EnemySpawn".into(),
        pivot: vec![0.5, 1.0],
        px: [0, 0],
        width: 16,
        height: 16,
        field_instances: vec![LdtkFieldInstance {
            identifier: name.into(),
            value,
            real_editor_values: Vec::new(),
        }],
    }
}

#[test]
fn field_entity_ref_reads_entity_iid_from_object_or_bare_string() {
    // Canonical LDtk EntityRef shape: an object carrying entityIid.
    let obj = entity_with_field(
        "mounted_on",
        serde_json::json!({
            "entityIid": "mount-abc",
            "layerIid": "layer-1",
            "levelIid": "level-1",
            "worldIid": "world-1",
        }),
    );
    assert_eq!(
        field_entity_ref(&obj, "mounted_on"),
        Some("mount-abc".to_string()),
    );
    // A flattened bare-iid string is also accepted.
    let bare = entity_with_field("mounted_on", Value::String("mount-xyz".into()));
    assert_eq!(
        field_entity_ref(&bare, "mounted_on"),
        Some("mount-xyz".to_string()),
    );
    // An unset (null) ref, an empty string, and a missing field are None.
    let null = entity_with_field("mounted_on", Value::Null);
    assert_eq!(field_entity_ref(&null, "mounted_on"), None);
    let empty = entity_with_field("mounted_on", Value::String(String::new()));
    assert_eq!(field_entity_ref(&empty, "mounted_on"), None);
    assert_eq!(field_entity_ref(&null, "not_a_field"), None);
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
        CharacterBrain::Patrol { path_id: Some(p) } if p == "loop_a"
    ));
    assert!(matches!(
        parse_enemy_brain("Guard:120"),
        CharacterBrain::Guard { leash_radius } if (leash_radius - 120.0).abs() < 1e-3
    ));
    assert!(matches!(
        parse_enemy_brain("Passive"),
        CharacterBrain::Passive
    ));
    assert!(matches!(
        parse_enemy_brain("Goblin"),
        CharacterBrain::Custom(s) if s == "Goblin"
    ));
}

#[test]
fn parse_boss_brain_dispatches_phasescript_and_falls_back_to_custom() {
    assert!(matches!(
        parse_boss_brain("PhaseScript:gnu_ton_rider"),
        BossBrain::PhaseScript { script_id } if script_id == "gnu_ton_rider"
    ));
    assert!(matches!(parse_boss_brain("Dormant"), BossBrain::Dormant));
    assert!(matches!(
        parse_boss_brain("Mystery"),
        BossBrain::Custom(s) if s == "Mystery"
    ));
}
