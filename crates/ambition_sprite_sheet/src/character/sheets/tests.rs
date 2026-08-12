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
