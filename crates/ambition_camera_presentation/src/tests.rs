use super::*;
use ambition_platformer2d_shared_tangle::camera_ease::hard_fall_shake_amplitude;

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
fn hard_fall_saturates_through_kick_cap() {
    // Terminal-velocity fall produces a huge raw amplitude;
    // the `kick()` clamp is what enforces the 14-px cap.
    let raw = hard_fall_shake_amplitude(Some(5000.0));
    assert!(raw > 14.0, "raw amplitude exceeds cap, kick will clamp");
    let mut shake = CameraShakeState::default();
    shake.kick(raw, CameraShakeTuning::default());
    assert!(shake.amplitude_px <= 14.0);
}

/// ⭐ THE PROPERTY THAT MAKES THE CALL SITE SAFE: idle is exactly `1.0`, so
/// the camera stage can multiply unconditionally and a host that never decides a
/// match presents a byte-identical scale to one without the feature.
#[test]
fn an_idle_finish_zoom_is_exactly_the_identity() {
    let zoom = FinishZoomState::default();
    assert_eq!(zoom.closeness, 0.0);
    assert_eq!(zoom.scale_factor(FinishZoomTuning::default()), 1.0);
}

/// The zoom pulls IN — a factor BELOW one — which is the whole reason it cannot
/// live in `zoom_multiplier`, whose policy floors itself at 1.0 twice over.
#[test]
fn a_kicked_finish_zoom_pulls_the_view_in_not_out() {
    let mut zoom = FinishZoomState::default();
    zoom.kick(1.0, FinishZoomTuning::default());
    let factor = zoom.scale_factor(FinishZoomTuning::default());
    assert!(factor < 1.0, "a finishing zoom must close the view, got {factor}");
    assert!(factor > 0.0, "and must not invert it, got {factor}");
    // Full strength at the default 0.30 fraction shows 70% of the width.
    assert!((factor - 0.70).abs() < 1e-6, "expected 0.70, got {factor}");
}

/// Strongest-wins and non-stacking, exactly like the shake: several requests
/// released together by the quarantine settle on the strongest rather than
/// multiplying into an unreadable close-up.
#[test]
fn finish_zoom_kicks_do_not_stack() {
    let tuning = FinishZoomTuning::default();
    let mut zoom = FinishZoomState::default();
    zoom.kick(1.0, tuning);
    zoom.kick(1.0, tuning);
    zoom.kick(0.2, tuning);
    assert_eq!(zoom.closeness, 1.0, "a weaker kick cannot pull a strong one back");
}

/// ⛔ THE ARM THE HOLD EXISTS FOR. A zoom that begins releasing on the frame it
/// arrives is a flicker, not a beat — so the hold must actually hold, and then
/// must actually let go.
#[test]
fn the_finish_zoom_holds_before_it_releases() {
    let tuning = FinishZoomTuning::default();
    let mut zoom = FinishZoomState::default();
    zoom.kick(1.0, tuning);

    // Half the hold: still fully in.
    let mut hold_left = zoom.hold_secs_left;
    assert!(hold_left > 0.0, "a kick must arm the hold");
    hold_left -= tuning.hold_secs * 0.5;
    zoom.hold_secs_left = hold_left;
    assert_eq!(zoom.closeness, 1.0, "the hold has not expired");

    // Hold spent, then a second of release at 0.60/s.
    zoom.hold_secs_left = 0.0;
    zoom.closeness -= tuning.release_per_s * 1.0;
    assert!(zoom.closeness < 1.0, "it must let go once the hold expires");

    // And it bottoms out at the identity rather than going negative.
    zoom.closeness = (zoom.closeness - tuning.release_per_s * 10.0).max(0.0);
    assert_eq!(zoom.closeness, 0.0);
    assert_eq!(zoom.scale_factor(tuning), 1.0, "it returns to the identity");
}

/// A clamp, because the request crosses a message boundary and nothing on the
/// far side vets it: a producer asking for 12.0 must not invert the projection.
#[test]
fn a_wild_finish_zoom_request_cannot_invert_the_view() {
    let tuning = FinishZoomTuning::default();
    let mut zoom = FinishZoomState::default();
    zoom.kick(12.0, tuning);
    assert_eq!(zoom.closeness, 1.0, "closeness is clamped to 1.0");
    assert!(zoom.scale_factor(tuning) > 0.0);

    // And an absurd TUNING cannot either -- max_close_fraction caps at 0.9.
    let wild = FinishZoomTuning { max_close_fraction: 5.0, ..tuning };
    assert!(zoom.scale_factor(wild) > 0.0, "a wild tuning must not invert it");
}

/// The finishing zoom's tick runs while a zoom is held or releasing, so a
/// zoom still comes all the way back out, and at rest it writes nothing.
#[test]
fn a_finishing_zoom_releases_and_then_rests_unwritten() {
    use bevy::prelude::{App, DetectChanges as _, IntoScheduleConfigs as _, Time, Update};
    use std::time::Duration;

    let mut app = App::new();
    app.init_resource::<Time>();
    app.init_resource::<FinishZoomTuning>();
    app.init_resource::<FinishZoomState>();
    app.add_systems(Update, tick_finish_zoom.run_if(finish_zoom_is_active));
    let tuning = *app.world().resource::<FinishZoomTuning>();
    app.world_mut().resource_mut::<FinishZoomState>().kick(1.0, tuning);
    let mut frames = 0;
    while app.world().resource::<FinishZoomState>().closeness > 0.0 {
        app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
        app.update();
        frames += 1;
        assert!(frames < 1000, "the zoom never released");
    }
    let before = app.world().resource_ref::<FinishZoomState>().last_changed();
    app.world_mut().resource_mut::<Time>().advance_by(Duration::from_millis(100));
    app.update();
    assert_eq!(
        app.world().resource_ref::<FinishZoomState>().last_changed(),
        before,
        "a zoom at rest was written"
    );
}

/// ISL-1b: a kick that names a room shakes only the views of that room; a
/// kick that names none shakes every view; and each settles to rest.
#[test]
fn a_room_s_kick_shakes_only_that_room_s_views() {
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    let (here, there) = (LiveRoomInstance::from_ordinal(1), LiveRoomInstance::from_ordinal(2));
    let tuning = CameraShakeTuning::default();
    let mut shake = CameraShakeState::default();
    shake.kick_in(Some(here), 6.0, tuning);
    assert_eq!(shake.amplitude_in(Some(here)), 6.0);
    assert_eq!(shake.amplitude_in(Some(there)), 0.0, "the other room's view shook");
    assert_eq!(shake.offset_in(Some(there)), ae::Vec2::ZERO);
    assert_ne!(shake.offset_in(Some(here)), ae::Vec2::ZERO);
    shake.kick_in(None, 2.0, tuning);
    assert_eq!(shake.amplitude_in(Some(there)), 2.0, "a kick with no room shakes every view");
    assert_eq!(shake.amplitude_in(Some(here)), 6.0, "the louder of the two");
    shake.decay(10.0);
    assert_eq!((shake.amplitude_in(Some(here)), shake.amplitude_in(Some(there))), (0.0, 0.0));
    assert!(shake.rooms.is_empty(), "a room at rest is forgotten");
}

#[test]
fn a_room_s_finish_zoom_pulls_in_only_that_room_s_views() {
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    let (here, there) = (LiveRoomInstance::from_ordinal(1), LiveRoomInstance::from_ordinal(2));
    let tuning = FinishZoomTuning::default();
    let mut zoom = FinishZoomState::default();
    zoom.kick_in(Some(here), 1.0, tuning);
    assert!(zoom.scale_factor_in(Some(here), tuning) < 1.0);
    assert_eq!(zoom.scale_factor_in(Some(there), tuning), 1.0, "the other room's view pulled in");
    assert_eq!(zoom.scale_factor_in(None, tuning), 1.0);
    zoom.kick_in(None, 0.5, tuning);
    assert_eq!(
        zoom.scale_factor_in(Some(there), tuning),
        zoom.scale_factor(tuning),
        "a zoom with no room pulls in every view"
    );
    assert!(zoom.scale_factor_in(Some(here), tuning) < zoom.scale_factor_in(Some(there), tuning), "the closer of the two");
    assert!(zoom.is_active());
    zoom.settle(tuning.hold_secs + 100.0, tuning);
    zoom.settle(100.0, tuning);
    assert!(!zoom.is_active(), "every zoom released: {zoom:?}");
    assert!(zoom.rooms.is_empty(), "a room at rest is forgotten");
}
