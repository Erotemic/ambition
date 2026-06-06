//! Sanity tests against the embedded `sandbox.ldtk` / `intro.ldtk` projects.
//!
//! These tests load the real authored content and pin that:
//! - parser-level validation passes (`embedded_ldtk_validates`),
//! - every authored surface-like entity round-trips through the typed
//!   surface conversion (`embedded_surface_like_entities_lower_through_surface_model`),
//! - room metadata (biome, music_track) and feature placement (ladders,
//!   moving platforms, props) reach `RoomSpec` end-to-end,
//! - kinematic-path resolution survives the LDtk → engine emission,
//! - intro authoring keeps painted tile layers + the prop / NPC split.

use crate::engine_core as ae;

use super::super::fields::*;
use super::super::project::*;
use super::super::surfaces::*;

#[test]
fn embedded_ldtk_validates() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let report = project.validate();
    assert!(report.errors.is_empty(), "{:#?}", report.errors);
}

/// Surface-like entity identifiers are intentionally still supported in the
/// embedded LDtk file. They lower through the typed `LdtkSurfaceSpec` pipeline,
/// which is now the canonical runtime IR for rectangular collision/contact
/// authoring.
///
/// Earlier migration-era tests banned legacy-looking identifiers such as
/// `Solid` and `HazardBlock`, assuming every static rectangle had to become an
/// IntGrid tile. That is too strict now: the editor still exposes
/// differentiated surface identifiers, while the runtime parser collapses them
/// into the shared surface model. This test keeps the useful invariant —
/// embedded surface entities must validate through that model — without banning
/// the authoring vocabulary.
#[test]
fn embedded_surface_like_entities_lower_through_surface_model() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let mut surface_count = 0usize;

    for level in &project.levels {
        for layer in &level.layer_instances {
            for entity in &layer.entity_instances {
                if !is_surface_like_identifier(&entity.identifier) {
                    continue;
                }
                surface_count += 1;
                let name =
                    field_string(entity, "name").unwrap_or_else(|| entity.identifier.clone());
                let spec = parse_surface_spec(
                    entity,
                    ae::Vec2::ZERO,
                    ae::Vec2::new(entity.width as f32, entity.height as f32),
                    name,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "{}::{} ({}) should parse as an LDtk surface: {error}",
                        level.identifier, entity.identifier, entity.iid
                    )
                });
                compile_surface(&spec).unwrap_or_else(|error| {
                    panic!(
                        "{}::{} ({}) should compile as an LDtk surface: {error}",
                        level.identifier, entity.identifier, entity.iid
                    )
                });
            }
        }
    }

    assert!(
        surface_count > 0,
        "embedded LDtk should contain at least one surface-like entity so this audit exercises real content"
    );
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
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
/// enters the area. `goblin_encounter` deliberately does NOT set a
/// `music_track` — its music swap is owned by the encounter
/// system and only fires when the encounter triggers, not when
/// the door opens.
#[test]
fn embedded_ldtk_water_world_carries_music_track() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let data = crate::content::data::SandboxDataSpec::load_embedded();
    let valid = data.audio.music_tracks.iter().map(|t| t.id.as_str());
    let warnings = project.music_track_warnings(valid);
    assert!(
        warnings.is_empty(),
        "embedded LDtk references music_track ids not present in the audio catalog: {warnings:?}"
    );
}

/// `goblin_encounter` must NOT declare a `music_track` so entering the
/// goblin encounter door does not pre-empt the encounter system's music
/// override. Encounter starts/clears own the swap, and the
/// hub default plays while the room is unarmed.
#[test]
fn embedded_ldtk_goblin_encounter_does_not_carry_music_track() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("embedded LDtk should compose");
    let mob = room_set
        .rooms
        .iter()
        .find(|r| r.id == "goblin_encounter")
        .expect("goblin_encounter active area exists");
    assert_eq!(mob.metadata.biome.as_deref(), Some("mob_arena"));
    assert_eq!(
        mob.metadata.music_track, None,
        "goblin_encounter must not carry a music_track — the encounter system owns the swap"
    );
}

/// Phase 5 of the character-catalog refactor (see
/// `TODO-character-catalog-and-hall.md`). The Hall of Characters
/// room is auto-generated from `character_catalog.ron` and ships
/// one pedestal per spawnable character. Pins the room into
/// `sandbox.ldtk` so a future "delete the file by accident" trips
/// here rather than at runtime, and ensures every NpcSpawn the
/// generator emitted resolves to a catalog id.
#[test]
fn embedded_ldtk_hall_of_characters_has_expected_pedestals() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let hall = project
        .levels
        .iter()
        .find(|l| l.identifier == "hall_of_characters")
        .expect("hall_of_characters level should exist (Phase 5)");

    // Count NpcSpawn entities in the hall; every catalog entry
    // gets one. The catalog covers 99 characters today (89 main
    // hall + 10 basement); future additions should bump this
    // number, not break the room.
    let npc_count = hall
        .layer_instances
        .iter()
        .flat_map(|layer| &layer.entity_instances)
        .filter(|e| e.identifier == "NpcSpawn")
        .count();
    assert!(
        npc_count >= 80,
        "hall_of_characters should hold one NpcSpawn per catalog entry; got {npc_count}",
    );

    // Every NpcSpawn must carry a `character_id` field that
    // resolves in the embedded catalog.
    let catalog = crate::content::character_catalog::load_embedded();
    let mut unresolved: Vec<String> = Vec::new();
    for layer in &hall.layer_instances {
        for entity in &layer.entity_instances {
            if entity.identifier != "NpcSpawn" {
                continue;
            }
            let cid = field_string(entity, "character_id").unwrap_or_default();
            if cid.is_empty() {
                unresolved.push(format!("(empty) iid={}", entity.iid));
                continue;
            }
            if !catalog.characters.contains_key(&cid) {
                unresolved.push(cid);
            }
        }
    }
    assert!(
        unresolved.is_empty(),
        "hall NpcSpawns reference character_ids not in the catalog: {unresolved:?}",
    );
}

/// Companion to the pedestal-count test above: walks every NpcSpawn
/// in the Hall of Characters and verifies its character_id resolves
/// to either a sprite spec OR a graceful fallback (`None`). The
/// runtime renderer panics on a half-built spec (no Idle row), so
/// any failure mode here means the hall would crash on entry — which
/// is exactly the regression Jon hit 2026-05-24.
///
/// This test is the closest we have to "walk into the room without
/// crashing" without a full Bevy-app integration. Pairs with
/// `every_catalog_sprite_spec_has_idle_row_if_loaded` (which checks
/// the catalog->spec invariant) by checking it from the LDtk side
/// (the hall's actual placements).
#[test]
fn embedded_ldtk_hall_of_characters_every_npc_resolves_a_safe_sprite_state() {
    use crate::presentation::character_sprites::{sheet_for_character_id, CharacterAnim};

    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let hall = project
        .levels
        .iter()
        .find(|l| l.identifier == "hall_of_characters")
        .expect("hall_of_characters level should exist");

    let mut bad_specs: Vec<(String, String)> = Vec::new();
    for layer in &hall.layer_instances {
        for entity in &layer.entity_instances {
            if entity.identifier != "NpcSpawn" {
                continue;
            }
            let cid = field_string(entity, "character_id").unwrap_or_default();
            if cid.is_empty() {
                bad_specs.push(("(empty)".to_string(), "missing character_id".to_string()));
                continue;
            }
            // None is acceptable (colored-rectangle fallback).
            // Some(spec) MUST include an Idle row — otherwise
            // `flat_index` would panic at first frame.
            if let Some(spec) = sheet_for_character_id(&cid) {
                if !spec
                    .rows
                    .iter()
                    .any(|(anim, _)| matches!(anim, CharacterAnim::Idle))
                {
                    bad_specs.push((cid, "spec has no Idle row".to_string()));
                }
            }
        }
    }
    assert!(
        bad_specs.is_empty(),
        "hall NpcSpawns with unsafe sprite specs (would crash on render): {bad_specs:?}",
    );
}

#[test]
fn embedded_ldtk_composes_central_hub_complex() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
        room.boss_spawns.is_empty(),
        "boss belongs in the boss lab, not the stitched hub basement"
    );
    let boss_room = room_set
        .rooms
        .iter()
        .find(|room| room.id == "basement_boss")
        .expect("boss lab room exists");
    assert!(boss_room
        .boss_spawns
        .iter()
        .any(|authored| authored.name.contains("clockwork warden")));
}

#[test]
fn embedded_ldtk_central_hub_carries_authored_moving_platforms() {
    // Moving platforms are LDtk-authored gameplay objects. This test ensures
    // the central hub basement entity reaches the RoomSpec via the parser +
    // emission path so the runtime has no hidden procedural platform to fall
    // back to.
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
    let platform = &hub.moving_platforms[0];
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
fn embedded_ldtk_patrol_enemy_resolves_kinematic_path_index() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
    let patrol_path_id = room
        .enemy_spawns
        .iter()
        .find_map(|authored| match &authored.payload {
            crate::actor::EnemyBrain::Patrol {
                path_id: Some(path_id),
            } => Some(path_id.as_str()),
            _ => None,
        })
        .expect("basement_enemies should contain an authored patrol enemy");
    assert!(
        room.kinematic_paths
            .iter()
            .any(|spec| spec.matches_id(patrol_path_id)),
        "at least one authored patrol enemy should resolve through RoomSpec::kinematic_paths"
    );
}

#[test]
fn central_hub_collision_layer_lowers_to_engine_blocks() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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

/// Regression for ADR 0015 Step 2. Every intro level must carry a
/// painted IntroLabTiles layer instance; without painted tiles the
/// renderer wiring (`sync_ldtk_world_transform`) would draw nothing
/// over the blank LdtkWorldBundle.
///
/// The test counts non-empty `gridTiles` arrays per level rather
/// than asserting an exact pixel match — the tile content is
/// regenerable from the Collision IntGrid via `tileset paint`.
#[test]
fn intro_levels_carry_painted_tileset_layers() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox + intro LDtk should load");

    // The intro tileset and its Tiles layer were registered in
    // commit 66e62ad / the autonomous follow-up session.
    let tiles_layer_id = "IntroLabTiles";

    let intro_levels = [
        "intro_wake_room",
        "intro_raid_corridor",
        "intro_escape_shaft",
        "drain_alley",
        "gate_stack_lower",
    ];

    for level_id in &intro_levels {
        let level = project
            .levels
            .iter()
            .find(|l| l.identifier == *level_id)
            .unwrap_or_else(|| panic!("intro.ldtk must contain level '{level_id}'"));
        let tiles_layer = level
            .layer_instances
            .iter()
            .find(|l| l.identifier == tiles_layer_id)
            .unwrap_or_else(|| {
                panic!(
                    "level '{level_id}' must carry a '{tiles_layer_id}' Tiles layer; \
                     re-run `tileset add-layer` if a layer schema regression dropped it."
                )
            });
        assert!(
            !tiles_layer.grid_tiles.is_empty(),
            "level '{level_id}' / '{tiles_layer_id}' has 0 painted tiles; \
             re-run `tileset paint <ldtk> {level_id} {tiles_layer_id} \
             --from-intgrid Collision --map 1=0 --map 2=28 --in-place`"
        );
    }
}

/// Regression for the §1.3 portal bug fixed in commit 195b5ce. The
/// gate ring, gate portal, lab props, and intro cart used to be
/// authored as `NpcSpawn` entities with `prompt: ""` and
/// `dialogue_id: generic_npc` (the v1 hack), which leaked an
/// "Interact" prompt onto decorative props. The dedicated `Prop`
/// LDtk entity type replaces that — Props never grow an
/// Interactable, so the player walks past silently.
///
/// This test pins both halves of the invariant:
/// 1. Every expected prop kind shows up as a `PropSpec` on its
///    room's `RoomSpec.props`.
/// 2. No `RoomObject` with `InteractionKind::Npc` matches any of
///    those prop kinds' positions (a stronger check than just
///    counting NPCs, because story-content NPCs *do* still spawn
///    elsewhere).
#[test]
fn intro_props_do_not_grow_interactables() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox + intro LDtk should load");
    let room_set = project.to_room_set().expect("LDtk should compose");

    // The 6 prop kinds the v1 hack migrated. (PropSpec.kind values
    // straight out of intro.ldtk.)
    let expected_kinds = [
        "intro_cart",
        "lab_neural_console",
        "lab_genesis_vat",
        "lab_power_core",
        "gate_ring",
        "gate_portal",
    ];

    let mut seen_kinds: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut prop_positions: Vec<(String, ae::Vec2)> = Vec::new();
    for room in &room_set.rooms {
        for prop in &room.props {
            if let Some(kind) = expected_kinds.iter().find(|k| ***k == prop.kind) {
                seen_kinds.insert(*kind);
                prop_positions.push((prop.kind.clone(), prop.pos));
            }
        }
    }

    for kind in &expected_kinds {
        assert!(
            seen_kinds.contains(*kind),
            "expected Prop kind '{kind}' missing from intro.ldtk room set; \
             did a refactor drop the migrated Prop entity?"
        );
    }

    // No authored NPC interactable matches a prop's position. Props
    // and NPCs are now distinct authored families; this pin keeps the
    // separation honest.
    for room in &room_set.rooms {
        for authored in &room.interactables {
            if !matches!(
                authored.payload.kind,
                crate::interaction::InteractionKind::Npc { .. }
            ) {
                continue;
            }
            use ae::AabbExt as _;
            let it_center = authored.aabb.center();
            for (kind, prop_pos) in &prop_positions {
                let dx = (it_center.x - prop_pos.x).abs();
                let dy = (it_center.y - prop_pos.y).abs();
                assert!(
                    dx > 1.0 || dy > 1.0,
                    "authored NPC overlaps prop '{kind}' at ({:.1}, {:.1}); \
                     the dedicated Prop entity should NOT emit an Interactable. \
                     Either the migration regressed, or a new NPC was placed \
                     exactly on top of a prop.",
                    prop_pos.x,
                    prop_pos.y,
                );
            }
        }
    }
}

/// `GroundItem` LDtk entities convert into `RoomSpec.ground_items` carrying the
/// authored `held_item` registry id. This is the authored-placement home for
/// the gauntlet/weapon pickups that `spawn_debug_ground_items_once` used to
/// drop near the player; the test pins that the convert + RoomSpec plumbing is
/// intact by checking a representative subset of the tiny_chamber armory shelf.
#[test]
fn ldtk_authors_gauntlet_ground_items() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox + intro LDtk should load");
    let room_set = project.to_room_set().expect("LDtk should compose");

    let held_ids: std::collections::HashSet<&str> = room_set
        .rooms
        .iter()
        .flat_map(|room| room.ground_items.iter())
        .map(|gi| gi.held_item.as_str())
        .collect();

    for expected in ["meteor", "bomb", "blink", "puppy_slug_gun", "gun_sword"] {
        assert!(
            held_ids.contains(expected),
            "expected GroundItem held_item '{expected}' missing from the LDtk room \
             set; did the GroundItem entity def, convert_ground_item, or the \
             basement armory authoring break? (saw: {held_ids:?})"
        );
    }
}

/// `PortalGunSpawn` converts into `RoomSpec.portal_gun_spawns` — the authored
/// home for the portal-gun pickup (replacing spawn_debug_portal_gun_pickup_once).
#[cfg(feature = "portal_ldtk")]
#[test]
fn ldtk_authors_portal_gun_spawn() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox + intro LDtk should load");
    let room_set = project.to_room_set().expect("LDtk should compose");
    let total: usize = room_set
        .rooms
        .iter()
        .map(|room| room.portal_gun_spawns.len())
        .sum();
    assert!(
        total >= 1,
        "expected at least one authored PortalGunSpawn in the LDtk room set; did \
         the entity def, convert_portal_gun_spawn, or the authoring break?"
    );
}

/// The `portal_lab` test room authors its static portal pairs: 8 `Portal`
/// entities (four complementary pairs, one per orientation test case), and each
/// placed color has its partner placed (so every pair links).
#[cfg(feature = "portal_ldtk")]
#[test]
fn ldtk_authors_portal_lab_pairs() {
    use crate::portal::PortalColor;
    let project = LdtkProject::load_default_for_dev().expect("sandbox + intro LDtk should load");
    let room_set = project.to_room_set().expect("LDtk should compose");
    let portals: Vec<_> = room_set
        .rooms
        .iter()
        .flat_map(|room| room.portals.iter())
        .collect();
    assert!(
        portals.len() >= 8,
        "expected the portal_lab's 8 authored Portals; got {} — did the Portal \
         entity def / convert_portal / the area authoring break?",
        portals.len()
    );
    // Every placed color must have its partner placed, or a pair can't link.
    for portal in &portals {
        assert!(
            portals.iter().any(|p| p.color == portal.color.partner()),
            "portal color {:?} has no partner {:?} placed — the pair won't link",
            portal.color,
            portal.color.partner()
        );
    }
    // The four non-gun pairs the lab uses are all present.
    for color in [
        PortalColor::Purple,
        PortalColor::Yellow,
        PortalColor::Teal,
        PortalColor::Red,
        PortalColor::Green,
        PortalColor::Magenta,
        PortalColor::Cyan,
        PortalColor::Rose,
    ] {
        assert!(
            portals.iter().any(|p| p.color == color),
            "portal_lab should author a {color:?} portal"
        );
    }
}

/// `ShrineSpawn` and `GravityZone` convert into their `RoomSpec` lists — the
/// authored homes for the last two near-player debug spawns.
#[test]
fn ldtk_authors_shrine_and_gravity_zone() {
    let project = LdtkProject::load_default_for_dev().expect("sandbox + intro LDtk should load");
    let room_set = project.to_room_set().expect("LDtk should compose");
    let shrines: usize = room_set.rooms.iter().map(|r| r.shrines.len()).sum();
    let zones: usize = room_set.rooms.iter().map(|r| r.gravity_zones.len()).sum();
    assert!(
        shrines >= 1,
        "expected an authored ShrineSpawn in the LDtk room set"
    );
    assert!(
        zones >= 1,
        "expected an authored GravityZone in the LDtk room set (did convert_gravity_zone break?)"
    );
    // A sideways (horizontal-gravity) zone must exist so wall-walking is
    // reachable in-game (the gravity_lab `glab_wall_walk_right` zone).
    let sideways = room_set
        .rooms
        .iter()
        .flat_map(|r| r.gravity_zones.iter())
        .any(|z| z.dir.x != 0.0);
    assert!(
        sideways,
        "expected a sideways (wall-walking) GravityZone — the wall-walking demo is unreachable without one"
    );
}
