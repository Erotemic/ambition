//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

/// A split (multi-page) sheet round-trips: the generator emits an
/// `images: [...]` list and a `page:` per row, with each page's rects in
/// that page's own pixel space. Regressing the `#[serde(default)]` on
/// either field would silently collapse every row onto page 0 and address
/// the wrong texture.
#[test]
fn multi_page_sheet_round_trips() {
    let ron_text = r#"
    [(
        target: "huge_boss",
        image: "huge_boss_spritesheet.png",
        images: ["huge_boss_spritesheet.png", "huge_boss_spritesheet.1.png"],
        label_width: 100,
        frame_width: 384,
        frame_height: 529,
        rows: [
            (animation: "idle", row_index: 0, frame_count: 2, duration_ms: 120, duration_secs: 0.12,
             rects: [(x: 100, y: 0, w: 384, h: 529), (x: 484, y: 0, w: 384, h: 529)]),
            (animation: "charge", row_index: 1, frame_count: 1, duration_ms: 90, duration_secs: 0.09,
             page: 1,
             rects: [(x: 100, y: 0, w: 384, h: 529)]),
        ],
    )]
    "#;
    let records: Vec<SheetRecord> =
        ron::from_str(ron_text).expect("multi-page SheetRecord should deserialize");
    let record = &records[0];
    assert_eq!(record.page_count(), 2);
    assert_eq!(record.page_image(0), "huge_boss_spritesheet.png");
    assert_eq!(record.page_image(1), "huge_boss_spritesheet.1.png");
    // Out-of-range page falls back to the primary image.
    assert_eq!(record.page_image(9), "huge_boss_spritesheet.png");
    assert_eq!(record.rows[0].page, 0, "idle defaults to page 0");
    assert_eq!(record.rows[1].page, 1, "charge lives on page 1");
    // The two rows share y=0 because each page is its own coordinate space.
    assert_eq!(record.rows[0].rects[0].y, 0);
    assert_eq!(record.rows[1].rects[0].y, 0);
}

/// An alpha-trimmed frame round-trips its `off` (trim offset within the
/// logical frame). Frames without `off` default to `(0, 0)` = untrimmed, so
/// pre-packer RON stays byte-identical.
#[test]
fn trimmed_frame_offset_round_trips() {
    let ron_text = r#"
    [(
        target: "packed",
        image: "packed_spritesheet.png",
        label_width: 0,
        frame_width: 384,
        frame_height: 529,
        rows: [
            (animation: "idle", row_index: 0, frame_count: 2, duration_ms: 120, duration_secs: 0.12,
             rects: [
                (x: 2, y: 2, w: 180, h: 420, off: (100, 80)),
                (x: 190, y: 2, w: 175, h: 410),
             ]),
        ],
    )]
    "#;
    let records: Vec<SheetRecord> =
        ron::from_str(ron_text).expect("trimmed SheetRecord should deserialize");
    let row = &records[0].rows[0];
    assert_eq!(
        row.rects[0].off,
        (100, 80),
        "trimmed frame keeps its offset"
    );
    assert_eq!(
        row.rects[1].off,
        (0, 0),
        "frame without `off` defaults to untrimmed"
    );
    // The stored rect is the TRIMMED size, smaller than the logical frame.
    assert!(row.rects[0].w < records[0].frame_width as i32);
}

/// A legacy single-page sheet (no `images`, no `page`) still parses and
/// reports one page addressing the single `image`.
#[test]
fn single_page_sheet_defaults_to_one_page() {
    let ron_text = r#"
    [(
        target: "goblin",
        image: "goblin_spritesheet.png",
        label_width: 0,
        frame_width: 128,
        frame_height: 128,
        rows: [
            (animation: "idle", row_index: 0, frame_count: 1, duration_ms: 120, duration_secs: 0.12,
             rects: [(x: 0, y: 0, w: 128, h: 128)]),
        ],
    )]
    "#;
    let records: Vec<SheetRecord> =
        ron::from_str(ron_text).expect("single-page SheetRecord should deserialize");
    let record = &records[0];
    assert_eq!(record.page_count(), 1);
    assert_eq!(record.page_image(0), "goblin_spritesheet.png");
    assert_eq!(record.rows[0].page, 0);
}

/// The Python renderer emits `body_metrics.animations` as a
/// map keyed by animation name. This test pins that the
/// Rust deserializer reads it back — regressing this would
/// silently fall back to the legacy `body_pixel_bbox`
/// (cyan box stays at idle-pose size during attacks).
#[test]
fn body_metrics_animations_round_trip_from_renderer_emit() {
    // Matches the shape emitted by `_ron_anim_metrics_map` in the
    // renderer's `core/manifest_ron.py` (the unified RON emitter) for the boss.
    let ron_text = r#"
    (
        body_pixel_bbox: Some((x: 8, y: 5, w: 106, h: 83)),
        feet_pixel: Some((x: 60.5, y: 87.0)),
        feet_anchor_norm: Some((x: -0.02734375, y: -0.1796875)),
        animations: {
            "rest": (hurtbox: Some((bbox: Some((x: 8, y: 4, w: 106, h: 84))))),
            "floor_slam": (
                hurtbox: Some((bbox: Some((x: 5, y: 0, w: 111, h: 110)))),
                hitbox: Some((bbox: Some((x: 4, y: 88, w: 120, h: 30))))
            ),
            "side_sweep": (
                hurtbox: Some((bbox: Some((x: 1, y: 5, w: 127, h: 86)))),
                hitbox: Some((parts: [
                    (name: "left", x: 0, y: 40, w: 32, h: 50),
                    (name: "right", x: 96, y: 40, w: 32, h: 50)
                ]))
            )
        }
    )
    "#;
    let metrics: BodyMetrics =
        ron::from_str(ron_text).expect("BodyMetrics should deserialize from renderer-emitted RON");

    assert_eq!(metrics.animations.len(), 3);
    let rest = metrics.animations.get("rest").expect("`rest` present");
    let rest_hurt = rest.hurtbox.as_ref().expect("`rest` hurtbox");
    assert!(rest_hurt.bbox.is_some(), "rest hurtbox has bbox");
    assert!(rest.hitbox.is_none(), "rest has no hitbox (idle pose)");

    let floor = metrics
        .animations
        .get("floor_slam")
        .expect("`floor_slam` present");
    let floor_hit = floor.hitbox.as_ref().expect("`floor_slam` hitbox");
    let bbox = floor_hit.bbox.expect("floor_slam hitbox bbox");
    assert_eq!(bbox.w, 120);
    assert_eq!(bbox.h, 30);

    let sweep = metrics
        .animations
        .get("side_sweep")
        .expect("`side_sweep` present");
    let sweep_hit = sweep.hitbox.as_ref().expect("`side_sweep` hitbox");
    assert_eq!(
        sweep_hit.parts.len(),
        2,
        "side_sweep has left + right parts"
    );
    assert_eq!(sweep_hit.parts[0].name, "left");
    assert_eq!(sweep_hit.parts[1].name, "right");
}

/// Verify the actual on-disk boss sheet RON parses. If the
/// Python renderer + Rust schema ever drift this test catches
/// it on the spot rather than at runtime via a silent
/// "animations: empty" fallback.
#[test]
fn live_boss_spritesheet_ron_round_trips() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/sprites/boss_spritesheet.ron");
    if !path.exists() {
        // Sprites are gitignored; if a clean checkout hasn't
        // regenerated yet, skip rather than fail.
        return;
    }
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let records: Vec<SheetRecord> =
        ron::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let record = records
        .into_iter()
        .find(|r| r.target == "boss")
        .expect("boss record");
    let metrics = record.body_metrics.expect("body_metrics");
    assert!(
        !metrics.animations.is_empty(),
        "expected per-animation metadata in boss_spritesheet.ron — \
         check that the Python renderer emitted `animations:` and that \
         this test is reading the regenerated file"
    );
    // Spot-check the floor_slam hitbox (adapter-declared) so a
    // future renderer change that drops author-declared hitboxes
    // trips this guard.
    let floor_slam = metrics
        .animations
        .get("floor_slam")
        .expect("floor_slam animation present");
    assert!(
        floor_slam.hitbox.is_some(),
        "floor_slam should have an authored hitbox (boss adapter declares it)"
    );
    // The boss hurtbox is split into head + body parts so the
    // player must aim at the central body (not extended arms).
    // Pin both parts come through so a renderer regression that
    // drops `hurtbox_parts` reverts to the loose single-bbox
    // alpha hurtbox.
    let rest = metrics.animations.get("rest").expect("rest animation");
    let rest_hurt = rest.hurtbox.as_ref().expect("rest hurtbox");
    assert!(
        !rest_hurt.parts.is_empty(),
        "rest hurtbox must be the multi-part head + body override (parts empty implies the adapter's hurtbox_parts was lost)"
    );
    let part_names: Vec<&str> = rest_hurt.parts.iter().map(|p| p.name.as_str()).collect();
    assert!(
        part_names.contains(&"head") && part_names.contains(&"body"),
        "rest hurtbox parts must include 'head' and 'body'; got {part_names:?}"
    );
    // SideSweep should also have head + body hurtbox parts (not
    // a single bbox that would include the extended arms).
    let sweep = metrics
        .animations
        .get("side_sweep")
        .expect("side_sweep animation");
    let sweep_hurt = sweep.hurtbox.as_ref().expect("side_sweep hurtbox");
    assert!(
        sweep_hurt.parts.len() >= 2,
        "side_sweep hurtbox must be multi-part; got {} parts",
        sweep_hurt.parts.len()
    );
}

/// A quality-variant RON (`sprites_potato/…`, baked as `<root>.potato` by
/// `build.rs::baked_key_for_path`) must NOT clobber the full-res base in the
/// target-keyed `SheetRegistry`. Every resolution variant of a sheet carries the
/// IDENTICAL `record.target`, so a naive last-write-wins insert left
/// `get("robot_slash")` returning the 8px potato frames — and any consumer that
/// crops the full-res PNG with those tiny rects rendered a mis-cropped dark strip
/// (the "translucent black box" slash-VFX bug, 2026-07-12). The base must win.
///
/// Deterministic: hand-built table (the real `BAKED_SHEET_RONS` only carries
/// variant rows when the gitignored `sprites_*x/` folders exist locally, so a
/// registry-level assertion would silently pass in CI). Sorted order puts the
/// base (`"slash"`) before the variant (`"slash.potato"`), so a target-keyed
/// last-write-wins would otherwise pick potato — exactly the bug.
#[test]
fn quality_variant_records_do_not_clobber_the_base_registry() {
    let base = r#"[(target: "slash", image: "slash_spritesheet.png", label_width: 100,
        frame_width: 116, frame_height: 118,
        rows: [(animation: "side", row_index: 0, frame_count: 1, duration_ms: 60, duration_secs: 0.06,
                rects: [(x: 100, y: 0, w: 116, h: 118)])])]"#;
    let potato = r#"[(target: "slash", image: "slash_spritesheet.png", label_width: 7,
        frame_width: 8, frame_height: 8,
        rows: [(animation: "side", row_index: 0, frame_count: 1, duration_ms: 60, duration_secs: 0.06,
                rects: [(x: 1, y: 1, w: 5, h: 6)])])]"#;
    let table: &[(&str, &str)] = &[("slash", base), ("slash.potato", potato)];
    let registry = SheetRegistry::from_baked_table(table);
    let record = registry.get("slash").expect("base record present");
    assert_eq!(
        record.frame_width, 116,
        "full-res base must win, not the 8px potato variant"
    );
    assert_eq!(record.frame_height, 118);
    // The variant is not smuggled in under a suffixed key either.
    assert!(registry.get("slash.potato").is_none());
    // Sanity: the marker classifier agrees on which roots are variants.
    assert!(is_quality_variant_file_root("slash.potato"));
    assert!(is_quality_variant_file_root("robot_slash.0_5x"));
    assert!(!is_quality_variant_file_root("slash"));
}
