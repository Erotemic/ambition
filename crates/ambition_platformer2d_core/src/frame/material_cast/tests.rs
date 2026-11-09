use super::*;
use crate::frame::thread::{PieceBudget, PortalLink};
use crate::frame::{MapConvention, PortalAperture, PortalFrame};
use bevy_math::Mat2;

fn box_at(center: Vec2, half: Vec2) -> Vec<Vec2> {
    corners(Aabb::new(center, half)).to_vec()
}

#[test]
fn a_new_cell_owns_contact_at_a_topology_boundary() {
    let body = CombatVolume::circle(Vec2::ZERO, 1.0);
    let mut cells = MaterialCast { material: MaterialPrimitive::world(&body),
        root: Vec2::ZERO, delta: Vec2::new(8.0, 0.0), time: 0.0..1.0 }.cells(None).unwrap();
    let obstacle = box_at(Vec2::new(5.0, 0.0), Vec2::ONE);
    cells.parts[0].time = 0.0..0.375;
    assert!(cells.against(&cells.parts[0], &obstacle, Vec2::ZERO).unwrap().is_none());
    cells.parts[0].time = 0.375..1.0;
    let hit = cells.against(&cells.parts[0], &obstacle, Vec2::ZERO).unwrap().unwrap();
    assert!((hit.toi - 0.375).abs() < 1.0e-12);
    let end_contact = box_at(Vec2::new(10.0, 0.0), Vec2::ONE);
    let hit = cells.against(&cells.parts[0], &end_contact, Vec2::ZERO).unwrap().unwrap();
    assert_eq!(hit.toi, 1.0);
}

#[test]
fn a_cells_map_travel_sets_the_physical_contact_pose() {
    let shape = CombatVolume::circle(Vec2::ZERO, 1.0);
    let mut cells = MaterialCast {
        material: MaterialPrimitive::world(&shape), root: Vec2::ZERO,
        delta: Vec2::ZERO, time: 0.0..1.0,
    }.cells(None).unwrap();
    cells.parts[0].travel = DVec2::new(20.0, 0.0);
    let cell = &cells.parts()[0];
    let bounds = cells.bounds(cell).unwrap().unwrap();
    assert_eq!(bounds, Aabb { min: Vec2::splat(-1.0), max: Vec2::new(21.0, 1.0) });
    let obstacle = box_at(Vec2::new(10.0, 0.0), Vec2::new(1.0, 2.0));
    let hit = cells.against(cell, &obstacle, Vec2::ZERO).unwrap().unwrap();
    assert!((hit.toi - 0.4).abs() < 1.0e-12);
    assert_eq!(hit.normal, -Vec2::X);
    assert!(hit.point.distance(Vec2::X) < 1.0e-6);
    assert!(hit.world_point.distance(Vec2::new(9.0, 0.0)) < 1.0e-6);
    assert!(hit.map.point(hit.point).unwrap().distance(hit.world_point) < 1.0e-6);
}

#[test]
fn a_translating_cast_keeps_the_contact_in_canonical_material() {
    let shape = CombatVolume::circle(Vec2::ZERO, 1.0);
    for scale in [0.5, 1.0, 2.0, 1000.0] {
        for parity in [-1.0, 1.0] {
            let rotation = Mat2::from_angle(0.37);
            let root = Sim2::new(scale, Mat2::from_cols(rotation.x_axis * parity,
                rotation.y_axis), Vec2::new(50.0, -25.0)).unwrap();
            let cast = MaterialCast {
                material: MaterialPrimitive { domain: &shape, to_world: root },
                root: root.point(Vec2::ZERO).unwrap(),
                delta: root.vector(Vec2::new(8.0, 0.0)).unwrap(), time: 0.0..1.0,
            }.cells(None).unwrap();
            let obstacle = box_at(Vec2::new(4.0, 0.0), Vec2::new(1.0, 2.0)).into_iter()
                .map(|p| root.point(p).unwrap()).collect::<Vec<_>>();
            let cell = &cast.parts()[0];
            let hit = cast.against(cell, &obstacle, Vec2::ZERO).unwrap().unwrap();
            let source = cell.at(hit.toi).unwrap().inverse().unwrap().point(hit.world_point).unwrap();
            assert!(source.distance(Vec2::X) < 0.0001, "{scale}, {parity}: {source}");
            assert!((hit.toi - 0.25).abs() < 0.0001);
            assert!(hit.map.point(hit.point).unwrap().distance(hit.world_point) < 0.0001);
        }
    }
}

#[test]
fn a_cast_keeps_the_motion_of_its_cell_boundaries() {
    let shape = CombatVolume::circle(Vec2::ZERO, 2.0);
    let mut cells = MaterialCast {
        material: MaterialPrimitive::world(&shape), root: Vec2::ZERO,
        delta: Vec2::ZERO, time: 0.0..1.0,
    }.cells(None).unwrap();
    cells.parts[0].planes.push(translation::MovingPlane {
        plane: MaterialPlane { inward: DVec2::X, offset: 3.0 }, rate: -6.0,
    });
    let cell = &cells.parts()[0];
    let obstacle = box_at(Vec2::ZERO, Vec2::new(1.0, 2.0));
    let hit = cells.against(cell, &obstacle, Vec2::ZERO).unwrap().unwrap();
    assert!((hit.toi - 1.0 / 3.0).abs() < 1.0e-12);
    assert!(cells.bounds(cell).unwrap().unwrap().max.x >= hit.world_point.x);
}

#[test]
fn a_moving_cell_keeps_its_host_boundary_without_a_false_strip() {
    for shape in [CombatVolume::aabb(Aabb::new(Vec2::new(-6.0, 0.0), Vec2::new(10.0, 4.0))),
        CombatVolume::circle(Vec2::new(-6.0, 0.0), 10.0)] {
        let mut cells = MaterialCast {
            material: MaterialPrimitive::world(&shape), root: Vec2::new(-6.0, 0.0),
            delta: Vec2::new(20.0, 0.0), time: 0.0..1.0,
        }.cells(None).unwrap();
        cells.parts[0].planes.push(translation::MovingPlane {
            plane: MaterialPlane { inward: -DVec2::X, offset: 0.0 }, rate: 0.0,
        });
        let host = box_at(Vec2::new(25.0, 0.0), Vec2::new(5.0, 20.0));
        assert!(cells.against(&cells.parts[0], &host, Vec2::new(20.0, 0.0))
            .unwrap().is_none());
    }
}

fn through() -> [PortalLink; 2] {
    PortalLink::both_ways(
        0,
        (
            PortalAperture {
                frame: PortalFrame::fixed(Vec2::ZERO, -Vec2::X),
                half_length: 40.0,
            },
            PortalAperture {
                frame: PortalFrame::fixed(Vec2::new(100.0, 0.0), Vec2::X),
                half_length: 20.0,
            },
        ),
    )
}

#[test]
fn a_moving_block_hits_a_stationary_body_between_its_endpoint_poses() {
    let shape=CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::ONE));
    let mut block=crate::Block::solid("moving wall",Vec2::new(9.0,-2.0),Vec2::new(2.0,4.0));
    block.velocity=Vec2::new(20.0,0.0);
    let mut world=crate::World::new("moving wall",Vec2::splat(100.0),Vec2::ZERO,Vec::new());
    world.blocks.push(block);
    let hit=world.first_material_sweep(MaterialCast { time:0.0..1.0,
        material:MaterialPrimitive {domain:&shape,to_world:Sim2::IDENTITY},
        root:Vec2::ZERO,delta:Vec2::ZERO,
    },None,|_,_,_| Ok(true)).unwrap().unwrap();
    assert!((hit.contact.toi-0.4).abs()<1.0e-12,"{hit:?}");
    assert_eq!(hit.contact.normal,Vec2::X);
    assert!((hit.contact.world_point.x+1.0).abs()<1.0e-6);
}

#[test]
fn a_translating_line_hits_material_between_its_endpoint_poses() {
    let body = CombatVolume::circle(Vec2::ZERO, 1.0);
    let mut line = crate::SurfaceChain::open("moving line", vec![Vec2::new(10.0, -2.0), Vec2::new(10.0, 2.0)]);
    line.velocity = Vec2::new(20.0, 0.0);
    let world = crate::World::new("line sweep", Vec2::splat(100.0), Vec2::ZERO, vec![]).with_chains(vec![line]);
    let hit = world.first_material_sweep(MaterialCast { material: MaterialPrimitive::world(&body),
        root: Vec2::ZERO, delta: Vec2::ZERO, time: 0.0..1.0 }, None, |_,_,_|Ok(true)).unwrap().unwrap();
    assert!((hit.contact.toi - 0.45).abs() < 1.0e-12, "{hit:?}");
    assert_eq!(hit.contact.normal, Vec2::X);
    assert!((hit.contact.world_point.x + 1.0).abs() < 0.00001);
}

#[test]
fn a_partial_step_uses_only_its_world_time_interval() {
    let shape=CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::ONE));
    let mut block=crate::Block::solid("moving wall",Vec2::new(9.0,-2.0),Vec2::new(2.0,4.0));
    block.velocity=Vec2::new(20.0,0.0);
    let world=crate::World::new("partial motion",Vec2::splat(100.0),Vec2::ZERO,vec![block]);
    let cast=|time|world.first_material_sweep(MaterialCast {
        time,material:MaterialPrimitive {domain:&shape,to_world:Sim2::IDENTITY},
        root:Vec2::ZERO,delta:Vec2::ZERO,
    },None,|_,_,_|Ok(true)).unwrap();
    assert!(cast(0.0..0.2).is_none());
    let hit=cast(0.3..0.6).unwrap();
    assert!((hit.contact.toi-1.0/3.0).abs()<1.0e-12,"{hit:?}");
    assert!((hit.contact.world_point.x+1.0).abs()<1.0e-6);
    assert!(cast(0.6..1.0).is_none());
}

#[test]
fn an_outgoing_boundary_touch_does_not_repeat_an_earlier_contact() {
    for shape in [CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::ONE)),
        CombatVolume::Circle {center:Vec2::ZERO,radius:1.0}] {
        let cells=MaterialCast {
            time:0.0..1.0,material:MaterialPrimitive {domain:&shape,to_world:Sim2::IDENTITY},
            root:Vec2::ZERO,delta:-Vec2::X,
        }.cells(None).unwrap();
        assert!(cells.against(&cells.parts()[0],&box_at(Vec2::new(2.0,0.0),Vec2::ONE),Vec2::ZERO).unwrap().is_none());
    }
}

#[test]
fn a_material_step_refuses_an_invalid_world_time_interval() {
    let shape=CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::ONE));
    for time in [-0.1..1.0,0.0..1.1,0.8..0.2,0.0..f64::NAN] {
        let result=MaterialCast {
            time,material:MaterialPrimitive {domain:&shape,to_world:Sim2::IDENTITY},
            root:Vec2::ZERO,delta:Vec2::ZERO,
        }.cells(None);
        assert!(matches!(result,Err(Refusal {reason:RefusalReason::Numeric(_),..})));
    }
}

#[test]
fn moving_obstacles_keep_the_exact_circle_corner() {
    for scale in [0.001,1.0,1000.0] {
        for reflected in [false,true] {
            let mut q=Mat2::from_angle(0.37);
            if reflected {q.x_axis=-q.x_axis;}
            let root=Sim2::new(scale,q,Vec2::new(100.0,-50.0)).unwrap();
            let shape=CombatVolume::Circle {center:Vec2::ZERO,radius:1.0};
            let cells=MaterialCast { time:0.0..1.0,
                material:MaterialPrimitive {domain:&shape,to_world:root},
                root:root.point(Vec2::ZERO).unwrap(),delta:Vec2::ZERO,
            }.cells(None).unwrap();
            for (height,expected) in [(1.6,None),(1.3,Some(2.9/8.0))] {
                let obstacle=box_at(Vec2::new(4.0,height),Vec2::splat(0.5)).into_iter()
                    .map(|p|root.point(p).unwrap()).collect::<Vec<_>>();
                let hit=cells.against(&cells.parts()[0],&obstacle,root.vector(Vec2::new(8.0,0.0)).unwrap()).unwrap();
                match expected {
                    None=>assert!(hit.is_none(),"a circle's bounding corner is empty: {hit:?}"),
                    Some(time)=>assert!((hit.unwrap().toi-time).abs()<0.003),
                }
            }
        }
    }
}

#[test]
fn a_moving_obstacle_can_cross_the_far_cell_during_the_body_step() {
    let links=through();
    let shape=CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::splat(10.0)));
    let root=Sim2::new(1.0,Mat2::IDENTITY,Vec2::new(-20.0,0.0)).unwrap();
    let mut block=crate::Block::solid("crossing wall",Vec2::new(84.0,-20.0),Vec2::new(2.0,40.0));
    block.velocity=Vec2::new(-30.0,0.0);
    let mut world=crate::World::new("crossing wall",Vec2::splat(300.0),Vec2::ZERO,Vec::new());
    world.blocks.push(block);
    let hit=world.first_material_sweep(MaterialCast { time:0.0..1.0,
        material:MaterialPrimitive {domain:&shape,to_world:root},
        root:Vec2::new(-20.0,0.0),delta:Vec2::new(30.0,0.0),
    },Some(MotionAdmission {links:&links,first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection}),
    |_,part,_|Ok(part.path==[0])).unwrap().unwrap();
    assert!((hit.contact.toi-19.0/45.0).abs()<1.0e-10,"{hit:?}");
    assert_eq!(hit.contact.path,vec![0]);
    assert!((hit.contact.world_point.x-(95.0+15.0*19.0/45.0)).abs()<1.0e-5);
    assert!(-20.0+30.0*hit.contact.toi<0.0);
}

#[test]
fn a_moving_solid_that_only_touches_the_cell_plane_is_not_a_wall() {
    let links=through();
    for shape in [CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::splat(10.0))),
        CombatVolume::Circle {center:Vec2::ZERO,radius:10.0}] {
        let cells=MaterialCast { time:0.0..1.0,
            material:MaterialPrimitive {domain:&shape,to_world:Sim2::new(1.0,Mat2::IDENTITY,Vec2::new(-6.0,0.0)).unwrap()},
            root:Vec2::new(-6.0,0.0),delta:Vec2::new(2.0,0.0),
        }.cells(Some(MotionAdmission {links:&links,first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection})).unwrap();
        let obstacle=box_at(Vec2::new(99.0,5.0),Vec2::new(1.0,50.0));
        let part=cells.parts().iter().find(|part|part.path==[0]).unwrap();
        assert!(cells.against(part,&obstacle,Vec2::new(0.0,10.0)).unwrap().is_none(),"{shape:?}");
    }
}

#[test]
fn a_circle_meets_a_moving_clipped_corner_at_its_round_edge() {
    let links=through();
    let shape=CombatVolume::Circle {center:Vec2::ZERO,radius:10.0};
    let cells=MaterialCast { time:0.0..1.0,
        material:MaterialPrimitive {domain:&shape,to_world:Sim2::new(1.0,Mat2::IDENTITY,Vec2::new(-6.0,0.0)).unwrap()},
        root:Vec2::new(-6.0,0.0),delta:Vec2::ZERO,
    }.cells(Some(MotionAdmission {links:&links,first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection})).unwrap();
    let part=cells.parts().iter().find(|part|part.path==[0]).unwrap();
    let obstacle=box_at(Vec2::new(105.0,-10.0),Vec2::new(20.0,1.0));
    let hit=cells.against(part,&obstacle,Vec2::new(0.0,-20.0)).unwrap().unwrap();
    assert!((hit.toi-0.25).abs()<1.0e-10,"{hit:?}");
    assert!(hit.world_point.distance(Vec2::new(100.0,4.0))<1.0e-5,"{hit:?}");
    assert!(hit.map.direction(hit.normal).distance(Vec2::new(-0.6,-0.8))<1.0e-5,"{hit:?}");
}

#[test]
fn a_solid_enters_an_occupied_cell_before_its_far_edge_meets_the_body() {
    let links=through();
    for shape in [CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::splat(10.0))),
        CombatVolume::Circle {center:Vec2::ZERO,radius:10.0}] {
        let cells=MaterialCast { time:0.0..1.0,
            material:MaterialPrimitive {domain:&shape,to_world:Sim2::new(1.0,Mat2::IDENTITY,Vec2::new(-6.0,0.0)).unwrap()},
            root:Vec2::new(-6.0,0.0),delta:Vec2::ZERO,
        }.cells(Some(MotionAdmission {links:&links,first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection})).unwrap();
        let part=cells.parts().iter().find(|part|part.path==[0]).unwrap();
        let obstacle=box_at(Vec2::new(110.0,0.0),Vec2::new(1.0,10.0));
        let hit=cells.against(part,&obstacle,Vec2::new(20.0,0.0)).unwrap().unwrap();
        assert!((hit.toi-0.45).abs()<1.0e-10,"{shape:?}: {hit:?}");
        assert!((hit.world_point.x-100.0).abs()<1.0e-5,"{hit:?}");
        assert_eq!(hit.normal,Vec2::X);
    }
}

#[test]
fn a_scaled_far_side_obstacle_stops_material_before_its_root_crosses() {
    let links = through();
    let shape = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::splat(10.0)));
    let root = Sim2::new(1.0, Mat2::IDENTITY, Vec2::new(-20.0, 0.0)).unwrap();
    let cast = MaterialCast { time:0.0..1.0,
        material: MaterialPrimitive {
            domain: &shape,
            to_world: root,
        },
        root: Vec2::new(-20.0, 0.0),
        delta: Vec2::new(30.0, 0.0),
    }
    .cells(Some(MotionAdmission {
        links: &links,
        first: None,
        budget: PieceBudget::COMPOSED,
        convention: MapConvention::Reflection,
    }))
    .unwrap();
    let obstacle = box_at(Vec2::new(103.0, 0.0), Vec2::new(1.0, 10.0));
    let hit = cast
        .parts()
        .iter()
        .filter_map(|part| cast.against(part, &obstacle, Vec2::ZERO).unwrap())
        .min_by(|a, b| a.toi.total_cmp(&b.toi))
        .unwrap();
    assert!((hit.toi - 14.0 / 30.0).abs() < 1.0e-6, "{hit:?}");
    assert_eq!(hit.path, vec![0]);
    assert!((hit.point.x - 4.0).abs() < 1.0e-5);
    assert!((hit.world_point.x - 102.0).abs() < 1.0e-5);
    assert_eq!(hit.normal, -Vec2::X);
    assert!((-20.0 + 30.0 * hit.toi) < 0.0);
}

#[test]
fn a_circle_keeps_its_round_corner_under_rotation_reflection_and_scale() {
    for scale in [0.001, 1.0, 1000.0] {
        for reflected in [false, true] {
            let mut q = Mat2::from_angle(0.73);
            if reflected {
                q.x_axis = -q.x_axis;
            }
            let root = Sim2::new(scale, q, Vec2::new(100.0, -50.0)).unwrap();
            let shape = CombatVolume::Circle {
                center: Vec2::ZERO,
                radius: 1.0,
            };
            let delta = root.vector(Vec2::new(4.0, 0.0)).unwrap();
            let cast = MaterialCast { time:0.0..1.0,
                material: MaterialPrimitive {
                    domain: &shape,
                    to_world: root,
                },
                root: root.point(Vec2::ZERO).unwrap(),
                delta,
            }
            .cells(None)
            .unwrap();
            let obstacle = box_at(Vec2::new(3.0, 1.6), Vec2::new(0.5, 0.5))
                .into_iter()
                .map(|p| root.point(p).unwrap())
                .collect::<Vec<_>>();
            assert!(
                cast.against(&cast.parts()[0], &obstacle, Vec2::ZERO).unwrap().is_none(),
                "the circle's AABB must not become its contact shape"
            );
            let obstacle = box_at(Vec2::new(3.0, 0.5), Vec2::new(0.5, 0.5))
                .into_iter()
                .map(|p| root.point(p).unwrap())
                .collect::<Vec<_>>();
            let hit = cast.against(&cast.parts()[0], &obstacle, Vec2::ZERO).unwrap().unwrap();
            assert!((hit.toi - 0.375).abs() < 0.003, "{hit:?}");
        }
    }
}

#[test]
fn an_oblique_box_does_not_collide_with_its_envelope_corner() {
    let shape = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::new(5.0, 0.5)));
    let root = Sim2::new(
        1.0,
        Mat2::from_angle(std::f32::consts::FRAC_PI_4),
        Vec2::ZERO,
    )
    .unwrap();
    let cast = MaterialCast { time:0.0..1.0,
        material: MaterialPrimitive {
            domain: &shape,
            to_world: root,
        },
        root: Vec2::ZERO,
        delta: Vec2::new(0.1, 0.0),
    }
    .cells(None)
    .unwrap();
    let obstacle = box_at(Vec2::new(-3.0, 3.0), Vec2::splat(0.2));
    assert!(cast.against(&cast.parts()[0], &obstacle, Vec2::ZERO).unwrap().is_none());
    let obstacle = box_at(Vec2::new(4.0, 0.0), Vec2::new(1.0, 10.0));
    let travel = Vec2::new(10.0, 0.0);
    let cast = MaterialCast { time:0.0..1.0,
        material: MaterialPrimitive {
            domain: &shape,
            to_world: root,
        },
        root: Vec2::ZERO,
        delta: travel,
    }
    .cells(None)
    .unwrap();
    let hit = cast.against(&cast.parts()[0], &obstacle, Vec2::ZERO).unwrap().unwrap();
    assert!(hit.toi < 0.1, "the control must meet the wall: {hit:?}");
}

#[test]
fn parallel_edge_touch_is_not_a_blocking_sweep() {
    let shape = CombatVolume::aabb(Aabb::new(Vec2::new(-3.0, -1.0), Vec2::ONE));
    let cast = MaterialCast { time:0.0..1.0,
        material: MaterialPrimitive::world(&shape),
        root: Vec2::new(-3.0, -1.0),
        delta: Vec2::new(6.0, 0.0),
    }
    .cells(None)
    .unwrap();
    let obstacle = box_at(Vec2::new(0.0, 1.0), Vec2::ONE);
    assert!(cast.against(&cast.parts()[0], &obstacle, Vec2::ZERO).unwrap().is_none());
}

#[test]
fn world_contacts_keep_the_source_identity_and_directed_path() {
    use crate::{Block, GeoId, PlacementId, World};
    let links = through();
    let shape = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::splat(10.0)));
    let make = || MaterialCast { time:0.0..1.0,
        material: MaterialPrimitive {
            domain: &shape,
            to_world: Sim2::new(1.0, Mat2::IDENTITY, Vec2::new(-20.0, 0.0)).unwrap(),
        },
        root: Vec2::new(-20.0, 0.0),
        delta: Vec2::new(30.0, 0.0),
    };
    let mut a = Block::solid("physical A", Vec2::new(102.0, -20.0), Vec2::new(2.0, 40.0));
    a.id = GeoId::placement(PlacementId::new("a"), 0);
    let mut b = a.clone();
    b.id = GeoId::placement(PlacementId::new("b"), 0);
    let admission = MotionAdmission {
        links: &links,
        first: None,
        budget: PieceBudget::COMPOSED,
        convention: MapConvention::Reflection,
    };
    for blocks in [vec![a.clone(), b.clone()], vec![b.clone(), a.clone()]] {
        let world = World::new("material contact", Vec2::splat(1000.0), Vec2::ZERO, blocks);
        let hit = world
            .first_material_sweep(make(), Some(admission), |_, _, _| Ok(true))
            .unwrap()
            .unwrap();
        assert_eq!(
            hit.collider.source(),
            crate::collision_semantics::ContactSource::Block {
                kind: a.kind,
                id: a.id.clone()
            }
        );
        assert_eq!(hit.contact.path, vec![0]);
        assert!((hit.contact.toi - 14.0 / 30.0).abs() < 1.0e-6);
    }
}

#[test]
fn a_chain_contact_keeps_its_logical_index_in_a_layered_world() {
    use crate::{SurfaceChain, World};
    let mut world =
        World::new("chain cells", Vec2::splat(1000.0), Vec2::ZERO, vec![]).with_chains(vec![
            SurfaceChain::open("discarded", vec![Vec2::splat(500.0), Vec2::splat(600.0)]),
            SurfaceChain::open("kept", vec![Vec2::new(5.0, -10.0), Vec2::new(5.0, 10.0)]),
        ]);
    world.chains.hide_where(|chain| chain.name == "discarded");
    let shape = CombatVolume::Circle {
        center: Vec2::ZERO,
        radius: 1.0,
    };
    let cast = MaterialCast { time:0.0..1.0,
        material: MaterialPrimitive::world(&shape),
        root: Vec2::ZERO,
        delta: Vec2::new(10.0, 0.0),
    };
    let hit = world
        .first_material_sweep(cast, None, |_, _, _| Ok(true))
        .unwrap()
        .unwrap();
    assert!((hit.contact.toi - 0.4).abs() < 1.0e-6);
    assert_eq!(
        hit.collider.source(),
        crate::collision_semantics::ContactSource::Chain {
            chain: 0,
            segment: 0
        }
    );
    assert_eq!(world.chains[0].name, "kept");
}

#[test]
fn a_stationary_overlap_is_reported_without_turning_a_touch_into_penetration() {
    for shape in [CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::ONE)),CombatVolume::Circle { center:Vec2::ZERO,radius:1.0 }] {
        let cast=MaterialCast { time:0.0..1.0, material:MaterialPrimitive::world(&shape),root:Vec2::ZERO,delta:Vec2::ZERO }.cells(None).unwrap();
        let inside=box_at(Vec2::new(0.0,1.0),Vec2::ONE);
        let hit=cast.against(&cast.parts()[0],&inside,Vec2::ZERO).unwrap().unwrap();
        assert_eq!(hit.toi,0.0);
        assert_eq!(hit.normal,-Vec2::Y);
        let touching=box_at(Vec2::new(0.0,2.0),Vec2::ONE);
        assert!(cast.against(&cast.parts()[0],&touching,Vec2::ZERO).unwrap().is_none());
    }
}

#[test]
fn surface_velocity_returns_through_a_scaled_moving_path() {
    let mut links=through();
    links[0].enter.frame.velocity=Vec2::new(7.0,3.0);
    links[0].exit.frame.velocity=Vec2::new(-2.0,5.0);
    links[0].enter.frame.spin=0.2;
    links[0].exit.frame.spin=-0.4;
    let admission=MotionAdmission { links:&links,first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection };
    let source_point=Vec2::new(4.0,2.0);
    let source_velocity=Vec2::new(80.0,17.0);
    let link=links[0]; let map=link.map(admission.convention).unwrap();
    let physical_point=map.point(source_point).unwrap();
    let physical_velocity=map.velocity(&link.enter.frame,&link.exit.frame,source_point,source_velocity).unwrap();
    let returned=admission.uncarry_velocity(&[link.key],physical_point,physical_velocity,1.0).unwrap();
    assert!(returned.distance(source_velocity)<0.001,"{returned:?}");
}

#[test]
fn portal_hosts_do_not_need_a_box_carve_and_a_near_exit_obstacle_remains_solid() {
    use crate::{Block, World};
    let links=through();
    let source=CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::splat(10.0)));
    let cast=|| MaterialCast { time:0.0..1.0,
        material:MaterialPrimitive { domain:&source,to_world:Sim2::new(1.0,Mat2::IDENTITY,Vec2::new(-20.0,0.0)).unwrap() },
        root:Vec2::new(-20.0,0.0),delta:Vec2::new(20.0,0.0),
    };
    let admission=MotionAdmission { links:&links,first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection };
    let mut world=World::new("uncarved hosts",Vec2::splat(1000.0),Vec2::ZERO,vec![
        Block::solid("entry",Vec2::new(0.0,-40.0),Vec2::new(10.0,80.0)),
        Block::solid("exit",Vec2::new(90.0,-40.0),Vec2::new(10.0,80.0)),
    ]);
    assert!(world.first_material_sweep(cast(),Some(admission),|_,_,_|Ok(true)).unwrap().is_none());
    world.blocks.push(Block::solid("near exit",Vec2::new(102.0,-40.0),Vec2::new(2.0,80.0)));
    let hit=world.first_material_sweep(cast(),Some(admission),|_,_,_|Ok(true)).unwrap().unwrap();
    assert_eq!(hit.contact.path,vec![0]);
    assert!((hit.contact.toi-0.7).abs()<0.001,"{hit:?}");
    assert_eq!(hit.contact.world_point.x,102.0);
}

#[test]
fn a_line_host_keeps_its_finite_portal_ends_solid_for_boxes_and_circles() {
    use crate::{SurfaceChain,World};
    let links=through();
    let world=World::new("line portal",Vec2::splat(1000.0),Vec2::ZERO,vec![])
        .with_chains(vec![SurfaceChain::open("host",vec![Vec2::new(0.0,100.0),Vec2::new(0.0,-100.0)])]);
    let admission=MotionAdmission { links:&links,first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection };
    for (shape,expected) in [
        (CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::splat(10.0))),0.5),
        (CombatVolume::Circle { center:Vec2::ZERO,radius:10.0 },(20.0-75.0_f64.sqrt())/20.0),
    ] {
        let cast=|y| MaterialCast { time:0.0..1.0,
            material:MaterialPrimitive { domain:&shape,to_world:Sim2::new(1.0,Mat2::IDENTITY,Vec2::new(-20.0,y)).unwrap() },
            root:Vec2::new(-20.0,y),delta:Vec2::new(20.0,0.0),
        };
        assert!(world.first_material_sweep(cast(0.0),Some(admission),|_,_,_|Ok(true)).unwrap().is_none());
        let hit=world.first_material_sweep(cast(35.0),Some(admission),|_,_,_|Ok(true)).unwrap().unwrap();
        assert!(hit.contact.path.is_empty());
        assert!((hit.contact.toi-expected).abs()<0.001,"{hit:?}");
    }
}
