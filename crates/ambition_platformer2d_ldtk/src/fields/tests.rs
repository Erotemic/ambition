//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_platformer2d_core::KinematicPathMode;
use ambition_entity_catalog::placements::{BossBrain, CharacterBrain};
use ambition_platformer2d_world::rooms::PickupKind as PickupKind;

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
    // ⛔ **`Patrol:` is NOT a brain spelling any more** — the patrol path is a
    // native `EnemySpawn.path_ref` EntityRef, and `convert_enemy_spawn` refuses
    // the retired prefix out loud. Nothing here may quietly parse it: a second
    // road that still understands the string is the string surviving.
    assert!(matches!(
        parse_enemy_brain("Patrol:loop_a"),
        CharacterBrain::Custom(s) if s == "Patrol:loop_a"
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

/// A level carrying one Collision IntGrid layer with `solid` painted at the
/// given (col, row) cells, on a `w`x`h` grid of 16px cells.
fn level_with_collision(w: i32, h: i32, solid: &[(i32, i32)]) -> LdtkLevel {
    let mut csv = vec![0; (w * h) as usize];
    for (c, r) in solid {
        csv[(r * w + c) as usize] = 1;
    }
    let json = serde_json::json!({
        "identifier": "probe",
        "iid": "probe-iid",
        "worldX": 0,
        "worldY": 0,
        "pxWid": w * 16,
        "pxHei": h * 16,
        "fieldInstances": [],
        "layerInstances": [{
            "__identifier": "Collision",
            "__type": "IntGrid",
            "__cWid": w,
            "__cHei": h,
            "__gridSize": 16,
            "intGridCsv": csv,
            "entityInstances": [],
        }],
    });
    serde_json::from_value(json).expect("the probe level parses")
}

/// **THE REACHABILITY RULE CAN SEE THE COLLISION GRID.**
///
/// ⛔⛔ its sibling — `EdgeExit ... overlaps solid X` — scans entities named
/// `Solid`, and these levels paint their floors into the Collision IntGrid, so
/// that rule could not fire on the case it was written for. This is the same
/// question asked of the geometry a body actually collides with.
#[test]
fn a_solid_collision_cell_inside_a_rect_is_counted() {
    // The shape measured in `central_hub_main`: an empty column with a floor
    // lip in its bottom row only.
    let level = level_with_collision(8, 8, &[(6, 7), (7, 7)]);
    // A rect over cols 6..7, rows 4..7 — the lip is inside it.
    assert_eq!(solid_cells_in_rect(&level, (96, 64, 32, 64)), 2);
}

/// **⛔ AND IT REPORTS ZERO FOR A CLEAR ONE, which is the half that makes the
/// count mean something.** A rule that answered "blocked" everywhere would pass
/// the test above and be useless.
#[test]
fn a_rect_clear_of_collision_counts_nothing() {
    let level = level_with_collision(8, 8, &[(6, 7), (7, 7)]);
    // Same columns, but stopping ABOVE the lip's row.
    assert_eq!(solid_cells_in_rect(&level, (96, 64, 32, 48)), 0);
    // And a level that paints no collision at all blocks nobody.
    let empty = level_with_collision(8, 8, &[]);
    assert_eq!(solid_cells_in_rect(&empty, (0, 0, 128, 128)), 0);
}

/// **⚠ HALF-OPEN IN PIXELS**: a rect ending exactly on a cell boundary must not
/// claim the next cell, or every zone would report its neighbour's floor.
#[test]
fn a_rect_ending_on_a_cell_boundary_does_not_claim_the_next_cell() {
    let level = level_with_collision(8, 8, &[(4, 0)]);
    // Cols 0..3 only: x 0..64 is four whole cells, and col 4 starts at 64.
    assert_eq!(solid_cells_in_rect(&level, (0, 0, 64, 16)), 0);
    // One pixel further and col 4 is in.
    assert_eq!(solid_cells_in_rect(&level, (0, 0, 65, 16)), 1);
}
