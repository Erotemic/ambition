//! Tests for `sheets`: that `spec_from_record` prefers manifest-authored
//! `tuning:` over the passed-in Rust `SheetTuning` const.

use super::*;
use crate::character::CharacterAnimator;

#[test]
fn a_sheet_with_sit_rows_enters_and_leaves_a_conversation_pose() {
    let record: SheetRecord = ron::from_str(r#"(
        target: "social_fixture", image: "social.png", label_width: 0,
        frame_width: 16, frame_height: 16,
        rows: [
            (animation: "idle", row_index: 0, frame_count: 1, duration_ms: 100, duration_secs: 0.1),
            (animation: "sit_down", row_index: 1, frame_count: 2, duration_ms: 100, duration_secs: 0.1),
            (animation: "sit_idle", row_index: 2, frame_count: 2, duration_ms: 100, duration_secs: 0.1),
            (animation: "stand_up", row_index: 3, frame_count: 2, duration_ms: 100, duration_secs: 0.1),
            (animation: "bark", row_index: 4, frame_count: 2, duration_ms: 100, duration_secs: 0.1),
            (animation: "death", row_index: 5, frame_count: 2, duration_ms: 100, duration_secs: 0.1),
        ],
    )"#).expect("the social sheet parses");
    let asset = CharacterSpriteAsset {
        texture: Default::default(),
        layout: Default::default(),
        spec: spec_from_record(&record, &SheetTuning::new(1.0, 1)),
        pages: Vec::new(),
        requested_tier: Default::default(),
        resolved_tier: Default::default(),
    };
    let mut animator = CharacterAnimator::new(&asset);
    let request = |animator: &mut CharacterAnimator, held, bark| {
        animator.request_actor_pose(CharacterAnim::Idle, [], false, held, bark)
    };
    request(&mut animator, true, false);
    assert_eq!(animator.current, CharacterAnim::SitDown);
    animator.tick(0.21);
    request(&mut animator, true, false);
    assert_eq!(animator.current, CharacterAnim::SitIdle);
    request(&mut animator, false, false);
    assert_eq!(animator.current, CharacterAnim::StandUp);
    animator.tick(0.21);
    request(&mut animator, false, true);
    assert_eq!(animator.current, CharacterAnim::Bark);
    request(&mut animator, true, false);
    request(&mut animator, true, false);
    request(&mut animator, false, false);
    request(&mut animator, false, false);
    animator.request_actor_pose(CharacterAnim::Death, [], false, true, true);
    assert_eq!(animator.current, CharacterAnim::Death);
}

/// When the manifest has a `tuning:` block, `spec_from_record` uses it, not the
/// passed-in `SheetTuning` const.
#[test]
fn spec_from_record_prefers_manifest_tuning_when_present() {
    // Manifest tuning far from the const, so a mix-up is visible.
    let ron_text = r#"
            (
                target: "synthetic_test",
                image: "synthetic_test.png",
                label_width: 0,
                frame_width: 64,
                frame_height: 64,
                tuning: Some((
                    collision_scale: 3.7,
                    frame_sample_inset: 2,
                )),
                rows: [
                    (
                        animation: "idle",
                        row_index: 0,
                        frame_count: 1,
                        duration_ms: 100,
                        duration_secs: 0.1,
                    ),
                ],
            )
        "#;
    let record: SheetRecord = ron::from_str(ron_text).expect("synthetic record parses");
    // A very different const tuning. The manifest values must win.
    let legacy_tuning = SheetTuning::new(99.9, 99);
    let spec = spec_from_record(&record, &legacy_tuning);
    assert!(
        (spec.collision_scale - 3.7).abs() < 1e-5,
        "manifest tuning's collision_scale=3.7 should win over legacy 99.9; got {}",
        spec.collision_scale
    );
    assert_eq!(
        spec.frame_sample_inset, 2,
        "manifest tuning's frame_sample_inset=2 should win over legacy 99",
    );
}

/// When the manifest has no `tuning:` block, `spec_from_record` uses the
/// passed-in const.
#[test]
fn spec_from_record_falls_back_to_const_when_manifest_omits_tuning() {
    let ron_text = r#"
            (
                target: "synthetic_test",
                image: "synthetic_test.png",
                label_width: 0,
                frame_width: 64,
                frame_height: 64,
                rows: [
                    (
                        animation: "idle",
                        row_index: 0,
                        frame_count: 1,
                        duration_ms: 100,
                        duration_secs: 0.1,
                    ),
                ],
            )
        "#;
    let record: SheetRecord = ron::from_str(ron_text).expect("tuning-omitted record parses");
    assert!(record.tuning.is_none());
    let legacy_tuning = SheetTuning::new(2.1, 1);
    let spec = spec_from_record(&record, &legacy_tuning);
    assert!((spec.collision_scale - 2.1).abs() < 1e-5);
    assert_eq!(spec.frame_sample_inset, 1);
}

/// The quad is the body rectangle scaled onto the collision box.
/// `collision_scale` has no effect on it: two very different values must give
/// identical quads.
#[test]
fn a_published_body_sizes_the_quad_and_collision_scale_is_inert() {
    // A 40x80 body off-centre in a 100x120 frame, so a frame-based quad and a
    // body-based quad cannot match.
    let ron_text = r#"
            (
                target: "synthetic_body",
                image: "synthetic_body.png",
                label_width: 0,
                frame_width: 100,
                frame_height: 120,
                body_metrics: Some((
                    body_pixel_bbox: Some((x: 12, y: 30, w: 40, h: 80)),
                )),
                rows: [
                    (
                        animation: "idle",
                        row_index: 0,
                        frame_count: 1,
                        duration_ms: 100,
                        duration_secs: 0.1,
                    ),
                ],
            )
        "#;
    let record: SheetRecord = ron::from_str(ron_text).expect("synthetic body record parses");
    let collision = Vec2::new(20.0, 40.0);

    let timid = spec_from_record(&record, &SheetTuning::new(0.4, 1));
    let absurd = spec_from_record(&record, &SheetTuning::new(9.9, 1));
    let quad = sprite_render_size(&timid, collision);
    assert_eq!(
        quad,
        sprite_render_size(&absurd, collision),
        "collision_scale 0.4 and 9.9 produced different quads, so the field is \
         still sizing characters"
    );

    // The body fills the box exactly: 40x80 body at 0.5 world units per pixel.
    let drawn = Vec2::new(40.0 / 100.0 * quad.x, 80.0 / 120.0 * quad.y);
    assert!(
        (drawn - collision).length() < 1e-4,
        "the drawn body measured {drawn:?} inside a {collision:?} collision box"
    );
    // Uniform scale: the frame aspect is kept, so the art is not stretched.
    assert!(
        ((quad.x / 100.0) - (quad.y / 120.0)).abs() < 1e-4,
        "the quad {quad:?} scales the 100x120 frame by different amounts per axis"
    );
}

/// Sheets with no published body (`creator_lab_props`, `weird_hermit`) keep
/// the old arithmetic. This fallback is the only use of `collision_scale`.
#[test]
fn a_sheet_with_no_published_body_still_reads_collision_scale() {
    let ron_text = r#"
            (
                target: "synthetic_bodyless",
                image: "synthetic_bodyless.png",
                label_width: 0,
                frame_width: 64,
                frame_height: 64,
                rows: [
                    (
                        animation: "idle",
                        row_index: 0,
                        frame_count: 1,
                        duration_ms: 100,
                        duration_secs: 0.1,
                    ),
                ],
            )
        "#;
    let record: SheetRecord = ron::from_str(ron_text).expect("bodyless record parses");
    let collision = Vec2::new(20.0, 40.0);
    let small = sprite_render_size(
        &spec_from_record(&record, &SheetTuning::new(1.0, 1)),
        collision,
    );
    let big = sprite_render_size(
        &spec_from_record(&record, &SheetTuning::new(2.0, 1)),
        collision,
    );
    assert!((big.y - small.y * 2.0).abs() < 1e-4);
}

/// A clip resolves to its row slot, and a missing clip resolves to `None`.
///
/// `CharacterAnim` has no variants for many fighter rows (`smash_forward`,
/// `air_dodge`, `tumble`), so the authored clip name is the key.
///
/// The `None` case is the important one. `row_index_of(name).unwrap_or(0)`
/// draws row zero (idle) for a missing row, which looks like a character that
/// does not swing. The caller must get `None` and fall back to the semantic
/// pose ladder.
#[test]
fn a_clip_chain_resolves_to_a_row_slot_or_to_nothing() {
    let ron_text = r#"
            (
                target: "synthetic_fighter",
                image: "synthetic_fighter.png",
                label_width: 0,
                frame_width: 64,
                frame_height: 64,
                rows: [
                    (
                        animation: "idle",
                        row_index: 0,
                        frame_count: 1,
                        duration_ms: 100,
                        duration_secs: 0.1,
                    ),
                    (
                        animation: "attack_side",
                        row_index: 1,
                        frame_count: 3,
                        duration_ms: 100,
                        duration_secs: 0.1,
                    ),
                    (
                        animation: "slash",
                        row_index: 2,
                        frame_count: 2,
                        duration_ms: 100,
                        duration_secs: 0.1,
                    ),
                    (
                        animation: "smash_forward",
                        row_index: 3,
                        frame_count: 5,
                        duration_ms: 100,
                        duration_secs: 0.1,
                    ),
                ],
            )
        "#;
    let record: SheetRecord = ron::from_str(ron_text).expect("synthetic record parses");
    let spec = spec_from_record(&record, &SheetTuning::new(1.0, 0));

    assert_eq!(
        spec.clip_slot(["smash_forward", "attack_side", "slash"]),
        Some(3),
        "the exact authored row must win over its fallbacks"
    );
    assert_eq!(
        spec.clip_slot(["air_back", "attack_side", "slash"]),
        Some(1),
        "a sheet without the exact row falls through the AUTHORED chain, in order"
    );
    assert_eq!(
        spec.clip_slot(["air_back", "tumble", "knockdown"]),
        None,
        "a sheet with none of the chain must answer NOTHING — row zero here is \
         idle, and drawing idle for a missing attack row looks like a character \
         that does not swing"
    );

    // The slot indexes the real row, so a resolved clip can draw.
    assert_eq!(spec.row_at(3).frame_count, 5, "slot 3 is the 5-frame smash");
    assert_eq!(spec.flat_index_at(3, 2), spec.flat_index_at(3, 0) + 2);
}

/// A playing clip on a trimmed sheet is sized and anchored by the clip's row.
///
/// Both lookups clamp row and frame, so a wrong row gives a misplaced sprite,
/// not an error. Most shipped sheets are trimmed.
///
/// The two rows have very different trims, and the clip row name is unknown to
/// `CharacterAnim::from_name`. Effect sheets (`generic_action_fx`: `hit_hard`,
/// `poof_small`, `release_ring`) are addressable only by row name.
#[test]
fn a_clip_on_a_trimmed_sheet_is_measured_by_the_clip_row() {
    // `trimmed_render`, `FrameTrim`, and `CharacterSpriteAsset` come from
    // `use super::*`.
    use crate::character::{CharacterAnimator, CharacterSpritePage};

    let ron_text = r#"
        (
            target: "synthetic_fx",
            image: "synthetic_fx.png",
            label_width: 0,
            frame_width: 128,
            frame_height: 128,
            rows: [
                (animation: "idle", row_index: 0, frame_count: 2, duration_ms: 100, duration_secs: 0.1,
                 rects: [
                    (x: 0, y: 0, w: 10, h: 10, off: (59, 59)),
                    (x: 16, y: 0, w: 10, h: 10, off: (59, 59)),
                 ]),
                (animation: "hit_hard", row_index: 1, frame_count: 2, duration_ms: 40, duration_secs: 0.04,
                 rects: [
                    (x: 0, y: 32, w: 88, h: 80, off: (20, 24)),
                    (x: 96, y: 32, w: 88, h: 80, off: (20, 24)),
                 ]),
            ],
        )
    "#;
    let record: SheetRecord = ron::from_str(ron_text).expect("synthetic record parses");
    let spec = spec_from_record(&record, &SheetTuning::new(1.0, 1));

    // Both sides of the comparison are observed, so this cannot pass vacuously.
    assert!(spec.is_trimmed(), "the fixture must actually be trimmed");
    let clip_slot = spec
        .clip_slot(["hit_hard"])
        .expect("a row the pose enum does not name must still be reachable by NAME");
    let clip_trim = spec.frame_trim_at(clip_slot, 0);
    let pose_trim = spec.frame_trim(CharacterAnim::Idle, 0);
    assert_ne!(
        clip_trim, pose_trim,
        "fixture is useless unless the two rows disagree about trim"
    );

    let asset = CharacterSpriteAsset {
        texture: Default::default(),
        layout: Default::default(),
        spec: spec.clone(),
        pages: vec![CharacterSpritePage {
            texture: Default::default(),
            layout: Default::default(),
        }],
        requested_tier: Default::default(),
        resolved_tier: Default::default(),
    };
    let mut animator = CharacterAnimator::new(&asset);
    let base_size = Vec2::new(128.0, 128.0);
    let base_anchor = Vec2::new(0.0, -0.4);
    animator.ensure_render_basis(base_size, base_anchor);

    animator.request_clip(["hit_hard"], CharacterAnim::Idle);
    assert_eq!(
        animator.current_render(),
        Some(trimmed_render(&clip_trim, base_size, base_anchor)),
        "a playing clip must be measured by its OWN row, not by `current`"
    );

    // Dropping the clip returns to the pose's trim.
    animator.request(CharacterAnim::Idle);
    assert_eq!(
        animator.current_render(),
        Some(trimmed_render(&pose_trim, base_size, base_anchor)),
        "with no clip playing the semantic pose still measures the frame"
    );
}

/// Repacking a sheet does not redraw it.
///
/// The ultrapack builds its own [`SheetRecord`] from atlas rects and cannot know
/// the drawn facing. The base manifest's facing must be carried onto it, as the
/// caller carries `tuning`. Both west-drawn characters are packed at all four
/// tiers.
///
/// A tree with no regen output has no pack. The `has_baked_packs` build-script
/// cfg then marks the test `ignored` with a reason, not failed.
#[test]
#[cfg_attr(
    not(has_baked_packs),
    ignore = "this tree has no ultrapack (regen output is gitignored) — run ./scripts/regen/sprites.sh"
)]
fn a_packed_target_keeps_the_facing_its_artwork_was_drawn_in() {
    for target in ["patent_clerk", "carl_stargan"] {
        let base = record_for_sheet_key(target)
            .unwrap_or_else(|| panic!("{target}'s sheet is baked into the sheet table"));
        // Premise: without it the checks below pass vacuously.
        assert!(
            base.authored_faces_left,
            "{target}'s base manifest must declare its left-drawn artwork"
        );

        let mut tiers_checked = 0usize;
        for scale in [
            super::super::TextureResolutionScale::Full,
            super::super::TextureResolutionScale::Half,
            super::super::TextureResolutionScale::Quarter,
            super::super::TextureResolutionScale::Potato,
        ] {
            let Some((tier, spec)) = try_load_pack_spec_for_target(target, &DEFAULT_TUNING, scale)
                .map(|(spec, tier)| (tier, spec))
            else {
                continue;
            };
            tiers_checked += 1;
            assert!(
                spec.authored_faces_left(),
                "the {tier} pack dropped {target}'s drawn facing"
            );
        }
        assert!(
            tiers_checked > 0,
            "no baked pack tier resolved {target}, so this proved nothing"
        );
    }
}

/// A character's gameplay body must not depend on the graphics setting.
///
/// Each sheet is published at four tiers (full, `0_5x`, `0_25x`, `potato`),
/// each with its own `body_metrics`. `authored_body` says `body_pixel_bbox` is
/// a gameplay body, and `authored_body_pixel_size` needs it. If tiers disagree,
/// the collision box changes size with the graphics setting.
///
/// This reads the baked index, the same table the runtime reads, not the files.
#[test]
fn a_sheets_gameplay_body_does_not_depend_on_the_graphics_setting() {
    const TIERS: [&str; 3] = ["0_5x", "0_25x", "potato"];

    let index = record_index();
    let mut compared = 0usize;
    let mut disagreements: Vec<String> = Vec::new();

    for (target, full) in index.iter() {
        // Full-resolution targets only; tier keys have a suffix.
        if TIERS
            .iter()
            .any(|tier| target.ends_with(&format!(".{tier}")))
        {
            continue;
        }
        for tier in TIERS {
            let Some(reduced) = index.get(&format!("{target}.{tier}")) else {
                continue;
            };
            compared += 1;
            let claim = |record: &SheetRecord| {
                record
                    .body_metrics
                    .as_ref()
                    .is_some_and(|metrics| metrics.authored_body)
            };
            if claim(full) != claim(reduced) {
                disagreements.push(format!(
                    "{target}: full-res authored_body={} but .{tier} says {}",
                    claim(full),
                    claim(reduced),
                ));
            }
        }
    }

    // Zero floor: with no tier variants, or tier keys without suffixes, nothing
    // is compared and the test would pass.
    assert!(
        compared > 100,
        "only {compared} sheet/tier pairs were compared, so this proved almost \
         nothing: either the quality variants are missing from the baked index \
         or their target keys stopped carrying a tier suffix"
    );
    assert!(
        disagreements.is_empty(),
        "a character's gameplay body depends on the graphics setting — one road \
         was regenerated and the other was not. Re-render the full-resolution \
         sheets (`./scripts/regen/sprites.sh --target <name>`), then the tiers:\n{}",
        disagreements.join("\n")
    );
}

/// The art is drawn on its own box, not on the middle of its packed cell.
///
/// A frame is a cell sized by the widest pose, and the art sits where the crop
/// left it. An anchor `x` of zero centres the quad on the cell, so the art is
/// drawn off the collision box by the packing offset.
///
/// `FrameToBody::planting_feet` maps hitbox polygons by `(px - feet.x)`, so
/// hitboxes are measured from the body centre. The art must be measured the
/// same way, or art, hitboxes, and collision box are in different places.
///
/// A body packed near the centre passes with any anchor, so the test needs
/// sheets that are far off-centre.
#[test]
fn a_body_packed_off_centre_is_drawn_on_its_box_and_not_on_its_frame() {
    // Far off-centre and near-centre rows together. The near-centre row shows
    // the fix is a correction, not a constant shift for all bodies.
    for (target, min_offset) in [
        ("projectile_polygon", 0.10_f32),
        ("officer", 0.10),
        ("pointed_polygon", 0.0),
    ] {
        // Panic on a missing key; do not skip. A skipped row passes with the
        // defect present.
        let record = record_for_sheet_key(target).unwrap_or_else(|| {
            panic!(
                "`{target}` is not a baked sheet key, so this row asserted \
                 nothing (keys are bare names -- `projectile_polygon`, not \
                 `projectile_polygon_spritesheet`)"
            )
        });
        let metrics = record
            .body_metrics
            .as_ref()
            .unwrap_or_else(|| panic!("{target} publishes body metrics"));
        let feet = metrics
            .feet_pixel
            .unwrap_or_else(|| panic!("{target} publishes a feet pixel"));
        let frame_w = record.frame_width.max(1) as f32;
        // The body centre as a fraction of the frame. This is the value in
        // `feet_anchor_norm.x`, recomputed from the feet pixel so the test does
        // not read back the field under test.
        let want = feet.x / frame_w - 0.5;
        assert!(
            want.abs() >= min_offset,
            "{target} is only {:.1}% off its frame centre, so it cannot tell a \
             body-centred anchor from a frame-centred one — this row was chosen \
             to be the case that can fail",
            want.abs() * 100.0,
        );

        let spec = spec_from_record(record, &SheetTuning::default());
        let collision = Vec2::new(40.0, 80.0);
        let anchor =
            feet_anchor_for_render_size(&spec, collision, sprite_render_size(&spec, collision));
        // The tolerance admits only authoring rounding. Some sheets store a
        // `feet_anchor_norm.x` that differs from the feet pixel in the third
        // decimal (the officer: -0.250000 against -0.248466). `0.01` is well
        // above that drift and well below the defect.
        assert!(
            (anchor.0.x - want).abs() < 0.01,
            "{target}: the sprite anchors at x={:.6} but its body sits at x={:.6} \
             of the frame — the quad is centred on the packed cell, so the art is \
             drawn {:.1}% of a {frame_w}px frame away from the box that represents it",
            anchor.0.x,
            want,
            (anchor.0.x - want).abs() * 100.0,
        );
    }
}

/// Split baked sheet keys by the tier marker that
/// `build.rs::baked_key_for_path` appends. Return the bare roots under each
/// marker (`""` for the base tier).
///
/// Split on the last dot, and only for the three known markers. A root can
/// contain dots, and a key that only looks like it has a marker is a base-tier
/// sheet.
fn tier_partition<'a>(
    keys: impl Iterator<Item = &'a str>,
) -> std::collections::BTreeMap<&'a str, std::collections::BTreeSet<&'a str>> {
    let mut out: std::collections::BTreeMap<&str, std::collections::BTreeSet<&str>> =
        std::collections::BTreeMap::new();
    for key in keys {
        let (root, marker) = ["0_5x", "0_25x", "potato"]
            .into_iter()
            .find_map(|marker| {
                key.strip_suffix(marker)
                    .and_then(|head| head.strip_suffix('.').map(|root| (root, marker)))
            })
            .unwrap_or((key, ""));
        out.entry(marker).or_default().insert(root);
    }
    out
}

/// The Bevy resource's index and [`record_index`] are the same map, built twice
/// over the same baked table.
///
/// `attack_hitbox::warm_file_root_registry` says the two are different
/// registries. [`record_index`] says it calls [`crate::index_baked_table`] so
/// they agree. This test checks it: if the indices diverge, it fails.
///
/// Records are compared through `{:?}` because [`crate::SheetRecord`] does not
/// derive `PartialEq`. `BodyMetrics::animations` and `FrameRect::anchors` are
/// `BTreeMap`s so `Debug` output has a stable order; a `HashMap` gives false
/// differences.
///
/// NaN: `{:?}` prints `NaN`, so two NaNs compare equal as strings. This is
/// more lenient than `PartialEq`, which is safe: both indices parsed the same
/// RON, and agreement is what the test checks.
#[test]
fn the_registry_and_the_record_index_are_one_map_built_twice() {
    let index = record_index();
    let registry =
        crate::SheetRegistry::from_baked_table(crate::baked_sheet_rons::BAKED_SHEET_RONS);

    // Anti-vacuity check first: two empty maps are equal.
    //
    // `build.rs` bakes four sibling directories into one key space and tags
    // three with `.0_5x` / `.0_25x` / `.potato`. A correctly globbed tree has
    // the same names in all four tiers. A glob that lost a directory, matched
    // a different tree, or caught a tier mid-publish fails this. The check
    // follows the roster, not a fixed count.
    let tiers = tier_partition(index.keys().map(String::as_str));
    let base = tiers.get("").cloned().unwrap_or_default();
    for marker in ["0_5x", "0_25x", "potato"] {
        let variant = tiers.get(marker).cloned().unwrap_or_default();
        let unpublished: Vec<&str> = base.difference(&variant).copied().collect();
        let orphaned: Vec<&str> = variant.difference(&base).copied().collect();
        assert!(
            unpublished.is_empty() && orphaned.is_empty(),
            "the `{marker}` tier does not publish the same sheets as the base tier: \
             {} in the base tier with no `{marker}` variant (e.g. {:?}), {} `{marker}` \
             variants with no base sheet (e.g. {:?}). `build.rs` globs four sibling \
             directories into one key space, so four tiers holding the same names is \
             how a correctly-globbed tree states itself -- a glob that matched a \
             different tree, lost a directory, or caught a tier mid-publish breaks \
             here.",
            unpublished.len(),
            &unpublished[..unpublished.len().min(5)],
            orphaned.len(),
            &orphaned[..orphaned.len().min(5)],
        );
    }

    // A uniform halving keeps the tiers in agreement, so also check a size.
    // 150 is below the roster's sheet count and well above half of it. Derive
    // the value from the roster (`scripts/regen/sprites.sh`), not this machine.
    assert!(
        base.len() > 150,
        "the base tier holds {} sheet(s), below the floor of 150 -- the publish \
         roster claims 175, so either the baked table did not compile in (every \
         comparison below would pass vacuously) or the glob matched a truncated \
         tree. `find crates/ambition_platformer2d_actor_monolith/assets/sprites \
         -name '*_spritesheet.ron' | wc -l` is the one-line check.",
        base.len(),
    );
    assert_eq!(
        registry.len(),
        index.len(),
        "the registry and the record index disagree about how many sheets exist \
         before a single key is compared -- same vacuity, other side",
    );

    // Check both directions. A subset test passes when one side drops keys.
    let missing: Vec<&str> = index
        .keys()
        .filter(|key| registry.get(key).is_none())
        .map(String::as_str)
        .collect();
    let extra: Vec<&str> = registry
        .iter()
        .map(|(key, _)| key)
        .filter(|key| !index.contains_key(*key))
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "the two indices over one baked table disagree on WHICH sheets exist.\n  \
         {} key(s) only in record_index: {:?}\n  \
         {} key(s) only in the registry: {:?}\n  \
         ⇒ they are not the same map, and whichever caller holds the smaller one \
         is answering lookups it should not.",
        missing.len(),
        missing.iter().take(8).collect::<Vec<_>>(),
        extra.len(),
        extra.iter().take(8).collect::<Vec<_>>(),
    );

    // Compare records, not only keys: the same key can answer a different
    // record (one index once gave `tech_bro_disruptor`'s page for `robot`).
    let mut differing: Vec<String> = Vec::new();
    for (key, mine) in index.iter() {
        let theirs = registry.get(key).expect("key sets agreed above");
        if format!("{mine:?}") != format!("{theirs:?}") {
            differing.push(key.clone());
        }
    }
    assert!(
        differing.is_empty(),
        "{} key(s) resolve to DIFFERENT records in the two indices (first {:?}) \
         — same keys, different answers, which is worse than a missing key \
         because every caller believes it looked the sheet up correctly",
        differing.len(),
        differing.iter().take(8).collect::<Vec<_>>(),
    );
}

/// A control for the comparison in
/// `the_registry_and_the_record_index_are_one_map_built_twice`.
///
/// On a checkout with no published sheets, that test fails its floor before
/// the comparisons run. This synthetic two-record table exercises them.
#[test]
fn the_index_comparison_can_tell_two_tables_apart() {
    // The record has a populated `animations` map with keys in unsorted
    // order. Empty maps print the same under any order, so they could not
    // catch a map with unstable iteration order.
    fn one(target: &str, image: &str) -> String {
        format!(
            "[(target: \"{target}\", image: \"{image}\", label_width: 0, \
              frame_width: 64, frame_height: 64, rows: [], \
              body_metrics: Some((animations: {{ \
                \"side_sweep\": (), \"rest\": (), \"floor_slam\": (), \"idle\": (), \
                \"hit\": () }})))]"
        )
    }
    let a_text = one("alpha", "alpha.png");
    let b_text = one("beta", "beta.png");
    let same = crate::SheetRegistry::from_baked_table(&[
        ("alpha", a_text.as_str()),
        ("beta", b_text.as_str()),
    ]);
    let twin = crate::SheetRegistry::from_baked_table(&[
        ("alpha", a_text.as_str()),
        ("beta", b_text.as_str()),
    ]);
    assert_eq!(same.len(), 2, "premise: the synthetic table indexes");
    // Floor on the fixture: if the parser dropped `body_metrics`, the maps
    // would be empty and this control would check nothing.
    assert!(
        same.get("alpha")
            .and_then(|r| r.body_metrics.as_ref())
            .is_some_and(|m| m.animations.len() >= 5),
        "the synthetic record did not parse its `animations` map, so this \
         control is back to comparing empty maps — which is what it exists to \
         stop doing"
    );

    // Same inputs agree: no false differences.
    for (key, record) in same.iter() {
        assert_eq!(
            format!("{record:?}"),
            format!("{:?}", twin.get(key).expect("the twin holds every key")),
            "{key}: identical tables must produce identical records"
        );
    }

    // Different inputs disagree: the comparison can fail.
    let changed = one("alpha", "DIFFERENT.png");
    let other = crate::SheetRegistry::from_baked_table(&[("alpha", changed.as_str())]);
    assert_ne!(
        format!("{:?}", same.get("alpha").expect("premise")),
        format!("{:?}", other.get("alpha").expect("premise")),
        "a record whose image differs must compare UNEQUAL, or the {{:?}} proxy \
         is blind and the sibling test proves nothing"
    );
    let missing: Vec<&str> = same
        .iter()
        .map(|(key, _)| key)
        .filter(|k| other.get(k).is_none())
        .collect();
    assert_eq!(
        missing,
        ["beta"],
        "the key-set comparison must notice a key the other side lacks"
    );
}
