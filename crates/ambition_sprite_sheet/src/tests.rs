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
    // The boss hurtbox is split into head + body parts so the player must aim at the central body
    // (not extended arms).
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

/// ⭐⭐ **TWO SHEETS DECLARING ONE RIG TARGET EACH KEEP THEIR OWN PAGE.**
///
/// ⛔⛔ this test used to assert the OPPOSITE, and it AGREED WITH THE BUG: it was
/// called `two_manifests_claiming_one_target_with_different_geometry_still_resolve`
/// and its body checked that last-wins picked the second file, calling that
/// *"preserved"*. Under target keying one of the two sheets was simply
/// unreachable — in the shipped table `robot` lost its own 256x256 page to
/// `tech_bro_disruptor`. Keyed by FILE ROOT (§19) neither wins, because they were
/// never competing: a rig target is which adapter DREW a sheet, and 48 sheets
/// share five of them.
#[test]
fn two_manifests_declaring_one_rig_target_each_keep_their_own_page() {
    let bess = r#"[(target: "pirate_heavy", image: "bess.png", label_width: 100,
        frame_width: 172, frame_height: 138,
        rows: [(animation: "side", row_index: 0, frame_count: 1, duration_ms: 60, duration_secs: 0.06,
                rects: [(x: 0, y: 0, w: 172, h: 138)])])]"#;
    let broadside = r#"[(target: "pirate_heavy", image: "broadside.png", label_width: 100,
        frame_width: 319, frame_height: 250,
        rows: [(animation: "side", row_index: 0, frame_count: 1, duration_ms: 60, duration_secs: 0.06,
                rects: [(x: 0, y: 0, w: 319, h: 250)])])]"#;
    // Sorted as `build.rs` sorts them.
    let table: &[(&str, &str)] = &[
        ("pirate_heavy_bess", bess),
        ("pirate_heavy_broadside", broadside),
    ];
    let registry = SheetRegistry::from_baked_table(table);

    let first = registry
        .get("pirate_heavy_bess")
        .expect("the first sheet resolves by its own file root");
    assert_eq!(
        (first.frame_width, first.frame_height),
        (172, 138),
        "the shared rig target must not cost this sheet its own page",
    );
    let second = registry
        .get("pirate_heavy_broadside")
        .expect("the second sheet resolves by its own file root");
    assert_eq!((second.frame_width, second.frame_height), (319, 250));

    // The rig target is not a key at all — that is the whole ruling.
    assert!(
        registry.get("pirate_heavy").is_none(),
        "a renderer target string must not be a durable engine identity",
    );
    // Nothing shadowed anything: they never shared a key.
    assert!(registry.shadowed_targets().is_empty());
    // ⭐ AND THE TWO IDENTITIES BOTH SURVIVE, which is the point of keeping them
    // in separate fields: the KEY is how you ask for this sheet, the TARGET is
    // still the rig that drew it. Assigning the key used to overwrite the rig
    // target, so the shared authoring fact was destroyed to produce the lookup.
    assert_eq!(first.key, "pirate_heavy_bess");
    assert_eq!(first.target, "pirate_heavy");
    assert_eq!(second.key, "pirate_heavy_broadside");
    assert_eq!(second.target, "pirate_heavy");
}

/// A quality-variant RON (`sprites_potato/…`, baked as `<root>.potato` by
/// `build.rs::baked_key_for_path`) must not answer a request for the full-res
/// base: a consumer that cropped the full-res PNG with 8px potato rects drew a
/// mis-cropped dark strip.
///
/// ⭐ **under file-root keying this holds BY CONSTRUCTION**, and that is the
/// point of the row: `slash` and `slash.potato` are different keys, so the
/// variant no longer has to be SKIPPED to keep it away from the base. The old
/// build dropped every variant record on the floor because every tier of
/// `robot_slash` carries the identical `target: "robot_slash"` — a hazard that
/// belonged to target keying and left with it.
///
/// Deterministic: hand-built table (the real `BAKED_SHEET_RONS` only carries
/// variant rows when the gitignored `sprites_*x/` folders exist locally, so a
/// registry-level assertion would silently pass in CI).
#[test]
fn a_quality_variant_answers_only_its_own_key() {
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
        "the full-res base answers its own root, not the 8px potato variant"
    );
    assert_eq!(record.frame_height, 118);

    // ⭐ the variant is REACHABLE now rather than dropped — the resolution-pair
    // loader asks for exactly this key.
    let variant = registry
        .get("slash.potato")
        .expect("the variant keeps its own key");
    assert_eq!((variant.frame_width, variant.frame_height), (8, 8));
}

/// A target packed into a SHARED pack references a sparse subset of the
/// pack's pages, and the loader must load that subset — not the range
/// `0..page_count`.
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
/// They came here with the join: they name no catalog and no character, only
/// [`BodyMetrics::body_pixel_extent`], and it had no tests of its own in the crate that defines it.
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

/// A file root that names several records does not resolve to the first one —
/// each record keeps its own target key instead.
///
/// ⭐ the pair of assertions is the whole rule: the ROOT is refused (it names all
/// eight props and therefore none) while the RECORDS stay reachable, which is
/// why `creator_lab_props`' eight props survived the move to file-root keying.
#[test]
fn a_multi_record_file_root_is_refused_while_its_records_keep_their_targets() {
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
    let reg = SheetRegistry::from_baked_table(&[("solo", one.as_str()), ("props", two.as_str())]);

    // The single-record root still resolves — the refusal is narrow.
    assert!(
        reg.get("solo").is_some(),
        "a single-record file root must still be indexed by its file root"
    );

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

    // ⭐ and the records themselves are still there. A packed atlas is the ONE
    // case where `record.target` is the key, because the file root cannot be.
    assert!(
        reg.get("first_prop").is_some() && reg.get("second_prop").is_some(),
        "refusing the ambiguous ROOT must not drop the records it names — that \
         would make eight lab props unreachable"
    );
}

/// ⭐⭐ **THE THREE SHEETS THAT USED TO LOSE THEIR OWN PAGE KEEP IT.**
///
/// Measured on the real baked table 2026-08-19, target-keyed: `robot` lost its
/// own 256x256 page to `tech_bro_disruptor` (215x256), `goblin` lost 239x253 to
/// `ranged_skirmisher` (235x229), `sandbag` lost 128x128 to
/// `sandbag_full_review` (256x256). All three are file roots of their own, so
/// under §19's keying each answers with its own geometry and the usurper answers
/// with its.
///
/// ⚠ **the skip is falsifiable, deliberately.** These sheets are generated art
/// and gitignored, so a checkout that never ran regen bakes an empty table —
/// which is why a bare `if let Some(..)` here would be a check that cannot fail.
/// If no pair is present the test instead asserts the table really is art-less,
/// so "nothing to check" has to be TRUE rather than merely convenient.
#[test]
fn a_shared_rig_target_no_longer_costs_a_sheet_its_own_page() {
    let reg = SheetRegistry::from_baked_table(baked_sheet_rons::BAKED_SHEET_RONS);

    // (file root, its own frame size, the sheet that used to take the key)
    let pairs = [
        (
            "robot",
            (256u32, 256u32),
            "tech_bro_disruptor",
            (215u32, 256u32),
        ),
        ("goblin", (239, 253), "ranged_skirmisher", (235, 229)),
        ("sandbag", (128, 128), "sandbag_full_review", (256, 256)),
    ];

    let mut checked = 0usize;
    for (root, own, usurper, usurper_frame) in pairs {
        let (Some(mine), Some(theirs)) = (reg.get(root), reg.get(usurper)) else {
            continue;
        };
        assert_eq!(
            (mine.frame_width, mine.frame_height),
            own,
            "`{root}` answered with someone else's page — the rig target is \
             acting as a durable identity again",
        );
        assert_eq!(
            (theirs.frame_width, theirs.frame_height),
            usurper_frame,
            "`{usurper}` must keep its own page too; both are real characters \
             and neither is a stale manifest",
        );
        checked += 1;
    }

    if checked == 0 {
        assert!(
            reg.is_empty(),
            "the baked table has sheets but none of the three known collision \
             roots — re-measure rather than letting this test go quiet",
        );
    }
}

/// The refusal reaches the real baked table without taking the one key this
/// index exists to answer.
///
///  it deliberately does NOT assert that no CHARACTER was refused, though
/// an earlier draft's name claimed to: this crate has no catalog, so it cannot
/// tell a packed prop atlas from a character's sheet — the same gap
/// `shadowed_targets` documents. That assertion lives in `ambition_app`, which
/// owns the catalog. A test whose name promises a check its body does not make
/// is worse than no test.
#[test]
fn the_real_baked_table_still_resolves_the_players_variant() {
    let reg = SheetRegistry::from_baked_table(baked_sheet_rons::BAKED_SHEET_RONS);
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

/// ⭐⭐ **EVERY SHIPPED SHEET IS REACHABLE, AND NONE OF THEM SHADOWS ANOTHER.**
///
/// The keying ruling (file root; a packed atlas keys each member by its own
/// name) exists to make this true, and before it the shipped table had THIRTY
/// NINE shadowed targets, three of them real: `robot`'s own 256×256 page lost to
/// `robot_archivist`, `goblin`'s to `goblin_brute_hammer`, `sandbag`'s to
/// `sandbag_armored_review` — a 128px sheet cropped on a 256px grid. All 848
/// keys are now claimed once.
///
/// ⛔ THIS CAN STILL GO RED, which is why it is worth asserting rather than
/// assuming: the two spellings share ONE namespace. A packed atlas whose member
/// is named for an existing file root — a second props sheet with a `robot`
/// member, say — collides, and the survivor crops the loser's image with the
/// wrong grid. That is the day to give the key its `product::member` spelling;
/// until then there is exactly one packed product in the tree and building the
/// spelling for it would be machinery.
#[test]
fn no_shipped_sheet_key_is_claimed_twice() {
    let registry = crate::baked_sheet_registry();
    assert!(
        registry.len() > 500,
        "the baked table came up nearly empty ({}), so this measured nothing",
        registry.len(),
    );
    let shadowed: Vec<String> = registry
        .shadowed_targets()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(
        shadowed.is_empty(),
        "a shipped sheet key is claimed by two manifests with different frame \
         geometry, so one of them is unreachable and the other crops with the \
         wrong grid:\n{}",
        shadowed.join("\n"),
    );
}
