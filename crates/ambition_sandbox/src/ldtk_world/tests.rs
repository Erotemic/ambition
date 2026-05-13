use serde_json::Value;

use super::fields::*;
use super::intgrid::*;
use super::project::*;
use super::surfaces::*;
use super::*;

fn make_entity(identifier: &str, size: [i32; 2], fields: &[(&str, Value)]) -> LdtkEntityInstance {
    make_entity_at(identifier, [0, 0], size, fields)
}

fn make_entity_at(
    identifier: &str,
    px: [i32; 2],
    size: [i32; 2],
    fields: &[(&str, Value)],
) -> LdtkEntityInstance {
    LdtkEntityInstance {
        iid: format!("{identifier}-test-{}-{}", px[0], px[1]),
        identifier: identifier.to_string(),
        pivot: vec![0.0, 0.0],
        px,
        width: size[0],
        height: size[1],
        field_instances: fields
            .iter()
            .map(|(name, value)| LdtkFieldInstance {
                identifier: name.to_string(),
                value: value.clone(),
                real_editor_values: vec![Value::Null],
            })
            .collect(),
    }
}

fn compile_identifier(
    identifier: &str,
    size: [i32; 2],
    fields: &[(&str, Value)],
) -> SurfaceCompiled {
    let entity = make_entity(identifier, size, fields);
    let spec = parse_surface_spec(
        &entity,
        ae::Vec2::ZERO,
        ae::Vec2::new(size[0] as f32, size[1] as f32),
        identifier.to_string(),
    )
    .expect("surface spec parses");
    compile_surface(&spec).expect("surface compiles")
}

#[test]
fn embedded_ldtk_validates() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let report = project.validate();
    assert!(report.errors.is_empty(), "{:#?}", report.errors);
}

/// Audit: no static-collision entity types should appear in the
/// embedded LDtk file. Solid / OneWayPlatform / BlinkWall /
/// HazardBlock all belong on the IntGrid Collision layer (per
/// `tools/ldtk_intgrid_migration.py`); leaving them as entities
/// is the bug the user noticed — entities don't tile, so a
/// "Solid floor" entity stretches its single texture over a
/// long footprint and smears.
///
/// `DamageVolume` deliberately stays as an entity because it
/// can carry motion paths and per-volume damage; the audit
/// excludes it.
///
/// If a future authoring patch reintroduces any of these
/// entity types, this test fails and the author has to either
/// re-run the migration or convert the spec to use IntGrid
/// rectangles directly.
#[test]
fn no_static_collision_entities_in_embedded_ldtk() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    const BANNED: &[&str] = &["Solid", "OneWayPlatform", "BlinkWall", "HazardBlock"];
    let mut violations: Vec<String> = Vec::new();
    for level in &project.levels {
        for layer in &level.layer_instances {
            for entity in &layer.entity_instances {
                if BANNED.contains(&entity.identifier.as_str()) {
                    violations.push(format!(
                        "{}::{} (iid={})",
                        level.identifier, entity.identifier, entity.iid
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "found {} static-collision entit{} that should be IntGrid tiles \
         (run `python tools/ldtk_intgrid_migration.py` to migrate): {:#?}",
        violations.len(),
        if violations.len() == 1 { "y" } else { "ies" },
        violations,
    );
}

/// IntGrid value 5 (Hazard) must round-trip through the
/// `int_grid_value_to_block` mapping into a `BlockKind::Hazard`
/// block. Pinning the conversion so a future renumbering can't
/// silently drop hazard cells from the runtime collision world.
#[test]
fn int_grid_hazard_value_maps_to_hazard_block() {
    let block = int_grid_value_to_block(5, ae::Vec2::ZERO, ae::Vec2::new(16.0, 16.0))
        .expect("value 5 must map to a block");
    assert!(matches!(block.kind, ae::BlockKind::Hazard));
    assert_eq!(block.name, "ldtk hazard");
}

#[test]
fn level_metadata_reads_optional_biome_fields() {
    // Build a synthetic level whose fieldInstances declare every
    // optional metadata + visual-profile field. The reader should pick
    // them up and produce a RoomMetadata with each Some(...).
    use serde_json::Value;
    fn field(name: &str, value: &str) -> LdtkFieldInstance {
        LdtkFieldInstance {
            identifier: name.into(),
            value: Value::String(value.into()),
            real_editor_values: vec![],
        }
    }
    let level = LdtkLevel {
        iid: "level-iid".into(),
        identifier: "metadata_level".into(),
        world_x: 0,
        world_y: 0,
        px_wid: 256,
        px_hei: 256,
        field_instances: vec![
            field("activeArea", "metadata_area"),
            field("biome", "cave"),
            field("music_track", "loop_a"),
            field("ambient_profile", "damp"),
            field("visual_theme", "blue"),
            field("visual_profile", "intro_wakeup_room"),
            field("parallax_theme", "basement"),
            field("palette", "warm_terminal"),
            field("lighting_hint", "low_key"),
            field("foreground_treatment", "dusty_edges"),
        ],
        layer_instances: Vec::new(),
    };
    let meta = level.level_metadata();
    assert_eq!(meta.biome.as_deref(), Some("cave"));
    assert_eq!(meta.music_track.as_deref(), Some("loop_a"));
    assert_eq!(meta.ambient_profile.as_deref(), Some("damp"));
    assert_eq!(meta.visual_theme.as_deref(), Some("blue"));
    assert_eq!(meta.visual_profile.id.as_deref(), Some("intro_wakeup_room"));
    assert_eq!(
        meta.visual_profile.parallax_theme.as_deref(),
        Some("basement")
    );
    assert_eq!(
        meta.visual_profile.palette.as_deref(),
        Some("warm_terminal")
    );
    assert_eq!(
        meta.visual_profile.lighting_hint.as_deref(),
        Some("low_key")
    );
    assert_eq!(
        meta.visual_profile.foreground_treatment.as_deref(),
        Some("dusty_edges")
    );
}

#[test]
fn level_metadata_skips_blank_strings() {
    use serde_json::Value;
    fn field(name: &str, value: &str) -> LdtkFieldInstance {
        LdtkFieldInstance {
            identifier: name.into(),
            value: Value::String(value.into()),
            real_editor_values: vec![],
        }
    }
    let level = LdtkLevel {
        iid: "level-iid".into(),
        identifier: "blank_level".into(),
        world_x: 0,
        world_y: 0,
        px_wid: 256,
        px_hei: 256,
        field_instances: vec![
            field("activeArea", "blank_area"),
            field("biome", "   "),
            field("music_track", ""),
        ],
        layer_instances: Vec::new(),
    };
    let meta = level.level_metadata();
    assert!(
        meta.biome.is_none(),
        "whitespace-only must be treated as None"
    );
    assert!(meta.music_track.is_none());
}

#[test]
fn room_metadata_merge_first_non_empty_wins() {
    use crate::rooms::RoomMetadata;
    let mut a = RoomMetadata {
        biome: Some("hub".into()),
        music_track: None,
        ambient_profile: None,
        visual_theme: None,
        visual_profile: Default::default(),
    };
    let b = RoomMetadata {
        biome: Some("basement".into()),
        music_track: Some("dark_loop".into()),
        ambient_profile: Some("bass".into()),
        visual_theme: None,
        visual_profile: Default::default(),
    };
    a.merge(b);
    assert_eq!(a.biome.as_deref(), Some("hub"), "first non-empty wins");
    assert_eq!(
        a.music_track.as_deref(),
        Some("dark_loop"),
        "later levels fill in missing fields"
    );
    assert_eq!(a.ambient_profile.as_deref(), Some("bass"));
    assert_eq!(a.visual_theme, None);
}

/// Pin the biome-metadata seam end-to-end: every gameplay active
/// area in the embedded LDtk should compose with a non-empty
/// `biome` so the runtime resource (`ActiveRoomMetadata`) and
/// the room-music plumbing have something to read. Regression
/// guard for the "RoomSpec::metadata is always default" failure
/// mode where the seam compiles but the LDtk side never set a
/// value.
#[test]
fn embedded_ldtk_active_areas_have_biome_metadata() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("embedded LDtk should compose");
    let mut missing: Vec<&str> = Vec::new();
    for room in &room_set.rooms {
        if room.metadata.biome.is_none() {
            missing.push(room.id.as_str());
        }
    }
    assert!(
        missing.is_empty(),
        "every embedded LDtk active area should declare a biome; missing: {missing:?}"
    );
}

/// `crawl_lab` and `morph_lab` are the basement-reachable
/// body-mode showcase rooms. This test pins that they exist, are
/// reachable from `central_hub_complex` (the basement is part of
/// that activeArea), and that the basement carries a reciprocal
/// LoadingZone with the expected `target_room` per the spec
/// applies in 2026-05-07.
#[test]
fn embedded_ldtk_includes_basement_reachable_body_mode_rooms() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("embedded LDtk should compose");
    let room_ids: Vec<&str> = room_set.rooms.iter().map(|r| r.id.as_str()).collect();
    for required in ["crawl_lab", "morph_lab", "ladder_lab"] {
        assert!(
            room_ids.contains(&required),
            "basement-reachable showcase room '{required}' should exist; have: {room_ids:?}"
        );
    }
    // Basement (part of central_hub_complex active area) must
    // carry a LoadingZone for each showcase room. Walk the levels
    // for the central_hub_complex active area and look for door ids.
    let mut found_doors: Vec<String> = Vec::new();
    for level in &project.levels {
        for fi in &level.field_instances {
            if fi.identifier == "activeArea"
                && fi
                    .value
                    .as_str()
                    .map(|s| s == "central_hub_complex")
                    .unwrap_or(false)
            {
                for layer in &level.layer_instances {
                    if layer.identifier != "Ambition" {
                        continue;
                    }
                    for ent in &layer.entity_instances {
                        if ent.identifier != "LoadingZone" {
                            continue;
                        }
                        for ifield in &ent.field_instances {
                            if ifield.identifier == "target_room" {
                                if let Some(s) = ifield.value.as_str() {
                                    found_doors.push(s.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for required_target in ["crawl_lab", "morph_lab", "ladder_lab"] {
        assert!(
            found_doors.iter().any(|d| d == required_target),
            "central_hub_complex should have a LoadingZone with target_room='{required_target}'; have: {found_doors:?}"
        );
    }
}

/// `ladder_lab` ships a Climbable IntGrid layer with at least
/// one Ladder cell run. This test pins that the room
/// authoring → `World::climbable_regions` pipeline actually
/// produces a region the runtime can query, end-to-end. A
/// regression that drops the Climbable layer parser or the
/// Climbable layer instance from sandbox.ldtk would fail this
/// test immediately.
#[test]
fn embedded_ldtk_ladder_lab_has_a_ladder_climbable_region() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("embedded LDtk should compose");
    let ladder_lab = room_set
        .rooms
        .iter()
        .find(|r| r.id == "ladder_lab")
        .expect("ladder_lab active area should exist after spec apply");
    let regions = &ladder_lab.world.climbable_regions;
    assert!(
        !regions.is_empty(),
        "ladder_lab should ship at least one ClimbableRegion (the floor-to-ceiling ladder column)"
    );
    let ladder = regions
        .iter()
        .find(|r| matches!(r.kind, ae::ClimbableKind::Ladder))
        .expect("ladder_lab's region should be of kind Ladder");
    // The ladder column spans floor (y=992) up to upper platform
    // bottom (y=160). Pin the height as a sanity check.
    let height = ladder.aabb.max.y - ladder.aabb.min.y;
    assert!(
        height > 600.0,
        "ladder column should be tall (>600 px); got {height}"
    );
}

/// `water_world` is the canonical "non-default music_track"
/// example in the embedded LDtk. The room metadata flowing
/// through to `RoomSpec::metadata.music_track` is what lets the
/// runtime `RoomMusicRequest` swap the track when the player
/// enters the area. `mob_lab` deliberately does NOT set a
/// `music_track` — its music swap is owned by the encounter
/// system and only fires when the encounter triggers, not when
/// the door opens.
#[test]
fn embedded_ldtk_water_world_carries_music_track() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("embedded LDtk should compose");
    let water = room_set
        .rooms
        .iter()
        .find(|r| r.id == "water_world")
        .expect("water_world active area exists");
    assert_eq!(water.metadata.biome.as_deref(), Some("water"));
    assert_eq!(
        water.metadata.music_track.as_deref(),
        Some("pulse_drift_voyage"),
        "water_world should declare its non-default music track via the LDtk level field"
    );
}

/// `music_track_warnings` returns a warning per (level, unknown_id)
/// pair. The embedded LDtk's only declared music track today is
/// `pulse_drift_voyage` on water_world; pinning the matrix here
/// catches both directions of typo (LDtk-side and audio-catalog-side).
#[test]
fn music_track_warnings_flag_unknown_ids() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    // No tracks valid → every level that declares a music_track
    // produces one warning.
    let no_tracks = project.music_track_warnings(std::iter::empty::<&str>());
    assert!(
        !no_tracks.is_empty(),
        "embedded LDtk should declare at least one music_track field"
    );
    for warning in &no_tracks {
        assert!(
            warning.contains("references unknown music_track"),
            "warning should explain the missing reference: {warning}"
        );
    }
    // Including the real track id silences its warning.
    let with_water = project.music_track_warnings(["pulse_drift_voyage"]);
    assert!(
        !with_water
            .iter()
            .any(|w| w.contains("'pulse_drift_voyage'")),
        "valid tracks should not warn; got: {with_water:?}"
    );
}

/// Pin the audio-catalog × LDtk cross-validation as green for the
/// embedded sandbox. The visible binary's `init_sandbox_resources`
/// runs the same check at startup; this test fails the build if a
/// future LDtk edit introduces an unknown music_track id.
#[test]
fn embedded_ldtk_music_tracks_match_audio_catalog() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let data = crate::data::SandboxDataSpec::load_embedded();
    let valid = data.audio.music_tracks.iter().map(|t| t.id.as_str());
    let warnings = project.music_track_warnings(valid);
    assert!(
        warnings.is_empty(),
        "embedded LDtk references music_track ids not present in the audio catalog: {warnings:?}"
    );
}

/// `mob_lab` must NOT declare a `music_track` so entering the
/// mob lab door does not pre-empt the encounter system's music
/// override. Encounter starts/clears own the swap, and the
/// hub default plays while the room is unarmed.
#[test]
fn embedded_ldtk_mob_lab_does_not_carry_music_track() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("embedded LDtk should compose");
    let mob = room_set
        .rooms
        .iter()
        .find(|r| r.id == "mob_lab")
        .expect("mob_lab active area exists");
    assert_eq!(mob.metadata.biome.as_deref(), Some("mob_arena"));
    assert_eq!(
        mob.metadata.music_track, None,
        "mob_lab must not carry a music_track — the encounter system owns the swap"
    );
}

#[test]
fn embedded_ldtk_composes_central_hub_complex() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("embedded LDtk should compose");
    assert!(
        room_set.rooms.len() > 1,
        "old sandbox rooms should be represented as LDtk active areas"
    );
    let room = room_set
        .rooms
        .iter()
        .find(|room| room.id == "central_hub_complex")
        .expect("central hub active area exists");
    assert!(
        room.world.size.y > 1000.0,
        "basement should extend below hub"
    );
    assert!(
        !room
            .world
            .objects
            .iter()
            .any(|object| matches!(&object.kind, ae::RoomObjectKind::BossSpawn(_))),
        "boss belongs in the boss lab, not the stitched hub basement"
    );
    let boss_room = room_set
        .rooms
        .iter()
        .find(|room| room.id == "basement_boss")
        .expect("boss lab room exists");
    assert!(boss_room.world.objects.iter().any(|object| matches!(
        &object.kind,
        ae::RoomObjectKind::BossSpawn(_)
    ) && object
        .name
        .contains("clockwork warden")));
}

#[test]
fn embedded_ldtk_central_hub_carries_authored_moving_platforms() {
    // Moving platforms are LDtk-authored gameplay objects. This test ensures
    // the central hub basement entity reaches the RoomSpec via the parser +
    // emission path so the runtime has no hidden procedural platform to fall
    // back to.
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("embedded LDtk should compose");
    let hub = room_set
        .rooms
        .iter()
        .find(|room| room.id == "central_hub_complex")
        .expect("central hub active area exists");
    assert!(
        !hub.moving_platforms.is_empty(),
        "central_hub_basement should author at least one MovingPlatform entity"
    );
    let platform = hub.moving_platforms[0];
    assert!(
        platform.size.x > 100.0 && platform.size.y > 0.0,
        "platform AABB authored from LDtk size, got {:?}",
        platform.size
    );
    // Authored sweep_dx is positive → platform starts at min_x and
    // travels right initially.
    assert_eq!(platform.direction(), 1.0);
}

#[test]
fn synthetic_area_collects_multiple_moving_platforms() {
    fn field(name: &str, value: &str) -> LdtkFieldInstance {
        LdtkFieldInstance {
            identifier: name.into(),
            value: Value::String(value.into()),
            real_editor_values: vec![],
        }
    }

    let platform_a = make_entity_at(
        "MovingPlatform",
        [128, 320],
        [96, 16],
        &[
            ("sweep_dx", Value::Number(serde_json::Number::from(160))),
            ("speed", Value::Number(serde_json::Number::from(80))),
        ],
    );
    let platform_b = make_entity_at(
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
            field_instances: vec![field("activeArea", "multi_platform_lab")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 40,
                c_hei: 30,
                grid_size: 16,
                entity_instances: vec![
                    make_entity_at("PlayerStart", [32, 400], [16, 32], &[]),
                    platform_a,
                    platform_b,
                ],
                int_grid_csv: Vec::new(),
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
    fn field(name: &str, value: &str) -> LdtkFieldInstance {
        LdtkFieldInstance {
            identifier: name.into(),
            value: Value::String(value.into()),
            real_editor_values: vec![],
        }
    }

    let camera_zone = make_entity_at(
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
            field_instances: vec![field("activeArea", "camera_lab")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 40,
                c_hei: 30,
                grid_size: 16,
                entity_instances: vec![
                    make_entity_at("PlayerStart", [32, 400], [16, 32], &[]),
                    camera_zone,
                ],
                int_grid_csv: Vec::new(),
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
    fn field(name: &str, value: &str) -> LdtkFieldInstance {
        LdtkFieldInstance {
            identifier: name.into(),
            value: Value::String(value.into()),
            real_editor_values: vec![],
        }
    }

    let path = make_entity_at(
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
                field_instances: vec![field("activeArea", "path_lab")],
                layer_instances: vec![LdtkLayerInstance {
                    identifier: "Ambition".into(),
                    layer_type: "Entities".into(),
                    c_wid: 20,
                    c_hei: 15,
                    grid_size: 16,
                    entity_instances: vec![make_entity_at("PlayerStart", [32, 160], [16, 32], &[])],
                    int_grid_csv: Vec::new(),
                }],
            },
            LdtkLevel {
                iid: "right-level".into(),
                identifier: "path_lab_right".into(),
                world_x: 1320,
                world_y: 0,
                px_wid: 320,
                px_hei: 240,
                field_instances: vec![field("activeArea", "path_lab")],
                layer_instances: vec![LdtkLayerInstance {
                    identifier: "Ambition".into(),
                    layer_type: "Entities".into(),
                    c_wid: 20,
                    c_hei: 15,
                    grid_size: 16,
                    entity_instances: vec![path],
                    int_grid_csv: Vec::new(),
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
    assert!(room.world.objects.iter().any(|object| matches!(
        &object.kind,
        ae::RoomObjectKind::KinematicPath(path) if path.points == spec.path.points
    )));
}

#[test]
fn feature_runtime_uses_room_spec_kinematic_path_aliases() {
    fn field(name: &str, value: &str) -> LdtkFieldInstance {
        LdtkFieldInstance {
            identifier: name.into(),
            value: Value::String(value.into()),
            real_editor_values: vec![],
        }
    }

    let path = make_entity_at(
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
    let enemy = make_entity_at(
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
            field_instances: vec![field("activeArea", "feature_path_lab")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 20,
                c_hei: 15,
                grid_size: 16,
                entity_instances: vec![
                    make_entity_at("PlayerStart", [32, 160], [16, 32], &[]),
                    path,
                    enemy,
                ],
                int_grid_csv: Vec::new(),
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
    let features = crate::features::FeatureRuntime::from_room_spec(room);
    assert_eq!(features.enemies.len(), 1);
    assert!(
        features.enemies[0].motion.is_some(),
        "EnemySpawn Patrol:<path id> should resolve through RoomSpec::kinematic_paths"
    );
}

#[test]
fn embedded_ldtk_patrol_enemy_resolves_kinematic_path_index() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("embedded LDtk should compose");
    let room = room_set
        .rooms
        .iter()
        .find(|room| room.id == "basement_enemies")
        .expect("basement_enemies active area exists");
    assert!(
        !room.kinematic_paths.is_empty(),
        "basement_enemies should expose authored KinematicPath specs"
    );
    let features = crate::features::FeatureRuntime::from_room_spec(room);
    assert!(
        features.enemies.iter().any(|enemy| enemy.motion.is_some()),
        "at least one authored patrol enemy should resolve through RoomSpec::kinematic_paths"
    );
}

#[test]
fn central_hub_collision_layer_lowers_to_engine_blocks() {
    let project = LdtkProject::load_default().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("embedded LDtk should compose");
    let hub = room_set
        .rooms
        .iter()
        .find(|room| room.id == "central_hub_complex")
        .expect("central hub active area exists");
    let solid_blocks = hub
        .world
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::Solid))
        .count();
    let one_way_blocks = hub
        .world
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::OneWay))
        .count();
    let blink_blocks = hub
        .world
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, ae::BlockKind::BlinkWall { .. }))
        .count();
    // Step E migration painted Solid + OneWayPlatform + BlinkWall in
    // central_hub_main as IntGrid cells; the rect-merge collapses
    // adjacent same-value runs into single blocks. Each kind should
    // still produce at least one block, and the total stays well
    // below the unmerged 1004-cell count to confirm merging actually
    // ran.
    assert!(
        solid_blocks >= 1,
        "expected at least one solid IntGrid block in central hub; got {solid_blocks}"
    );
    assert!(
        one_way_blocks >= 1,
        "expected at least one OneWay IntGrid block in central hub; got {one_way_blocks}"
    );
    assert!(
        blink_blocks >= 1,
        "expected at least one BlinkWall IntGrid block in central hub; got {blink_blocks}"
    );
    let total = solid_blocks + one_way_blocks + blink_blocks;
    eprintln!(
        "central_hub_complex IntGrid blocks after merge: solid={solid_blocks} one_way={one_way_blocks} blink={blink_blocks} total={total}"
    );
    assert!(
        total < 200,
        "expected rect-merged collision count well below the 1004 unmerged cells; got {total}"
    );
}

#[test]
fn intgrid_rect_merge_collapses_a_horizontal_run() {
    // 5x1 row of value=1 cells should produce a single 5*16-wide block.
    let layer = LdtkLayerInstance {
        identifier: "Collision".to_string(),
        layer_type: "IntGrid".to_string(),
        c_wid: 5,
        c_hei: 1,
        grid_size: 16,
        entity_instances: Vec::new(),
        int_grid_csv: vec![1; 5],
    };
    let blocks =
        emit_collision_blocks_from_intgrid(&layer, ae::Vec2::ZERO).expect("merge succeeds");
    assert_eq!(blocks.len(), 1, "horizontal run should merge to one block");
    let block = &blocks[0];
    assert!(matches!(block.kind, ae::BlockKind::Solid));
    let size = ae::AabbExt::half_size(block.aabb) * 2.0;
    assert!(
        (size.x - 80.0).abs() < 0.001,
        "merged width = 5 cells * 16px"
    );
    assert!((size.y - 16.0).abs() < 0.001, "merged height = 1 cell");
}

#[test]
fn intgrid_rect_merge_does_not_collapse_columns_into_vertical_bars() {
    // A staircase pattern is the regression case: greedy vertical
    // merge previously collapsed each diagonal step into a tall
    // 1-wide bar, which rendered as vertical walls instead of the
    // staircase the editor shows. Horizontal-only merge keeps each
    // cell's row the way the artist painted it — so a 3-step
    // staircase produces 6 blocks (1 + 2 + 3 cells across), one per
    // run, not three vertical bars.
    let layer = LdtkLayerInstance {
        identifier: "Collision".to_string(),
        layer_type: "IntGrid".to_string(),
        c_wid: 3,
        c_hei: 3,
        grid_size: 16,
        entity_instances: Vec::new(),
        int_grid_csv: vec![
            0, 0, 1, // row 0
            0, 1, 1, // row 1
            1, 1, 1, // row 2
        ],
    };
    let blocks =
        emit_collision_blocks_from_intgrid(&layer, ae::Vec2::ZERO).expect("merge succeeds");
    assert_eq!(
        blocks.len(),
        3,
        "staircase should produce one block per row, not collapsed verticals"
    );
    let widths: Vec<i32> = blocks
        .iter()
        .map(|b| (ae::AabbExt::half_size(b.aabb).x * 2.0 / 16.0).round() as i32)
        .collect();
    assert_eq!(widths, vec![1, 2, 3]);
}

#[test]
fn intgrid_rect_merge_separates_distinct_values() {
    // Row [Solid, Solid, OneWay, Solid] should produce 3 blocks: a
    // 2-cell solid, a 1-cell one-way, and a 1-cell solid.
    let layer = LdtkLayerInstance {
        identifier: "Collision".to_string(),
        layer_type: "IntGrid".to_string(),
        c_wid: 4,
        c_hei: 1,
        grid_size: 16,
        entity_instances: Vec::new(),
        int_grid_csv: vec![
            INT_GRID_SOLID,
            INT_GRID_SOLID,
            INT_GRID_ONE_WAY,
            INT_GRID_SOLID,
        ],
    };
    let blocks =
        emit_collision_blocks_from_intgrid(&layer, ae::Vec2::ZERO).expect("merge succeeds");
    assert_eq!(blocks.len(), 3);
    assert!(matches!(blocks[0].kind, ae::BlockKind::Solid));
    assert!(matches!(blocks[1].kind, ae::BlockKind::OneWay));
    assert!(matches!(blocks[2].kind, ae::BlockKind::Solid));
}

#[test]
fn solid_is_a_promoted_runtime_role() {
    let role = LdtkRuntimeRole::from_identifier("Solid");
    assert_eq!(role, LdtkRuntimeRole::Solid);
    assert!(role.promoted(), "Solid is a Step 1 promoted runtime role");
    let summary = LdtkRuntimeSpineIndex::default().promoted_summary();
    assert!(
        summary.contains("solids"),
        "promoted summary surfaces solid count: {summary}"
    );
}

#[test]
fn solid_index_replaces_only_when_changed() {
    let mut index = LdtkRuntimeSolidIndex::default();
    let solid_a = LdtkRuntimeSolid {
        iid: "solid-a".to_string(),
        min: ae::Vec2::ZERO,
        size: ae::Vec2::new(64.0, 16.0),
    };
    let solid_b = LdtkRuntimeSolid {
        iid: "solid-b".to_string(),
        min: ae::Vec2::new(64.0, 0.0),
        size: ae::Vec2::new(64.0, 16.0),
    };
    index.replace_if_changed(LdtkRuntimeSolidIndex {
        active_area: "central_hub_complex".to_string(),
        solids: vec![solid_b.clone(), solid_a.clone()],
        revision: 0,
    });
    assert_eq!(index.count(), 2);
    assert_eq!(
        index.solids[0].iid, "solid-a",
        "solids are sorted by iid for stable diffs"
    );
    assert_eq!(index.revision, 1);

    let before = index.revision;
    index.replace_if_changed(LdtkRuntimeSolidIndex {
        active_area: "central_hub_complex".to_string(),
        solids: vec![solid_a, solid_b],
        revision: index.revision,
    });
    assert_eq!(
        index.revision, before,
        "no-op replace must not bump revision"
    );
}

#[test]
fn one_way_platform_compiles_to_one_way_block() {
    let compiled = compile_identifier("OneWayPlatform", [96, 16], &[]);
    assert_eq!(compiled.blocks.len(), 1);
    assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::OneWay));
}

#[test]
fn solid_compiles_to_solid_block() {
    let compiled = compile_identifier("Solid", [128, 32], &[]);
    assert_eq!(compiled.objects.len(), 0);
    assert_eq!(compiled.blocks.len(), 1);
    assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::Solid));
}

#[test]
fn hazard_block_compiles_to_hazard_block() {
    let compiled = compile_identifier("HazardBlock", [64, 16], &[]);
    assert_eq!(compiled.blocks.len(), 1);
    assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::Hazard));
}

#[test]
fn pogo_orb_compiles_to_pogo_orb_block() {
    let compiled = compile_identifier("PogoOrb", [32, 32], &[]);
    assert_eq!(compiled.blocks.len(), 1);
    assert!(matches!(compiled.blocks[0].kind, ae::BlockKind::PogoOrb));
}

#[test]
fn rebound_pad_compiles_to_rebound_block() {
    let compiled = compile_identifier(
        "ReboundPad",
        [32, 16],
        &[
            ("impulseX", Value::Number(serde_json::Number::from(0))),
            ("impulseY", Value::Number(serde_json::Number::from(-600))),
        ],
    );
    assert_eq!(compiled.blocks.len(), 1);
    assert!(matches!(
        compiled.blocks[0].kind,
        ae::BlockKind::Rebound { .. }
    ));
}

#[test]
fn blink_wall_uses_tier_field() {
    let soft = compile_identifier(
        "BlinkWall",
        [32, 32],
        &[("tier", Value::String("Soft".into()))],
    );
    let hard = compile_identifier(
        "BlinkWall",
        [32, 32],
        &[("tier", Value::String("Hard".into()))],
    );
    assert!(matches!(
        soft.blocks[0].kind,
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Soft
        }
    ));
    assert!(matches!(
        hard.blocks[0].kind,
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Hard
        }
    ));
}

#[test]
fn rebound_pad_requires_impulse_fields() {
    let entity = make_entity("ReboundPad", [16, 16], &[]);
    let err = parse_surface_spec(
        &entity,
        ae::Vec2::ZERO,
        ae::Vec2::new(16.0, 16.0),
        "rp".into(),
    )
    .expect_err("missing impulses");
    assert!(err.contains("missing impulseX"), "{err}");
}

/// `BreakablePlatform` with `collision=Solid` lowers to a Breakable
/// runtime object with hard collision while intact.
#[test]
fn breakable_platform_solid_compiles_with_solid_collision() {
    let compiled = compile_identifier(
        "BreakablePlatform",
        [48, 48],
        &[
            ("collision", Value::String("Solid".into())),
            ("trigger", Value::String("OnHit".into())),
            ("max_hp", Value::Number(serde_json::Number::from(2))),
        ],
    );
    assert!(compiled.blocks.is_empty());
    assert_eq!(compiled.objects.len(), 1);
    match &compiled.objects[0].kind {
        ae::RoomObjectKind::Breakable(breakable) => {
            assert_eq!(breakable.collision, ae::BreakableCollision::Solid);
            assert_eq!(breakable.trigger, ae::BreakableTrigger::OnHit);
            assert_eq!(breakable.health.max, 2);
            assert!(!breakable.pogo_refresh);
        }
        other => panic!("expected Breakable, got {other:?}"),
    }
}

/// `BreakablePlatform` with `collision=OneWayUp` lowers to a Breakable
/// runtime object that lands as a one-way platform.
#[test]
fn breakable_platform_one_way_up_compiles() {
    let compiled = compile_identifier(
        "BreakablePlatform",
        [80, 16],
        &[
            ("collision", Value::String("OneWayUp".into())),
            ("trigger", Value::String("OnStand".into())),
        ],
    );
    assert_eq!(compiled.objects.len(), 1);
    match &compiled.objects[0].kind {
        ae::RoomObjectKind::Breakable(breakable) => {
            assert_eq!(breakable.collision, ae::BreakableCollision::OneWayUp);
            assert_eq!(breakable.trigger, ae::BreakableTrigger::OnStand);
        }
        other => panic!("expected Breakable, got {other:?}"),
    }
}

/// `BreakablePlatform` rejects unknown collision values. The LDtk enum
/// has only Solid|OneWayUp, so the previous OnStand+None combo is
/// unrepresentable in the editor and we don't even need a degrade path.
#[test]
fn breakable_platform_rejects_unknown_collision() {
    let entity = make_entity(
        "BreakablePlatform",
        [32, 32],
        &[("collision", Value::String("None".into()))],
    );
    let err = parse_surface_spec(
        &entity,
        ae::Vec2::ZERO,
        ae::Vec2::new(32.0, 32.0),
        "p".into(),
    )
    .expect_err("None is not a valid BreakablePlatform collision");
    assert!(err.contains("BreakablePlatform"), "{err}");
}

/// Engine compile path stays strict: a hand-crafted incoherent combo
/// (BreakOnStand with collision=None) is still rejected, even though
/// the LDtk adapter can no longer produce one for BreakablePlatform.
#[test]
fn engine_compile_still_rejects_on_stand_without_collision() {
    let bad_spec = LdtkSurfaceSpec {
        iid: "test".into(),
        name: "test".into(),
        min: ae::Vec2::ZERO,
        size: ae::Vec2::new(32.0, 32.0),
        collision: SurfaceCollision::None,
        breakability: SurfaceBreakability::BreakOnStand,
        contact: SurfaceContact::None,
        respawn: SurfaceRespawn::Never,
        max_hp: 3,
    };
    let err = compile_surface(&bad_spec).expect_err("BreakOnStand requires collision");
    assert!(
        err.contains("BreakOnStand requires non-None collision"),
        "{err}"
    );
}

/// `respawn = AfterSeconds` requires a positive `respawn_seconds` field.
#[test]
fn breakable_platform_after_seconds_requires_positive_respawn_seconds() {
    let missing_field = make_entity(
        "BreakablePlatform",
        [32, 32],
        &[
            ("collision", Value::String("Solid".into())),
            ("trigger", Value::String("OnHit".into())),
            ("respawn", Value::String("AfterSeconds".into())),
        ],
    );
    let err = parse_surface_spec(
        &missing_field,
        ae::Vec2::ZERO,
        ae::Vec2::new(32.0, 32.0),
        "p".into(),
    )
    .expect_err("AfterSeconds without respawn_seconds is rejected");
    assert!(err.contains("respawn_seconds"), "{err}");

    let zero_seconds = make_entity(
        "BreakablePlatform",
        [32, 32],
        &[
            ("collision", Value::String("Solid".into())),
            ("trigger", Value::String("OnHit".into())),
            ("respawn", Value::String("AfterSeconds".into())),
            (
                "respawn_seconds",
                Value::Number(serde_json::Number::from(0)),
            ),
        ],
    );
    let err = parse_surface_spec(
        &zero_seconds,
        ae::Vec2::ZERO,
        ae::Vec2::new(32.0, 32.0),
        "p".into(),
    )
    .expect_err("respawn_seconds must be positive");
    assert!(err.contains("positive"), "{err}");
}

/// `BreakablePogoOrb` lowers to a Breakable with the `pogo_refresh`
/// flag set, so the gameplay loop emits a PogoOrb collision-world
/// block while intact and routes pogo bounces back as damage.
#[test]
fn breakable_pogo_orb_compiles_with_pogo_flag() {
    let compiled = compile_identifier(
        "BreakablePogoOrb",
        [36, 36],
        &[("max_hp", Value::Number(serde_json::Number::from(4)))],
    );
    assert!(compiled.blocks.is_empty());
    assert_eq!(compiled.objects.len(), 1);
    match &compiled.objects[0].kind {
        ae::RoomObjectKind::Breakable(breakable) => {
            assert!(breakable.pogo_refresh);
            assert_eq!(breakable.collision, ae::BreakableCollision::None);
            assert_eq!(breakable.trigger, ae::BreakableTrigger::OnHit);
            assert_eq!(breakable.health.max, 4);
        }
        other => panic!("expected Breakable, got {other:?}"),
    }
}

#[test]
fn no_surface_authoring_primitive_is_registered() {
    // The LDtk editor stays differentiated; there should be no canonical
    // generic Surface entity registered or routed through the parser.
    assert!(
        !known_entity("Surface"),
        "Surface must not be a registered LDtk entity"
    );
    assert!(
        !is_surface_like_identifier("Surface"),
        "Surface must not route through the typed surface conversion path"
    );
    // The legacy generic `Breakable` is gone; only the narrow types
    // remain.
    assert!(!known_entity("Breakable"), "legacy Breakable was removed");
    assert!(
        !is_surface_like_identifier("Breakable"),
        "legacy Breakable parser branch was removed"
    );
    // Differentiated identifiers DO still route through the typed
    // conversion path.
    for id in [
        "Solid",
        "OneWayPlatform",
        "BlinkWall",
        "HazardBlock",
        "PogoOrb",
        "ReboundPad",
        "BreakablePlatform",
        "BreakablePogoOrb",
    ] {
        assert!(is_surface_like_identifier(id), "{id}");
    }
    for id in ["PlayerStart", "LoadingZone", "DebugLabel", "NpcSpawn"] {
        assert!(!is_surface_like_identifier(id), "{id}");
    }
}

fn intgrid_layer(identifier: &str, c_wid: i32, c_hei: i32, csv: Vec<i32>) -> LdtkLayerInstance {
    LdtkLayerInstance {
        identifier: identifier.to_string(),
        layer_type: "IntGrid".to_string(),
        c_wid,
        c_hei,
        grid_size: GRID,
        entity_instances: Vec::new(),
        int_grid_csv: csv,
    }
}

#[test]
fn climbable_intgrid_emits_ladder_region_for_value_one() {
    // 4x3 layer, single column of ladder cells in the middle.
    // CSV is row-major: row0 row1 row2.
    let csv = vec![
        0, 0, 1, 0, // row 0
        0, 0, 1, 0, // row 1
        0, 0, 1, 0, // row 2
    ];
    let layer = intgrid_layer(CLIMBABLE_LAYER, 4, 3, csv);
    let regions = emit_climbable_regions_from_intgrid(&layer, ae::Vec2::ZERO).unwrap();
    assert_eq!(regions.len(), 1, "ladder column should merge to one region");
    assert_eq!(regions[0].kind, ae::ClimbableKind::Ladder);
    // Cell (cx=2, cy=0..2). With GRID=16, x in [32, 48], y in [0, 48].
    assert_eq!(regions[0].aabb.min.x, 32.0);
    assert_eq!(regions[0].aabb.min.y, 0.0);
    assert_eq!(regions[0].aabb.max.x, 48.0);
    assert_eq!(regions[0].aabb.max.y, 48.0);
}

#[test]
fn climbable_intgrid_distinguishes_ladder_vine_wall() {
    let layer = intgrid_layer(
        CLIMBABLE_LAYER,
        3,
        1,
        vec![
            CLIMBABLE_INT_GRID_LADDER,
            CLIMBABLE_INT_GRID_VINE,
            CLIMBABLE_INT_GRID_WALL,
        ],
    );
    let regions = emit_climbable_regions_from_intgrid(&layer, ae::Vec2::ZERO).unwrap();
    assert_eq!(regions.len(), 3);
    // Sort by min.x for deterministic comparison; merge_intgrid_rects
    // emits in row-major order, so regions[0] is leftmost.
    assert_eq!(regions[0].kind, ae::ClimbableKind::Ladder);
    assert_eq!(regions[1].kind, ae::ClimbableKind::Vine);
    assert_eq!(regions[2].kind, ae::ClimbableKind::Wall);
}

#[test]
fn climbable_intgrid_rejects_unknown_value() {
    let layer = intgrid_layer(CLIMBABLE_LAYER, 1, 1, vec![99]);
    let err = emit_climbable_regions_from_intgrid(&layer, ae::Vec2::ZERO)
        .expect_err("unknown value should error");
    assert!(
        err.contains("unknown Climbable IntGrid value 99"),
        "expected error to mention the bad value, got: {err}"
    );
}

#[test]
fn climbable_intgrid_returns_empty_for_all_zero_layer() {
    let layer = intgrid_layer(CLIMBABLE_LAYER, 4, 4, vec![0; 16]);
    let regions = emit_climbable_regions_from_intgrid(&layer, ae::Vec2::ZERO).unwrap();
    assert!(regions.is_empty());
}
