
use super::{ease_fold_amount, MAX_FOLD_EASE_DT};

/// A single huge-delta frame (the un-pause hitch on close) must NOT collapse the
/// fold to ~0 in one step. Before the `dt` clamp this snapped straight past the
/// host's `amount > 0.08` visibility cutoff, so the close read as an instant snap.
#[test]
fn one_hitchy_close_frame_does_not_snap_past_the_visibility_cutoff() {
    // Close rate = open_close_speed(8) * close_speed_scale(2) = 16; a 250 ms hitch
    // (Bevy's default Time max delta) is the worst realistic spike.
    let rate = 16.0;
    let after = ease_fold_amount(1.0, 0.0, rate, 0.250);
    assert!(
        after > 0.08,
        "one hitchy close frame snapped the fold past the host cutoff: amount={after}"
    );
}

/// The clamp does not change normal-frame easing: a 16 ms frame advances exactly
/// as the unclamped formula would (the clamp only bites above MAX_FOLD_EASE_DT).
#[test]
fn normal_frames_are_unaffected_by_the_clamp() {
    let rate = 16.0;
    let dt = 1.0 / 60.0;
    assert!(dt < MAX_FOLD_EASE_DT);
    let after = ease_fold_amount(1.0, 0.0, rate, dt);
    let expected = 1.0 + (0.0 - 1.0) * (1.0 - (-rate * dt).exp());
    assert!(
        (after - expected).abs() < 1e-6,
        "after={after} expected={expected}"
    );
}

/// The fold still completes: stepping at the clamped max dt converges to target.
#[test]
fn fold_still_converges_to_target() {
    let rate = 16.0;
    let mut amount = 1.0;
    for _ in 0..30 {
        amount = ease_fold_amount(amount, 0.0, rate, MAX_FOLD_EASE_DT);
    }
    assert_eq!(amount, 0.0, "fold did not converge: amount={amount}");
}
