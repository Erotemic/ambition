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

/// **Two manifests, one target, DIFFERENT frame geometry — the Broadside Bess
/// shape.** The survivor crops the loser's image with the wrong grid, and the
/// registry used to do it silently.
///
/// ⚠ the assertion is about the SURVIVOR, not about the log line: a test that
/// captured the warning would be checking that a scanner sees its own fixture.
/// What matters is that last-wins still resolves (so the seventeen legitimate
/// archetype-sharers keep working) while the geometry mismatch is the condition
/// a human is told about.
#[test]
fn two_manifests_claiming_one_target_with_different_geometry_still_resolve() {
    let stale = r#"[(target: "pirate_heavy_broadside_bess", image: "bess.png", label_width: 100,
        frame_width: 172, frame_height: 138,
        rows: [(animation: "side", row_index: 0, frame_count: 1, duration_ms: 60, duration_secs: 0.06,
                rects: [(x: 0, y: 0, w: 172, h: 138)])])]"#;
    let live = r#"[(target: "pirate_heavy_broadside_bess", image: "bess.png", label_width: 100,
        frame_width: 319, frame_height: 250,
        rows: [(animation: "side", row_index: 0, frame_count: 1, duration_ms: 60, duration_secs: 0.06,
                rects: [(x: 0, y: 0, w: 319, h: 250)])])]"#;
    // Sorted as `build.rs` sorts them: the stale `pirate_heavy_…` root first.
    let table: &[(&str, &str)] = &[
        ("pirate_heavy", stale),
        ("pirate_heavy_broadside_bess", live),
    ];
    let registry = SheetRegistry::from_baked_table(table);
    let record = registry
        .get("pirate_heavy_broadside_bess")
        .expect("the target resolves");
    assert_eq!(
        (record.frame_width, record.frame_height),
        (319, 250),
        "last-wins is preserved — the warn is a diagnosis, not a policy change",
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

/// A target packed into a SHARED pack references a sparse subset of the
/// pack's pages, and the loader must load that subset — not the range
/// `0..page_count`.
///
/// This is the boot-time regression that made one decorative prop pull in
/// every page of the ultrapack: its frames sit on pages 4 and 53, so
/// `page_count()` reports 54 and a `0..54` load decoded ~221 megapixels of
/// pages nothing would ever sample. `used_pages()` reports exactly `{4, 53}`.
#[test]
fn packed_target_uses_only_the_pages_its_frames_reference() {
    let ron_text = r#"
    [(
        target: "intro_cart",
        image: "ultrapack_0.png",
        images: ["ultrapack_0.png", "ultrapack_4.png", "ultrapack_53.png"],
        label_width: 0,
        frame_width: 64,
        frame_height: 64,
        rows: [
            (animation: "idle", row_index: 0, frame_count: 2, duration_ms: 120, duration_secs: 0.12,
             rects: [(x: 0, y: 0, w: 64, h: 64, page: 4), (x: 64, y: 0, w: 64, h: 64, page: 53)]),
        ],
    )]
    "#;
    let records: Vec<SheetRecord> =
        ron::from_str(ron_text).expect("packed SheetRecord should deserialize");
    let record = &records[0];
    // The count is still the high-water mark — that contract is unchanged.
    assert_eq!(record.page_count(), 54);
    // The LOAD SET is what shrank.
    let used: Vec<u32> = record.used_pages().into_iter().collect();
    assert_eq!(
        used,
        vec![4, 53],
        "a packed target must not claim the 52 pages between its frames"
    );
}

/// The sparse-page path must not change the common case: a dedicated sheet
/// owns contiguous pages, so its used set IS `0..page_count` and the loader
/// behaves exactly as before.
#[test]
fn dedicated_sheet_uses_every_page_it_counts() {
    let ron_text = r#"
    [(
        target: "huge_boss",
        image: "huge_boss_spritesheet.png",
        images: ["huge_boss_spritesheet.png", "huge_boss_spritesheet.1.png"],
        label_width: 100,
        frame_width: 384,
        frame_height: 529,
        rows: [
            (animation: "idle", row_index: 0, frame_count: 1, duration_ms: 120, duration_secs: 0.12,
             rects: [(x: 100, y: 0, w: 384, h: 529)]),
            (animation: "charge", row_index: 1, frame_count: 1, duration_ms: 90, duration_secs: 0.09,
             page: 1,
             rects: [(x: 100, y: 0, w: 384, h: 529, page: 1)]),
        ],
    )]
    "#;
    let records: Vec<SheetRecord> = ron::from_str(ron_text).expect("should deserialize");
    let record = &records[0];
    let used: Vec<u32> = record.used_pages().into_iter().collect();
    assert_eq!(used, vec![0, 1]);
    assert_eq!(used.len() as u32, record.page_count());
}

/// An unpacked multi-page row carries its page on the ROW, with no per-rect
/// page. Falling back to `row.page` only when rects are absent is what keeps
/// that layout loading its pages at all.
#[test]
fn unpacked_rows_fall_back_to_the_row_page() {
    let ron_text = r#"
    [(
        target: "legacy",
        image: "legacy.png",
        images: ["legacy.png", "legacy.1.png", "legacy.2.png"],
        label_width: 0,
        frame_width: 32,
        frame_height: 32,
        rows: [
            (animation: "idle", row_index: 0, frame_count: 1, duration_ms: 100, duration_secs: 0.1,
             page: 2, rects: []),
        ],
    )]
    "#;
    let records: Vec<SheetRecord> = ron::from_str(ron_text).expect("should deserialize");
    let used: Vec<u32> = records[0].used_pages().into_iter().collect();
    assert_eq!(
        used,
        vec![2],
        "row.page is authoritative when rects are empty"
    );
}

/// A second, different declaration for one sheet target is rejected rather than
/// resolving by plugin order.
#[test]
fn two_providers_cannot_silently_claim_one_sheet_target() {
    use crate::character::sheets::AuthoredSheets;

    fn one_record(target: &str, image: &str) -> String {
        format!(
            r#"[(
                target: "{target}",
                image: "{image}",
                label_width: 0,
                frame_width: 32,
                frame_height: 32,
                rows: [
                    (animation: "idle", row_index: 0, frame_count: 1,
                     duration_ms: 100, duration_secs: 0.1, page: 0, rects: []),
                ],
            )]"#
        )
    }

    let mut sheets = AuthoredSheets::default();
    let first = one_record("duelist", "first.png");
    assert_eq!(
        sheets.insert_ron("duelist", &first),
        Ok(1),
        "the first claim on a free target is indexed"
    );

    // Same file, same bytes: a plugin built twice has not made a decision.
    assert_eq!(
        sheets.insert_ron("duelist", &first),
        Ok(0),
        "re-registering the identical declaration is idempotent, not a conflict"
    );
    assert_eq!(
        sheets.get("duelist").map(|record| record.image.as_str()),
        Some("first.png"),
    );

    let error = sheets
        .insert_ron("duelist", &one_record("duelist", "second.png"))
        .expect_err("a second, DIFFERENT claim on one target must be refused");
    assert!(
        error.contains("duelist") && error.contains("claimed twice"),
        "the refusal must name the target and both claimants: {error}"
    );
    assert_eq!(
        sheets.get("duelist").map(|record| record.image.as_str()),
        Some("first.png"),
        "the refused claim must not have displaced the held one"
    );
}

/// A multi-record sheet whose LAST record collides must not leave its earlier
/// records installed under an error return — a provider told "rejected" and
/// handed a half-populated registry is worse off than one told nothing.
#[test]
fn a_refused_multi_record_sheet_indexes_none_of_its_records() {
    use crate::character::sheets::AuthoredSheets;

    fn pack(targets: [&str; 2]) -> String {
        let body: Vec<String> = targets
            .iter()
            .map(|target| {
                format!(
                    r#"(
                        target: "{target}",
                        image: "pack.png",
                        label_width: 0,
                        frame_width: 32,
                        frame_height: 32,
                        rows: [
                            (animation: "idle", row_index: 0, frame_count: 1,
                             duration_ms: 100, duration_secs: 0.1, page: 0, rects: []),
                        ],
                    )"#
                )
            })
            .collect();
        format!("[{}]", body.join(","))
    }

    let mut sheets = AuthoredSheets::default();
    sheets
        .insert_ron("held", &pack(["taken", "other"]))
        .expect("the first pack indexes cleanly");

    sheets
        .insert_ron("rival", &pack(["fresh", "taken"]))
        .expect_err("the pack collides on its second record");

    assert!(
        sheets.get("fresh").is_none(),
        "the record BEFORE the collision must not survive a refused file"
    );
}

/// `BodyMetrics` carrying only a static body rectangle, for the extent tests
/// below. `animations` is empty on purpose: these fixtures are about the
/// measured path, which is what a sheet publishing no per-pose hurtbox falls
/// back to.
///
/// ⚠ these three tests lived beside the catalog join in
/// `ambition_platformer2d_actor_monolith` until 2026-08-09, reaching the method
/// through a one-line private wrapper. They came here with the join: they name
/// no catalog and no character, only [`BodyMetrics::body_pixel_extent`], and it
/// had no tests of its own in the crate that defines it.
fn metrics_with_bbox(bbox: Option<PixelRect>, parts: Vec<NamedPixelRect>) -> BodyMetrics {
    BodyMetrics {
        body_pixel_bbox: bbox,
        body_pixel_parts: parts,
        animations: Default::default(),
        feet_pixel: None,
        feet_anchor_norm: None,
        authored_body: false,
    }
}

#[test]
fn body_extent_prefers_single_bbox_when_no_parts() {
    let m = metrics_with_bbox(
        Some(PixelRect {
            x: 8,
            y: 5,
            w: 106,
            h: 83,
        }),
        vec![],
    );
    assert_eq!(
        m.body_pixel_extent(character::CharacterAnim::Idle),
        Some((106.0, 83.0))
    );
}

#[test]
fn body_extent_bounds_disjoint_parts() {
    // Two parts at x∈[0,32] and x∈[96,128], y∈[40,90] → bbox 128 × 50.
    let m = metrics_with_bbox(
        // bbox present but ignored: parts win for disjoint bodies.
        Some(PixelRect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        }),
        vec![
            NamedPixelRect {
                name: "left".into(),
                x: 0,
                y: 40,
                w: 32,
                h: 50,
                poly: Vec::new(),
            },
            NamedPixelRect {
                name: "right".into(),
                x: 96,
                y: 40,
                w: 32,
                h: 50,
                poly: Vec::new(),
            },
        ],
    );
    assert_eq!(
        m.body_pixel_extent(character::CharacterAnim::Idle),
        Some((128.0, 50.0))
    );
}

#[test]
fn body_extent_rejects_degenerate_box() {
    let m = metrics_with_bbox(
        Some(PixelRect {
            x: 0,
            y: 0,
            w: 0,
            h: 10,
        }),
        vec![],
    );
    assert_eq!(m.body_pixel_extent(character::CharacterAnim::Idle), None);
}

/// **A file root that names several records is refused, not truncated to the
/// first one.**
///
/// ⛔ the bug this pins is not a wrong answer, it is an answer that depends on
/// the packer's emission order — and the packer re-runs per quality tier, which
/// is exactly the kind of "the tier changed and my character changed" symptom
/// this repo has spent days on. Keeping `records[0]` made that a coin flip that
/// happens to land the same way today.
#[test]
fn a_multi_record_file_root_is_refused_rather_than_silently_truncated() {
    let rec = |t: &str, img: &str| {
        format!(
            r#"(target: "{t}", image: "{img}", label_width: 10, frame_width: 10,
                frame_height: 10, rows: [(animation: "idle", row_index: 0,
                frame_count: 1, duration_ms: 100, duration_secs: 0.1,
                rects: [(x: 0, y: 0, w: 10, h: 10)])])"#
        )
    };
    let one = format!("[{}]", rec("solo", "solo.png"));
    let two = format!(
        "[{},{}]",
        rec("first_prop", "props.png"),
        rec("second_prop", "props.png")
    );
    let reg = SheetRegistry::from_baked_table_by_file_root(&[
        ("solo", one.as_str()),
        ("props", two.as_str()),
    ]);

    // The single-record root still resolves — the refusal is narrow.
    assert!(
        reg.get("solo").is_some(),
        "a single-record file root must still be indexed by its file root"
    );

    // ⭐ THE POISON: before this change `reg.get("props")` returned
    // `first_prop`, and nothing anywhere said which of the two it was.
    assert!(
        reg.get("props").is_none(),
        "the ambiguous root resolved to a record — this is the truncation bug: \
         whichever record the packer emitted first silently became `props`"
    );

    let refused = reg.ambiguous_file_roots();
    assert_eq!(refused.len(), 1, "expected exactly one refused root");
    assert_eq!(refused[0].file_root, "props");
    assert_eq!(
        refused[0].targets,
        vec!["first_prop".to_string(), "second_prop".to_string()],
        "the refusal must carry EVERY target, so a caller with a catalog can \
         tell a packed prop atlas from a character it needs to resolve"
    );
}

/// The refusal reaches the real baked table without taking the one key this
/// index exists to answer.
///
/// ⛔ **it deliberately does NOT assert that no CHARACTER was refused**, though
/// an earlier draft's name claimed to: this crate has no catalog, so it cannot
/// tell a packed prop atlas from a character's sheet — the same gap
/// `shadowed_targets` documents. That assertion lives in `ambition_app`, which
/// owns the catalog. A test whose name promises a check its body does not make
/// is worse than no test.
#[test]
fn the_real_baked_table_still_resolves_the_players_variant() {
    let reg = SheetRegistry::from_baked_table_by_file_root(baked_sheet_rons::BAKED_SHEET_RONS);
    // The player's variant is the reason this index exists at all.
    assert!(
        reg.get("player_robot_v3").is_some(),
        "`player_robot_v3` must stay resolvable by file root — the target-keyed \
         registry cannot tell it from the enemy `robot`"
    );
    for refused in reg.ambiguous_file_roots() {
        assert!(
            refused.targets.len() > 1,
            "a root was refused without being ambiguous: {refused}"
        );
    }
}
