//! Tests pinning each boss sheet's row count + frame dimensions to its published layout.

use super::*;

/// Page-local flat atlas index via the shared frame algebra over the const's
/// grid-only synthetic record — the same path the runtime takes when no
/// published sheet RON exists.
fn const_flat(spec: &BossSheetSpec, anim: BossAnim, frame: usize) -> usize {
    spec.synth_record("x.png")
        .flat_index_in_page(spec.record_row(anim), frame)
}

/// Number of atlas cells the const's grid produces through the shared algebra.
fn const_atlas_len(spec: &BossSheetSpec) -> usize {
    spec.synth_record("x.png")
        .atlas_page(0, spec.frame_sample_inset)
        .rects
        .len()
}

#[test]
fn boss_sheet_render_basis_diverges_from_the_baked_sheet_dims() {
    // Archetype swap AS4b decision pin (fable AD3), turned into a standing
    // characterization guard. The boss RENDER draws at `spec.render_size(kin.size)`
    // where the loaded spec's frame dims are overwritten from the BAKED sheet record;
    // gameplay's const `render_size` uses the CONST dims. `render_size` height is
    // collision-scale-only, so only the WIDTH (frame aspect fw/fh) can diverge.
    //
    // This documents WHY AS4b stores the seed render-basis on the boss and lets the
    // render keep its own `spec.render_size(seed)` (byte-identical), instead of
    // routing render onto a const-derived size — the const and baked aspects do NOT
    // match for real bosses, so a const-derived `ActorRenderSize` would resize the
    // sprite. Convergence (render + hurtbox on one true size) is a separate blind
    // slice per AD3's "latent bug to fix regardless"; this guard fails loudly if the
    // gap ever CLOSES (at which point the const-derived path becomes safe and this
    // note is stale).
    let known_divergent = [
        ("boss", &BOSS_SHEET),
        ("mockingbird_boss", &MOCKINGBIRD_SHEET),
    ];
    let mut any_divergent = false;
    for (target, spec) in known_divergent {
        let Some(record) = crate::character_sprites::record_for_target(target) else {
            continue;
        };
        if spec.frame_width as u64 * record.frame_height as u64
            != spec.frame_height as u64 * record.frame_width as u64
        {
            any_divergent = true;
        }
    }
    assert!(
        any_divergent,
        "const vs baked sheet aspects now AGREE — the AS4b seed-render-basis workaround \
         may be replaceable with a const-derived ActorRenderSize; revisit E33/AD3.",
    );
}

#[test]
fn boss_sheet_has_seven_animation_rows() {
    // The enum has 7 variants and the spec has 7 rows; if these
    // ever drift, indexing by `anim as usize` would panic at
    // runtime.
    assert_eq!(BOSS_SHEET.rows.len(), 7);
}

#[test]
fn fsm_and_trex_sheets_match_their_published_layouts() {
    // FSM: 7 PNG rows, every BossAnim used once. The row mapping (not the exact
    // pixel dims, which are now a fallback the published RON overrides) is what
    // a drift here would corrupt — the boss would render frames from the wrong
    // row.
    assert_eq!(FLYING_SPAGHETTI_MONSTER_SHEET.rows.len(), 7);
    assert_eq!(FLYING_SPAGHETTI_MONSTER_SHEET.frame_width, 393);
    assert_eq!(FLYING_SPAGHETTI_MONSTER_SHEET.frame_height, 344);
    assert_eq!(
        FLYING_SPAGHETTI_MONSTER_SHEET.frame_count(BossAnim::Rest),
        6
    );
    assert_eq!(
        FLYING_SPAGHETTI_MONSTER_SHEET.frame_count(BossAnim::Death),
        8
    );
    assert!(FLYING_SPAGHETTI_MONSTER_SHEET.body_centered, "FSM floats");
    // Rest is row 0; FloorSlam (meatball_volley) is row 3 → 6+8+7 frames before.
    assert_eq!(
        const_flat(&FLYING_SPAGHETTI_MONSTER_SHEET, BossAnim::Rest, 0),
        0
    );
    assert_eq!(
        const_flat(&FLYING_SPAGHETTI_MONSTER_SHEET, BossAnim::FloorSlam, 0),
        6 + 8 + 7
    );

    // T-Rex: 9 PNG rows (398×320); tail_swipe/stomp reuse SideSweep/FloorSlam
    // labels but every physical row is still listed so the atlas stays aligned.
    assert_eq!(TREX_BOSS_SHEET.rows.len(), 9);
    assert_eq!(TREX_BOSS_SHEET.frame_width, 398);
    assert_eq!(TREX_BOSS_SHEET.frame_height, 320);
    assert!(!TREX_BOSS_SHEET.body_centered, "T-Rex is grounded");
    assert_eq!(TREX_BOSS_SHEET.frame_count(BossAnim::Rest), 6);
    // SideSweep (bite) is row 3, not the later tail_swipe dup at row 5.
    assert_eq!(
        const_flat(&TREX_BOSS_SHEET, BossAnim::SideSweep, 0),
        6 + 8 + 8
    );

    // Both atlases build without panic and have one rect per frame.
    let fsm_frames: usize = FLYING_SPAGHETTI_MONSTER_SHEET
        .rows
        .iter()
        .map(|(_, r)| r.frame_count)
        .sum();
    assert_eq!(const_atlas_len(&FLYING_SPAGHETTI_MONSTER_SHEET), fsm_frames);
    let trex_frames: usize = TREX_BOSS_SHEET
        .rows
        .iter()
        .map(|(_, r)| r.frame_count)
        .sum();
    assert_eq!(const_atlas_len(&TREX_BOSS_SHEET), trex_frames);
}

fn fsm_record(frame_w: u32, frame_h: u32, label_w: u32) -> ambition_sprite_sheet::SheetRecord {
    use ambition_sprite_sheet::{FrameRect, SheetRow};
    // Mirror the FSM const's PNG-order rows + frame counts, laid out as the
    // generator emits: a uniform grid `label + col*fw, row*fh` (border 0).
    let counts = [
        ("idle", 6u32),
        ("drift", 8),
        ("noodle_whip", 7),
        ("meatball_volley", 7),
        ("eye_beam", 7),
        ("hurt", 4),
        ("death", 8),
    ];
    let rows = counts
        .iter()
        .enumerate()
        .map(|(row_idx, (name, n))| SheetRow {
            animation: (*name).to_string(),
            row_index: row_idx as u32,
            frame_count: *n,
            duration_ms: 100,
            duration_secs: 0.1,
            page: 0,
            rects: (0..*n)
                .map(|col| FrameRect {
                    x: (label_w + col * frame_w) as i32,
                    y: (row_idx as u32 * frame_h) as i32,
                    w: frame_w as i32,
                    h: frame_h as i32,
                    page: 0,
                    off: (0, 0),
                    anchors: Default::default(),
                })
                .collect(),
        })
        .collect();
    ambition_sprite_sheet::SheetRecord {
        target: "flying_spaghetti_monster_boss".to_string(),
        image: "flying_spaghetti_monster_boss_spritesheet.png".to_string(),
        images: Vec::new(),
        label_width: label_w,
        frame_width: frame_w,
        frame_height: frame_h,
        y_offset: 0,
        body_metrics: None,
        tuning: None,
        rows,
    }
}

#[test]
fn boss_ron_target_strips_the_sheet_suffix() {
    assert_eq!(
        boss_ron_target("sprites/flying_spaghetti_monster_boss_spritesheet.png"),
        Some("flying_spaghetti_monster_boss")
    );
    assert_eq!(
        boss_ron_target("sprites/gnu_ton_boss/gnu_ton_boss_spritesheet.png"),
        Some("gnu_ton_boss")
    );
    // GNU-ton's split body/hands textures resolve back to the ONE packed
    // `gnu_ton_boss` record (they share an atlas layout, lockstep-packed).
    assert_eq!(
        boss_ron_target("sprites/gnu_ton_boss/gnu_ton_boss_body_spritesheet.png"),
        Some("gnu_ton_boss")
    );
    assert_eq!(
        boss_ron_target("sprites/gnu_ton_boss/gnu_ton_boss_hands_spritesheet.png"),
        Some("gnu_ton_boss")
    );
}

#[test]
fn gnu_ton_baked_record_drives_the_split_layers_packed() {
    // End-to-end convergence guard: the regenerated `gnu_ton_boss` sheet is a
    // lockstep alpha-trim/packed sheet (one shared atlas layout for the
    // full/body/hands textures). The published record must (a) line up with the
    // const so it drives the pixels, (b) be trimmed, and (c) stay single-page
    // (the split-layer record carries one image per layer — multi-page siblings
    // would resolve the wrong layer's filename).
    let record = crate::character_sprites::record_for_target("gnu_ton_boss")
        .expect("baked gnu_ton_boss record present (run regen_sprites.sh)");
    assert!(
        record_aligns_with_const(record, &GNU_TON_SHEET),
        "packed gnu_ton record lines up with the const → drives body + hands pixels"
    );
    assert!(
        record.is_trimmed(),
        "gnu_ton sheet is alpha-trimmed/packed, not a raw grid"
    );
    assert_eq!(
        record.page_count(),
        1,
        "split-layer lockstep record must stay on one page"
    );
}

#[test]
fn boss_atlas_tracks_the_published_rects_not_the_const_grid() {
    // The flashing bug: the boss atlas was a uniform grid recomputed from the
    // const's frame_width, so when the regenerated sheet's cells changed size
    // the boss indexed the wrong pixels. The data-driven path must lay cells out
    // at the PUBLISHED rect stride. Use a deliberately DIFFERENT frame width
    // (300) from the const so a grid-from-const would land cells elsewhere.
    let record = fsm_record(300, 280, 100);
    assert!(
        record_aligns_with_const(&record, &FLYING_SPAGHETTI_MONSTER_SHEET),
        "aligned record drives the pixels"
    );
    let page = record.atlas_page(0, FLYING_SPAGHETTI_MONSTER_SHEET.frame_sample_inset);
    // One rect per frame (47 total for the FSM row set).
    assert_eq!(page.rects.len(), 6 + 8 + 7 + 7 + 7 + 4 + 8);
    // drift (row 1) frame 0 starts at label(100) + 0*300 = 100 on x and
    // 1*280 = 280 on y — proving the layout follows the record's 300/280 stride,
    // NOT the const's 393/344 grid.
    let drift_frame0 = page.rects[6]; // first frame after idle's 6
    assert_eq!(
        drift_frame0.min.y,
        280 + FLYING_SPAGHETTI_MONSTER_SHEET.frame_sample_inset
    );
}

#[test]
fn boss_atlas_falls_back_when_record_rows_dont_line_up() {
    // A record with fewer rows than the const can't be addressed in the const's
    // flat_index order, so the helper declines (caller uses the const grid).
    use ambition_sprite_sheet::SheetRow;
    let mut record = fsm_record(393, 344, 100);
    record.rows.truncate(3);
    assert!(!record_aligns_with_const(
        &record,
        &FLYING_SPAGHETTI_MONSTER_SHEET
    ));
    // A row with too few frames also declines.
    let mut record = fsm_record(393, 344, 100);
    record.rows[0] = SheetRow {
        frame_count: 1,
        rects: record.rows[0].rects[..1].to_vec(),
        ..record.rows[0].clone()
    };
    assert!(!record_aligns_with_const(
        &record,
        &FLYING_SPAGHETTI_MONSTER_SHEET
    ));
}

#[test]
fn mockingbird_flips_to_face_the_player_unlike_right_facing_sheets() {
    // Regression for "the mockingbird always faces away from you": its
    // sheet is authored facing left, so the flip must be inverted vs a
    // normal right-facing sheet. Player to the right ⇒ facing > 0.
    assert!(MOCKINGBIRD_SHEET.authored_faces_left);
    assert!(!BOSS_SHEET.authored_faces_left);
    assert!(!GNU_TON_SHEET.authored_faces_left);
    assert!(!SMIRKING_BEHEMOTH_SHEET.authored_faces_left);

    // Right-facing sheet: face right (no flip) when the player is right,
    // flip when the player is left — the unchanged default.
    assert!(!BOSS_SHEET.flip_x(1.0));
    assert!(BOSS_SHEET.flip_x(-1.0));

    // Left-authored mockingbird: inverted, so it still faces the player.
    assert!(
        MOCKINGBIRD_SHEET.flip_x(1.0),
        "player on the right ⇒ flip so the left-drawn bird faces right"
    );
    assert!(
        !MOCKINGBIRD_SHEET.flip_x(-1.0),
        "player on the left ⇒ no flip, bird faces left toward them"
    );
}

#[test]
fn mockingbird_anchor_keeps_body_inside_aabb() {
    // Mockingbird is body_centered, so collision_anchor must return
    // the spec's feet_anchor_y verbatim (no half_collision_y boost).
    // Concrete repro: with the old collision_anchor the bird hung
    // ~half its render height below the AABB.
    let aabb = Vec2::new(150.0, 185.0);
    let anchor = MOCKINGBIRD_SHEET.collision_anchor(aabb);
    assert!(MOCKINGBIRD_SHEET.body_centered);
    // feet_anchor_y is small (slight downward offset), nowhere
    // near +0.5 (which is what the feet-delta term would push it
    // to for this AABB / scale combo).
    assert!(anchor.0.y.abs() < 0.20, "anchor.y = {}", anchor.0.y);
}

#[test]
fn boss_sheet_anchor_adds_feet_delta_when_not_body_centered() {
    // The gradient sentinel keeps the feet-on-floor anchoring:
    // collision_anchor adds half_collision_y / render_height to
    // feet_anchor_y. Pin the additive behavior — if body_centered
    // accidentally flips to true here, the sprite would slide
    // half its render height down.
    assert!(!BOSS_SHEET.body_centered);
    let aabb = Vec2::new(60.0, 80.0);
    let anchor = BOSS_SHEET.collision_anchor(aabb);
    let render_h = aabb.x.max(aabb.y).max(8.0) * BOSS_SHEET.collision_scale;
    let expected = BOSS_SHEET.feet_anchor_y + (aabb.y * 0.5) / render_h;
    assert!(
        (anchor.0.y - expected).abs() < 1e-4,
        "expected {} got {}",
        expected,
        anchor.0.y
    );
    // And the additive term must be non-trivial (not a no-op).
    assert!((anchor.0.y - BOSS_SHEET.feet_anchor_y).abs() > 0.05);
}

#[test]
fn mockingbird_sheet_maps_six_rows_with_passthrough_for_missing() {
    // The mockingbird sheet ships hover/thrust/bite/slash/hit/death,
    // mapped onto the existing BossAnim vocabulary. SideSweep is
    // intentionally absent — `resolve_anim` must fall back to Rest
    // so a schedule that asks for SideSweep doesn't panic the
    // indexer.
    assert_eq!(MOCKINGBIRD_SHEET.rows.len(), 6);
    assert_eq!(
        MOCKINGBIRD_SHEET.resolve_anim(BossAnim::SideSweep),
        BossAnim::Rest
    );
    // The mapped rows resolve to themselves.
    for anim in [
        BossAnim::Rest,
        BossAnim::DashEcho,
        BossAnim::FloorSlam,
        BossAnim::SpikeHalo,
        BossAnim::Hit,
        BossAnim::Death,
    ] {
        assert_eq!(MOCKINGBIRD_SHEET.resolve_anim(anim), anim);
    }
}

#[test]
fn frame_count_matches_spec_rows() {
    assert_eq!(BOSS_SHEET.frame_count(BossAnim::Rest), 8);
    assert_eq!(BOSS_SHEET.frame_count(BossAnim::FloorSlam), 7);
    assert_eq!(BOSS_SHEET.frame_count(BossAnim::Death), 8);
}

#[test]
fn flat_index_lays_rows_end_to_end() {
    // First frame of each row sits at the cumulative sum of prior
    // frame counts. The first row starts at 0.
    assert_eq!(const_flat(&BOSS_SHEET, BossAnim::Rest, 0), 0);
    assert_eq!(const_flat(&BOSS_SHEET, BossAnim::FloorSlam, 0), 8);
    assert_eq!(const_flat(&BOSS_SHEET, BossAnim::SideSweep, 0), 8 + 7);
    assert_eq!(const_flat(&BOSS_SHEET, BossAnim::SpikeHalo, 0), 8 + 7 + 7);
}

#[test]
fn flat_index_clamps_to_last_frame_of_row() {
    // Asking for frame index past the end of a row clamps to the
    // last valid frame; this avoids out-of-bounds atlas reads when
    // an animation cursor overshoots due to a long delta-t.
    let last_rest = const_flat(&BOSS_SHEET, BossAnim::Rest, 999);
    assert_eq!(last_rest, BOSS_SHEET.frame_count(BossAnim::Rest) - 1);
}

#[test]
fn render_size_preserves_frame_aspect_ratio() {
    // BOSS_SHEET is 128x128 (square) → width / height = 1.
    let size = BOSS_SHEET.render_size(Vec2::new(50.0, 50.0));
    assert!((size.x - size.y).abs() < 1e-3);
}

#[test]
fn render_size_floors_at_minimum_extent() {
    // collision_scale * max(min_extent, 8.0): collision smaller
    // than 8 should still produce a visible quad.
    let size = BOSS_SHEET.render_size(Vec2::new(2.0, 2.0));
    assert!(size.y >= 8.0 * BOSS_SHEET.collision_scale - 1e-3);
}

#[test]
fn gnu_ton_sheet_has_six_rows() {
    assert_eq!(GNU_TON_SHEET.rows.len(), 6);
}

#[test]
fn gnu_ton_sheet_is_body_centered() {
    // body_centered:true is required so the man (at top of frame)
    // is placed at the entity transform origin rather than the
    // GNU's hooves (at the bottom of frame).
    assert!(GNU_TON_SHEET.body_centered);
}

#[test]
fn gnu_ton_anchor_is_above_sprite_center() {
    // feet_anchor_y > 0 means the entity position is above the
    // sprite center — placing the man (upper frame) at entity pos.
    assert!(
        GNU_TON_SHEET.feet_anchor_y > 0.0,
        "feet_anchor_y should be positive for GNU-ton (man at top), got {}",
        GNU_TON_SHEET.feet_anchor_y
    );
    // Should not be so large that the man falls outside the frame.
    assert!(
        GNU_TON_SHEET.feet_anchor_y < 0.5,
        "feet_anchor_y too large, would place entity at sprite top edge"
    );
}

#[test]
fn gnu_ton_side_sweep_resolves_to_itself() {
    assert_eq!(
        GNU_TON_SHEET.resolve_anim(BossAnim::SideSweep),
        BossAnim::SideSweep
    );
}

/// C6 fixture-vs-const: the content-authored `boss_sheets.ron` deserializes to
/// sheet specs BYTE-IDENTICAL to the engine's built-in demo-boss defaults. This
/// is what makes the install safe — content owns the data, but the shipped
/// bosses render unchanged until someone deliberately edits a row.
#[test]
fn boss_sheets_ron_matches_builtin_defaults() {
    let registry = BossSheetRegistry::from_ron(include_str!(
        "../../../../../game/ambition_content/assets/data/boss_sheets.ron"
    ));
    for (key, builtin) in super::builtin_boss_sheets() {
        let authored = registry
            .get(&key)
            .unwrap_or_else(|| panic!("boss_sheets.ron is missing the built-in key {key:?}"));
        assert_eq!(
            *authored, builtin,
            "authored sheet for {key:?} drifted from the built-in default"
        );
    }
}

/// C6: a content-authored sheet REPLACES the built-in for that key — the whole
/// point of the override seam. Uses the registry directly (not the process-global
/// install) so the test carries no global state.
#[test]
fn an_authored_sheet_overrides_the_built_in_layout() {
    let ron = r#"{
        "mockingbird": (
            label_width: 0,
            frame_width: 999,
            frame_height: 111,
            rows: [(Rest, (frame_count: 3, duration_secs: 0.1))],
            collision_scale: 2.0,
            feet_anchor_y: 0.0,
            frame_sample_inset: 1,
            body_centered: false,
            authored_faces_left: false,
        ),
    }"#;
    let registry = BossSheetRegistry::from_ron(ron);
    let over = registry
        .get("mockingbird")
        .expect("authored mockingbird override");
    assert_eq!(over.frame_width, 999, "override frame_width takes effect");
    assert_ne!(
        over.frame_width, MOCKINGBIRD_SHEET.frame_width,
        "the override differs from the built-in default"
    );
    assert_eq!(over.rows.len(), 1, "override authors its own row set");
}
