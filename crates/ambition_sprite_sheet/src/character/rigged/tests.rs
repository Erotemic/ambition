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

/// EVERY published flipbook is one the game can realize: its clips are rows
/// of the sheet published beside it (the realization refuses another, a panic
/// in the hall of characters: `robot` named a clip `air_back` its embedded
/// sheet did not have, 2026-10-03), and every quality tier's table is this
/// flipbook's. A sheet and its flipbook are written by one publish; this is
/// the census that they shipped together.
#[test]
fn every_published_flipbook_realizes_its_sheet_at_every_tier() {
    use ambition_persistence::settings::TextureResolutionScale;
    let targets: Vec<&str> = crate::baked_part_flipbooks::baked_part_flipbook_targets().collect();
    let mut failures = Vec::new();
    for target in &targets {
        let asset = RiggedSpriteAsset::baked(target).expect("listed");
        let Some(record) = crate::character::sheets::record_for_sheet_key(target) else {
            failures.push(format!("`{target}`: no published sheet"));
            continue;
        };
        if let Err(error) = asset.check_rows(record.rows.iter().map(|row| row.animation.as_str())) {
            failures.push(format!("`{target}` {error}"));
        }
        for tier in [TextureResolutionScale::Half, TextureResolutionScale::Quarter, TextureResolutionScale::Potato] {
            if let Some(Err(error)) = asset.for_tier(tier) {
                failures.push(format!("`{target}` at {tier:?}: {error}"));
            }
        }
    }
    // ⛔ Premise: the census has a population (an unpublished checkout has none).
    assert!(targets.len() >= 100, "only {} published flipbooks: run scripts/regen/sprites.sh", targets.len());
    assert!(failures.is_empty(), "{} flipbook(s) the game would refuse:\n{}", failures.len(), failures.join("\n"));
}

/// The road a flipbook is drawn by is the publish's measured verdict: absent
/// means parts; `realize: baked` (parts costlier than the sheet) is read back.
#[test]
fn a_flipbook_states_the_road_it_is_drawn_by() {
    let parts = crate::baked_part_flipbooks::baked_part_flipbook_targets()
        .filter_map(|target| RiggedSpriteAsset::baked(target))
        .partition::<Vec<_>, _>(|asset| asset.realize == Realize::Parts);
    assert!(!parts.0.is_empty() && !parts.1.is_empty(), "both roads are published: {} parts, {} baked", parts.0.len(), parts.1.len());
    let text = crate::baked_part_flipbooks::published_ron_on_build_host("director").expect("published");
    let baked = text.replacen("    placement:", "    realize: baked,\n    placement:", 1);
    assert_eq!(RiggedSpriteAsset::from_published_ron(&baked).unwrap().realize, Realize::Baked);
    assert_eq!(RiggedSpriteAsset::from_published_ron(&text).unwrap().realize, Realize::Parts);
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
    assert_eq!((swing.opacity(), swing.scale), (0.2, Vec2::new(-1.0, 1.0)));
    assert_eq!(asset.frame("swing", 1).unwrap()[0].opacity(), 1.0, "an absent opacity is opaque");
    let mut out = Vec::new();
    asset.tween_into("swing", 0, 0.5, &mut out).unwrap();
    assert!((out[0].opacity() - 0.6).abs() < 1.0 / 255.0, "{}", out[0].opacity());
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
    assert_eq!(older.frame("idle", 0).unwrap()[0].opacity(), 1.0);
}


/// A draw's tint (a back limb drawn as its front limb, darker) is read from
/// the table, absent is as painted, and it moves linearly in a tween.
#[test]
fn a_draw_tint_is_read_and_tweened() {
    let ron = FIXTURE
        .replace("schema_version: 1", "schema_version: 3")
        .replace(
            "[(part: 0, at: (-1.0, -20.0), rotation: 0.5, scale: (1.0, 0.9))]",
            "[(part: 0, at: (-1.0, -20.0), rotation: 0.5, scale: (1.0, 0.9), tint: (0.8, 0.6, 0.4))]",
        );
    let asset = RiggedSpriteAsset::from_published_ron(&ron).expect("a tinted draw parses");
    let tinted = asset.frame("idle", 0).unwrap()[0];
    assert!((tinted.tint() - Vec3::new(0.8, 0.6, 0.4)).abs().max_element() < 1.0 / 255.0, "{:?}", tinted.tint());
    assert_eq!(asset.frame("idle", 1).unwrap()[1].tint(), Vec3::ONE, "an absent tint draws as painted");
}

/// The game decodes each table from the bincode `build.rs` wrote; that must be
/// the flipbook its RON is. Every embedded table, every tier: a field the
/// bincode lost, or a table the build could not read (embedded as text, so
/// parsed in the hall again), fails here.
#[test]
fn every_embedded_table_decodes_to_the_flipbook_its_ron_is() {
    use crate::baked_part_flipbooks::{published_ron_on_build_host, BakedPartFlipbook, BAKED_PART_FLIPBOOKS};
    let mut failures = Vec::new();
    for (key, table, _) in BAKED_PART_FLIPBOOKS {
        if matches!(table, BakedPartFlipbook::Unread(_)) {
            failures.push(format!("`{key}`: embedded as text, the build could not read it"));
            continue;
        }
        let text = published_ron_on_build_host(key).expect("the published file");
        let decoded = table.decode().unwrap_or_else(|error| panic!("`{key}` {error}"));
        if decoded != RiggedSpriteAsset::from_published_ron(&text).unwrap_or_else(|error| panic!("`{key}` {error}")) {
            failures.push(format!("`{key}`: decodes to another flipbook than its RON"));
        }
    }
    // ⛔ Premise: the census has a population.
    assert!(BAKED_PART_FLIPBOOKS.len() >= 100, "only {} embedded tables", BAKED_PART_FLIPBOOKS.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `target`'s published flipbook, its body rig, and the rig's bindings.
fn posed(target: &str) -> (RiggedSpriteAsset, ambition_characters::actor::PreparedBodyRig, PosedParts) {
    let flipbook = RiggedSpriteAsset::baked(target).expect("a published flipbook");
    let rig = ambition_characters::actor::BodyRigDefinition::from_published_ron(
        crate::baked_body_rigs::baked_body_rig(target).expect("a published body rig"),
    )
    .expect("the rig parses")
    .prepare()
    .expect("the rig prepares");
    let posed = PosedParts::bind(&flipbook, &rig, 1.0, 0.02);
    (flipbook, rig, posed)
}

/// ⭐ ONE SEMANTIC DECOMPOSITION. Mary-O's part flipbook is what her body
/// rig's pose places: every rig part rides a joint (its pivot and angle fixed
/// in the joint's frame, within a pixel and a degree), and the rig's own frame
/// of each clip, through those bindings, puts every draw where the flipbook
/// drew it. The flipbook's transform table is then a cache of the rig's pose,
/// not a second authority (`scripts/measure_track_joints.py` measures every
/// character that publishes both; the pirates do not agree yet).
#[test]
fn mary_os_parts_are_placed_by_her_body_rigs_pose() {
    for target in ["mary_o_v2", "mary_o_v2_fire", "mary_o_v2_tall"] {
        let (flipbook, rig, posed) = posed(target);
        assert!(posed.frames_measured() > 0, "{target}: no clip shares its frames with the rig, so this measures nothing");
        let (mut joints, mut placed) = (Vec::new(), Vec::new());
        let mut compared = 0;
        for row in flipbook.clip_names() {
            let Some(rig_clip) = rig.clip(row) else { continue };
            let clip = flipbook.clip(row).unwrap();
            if rig_clip.frames.len() != clip.frame_count() {
                continue;
            }
            for index in 0..clip.frame_count() {
                assert!(rig.solve(row, index, &mut joints));
                let frame = flipbook.frame(row, index).unwrap();
                // ⛔ The frame's own placements are wiped first: the pose
                // alone must put each part back, or a `place` that returned
                // its input would compare every draw with itself.
                let unplaced: Vec<PartDraw> = frame.iter().map(|draw| PartDraw { at: Vec2::ZERO, rotation: 0.0, ..*draw }).collect();
                posed.place(&unplaced, &joints, &mut placed);
                for (drawn, from_pose) in frame.iter().zip(&placed) {
                    // Every rig part a shared frame draws rides a joint. (A row
                    // the rig does not author, `death` or `shrink`, draws parts
                    // nothing here measures.)
                    let name = drawn.track.map(|track| flipbook.tracks[usize::from(track)].as_str()).unwrap_or("?");
                    if name.starts_with("overlay:") {
                        continue;
                    }
                    assert!(
                        drawn.track.and_then(|track| posed.binding(track)).is_some(),
                        "{target} {row}[{index}]: the part `{name}` rides no joint of the body rig"
                    );
                    compared += 1;
                    assert!(
                        drawn.at.distance(from_pose.at) <= 1.0 && (drawn.rotation - from_pose.rotation).abs() <= 0.02,
                        "{target} {row}[{index}]: the rig's pose puts part {} at {:?} turned {}, the flipbook at {:?} turned {}",
                        drawn.part,
                        from_pose.at,
                        from_pose.rotation,
                        drawn.at,
                        drawn.rotation
                    );
                }
            }
        }
        assert!(compared > 0, "{target}: no draw compared");
    }
}

/// ⭐ POSE IS AN INPUT (the ragdoll seam). A pose no clip authored — Mary-O's
/// idle with one joint turned, as a physics step would turn it — moves the
/// parts that ride that joint, rigidly about it, and leaves every other part
/// where idle drew it. No flipbook frame for that pose exists; the same parts
/// draw it.
#[test]
fn a_pose_no_clip_authored_moves_the_parts_that_ride_the_turned_joint() {
    let (flipbook, rig, posed) = posed("mary_o_v2");
    let mut joints = Vec::new();
    assert!(rig.solve("idle", 0, &mut joints));
    let idle = flipbook.frame("idle", 0).unwrap();
    let mut at_rest = Vec::new();
    posed.place(idle, &joints, &mut at_rest);

    let arm = rig.joint_names().iter().position(|name| name == "near_arm").expect("Mary-O's rig has a near arm") as u16;
    // ⛔ Premise: some part rides the arm, and some does not.
    let rides = |draw: &PartDraw| draw.track.and_then(|track| posed.binding(track)).is_some_and(|binding| binding.joint == arm);
    assert!(idle.iter().any(rides) && !idle.iter().all(rides), "premise: the arm carries some parts and not all");

    let turn = 1.0_f32;
    let pivot = joints[usize::from(arm)].translation;
    use bevy::math::Affine2;
    let swing = Affine2::from_translation(pivot) * Affine2::from_angle(turn) * Affine2::from_translation(-pivot);
    let mut flailing = joints.clone();
    for (index, frame) in flailing.iter_mut().enumerate() {
        // The arm and every joint under it (none in this rig) turn with it.
        if index == usize::from(arm) {
            *frame = swing * *frame;
        }
    }
    let mut moved = Vec::new();
    posed.place(idle, &flailing, &mut moved);
    for ((rest, now), draw) in at_rest.iter().zip(&moved).zip(idle) {
        if rides(draw) {
            let expected = swing.transform_point2(rest.at);
            assert!(now.at.distance(expected) < 1.0e-3, "part {} did not swing with the arm", draw.part);
            assert!((now.rotation - rest.rotation - turn).abs() < 1.0e-4, "part {} did not turn with the arm", draw.part);
        } else {
            assert_eq!((now.at, now.rotation), (rest.at, rest.rotation), "part {} moved, and it does not ride the arm", draw.part);
        }
    }
}

/// A long bar that turns about the centre of a frame that is wide and not
/// tall. It lies along the frame in each of its two frames, and it stands
/// across the frame half-way between them.
const TURNING_BAR: &str = r#"(
    schema_version: 2,
    target: "bar",
    pages: ["bar_parts.png"],
    frame_size: (64, 24),
    feet_pixel: (32.0, 12.0),
    parts: [(name: "bar", page: 0, rect: (0, 0, 60, 4), pivot: (30.0, 2.0))],
    tracks: ["bar"],
    clips: {
        "spin": (frame_duration_s: 0.1, tween: Linear, frames: [
            [(part: 0, at: (0.0, 0.0), rotation: 0.0, scale: (1.0, 1.0), track: 0)],
            [(part: 0, at: (0.0, 0.0), rotation: 3.0, scale: (1.0, 1.0), track: 0)],
        ]),
    },
)"#;

/// THE ROOM A CELL NEEDS COVERS THE IN-BETWEENS OF A TWEENED CLIP.
///
/// `art_overhang` was the reach of the AUTHORED frames. A tweened clip draws
/// between them, and a part that turns between two frames reaches where
/// neither frame does: this bar fits the frame at each end and stands 18 px
/// past it half-way. A cell sized from the authored frames cut it.
#[test]
fn the_overhang_of_a_tweened_clip_covers_its_in_betweens() {
    let asset = RiggedSpriteAsset::from_published_ron(TURNING_BAR).expect("the bar parses");
    for index in 0..2 {
        let reach = asset.reach_past_frame(asset.frame("spin", index).unwrap());
        assert!(reach <= 0.0, "premise: frame {index} of the bar fits the frame, and it reaches {reach} px past");
    }
    let mut out = Vec::new();
    let mut farthest = 0.0_f32;
    for index in 0..2 {
        for step in 0..=200 {
            asset.tween_into("spin", index, step as f32 / 200.0, &mut out).unwrap();
            let reach = asset.reach_past_frame(&out);
            assert!(
                reach <= asset.art_overhang + 1.0e-3,
                "frame {index}, {step}/200 of the way: the bar reaches {reach} px past the frame, and the flipbook states {}",
                asset.art_overhang
            );
            farthest = farthest.max(reach);
        }
    }
    assert!(farthest > 15.0, "premise: half-way, the bar stands past the frame; it reached {farthest} px");
    assert!(
        asset.art_overhang <= farthest + 1.0,
        "the stated overhang, {} px, is more than a pixel past the farthest the bar reaches, {farthest} px",
        asset.art_overhang
    );
}

/// The same law over what is published: no in-between of a tweened clip of a
/// published flipbook reaches past the overhang the flipbook states.
#[test]
fn no_in_between_of_a_published_flipbook_reaches_past_its_overhang() {
    let mut out = Vec::new();
    let mut tweened = 0;
    for key in crate::baked_part_flipbooks::baked_part_flipbook_targets() {
        let asset = RiggedSpriteAsset::baked(key).expect("a published flipbook parses");
        let rows: Vec<String> = asset.clip_names().map(str::to_owned).collect();
        for row in rows {
            let clip = asset.clip(&row).unwrap();
            if clip.tween == ClipTween::Step {
                continue;
            }
            tweened += 1;
            for index in 0..clip.frame_count() {
                for step in 0..=16 {
                    asset.tween_into(&row, index, step as f32 / 16.0, &mut out).unwrap();
                    let reach = asset.reach_past_frame(&out);
                    assert!(
                        reach <= asset.art_overhang + 1.0e-3,
                        "`{key}` `{row}` frame {index}, {step}/16 of the way: {reach} px past the frame, stated {}",
                        asset.art_overhang
                    );
                }
            }
        }
    }
    assert!(tweened > 0, "premise: no published flipbook has a tweened clip, so this checked nothing");
}

#[test]
fn a_blink_row_and_its_mirror_have_the_teleport_warp() {
    assert_eq!(BodyWarp::of_row("blink_out"), Some(BodyWarp::TeleportOut));
    assert_eq!(BodyWarp::of_row("blink_in~mirrored"), Some(BodyWarp::TeleportIn));
    assert_eq!(BodyWarp::of_row("idle"), None);
    assert_eq!(BodyWarp::of_row("walk~mirrored"), None);
}
