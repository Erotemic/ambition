//! The reach is the move: a tether that finds any ledge anywhere is not a
//! tether. So bite tests come in pairs: the same stage and fighter, one reach
//! that arrives and one that does not.
//!
//! The fixture integrates position itself. These tests run the two tether
//! systems without the movement kernel; `integrate` does only `pos += vel * dt`,
//! kept minimal so the file does not test a second movement implementation.
//!
//! These tests do not prove she ends in a ledge hang; that is the ledge
//! authority's job inside the kernel. They prove what this module owns: the
//! line bites only within reach, the reel lands on the anchor, and it hands
//! her over in a state the authority can catch.

use super::*;
use ambition_platformer2d::engine_core::world::Block;

/// The reference geometry from `ledge_grab`'s own probe tests: a block spanning
/// x 100..300, y 100..300, whose left lip a body centred near x = 86 catches.
fn stage() -> Vec<Block> {
    vec![Block::solid(
        "ledge",
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(200.0, 200.0),
    )]
}

const PLAYER_SIZE: ae::Vec2 = ae::Vec2::new(28.0, 46.0);
/// 60px short of the lip, and 4 sample steps out at a 150px reach.
const START: ae::Vec2 = ae::Vec2::new(26.0, 110.0);

fn integrate(
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut bodies: Query<&mut ae::BodyKinematics>,
) {
    let dt = time.sim_dt();
    for mut kin in &mut bodies {
        let velocity = kin.vel;
        kin.pos += velocity * dt;
    }
}

fn app(blocks: Vec<Block>) -> App {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    app.add_message::<ActorActionMessage>();
    {
        let mut time = app
            .world_mut()
            .resource_mut::<ambition_platformer2d::time::WorldTime>();
        time.scaled_dt = 1.0 / 60.0;
        time.raw_dt = 1.0 / 60.0;
    }
    // The room reaches `CollisionWorld` as a component on the session-world
    // root (`SessionWorldRef` is a `Single<Ref<T>, With<SessionRoot>>`), as in
    // production.
    ambition_platformer2d::session::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "tether",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::ZERO,
            blocks,
        )),
    );
    app.add_systems(
        Update,
        (
            begin_authored_tether_pulls,
            reel_tethered_fighters,
            integrate,
        )
            .chain(),
    );
    app
}

fn fighter(app: &mut App, at: ae::Vec2, on_ground: bool) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: at,
                size: PLAYER_SIZE,
                facing: 1.0,
                ..Default::default()
            },
            ae::BodyGroundState {
                on_ground,
                ..Default::default()
            },
            ambition_platformer2d::world::ResolvedMotionFrame::default(),
        ))
        .id()
}

fn throw(app: &mut App, actor: Entity, reach: f32) {
    let params = TetherPullParams {
        reach,
        speed: 900.0,
        timeout_s: 0.5,
    };
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(TETHER_PULL.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params)
            .expect("tether params serialize"),
    };
    app.world_mut()
        .write_message(ActorActionMessage { actor, request, move_instance: None });
    app.update();
}

fn reel(app: &App, who: Entity) -> Option<TetherReel> {
    app.world().get::<TetherReel>(who).cloned()
}

fn body(app: &App, who: Entity) -> ae::BodyKinematics {
    app.world().get::<ae::BodyKinematics>(who).unwrap().clone()
}

/// A line long enough to reach the lip bites it and starts reeling.
#[test]
fn a_line_within_reach_bites_the_ledge() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    throw(&mut app, her, 150.0);
    let reel = reel(&app, her).expect("a ledge 60px away is inside a 150px line");
    assert!(
        (reel.anchor.x - 87.0).abs() < 6.0,
        "latched {:?}, expected the block's left lip near x=87",
        reel.anchor,
    );
}

/// The line goes where she faces, in her frame's side axis, not world +x.
///
/// `probe_ledge_grab_in_frame`'s `wall_normal_x` is in the body's local side
/// axis, so the walk must be too. Under normal gravity the two agree.
///
/// The stage is the one above turned 90°: `(x,y) -> (y,-x)`, slid back into
/// the room, maps the block onto itself. The expected anchor is the rotation
/// of that test's, `(87,119) -> (119,313)`.
///
/// The decoy makes a failure specific: it sits where a world-x walk would look
/// (lip 90..135px along world +x, side face 10px from her reaching side), so
/// the wrong walk latches it and reels her the wrong way. The two blocks are
/// invisible to each other's walk.
#[test]
fn the_line_sweeps_her_own_side_axis_when_the_frame_is_turned() {
    let mut app = app(vec![
        stage().remove(0),
        // Where a world-x walk would search, and nowhere the correct walk looks.
        Block::solid(
            "decoy",
            ae::Vec2::new(200.0, 320.0),
            ae::Vec2::new(100.0, 40.0),
        ),
    ]);
    // START, rotated: (26,110) -> (110,-26) -> +400y.
    let her = fighter(&mut app, ae::Vec2::new(110.0, 374.0), false);
    {
        let rotated = ae::MotionFrame::from_direction(ae::Vec2::new(1.0, 0.0), 0.0);
        let mut her_mut = app.world_mut().entity_mut(her);
        let mut frame = her_mut
            .get_mut::<ambition_platformer2d::world::ResolvedMotionFrame>()
            .expect("the fixture gives every fighter a frame");
        frame.publish_resolved_frame(rotated);
    }
    throw(&mut app, her, 150.0);

    let reel = reel(&app, her).expect("the same 60px-away ledge, asked in her own frame");
    assert!(
        (reel.anchor - ae::Vec2::new(119.0, 313.0)).length() < 6.0,
        "latched {:?}; the reference block's lip is at (119,313) in this frame and \
         the decoy the world-x walk finds is at (219,373)",
        reel.anchor,
    );
}

/// The paired miss: the same stage and fighter with a line too short.
#[test]
fn a_line_too_short_bites_nothing() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    throw(&mut app, her, 30.0);
    assert!(
        reel(&app, her).is_none(),
        "a 30px line reached a ledge 60px away",
    );
}

/// A tether is an aerial move. On the ground the same fiction is her grab,
/// and a grounded line would be a free horizontal dash.
#[test]
fn a_tether_thrown_from_the_ground_does_not_fire() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, true);
    throw(&mut app, her, 150.0);
    assert!(reel(&app, her).is_none(), "the line fired while standing");
}

/// She lands at the anchor, not past it. At 900px/s a tick covers 15px, and
/// 15px past a ledge is beside it, not on it.
#[test]
fn the_reel_stops_on_the_anchor_and_hands_her_to_gravity() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    throw(&mut app, her, 150.0);
    let anchor = reel(&app, her).expect("bit the ledge").anchor;
    for _ in 0..30 {
        if reel(&app, her).is_none() {
            break;
        }
        app.update();
    }
    assert!(reel(&app, her).is_none(), "the reel never let go");
    let kin = body(&app, her);
    // Not "on the anchor": the anchor is a hang position that overlaps the
    // wall slightly, so the swept resolve stops her about 1px short. The
    // reel's contract is "somewhere the authority accepts"
    // (`where_the_reel_releases_her_is_a_place_the_authority_would_catch`).
    // Here we check she gets close.
    assert!(
        (kin.pos - anchor).length() < 24.0,
        "released at {:?}, {}px from the anchor {:?} — that is not a reel, it is \
         a shrug",
        kin.pos,
        (kin.pos - anchor).length(),
        anchor,
    );
    // Zero: the ledge authority catches a falling body (a stick wall normal
    // or `FALL_SNAP_MIN_VY` of descent), not one moving upward.
    assert_eq!(
        kin.vel,
        ae::Vec2::ZERO,
        "released still moving, so the authority cannot catch her",
    );
}

/// The handover, checked with the authority's own probe:
/// `probe_ledge_grab_in_frame`, which the kernel calls every frame from
/// `try_start_ledge_grab_clusters_in_frame`.
///
/// This is the geometric half only; catching also needs a stick wall normal or
/// `FALL_SNAP_MIN_VY` of descent, which gravity supplies (see the header).
#[test]
fn where_the_reel_releases_her_is_a_place_the_authority_would_catch() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    throw(&mut app, her, 150.0);
    for _ in 0..30 {
        if reel(&app, her).is_none() {
            break;
        }
        app.update();
    }
    assert!(reel(&app, her).is_none(), "the reel never let go");
    let kin = body(&app, her);
    let world = ae::World::new(
        "tether",
        ae::Vec2::new(800.0, 600.0),
        ae::Vec2::ZERO,
        stage(),
    );
    let contact = ae::ledge_grab::probe_ledge_grab_in_frame(
        kin.pos,
        kin.size,
        // She faces right, so the wall's face points back at her: -1.0, as in
        // the reference probe test.
        -1.0,
        &world,
        ambition_platformer2d::world::ResolvedMotionFrame::default().down(),
    );
    assert!(
        contact.is_some(),
        "the reel released her at {:?}, and the ledge authority's own probe does \
         not find a ledge from there",
        kin.pos,
    );
}

/// A reel authored at exactly its budget must pull.
///
/// `author_tether_pull` asserts `speed * timeout_s >= reach`. The budget is
/// tested before it is spent, so an N-tick reel gets N pulls. The assertion is
/// the movement, not the component: a reel that exists but pulls nothing
/// would pass a presence check.
#[test]
fn a_reel_with_exactly_one_tick_left_spends_it_pulling() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    let anchor = START + ae::Vec2::new(60.0, 0.0);
    {
        let world = app.world_mut();
        world.entity_mut(her).insert(TetherReel {
            // Exactly one tick at the fixture's 1/60 dt, the boundary the
            // authoring assert accepts.
            remaining_s: 1.0 / 60.0,
            speed: 900.0,
            anchor,
        });
        world.entity_mut(her).get_mut::<ae::BodyKinematics>().unwrap().vel = ae::Vec2::ZERO;
    }
    app.update();
    let moved = body(&app, her).vel;
    assert!(
        moved.length() > 0.0,
        "a reel with a full tick of budget issued no pull at all: vel {moved:?}",
    );
    assert!(
        moved.x > 0.0,
        "the pull went the wrong way for an anchor to her right: {moved:?}",
    );
}

/// N ticks of budget buy N pulls, counted: "it moved" is true of a reel that
/// lost one tick of five.
#[test]
fn a_reel_spends_every_tick_of_its_budget() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    let anchor = START + ae::Vec2::new(4000.0, 0.0);
    {
        let world = app.world_mut();
        world.entity_mut(her).insert(TetherReel {
            // Three ticks exactly.
            remaining_s: 3.0 / 60.0,
            speed: 900.0,
            anchor,
        });
    }
    // Zero the velocity before every tick: `vel` persists, so leftover motion
    // could make the give-up tick look like a pull.
    let mut pulls = 0;
    for _ in 0..4 {
        app.world_mut()
            .entity_mut(her)
            .get_mut::<ae::BodyKinematics>()
            .unwrap()
            .vel = ae::Vec2::ZERO;
        app.update();
        if body(&app, her).vel.length() > 0.0 {
            pulls += 1;
        }
    }
    assert_eq!(pulls, 3, "three ticks of budget must buy three pulls, not {pulls}");
}

/// Giving up is not arriving: an expired reel leaves her momentum alone, or
/// it would delete her remaining recovery.
#[test]
fn a_reel_that_gives_up_leaves_her_momentum_alone() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    let drifting = ae::Vec2::new(-40.0, 120.0);
    {
        let world = app.world_mut();
        world.entity_mut(her).insert(TetherReel {
            // Already expired: this update gives up. Any budget above zero is
            // spent pulling.
            remaining_s: 0.0,
            speed: 900.0,
            anchor: ae::Vec2::new(4000.0, 110.0),
        });
        world.entity_mut(her).get_mut::<ae::BodyKinematics>().unwrap().vel = drifting;
    }
    app.update();
    assert!(reel(&app, her).is_none(), "the expired reel kept pulling");
    assert_eq!(
        body(&app, her).vel,
        drifting,
        "giving up stopped her dead instead of leaving her momentum alone",
    );
}
