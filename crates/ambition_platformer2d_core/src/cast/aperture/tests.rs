use super::*;
use crate::frame::{PortalAperture, PortalFrame};
use crate::{Aabb, Block, SurfaceChain, World};

fn aperture(x: f32, normal: Vec2, half_length: f32) -> PortalAperture {
    PortalAperture {
        frame: PortalFrame::fixed(Vec2::new(x, 0.0), normal),
        half_length,
    }
}

fn pair() -> [PortalLink; 2] {
    PortalLink::both_ways(
        0,
        (
            aperture(10.0, -Vec2::X, 20.0),
            aperture(100.0, Vec2::X, 10.0),
        ),
    )
}

fn world(blocks: Vec<Block>) -> World {
    World::new("scaled ray", Vec2::splat(1000.0), Vec2::ZERO, blocks)
}

fn wall(x: f32) -> Block {
    Block::solid("wall", Vec2::new(x, -40.0), Vec2::new(10.0, 80.0))
}

fn ray(reach: f32) -> PortalRay {
    PortalRay { width: 0.0,
        origin: Vec2::ZERO,
        direction: Vec2::X,
        reach,
    }
}
fn budget() -> RayBudget {
    PieceBudget::COMPOSED.into()
}

#[test]
fn a_ray_band_keeps_its_width_after_shrinking_and_expanding() {
    for convention in [MapConvention::Reflection, MapConvention::Rotation] {
        for (origin, direction, target, expected) in [
            (Vec2::ZERO, Vec2::X, Block::solid("far edge", Vec2::new(105.0, 0.75), Vec2::new(2.0, 2.0)), 20.0),
            (Vec2::new(104.0, 0.0), -Vec2::X, Block::solid("far edge", Vec2::new(-6.0, 3.0), Vec2::new(2.0, 2.0)), 11.0),
        ] {
            let world = world(vec![wall(10.0), wall(90.0), target]);
            let beam = PortalRay { origin, direction, reach: 40.0, width: 4.0 };
            let hit = trace_through_apertures(&world, &pair(), beam, false, budget(), convention).unwrap();
            assert_eq!(hit.distance, expected, "{convention:?}: {hit:?}");
            assert!(hit.hit.is_some());
            let narrow = PortalRay { width: 0.25, ..beam };
            assert_eq!(trace_through_apertures(&world, &pair(), narrow, false, budget(), convention).unwrap().distance, 40.0);
        }
    }
}

#[test]
fn a_ray_band_cannot_hide_its_edges_in_an_aperture() {
    let links = PortalLink::both_ways(0, (aperture(10.0, -Vec2::X, 0.5), aperture(100.0, Vec2::X, 0.5)));
    let beam = PortalRay { width: 4.0, ..ray(40.0) };
    let trace = trace_through_apertures(&world(vec![wall(10.0), wall(90.0)]), &links, beam, false, budget(), MapConvention::Reflection).unwrap();
    assert_eq!(trace.distance, 10.0, "{trace:?}");
    assert_eq!(trace.hit.unwrap().0.x, 10.0);
}

#[test]
fn a_ray_spends_initial_chart_reach_after_scaling_and_starts_on_the_exit_face() {
    let links = pair();
    // The exit host ends at x=100. The target is only a quarter unit away.
    let world = world(vec![wall(10.0), wall(90.0), wall(100.25)]);
    let trace = trace_through_apertures(
        &world,
        &links,
        ray(40.0),
        false,
        budget(),
        MapConvention::Reflection,
    )
    .unwrap();
    assert_eq!(trace.path, vec![0]);
    assert_eq!(trace.distance, 10.5);
    assert_eq!(trace.hit, Some((Vec2::new(100.25, 0.0), -Vec2::X)));
    let short = trace_through_apertures(
        &world,
        &links,
        ray(10.49),
        false,
        budget(),
        MapConvention::Reflection,
    )
    .unwrap();
    assert_eq!(short.hit, None);
    assert_eq!(short.distance, 10.49);
    assert_eq!(short.path, vec![0]);
}

#[test]
fn the_reverse_link_expands_the_remaining_reach() {
    let world = world(vec![wall(-10.0), wall(10.0), wall(90.0)]);
    let ray = PortalRay { width: 0.0,
        origin: Vec2::new(110.0, 0.0),
        direction: -Vec2::X,
        reach: 30.0,
    };
    let trace = trace_through_apertures(
        &world,
        &pair(),
        ray,
        false,
        budget(),
        MapConvention::Reflection,
    )
    .unwrap();
    assert_eq!(trace.path, vec![1]);
    assert_eq!(trace.distance, 15.0);
    assert_eq!(trace.hit, Some((Vec2::ZERO, Vec2::X)));
}

#[test]
fn composed_reach_uses_the_scale_on_each_segment() {
    let mut links = pair().to_vec();
    links.extend(PortalLink::both_ways(
        1,
        (
            aperture(105.0, -Vec2::X, 10.0),
            aperture(200.0, Vec2::X, 20.0),
        ),
    ));
    let world = world(vec![
        wall(10.0),
        wall(90.0),
        wall(105.0),
        wall(190.0),
        wall(210.0),
    ]);
    let trace = trace_through_apertures(
        &world,
        &links,
        ray(30.0),
        false,
        budget(),
        MapConvention::Reflection,
    )
    .unwrap();
    assert_eq!(trace.path, vec![0, 2]);
    assert_eq!(trace.distance, 30.0);
    assert_eq!(trace.hit, Some((Vec2::new(210.0, 0.0), -Vec2::X)));
    let mut one = budget();
    one.max_hops = 1;
    assert_eq!(
        trace_through_apertures(
            &world,
            &links,
            ray(30.0),
            false,
            one,
            MapConvention::Reflection
        ),
        Err(Refusal {
            path: vec![0],
            link: Some(2),
            reason: RefusalReason::HopLimit
        })
    );
    one.max_hops = 0;
    assert_eq!(
        trace_through_apertures(
            &world,
            &links,
            ray(30.0),
            false,
            one,
            MapConvention::Reflection
        ),
        Err(Refusal {
            path: vec![],
            link: Some(0),
            reason: RefusalReason::HopLimit
        })
    );
}

#[test]
fn a_scale_cycle_refuses_at_the_limit_with_no_partial_ray() {
    let link = PortalLink::new(aperture(10.0,-Vec2::X,10.0),aperture(0.0,Vec2::X,20.0),7);
    let budget = RayBudget {
        max_hops: 100,
        max_scale_exponent: 2,
        max_pieces: 32,
    };
    assert_eq!(
        trace_through_apertures(
            &world(vec![]),
            &[link],
            ray(1000.0),
            false,
            budget,
            MapConvention::Reflection
        ),
        Err(Refusal {
            path: vec![7, 7],
            link: Some(7),
            reason: RefusalReason::ScaleLimit
        })
    );
}

#[test]
fn tied_apertures_use_the_directed_link_key() {
    let mut a = pair()[0];
    a.key = 8;
    let mut b = a;
    b.key = 3;
    b.exit.frame.origin.x = 300.0;
    b.exit_step = crate::frame::ApertureStep::fixed(b.exit.frame.origin, b.exit.frame.normal);
    for links in [[a, b], [b, a]] {
        let trace = trace_through_apertures(
            &world(vec![]),
            &links,
            ray(11.0),
            false,
            budget(),
            MapConvention::Reflection,
        )
        .unwrap();
        assert_eq!(trace.path, vec![3]);
    }
}

#[test]
fn invalid_maps_and_scale_policy_have_distinct_refusals() {
    let mut links = pair();
    let mut cap = budget();
    cap.max_scale_exponent = 0;
    let refused = trace_through_apertures(
        &world(vec![]),
        &links,
        ray(40.0),
        false,
        cap,
        MapConvention::Reflection,
    )
    .unwrap_err();
    assert_eq!(refused.reason, RefusalReason::ScaleLimit);
    links[0].exit.half_length = f32::NAN;
    let refused = trace_through_apertures(
        &world(vec![]),
        &links,
        ray(40.0),
        false,
        budget(),
        MapConvention::Reflection,
    )
    .unwrap_err();
    assert_eq!(
        refused,
        Refusal {
            path: vec![],
            link: Some(0),
            reason: RefusalReason::Numeric(NumericFault::NonFinite)
        }
    );
}

#[test]
fn an_aperture_does_not_take_a_ray_outside_its_opening() {
    let ray = PortalRay { width: 0.0,
        origin: Vec2::new(0.0, 21.0),
        ..ray(40.0)
    };
    let trace = trace_through_apertures(
        &world(vec![wall(10.0)]),
        &pair(),
        ray,
        false,
        budget(),
        MapConvention::Reflection,
    )
    .unwrap();
    assert!(trace.path.is_empty());
    assert_eq!(trace.distance, 10.0);
    assert_eq!(trace.hit, Some((Vec2::new(10.0, 21.0), -Vec2::X)));
}

#[test]
fn rotated_scaled_rays_stop_on_the_same_one_sided_surface() {
    use bevy_math::Mat2;
    for scale in [0.001, 1.0, 1000.0] {
        for reflected in [false, true] {
            for convention in [MapConvention::Reflection, MapConvention::Rotation] {
                let mut q = Mat2::from_angle(0.73);
                if reflected {
                    q.x_axis = -q.x_axis;
                }
                let map = Sim2::new(scale, q, Vec2::new(30.0, -25.0)).unwrap();
                let links = pair().map(|mut link| {
                    for aperture in [&mut link.enter, &mut link.exit] {
                        aperture.frame.origin = map.point(aperture.frame.origin).unwrap();
                        aperture.frame.normal = map.direction(aperture.frame.normal);
                        aperture.half_length = map.length(aperture.half_length).unwrap();
                    }
                    PortalLink::new(link.enter, link.exit, link.key)
                });
                let mut points = vec![
                    map.point(Vec2::new(105.0, 5.0)).unwrap(),
                    map.point(Vec2::new(105.0, -5.0)).unwrap(),
                ];
                if reflected {
                    points.reverse();
                }
                let world = world(vec![]).with_chains(vec![SurfaceChain::open("target", points)]);
                let ray = PortalRay { width: 0.0,
                    origin: map.point(Vec2::ZERO).unwrap(),
                    direction: map.direction(Vec2::X),
                    reach: map.length(40.0).unwrap(),
                };
                let trace =
                    trace_through_apertures(&world, &links, ray, false, budget(), convention)
                        .unwrap();
                assert_eq!(trace.path, vec![0]);
                assert!(
                    (trace.distance / scale - 20.0).abs() < 0.02,
                    "{scale} {reflected} {convention:?}: {trace:?}"
                );
                let (hit, normal) = trace.hit.unwrap();
                assert!(hit.distance(map.point(Vec2::new(105.0, 0.0)).unwrap()) / scale < 0.02);
                assert!(normal.dot(map.direction(-Vec2::X)) > 0.999);
            }
        }
    }
}

#[test]
fn rays_on_a_box_boundary_can_leave_and_slide_but_cannot_enter() {
    let bounds = Aabb::new(Vec2::ZERO, Vec2::ONE);
    for (origin, direction) in [
        (Vec2::X, Vec2::X),
        (Vec2::X, Vec2::Y),
        (Vec2::new(1.0, 1.0), Vec2::ONE),
    ] {
        assert_eq!(super::super::ray_aabb(origin, direction, bounds), None);
    }
    assert_eq!(
        super::super::ray_aabb(Vec2::X, -Vec2::X, bounds),
        Some((0.0, Vec2::X))
    );
}

#[test]
fn an_angled_ray_does_not_round_back_into_its_exit_host() {
    let entry = 758.293212890625_f32;
    let exit = 2871.050537109375_f32;
    let links = PortalLink::both_ways(
        0,
        (
            aperture(entry, -Vec2::X, 4000.0),
            aperture(exit, Vec2::X, 2300.0),
        ),
    );
    let wall = |x| Block::solid("host", Vec2::new(x, -20000.0), Vec2::new(10.0, 40000.0));
    let world = world(vec![wall(entry), wall(exit - 10.0), wall(exit + 5.0)]);
    let ray = PortalRay { width: 0.0,
        origin: Vec2::new(-3.5724475383758545, 0.0),
        direction: Vec2::new(0.13619284331798553, 0.19575506448745728),
        reach: 5000.0,
    };
    let trace = trace_through_apertures(
        &world,
        &links,
        ray,
        false,
        budget(),
        MapConvention::Reflection,
    )
    .unwrap();
    assert_eq!(trace.path, vec![0]);
    assert_eq!(
        trace.hit.unwrap().0.x,
        exit + 5.0,
        "the ray must clear the exit host: {trace:?}"
    );
}
