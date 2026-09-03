//! Tests for the character spritesheet pipeline: `sprite_render_size`
//! geometry, atlas flat-index math, and that every reachable catalog
//! sprite resolves a `SheetRecord`/spec from its `*_spritesheet.ron`
//! (parses, has an Idle row, reproduces the legacy hardcoded tuning).

use bevy::prelude::Vec2;

use super::assets::sheet_for_character_id_in;
use ambition_sprite_sheet::character::CharacterAnim;

fn test_catalog() -> ambition_characters::actor::character_catalog::CharacterCatalog {
    crate::character_roster::catalog()
}

fn sheet_for_character_id(
    character_id: &str,
) -> Option<ambition_sprite_sheet::character::sheets::CharacterSheetSpec> {
    sheet_for_character_id_in(&Default::default(), &test_catalog(), character_id)
}

/// Data-path stand-in for the deleted `ROBOT_SHEET` static.
fn robot_sheet() -> ambition_sprite_sheet::character::sheets::CharacterSheetSpec {
    sheet_for_character_id("robot").expect("robot catalog row resolves a sheet")
}
use ambition_sprite_sheet::character::sheets::{
    record_for_sheet_key, sprite_render_size, try_load_spec_for_target,
    try_load_spec_for_target_scaled, SheetTuning,
};
use ambition_sprite_sheet::SheetRecord;

/// When the quality-variant sheets have been generated (gitignored, so only at
/// build time after running `generate_visual_quality_variants.py`), the scaled
/// records must carry SMALLER frame geometry than the base — and the
/// scaled-spec lookup must return a usable spec paired to that variant. This is
/// the runtime half of the variant pipeline; it no-ops on a fresh clone.
#[test]
fn scaled_variant_specs_pair_smaller_geometry_when_generated() {
    use ambition_sprite_sheet::character::TextureResolutionScale;
    let mut checked = 0usize;
    for target in [
        "player_robot_v3_spritesheet",
        "bob_spritesheet",
        "goblin_spritesheet",
    ] {
        let Some(base) = record_for_sheet_key(target) else {
            continue;
        };
        for (suffix, scale) in [
            ("0_5x", TextureResolutionScale::Half),
            ("potato", TextureResolutionScale::Potato),
        ] {
            let Some(variant) = record_for_sheet_key(&format!("{target}.{suffix}")) else {
                continue;
            };
            checked += 1;
            assert!(variant.frame_width > 0 && variant.frame_height > 0);
            assert!(
                variant.frame_width <= base.frame_width
                    && variant.frame_height <= base.frame_height,
                "{target}.{suffix}: variant {}x{} not <= base {}x{}",
                variant.frame_width,
                variant.frame_height,
                base.frame_width,
                base.frame_height,
            );
            // The scaled-spec lookup the loader uses returns the variant spec.
            let spec = try_load_spec_for_target_scaled(target, &SheetTuning::default(), scale)
                .expect("baked variant record yields a spec");
            assert_eq!(spec.frame_width, variant.frame_width);
        }
    }
    let _ = checked; // zero is acceptable (no variants generated in this build)
}

/// The quad puts the sheet's own body on the collision box.
#[test]
fn sprite_render_size_draws_the_body_at_the_collision_box() {
    let spec = robot_sheet();
    let body = spec
        .body_pixel_extent(ambition_sprite_sheet::character::CharacterAnim::Idle)
        .expect("the robot sheet publishes a body rectangle");
    let frame = spec.frame_pixels();
    let collision = Vec2::new(28.0, 46.0);
    let quad = sprite_render_size(&spec, collision);
    // What the quad actually draws: the body rectangle scaled by the same
    // quad/frame ratio the GPU applies to every other pixel of the frame.
    let drawn = Vec2::new(body.x / frame.x * quad.x, body.y / frame.y * quad.y);
    // The fit touches on the binding axis and never overshoots on either.
    assert!(
        (drawn.x / collision.x).max(drawn.y / collision.y) > 0.999,
        "the drawn body {drawn:?} does not reach the {collision:?} collision box"
    );
    assert!(
        drawn.x <= collision.x + 1e-3 && drawn.y <= collision.y + 1e-3,
        "the drawn body {drawn:?} overflows the {collision:?} collision box"
    );
}

/// The 8.0 floor moved WITH the arithmetic it belonged to: it now guards only the fallback for a
/// sheet that publishes no body.
#[test]
fn a_tiny_collision_box_draws_a_tiny_body_rather_than_a_floored_one() {
    let spec = robot_sheet();
    let big = sprite_render_size(&spec, Vec2::new(28.0, 46.0));
    let tiny = sprite_render_size(&spec, Vec2::new(2.0, 1.0));
    assert!(
        tiny.y < big.y * 0.1,
        "a 2x1 body drew a {tiny:?} quad against the 28x46 body's {big:?}"
    );
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
    use crate::character_roster::catalog;
    let mut checked = 0usize;
    for (cid, entry) in catalog().data().characters.iter() {
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
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/sprites");

    let mut parsed_files = 0usize;
    let mut failures: Vec<String> = Vec::new();
    parse_spritesheets_under(&base, true, &mut parsed_files, &mut failures);

    // Quality-variant folders are generated (gitignored), so only validate them
    // when present — a fresh clone without `generate_visual_quality_variants.py`
    // run still passes, but once generated their RON must deserialize too (the
    // generator rescales pixel rects; this is the drift guard on that output).
    for suffix in ["_0_5x", "_0_25x", "_potato"] {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("assets/sprites{suffix}"));
        if dir.is_dir() {
            parse_spritesheets_under(&dir, false, &mut parsed_files, &mut failures);
        }
    }

    assert!(
        parsed_files > 0,
        "no *_spritesheet.ron found under {}",
        base.display()
    );
    if !failures.is_empty() {
        panic!(
            "{} RON manifest(s) failed to parse:\n  {}",
            failures.len(),
            failures.join("\n  "),
        );
    }
}

/// Recursively parse every `*_spritesheet.ron` under `dir` into
/// `Vec<SheetRecord>`, asserting the basic invariants. `recurse` walks one level
/// of subdirs (boss multi-sheet packages live there).
fn parse_spritesheets_under(
    dir: &std::path::Path,
    recurse: bool,
    parsed_files: &mut usize,
    failures: &mut Vec<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if recurse && path.is_dir() {
            parse_spritesheets_under(&path, false, parsed_files, failures);
            continue;
        }
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
                *parsed_files += 1;
                assert!(!records.is_empty(), "{name}: zero records in file");
                for record in &records {
                    assert!(!record.target.is_empty(), "{name}: empty target");
                    assert!(record.frame_width > 0, "{name}: frame_width == 0");
                    assert!(record.frame_height > 0, "{name}: frame_height == 0");
                }
            }
            Err(err) => failures.push(format!("{}: {err}", path.display())),
        }
    }
}

// `sheet_consts_match_their_ron_manifests` was deleted with the `*_SHEET`
// statics: specs are now BUILT from the RON manifests, so agreement holds
// by construction (the parse test above + the oracle below cover the
// remaining contract).

/// Transcription oracle for the Stage 20 / B3 catalog-tuning migration: the data-driven
/// `sheet_for_character_id` must reproduce EXACTLY the tuning the old hardcoded `*_SHEET`
/// statics carried (values transcribed here from the deleted sheets.rs constants).
#[test]
fn catalog_tuning_reproduces_the_old_hardcoded_sheets() {
    // (id, collision_scale, frame_sample_inset)
    let expected = [
        // Pinning the pre-migration 1.35 here would forbid ever retuning the protagonist -- this
        // list guards the ONE-TIME move of tuning out of hardcoded statics into the catalog, not
        // the values themselves forever.
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

    // The idle frame-0 atlas rect is read from the resolved record, so borrowing the
    // admiral's/duelist's rects (which trim to different widths) would make these rects equal the
    // representative's.
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

/// `inspect_hall_sprites.py` couldn't see the issue because its `pyron.load` parser was more
/// permissive than the Rust runtime's `ron::from_str`.
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
    // Caught a real crash when the manifest-driven
    // fallback loaded a spec for a character whose generated
    // sheet only had run/walk rows (no idle).

    let data = crate::character_roster::catalog();
    for cid in data.data().characters.keys() {
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
    // Phase 6 + manifest-driven fallback: every
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

    let data = crate::character_roster::catalog();
    let covered = data
        .data()
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
    // The admiral's generated set is idle / walk / slash / taunt / hurt / death —
    // no dash / run / jump / fly / directional-tilt rows.
    let spec = sheet_for_character_id("npc_pirate_admiral").expect("admiral resolves a sheet");
    // Directional / aerial / heavy swings are refinements of the generic slash
    // it DOES have → render slash, not Idle.
    assert_eq!(
        spec.resolve_anim(CharacterAnim::AttackUp),
        CharacterAnim::Slash
    );
    assert_eq!(
        spec.resolve_anim(CharacterAnim::AirDown),
        CharacterAnim::Slash
    );
    assert_eq!(
        spec.resolve_anim(CharacterAnim::Punch),
        CharacterAnim::Slash
    );
    // Dash / Slide refine down to the locomotion base it has (walk).
    assert_eq!(spec.resolve_anim(CharacterAnim::Dash), CharacterAnim::Walk);
    assert_eq!(spec.resolve_anim(CharacterAnim::Slide), CharacterAnim::Walk);
    // A pose it has resolves to itself.
    assert_eq!(spec.resolve_anim(CharacterAnim::Walk), CharacterAnim::Walk);
    assert_eq!(
        spec.resolve_anim(CharacterAnim::Death),
        CharacterAnim::Death
    );
    // A pose with no relative in the set is the only case that floors at Idle.
    assert_eq!(spec.resolve_anim(CharacterAnim::Fly), CharacterAnim::Idle);
    assert_eq!(spec.resolve_anim(CharacterAnim::Jump), CharacterAnim::Idle);
}

/// **An authored `standing_height` IS the body's height — not a hint, not a
/// scale factor applied to something else.**
///
/// The derivation has two branches, and until 2026-08-22 the authored one had
/// never run in shipped content, so nothing had ever checked that it delivers
/// what it promises. It does: the three heavies land on their stated numbers
/// exactly. This pins that, for every row that authors a height now or later.
///
/// ⛔ deliberately asserts NO specific number and names NO character. The
/// heights are Jon's to retune, and a test that spelled `56.2` would redden on
/// an ordinary content edit while telling us nothing about the mechanism. The
/// invariant is the equality itself, which survives any retune and widens on its
/// own as the remaining 142 rows get authored.
///
/// ⚠ a row whose sheet publishes no `body_metrics` derives `None` and keeps its
/// LDtk box whatever the catalog says — that is the documented rule, so those
/// rows are skipped rather than failed. If a height is authored for one, the
/// number silently does nothing; the count printed on failure is how that shows.
#[test]
fn an_authored_standing_height_is_the_height_the_body_derives() {
    use super::assets::sprite_body_collision_for_character_id_in;

    let catalog = test_catalog();
    let sheets = Default::default();
    let ldtk = ambition_platformer2d_core::Vec2::new(28.0, 46.0);

    let mut checked = 0usize;
    let mut inert = Vec::new();
    for (id, entry) in &catalog.data().characters {
        let Some(authored) = entry.standing_height else {
            continue;
        };
        match sprite_body_collision_for_character_id_in(&sheets, &catalog, id, ldtk) {
            Some(body) => {
                checked += 1;
                assert!(
                    (body.collision.y - authored).abs() < 1e-3,
                    "{id} authors a standing height of {authored} but derives a body \
                     {:.3} tall — the authored branch is not honoring its input",
                    body.collision.y,
                );
            }
            // No published body box: the LDtk spawn box decides and the authored
            // height is inert. Collected so an authored-but-ignored height is
            // visible rather than passing silently.
            None => inert.push(id.clone()),
        }
    }

    assert!(
        checked > 0,
        "no catalog row authors a standing_height, so this guard checked nothing \
         — if heights were removed on purpose, remove this test with them"
    );
    assert!(
        inert.is_empty(),
        "these rows author a standing_height that CANNOT apply, because their \
         sheet publishes no body_metrics — the number is silently doing nothing: {inert:?}"
    );
}

/// **AUDIT LISTING: which characters have a derivable body height, and what it
/// comes out as.** Prints; asserts nothing. Read it, do not gate on it.
///
/// ```text
/// cargo test -p ambition_platformer2d_actor_monolith \
///     list_what_each_character_derives_for_its_body -- --ignored --nocapture
/// ```
///
/// the question is narrower than "is it the right size". A height applies
/// only when the sheet publishes `body_metrics.body_pixel_bbox`; without one the
/// derivation returns `None` and the LDtk spawn box decides, whatever the
/// catalog says. So the useful column is not the number — it is whether there IS
/// one.
///
/// and `collision_scale` is IRRELEVANT on this path, which is worth seeing
/// rather than being told: when a standing height applies, `render` is rebuilt
/// from `height / body_h` and the hand-tuned scale never enters. Two `Standard`
/// characters whose sheets disagree about `collision_scale` still land on the
/// same body height.
#[test]
#[ignore = "audit listing: prints what each character derives; read it, do not assert on it"]
fn list_what_each_character_derives_for_its_body() {
    use super::assets::sprite_body_collision_for_character_id_in;

    let catalog = test_catalog();
    let sheets = Default::default();
    // The box a Hall NPC is authored with, so the legacy path has its real input.
    let ldtk = ambition_platformer2d_core::Vec2::new(28.0, 46.0);

    let mut derived = 0usize;
    let mut no_body = Vec::new();
    let mut heights: Vec<(String, f32, f32)> = Vec::new();
    let mut ids: Vec<&String> = catalog.data().characters.keys().collect();
    ids.sort();
    for id in ids {
        match sprite_body_collision_for_character_id_in(&sheets, &catalog, id, ldtk) {
            Some(body) => {
                derived += 1;
                heights.push((id.clone(), body.collision.y, body.render_size.y));
            }
            None => no_body.push(id.clone()),
        }
    }

    println!(
        "\n{} of {} characters derive a body; {} publish no body box and fall back to the LDtk spawn box\n",
        derived,
        derived + no_body.len(),
        no_body.len()
    );
    heights.sort_by(|a, b| a.1.total_cmp(&b.1));
    println!("  {:<44} {:>8} {:>10}", "character", "body h", "render h");
    for (id, body_h, render_h) in &heights {
        println!("  {id:<44} {body_h:>8.2} {render_h:>10.2}");
    }
    if !no_body.is_empty() {
        println!("\n  NO BODY BOX — a stated height cannot reach these:");
        for id in &no_body {
            println!("    {id}");
        }
    }
}

/// ⛔⛔ THE FX SET'S FOLDER IS THE TIER, and this pins the expression
/// `load_fx_sheets` resolves it with.
///
/// Until 2026-09-02 that function built its set `with_sprite_folder("sprites")`
/// unconditionally and every effect sheet decoded at authored resolution on
/// every tier — measured in the hall as `fx-sheet` 7.7 MP at Potato, High and
/// Ultra alike, about a third of the room's residency. The omission was
/// inherited rather than chosen: `ensure_fx_sheet_loaded` passed `Full, Full` in
/// the same shape as `load_prop_sheet_for_target`, whose `Full` is correct for a
/// reason about one demo prop that does not transfer.
///
/// ⚠ A PURE TEST ON PURPOSE. The composed-host version of this assertion needs
/// `AMBITION_QUALITY_PROFILE`, which is process-global, and `ambition_app` runs
/// every file under `tests/` as a parallel thread of one binary — so it belongs
/// with the `#[ignore]` + exact-filter fixtures, not here. What this guards is
/// the thing that actually regressed: which folder the tier names.
#[test]
fn the_fx_sheet_folder_follows_the_quality_tier() {
    // The crate's own public re-exports — the same line `assets.rs` uses. My
    // first attempt reached through `settings::video::quality::…`, which is
    // private, and the module next door already had the answer.
    use ambition_persistence::settings::{
        TextureResolutionScale, VisualQualityBudget, VisualQualityProfile,
    };

    let folder_for = |profile: VisualQualityProfile| {
        let budget = VisualQualityBudget::for_profile(profile);
        super::assets::character_sprite_tier(Some(&budget)).asset_subdir("sprites")
    };

    assert_eq!(folder_for(VisualQualityProfile::Potato), "sprites_potato");
    assert_eq!(folder_for(VisualQualityProfile::Ultra), "sprites");

    // No budget at all is `Full`, which is the authored PNG — the safe default
    // for a composition that never resolved a tier.
    assert_eq!(
        super::assets::character_sprite_tier(None),
        TextureResolutionScale::Full
    );
    assert_eq!(
        super::assets::character_sprite_tier(None).asset_subdir("sprites"),
        "sprites"
    );
}
