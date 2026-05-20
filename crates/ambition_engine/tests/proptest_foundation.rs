use ambition_engine::{aabb_from_min_size, AabbExt, BossPatternSchedule, KinematicPath, Vec2};
use proptest::prelude::*;

proptest! {
    #[test]
    fn generated_aabb_contains_its_min_corner(
        min_x in -10_000.0f32..10_000.0,
        min_y in -10_000.0f32..10_000.0,
        size_x in 0.1f32..10_000.0,
        size_y in 0.1f32..10_000.0,
    ) {
        let min = Vec2::new(min_x, min_y);
        let size = Vec2::new(size_x, size_y);
        let aabb = aabb_from_min_size(min, size);
        prop_assert!(aabb.left() <= min_x + 0.001);
        prop_assert!(aabb.top() <= min_y + 0.001);
        prop_assert!(aabb.right() >= min_x + size_x - 0.001);
        prop_assert!(aabb.bottom() >= min_y + size_y - 0.001);
        prop_assert!(aabb.center().x.is_finite());
        prop_assert!(aabb.center().y.is_finite());
        prop_assert!(aabb.half_size().x.is_finite());
        prop_assert!(aabb.half_size().y.is_finite());
    }

    #[test]
    fn kinematic_path_validity_requires_two_points_and_positive_speed(
        ax in -1000.0f32..1000.0,
        ay in -1000.0f32..1000.0,
        bx in -1000.0f32..1000.0,
        by in -1000.0f32..1000.0,
        speed in 0.1f32..1000.0,
    ) {
        let path = KinematicPath::line(Vec2::new(ax, ay), Vec2::new(bx, by), speed);
        prop_assert!(path.is_valid());
        prop_assert_eq!(path.points.len(), 2);
        prop_assert!(path.speed > 0.0);
    }
}

#[test]
fn boss_pattern_schedules_have_finite_positive_timings() {
    for schedule in [
        BossPatternSchedule::gradient_sentinel_phase1(),
        BossPatternSchedule::gradient_sentinel_phase2(),
    ] {
        assert!(
            schedule.is_valid(),
            "{} phase {} should be valid",
            schedule.boss_id,
            schedule.phase
        );
        assert!(schedule.total_time().is_finite());
        assert!(schedule.total_time() > 0.0);
    }
}

