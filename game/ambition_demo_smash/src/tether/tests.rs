//! ⛔⛔ THE REACH IS THE MOVE. "It pulls her to a ledge" would pass against a
//! tether that finds any ledge anywhere, which is a recovery nobody has to
//! position for and the opposite of what a tether costs to use. So the bite
//! tests come in pairs: the same stage, the same fighter, one reach that
//! arrives and one that does not.
//!
//! ⚠ THE FIXTURE INTEGRATES POSITION ITSELF. These tests run the two tether
//! systems without the movement kernel, so nothing would turn a commanded
//! velocity into travel and a reel would never arrive. `integrate` below is a
//! stand-in for exactly one thing the kernel does — `pos += vel * dt` — and it
//! is deliberately the dumbest possible version, because a smarter one would be
//! this file quietly testing a second implementation of movement.
//!
//! ⛔ WHAT THESE TESTS DO NOT COVER, said plainly rather than left to a reader
//! to discover: they do not prove she ENDS IN A LEDGE HANG. That is the ledge
//! authority's job and it runs inside the kernel, so it needs a fixture with
//! real movement. What these prove is the contract this module actually owns —
//! that the line bites only within its reach, that the reel lands ON the anchor
//! rather than past it, and that it hands her over in the state the authority
//! can catch.

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
    // root, not as a resource — `SessionWorldRef` is a `Single<Ref<T>, With<
    // SessionRoot>>`. This helper is the sanctioned way to put one there, and
    // using it keeps the fixture honest about how production supplies geometry.
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
        .write_message(ActorActionMessage { actor, request });
    app.update();
}

fn reel(app: &App, who: Entity) -> Option<TetherReel> {
    app.world().get::<TetherReel>(who).cloned()
}

fn body(app: &App, who: Entity) -> ae::BodyKinematics {
    app.world().get::<ae::BodyKinematics>(who).unwrap().clone()
}

/// ⭐ A line long enough to reach the lip bites it and starts reeling.
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

/// ⛔ THE PAIRED MISS, and it is the half that makes the test above mean
/// something: the SAME stage and the SAME fighter with a line too short.
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

/// ⛔ A TETHER IS AN AERIAL MOVE. On the ground the same fiction is her grab,
/// and a line that fired while standing would be a free horizontal dash.
#[test]
fn a_tether_thrown_from_the_ground_does_not_fire() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, true);
    throw(&mut app, her, 150.0);
    assert!(reel(&app, her).is_none(), "the line fired while standing");
}

/// ⭐⭐ SHE LANDS ON THE ANCHOR, NOT PAST IT. At 900px/s a tick covers 15px, so
/// a reel that ran at a flat speed would overshoot the lip by up to that much —
/// and 15px past a ledge is a fighter beside the ledge rather than on it.
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
    assert!(
        (kin.pos - anchor).length() < 1.0,
        "released at {:?}, {}px from the anchor {:?}",
        kin.pos,
        (kin.pos - anchor).length(),
        anchor,
    );
    // ⭐ ZERO, because the ledge authority catches a FALLING body: it wants a
    // requested wall normal from the stick or `FALL_SNAP_MIN_VY` of descent, and
    // a fighter still travelling upward at the lip satisfies neither.
    assert_eq!(
        kin.vel,
        ae::Vec2::ZERO,
        "released still moving, so the authority cannot catch her",
    );
}

/// ⛔ GIVING UP IS NOT ARRIVING. The first draft collapsed the two exits, which
/// meant an expired reel also stopped her dead — deleting the recovery she had
/// left and reading as the game freezing her in the air.
#[test]
fn a_reel_that_gives_up_leaves_her_momentum_alone() {
    let mut app = app(stage());
    let her = fighter(&mut app, START, false);
    let drifting = ae::Vec2::new(-40.0, 120.0);
    {
        let world = app.world_mut();
        world.entity_mut(her).insert(TetherReel {
            // Already expired: this update is the one that gives up.
            remaining_s: 1.0 / 120.0,
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
