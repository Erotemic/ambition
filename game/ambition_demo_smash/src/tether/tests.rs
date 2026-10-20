//! The reach is the move: a tether that finds any ledge anywhere is not a
//! tether. So bite tests come in pairs: the same stage and fighter, one reach
//! that arrives and one that does not. The tether comes from its module on
//! the module road.
//!
//! The fixture integrates position itself, after the specials; `integrate`
//! does only `pos += vel * dt`, kept minimal so the file does not test a
//! second movement implementation.
//!
//! These tests do not prove she ends in a ledge hang; that is the ledge
//! authority's job inside the kernel. They prove what the move owns: the
//! line bites only within reach, the reel lands on the anchor, and it hands
//! her over in a state the authority can catch. Whether a reel holds is read
//! from the line she shows.

use bevy::prelude::*;

use crate::module_road::{app_on, runner_of, Road};
use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::world::Block;
use ambition_platformer2d::entity_catalog::smash_tether::{TetherPullParams, TETHER_PULL};
use ambition_platformer2d::extension::sdk::phases;
use ambition_platformer2d::extension::ExtensionSet;

const DT: f32 = 1.0 / 60.0;

/// The reference geometry from `ledge_grab`'s own probe tests: a block spanning
/// x 100..300, y 100..300, whose left lip a body centred near x = 86 catches.
fn stage() -> Vec<Block> {
    vec![Block::solid("ledge", ae::Vec2::new(100.0, 100.0), ae::Vec2::new(200.0, 200.0))]
}

const PLAYER_SIZE: ae::Vec2 = ae::Vec2::new(28.0, 46.0);
/// 60px short of the lip.
const START: ae::Vec2 = ae::Vec2::new(26.0, 110.0);

fn integrate(time: Res<ambition_platformer2d::time::WorldTime>, mut bodies: Query<&mut ae::BodyKinematics>) {
    let dt = time.sim_dt();
    for mut kin in &mut bodies {
        let velocity = kin.vel;
        kin.pos += velocity * dt;
    }
}

fn app_with(road: Road, blocks: Vec<Block>) -> App {
    let mut app = app_on(road, ambition_demo_smash_modules::tether::module);
    // The room reaches the collision world as a component on the session-world
    // root, as in production.
    ambition_platformer2d::platformer::lifecycle::insert_live_room_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new("tether", ae::Vec2::new(800.0, 600.0), ae::Vec2::ZERO, blocks)),
    );
    app.add_systems(Update, integrate.after(ExtensionSet::Lower(phases::HIT_REACTION)));
    app
}

fn app(blocks: Vec<Block>) -> App {
    app_with(Road::Linked, blocks)
}

fn fighter(app: &mut App, at: ae::Vec2, on_ground: bool) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics { pos: at, size: PLAYER_SIZE, facing: 1.0, ..Default::default() },
            ae::BodyGroundState { on_ground, ..Default::default() },
            ambition_platformer2d::world::ResolvedMotionFrame::default(),
        ))
        .id()
}

/// Throw a line of `reach` that reels at `speed` for `timeout_s`; the
/// throw's tick has run.
fn throw_with(app: &mut App, actor: Entity, reach: f32, speed: f32, timeout_s: f32) {
    let params = TetherPullParams { reach, speed, timeout_s };
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(TETHER_PULL.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params).expect("tether params serialize"),
    };
    app.world_mut().write_message(ActorActionMessage { actor, request, move_instance: None });
    app.update();
}

fn throw(app: &mut App, actor: Entity, reach: f32) {
    throw_with(app, actor, reach, 900.0, 0.5);
}

/// Where her line goes while a reel holds her.
fn line(app: &App, who: Entity) -> Option<ae::Vec2> {
    app.world().get::<ae::BodyLineAnchor>(who).map(|line| line.0)
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
    let anchor = line(&app, her).expect("a ledge 60px away is inside a 150px line");
    assert!((anchor.x - 87.0).abs() < 6.0, "latched {anchor:?}, expected the block's left lip near x=87");
    assert!(body(&app, her).vel.x > 0.0, "the line bit and did not pull on its tick");
}

/// The line goes where she faces, in her frame's side axis, not world +x.
///
/// The stage is the one above turned 90°: `(x,y) -> (y,-x)`, slid back into
/// the room, maps the block onto itself. The expected anchor is the rotation
/// of that test's, `(87,119) -> (119,313)`. The decoy sits where a world-x
/// walk would look, so the wrong walk latches it and reels her the wrong way.
#[test]
fn the_line_sweeps_her_own_side_axis_when_the_frame_is_turned() {
    let mut app = app(vec![
        stage().remove(0),
        Block::solid("decoy", ae::Vec2::new(200.0, 320.0), ae::Vec2::new(100.0, 40.0)),
    ]);
    // START, rotated: (26,110) -> (110,-26) -> +400y.
    let her = fighter(&mut app, ae::Vec2::new(110.0, 374.0), false);
    {
        let rotated = ae::MotionFrame::from_direction(ae::Vec2::new(1.0, 0.0), 0.0);
        let mut her_mut = app.world_mut().entity_mut(her);
        let mut frame = her_mut.get_mut::<ambition_platformer2d::world::ResolvedMotionFrame>().unwrap();
        frame.publish_resolved_frame(rotated);
    }
    throw(&mut app, her, 150.0);
    let anchor = line(&app, her).expect("the same 60px-away ledge, asked in her own frame");
    assert!(
        (anchor - ae::Vec2::new(119.0, 313.0)).length() < 6.0,
        "latched {anchor:?}; the reference block's lip is at (119,313) in this frame and the decoy a world-x walk \
         finds is at (219,373)"
    );
}

/// The paired miss: the same stage and fighter with a line too short.
#[test]
fn a_line_too_short_bites_nothing() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    throw(&mut app, her, 30.0);
    assert!(line(&app, her).is_none(), "a 30px line reached a ledge 60px away");
    assert_eq!(body(&app, her).vel, ae::Vec2::ZERO, "a line that bit nothing pulled");
}

/// A tether is an aerial move. On the ground the same fiction is her grab,
/// and a grounded line would be a free horizontal dash.
#[test]
fn a_tether_thrown_from_the_ground_does_not_fire() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, true);
    throw(&mut app, her, 150.0);
    assert!(line(&app, her).is_none(), "the line fired while standing");
}

/// Reel until the line goes, for at most `ticks`; whether it went.
fn reel_out(app: &mut App, her: Entity, ticks: usize) -> bool {
    for _ in 0..ticks {
        if line(app, her).is_none() {
            return true;
        }
        app.update();
    }
    line(app, her).is_none()
}

/// She lands at the anchor, not past it, stopped dead. At 900px/s a tick
/// covers 15px, and 15px past a ledge is beside it, not on it.
#[test]
fn the_reel_stops_on_the_anchor_and_hands_her_to_gravity() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    throw(&mut app, her, 150.0);
    let anchor = line(&app, her).expect("bit the ledge");
    assert!(reel_out(&mut app, her, 30), "the reel never let go");
    let kin = body(&app, her);
    assert!(
        (kin.pos - anchor).length() < 24.0,
        "released at {:?}, {}px from the anchor {anchor:?}",
        kin.pos,
        (kin.pos - anchor).length(),
    );
    // Zero: the ledge authority catches a falling body, not one moving upward.
    assert_eq!(kin.vel, ae::Vec2::ZERO, "released still moving, so the authority cannot catch her");
}

/// The handover, checked with the authority's own probe.
#[test]
fn where_the_reel_releases_her_is_a_place_the_authority_would_catch() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    throw(&mut app, her, 150.0);
    assert!(reel_out(&mut app, her, 30), "the reel never let go");
    let kin = body(&app, her);
    let world = ae::World::new("tether", ae::Vec2::new(800.0, 600.0), ae::Vec2::ZERO, stage());
    let contact = ae::ledge_grab::probe_ledge_grab_in_frame(
        kin.pos,
        kin.size,
        -1.0,
        &world,
        ambition_platformer2d::world::ResolvedMotionFrame::default().down(),
    );
    assert!(contact.is_some(), "the reel released her at {:?}, where the authority's probe finds no ledge", kin.pos);
}

/// N ticks of budget buy N pulls, counted, the throw's own among them: "it
/// moved" is true of a reel that lost one tick of three. Slow, so she does
/// not reach the ledge inside the budget.
#[test]
fn a_reel_spends_every_tick_of_its_budget() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    let pulled = |app: &mut App| {
        let kin = app.world_mut().entity_mut(her).get_mut::<ae::BodyKinematics>().map(|mut kin| std::mem::take(&mut kin.vel));
        kin.is_some_and(|vel| vel.length() > 0.0)
    };
    // Three ticks exactly: `3.0 * DT` rounds to a hair over three in f32.
    throw_with(&mut app, her, 150.0, 120.0, 3.0 / 60.0);
    let mut pulls = usize::from(pulled(&mut app));
    for _ in 0..3 {
        app.update();
        pulls += usize::from(pulled(&mut app));
    }
    assert_eq!(pulls, 3, "three ticks of budget must buy three pulls, not {pulls}");
    assert!(line(&app, her).is_none(), "the spent reel kept its line");
}

/// Giving up is not arriving: an expired reel leaves her momentum alone, or
/// it would delete her remaining recovery.
#[test]
fn a_reel_that_gives_up_leaves_her_momentum_alone() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    throw_with(&mut app, her, 150.0, 120.0, DT);
    assert!(line(&app, her).is_some(), "the line did not bite");
    let drifting = ae::Vec2::new(-40.0, 120.0);
    app.world_mut().entity_mut(her).get_mut::<ae::BodyKinematics>().unwrap().vel = drifting;
    app.update();
    assert!(line(&app, her).is_none(), "the expired reel kept pulling");
    assert_eq!(body(&app, her).vel, drifting, "giving up stopped her dead instead of leaving her momentum alone");
}

/// [`app`] with two live rooms: #0 (`ACTIVATION`) is empty and #1 holds
/// `blocks`. The two live rooms are returned.
fn two_room_app(blocks: Vec<Block>) -> (App, [ambition_platformer2d::platformer::lifecycle::LiveRoomInstance; 2]) {
    use ambition_platformer2d::platformer::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
    let mut app = app(Vec::new());
    let first = {
        let world = app.world_mut();
        world.query_filtered::<Entity, With<RoomInstanceRoot>>().single(world).expect("the fixture seats one live room")
    };
    let live = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
    app.world_mut().entity_mut(first).insert(live[0]);
    app.world_mut().spawn((
        RoomInstanceRoot,
        live[1],
        ae::RoomGeometry(ae::World::new("tether", ae::Vec2::new(800.0, 600.0), ae::Vec2::ZERO, blocks)),
    ));
    (app, live)
}

/// The tick the reel lets go, and her velocity then, for a fighter that
/// throws a 150px line in `room` of [`two_room_app`] (`None`: the one-room
/// [`app`]). `None` when the line bit nothing.
fn release(room: Option<usize>) -> Option<(usize, ae::Vec2)> {
    let (mut app, her) = match room {
        None => {
            let mut app = app(stage());
            let her = fighter(&mut app, START, false);
            (app, her)
        }
        Some(room) => {
            let (mut app, live) = two_room_app(stage());
            let her = fighter(&mut app, START, false);
            app.world_mut()
                .entity_mut(her)
                .insert(ambition_platformer2d::platformer::lifecycle::InRoomInstance(live[room]));
            (app, her)
        }
    };
    throw(&mut app, her, 150.0);
    line(&app, her)?;
    let tick = (1..=30).find(|_| {
        app.update();
        line(&app, her).is_none()
    })?;
    Some((tick, body(&app, her).vel))
}

/// A tether bites a ledge of the fighter's own live room, and the reel lets
/// go where the one-room fixture's does. The ledge is in #1: a fighter in #1
/// bites it; a fighter in #0 at the same place bites nothing.
#[test]
fn a_tether_bites_a_ledge_of_its_own_live_room() {
    let one_room = release(None).expect("control: the one-room fixture's line did not bite");
    assert_eq!(release(Some(1)), Some(one_room), "the fighter in #1 did not reel as in one room");
    assert_eq!(release(Some(0)), None, "the fighter in #0 bit a ledge of #1");
}

/// The tether built from its module file reels as the linked one does.
#[test]
fn the_tether_from_its_module_file_reels_as_the_linked_one_does() {
    let run = |road: Road| {
        let mut app = app_with(road, stage());
        let her = fighter(&mut app, START, false);
        throw(&mut app, her, 150.0);
        let mut path = vec![(body(&app, her).pos, line(&app, her))];
        for _ in 0..10 {
            app.update();
            path.push((body(&app, her).pos, line(&app, her)));
        }
        (path, runner_of(&app, TETHER_PULL))
    };
    let (linked_path, linked) = run(Road::Linked);
    let (loaded_path, loaded) = run(Road::Loaded);
    assert!(loaded.starts_with("Loaded") && !linked.starts_with("Loaded"), "{linked} / {loaded}");
    assert_eq!(loaded_path, linked_path);
    assert!(linked_path.iter().any(|(_, line)| line.is_some()) && linked_path.last().unwrap().1.is_none());
}
