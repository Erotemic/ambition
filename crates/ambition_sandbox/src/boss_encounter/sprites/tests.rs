use super::*;

#[test]
fn boss_sheet_has_seven_animation_rows() {
    // The enum has 7 variants and the spec has 7 rows; if these
    // ever drift, indexing by `anim as usize` would panic at
    // runtime.
    assert_eq!(BOSS_SHEET.rows.len(), 7);
}

#[test]
fn fsm_and_trex_sheets_match_their_published_layouts() {
    // FSM: 7 PNG rows (169×150), every BossAnim used once. These match
    // flying_spaghetti_monster_boss_spritesheet.yaml; a drift here means
    // the boss renders frames from the wrong row.
    assert_eq!(FLYING_SPAGHETTI_MONSTER_SHEET.rows.len(), 7);
    assert_eq!(FLYING_SPAGHETTI_MONSTER_SHEET.frame_width, 169);
    assert_eq!(FLYING_SPAGHETTI_MONSTER_SHEET.frame_height, 150);
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
        FLYING_SPAGHETTI_MONSTER_SHEET.flat_index(BossAnim::Rest, 0),
        0
    );
    assert_eq!(
        FLYING_SPAGHETTI_MONSTER_SHEET.flat_index(BossAnim::FloorSlam, 0),
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
        TREX_BOSS_SHEET.flat_index(BossAnim::SideSweep, 0),
        6 + 8 + 8
    );

    // Both atlases build without panic and have one rect per frame.
    let fsm_frames: usize = FLYING_SPAGHETTI_MONSTER_SHEET
        .rows
        .iter()
        .map(|(_, r)| r.frame_count)
        .sum();
    assert_eq!(
        FLYING_SPAGHETTI_MONSTER_SHEET.build_atlas().len(),
        fsm_frames
    );
    let trex_frames: usize = TREX_BOSS_SHEET
        .rows
        .iter()
        .map(|(_, r)| r.frame_count)
        .sum();
    assert_eq!(TREX_BOSS_SHEET.build_atlas().len(), trex_frames);
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
    assert_eq!(BOSS_SHEET.flat_index(BossAnim::Rest, 0), 0);
    assert_eq!(BOSS_SHEET.flat_index(BossAnim::FloorSlam, 0), 8);
    assert_eq!(BOSS_SHEET.flat_index(BossAnim::SideSweep, 0), 8 + 7);
    assert_eq!(BOSS_SHEET.flat_index(BossAnim::SpikeHalo, 0), 8 + 7 + 7);
}

#[test]
fn flat_index_clamps_to_last_frame_of_row() {
    // Asking for frame index past the end of a row clamps to the
    // last valid frame; this avoids out-of-bounds atlas reads when
    // an animation cursor overshoots due to a long delta-t.
    let last_rest = BOSS_SHEET.flat_index(BossAnim::Rest, 999);
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
fn is_boss_kind_only_true_for_boss_variant() {
    assert!(is_boss_kind(FeatureVisualKind::Boss));
    assert!(!is_boss_kind(FeatureVisualKind::Enemy));
    assert!(!is_boss_kind(FeatureVisualKind::Hazard));
    assert!(!is_boss_kind(FeatureVisualKind::Chest));
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
