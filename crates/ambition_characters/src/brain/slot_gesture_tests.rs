//! **An invalid participant identifier cannot write to a real participant.**
//!
//! The old `get_mut` clamped an out-of-range [`PlayerSlot`] onto the LAST valid
//! one, so `PlayerSlot(9)` and `PlayerSlot(3)` were the same controller. This
//! pins the replacement: invalidity is `None`, and every real slot is left
//! exactly as it was.

use super::{SlotGestures, SlotInteractionState};
use super::{PlayerSlot, SlotControls};

/// A gesture state that is visibly non-default in every slot, so a stray write
/// anywhere shows up as a difference rather than as a value that happened to
/// match.
fn distinctly_filled() -> SlotInteractionState {
    let mut state = SlotInteractionState::default();
    for index in 0..SlotControls::MAX_SLOTS {
        let gestures = state
            .get_mut(PlayerSlot(index as u8))
            .expect("every slot below MAX_SLOTS is valid");
        gestures.interact_buffer_timer = 1.0 + index as f32;
        gestures.double_tap_up_pending = true;
    }
    state
}

/// **THE INVARIANT.** An out-of-range slot resolves to nothing, and nothing in
/// the array moves.
#[test]
fn an_invalid_slot_cannot_alter_any_real_participant() {
    let before = distinctly_filled();
    let mut after = before;

    // ⚠ **the zero floor**: a `MAX_SLOTS` of 0 would make the loop below empty
    // and every assertion vacuous, and the fixture above would have written
    // nothing to compare.
    assert!(
        SlotControls::MAX_SLOTS >= 2,
        "this test needs at least two real slots to tell a clamp from a refusal"
    );

    for out_of_range in [
        SlotControls::MAX_SLOTS,
        SlotControls::MAX_SLOTS + 1,
        u8::MAX as usize,
    ] {
        let slot = PlayerSlot(out_of_range as u8);
        assert!(
            after.get_mut(slot).is_none(),
            "slot {out_of_range} is out of range but resolved to a real controller"
        );
        // A read of the same slot is a different question with a defensible
        // answer: a controller that does not exist is pressing nothing.
        assert_eq!(
            after.get(slot),
            SlotGestures::default(),
            "an out-of-range READ should report a controller pressing nothing"
        );
    }

    for index in 0..SlotControls::MAX_SLOTS {
        let slot = PlayerSlot(index as u8);
        assert_eq!(
            after.get(slot),
            before.get(slot),
            "slot {index}'s gestures changed while only INVALID slots were addressed — \
             this is the clamp bug: an out-of-range participant identifier was resolved \
             onto a real participant and consumed its buffered input"
        );
    }
}

/// The clamp's specific victim, named: the LAST valid slot is what an
/// out-of-range write used to land on.
#[test]
fn the_last_valid_slot_is_not_a_dumping_ground_for_bad_indices() {
    let last = PlayerSlot((SlotControls::MAX_SLOTS - 1) as u8);
    let mut state = distinctly_filled();
    let before_last = state.get(last);
    assert!(
        before_last != SlotGestures::default(),
        "the fixture never wrote to the last slot, so the assertion below could not fail"
    );

    if let Some(gestures) = state.get_mut(PlayerSlot(SlotControls::MAX_SLOTS as u8 + 7)) {
        gestures.reset();
    }

    assert_eq!(
        state.get(last),
        before_last,
        "a write addressed to a slot that does not exist landed on the last valid \
         participant"
    );
}
