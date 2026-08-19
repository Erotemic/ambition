//! Tests for `sheets`: that `spec_from_record` prefers manifest-authored
//! `tuning:` over the passed-in Rust `SheetTuning` const.

use super::*;

/// When the manifest carries a `tuning:` block,
/// `spec_from_record` must prefer it over the passed-in
/// `SheetTuning` const. Catches a regression where the migration
/// to manifest-authored tuning silently falls back to the Rust
/// const for every char that hasn't been migrated yet.
#[test]
fn spec_from_record_prefers_manifest_tuning_when_present() {
    // Synthetic record with manifest-authored tuning that
    // diverges sharply from the legacy const so any mix-up is
    // detectable.
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
    // Pass an OBVIOUSLY different legacy tuning. The override
    // path means the manifest values win.
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

/// When the manifest has no `tuning:` block (the common case for
/// existing chars whose `*_SHEET` const still owns their values),
/// `spec_from_record` falls back to the passed-in const. Pins
/// the backwards-compat half of the override path.
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

/// **The quad is the sheet's body rectangle scaled onto the collision box, and
/// `collision_scale` has nothing to do with it.**
///
/// ⭐ **the poison is the point, not the equality.** Asserting "the drawn body
/// equals the collision box" alone would pass under the OLD arithmetic for any
/// sheet whose `collision_scale` happened to be tuned right — which is exactly
/// how 116 authored values survived being useless. So this builds the same sheet
/// twice with wildly different `collision_scale` and requires the two quads to
/// be BYTE-IDENTICAL: the field cannot be doing work, whatever value it holds.
#[test]
fn a_published_body_sizes_the_quad_and_collision_scale_is_inert() {
    // A 40x80 character sitting off-centre inside a 100x120 frame, so a quad
    // taken from the FRAME and a quad taken from the BODY cannot coincide.
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
    // Uniform: the frame's aspect survives, so the art is scaled and never
    // stretched.
    assert!(
        ((quad.x / 100.0) - (quad.y / 120.0)).abs() < 1e-4,
        "the quad {quad:?} scales the 100x120 frame by different amounts per axis"
    );
}

/// The 2 baked sheets that publish NO body (`creator_lab_props`,
/// `weird_hermit`) keep the old arithmetic, because there is nothing else to
/// ask — and that fallback is the only thing `collision_scale` still drives.
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

/// **A CLIP RESOLVES TO ITS ROW SLOT, AND A MISSING ONE RESOLVES TO NOTHING.**
///
/// ⭐ sprite redirect P0. Everything else on this spec is keyed by
/// `CharacterAnim` — 56 semantic body states — and the new fighter sheets carry
/// rows it has no variant for at all (`smash_forward`, `air_dodge`, `tumble`).
/// Growing the enum toward the 271-entry fighter-motion catalog is what the
/// redirect rejects; the authored clip name is the key instead.
///
/// ⛔ **the `None` term is the important one.** The habit this path replaces is
/// `row_index_of(name).unwrap_or(0)`, which silently draws ROW ZERO — idle — for
/// a row the sheet does not have, and looks exactly like a character that never
/// swings. An unresolvable chain must say so, so the caller can fall back to the
/// semantic pose ladder.
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

    // ⚠ and the slot indexes the real row: a resolved clip must be able to draw.
    assert_eq!(spec.row_at(3).frame_count, 5, "slot 3 is the 5-frame smash");
    assert_eq!(spec.flat_index_at(3, 2), spec.flat_index_at(3, 0) + 2);
}

/// **A trimmed sheet's CLIP must be sized and anchored by the CLIP's row.**
///
/// ⛔ the falsifier this pins: `CharacterAnimator::current_render` used to read
/// `frame_trim(self.current, …)` unconditionally, so while an authored clip was
/// playing it drew the clip's atlas cell at the SEMANTIC pose's trim. Both
/// lookups clamp their row and frame, so the failure was a silently misplaced,
/// mis-sized sprite rather than an error — and 122 of the 185 shipped sheets are
/// trimmed.
///
/// The two rows here are deliberately far apart in trim AND the clip row's name
/// is one `CharacterAnim::from_name` does NOT know, which is the case that
/// matters: a reusable effect sheet (`generic_action_fx`'s `hit_hard`,
/// `poof_small`, `release_ring`, …) is addressable ONLY by row name. Requiring
/// it to be addressable by pose would mean aliasing 18 effect rows onto a
/// body-state enum, which is what the clip path exists to avoid.
#[test]
fn a_clip_on_a_trimmed_sheet_is_measured_by_the_clip_row() {
    // `trimmed_render` / `FrameTrim` / `CharacterSpriteAsset` arrive through
    // `use super::*` (this module re-exports them).
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

    // Both terms of the comparison are OBSERVED, so this cannot pass vacuously.
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

    // And the pose path is unchanged: dropping the clip returns to the pose's trim.
    animator.request(CharacterAnim::Idle);
    assert_eq!(
        animator.current_render(),
        Some(trimmed_render(&pose_trim, base_size, base_anchor)),
        "with no clip playing the semantic pose still measures the frame"
    );
}

/// **Repacking a sheet does not redraw it.**
///
/// The ultrapack synthesizes its own [`SheetRecord`] from atlas frame rects,
/// which cannot know which way the body in those pixels points — so the base
/// manifest's drawn facing has to be carried onto it, the same way the caller
/// carries the base spec's `tuning`. Both west-drawn characters are packed at
/// all four tiers, so a pack path that dropped this would have left them facing
/// backwards again on exactly the devices that load packs, while their own
/// sheets looked correct.
#[test]
fn a_packed_target_keeps_the_facing_its_artwork_was_drawn_in() {
    for target in ["patent_clerk", "carl_stargan"] {
        let base = record_for_target(target)
            .unwrap_or_else(|| panic!("{target}'s sheet is baked into the sheet table"));
        // The premise. Without it the assertions below hold vacuously for a
        // sheet that never exercised the inheritance.
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

/// **A CHARACTER'S GAMEPLAY BODY MUST NOT DEPEND ON THE GRAPHICS SETTING.**
///
/// Every sheet is published four times — full resolution plus `0_5x`, `0_25x`
/// and `potato` — and each publication carries its own `body_metrics`. Those
/// metrics are not decoration: `authored_body` is the sheet's claim that
/// `body_pixel_bbox` is a GAMEPLAY BODY rather than the extent of the drawing,
/// and `authored_body_pixel_size` refuses to answer without it. A collision box
/// derived from one tier and not another is a body whose SIZE changes when the
/// player turns the graphics down.
///
/// ⛔⛔ **and the tiers really did disagree — twelve sheets, measured
/// 2026-08-19**: `player_extended`, the three `player_*_review` sheets and eight
/// `robot_*` variants declared `authored_body: true` in all three reduced tiers
/// and NOT at full resolution, because the tier road was regenerated after a
/// generator gained a `body_inset` and the full-res road never was. ⭐ **it was
/// latent only by luck**: `authored_body_pixel_size` is called with the bare
/// target id, so full resolution always won and nothing observed the other
/// three. Luck is not an invariant, and the gap widened every time one road was
/// regenerated without the other.
///
/// ⚠ **this asks the BAKED INDEX, not the files** — the same table every runtime
/// lookup reads, so it cannot pass against a tree the build did not compile.
#[test]
fn a_sheets_gameplay_body_does_not_depend_on_the_graphics_setting() {
    const TIERS: [&str; 3] = ["0_5x", "0_25x", "potato"];

    let index = record_index();
    let mut compared = 0usize;
    let mut disagreements: Vec<String> = Vec::new();

    for (target, full) in index.iter() {
        // Full-resolution targets only — a tier key carries its suffix, and
        // comparing a tier against itself proves nothing.
        if TIERS.iter().any(|tier| target.ends_with(&format!(".{tier}"))) {
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

    // ⚠ the zero floor. A build that baked no quality variants at all — or an
    // index whose tier keys stopped carrying their suffix — would compare
    // NOTHING and report perfect agreement.
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
         sheets (`./regen_sprites.sh --target <name>`), then the tiers:\n{}",
        disagreements.join("\n")
    );
}
