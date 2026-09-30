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
