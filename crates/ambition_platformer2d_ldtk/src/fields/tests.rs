use super::*;
use ambition_entity_catalog::placements::{BossBrain, CharacterBrain};
use ambition_platformer2d_core::KinematicPathMode;
use ambition_platformer2d_world::rooms::PickupKind;

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
    //  `Patrol:` is NOT a brain spelling any more — the patrol path is a
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

/// THE REACHABILITY RULE ASKS ABOUT A STEP, NOT ABOUT SOLIDITY.
///
///  two proxies preceded it. The first scanned entities named `Solid` while
/// these levels paint their floors into the Collision IntGrid, so it read an
/// empty set on every world. The second counted solid CELLS inside the zone and
/// flagged five of twenty-four exits — three of them correct authoring.
///
/// This is `central_hub_main`'s real shape: the opening is a hole in a wall
/// whose bottom row is still solid, so the ground inside is one cell higher than
/// the ground you walk in from.
#[test]
fn an_exit_whose_ground_is_higher_than_the_approach_reports_the_step() {
    // Floor along row 7; the two exit columns have their floor at row 6 instead,
    // one cell higher — a sill.
    let level = level_with_collision(
        8,
        8,
        &[
            (0, 7),
            (1, 7),
            (2, 7),
            (3, 7),
            (4, 7),
            (5, 7),
            (6, 7),
            (7, 7),
            (6, 6),
            (7, 6),
        ],
    );
    // A zone over cols 6..7, rows 4..7.
    assert_eq!(edge_exit_step_up_px(&level, (96, 64, 32, 64)), 16);
}

///  AND A ZONE STANDING ON THE ROOM'S OWN FLOOR REPORTS NOTHING — which is
/// the half the previous rule got WRONG.
///
/// `scroll_lab`, `square_arena` and `tiny_chamber` all have solid cells in their
/// zone's bottom row, and all three are fine: that row is the floor, running
/// unbroken across the level. A zone stopping one row above it could never be
/// touched by a body standing on it. Counting cells called these defects;
/// asking about the step does not.
#[test]
fn a_zone_standing_on_the_rooms_own_floor_is_not_a_step() {
    let level = level_with_collision(
        8,
        8,
        &[
            (0, 7),
            (1, 7),
            (2, 7),
            (3, 7),
            (4, 7),
            (5, 7),
            (6, 7),
            (7, 7),
        ],
    );
    // The same zone as above: its bottom row IS the floor, and there is no step.
    assert_eq!(edge_exit_step_up_px(&level, (96, 64, 32, 64)), 0);
    // A level that paints no collision blocks nobody.
    let empty = level_with_collision(8, 8, &[]);
    assert_eq!(edge_exit_step_up_px(&empty, (0, 0, 128, 128)), 0);
}

///  THE APPROACH COLUMN IS ON THE ROOM'S SIDE, NOT ALWAYS THE LEFT.
///
/// An `EdgeExit` touches a level edge, so the room is on whichever side is not the edge.
#[test]
fn a_left_edge_exit_is_measured_against_the_room_to_its_right() {
    // Floor along row 7 except under cols 0..1, which are one cell higher.
    let level = level_with_collision(
        8,
        8,
        &[
            (0, 7),
            (1, 7),
            (2, 7),
            (3, 7),
            (4, 7),
            (5, 7),
            (6, 7),
            (7, 7),
            (0, 6),
            (1, 6),
        ],
    );
    // Cols 0..1, rows 4..7 — a sill at the LEFT edge.
    assert_eq!(edge_exit_step_up_px(&level, (0, 64, 32, 64)), 16);
}
