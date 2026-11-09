use super::*;
use crate::frame::thread::{PieceBudget, PortalLink};
use crate::frame::{MapConvention, PortalAperture, PortalFrame};
use bevy_math::Mat2;

fn box_at(center: Vec2, half: Vec2) -> Vec<Vec2> {
    corners(Aabb::new(center, half)).to_vec()
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
fn a_scaled_far_side_obstacle_stops_material_before_its_root_crosses() {
    let links = through();
    let shape = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::splat(10.0)));
    let root = Sim2::new(1.0, Mat2::IDENTITY, Vec2::new(-20.0, 0.0)).unwrap();
    let cast = MaterialCast {
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
        .filter_map(|part| cast.against(part, &obstacle).unwrap())
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
            let cast = MaterialCast {
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
                cast.against(&cast.parts()[0], &obstacle).unwrap().is_none(),
                "the circle's AABB must not become its contact shape"
            );
            let obstacle = box_at(Vec2::new(3.0, 0.5), Vec2::new(0.5, 0.5))
                .into_iter()
                .map(|p| root.point(p).unwrap())
                .collect::<Vec<_>>();
            let hit = cast.against(&cast.parts()[0], &obstacle).unwrap().unwrap();
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
    let cast = MaterialCast {
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
    assert!(cast.against(&cast.parts()[0], &obstacle).unwrap().is_none());
    let obstacle = box_at(Vec2::new(4.0, 0.0), Vec2::new(1.0, 10.0));
    let travel = Vec2::new(10.0, 0.0);
    let cast = MaterialCast {
        material: MaterialPrimitive {
            domain: &shape,
            to_world: root,
        },
        root: Vec2::ZERO,
        delta: travel,
    }
    .cells(None)
    .unwrap();
    let hit = cast.against(&cast.parts()[0], &obstacle).unwrap().unwrap();
    assert!(hit.toi < 0.1, "the control must meet the wall: {hit:?}");
}

#[test]
fn parallel_edge_touch_is_not_a_blocking_sweep() {
    let shape = CombatVolume::aabb(Aabb::new(Vec2::new(-3.0, -1.0), Vec2::ONE));
    let cast = MaterialCast {
        material: MaterialPrimitive::world(&shape),
        root: Vec2::new(-3.0, -1.0),
        delta: Vec2::new(6.0, 0.0),
    }
    .cells(None)
    .unwrap();
    let obstacle = box_at(Vec2::new(0.0, 1.0), Vec2::ONE);
    assert!(cast.against(&cast.parts()[0], &obstacle).unwrap().is_none());
}

#[test]
fn world_contacts_keep_the_source_identity_and_directed_path() {
    use crate::{Block, GeoId, PlacementId, World};
    let links = through();
    let shape = CombatVolume::aabb(Aabb::new(Vec2::ZERO, Vec2::splat(10.0)));
    let make = || MaterialCast {
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
    let cast = MaterialCast {
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
        let cast=MaterialCast { material:MaterialPrimitive::world(&shape),root:Vec2::ZERO,delta:Vec2::ZERO }.cells(None).unwrap();
        let inside=box_at(Vec2::new(0.0,1.0),Vec2::ONE);
        let hit=cast.against(&cast.parts()[0],&inside).unwrap().unwrap();
        assert_eq!(hit.toi,0.0);
        assert_eq!(hit.normal,-Vec2::Y);
        let touching=box_at(Vec2::new(0.0,2.0),Vec2::ONE);
        assert!(cast.against(&cast.parts()[0],&touching).unwrap().is_none());
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
    let returned=admission.uncarry_velocity(&[link.key],physical_point,physical_velocity).unwrap();
    assert!(returned.distance(source_velocity)<0.001,"{returned:?}");
}

#[test]
fn portal_hosts_do_not_need_a_box_carve_and_a_near_exit_obstacle_remains_solid() {
    use crate::{Block, World};
    let links=through();
    let source=CombatVolume::aabb(Aabb::new(Vec2::ZERO,Vec2::splat(10.0)));
    let cast=|| MaterialCast {
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
        let cast=|y| MaterialCast {
            material:MaterialPrimitive { domain:&shape,to_world:Sim2::new(1.0,Mat2::IDENTITY,Vec2::new(-20.0,y)).unwrap() },
            root:Vec2::new(-20.0,y),delta:Vec2::new(20.0,0.0),
        };
        assert!(world.first_material_sweep(cast(0.0),Some(admission),|_,_,_|Ok(true)).unwrap().is_none());
        let hit=world.first_material_sweep(cast(35.0),Some(admission),|_,_,_|Ok(true)).unwrap().unwrap();
        assert!(hit.contact.path.is_empty());
        assert!((hit.contact.toi-expected).abs()<0.001,"{hit:?}");
    }
}
