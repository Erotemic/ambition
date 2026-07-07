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
