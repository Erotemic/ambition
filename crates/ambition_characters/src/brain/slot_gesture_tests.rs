//! An invalid participant identifier cannot write to a real participant.
//!
//! The old `get_mut` clamped an out-of-range [`crate::control::PlayerSlot`] onto the LAST valid
//! one, so `crate::control::PlayerSlot(9)` and `crate::control::PlayerSlot(3)` were the same controller. This
//! pins the replacement: invalidity is `None`, and every real slot is left
//! exactly as it was.

use crate::control::SlotInteractionState;

/// A gesture state that is visibly non-default in every slot, so a stray write
/// anywhere shows up as a difference rather than as a value that happened to
/// match.
fn distinctly_filled() -> SlotInteractionState {
    let mut state = crate::control::SlotInteractionState::default();
    for index in 0..crate::control::SlotControls::MAX_SLOTS {
        let gestures = state
            .get_mut(crate::control::PlayerSlot(index as u8))
            .expect("every slot below MAX_SLOTS is valid");
        gestures.interact_buffer_timer = 1.0 + index as f32;
        gestures.double_tap_up_pending = true;
    }
    state
}

/// THE INVARIANT. An out-of-range slot resolves to nothing, and nothing in
/// the array moves.
#[test]
fn an_invalid_slot_cannot_alter_any_real_participant() {
    let before = distinctly_filled();
    let mut after = before;

    // the zero floor: a `MAX_SLOTS` of 0 would make the loop below empty
    // and every assertion vacuous, and the fixture above would have written
    // nothing to compare.
    assert!(
        crate::control::SlotControls::MAX_SLOTS >= 2,
        "this test needs at least two real slots to tell a clamp from a refusal"
    );

    for out_of_range in [
        crate::control::SlotControls::MAX_SLOTS,
        crate::control::SlotControls::MAX_SLOTS + 1,
        u8::MAX as usize,
    ] {
        let slot = crate::control::PlayerSlot(out_of_range as u8);
        assert!(
            after.get_mut(slot).is_none(),
            "slot {out_of_range} is out of range but resolved to a real controller"
        );
        // A read of the same slot is a different question with a defensible
        // answer: a controller that does not exist is pressing nothing.
        assert_eq!(
            after.get(slot),
            crate::control::SlotGestures::default(),
            "an out-of-range READ should report a controller pressing nothing"
        );
    }

    for index in 0..crate::control::SlotControls::MAX_SLOTS {
        let slot = crate::control::PlayerSlot(index as u8);
        assert_eq!(
            after.get(slot),
            before.get(slot),
            "slot {index}'s gestures changed while only INVALID slots were addressed — \
             this is the clamp bug: an out-of-range participant identifier was resolved \
             onto a real participant and consumed its buffered input"
        );
    }
}

#[test]
fn the_last_valid_slot_is_not_a_dumping_ground_for_bad_indices() {
    let last = crate::control::PlayerSlot((crate::control::SlotControls::MAX_SLOTS - 1) as u8);
    let mut state = distinctly_filled();
    let before_last = state.get(last);
    assert!(
        before_last != crate::control::SlotGestures::default(),
        "the fixture never wrote to the last slot, so the assertion below could not fail"
    );

    if let Some(gestures) = state.get_mut(crate::control::PlayerSlot(
        crate::control::SlotControls::MAX_SLOTS as u8 + 7,
    )) {
        gestures.reset();
    }

    assert_eq!(
        state.get(last),
        before_last,
        "a write addressed to a slot that does not exist landed on the last valid \
         participant"
    );
}

// ── Holding Up as an alternative interact ─────────────────────────────────

const HOLD: f32 = 2.0;

/// Holding Up interacts ONCE, not on every frame after the threshold.
#[test]
fn a_sustained_up_hold_interacts_exactly_once() {
    let mut gestures = crate::control::SlotGestures::default();
    let mut fired = 0;
    for _ in 0..240 {
        if gestures.held_up_interact(true, 1.0 / 60.0, HOLD) {
            fired += 1;
        }
    }
    assert_eq!(
        fired, 1,
        "four seconds of holding Up opened the door {fired} times"
    );
}

/// Letting go restarts the hold — two short holds are not one long one.
#[test]
fn releasing_up_restarts_the_hold() {
    let mut gestures = crate::control::SlotGestures::default();
    for _ in 0..100 {
        assert!(!gestures.held_up_interact(true, 1.0 / 60.0, HOLD));
    }
    assert!(!gestures.held_up_interact(false, 1.0 / 60.0, HOLD));
    for _ in 0..100 {
        assert!(
            !gestures.held_up_interact(true, 1.0 / 60.0, HOLD),
            "the second hold resumed the first instead of starting over"
        );
    }
}

/// The hold survives the wire: it is rollback state like every other timer here.
#[test]
fn the_up_hold_survives_a_snapshot() {
    let mut state = crate::control::SlotInteractionState::default();
    let slot = crate::control::PlayerSlot(2);
    state
        .get_mut(slot)
        .expect("slot 2 is valid")
        .held_up_interact(true, 1.5, HOLD);

    let bytes = ambition_platformer2d_core::snapshot::encode_state(&state);
    let restored = ambition_platformer2d_core::snapshot::decode_state::<
        crate::control::SlotInteractionState,
    >(&bytes)
    .expect("the encoding decodes");
    assert_eq!(restored.get(slot).up_hold_timer, 1.5);
}
