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
    shake.kick(4.0, CameraShakeTuning::default());
    assert_eq!(shake.amplitude_px, 4.0);
    // Smaller kick after a big one should NOT reduce the active shake.
    shake.kick(1.0, CameraShakeTuning::default());
    assert_eq!(shake.amplitude_px, 4.0);
    // Bigger kick raises it.
    shake.kick(8.0, CameraShakeTuning::default());
    assert_eq!(shake.amplitude_px, 8.0);
}

#[test]
fn kick_clamps_at_max_amplitude() {
    let mut shake = CameraShakeState::default();
    // 1000 px shake would white out the screen; cap holds.
    shake.kick(1000.0, CameraShakeTuning::default());
    assert!(shake.amplitude_px <= 14.0);
    assert!(shake.amplitude_px > 0.0);
}

#[test]
fn kick_clamps_negative_to_zero() {
    let mut shake = CameraShakeState::default();
    shake.kick(-5.0, CameraShakeTuning::default());
    assert_eq!(shake.amplitude_px, 0.0);
}

#[test]
fn offset_bounded_by_amplitude_and_independent_axes() {
    let mut shake = CameraShakeState::default();
    shake.kick(8.0, CameraShakeTuning::default());
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
fn hard_fall_no_shake_without_a_landing_event() {
    assert_eq!(hard_fall_shake_amplitude(None), 0.0);
}

#[test]
fn hard_fall_no_shake_below_floor_vy() {
    // A soft hop (vy < HARD_FALL_SHAKE_FLOOR_VY) shouldn't shake the camera.
    assert_eq!(
        hard_fall_shake_amplitude(Some(200.0)),
        0.0,
        "soft landing → no shake"
    );
    // Right at the floor: still no shake (clamp at zero).
    assert_eq!(
        hard_fall_shake_amplitude(Some(HARD_FALL_SHAKE_FLOOR_VY)),
        0.0
    );
}

#[test]
fn hard_fall_amplitude_scales_with_excess_vy() {
    let amp_a = hard_fall_shake_amplitude(Some(HARD_FALL_SHAKE_FLOOR_VY + 60.0));
    let amp_b = hard_fall_shake_amplitude(Some(HARD_FALL_SHAKE_FLOOR_VY + 360.0));
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
    let raw = hard_fall_shake_amplitude(Some(5000.0));
    assert!(raw > 14.0, "raw amplitude exceeds cap, kick will clamp");
    let mut shake = CameraShakeState::default();
    shake.kick(raw, CameraShakeTuning::default());
    assert!(shake.amplitude_px <= 14.0);
}

/// **A REFERENCE CONNECT SHAKES NOTHING, AND A SMASH SHAKES HARD** — the whole
/// of what P4.37 asked for, expressed as a scale rather than a special case.
///
/// ⛔ before 2026-08-12 no landed hit moved this camera at any severity, so the
/// row's goal (*"a strong hit should feel materially different from a weak
/// poke"*) was carried entirely by hitlag.
///
/// ⚠ the three assertions are three different claims and none implies the
/// others: a dead zone at the reference, a real jolt at the ceiling, and
/// MONOTONICITY between them. A function that returned a constant above the
/// floor would satisfy the first two.
#[test]
fn only_a_hit_harder_than_standard_moves_the_camera() {
    // The engine's shipped reference and the `hitlag_duration` band around it:
    // floored at half, capped at 4x.
    const REFERENCE: f32 = 0.070;

    assert_eq!(
        super::hit_shake_amplitude(REFERENCE * ae::hit_response::MIN_HITLAG_SCALE, REFERENCE),
        0.0,
        "the weakest connect the hitlag law admits must not rattle the camera — \
         it is already a readable beat through hitlag, and a shake there would \
         mean the screen moves on literally every hit in the game"
    );

    // ⛔⛔ **the dead zone is the WEAKEST connect, not the reference one.** It
    // was the reference for a day, and `duel_arena` — a real authored fight —
    // measured its hardest trade at 0.0595 s against this 0.070 s reference.
    // Every hit in Ambition's own combat landed under the old dead zone, so the
    // camera could not move in the shipped game at all. This assertion is that
    // regression: an ordinary trade is a small, real jolt.
    let ordinary = super::hit_shake_amplitude(REFERENCE * 0.85, REFERENCE);
    assert!(
        ordinary > 0.0,
        "the hardest connect an authored Ambition fight actually produces \
         (0.85x reference, measured in duel_arena) shook the camera by nothing, \
         so the dead zone has climbed back above the game's real combat and \
         this feature is dead outside a growth-knockback ruleset"
    );

    let hardest = super::hit_shake_amplitude(REFERENCE * 4.0, REFERENCE);
    assert!(
        (9.0..=13.0).contains(&hardest),
        "the hardest possible connect should be a heavy jolt that still leaves \
         headroom under the {DEFAULT_CAMERA_SHAKE_MAX_PX}px cap a hard fall can \
         reach — got {hardest}"
    );
    assert!(
        hardest > ordinary * 5.0,
        "a smash ({hardest}px) must feel MATERIALLY different from an ordinary \
         trade ({ordinary}px), which is the whole of what P4.37 asked for — a \
         dead zone low enough to include everything flattens the difference just \
         as surely as one high enough to exclude everything"
    );

    // ⭐ MONOTONIC, which is the property that makes this a SCALE. Sampled
    // across the whole band rather than at two points, so a step function cannot
    // pass.
    let mut previous = 0.0;
    for step in 0..=8 {
        let scale = ae::hit_response::MIN_HITLAG_SCALE
            + (step as f32) * (4.0 - ae::hit_response::MIN_HITLAG_SCALE) / 8.0;
        let shake = super::hit_shake_amplitude(REFERENCE * scale, REFERENCE);
        assert!(
            shake >= previous,
            "a harder hit shook the camera LESS at scale {scale}: {shake} < {previous}"
        );
        previous = shake;
    }
    assert!(
        previous > 0.0,
        "the sweep ended at zero, so it measured a function that is flat at the \
         floor rather than a scale"
    );
}

/// **The reference is the ROUTE's number, not this crate's.**
///
/// ⛔ restating `0.070` here would be a second literal agreeing with
/// `Platformer2dFeelTuningMonolith::hitlag_time` by coincidence — the shape that
/// has already cost this campaign a health pool. A route that retunes its hitlag
/// must retune its camera WITH it, in the same direction, and this is what says
/// so: the same absolute freeze is a heavy hit under a snappy route and nothing
/// at all under a heavy one.
#[test]
fn a_routes_own_hitlag_decides_what_counts_as_a_hard_hit() {
    let freeze = 0.140;
    let snappy = super::hit_shake_amplitude(freeze, 0.040);
    let heavy = super::hit_shake_amplitude(freeze, 0.140);
    assert!(
        snappy > heavy,
        "a 0.140s freeze is 3.5x reference under a snappy route and merely \
         standard under a heavy one, so it must shake the snappy route's camera \
         HARDER — got {snappy}px vs {heavy}px, which means the route's reference \
         is not reaching the law"
    );
    assert_eq!(
        super::hit_shake_amplitude(0.140 * ae::hit_response::MIN_HITLAG_SCALE, 0.140),
        0.0,
        "and the dead zone travels with the route too: the weakest connect a \
         HEAVY route admits must be as silent as the weakest a snappy one does"
    );
}
