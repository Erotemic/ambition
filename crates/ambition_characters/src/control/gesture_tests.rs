//! Slot gesture timing and interaction buffering.

use super::SlotGestures;

#[test]
fn register_down_tap_returns_true_on_double_tap_within_window() {
    let mut interaction = SlotGestures::default();
    // First tap: returns false, opens the window.
    assert!(!interaction.register_down_tap(true, 0.0, 0.25));
    // Tap again before window expires: returns true.
    assert!(interaction.register_down_tap(true, 0.05, 0.25));
}

#[test]
fn register_down_tap_window_closes_on_idle_frames() {
    let mut interaction = SlotGestures::default();
    assert!(!interaction.register_down_tap(true, 0.0, 0.25));
    // Many idle frames — tap timer drains.
    for _ in 0..20 {
        let _ = interaction.register_down_tap(false, 0.05, 0.25);
    }
    // Next tap is treated as the FIRST tap (window had closed).
    assert!(!interaction.register_down_tap(true, 0.0, 0.25));
}

#[test]
fn register_up_tap_mirrors_down_tap_semantics() {
    let mut interaction = SlotGestures::default();
    assert!(!interaction.register_up_tap(true, 0.0, 0.30));
    assert!(interaction.register_up_tap(true, 0.05, 0.30));
}

#[test]
fn buffered_interact_holds_for_window_seconds() {
    let mut interaction = SlotGestures::default();
    // Press once → buffer holds for `window` seconds.
    assert!(interaction.buffered_interact(true, 0.0, 0.12));
    // Subsequent frames within the window also report true.
    assert!(interaction.buffered_interact(false, 0.05, 0.12));
    assert!(interaction.buffered_interact(false, 0.05, 0.12));
    // After the window passes, the buffer drains.
    assert!(!interaction.buffered_interact(false, 0.20, 0.12));
}

#[test]
fn clear_interact_buffer_drops_buffer_immediately() {
    let mut interaction = SlotGestures::default();
    let _ = interaction.buffered_interact(true, 0.0, 1.0);
    // Without the clear, next frame would still report true.
    interaction.clear();
    assert!(!interaction.buffered_interact(false, 0.001, 1.0));
}
