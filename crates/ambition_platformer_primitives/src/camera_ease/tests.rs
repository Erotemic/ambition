//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

#[test]
fn shake_starts_at_zero_amplitude() {
    let shake = CameraShakeState::default();
    assert_eq!(shake.amplitude_px, 0.0);
    // Zero amplitude → exactly Vec2::ZERO offset (no jitter at rest).
    assert_eq!(shake.offset(), ae::Vec2::ZERO);
}

#[test]
fn kick_max_wins_no_stacking() {
    let mut shake = CameraShakeState::default();
    shake.kick(4.0);
    assert_eq!(shake.amplitude_px, 4.0);
    // Smaller kick after a big one should NOT reduce the active shake.
    shake.kick(1.0);
    assert_eq!(shake.amplitude_px, 4.0);
    // Bigger kick raises it.
    shake.kick(8.0);
    assert_eq!(shake.amplitude_px, 8.0);
}

#[test]
fn kick_clamps_at_max_amplitude() {
    let mut shake = CameraShakeState::default();
    // 1000 px shake would white out the screen; cap holds.
    shake.kick(1000.0);
    assert!(shake.amplitude_px <= 14.0);
    assert!(shake.amplitude_px > 0.0);
}

#[test]
fn kick_clamps_negative_to_zero() {
    let mut shake = CameraShakeState::default();
    shake.kick(-5.0);
    assert_eq!(shake.amplitude_px, 0.0);
}

#[test]
fn offset_bounded_by_amplitude_and_independent_axes() {
    let mut shake = CameraShakeState::default();
    shake.kick(8.0);
    // Sample several seeds; both axes must stay inside ±amplitude.
    let mut max_x: f32 = 0.0;
    let mut max_y: f32 = 0.0;
    for s in 0..32u32 {
        shake.seed = s;
        let o = shake.offset();
        assert!(
            o.x.abs() <= shake.amplitude_px + 0.001,
            "x={} exceeded amp",
            o.x
        );
        assert!(
            o.y.abs() <= shake.amplitude_px + 0.001,
            "y={} exceeded amp",
            o.y
        );
        max_x = max_x.max(o.x.abs());
        max_y = max_y.max(o.y.abs());
    }
    // Both axes should produce non-trivial offsets across 32 seeds
    // (otherwise the xorshift is degenerate / x and y would be
    // correlated into a diagonal shake).
    assert!(max_x > 1.0, "x range too small: {max_x}");
    assert!(max_y > 1.0, "y range too small: {max_y}");
}

#[test]
fn offset_below_dead_zone_is_zero() {
    let mut shake = CameraShakeState::default();
    shake.amplitude_px = 0.04; // below 0.05 dead-zone
    assert_eq!(shake.offset(), ae::Vec2::ZERO);
}

#[test]
fn hard_fall_no_shake_when_already_grounded() {
    // Player was already grounded last frame → no landing → no shake.
    assert_eq!(
        hard_fall_shake_amplitude(true, true, 800.0),
        0.0,
        "no transition → no shake"
    );
}

#[test]
fn hard_fall_no_shake_when_still_airborne() {
    // Was airborne and still airborne → no landing → no shake.
    assert_eq!(
        hard_fall_shake_amplitude(false, false, 800.0),
        0.0,
        "no landing → no shake"
    );
}

#[test]
fn hard_fall_no_shake_below_floor_vy() {
    // A soft hop (vy < HARD_FALL_SHAKE_FLOOR_VY) shouldn't shake the camera.
    assert_eq!(
        hard_fall_shake_amplitude(false, true, 200.0),
        0.0,
        "soft landing → no shake"
    );
    // Right at the floor: still no shake (clamp at zero).
    assert_eq!(
        hard_fall_shake_amplitude(false, true, HARD_FALL_SHAKE_FLOOR_VY),
        0.0
    );
}

#[test]
fn hard_fall_amplitude_scales_with_excess_vy() {
    let amp_a = hard_fall_shake_amplitude(false, true, HARD_FALL_SHAKE_FLOOR_VY + 60.0);
    let amp_b = hard_fall_shake_amplitude(false, true, HARD_FALL_SHAKE_FLOOR_VY + 360.0);
    assert!(amp_a > 0.0, "kick fires above floor_vy");
    assert!(amp_b > amp_a, "bigger fall → bigger amplitude");
    // Amplitude scales linearly with excess: 6× the excess → 6× the kick.
    let ratio = amp_b / amp_a;
    assert!(
        (ratio - 6.0).abs() < 0.01,
        "ratio should be ~6.0, got {ratio}"
    );
}

#[test]
fn hard_fall_saturates_through_kick_cap() {
    // Terminal-velocity fall produces a huge raw amplitude;
    // the `kick()` clamp is what enforces the 14-px cap.
    let raw = hard_fall_shake_amplitude(false, true, 5000.0);
    assert!(raw > 14.0, "raw amplitude exceeds cap, kick will clamp");
    let mut shake = CameraShakeState::default();
    shake.kick(raw);
    assert!(shake.amplitude_px <= 14.0);
}
