use super::*;

/// One record, with the fields a case changes as parameters. It has a body
/// box, a per-pose hurtbox and two rows.
fn record(
    image: &str,
    y_offset: u32,
    row_index: u32,
    rect_x: i32,
    body_w: i32,
    duration_ms: u32,
    hurt_poly_x: f32,
) -> SheetRecord {
    let text = format!(
        r#"[(target: "bob", image: "{image}", images: ["{image}"], label_width: 100, y_offset: {y_offset},
            frame_width: 64, frame_height: 96,
            body_metrics: Some((
                body_pixel_bbox: Some((x: 10, y: 20, w: {body_w}, h: 70)),
                animations: {{
                    "attack": (frame_duration_secs: Some(0.05),
                               hurtbox: Some((poly: [(1.0, 2.0), ({hurt_poly_x}, 9.0), (3.0, 9.0)]))),
                }},
                feet_anchor_norm: Some((x: 0.5, y: 0.98)),
            )),
            rows: [
                (animation: "idle", row_index: {row_index}, frame_count: 2, duration_ms: {duration_ms},
                 duration_secs: 0.2, rects: [(x: {rect_x}, y: 0, w: 64, h: 96), (x: 164, y: 0, w: 64, h: 96)]),
                (animation: "attack", row_index: 1, frame_count: 1, duration_ms: 50, duration_secs: 0.05,
                 page: 0, rects: [(x: 100, y: 96, w: 64, h: 96)]),
            ])]"#
    );
    let mut records: Vec<SheetRecord> = ron::from_str(&text).expect("the fixture sheet parses");
    let mut record = records.remove(0);
    record.key = "bob".to_string();
    record
}

fn base() -> SheetRecord {
    record("bob_spritesheet.png", 0, 0, 100, 40, 200, 8.0)
}

fn digest(record: &SheetRecord) -> String {
    sheet_mechanics_digest([record])
}

/// ⭐ PACKING ALONE IS THE SAME IDENTITY: another atlas image, another row
/// band, another row position and other frame rects draw the same body.
#[test]
fn a_sheet_whose_packing_alone_differs_has_the_same_digest() {
    assert_eq!(digest(&base()), digest(&base()), "control: one record, one digest");
    let repacked = record("ultrapack_7.png", 192, 5, 300, 40, 200, 8.0);
    assert_eq!(
        digest(&base()),
        digest(&repacked),
        "a new atlas packing moved the identity of a body it does not change"
    );
}

/// ⭐ A BODY THAT DIFFERS IS ANOTHER IDENTITY: each mechanical field a body
/// reads moves the digest on its own.
#[test]
fn a_sheet_whose_mechanics_differ_has_another_digest() {
    let base = digest(&base());
    for (what, changed) in [
        ("the body box", record("bob_spritesheet.png", 0, 0, 100, 41, 200, 8.0)),
        ("a row's duration", record("bob_spritesheet.png", 0, 0, 100, 40, 210, 8.0)),
        ("a hurtbox polygon", record("bob_spritesheet.png", 0, 0, 100, 40, 200, 8.5)),
    ] {
        assert_ne!(base, digest(&changed), "{what} changed and the digest did not");
    }
    let mut taller = super::tests::base();
    taller.frame_height = 97;
    assert_ne!(base, digest(&taller), "the frame size changed and the digest did not");
}

/// Two records cannot be read as one: the boundary between them is in the
/// digest, so moving a row from one record to the next moves it.
#[test]
fn the_records_are_a_sequence_with_boundaries() {
    let mut one = base();
    let mut two = base();
    two.key = "bob2".to_string();
    let both = sheet_mechanics_digest([&one, &two]);
    let moved = two.rows.remove(1);
    one.rows.push(moved);
    assert_ne!(both, sheet_mechanics_digest([&one, &two]));
}

/// The baked digest is the projection of this build's records in key order.
#[test]
fn the_baked_digest_is_this_builds_records_in_key_order() {
    use crate::character::sheets::{available_sheet_keys, baked_sheet_mechanics_digest, record_for_sheet_key};
    let records: Vec<&SheetRecord> = available_sheet_keys()
        .into_iter()
        .map(|key| record_for_sheet_key(key).expect("a listed key has a record"))
        .collect();
    assert_eq!(baked_sheet_mechanics_digest(), sheet_mechanics_digest(records));
}
