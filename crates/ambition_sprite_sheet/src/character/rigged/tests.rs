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
    let future = FIXTURE.replace("schema_version: 1", "schema_version: 2");
    assert!(matches!(
        RiggedSpriteAsset::from_published_ron(&future),
        Err(RiggedSpriteError::Schema { found: 2 })
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

/// Rig packet 9: Mary-O's flipbooks are hybrids. Her walk is drawn from parts,
/// and every other row of her sheet is stated as baked, the transition clips
/// with their effects among them. Each tier table is the same hybrid.
#[test]
fn mary_os_flipbooks_draw_her_walk_from_parts_and_leave_the_rest_baked() {
    use ambition_persistence::settings::TextureResolutionScale;
    for target in ["mary_o_v2", "mary_o_v2_tall", "mary_o_v2_fire"] {
        let asset = RiggedSpriteAsset::baked(target).unwrap_or_else(|| {
            panic!("`{target}` publishes no part flipbook: run scripts/regen/sprites.sh --target {target}")
        });
        let record = crate::character::sheets::record_for_sheet_key(target).expect("a baked sheet");
        let rows: Vec<&str> = record.rows.iter().map(|row| row.animation.as_str()).collect();
        asset
            .check_rows(rows.iter().copied())
            .unwrap_or_else(|error| panic!("`{target}` {error}"));
        assert_eq!(asset.clip_names().collect::<Vec<_>>(), vec!["walk"], "`{target}`");
        for row in rows.iter().filter(|row| **row != "walk") {
            assert_eq!(asset.realization(row), Some(ClipRealization::Baked), "`{target}` `{row}`");
        }
        let walk = record.rows.iter().find(|row| row.animation == "walk").unwrap();
        assert_eq!(asset.clip("walk").unwrap().frame_count(), walk.frame_count as usize, "`{target}`");
        for tier in [
            TextureResolutionScale::Half,
            TextureResolutionScale::Quarter,
            TextureResolutionScale::Potato,
        ] {
            let tiered = asset
                .for_tier(tier)
                .unwrap_or_else(|| panic!("`{target}` has no {tier:?} table"))
                .unwrap_or_else(|error| panic!("`{target}` {tier:?} {error}"));
            assert_eq!(tiered.realization("idle"), Some(ClipRealization::Baked), "`{target}` {tier:?}");
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
