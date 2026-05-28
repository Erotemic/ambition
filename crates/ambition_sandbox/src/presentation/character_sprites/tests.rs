use bevy::prelude::Vec2;

use super::anim::CharacterAnim;
use super::registry::SheetRecord;
use super::sheets::{
    sprite_render_size, CharacterSheetSpec, ABSURD_GENERAL_SHEET, ALICE_SHEET, ARCHITECT_SHEET,
    BOB_SHEET, BURNING_FLYING_SHARK_SHEET, CART_SHEET, CREATOR_SHEET, ERDISH_SHEET,
    GATE_PORTAL_SHEET, GATE_RING_SHEET, GOBLIN_CANTINA_CHIEFTAIN_SHEET, GOBLIN_SHEET,
    KERNEL_GUIDE_SHEET, LAB_PROP_DRONE_CRADLE, LAB_PROP_GENESIS_VAT, LAB_PROP_NEURAL_CONSOLE,
    LAB_PROP_PORTAL_CALIBRATOR, LAB_PROP_POWER_CORE, LAB_PROP_REPAIR_CRADLE,
    LAB_PROP_RESONANCE_COIL, LAB_PROP_SPECIMEN_JAR, MERCHANT_PROTOTYPE_SHEET, NEWS_BOARD_SHEET,
    NINJA_SHEET, OILER_SHEET, PIRATE_SHEET, PLAYER_ROBOT_SHEET, PULSE_VOYAGER_CAPTAIN_SHEET,
    RAID_ENFORCER_SHEET, ROBOT_SHEET, SANDBAG_SHEET, TECH_BRO_DISRUPTOR_SHEET, VAULT_KEEPER_SHEET,
};

#[test]
fn sprite_render_size_uses_max_collision_axis() {
    // Tall narrow body: render height tracks collision.y (the
    // larger axis), scaled by collision_scale.
    let collision = Vec2::new(28.0, 46.0);
    let size = sprite_render_size(&ROBOT_SHEET, collision);
    let expected_height = 46.0 * ROBOT_SHEET.collision_scale;
    assert!((size.y - expected_height).abs() < 1e-3);
}

#[test]
fn sprite_render_size_clamps_at_minimum_eight() {
    // Tiny collision boxes hit the 8.0 floor so micro-entities
    // (debris-sized actors) still render visibly.
    let collision = Vec2::new(2.0, 1.0);
    let size = sprite_render_size(&ROBOT_SHEET, collision);
    let expected_height = 8.0 * ROBOT_SHEET.collision_scale;
    assert!((size.y - expected_height).abs() < 1e-3);
}

#[test]
fn sprite_render_size_preserves_frame_aspect() {
    // Width tracks the frame's source aspect, not the collision
    // box, so cropped non-square frames don't get distorted.
    let collision = Vec2::new(28.0, 46.0);
    let size = sprite_render_size(&ROBOT_SHEET, collision);
    let expected_aspect = ROBOT_SHEET.frame_width as f32 / ROBOT_SHEET.frame_height as f32;
    let actual_aspect = size.x / size.y;
    assert!(
        (actual_aspect - expected_aspect).abs() < 1e-3,
        "expected aspect {expected_aspect}, got {actual_aspect}"
    );
}

#[test]
fn flat_index_zero_for_first_frame_of_first_row() {
    let idx = ROBOT_SHEET.flat_index(CharacterAnim::Idle, 0);
    assert_eq!(idx, 0);
}

#[test]
fn frame_count_positive_for_every_row() {
    for (anim, _) in &ROBOT_SHEET.rows {
        assert!(
            ROBOT_SHEET.frame_count(*anim) > 0,
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
    let last = ROBOT_SHEET.flat_index(CharacterAnim::Idle, 9_999);
    let expected = ROBOT_SHEET.frame_count(CharacterAnim::Idle) - 1;
    assert_eq!(last, expected);
}

#[test]
fn robot_sheet_has_fly_row() {
    // The generator's `hover` row is the source of the Fly visual.
    // If a future sheet regen drops or reorders hover, this test
    // catches it before runtime indexes a non-existent row.
    assert_eq!(ROBOT_SHEET.frame_count(CharacterAnim::Fly), 8);
    assert!((ROBOT_SHEET.frame_duration(CharacterAnim::Fly) - 0.078).abs() < 1e-4);
    // Hover is the LAST row in the regenerated sheet, so its frames
    // sit after every other row in atlas-flat-index space.
    let fly_first = ROBOT_SHEET.flat_index(CharacterAnim::Fly, 0);
    let dash_last = ROBOT_SHEET.flat_index(
        CharacterAnim::Dash,
        ROBOT_SHEET.frame_count(CharacterAnim::Dash),
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
    for (anim, _) in &ROBOT_SHEET.rows {
        assert!(
            ROBOT_SHEET.frame_duration(*anim) > 0.0,
            "anim {:?} has non-positive duration",
            anim
        );
    }
}

/// Touching every published `*_SHEET` static forces its `LazyLock` to
/// initialize via `load_spec(...)`. Catches the "this static refers to
/// a `target` id that no RON file actually exposes" failure mode at
/// `cargo test` time instead of waiting for `setup_presentation_system`
/// to panic at game launch. The list must include every public
/// `*_SHEET` and `LAB_PROP_*` static defined in `sheets.rs`.
#[test]
fn every_published_sheet_static_loads() {
    let specs: &[(&str, &CharacterSheetSpec)] = &[
        ("ABSURD_GENERAL_SHEET", &ABSURD_GENERAL_SHEET),
        ("ALICE_SHEET", &ALICE_SHEET),
        ("ARCHITECT_SHEET", &ARCHITECT_SHEET),
        ("BOB_SHEET", &BOB_SHEET),
        ("BURNING_FLYING_SHARK_SHEET", &BURNING_FLYING_SHARK_SHEET),
        ("CART_SHEET", &CART_SHEET),
        ("CREATOR_SHEET", &CREATOR_SHEET),
        ("ERDISH_SHEET", &ERDISH_SHEET),
        ("RAID_ENFORCER_SHEET", &RAID_ENFORCER_SHEET),
        ("GATE_PORTAL_SHEET", &GATE_PORTAL_SHEET),
        ("GATE_RING_SHEET", &GATE_RING_SHEET),
        (
            "GOBLIN_CANTINA_CHIEFTAIN_SHEET",
            &GOBLIN_CANTINA_CHIEFTAIN_SHEET,
        ),
        ("GOBLIN_SHEET", &GOBLIN_SHEET),
        ("KERNEL_GUIDE_SHEET", &KERNEL_GUIDE_SHEET),
        ("LAB_PROP_DRONE_CRADLE", &LAB_PROP_DRONE_CRADLE),
        ("LAB_PROP_GENESIS_VAT", &LAB_PROP_GENESIS_VAT),
        ("LAB_PROP_NEURAL_CONSOLE", &LAB_PROP_NEURAL_CONSOLE),
        ("LAB_PROP_PORTAL_CALIBRATOR", &LAB_PROP_PORTAL_CALIBRATOR),
        ("LAB_PROP_POWER_CORE", &LAB_PROP_POWER_CORE),
        ("LAB_PROP_REPAIR_CRADLE", &LAB_PROP_REPAIR_CRADLE),
        ("LAB_PROP_RESONANCE_COIL", &LAB_PROP_RESONANCE_COIL),
        ("LAB_PROP_SPECIMEN_JAR", &LAB_PROP_SPECIMEN_JAR),
        ("MERCHANT_PROTOTYPE_SHEET", &MERCHANT_PROTOTYPE_SHEET),
        ("NEWS_BOARD_SHEET", &NEWS_BOARD_SHEET),
        ("NINJA_SHEET", &NINJA_SHEET),
        ("OILER_SHEET", &OILER_SHEET),
        ("PIRATE_SHEET", &PIRATE_SHEET),
        ("PLAYER_ROBOT_SHEET", &PLAYER_ROBOT_SHEET),
        ("PULSE_VOYAGER_CAPTAIN_SHEET", &PULSE_VOYAGER_CAPTAIN_SHEET),
        ("ROBOT_SHEET", &ROBOT_SHEET),
        ("SANDBAG_SHEET", &SANDBAG_SHEET),
        ("TECH_BRO_DISRUPTOR_SHEET", &TECH_BRO_DISRUPTOR_SHEET),
        ("VAULT_KEEPER_SHEET", &VAULT_KEEPER_SHEET),
    ];
    for (name, spec) in specs {
        assert!(spec.frame_width > 0, "{name}: frame_width == 0");
        assert!(spec.frame_height > 0, "{name}: frame_height == 0");
        assert!(!spec.rows.is_empty(), "{name}: zero rows after load");
    }
}

/// Pull a `u32` value out of a YAML file via a tiny line-based parser.
/// The sprite manifests put `frame_width:`, `frame_height:`, `label_width:`
/// as top-level scalar fields at the head of each file, so we don't need
/// a real YAML dep to read them.
fn yaml_top_u32(text: &str, key: &str) -> Option<u32> {
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&format!("{key}:")) {
            let rest = rest.trim();
            return rest.split_whitespace().next()?.parse::<u32>().ok();
        }
    }
    None
}

/// Pull `body_metrics.feet_anchor_norm.y` out of a manifest. The nested
/// shape is consistent across every renderer:
///
///   body_metrics:
///     feet_anchor_norm:
///       x: ...
///       y: -0.482456
fn yaml_feet_anchor_norm_y(text: &str) -> Option<f32> {
    let mut in_metrics = false;
    let mut in_anchor = false;
    for line in text.lines() {
        let trimmed_start = line.trim_start();
        let indent = line.len() - trimmed_start.len();
        if indent == 0 {
            // Top-level key — reset our nesting tracker.
            in_metrics = trimmed_start.starts_with("body_metrics:");
            in_anchor = false;
            continue;
        }
        if !in_metrics {
            continue;
        }
        if indent == 2 && trimmed_start.starts_with("feet_anchor_norm:") {
            in_anchor = true;
            continue;
        }
        if indent == 2 && trimmed_start.ends_with(':') {
            // Different sub-key under body_metrics → exit anchor block.
            in_anchor = false;
            continue;
        }
        if in_anchor && indent >= 4 {
            if let Some(rest) = trimmed_start.strip_prefix("y:") {
                return rest.trim().parse::<f32>().ok();
            }
        }
    }
    None
}

/// Catch sheet-spec / YAML drift the moment it lands. The auto-crop in
/// `pirates/common::build_sheet` shrinks each frame to its union alpha
/// bbox + crop_margin, so any animation edit that changes the silhouette
/// envelope changes the YAML's `frame_width` / `frame_height` — and if
/// the hardcoded const stays at the old value, the game's URect samples
/// the wrong window of the PNG (the actual cause of the May 22 pirate /
/// shark misalignment bug). The follow-up is to drive sheet sizes from
/// the YAML at load time; until then, this test makes drift loud.
#[test]
fn sheet_consts_match_their_yaml_manifests() {
    // (const, sheet, yaml_target_id_in_assets_dir)
    let cases: &[(&str, &CharacterSheetSpec, &str)] = &[
        (
            "ABSURD_GENERAL_SHEET",
            &ABSURD_GENERAL_SHEET,
            "absurd_general",
        ),
        ("ALICE_SHEET", &ALICE_SHEET, "alice"),
        ("ARCHITECT_SHEET", &ARCHITECT_SHEET, "architect"),
        ("BOB_SHEET", &BOB_SHEET, "bob"),
        (
            "BURNING_FLYING_SHARK_SHEET",
            &BURNING_FLYING_SHARK_SHEET,
            "burning_flying_shark",
        ),
        ("CART_SHEET", &CART_SHEET, "intro_cart"),
        ("CREATOR_SHEET", &CREATOR_SHEET, "creator"),
        ("ERDISH_SHEET", &ERDISH_SHEET, "erdish"),
        ("RAID_ENFORCER_SHEET", &RAID_ENFORCER_SHEET, "raid_enforcer"),
        (
            "GATE_PORTAL_SHEET",
            &GATE_PORTAL_SHEET,
            "interdimensional_gate_portal",
        ),
        (
            "GATE_RING_SHEET",
            &GATE_RING_SHEET,
            "interdimensional_gate_ring",
        ),
        (
            "GOBLIN_CANTINA_CHIEFTAIN_SHEET",
            &GOBLIN_CANTINA_CHIEFTAIN_SHEET,
            "goblin_cantina_chieftain",
        ),
        ("GOBLIN_SHEET", &GOBLIN_SHEET, "goblin"),
        ("KERNEL_GUIDE_SHEET", &KERNEL_GUIDE_SHEET, "kernel_guide"),
        (
            "MERCHANT_PROTOTYPE_SHEET",
            &MERCHANT_PROTOTYPE_SHEET,
            "merchant_prototype",
        ),
        ("NEWS_BOARD_SHEET", &NEWS_BOARD_SHEET, "news_board"),
        ("OILER_SHEET", &OILER_SHEET, "oiler"),
        // PIRATE_SHEET is shared by every pirate generator; admiral is the
        // representative manifest. The set is regenerated together so any
        // member matches if one does.
        ("PIRATE_SHEET", &PIRATE_SHEET, "pirate_admiral"),
        ("PLAYER_ROBOT_SHEET", &PLAYER_ROBOT_SHEET, "player_robot"),
        (
            "PULSE_VOYAGER_CAPTAIN_SHEET",
            &PULSE_VOYAGER_CAPTAIN_SHEET,
            "pulse_voyager_captain",
        ),
        ("ROBOT_SHEET", &ROBOT_SHEET, "robot"),
        ("SANDBAG_SHEET", &SANDBAG_SHEET, "sandbag"),
        (
            "TECH_BRO_DISRUPTOR_SHEET",
            &TECH_BRO_DISRUPTOR_SHEET,
            "tech_bro_disruptor",
        ),
        ("VAULT_KEEPER_SHEET", &VAULT_KEEPER_SHEET, "vault_keeper"),
    ];

    let assets_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/sprites");
    let mut mismatches: Vec<String> = Vec::new();
    let mut checked = 0usize;
    let mut missing_yamls: Vec<&str> = Vec::new();

    for (name, spec, target) in cases {
        let path = assets_dir.join(format!("{target}_spritesheet.yaml"));
        if !path.exists() {
            missing_yamls.push(name);
            continue;
        }
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {}", path.display()));
        let yfw = yaml_top_u32(&text, "frame_width")
            .unwrap_or_else(|| panic!("frame_width missing in {}", path.display()));
        let yfh = yaml_top_u32(&text, "frame_height")
            .unwrap_or_else(|| panic!("frame_height missing in {}", path.display()));
        let ylw = yaml_top_u32(&text, "label_width")
            .unwrap_or_else(|| panic!("label_width missing in {}", path.display()));

        checked += 1;
        if spec.frame_width != yfw || spec.frame_height != yfh || spec.label_width != ylw {
            mismatches.push(format!(
                "{}: const=(lw={}, fw={}, fh={}) yaml=(lw={}, fw={}, fh={}) — {}",
                name,
                spec.label_width,
                spec.frame_width,
                spec.frame_height,
                ylw,
                yfw,
                yfh,
                path.display(),
            ));
        }
        // Feet anchor: the YAML records `feet_anchor_norm.y` (in [-1, 1]
        // relative to the cropped frame center). Tolerance is 0.001 — the
        // YAML is rounded to ~6 decimals and the const is hand-rounded to
        // 4; tighter than that catches real drift, looser hides it.
        if let Some(yfa) = yaml_feet_anchor_norm_y(&text) {
            if (spec.feet_anchor_y - yfa).abs() > 0.001 {
                mismatches.push(format!(
                    "{}: feet_anchor_y const={:.4} vs yaml={:.4} — {}",
                    name,
                    spec.feet_anchor_y,
                    yfa,
                    path.display(),
                ));
            }
        }
    }

    assert!(checked > 0, "no sheet manifests resolved at all");
    if !mismatches.is_empty() {
        panic!(
            "{} sheet const(s) drifted from their YAML manifests; \
             update the const in sheets.rs to match (or regenerate the YAML \
             intentionally and update both):\n  {}",
            mismatches.len(),
            mismatches.join("\n  "),
        );
    }
    // Soft note: we don't fail on missing yamls because some specs
    // (e.g. lab props) deliberately don't ship a manifest yet.
    if !missing_yamls.is_empty() {
        eprintln!(
            "sheet_consts_match_their_yaml_manifests: skipped (no YAML on disk): {missing_yamls:?}"
        );
    }
}

/// Every `*_spritesheet.ron` next to a YAML must deserialize cleanly
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

/// For every sheet const that has a paired RON manifest, the const's
/// label_width / frame_width / frame_height / feet_anchor_y must
/// agree with the RON. This is the same drift check the YAML test
/// runs, but against the *canonical* machine-readable manifest the
/// runtime `SheetRegistry` actually loads.
///
/// When the two tests disagree, the YAML and RON sidecar are out of
/// sync — that means a generator wrote one but not the other, and
/// `pirates/common::build_sheet` / `sheet::write_spritesheet` need a
/// re-run.
#[test]
fn sheet_consts_match_their_ron_manifests() {
    let cases: &[(&str, &CharacterSheetSpec, &str)] = &[
        (
            "ABSURD_GENERAL_SHEET",
            &ABSURD_GENERAL_SHEET,
            "absurd_general",
        ),
        ("ALICE_SHEET", &ALICE_SHEET, "alice"),
        ("ARCHITECT_SHEET", &ARCHITECT_SHEET, "architect"),
        ("BOB_SHEET", &BOB_SHEET, "bob"),
        (
            "BURNING_FLYING_SHARK_SHEET",
            &BURNING_FLYING_SHARK_SHEET,
            "burning_flying_shark",
        ),
        ("CART_SHEET", &CART_SHEET, "intro_cart"),
        ("CREATOR_SHEET", &CREATOR_SHEET, "creator"),
        ("ERDISH_SHEET", &ERDISH_SHEET, "erdish"),
        ("RAID_ENFORCER_SHEET", &RAID_ENFORCER_SHEET, "raid_enforcer"),
        (
            "GATE_PORTAL_SHEET",
            &GATE_PORTAL_SHEET,
            "interdimensional_gate_portal",
        ),
        (
            "GATE_RING_SHEET",
            &GATE_RING_SHEET,
            "interdimensional_gate_ring",
        ),
        (
            "GOBLIN_CANTINA_CHIEFTAIN_SHEET",
            &GOBLIN_CANTINA_CHIEFTAIN_SHEET,
            "goblin_cantina_chieftain",
        ),
        ("GOBLIN_SHEET", &GOBLIN_SHEET, "goblin"),
        ("KERNEL_GUIDE_SHEET", &KERNEL_GUIDE_SHEET, "kernel_guide"),
        (
            "MERCHANT_PROTOTYPE_SHEET",
            &MERCHANT_PROTOTYPE_SHEET,
            "merchant_prototype",
        ),
        ("NEWS_BOARD_SHEET", &NEWS_BOARD_SHEET, "news_board"),
        ("OILER_SHEET", &OILER_SHEET, "oiler"),
        ("PIRATE_SHEET", &PIRATE_SHEET, "pirate_admiral"),
        ("PLAYER_ROBOT_SHEET", &PLAYER_ROBOT_SHEET, "player_robot"),
        (
            "PULSE_VOYAGER_CAPTAIN_SHEET",
            &PULSE_VOYAGER_CAPTAIN_SHEET,
            "pulse_voyager_captain",
        ),
        ("ROBOT_SHEET", &ROBOT_SHEET, "robot"),
        ("SANDBAG_SHEET", &SANDBAG_SHEET, "sandbag"),
        (
            "TECH_BRO_DISRUPTOR_SHEET",
            &TECH_BRO_DISRUPTOR_SHEET,
            "tech_bro_disruptor",
        ),
        ("VAULT_KEEPER_SHEET", &VAULT_KEEPER_SHEET, "vault_keeper"),
    ];

    let assets_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/sprites");
    let mut mismatches: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for (name, spec, target) in cases {
        let path = assets_dir.join(format!("{target}_spritesheet.ron"));
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let records: Vec<SheetRecord> =
            ron::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        // Every per-character RON ships as a single-record list. The
        // file's name is the index key (the in-file `target` may be a
        // generator archetype like `"toon"` shared across files — see
        // `sheets::record_index`).
        let record = records
            .first()
            .unwrap_or_else(|| panic!("{}: zero records in file", path.display()));
        checked += 1;

        if spec.label_width != record.label_width
            || spec.frame_width != record.frame_width
            || spec.frame_height != record.frame_height
        {
            mismatches.push(format!(
                "{name}: const=(lw={}, fw={}, fh={}) ron=(lw={}, fw={}, fh={})",
                spec.label_width,
                spec.frame_width,
                spec.frame_height,
                record.label_width,
                record.frame_width,
                record.frame_height,
            ));
        }
        if let Some(metrics) = &record.body_metrics {
            if let Some(anchor) = metrics.feet_anchor_norm {
                if (spec.feet_anchor_y - anchor.y).abs() > 0.001 {
                    mismatches.push(format!(
                        "{name}: feet_anchor_y const={:.4} vs ron={:.4}",
                        spec.feet_anchor_y, anchor.y,
                    ));
                }
            }
        }
    }

    assert!(checked > 0, "no RON manifests resolved at all");
    if !mismatches.is_empty() {
        panic!(
            "{} const(s) drifted from RON manifests; resync the const or regen the sheet:\n  {}",
            mismatches.len(),
            mismatches.join("\n  "),
        );
    }
}
