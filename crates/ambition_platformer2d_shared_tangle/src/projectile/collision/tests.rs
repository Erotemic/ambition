//! The fixtures cast one step to their stated destination pose.

use super::*;

#[test]
fn a_rotating_one_way_line_keeps_the_front_contact_after_the_back_contact() {
    for turn in [std::f32::consts::TAU,1.0e20_f32] {
    let direction=(bevy::math::DMat2::from_angle(f64::from(turn))*Vec2::X.as_dvec2()).as_vec2();
    let mut line=ae::SurfaceChain::open("one-way turn",vec![-direction*10.0,direction*10.0]);
    line.turn=turn;
    line.pivot=Vec2::ZERO;
    let world=ae::World::new("two approaches",Vec2::splat(100.0),Vec2::ZERO,vec![]).with_chains(vec![line]);
    let mut shot=straight_projectile(Vec2::new(0.0,8.0));
    shot.kin.canonical_size=Vec2::splat(2.0);
    shot.kin.vel=Vec2::ZERO;
    let contact=query_world_collision(shot.kin,shot.kin,shot.game,&world,Vec2::Y,1.0,0.0..1.0,ambition_platformer2d_core::LinearTravel::Complete,None).unwrap();
    let response=contact.response().unwrap();
    let hit=contact.contact.unwrap();
    let advance=hit.contact.toi*f64::from(turn);
    assert!(advance>4.0 && advance<5.0,"{hit:?}, advance={advance}");
    assert!(matches!(response.outcome,WorldHitOutcome::Expired {..}));
    }
}

fn resolve_motion(
    kin: &mut BodyKinematics, game: &mut ProjectileGameplay, world: &ae::World,
    policy: WorldHitPolicy, gravity: ae::Vec2,
) -> WorldHitOutcome {
    let mut before=*kin;
    before.pos-=kin.vel*0.1;
    game.world_hit=policy;
    let contact=super::query_world_collision(before,*kin,*game,world,gravity,0.1,0.0..1.0,ambition_platformer2d_core::LinearTravel::Complete,None).unwrap();
    let response=contact.response().unwrap();
    *kin=response.body;
    game.bounces_remaining=response.bounces_remaining;
    response.outcome
}

fn world_with_block(kind: ae::BlockKind, center: ae::Vec2, half: ae::Vec2) -> ae::World {
    let mut w = ae::World::new("test", ae::Vec2::new(800.0, 600.0), ae::Vec2::ZERO, vec![]);
    w.blocks.push(ae::Block {
        id: ae::GeoId::anon(),
        name: "wall".into(),
        aabb: ae::Aabb::new(center, half),
        kind,
        velocity: ae::Vec2::ZERO,
        art_color: None,
    });
    w
}

fn straight_projectile(pos: ae::Vec2) -> crate::projectile::ProjectileBody {
    let spec = crate::projectile::ProjectileSpec {
        origin: pos,
        direction: ae::Vec2::new(1.0, 0.0),
        damage: 1,
        speed: 200.0,
        max_lifetime: 1.0,
        half_extent: ae::Vec2::new(6.0, 6.0),
        gravity: 0.0,
        bounces: 0,
        world_hit: crate::projectile::WorldHitPolicy::ExpireOnContact,
        charge_tier: 0,
        boomerang_return_s: None,
        splash_half_extent: 0.0,
    };
    let mut body = crate::projectile::ProjectileBody::from_spec(spec);
    body.game.bounces_remaining = 0; // baseline: no bouncing
    body
}

#[test]
fn expire_policy_expires_on_solid_contact() {
    let world = world_with_block(
        ae::BlockKind::Solid,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 50.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 100.0));
    let outcome = resolve_motion(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::ExpireOnContact,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(matches!(outcome, WorldHitOutcome::Expired { .. }));
}

#[test]
fn expire_policy_expires_on_one_way_contact() {
    // This policy expires on every face of a one-way surface.
    let world = world_with_block(
        ae::BlockKind::OneWay,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 4.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 100.0));
    let outcome = resolve_motion(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::ExpireOnContact,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(matches!(outcome, WorldHitOutcome::Expired { .. }));
}

#[test]
fn bouncing_policy_expires_on_solid_when_out_of_bounces() {
    let world = world_with_block(
        ae::BlockKind::Solid,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 50.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 100.0));
    // bounces_remaining = 0 (straight shot)
    let outcome = resolve_motion(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(matches!(outcome, WorldHitOutcome::Expired { .. }));
}

#[test]
fn bouncing_policy_passes_through_one_way_at_zero_bounces() {
    // A shot with no bounces passes through a one-way surface.
    let world = world_with_block(
        ae::BlockKind::OneWay,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 4.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 100.0));
    let outcome = resolve_motion(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(matches!(outcome, WorldHitOutcome::Continue));
}

/// A falling shot uses the same support response on solid and one-way surfaces.
fn falling_shot(pos: ae::Vec2, bounces: u8) -> crate::projectile::ProjectileBody {
    let mut body = straight_projectile(pos);
    body.kin.pos = pos;
    body.kin.vel = ae::Vec2::new(0.0, 80.0); // toward world +Y, normal down-gravity case
    body.game.bounces_remaining = bounces;
    body
}

fn cardinal_gravity_dirs() -> [ae::Vec2; 4] {
    [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ]
}

fn local_to_world(frame: ae::AccelerationFrame, local: ae::Vec2) -> ae::Vec2 {
    frame.to_world(local)
}

fn world_to_local(frame: ae::AccelerationFrame, world: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new(world.dot(frame.side), world.dot(frame.down))
}

fn frame_world_with_block(
    gravity_dir: ae::Vec2,
    kind: ae::BlockKind,
    local_center: ae::Vec2,
    local_half: ae::Vec2,
) -> (ae::World, ae::AccelerationFrame) {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let origin = ae::Vec2::new(300.0, 300.0);
    let world_center = origin + local_to_world(frame, local_center);
    let world_half = frame.to_world_half(local_half);
    (world_with_block(kind, world_center, world_half), frame)
}

#[test]
fn bouncing_shot_bounce_is_frame_equivalent_on_solid_supports() {
    for gravity_dir in cardinal_gravity_dirs() {
        let (world, frame) = frame_world_with_block(
            gravity_dir,
            ae::BlockKind::Solid,
            ae::Vec2::new(0.0, 100.0),
            ae::Vec2::new(50.0, 50.0),
        );
        let origin = ae::Vec2::new(300.0, 300.0);
        let mut body =
            straight_projectile(origin + local_to_world(frame, ae::Vec2::new(0.0, 48.0)));
        body.kin.vel = local_to_world(frame, ae::Vec2::new(-15.0, 80.0));
        body.game.bounces_remaining = 2;

        let outcome = resolve_motion(
            &mut body.kin,
            &mut body.game,
            &world,
            WorldHitPolicy::Bouncing,
            gravity_dir,
            );
        assert!(
            matches!(outcome, WorldHitOutcome::Bounced { .. }),
            "solid support should bounce in frame {gravity_dir:?}: {outcome:?}"
        );
        let local_vel = world_to_local(frame, body.kin.vel);
        assert!((local_vel.x + 15.0).abs() < 1e-3);
        assert!(local_vel.y < 0.0);
    }
}

#[test]
fn bouncing_shot_bounce_is_frame_equivalent_on_one_way_supports() {
    for gravity_dir in cardinal_gravity_dirs() {
        let (world, frame) = frame_world_with_block(
            gravity_dir,
            ae::BlockKind::OneWay,
            ae::Vec2::new(0.0, 100.0),
            ae::Vec2::new(50.0, 4.0),
        );
        let origin = ae::Vec2::new(300.0, 300.0);
        let mut body =
            straight_projectile(origin + local_to_world(frame, ae::Vec2::new(0.0, 94.0)));
        body.kin.vel = local_to_world(frame, ae::Vec2::new(20.0, 80.0));
        body.game.bounces_remaining = 2;

        let outcome = resolve_motion(
            &mut body.kin,
            &mut body.game,
            &world,
            WorldHitPolicy::Bouncing,
            gravity_dir,
            );
        assert!(
            matches!(outcome, WorldHitOutcome::Bounced { .. }),
            "one-way support should bounce in frame {gravity_dir:?}: {outcome:?}"
        );
        let local_vel = world_to_local(frame, body.kin.vel);
        assert!(
            (local_vel.x - 20.0).abs() < 1e-3,
            "side velocity preserved: {local_vel:?}"
        );
        assert!(
            local_vel.y < 0.0,
            "local-down component should reflect away from support: {local_vel:?}"
        );
    }
}

#[test]
fn bouncing_shot_one_way_passthrough_is_frame_equivalent_from_feet_side() {
    for gravity_dir in cardinal_gravity_dirs() {
        let (world, frame) = frame_world_with_block(
            gravity_dir,
            ae::BlockKind::OneWay,
            ae::Vec2::new(0.0, 100.0),
            ae::Vec2::new(50.0, 4.0),
        );
        let origin = ae::Vec2::new(300.0, 300.0);
        let mut body =
            straight_projectile(origin + local_to_world(frame, ae::Vec2::new(0.0, 106.0)));
        body.kin.vel = local_to_world(frame, ae::Vec2::new(0.0, -80.0));
        body.game.bounces_remaining = 2;

        let outcome = resolve_motion(
            &mut body.kin,
            &mut body.game,
            &world,
            WorldHitPolicy::Bouncing,
            gravity_dir,
            );
        assert!(
            matches!(outcome, WorldHitOutcome::Continue),
            "one-way should pass through from feet side in frame {gravity_dir:?}: {outcome:?}"
        );
        assert_eq!(body.game.bounces_remaining, 2);
    }
}

#[test]
fn bouncing_shot_bounces_off_one_way_top_landing_with_budget() {
    // Thin one-way platform, top at y=96.
    let world = world_with_block(
        ae::BlockKind::OneWay,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 4.0),
    );
    // Falling onto the platform top, overlapping it from above.
    let mut body = falling_shot(ae::Vec2::new(100.0, 94.0), 2);
    let outcome = resolve_motion(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(
        matches!(outcome, WorldHitOutcome::Bounced { .. }),
        "bouncing shot should skip off a one-way top like a floor; got {outcome:?}"
    );
    assert_eq!(
        body.game.bounces_remaining, 1,
        "a bounce should be consumed"
    );
    assert!(
        body.kin.vel.y < 0.0,
        "bounce should reflect velocity upward"
    );
}

#[test]
fn bouncing_shot_passes_through_one_way_from_below() {
    // Same platform, but the projectile rises into it from underneath
    // with full bounce budget — a one-way is non-solid from below, so
    // it must pass through rather than bounce.
    let world = world_with_block(
        ae::BlockKind::OneWay,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 4.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 106.0));
    body.kin.vel = ae::Vec2::new(0.0, -80.0); // upward
    body.game.bounces_remaining = 2;
    let outcome = resolve_motion(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(
        matches!(outcome, WorldHitOutcome::Continue),
        "a one-way is non-solid from below; got {outcome:?}"
    );
    assert_eq!(
        body.game.bounces_remaining, 2,
        "passthrough must not spend a bounce"
    );
}

#[test]
fn bouncing_shot_bounces_off_solid_top_landing_with_budget() {
    // Skip-across-floor on a thick solid (parity with the one-way top
    // landing), to pin that both surfaces bounce identically.
    let world = world_with_block(
        ae::BlockKind::Solid,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(50.0, 50.0),
    );
    let mut body = falling_shot(ae::Vec2::new(100.0, 48.0), 2);
    let outcome = resolve_motion(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(
        matches!(outcome, WorldHitOutcome::Bounced { .. }),
        "bouncing shot with budget should skip off a solid floor; got {outcome:?}"
    );
    assert_eq!(body.game.bounces_remaining, 1);
}

#[test]
fn no_contact_returns_continue() {
    let world = world_with_block(
        ae::BlockKind::Solid,
        ae::Vec2::new(500.0, 500.0),
        ae::Vec2::new(10.0, 10.0),
    );
    let mut body = straight_projectile(ae::Vec2::new(100.0, 100.0));
    let outcome = resolve_motion(
        &mut body.kin,
        &mut body.game,
        &world,
        WorldHitPolicy::Bouncing,
        ae::Vec2::new(0.0, 1.0),
    );
    assert!(matches!(outcome, WorldHitOutcome::Continue));
}

#[test]
fn an_overlapping_far_part_hits_before_the_shot_can_withdraw() {
    use ae::frame::{MapConvention, PortalAperture, PortalFrame};
    use ae::frame::thread::{PieceBudget, PortalLink};
    let links = PortalLink::both_ways(0, (
        PortalAperture { frame: PortalFrame::fixed(Vec2::ZERO, -Vec2::X), half_length: 40.0 },
        PortalAperture { frame: PortalFrame::fixed(Vec2::new(100.0, 0.0), Vec2::X), half_length: 20.0 },
    ));
    let mut shot = straight_projectile(Vec2::new(-2.0, 0.0));
    shot.kin.canonical_size = Vec2::splat(20.0);
    shot.kin.vel = -Vec2::X * 200.0;
    let before = shot.kin;
    let mut after = before;
    after.pos.x -= 20.0;
    let world = world_with_block(ae::BlockKind::Solid, Vec2::new(102.0, 0.0), Vec2::new(1.0, 20.0));
    let admission = MotionAdmission { links: &links, first: None, budget: PieceBudget::COMPOSED, convention: MapConvention::Reflection };
    let contact=super::query_world_collision(before, after, shot.game, &world, Vec2::Y, 0.1,
        0.0..1.0, ae::LinearTravel::Complete, Some(admission)).unwrap();
    let response=contact.response().unwrap();
    assert!(matches!(response.outcome, WorldHitOutcome::Expired { .. }));
    assert_eq!(response.body.pos, before.pos);
    let contact = contact.contact.unwrap();
    assert_eq!(contact.contact.toi, 0.0);
    assert_eq!(contact.contact.path, vec![0]);
}

#[test]
fn a_projectile_meets_the_far_wall_before_its_root_crosses() {
    use ae::frame::{MapConvention,PortalAperture,PortalFrame};
    use ae::frame::thread::{PieceBudget,PortalLink};
    let links=PortalLink::both_ways(0,(
        PortalAperture { frame:PortalFrame::fixed(Vec2::ZERO,-Vec2::X),half_length:40.0 },
        PortalAperture { frame:PortalFrame::fixed(Vec2::new(100.0,0.0),Vec2::X),half_length:20.0 },
    ));
    let mut shot=straight_projectile(Vec2::new(-20.0,0.0));
    shot.kin.canonical_size=Vec2::splat(20.0);
    let before=shot.kin;
    let mut after=before; after.pos=Vec2::ZERO;
    let world=world_with_block(ae::BlockKind::Solid,Vec2::new(103.0,0.0),Vec2::new(1.0,20.0));
    let admission=MotionAdmission { links:&links,first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection };
    let contact=super::query_world_collision(before,after,shot.game,&world,Vec2::Y,0.1,0.0..1.0,ambition_platformer2d_core::LinearTravel::Complete,Some(admission)).unwrap();
    let response=contact.response().unwrap();
    assert!(matches!(response.outcome,WorldHitOutcome::Expired { .. }));
    assert!((response.body.pos.x+6.0).abs()<0.001,"{response:?}");
    let contact=contact.contact.unwrap();
    assert_eq!(contact.contact.path,vec![0]);
    assert_eq!(contact.contact.world_point.x,102.0);
    assert!(response.body.pos.x<0.0);
}

#[test]
fn a_far_side_bounce_uses_relative_surface_motion_in_the_root_chart() {
    use ae::frame::{MapConvention,PortalAperture,PortalFrame};
    use ae::frame::thread::{PieceBudget,PortalLink};
    let mut enter=PortalAperture { frame:PortalFrame::fixed(Vec2::ZERO,-Vec2::Y),half_length:40.0 };
    let mut exit=PortalAperture { frame:PortalFrame::fixed(Vec2::new(100.0,0.0),Vec2::X),half_length:20.0 };
    enter.frame.velocity=Vec2::new(0.0,5.0); exit.frame.velocity=Vec2::new(3.0,0.0);
    let links=PortalLink::both_ways(0,(enter,exit));
    let mut shot=falling_shot(Vec2::new(0.0,-12.0),2);
    shot.kin.canonical_size=Vec2::splat(10.0);
    shot.kin.vel=Vec2::new(0.0,110.0);
    shot.game.rebound=crate::projectile::Rebound { restitution:0.5,min_speed:0.0 };
    shot.game.world_hit=WorldHitPolicy::Bouncing;
    let before=shot.kin; let mut after=before; after.pos.y=-1.0;
    let mut world=world_with_block(ae::BlockKind::Solid,Vec2::new(102.0,0.0),Vec2::new(1.0,40.0));
    world.blocks[0].velocity=Vec2::X;
    let admission=MotionAdmission { links:&links,first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection };
    let contact=super::query_world_collision(before,after,shot.game,&world,Vec2::Y,0.1,0.0..1.0,ambition_platformer2d_core::LinearTravel::Complete,Some(admission)).unwrap();
    let response=contact.response().unwrap();
    assert!(matches!(response.outcome,WorldHitOutcome::Bounced { .. }),"{response:?}");
    assert_eq!(response.bounces_remaining,1);
    assert!((response.body.vel.y+26.5).abs()<0.001,"{response:?}");
    assert!(response.body.pos.y<0.0);
    assert_eq!(contact.contact.unwrap().contact.path,vec![0]);
}

#[test]
fn a_rebound_must_fit_every_occupied_material_chart() {
    use ae::frame::{MapConvention,PortalAperture,PortalFrame};
    use ae::frame::thread::{PieceBudget,PortalLink};
    for exit_half in [20.0,80.0] {
        let links=PortalLink::both_ways(0,(
            PortalAperture {frame:PortalFrame::fixed(Vec2::ZERO,-Vec2::Y),half_length:40.0},
            PortalAperture {frame:PortalFrame::fixed(Vec2::new(100.0,0.0),Vec2::X),half_length:exit_half},
        ));
        let mut shot=falling_shot(Vec2::new(0.0,-12.0),2);
        shot.kin.canonical_size=Vec2::splat(10.0);
        shot.kin.vel=Vec2::new(0.0,110.0);
        shot.game.rebound=crate::projectile::Rebound {restitution:0.5,min_speed:1.8e38};
        shot.game.world_hit=WorldHitPolicy::Bouncing;
        let before=shot.kin;
        let after=BodyKinematics {pos:Vec2::new(0.0,-1.0),..before};
        let world=world_with_block(ae::BlockKind::Solid,Vec2::new(102.0,0.0),Vec2::new(1.0,40.0));
        let admission=MotionAdmission {links:&links,first:None,budget:PieceBudget::COMPOSED,convention:MapConvention::Reflection};
        let contact=super::query_world_collision(before,after,shot.game,&world,Vec2::Y,0.1,
            0.0..1.0,ae::LinearTravel::Complete,Some(admission)).unwrap();
        assert_eq!(contact.contact.as_ref().unwrap().contact.path,vec![0]);
        if exit_half==80.0 {
            assert_eq!(contact.response().unwrap_err().reason,RefusalReason::Numeric(NumericFault::Overflow));
        } else {
            let response=contact.response().unwrap();
            assert!(matches!(response.outcome,WorldHitOutcome::Bounced {..}));
            assert_eq!(response.body.vel,Vec2::new(0.0,-1.8e38));
        }
    }
}

#[test]
fn a_shot_born_inside_a_wall_hits_before_it_can_escape() {
    let world = world_with_block(ae::BlockKind::Solid, Vec2::new(-16.0, 100.0), Vec2::new(16.0, 100.0));
    for shape in [Vec2::splat(2.0), Vec2::new(12.0, 6.0)] {
        let mut shot = straight_projectile(Vec2::new(-27.0, 100.0));
        shot.kin.canonical_size = shape;
        shot.kin.vel = Vec2::new(-300.0, 0.0);
        let before = shot.kin;
        let after = BodyKinematics { pos: before.pos + before.vel * 0.1, ..before };
        let contact=query_world_collision(before, after, shot.game, &world,
            Vec2::Y, 0.1, 0.0..1.0, ae::LinearTravel::Complete, None).unwrap();
    let response=contact.response().unwrap();
        assert!(matches!(response.outcome, WorldHitOutcome::Expired { .. }));
        assert_eq!(contact.contact.unwrap().contact.toi, 0.0);
        assert_eq!(response.body.pos, before.pos);
    }
}
