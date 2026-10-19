use super::*;
use ambition_characters::actor::{GuardBreak, PartBox};

const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);

/// A shield 20 wide and 80 tall, its top at y = 20, held by a body facing
/// `facing`.
fn shield(faces: GuardFaces, facing: f32, open_above: bool) -> GuardPiece {
    GuardPiece {
        guard: 0,
        volume: ae::Aabb {
            min: ae::Vec2::new(10.0, 20.0),
            max: ae::Vec2::new(30.0, 100.0),
        },
        faces,
        facing,
        open_above,
    }
}

/// A front guard covers the side its body faces and not the other; a strike
/// from above its top goes over it unless it is closed above. A guard all
/// round covers both sides.
#[test]
fn a_guard_covers_its_front_and_not_its_back_or_over_its_top() {
    let front = shield(GuardFaces::Front, 1.0, true);
    let ahead = ae::Vec2::new(60.0, 60.0);
    let behind = ae::Vec2::new(-20.0, 60.0);
    let above = ae::Vec2::new(60.0, 0.0);
    assert!(front.covers(ahead, DOWN), "a strike from the front is turned");
    assert!(!front.covers(behind, DOWN), "a strike from behind gets past");
    assert!(!front.covers(above, DOWN), "a strike from above its top goes over it");
    assert!(shield(GuardFaces::Front, 1.0, false).covers(above, DOWN), "a guard closed above turns it");
    // Facing the other way, its front is the other side.
    assert!(shield(GuardFaces::Front, -1.0, true).covers(behind, DOWN));
    assert!(!shield(GuardFaces::Front, -1.0, true).covers(ahead, DOWN));
    let around = shield(GuardFaces::Around, 1.0, true);
    assert!(around.covers(ahead, DOWN) && around.covers(behind, DOWN));
    // Under gravity that points up, "above" is the other way.
    let up = ae::Vec2::new(0.0, -1.0);
    assert!(!shield(GuardFaces::Around, 1.0, true).covers(ae::Vec2::new(20.0, 120.0), up));
    assert!(shield(GuardFaces::Around, 1.0, true).covers(ae::Vec2::new(20.0, 60.0), up));
}

fn spec(breaks: Option<GuardBreak>) -> GuardSpec {
    GuardSpec {
        part: PartBox::Shield,
        faces: GuardFaces::Front,
        open_above: true,
        lowered_in: Vec::new(),
        inset_px: 0.0,
        deflect: Default::default(),
        breaks,
    }
}

/// Deflects in quick succession break a guard on the authored count; a slow
/// one starts the run again; a heavy strike breaks it at once; a guard that
/// never breaks only counts.
#[test]
fn quick_deflects_or_a_heavy_one_break_a_guard() {
    let breaks = GuardBreak {
        blocks: 3,
        within_s: 1.0,
        heavy: Some(4),
        stagger_s: 1.5,
    };
    let guard = spec(Some(breaks));
    let mut state = GuardState::default();
    assert!(!state.count_deflect(&guard, 1));
    state.since_deflect_s = 0.5;
    assert!(!state.count_deflect(&guard, 1));
    state.since_deflect_s = 0.5;
    assert!(state.count_deflect(&guard, 1), "the third quick deflect breaks it: {state:?}");
    assert!(state.broken() && state.broken_s == 1.5 && state.deflects == 3);

    let mut slow = GuardState::default();
    for _ in 0..5 {
        assert!(!slow.count_deflect(&guard, 1), "slow deflects never break it: {slow:?}");
        slow.since_deflect_s = 2.0;
    }

    let mut heavy = GuardState::default();
    assert!(heavy.count_deflect(&guard, 4), "a heavy strike breaks it at once");

    let mut never = GuardState::default();
    for _ in 0..10 {
        assert!(!never.count_deflect(&spec(None), 99));
    }
    assert_eq!(never.deflects, 10);
}
