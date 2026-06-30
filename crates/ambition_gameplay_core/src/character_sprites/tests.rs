//! Tests for the character spritesheet pipeline: `sprite_render_size`
//! geometry, atlas flat-index math, and that every reachable catalog
//! sprite resolves a `SheetRecord`/spec from its `*_spritesheet.ron`
//! (parses, has an Idle row, reproduces the legacy hardcoded tuning).

use bevy::prelude::Vec2;

use super::anim::CharacterAnim;
use super::assets::sheet_for_character_id;

/// Data-path stand-in for the deleted `ROBOT_SHEET` static.
fn robot_sheet() -> super::sheets::CharacterSheetSpec {
    sheet_for_character_id("robot").expect("robot catalog row resolves a sheet")
}
use super::registry::SheetRecord;
use super::sheets::{sprite_render_size, try_load_spec_for_target, SheetTuning};

#[test]
fn sprite_render_size_uses_max_collision_axis() {
    // Tall narrow body: render height tracks collision.y (the
    // larger axis), scaled by collision_scale.
    let collision = Vec2::new(28.0, 46.0);
    let size = sprite_render_size(&robot_sheet(), collision);
    let expected_height = 46.0 * robot_sheet().collision_scale;
    assert!((size.y - expected_height).abs() < 1e-3);
}

#[test]
fn sprite_render_size_clamps_at_minimum_eight() {
    // Tiny collision boxes hit the 8.0 floor so micro-entities
    // (debris-sized actors) still render visibly.
    let collision = Vec2::new(2.0, 1.0);
    let size = sprite_render_size(&robot_sheet(), collision);
    let expected_height = 8.0 * robot_sheet().collision_scale;
    assert!((size.y - expected_height).abs() < 1e-3);
}

#[test]
fn sprite_render_size_preserves_frame_aspect() {
    // Width tracks the frame's source aspect, not the collision
    // box, so cropped non-square frames don't get distorted.
    let collision = Vec2::new(28.0, 46.0);
    let size = sprite_render_size(&robot_sheet(), collision);
    let expected_aspect = robot_sheet().frame_width as f32 / robot_sheet().frame_height as f32;
    let actual_aspect = size.x / size.y;
    assert!(
        (actual_aspect - expected_aspect).abs() < 1e-3,
        "expected aspect {expected_aspect}, got {actual_aspect}"
    );
}

#[test]
fn flat_index_zero_for_first_frame_of_first_row() {
    let idx = robot_sheet().flat_index(CharacterAnim::Idle, 0);
    assert_eq!(idx, 0);
}

#[test]
fn frame_count_positive_for_every_row() {
    for anim in robot_sheet().mapped_anims() {
        assert!(
            robot_sheet().frame_count(anim) > 0,
            "anim {:?} has zero frames",
            anim
        );
    }
}

#[test]
fn flat_index_clamps_to_last_frame_of_row() {
    // Asking for frame past the end of a row clamps to the last
    // valid frame; this avoids out-of-bounds atlas reads when the
    // animation cursor overshoots due to a long delta-t.
    let last = robot_sheet().flat_index(CharacterAnim::Idle, 9_999);
    let expected = robot_sheet().frame_count(CharacterAnim::Idle) - 1;
    assert_eq!(last, expected);
}

#[test]
fn robot_sheet_has_fly_row() {
    // The generator's `hover` row is the source of the Fly visual.
    // If a future sheet regen drops or reorders hover, this test
    // catches it before runtime indexes a non-existent row.
    assert_eq!(robot_sheet().frame_count(CharacterAnim::Fly), 8);
    assert!((robot_sheet().frame_duration(CharacterAnim::Fly) - 0.078).abs() < 1e-4);
    // Hover is the LAST row in the regenerated sheet, so its frames
    // sit after every other row in atlas-flat-index space.
    let fly_first = robot_sheet().flat_index(CharacterAnim::Fly, 0);
    let dash_last = robot_sheet().flat_index(
        CharacterAnim::Dash,
        robot_sheet().frame_count(CharacterAnim::Dash),
    );
    assert!(
        fly_first > dash_last,
        "Fly row must follow Dash; fly_first={fly_first} dash_last={dash_last}"
    );
}

#[test]
fn frame_duration_positive_for_every_row() {
    // Zero or negative duration would wedge the animation cursor
    // (advance_anim divides by it). Pin the contract.
    for anim in robot_sheet().mapped_anims() {
        assert!(
            robot_sheet().frame_duration(anim) > 0.0,
            "anim {:?} has non-positive duration",
            anim
        );
    }
}

/// Every sheet the game can reach must load with sane geometry:
/// every catalog character id that names a manifest, plus every
/// manifest target the content intro/prop registries reference
/// (formerly the `*_SHEET` statics; Stage 20 / B3 made them data).
#[test]
fn every_reachable_sheet_loads() {
    use crate::character_roster::EMBEDDED_CATALOG;
    let mut checked = 0usize;
    for (cid, entry) in EMBEDDED_CATALOG.characters.iter() {
        let Some(target) = entry.manifest_target() else {
            continue;
        };
        let Some(spec) = try_load_spec_for_target(target, &SheetTuning::default()) else {
            // No manifest on disk yet — the runtime renders the
            // colored-rectangle placeholder; not a test failure.
            continue;
        };
        checked += 1;
        assert!(spec.frame_width > 0, "{cid}: frame_width == 0");
        assert!(spec.frame_height > 0, "{cid}: frame_height == 0");
        assert!(
            spec.mapped_anims().next().is_some(),
            "{cid}: zero mapped rows after load"
        );
    }
    assert!(
        checked >= 20,
        "expected at least 20 catalog sheets to load, got {checked} — \
         did the manifest index break?"
    );
}

/// Every `*_spritesheet.ron` manifest must deserialize cleanly
/// into `Vec<SheetRecord>`. This is the runtime contract the
/// `SheetRegistry` startup loader depends on; if a generator emits
/// a RON the loader can't parse, the registry silently drops it
/// and any consumer that expected that sheet falls back to default.
///
/// Validating the parse here catches malformed RON at `cargo test`
/// time instead of at game startup.
#[test]
fn every_spritesheet_ron_parses_into_sheet_record() {
    let assets_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/sprites");
    let entries = std::fs::read_dir(&assets_dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", assets_dir.display()));

    let mut parsed_records = 0usize;
    let mut parsed_files = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with("_spritesheet.ron") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        match ron::from_str::<Vec<SheetRecord>>(&text) {
            Ok(records) => {
                parsed_files += 1;
                assert!(!records.is_empty(), "{name}: zero records in file");
                for record in &records {
                    parsed_records += 1;
                    assert!(!record.target.is_empty(), "{name}: empty target");
                    assert!(record.frame_width > 0, "{name}: frame_width == 0");
                    assert!(record.frame_height > 0, "{name}: frame_height == 0");
                }
            }
            Err(err) => {
                failures.push(format!("{name}: {err}"));
            }
        }
    }
    assert!(
        parsed_files > 0,
        "no *_spritesheet.ron found under {}",
        assets_dir.display()
    );
    assert!(parsed_records >= parsed_files);
    if !failures.is_empty() {
        panic!(
            "{} RON manifest(s) failed to parse:\n  {}",
            failures.len(),
            failures.join("\n  "),
        );
    }
}

// `sheet_consts_match_their_ron_manifests` was deleted with the `*_SHEET`
// statics: specs are now BUILT from the RON manifests, so agreement holds
// by construction (the parse test above + the oracle below cover the
// remaining contract).

/// Transcription oracle for the Stage 20 / B3 catalog-tuning migration:
/// the data-driven `sheet_for_character_id` must reproduce EXACTLY the
/// tuning the old hardcoded `*_SHEET` statics carried (values
/// transcribed here from the deleted sheets.rs constants). Each character
/// must index its OWN published sheet: the former cross-id atlas borrow
/// (`sprite_target`: standard pirates -> admiral, oni leader -> duelist)
/// was removed because it misaligned the animation once sheets became
/// per-frame alpha-trimmed (own texture + a foreign sheet's rects).
#[test]
fn catalog_tuning_reproduces_the_old_hardcoded_sheets() {
    // (id, collision_scale, frame_sample_inset)
    let expected = [
        ("player", 1.35, 1),
        ("robot", 2.1, 1),
        ("goblin", 2.1, 1),
        ("sandbag", 1.38, 1),
        ("npc_general", 1.15, 2),
        ("npc_goblin_cantina_chieftain", 1.16, 1),
        ("npc_pulse_voyager_captain", 1.20, 1),
        ("npc_tech_bro_disruptor", 1.20, 1),
        ("npc_pirate_admiral", 1.6, 1),
        ("npc_pirate_quartermaster", 1.6, 1),
        ("npc_pirate_lookout", 1.6, 1),
        ("npc_pirate_navigator", 1.6, 1),
        ("npc_pirate_raider", 1.6, 1),
        ("npc_pirate_heavy_broadside_bess", 1.95, 1),
        ("npc_pirate_heavy_iron_mary", 1.95, 1),
        ("npc_pirate_heavy_salt_annet", 1.95, 1),
        ("npc_burning_flying_shark", 0.8, 1),
        ("npc_puppy_slug", 1.4, 1),
        ("npc_ninja_shadow_oni_leader", 1.5, 1),
        ("npc_ninja_shadow_duelist", 1.5, 1),
        ("npc_architect", 1.10, 2),
        ("npc_kernel_guide", 1.10, 2),
        ("npc_vault_keeper", 1.10, 2),
        ("npc_merchant_prototype", 1.10, 2),
    ];
    for (id, collision_scale, inset) in expected {
        let spec = sheet_for_character_id(id)
            .unwrap_or_else(|| panic!("catalog id '{id}' must resolve a sheet spec"));
        assert!(
            (spec.collision_scale - collision_scale).abs() < 1e-6,
            "{id}: collision_scale {} != legacy {collision_scale}",
            spec.collision_scale
        );
        assert_eq!(
            spec.frame_sample_inset, inset,
            "{id}: frame_sample_inset != legacy value"
        );
    }

    // Own-sheet resolution (regression guard for the removed `sprite_target`
    // atlas borrow): each of these characters must index its OWN packed sheet,
    // not the representative's. The idle frame-0 atlas rect is read from the
    // resolved record, so borrowing the admiral's/duelist's rects (which trim
    // to different widths) would make these rects equal the representative's.
    let idle_rect = |id: &str| {
        sheet_for_character_id(id)
            .unwrap_or_else(|| panic!("{id} must resolve a sheet spec"))
            .texture_rect_for_flat_index(0)
            .unwrap_or_else(|| panic!("{id} idle frame 0 must have an atlas rect"))
    };
    let admiral_idle = idle_rect("npc_pirate_admiral");
    for id in [
        "npc_pirate_quartermaster",
        "npc_pirate_lookout",
        "npc_pirate_navigator",
    ] {
        assert_ne!(
            idle_rect(id),
            admiral_idle,
            "{id} must index its own packed sheet, not the admiral's \
             (cross-id atlas borrow regressed)",
        );
    }
    assert_ne!(
        idle_rect("npc_ninja_shadow_oni_leader"),
        idle_rect("npc_ninja_shadow_duelist"),
        "oni leader must index its own packed sheet, not the duelist's",
    );
}

// Catalog<->sheet integration tests, moved from actor::character_catalog
// (Stage 22): they exercise sheet RESOLUTION, which is this module's contract,
// and keeping them here lets the actor unit drop its presentation dependency.

/// Regression: the boss subdir manifests (gnu_ton_boss,
/// mockingbird_boss) must produce a working `CharacterSheetSpec`
/// at runtime. They reached the catalog via `synth_boss_manifest`,
/// but that script's `anchors: {}` was serialized as `anchors:
/// ()` by an in-house RON dumper bug — RON rejected `()` as a
/// HashMap value, the manifest failed to parse, and the Hall
/// silently rendered placeholders for both bosses (2026-05-24).
/// `inspect_hall_sprites.py` couldn't see the issue because its
/// `pyron.load` parser was more permissive than the Rust runtime's
/// `ron::from_str`. Pin both ids here so any future
/// reintroduction of the bug trips a focused diff.
#[test]
fn boss_subdir_manifests_resolve_through_catalog() {
    for cid in &["npc_gnu_ton_boss", "npc_mockingbird_boss"] {
        let spec = sheet_for_character_id(cid);
        assert!(
            spec.is_some(),
            "{cid}: sheet_for_character_id returned None — manifest \
                 parse error or subdir scan miss. Runtime would render \
                 as placeholder. Check the on-disk RON parses with \
                 `ron::from_str::<Vec<SheetRecord>>`.",
        );
    }
}

#[test]
fn every_catalog_sprite_spec_has_idle_row_if_loaded() {
    // The actor renderer's `flat_index` falls back to `Idle`
    // for any animation that doesn't have its own row. A spec
    // *without* an Idle row crashes on the first frame. This
    // test walks every catalog id, asks the sprite loader for
    // a spec, and verifies the spec either declines to load
    // (None) or includes an Idle row — never an Idle-less spec
    // that the runtime would unwrap into a panic.
    //
    // Caught a real crash 2026-05-24 when the manifest-driven
    // fallback loaded a spec for a character whose generated
    // sheet only had run/walk rows (no idle).

    let data = crate::character_roster::load_embedded();
    for cid in data.characters.keys() {
        let Some(spec) = sheet_for_character_id(cid) else {
            continue;
        };
        let has_idle = spec.maps(CharacterAnim::Idle);
        assert!(
            has_idle,
            "catalog id '{cid}' loaded a spec without an Idle row; \
                 sheet_for_character_id must return None or a spec with Idle",
        );
    }
}

#[test]
fn sprite_loader_resolves_a_sheet_for_most_catalog_entries() {
    // Phase 6 + manifest-driven fallback (2026-05-24): every
    // catalog id either resolves to a hardcoded `*_SHEET` const
    // (for the entries that need bespoke tuning) or falls back
    // to the manifest-driven `try_load_spec_for_character_id`
    // path (everything else with a sheet on disk).
    //
    // The Hall of Characters is the visible consumer of this
    // coverage — every pedestal whose `sheet_for_character_id`
    // returns `None` shows a colored-rectangle fallback. Pin
    // a generous lower bound (>=70 of ~99) so the Hall stays
    // mostly populated; the few stragglers (robot_heavy and
    // similar variant-only targets) ship later when their
    // publisher lands.

    let data = crate::character_roster::load_embedded();
    let covered = data
        .characters
        .keys()
        .filter(|cid| sheet_for_character_id(cid).is_some())
        .count();
    assert!(
        covered >= 70,
        "expected >=70 catalog ids to resolve to a sheet spec (hardcoded const \
             or manifest); got {covered}",
    );
}

/// `resolve_anim` renders the most-specific pose in the actor's OWN anim set
/// (the rows the generator wrote into the manifest), walking the pose taxonomy
/// toward the base — never snapping to `Idle` for a pose it has a relative of.
/// This is what lets every body run the one shared ladder: the body can be
/// driven into any state, and its sheet decides how richly it reads.
#[test]
fn resolve_anim_renders_most_specific_pose_in_the_actor_anim_set() {
    use super::CharacterAnim;
    // The admiral's generated set is idle / walk / slash / taunt / hurt / death —
    // no dash / run / jump / fly / directional-tilt rows.
    let spec = sheet_for_character_id("npc_pirate_admiral").expect("admiral resolves a sheet");
    // Directional / aerial / heavy swings are refinements of the generic slash
    // it DOES have → render slash, not Idle.
    assert_eq!(spec.resolve_anim(CharacterAnim::AttackUp), CharacterAnim::Slash);
    assert_eq!(spec.resolve_anim(CharacterAnim::AirDown), CharacterAnim::Slash);
    assert_eq!(spec.resolve_anim(CharacterAnim::Punch), CharacterAnim::Slash);
    // Dash / Slide refine down to the locomotion base it has (walk).
    assert_eq!(spec.resolve_anim(CharacterAnim::Dash), CharacterAnim::Walk);
    assert_eq!(spec.resolve_anim(CharacterAnim::Slide), CharacterAnim::Walk);
    // A pose it has resolves to itself.
    assert_eq!(spec.resolve_anim(CharacterAnim::Walk), CharacterAnim::Walk);
    assert_eq!(spec.resolve_anim(CharacterAnim::Death), CharacterAnim::Death);
    // A pose with no relative in the set is the only case that floors at Idle.
    assert_eq!(spec.resolve_anim(CharacterAnim::Fly), CharacterAnim::Idle);
    assert_eq!(spec.resolve_anim(CharacterAnim::Jump), CharacterAnim::Idle);
}
