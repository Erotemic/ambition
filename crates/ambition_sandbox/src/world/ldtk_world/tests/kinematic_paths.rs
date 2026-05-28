//! Synthetic-LDtk tests for moving platforms, camera zones, and
//! kinematic-path resolution across enemies, NPCs, and hazards.

use ambition_engine as ae;
use serde_json::Value;

use super::super::project::*;

/// Helper used by several of these synthetic projects.
fn area_field(name: &str, value: &str) -> LdtkFieldInstance {
    LdtkFieldInstance {
        identifier: name.into(),
        value: Value::String(value.into()),
        real_editor_values: vec![],
    }
}

#[test]
fn synthetic_area_collects_multiple_moving_platforms() {
    let platform_a = super::make_entity_at(
        "MovingPlatform",
        [128, 320],
        [96, 16],
        &[
            ("sweep_dx", Value::Number(serde_json::Number::from(160))),
            ("speed", Value::Number(serde_json::Number::from(80))),
        ],
    );
    let platform_b = super::make_entity_at(
        "MovingPlatform",
        [384, 256],
        [64, 16],
        &[
            ("sweep_dx", Value::Number(serde_json::Number::from(-96))),
            ("speed", Value::Number(serde_json::Number::from(70))),
        ],
    );
    let project = LdtkProject {
        json_version: "1.5.3".into(),
        levels: vec![LdtkLevel {
            iid: "level-iid".into(),
            identifier: "multi_platform_lab".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 640,
            px_hei: 480,
            field_instances: vec![area_field("activeArea", "multi_platform_lab")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 40,
                c_hei: 30,
                grid_size: 16,
                entity_instances: vec![
                    super::make_entity_at("PlayerStart", [32, 400], [16, 32], &[]),
                    platform_a,
                    platform_b,
                ],
                int_grid_csv: Vec::new(),
                grid_tiles: Vec::new(),
            }],
        }],
    };

    let room_set = project
        .to_room_set()
        .expect("synthetic LDtk should compose");
    let room = room_set
        .rooms
        .iter()
        .find(|room| room.id == "multi_platform_lab")
        .expect("room should exist");
    assert_eq!(
        room.moving_platforms.len(),
        2,
        "all authored MovingPlatform entities in an active area should reach runtime state"
    );
    assert_eq!(room.moving_platforms[0].size, ae::Vec2::new(96.0, 16.0));
    assert_eq!(room.moving_platforms[1].direction(), -1.0);
}

#[test]
fn synthetic_camera_zone_reaches_room_spec() {
    let camera_zone = super::make_entity_at(
        "CameraZone",
        [128, 96],
        [256, 160],
        &[
            ("id", Value::String("intro_reveal".into())),
            ("priority", Value::Number(serde_json::Number::from(7))),
            (
                "zoom",
                Value::Number(serde_json::Number::from_f64(1.35).unwrap()),
            ),
            (
                "target_offset_x",
                Value::Number(serde_json::Number::from(24)),
            ),
            (
                "target_offset_y",
                Value::Number(serde_json::Number::from(-32)),
            ),
            ("easing_hz", Value::Number(serde_json::Number::from(5))),
            ("cinematic_lock", Value::Bool(true)),
            ("clamp_mode", Value::String("zone_bounds".into())),
        ],
    );
    let project = LdtkProject {
        json_version: "1.5.3".into(),
        levels: vec![LdtkLevel {
            iid: "level-iid".into(),
            identifier: "camera_lab".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 640,
            px_hei: 480,
            field_instances: vec![area_field("activeArea", "camera_lab")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 40,
                c_hei: 30,
                grid_size: 16,
                entity_instances: vec![
                    super::make_entity_at("PlayerStart", [32, 400], [16, 32], &[]),
                    camera_zone,
                ],
                int_grid_csv: Vec::new(),
                grid_tiles: Vec::new(),
            }],
        }],
    };

    let room_set = project
        .to_room_set()
        .expect("synthetic LDtk should compose");
    let room = room_set
        .rooms
        .iter()
        .find(|room| room.id == "camera_lab")
        .expect("room should exist");
    assert_eq!(room.camera_zones.len(), 1);
    let zone = &room.camera_zones[0];
    assert_eq!(zone.id, "intro_reveal");
    assert_eq!(zone.priority, 7);
    assert_eq!(zone.zoom, Some(1.35));
    assert_eq!(zone.target_offset, ae::Vec2::new(24.0, -32.0));
    assert_eq!(zone.easing_hz, Some(5.0));
    assert!(zone.cinematic_lock);
    assert_eq!(zone.clamp_mode, crate::rooms::CameraClampMode::ZoneBounds);
}

#[test]
fn synthetic_kinematic_path_reaches_room_spec_with_area_offset() {
    let path = super::make_entity_at(
        "KinematicPath",
        [16, 16],
        [128, 12],
        &[
            ("name", Value::String("patrol_alpha".into())),
            ("points", Value::String("24,40;96,40".into())),
            ("speed", Value::Number(serde_json::Number::from(90))),
            ("mode", Value::String("Loop".into())),
        ],
    );
    let project = LdtkProject {
        json_version: "1.5.3".into(),
        levels: vec![
            LdtkLevel {
                iid: "left-level".into(),
                identifier: "path_lab_left".into(),
                world_x: 1000,
                world_y: 0,
                px_wid: 320,
                px_hei: 240,
                field_instances: vec![area_field("activeArea", "path_lab")],
                layer_instances: vec![LdtkLayerInstance {
                    identifier: "Ambition".into(),
                    layer_type: "Entities".into(),
                    c_wid: 20,
                    c_hei: 15,
                    grid_size: 16,
                    entity_instances: vec![super::make_entity_at(
                        "PlayerStart",
                        [32, 160],
                        [16, 32],
                        &[],
                    )],
                    int_grid_csv: Vec::new(),
                    grid_tiles: Vec::new(),
                }],
            },
            LdtkLevel {
                iid: "right-level".into(),
                identifier: "path_lab_right".into(),
                world_x: 1320,
                world_y: 0,
                px_wid: 320,
                px_hei: 240,
                field_instances: vec![area_field("activeArea", "path_lab")],
                layer_instances: vec![LdtkLayerInstance {
                    identifier: "Ambition".into(),
                    layer_type: "Entities".into(),
                    c_wid: 20,
                    c_hei: 15,
                    grid_size: 16,
                    entity_instances: vec![path],
                    int_grid_csv: Vec::new(),
                    grid_tiles: Vec::new(),
                }],
            },
        ],
    };

    let room_set = project
        .to_room_set()
        .expect("synthetic LDtk should compose");
    let room = room_set
        .rooms
        .iter()
        .find(|room| room.id == "path_lab")
        .expect("path_lab active area exists");
    assert_eq!(room.kinematic_paths.len(), 1);
    let spec = &room.kinematic_paths[0];
    assert_eq!(spec.id, "patrol_alpha");
    assert_eq!(spec.name, "patrol_alpha");
    assert_eq!(spec.path.mode, ae::KinematicPathMode::Loop);
    assert_eq!(spec.path.speed, 90.0);
    assert_eq!(
        spec.path.points,
        vec![ae::Vec2::new(344.0, 40.0), ae::Vec2::new(416.0, 40.0)],
        "LDtk path points are level-local and must be converted into active-area-local coordinates"
    );
    // KinematicPath now lives only in `room.kinematic_paths`. The
    // engine no longer carries a `RoomObject` mirror.
    assert!(room
        .kinematic_paths
        .iter()
        .any(|p| p.path.points == spec.path.points));
}

#[test]
fn enemy_spawn_uses_room_spec_kinematic_path_aliases() {
    let path = super::make_entity_at(
        "KinematicPath",
        [64, 160],
        [128, 12],
        &[
            ("name", Value::String("patrol_alpha".into())),
            ("points", Value::String("64,160;192,160".into())),
            ("speed", Value::Number(serde_json::Number::from(90))),
            ("mode", Value::String("PingPong".into())),
        ],
    );
    let enemy = super::make_entity_at(
        "EnemySpawn",
        [80, 128],
        [44, 58],
        &[
            ("name", Value::String("path follower".into())),
            ("brain", Value::String("Patrol:patrol_alpha".into())),
        ],
    );
    let project = LdtkProject {
        json_version: "1.5.3".into(),
        levels: vec![LdtkLevel {
            iid: "level-iid".into(),
            identifier: "feature_path_lab".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 320,
            px_hei: 240,
            field_instances: vec![area_field("activeArea", "feature_path_lab")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 20,
                c_hei: 15,
                grid_size: 16,
                entity_instances: vec![
                    super::make_entity_at("PlayerStart", [32, 160], [16, 32], &[]),
                    path,
                    enemy,
                ],
                int_grid_csv: Vec::new(),
                grid_tiles: Vec::new(),
            }],
        }],
    };

    let room_set = project
        .to_room_set()
        .expect("synthetic LDtk should compose");
    let room = room_set
        .rooms
        .iter()
        .find(|room| room.id == "feature_path_lab")
        .expect("room should exist");
    let path_id = room
        .enemy_spawns
        .iter()
        .find_map(|authored| match &authored.payload {
            ae::EnemyBrain::Patrol {
                path_id: Some(path_id),
            } => Some(path_id.as_str()),
            _ => None,
        })
        .expect("EnemySpawn Patrol:<path id> should survive LDtk lowering");
    assert_eq!(path_id, "patrol_alpha");
    assert!(
        room.kinematic_paths
            .iter()
            .any(|spec| spec.matches_id(path_id)),
        "ECS actor spawn should resolve EnemySpawn Patrol:<path id> through RoomSpec::kinematic_paths"
    );
}

#[test]
fn moving_platform_path_id_resolves_through_kinematic_path_index() {
    let path = ae::KinematicPath {
        points: vec![ae::Vec2::new(32.0, 64.0), ae::Vec2::new(160.0, 64.0)],
        speed: 64.0,
        mode: ae::KinematicPathMode::PingPong,
        start_offset_seconds: 0.0,
    };
    let path_spec = crate::rooms::KinematicPathSpec::new(
        "intro_lift_path",
        "Intro Lift",
        ae::Aabb::new(ae::Vec2::new(0.0, 0.0), ae::Vec2::splat(8.0)),
        path,
    );

    let mut platform = crate::world::platforms::MovingPlatformSpec::from_authored(
        "lift",
        "lift platform",
        ae::Vec2::new(348.0, 128.0),
        ae::Vec2::new(96.0, 16.0),
        999.0,
        1.0,
        Some("intro_lift_path".into()),
    )
    .resolve(&[path_spec])
    .expect("path id resolves through the active room kinematic path index");

    assert_eq!(
        platform.pos,
        ae::Vec2::new(32.0, 64.0),
        "path-driven MovingPlatform starts on the first KinematicPath point, not its fallback sweep AABB"
    );
    let delta = platform.update(0.5);
    assert_eq!(delta, ae::Vec2::new(32.0, 0.0));
}

#[test]
fn npc_path_id_resolves_through_room_spec_kinematic_paths() {
    let path = super::make_entity_at(
        "KinematicPath",
        [0, 0],
        [16, 16],
        &[
            ("id", Value::String("guide_patrol".into())),
            ("name", Value::String("Guide Patrol".into())),
            ("points", Value::String("80,160;160,160".into())),
            ("speed", Value::Number(serde_json::Number::from(80))),
        ],
    );
    let npc = super::make_entity_at(
        "NpcSpawn",
        [80, 160],
        [28, 44],
        &[
            ("name", Value::String("Guide".into())),
            ("prompt", Value::String("Talk".into())),
            ("dialogue_id", Value::String("guide".into())),
            ("path_id", Value::String("guide_patrol".into())),
        ],
    );
    let project = LdtkProject {
        json_version: "1.5.3".into(),
        levels: vec![LdtkLevel {
            iid: "level-iid".into(),
            identifier: "npc_path_lab".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 320,
            px_hei: 240,
            field_instances: vec![area_field("activeArea", "npc_path_lab")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 20,
                c_hei: 15,
                grid_size: 16,
                entity_instances: vec![
                    super::make_entity_at("PlayerStart", [32, 160], [16, 32], &[]),
                    path,
                    npc,
                ],
                int_grid_csv: Vec::new(),
                grid_tiles: Vec::new(),
            }],
        }],
    };

    let room_set = project
        .to_room_set()
        .expect("synthetic LDtk should compose");
    let room = room_set
        .rooms
        .iter()
        .find(|room| room.id == "npc_path_lab")
        .expect("room should exist");
    let path_id = room
        .interactables
        .iter()
        .find_map(|authored| match &authored.payload.kind {
            crate::interaction::InteractionKind::Npc {
                patrol_path_id: Some(path_id),
                ..
            } => Some(path_id.as_str()),
            _ => None,
        })
        .expect("NpcSpawn path_id should survive LDtk lowering");
    assert_eq!(path_id, "guide_patrol");
    assert!(
        room.kinematic_paths
            .iter()
            .any(|spec| spec.matches_id(path_id)),
        "ECS actor spawn should resolve NpcSpawn path_id through RoomSpec::kinematic_paths"
    );
}

#[test]
fn damage_volume_path_id_resolves_through_room_spec_kinematic_paths() {
    let path = super::make_entity_at(
        "KinematicPath",
        [0, 0],
        [16, 16],
        &[
            ("id", Value::String("saw_patrol".into())),
            ("name", Value::String("Saw Patrol".into())),
            ("points", Value::String("96,128;192,128".into())),
            ("speed", Value::Number(serde_json::Number::from(96))),
        ],
    );
    let hazard = super::make_entity_at(
        "DamageVolume",
        [24, 64],
        [32, 32],
        &[
            ("name", Value::String("path saw".into())),
            ("damage", Value::Number(serde_json::Number::from(1))),
            ("path_id", Value::String("saw_patrol".into())),
        ],
    );
    let project = LdtkProject {
        json_version: "1.5.3".into(),
        levels: vec![LdtkLevel {
            iid: "level-iid".into(),
            identifier: "hazard_path_lab".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 320,
            px_hei: 240,
            field_instances: vec![area_field("activeArea", "hazard_path_lab")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 20,
                c_hei: 15,
                grid_size: 16,
                entity_instances: vec![
                    super::make_entity_at("PlayerStart", [32, 160], [16, 32], &[]),
                    path,
                    hazard,
                ],
                int_grid_csv: Vec::new(),
                grid_tiles: Vec::new(),
            }],
        }],
    };

    let room_set = project
        .to_room_set()
        .expect("synthetic LDtk should compose");
    let room = room_set
        .rooms
        .iter()
        .find(|room| room.id == "hazard_path_lab")
        .expect("room should exist");
    let room = room.clone();
    let mut app = bevy::prelude::App::new();
    app.add_systems(
        bevy::prelude::Update,
        move |mut commands: bevy::prelude::Commands| {
            crate::features::spawn_room_feature_entities(&mut commands, &room);
        },
    );
    app.update();
    let world = app.world_mut();
    let mut query = world.query::<&crate::features::HazardFeature>();
    let mut hazards: Vec<crate::features::HazardFeature> = query.iter(world).cloned().collect();
    assert_eq!(hazards.len(), 1);
    assert!(hazards[0].hazard.motion.is_some());
    assert_eq!(hazards[0].hazard.pos, ae::Vec2::new(96.0, 128.0));
    hazards[0].hazard.update(0.5);
    assert_eq!(hazards[0].hazard.pos, ae::Vec2::new(144.0, 128.0));
}
