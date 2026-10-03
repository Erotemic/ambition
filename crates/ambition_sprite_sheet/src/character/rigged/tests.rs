use super::*;

const FIXTURE: &str = r#"(
    schema_version: 1,
    target: "fixture",
    pages: ["fixture_parts.png"],
    frame_size: (20, 30),
    feet_pixel: (10.0, 29.0),
    parts: [
        (name: "a", page: 0, rect: (0, 0, 4, 5), pivot: (2.0, 2.5)),
        (name: "b", page: 0, rect: (5, 0, 3, 3), pivot: (0.0, 0.0)),
    ],
    clips: {
        "idle": (frame_duration_s: 0.1, frames: [
            [(part: 0, at: (-1.0, -20.0), rotation: 0.5, scale: (1.0, 0.9))],
            [(part: 1, at: (0.0, -2.0), rotation: 0.0, scale: (1.0, 1.0)),
             (part: 0, at: (-1.0, -21.0), rotation: 0.0, scale: (1.0, 1.0))],
        ]),
    },
)"#;

#[test]
fn a_published_flipbook_keeps_its_draw_order_and_holds_its_last_frame() {
    let asset = RiggedSpriteAsset::from_published_ron(FIXTURE).expect("the fixture parses");
    assert_eq!(asset.max_draws(), 2);
    assert_eq!(asset.draw_count(), 3);
    assert_eq!(asset.parts[0].rect, URect::new(0, 0, 4, 5));
    let second = asset.frame("idle", 1).unwrap();
    assert_eq!(second.iter().map(|d| d.part).collect::<Vec<_>>(), vec![1, 0]);
    assert_eq!(asset.frame("idle", 9), Some(second), "past the end holds the last frame");
    assert_eq!(asset.frame("walk", 0), None);
}

#[test]
fn a_flipbook_that_names_what_it_does_not_have_is_refused() {
    let bad_part = FIXTURE.replace("(part: 1,", "(part: 7,");
    assert!(matches!(
        RiggedSpriteAsset::from_published_ron(&bad_part),
        Err(RiggedSpriteError::MissingPart { part: 7, .. })
    ));
    let bad_page = FIXTURE.replace("page: 0, rect: (5", "page: 3, rect: (5");
    assert!(matches!(
        RiggedSpriteAsset::from_published_ron(&bad_page),
        Err(RiggedSpriteError::MissingPage { part: 1, page: 3 })
    ));
    let future = FIXTURE.replace("schema_version: 1", "schema_version: 4");
    assert!(matches!(
        RiggedSpriteAsset::from_published_ron(&future),
        Err(RiggedSpriteError::Schema { found: 4 })
    ));
}

/// A hybrid states each row of its sheet as a part clip or a baked clip, and
/// a row it states as neither, or as both, is refused.
#[test]
fn a_hybrid_states_each_row_as_parts_or_baked() {
    let hybrid = FIXTURE.replace("    },\n)", "    },\n    baked_clips: [\"transform\"],\n)");
    assert_ne!(hybrid, FIXTURE, "the fixture edit did not apply");
    let asset = RiggedSpriteAsset::from_published_ron(&hybrid).expect("the hybrid parses");
    assert_eq!(asset.realization("idle"), Some(ClipRealization::Parts));
    assert_eq!(asset.realization("transform"), Some(ClipRealization::Baked));
    assert_eq!(asset.realization("walk"), None);
    assert_eq!(asset.frame("transform", 0), None, "a baked clip has no draws");
    assert_eq!(asset.check_rows(["idle", "transform"]), Ok(()));
    assert_eq!(
        asset.check_rows(["idle", "transform", "walk"]),
        Err(RiggedSpriteError::Unrealized("walk".to_owned()))
    );
    assert_eq!(
        asset.check_rows(["idle"]),
        Err(RiggedSpriteError::UnknownRow("transform".to_owned()))
    );
    let both = FIXTURE.replace("    },\n)", "    },\n    baked_clips: [\"idle\"],\n)");
    assert_eq!(
        RiggedSpriteAsset::from_published_ron(&both),
        Err(RiggedSpriteError::TwoRealizations("idle".to_owned()))
    );
}

#[test]
fn a_draw_record_fits_the_planned_budget() {
    assert!(std::mem::size_of::<PartDraw>() <= 32);
}

/// A pirate's flipbook realizes the rows of its baked sheet: the same row
/// names, the same frame counts, the same frame size.
#[test]
fn the_pirate_flipbooks_realize_the_rows_of_their_sheets() {
    for target in [
        "pirate_admiral",
        "pirate_lookout",
        "pirate_navigator",
        "pirate_quartermaster",
        "pirate_raider",
    ] {
        let asset = RiggedSpriteAsset::baked(target)
            .unwrap_or_else(|| panic!("`{target}` publishes no part flipbook: run scripts/regen/sprites.sh"));
        let record = crate::character::sheets::record_for_sheet_key(target).expect("a baked sheet");
        let rows: Vec<(&str, usize)> = record
            .rows
            .iter()
            .map(|row| (row.animation.as_str(), row.frame_count as usize))
            .collect();
        let mut clips: Vec<(&str, usize)> = asset
            .clip_names()
            .map(|name| (name, asset.clip(name).unwrap().frame_count()))
            .collect();
        let mut expected = rows.clone();
        clips.sort();
        expected.sort();
        assert_eq!(clips, expected, "`{target}`");
        asset
            .check_rows(rows.iter().map(|(row, _)| *row))
            .unwrap_or_else(|error| panic!("`{target}` {error}"));
        assert_eq!(asset.baked_clip_names().count(), 0, "`{target}` is drawn from parts in every row");
        assert_eq!(
            asset.frame_size,
            UVec2::new(record.frame_width, record.frame_height),
            "`{target}`"
        );
    }
}

/// A tier draws the same parts at the same size from its own smaller rects.
#[test]
fn a_tier_keeps_every_part_size_and_samples_its_own_rects() {
    use ambition_persistence::settings::TextureResolutionScale;
    let full = RiggedSpriteAsset::baked("pirate_raider").expect("a published flipbook");
    for tier in [TextureResolutionScale::Half, TextureResolutionScale::Quarter, TextureResolutionScale::Potato] {
        let tiered = full
            .for_tier(tier)
            .unwrap_or_else(|| panic!("no {tier:?} table: run generate_visual_quality_variants.py"))
            .expect("the tier table is this flipbook's");
        assert!(tiered.texel_scale < 1.0, "{tier:?}");
        assert_eq!(tiered.draw_count(), full.draw_count());
        for (part, full_part) in tiered.parts.iter().zip(&full.parts) {
            assert_eq!((part.size, part.pivot), (full_part.size, full_part.pivot));
            let expected = full_part.size * tiered.texel_scale;
            let got = part.rect.size().as_vec2();
            assert!(
                (got - expected.round().max(Vec2::ONE)).abs().max_element() <= 1.0,
                "{tier:?}: a {:?} part has a {got:?} rect",
                full_part.size
            );
        }
    }
    assert_eq!(full.for_tier(TextureResolutionScale::Full).unwrap().unwrap(), full);
}

/// Mary-O is drawn entirely from parts
/// (`docs/planning/engine/mary-o-part-realization.md`): every row of every
/// form, the transition clips with their effects among them, is a part clip
/// with the sheet's frame count, at every tier.
#[test]
fn mary_os_flipbooks_draw_every_row_from_parts() {
    use ambition_persistence::settings::TextureResolutionScale;
    for target in ["mary_o_v2", "mary_o_v2_tall", "mary_o_v2_fire"] {
        let asset = RiggedSpriteAsset::baked(target).unwrap_or_else(|| {
            panic!("`{target}` publishes no part flipbook: run scripts/regen/sprites.sh --target {target}")
        });
        let record = crate::character::sheets::record_for_sheet_key(target).expect("a baked sheet");
        assert!(!record.rows.is_empty(), "`{target}` sheet has no rows");
        asset
            .check_rows(record.rows.iter().map(|row| row.animation.as_str()))
            .unwrap_or_else(|error| panic!("`{target}` {error}"));
        assert_eq!(asset.baked_clip_names().count(), 0, "`{target}` still leaves rows baked");
        // Her locomotion loops tween; transitions and one-frame rows step (D3).
        for row in &record.rows {
            let expected = if ["walk", "crouch_walk", "climb", "swim"].contains(&row.animation.as_str()) {
                ClipTween::Linear
            } else {
                ClipTween::Step
            };
            assert_eq!(asset.clip(&row.animation).unwrap().tween, expected, "`{target}` `{}`", row.animation);
        }
        for row in &record.rows {
            assert_eq!(
                asset.realization(&row.animation),
                Some(ClipRealization::Parts),
                "`{target}` `{}`",
                row.animation
            );
            assert_eq!(
                asset.clip(&row.animation).unwrap().frame_count(),
                row.frame_count as usize,
                "`{target}` `{}`",
                row.animation
            );
        }
        for tier in [
            TextureResolutionScale::Half,
            TextureResolutionScale::Quarter,
            TextureResolutionScale::Potato,
        ] {
            let tiered = asset
                .for_tier(tier)
                .unwrap_or_else(|| panic!("`{target}` has no {tier:?} table"))
                .unwrap_or_else(|error| panic!("`{target}` {tier:?} {error}"));
            for row in &record.rows {
                assert_eq!(
                    tiered.realization(&row.animation),
                    Some(ClipRealization::Parts),
                    "`{target}` {tier:?} `{}`",
                    row.animation
                );
            }
        }
    }
}

/// The flipbooks are on unless the environment turns them off (Jon's
/// go-ahead, 2026-10-01): unset is on, and only an off word refuses them.
#[test]
fn the_flipbooks_are_on_unless_the_environment_turns_them_off() {
    let admits = |value: Option<&str>| RiggedSpriteAdmission::from_setting(value).admit;
    assert_eq!(
        [None, Some("1"), Some("on"), Some(""), Some("0"), Some(" Off "), Some("false"), Some("no")].map(admits),
        [true, true, true, true, false, false, false, false],
        "admitted for: unset, 1, on, empty, 0, Off, false, no"
    );
    assert_eq!(RiggedSpriteAdmission::default(), RiggedSpriteAdmission::ADMIT);
}

/// Schema 2's tween rule (`RiggedSpriteAsset::tween_into`): a tracked draw
/// with the same part in the next frame moves linearly and turns the shorter
/// way; a draw whose part changes, or that has no track, holds; a clip that
/// steps is the frame itself; the last frame tweens to the first.
#[test]
fn a_tweened_clip_moves_each_track_to_its_next_place() {
    let text = r#"(
        schema_version: 2,
        target: "toy",
        pages: ["toy_parts.png"],
        frame_size: (64, 64),
        feet_pixel: (32.0, 60.0),
        parts: [
            (name: "a", page: 0, rect: (0, 0, 4, 4), pivot: (2.0, 2.0)),
            (name: "b", page: 0, rect: (4, 0, 4, 4), pivot: (2.0, 2.0)),
            (name: "c", page: 0, rect: (8, 0, 4, 4), pivot: (2.0, 2.0)),
        ],
        tracks: ["arm", "head"],
        clips: {
            "walk": (frame_duration_s: 0.1, tween: Linear, frames: [
                [(part: 0, at: (0.0, 0.0), rotation: 3.0, scale: (1.0, 1.0), track: 0),
                 (part: 1, at: (5.0, 5.0), rotation: 0.0, scale: (1.0, 1.0), track: 1),
                 (part: 2, at: (7.0, 7.0), rotation: 0.0, scale: (1.0, 1.0))],
                [(part: 0, at: (10.0, -4.0), rotation: -3.0, scale: (1.0, 1.0), track: 0),
                 (part: 2, at: (9.0, 9.0), rotation: 0.0, scale: (1.0, 1.0), track: 1)],
            ]),
            "idle": (frame_duration_s: 0.1, frames: [
                [(part: 0, at: (0.0, 0.0), rotation: 0.0, scale: (1.0, 1.0), track: 0)],
                [(part: 0, at: (4.0, 0.0), rotation: 0.0, scale: (1.0, 1.0), track: 0)],
            ]),
        },
    )"#;
    let asset = RiggedSpriteAsset::from_published_ron(text).expect("a schema-2 flipbook");
    let mut out = Vec::new();
    asset.tween_into("walk", 0, 0.5, &mut out).unwrap();
    assert_eq!(out[0].at, Vec2::new(5.0, -2.0));
    // 3.0 to -3.0 is 0.28 rad the short way, through pi; not 6 rad back.
    let turned = 3.0 + (std::f32::consts::TAU - 6.0) * 0.5;
    assert!((out[0].rotation - turned).abs() < 1.0e-5, "{}", out[0].rotation);
    assert_eq!(out[1].at, Vec2::new(5.0, 5.0), "a track whose part changes holds");
    assert_eq!(out[2].at, Vec2::new(7.0, 7.0), "an untracked draw holds");
    asset.tween_into("walk", 1, 0.5, &mut out).unwrap();
    assert_eq!(out[0].at, Vec2::new(5.0, -2.0), "the last frame tweens to the first");
    asset.tween_into("idle", 0, 0.5, &mut out).unwrap();
    assert_eq!(out[0].at, Vec2::ZERO, "a clip that steps holds its frame");
    assert_eq!(asset.clip("idle").unwrap().tween, ClipTween::Step);
}

/// A schema-1 file still reads: untracked clips that step.
#[test]
fn a_schema_one_flipbook_reads_as_untracked_steps() {
    let text = r#"(schema_version: 1, target: "toy", pages: ["p.png"], frame_size: (8, 8), feet_pixel: (4.0, 8.0),
        parts: [(name: "a", page: 0, rect: (0, 0, 2, 2), pivot: (1.0, 1.0))],
        clips: {"idle": (frame_duration_s: 0.1, frames: [[(part: 0, at: (0.0, 0.0), rotation: 0.0, scale: (1.0, 1.0))]])})"#;
    let asset = RiggedSpriteAsset::from_published_ron(text).expect("a schema-1 flipbook");
    assert_eq!(asset.clip("idle").unwrap().tween, ClipTween::Step);
    assert_eq!(asset.frame("idle", 0).unwrap()[0].track, None);
    let bad = text.replace("scale: (1.0, 1.0))", "scale: (1.0, 1.0), track: 3)");
    assert!(matches!(
        RiggedSpriteAsset::from_published_ron(&bad),
        Err(RiggedSpriteError::MissingTrack { track: 3, .. })
    ));
}

/// Schema 3: the placement rule, a draw's own opacity, and the opacity of a
/// frame that fades as one picture. An in-between moves a draw's opacity with
/// its place; the frame opacity is the current frame's. An older file is
/// snapped and opaque.
#[test]
fn a_schema_three_flipbook_carries_its_placement_and_opacities() {
    let text = r#"(schema_version: 3, target: "toy", pages: ["p.png"], frame_size: (8, 8), feet_pixel: (4.0, 8.0),
        placement: Continuous,
        parts: [(name: "a", page: 0, rect: (0, 0, 2, 2), pivot: (1.0, 1.0))],
        tracks: ["blade"],
        clips: {
            "swing": (frame_duration_s: 0.1, tween: Linear, frames: [
                [(part: 0, at: (0.5, 0.25), rotation: 0.0, scale: (-1.0, 1.0), track: 0, opacity: 0.2)],
                [(part: 0, at: (1.5, 0.25), rotation: 0.0, scale: (-1.0, 1.0), track: 0)],
            ]),
            "death": (frame_duration_s: 0.1, frame_opacity: [1.0, 0.5], frames: [
                [(part: 0, at: (0.0, 0.0), rotation: 0.0, scale: (1.0, 1.0))],
                [(part: 0, at: (0.0, 0.0), rotation: 0.0, scale: (1.0, 1.0))],
            ]),
        })"#;
    let asset = RiggedSpriteAsset::from_published_ron(text).expect("a schema-3 flipbook");
    assert_eq!(asset.placement, RigPlacement::Continuous);
    let swing = asset.frame("swing", 0).unwrap()[0];
    assert_eq!((swing.opacity, swing.scale), (0.2, Vec2::new(-1.0, 1.0)));
    assert_eq!(asset.frame("swing", 1).unwrap()[0].opacity, 1.0, "an absent opacity is opaque");
    let mut out = Vec::new();
    asset.tween_into("swing", 0, 0.5, &mut out).unwrap();
    assert!((out[0].opacity - 0.6).abs() < 1.0e-6, "{}", out[0].opacity);
    assert_eq!(out[0].at, Vec2::new(1.0, 0.25));
    assert_eq!(asset.frame_opacity("death", 1), 0.5);
    assert_eq!(asset.frame_opacity("death", 9), 0.5, "past the end holds the last frame");
    assert_eq!(asset.frame_opacity("swing", 0), 1.0);
    let short = text.replace("frame_opacity: [1.0, 0.5]", "frame_opacity: [0.5]");
    assert!(matches!(
        RiggedSpriteAsset::from_published_ron(&short),
        Err(RiggedSpriteError::FrameOpacityCount { opacities: 1, frames: 2, .. })
    ));
    let older = RiggedSpriteAsset::from_published_ron(FIXTURE).expect("a schema-1 flipbook");
    assert_eq!(older.placement, RigPlacement::Snapped);
    assert_eq!(older.frame("idle", 0).unwrap()[0].opacity, 1.0);
}
