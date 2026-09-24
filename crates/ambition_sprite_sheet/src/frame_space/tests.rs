use super::*;
use ambition_platformer2d_core as ae;

const FRAME_W: f32 = 300.0;
const FRAME_H: f32 = 200.0;
const FEET_X: f32 = 150.0;
const FEET_Y: f32 = 190.0;

/// A one-row sheet whose `jab` authors a hitbox at `poly`, drawn facing
/// `faces_left`. All else is identical, so handedness is the only variable.
fn sheet(faces_left: bool, poly: &[(f32, f32)]) -> SheetRecord {
    let points: Vec<String> = poly.iter().map(|(x, y)| format!("({x}, {y})")).collect();
    let text = format!(
        r#"(
            target: "fixture",
            image: "fixture.png",
            label_width: 0,
            frame_width: {FRAME_W},
            frame_height: {FRAME_H},
            authored_faces_left: {faces_left},
            body_metrics: Some((
                feet_pixel: Some((x: {FEET_X}, y: {FEET_Y})),
                animations: {{ "jab": (hitbox: Some((poly: [{}]))) }},
            )),
            rows: [],
        )"#,
        points.join(", "),
    );
    ron::from_str(&text).expect("the fixture sheet parses")
}

fn jab(record: &SheetRecord) -> &AnimationBox {
    record
        .body_metrics
        .as_ref()
        .expect("fixture publishes body metrics")
        .animations
        .get("jab")
        .expect("fixture publishes a jab row")
        .hitbox
        .as_ref()
        .expect("fixture publishes a jab hitbox")
}

/// One world unit per frame pixel, so a body-local offset reads as pixels.
fn map(record: &SheetRecord) -> FrameToBody {
    FrameToBody::planting_feet(
        record,
        ae::Vec2::new(FRAME_W, FRAME_H),
        ae::Vec2::new(30.0, 48.0),
    )
}

fn points(volume: &ae::CombatVolume) -> Vec<ae::Vec2> {
    match volume {
        ae::CombatVolume::Convex { points, .. } => points.clone(),
        other => panic!("expected an authored hull, got {other:?}"),
    }
}

/// The same swing drawn the other way round is the same swing.
///
/// Two sheets with identical art mirrored about the feet; one declares
/// `authored_faces_left`. If the map reads that flag, both resolve to the same
/// body-local blade. If it reads only `facing`, the left-drawn blade comes out
/// behind the body.
#[test]
fn a_left_drawn_sheet_and_its_mirror_image_author_the_same_body_local_blade() {
    let right = [(200.0, 80.0), (290.0, 95.0), (200.0, 110.0)];
    // The same drawing, mirrored about the feet.
    let left: Vec<(f32, f32)> = right
        .iter()
        .map(|(x, y)| (2.0 * FEET_X - x, *y))
        .collect();

    let right_sheet = sheet(false, &right);
    let left_sheet = sheet(true, &left);

    let from_right = points(&map(&right_sheet).volume(jab(&right_sheet), None).unwrap());
    let from_left = points(&map(&left_sheet).volume(jab(&left_sheet), None).unwrap());

    assert_eq!(from_right.len(), from_left.len());
    for (r, l) in from_right.iter().zip(&from_left) {
        assert!(
            (*r - *l).length() < 1.0e-3,
            "the mirrored drawing must resolve to the same body-local blade: \
             right-drawn {r:?} vs left-drawn {l:?} (the whole hulls were \
             {from_right:?} and {from_left:?})"
        );
    }
    // The swing reaches forward, so the test above cannot pass with both
    // sheets wrong in the same direction.
    assert!(
        from_right.iter().any(|p| p.x > 15.0),
        "the fixture jab must reach forward of the body, got {from_right:?}"
    );
}

/// A left-drawn sheet's forward is `-x` in its own art. Stated directly, not
/// left implicit in `point`.
#[test]
fn the_art_forward_sign_is_the_whole_of_the_handedness() {
    assert_eq!(sheet(false, &[(0.0, 0.0)]).art_forward_x(), 1.0);
    assert_eq!(sheet(true, &[(0.0, 0.0)]).art_forward_x(), -1.0);
}

/// The renderer's mirror and the geometry map's mirror are one decision.
///
/// `art_is_mirrored` is what `apply_character_frame` writes to `flip_x`. A
/// left-drawn sheet mirrors when facing right, the opposite of a plain
/// `facing < 0` rule.
#[test]
fn a_left_drawn_sheet_mirrors_on_the_opposite_facing_from_a_right_drawn_one() {
    let down = ae::Vec2::new(0.0, 1.0);
    let right_drawn = sheet(false, &[(0.0, 0.0)]);
    let left_drawn = sheet(true, &[(0.0, 0.0)]);
    for facing in [-1.0_f32, 1.0] {
        assert_ne!(
            right_drawn.art_is_mirrored(facing, down),
            left_drawn.art_is_mirrored(facing, down),
            "at facing {facing} two oppositely-drawn sheets cannot want the same mirror"
        );
    }
    assert!(!right_drawn.art_is_mirrored(1.0, down));
    assert!(left_drawn.art_is_mirrored(1.0, down));
}

/// Per-frame geometry outranks the coarse per-animation box, and a frame index
/// past the authored samples holds the last one instead of falling back.
#[test]
fn a_per_frame_sample_outranks_the_coarse_box_and_the_last_sample_holds() {
    let text = r#"(
        parts: [],
        bbox: Some((x: 0, y: 0, w: 10, h: 10)),
        poly: [(200.0, 80.0), (290.0, 95.0), (200.0, 110.0)],
        frames: [
            (poly: [(160.0, 80.0), (170.0, 95.0), (160.0, 110.0)]),
            (poly: [(160.0, 80.0), (260.0, 95.0), (160.0, 110.0)]),
        ],
    )"#;
    let box_: AnimationBox = ron::from_str(text).expect("the fixture box parses");
    let width = |frame: Option<usize>| {
        let record = sheet(false, &[(0.0, 0.0)]);
        let bounds = map(&record).volume(&box_, frame).unwrap().bounds();
        bounds.max.x - bounds.min.x
    };
    assert!((width(Some(0)) - 10.0).abs() < 1.0e-3, "frame 0 is the short poke");
    assert!((width(Some(1)) - 100.0).abs() < 1.0e-3, "frame 1 is the extension");
    assert!(
        (width(Some(9)) - 100.0).abs() < 1.0e-3,
        "a frame past the authored samples holds the last one"
    );
    assert!(
        (width(None) - 90.0).abs() < 1.0e-3,
        "no frame index falls back to the coarse per-animation poly"
    );
}
