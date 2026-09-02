//! The end-to-end run, on a course nobody authors.
//!
//! the fixture is boring on purpose ([`test_course`]): flat ground, one
//! ?-block, one snake, a goal. What made the old routes brittle was not the
//! level's complexity but the route needing TIMING — a jump that must clear a
//! pit has a distance, a speed and a launch frame that all have to stay true. A
//! course with no pit has no numbers to go stale, and still exercises every seam
//! the playthrough existed for.

use bevy::prelude::*;

use ambition_demo_mary_o::flag::{FlagPhase, FlagSequence};
use ambition_demo_mary_o::ldtk_vocabulary::{block_look_of, MaryOBlockLook};
use ambition_demo_mary_o::powerups::{SpentPowerBlocks, STAR_WAND_ID};
use ambition_demo_mary_o::snake::SnakeShell;
use ambition_demo_mary_o::test_course::{
    course_block_aabb, course_pole_x, test_course, TEST_COURSE_ROOM_ID,
};
use ambition_demo_mary_o::MaryOLevelState;
use ambition_platformer2d::world_items::WorldItem;
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::combat::components::CenteredAabb;
use ambition_platformer2d::engine_core::{self as ae, AabbExt};
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

/// Boot the real host, entering the fixture course rather than 1-1.
///
/// no new host plumbing was needed. The provider installs its world source
/// as a SYSTEM — its own doc says it *"may read the provider's own resources"* —
/// so the entry room is a resource read on the update that prepares the session.
/// Inserting it after the app is built and before the first `update()` is early
/// enough, which is why this is a test-side choice rather than a host variant.
fn boot_course() -> App {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(
        TEST_COURSE_ROOM_ID.to_string(),
    ));
    app
}

fn player_pos(app: &mut App) -> Option<ae::Vec2> {
    let mut q = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    q.iter(app.world()).next().map(|kin| kin.pos)
}

fn settle(app: &mut App) -> ae::Vec2 {
    for _ in 0..600 {
        app.update();
        if let Some(pos) = player_pos(app) {
            if pos.y > 0.0 {
                return pos;
            }
        }
    }
    panic!("the course never produced a playable body");
}

/// She spawns into the course, and the course is the one the fixture built.
///
/// The first thing worth proving is that the entry-room seam works at all: a
/// resource decides which room a session starts in, and a shipped game that does
/// not insert it must still get 1-1.
#[test]
fn the_session_enters_the_fixture_course_when_asked() {
    let mut app = boot_course();
    let spawn = settle(&mut app);
    let block = course_block_aabb();
    assert!(
        spawn.x < block.min.x,
        "she starts left of the course's ?-block, with room to walk at it: \
         spawn {spawn:?} vs block {block:?}"
    );

    // The course's own geometry reached the running session — not 1-1's.
    let mut worlds = app.world_mut().query::<&ae::RoomGeometry>();
    let named: Vec<String> = worlds
        .iter(app.world())
        .map(|geo| geo.0.name.clone())
        .collect();
    assert!(
        named.iter().any(|n| n.contains("test course")),
        "the live room is the fixture course, not the authored level: {named:?}"
    );
}

/// The default is still 1-1, so a shipped game cannot depend on a resource
/// only a test inserts.
#[test]
fn a_host_that_says_nothing_still_enters_level_one() {
    let shipped = ambition_demo_mary_o::mary_o_session_world();
    // the AUTHORED world's name, which the LDtk file supplies — not the string
    // the old Rust builder passed to `World::new`.
    assert!(
        !shipped.geometry.0.name.contains("test course"),
        "no entry resource means the real level, not the fixture: {}",
        shipped.geometry.0.name
    );
    let asked = ambition_demo_mary_o::provider::mary_o_session_world_entering(TEST_COURSE_ROOM_ID);
    assert!(
        asked.geometry.0.name.contains("test course"),
        "and asking for the course gets the course: {}",
        asked.geometry.0.name
    );
}

// ── The run ───────────────────────────────────────────────────────────────
//
// NOT ONE FRAME COUNT IN THIS ROUTE. Every beat below drives to a
// CONDITION read off the course — she is under the block, her head has reached
// its underside, the snake is in reach, the pole has answered — because a route
// expressed in frames is exactly what killed the two runs this replaces. The one
// number here is `LIVENESS_CAP`, and it measures nothing about the route.

/// `mary_o_tall`'s id is private to the demo's `powerups` module; the demo's own
/// `power_loop.rs` and the 1-1 acceptance run both name it the same way.
const TALL_ID: &str = "mary_o_tall";

/// A DEADLOCK DETECTOR, not a route measurement.
///
/// No beat of this run is timed: each one steps until the course says it is done. This is the
/// backstop for the case where it never will be — a body wedged against a wall, a bonk that cannot
/// reach, a goal that no longer answers — so a broken run fails loudly with the beat's name in it
/// instead of hanging the suite.
const LIVENESS_CAP: usize = 6000;

/// The scripted stick, republished every frame in `PreUpdate` because Bevy runs
/// the fixed-timestep loop BEFORE `Update` — intent written any later is not seen
/// by the tick it was meant to drive.

/// The course, booted with a stick in her hand.
fn boot_course_scripted() -> App {
    let mut app = boot_course();
    // the ordering lives in ONE place now — after the participant pipeline's routing stage and
    // before the frame→tick latch.
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    app
}

/// Everything the route may look at — where she is, how fast, how big, and
/// whether she has footing. What a player reads off the screen, with no
/// privileged access to level state.
#[derive(Clone, Copy, Debug)]
struct Body {
    pos: Vec2,
    vel: Vec2,
    size: Vec2,
    on_ground: bool,
}

impl Body {
    fn head(&self) -> f32 {
        self.pos.y - self.size.y * 0.5
    }
    fn feet(&self) -> f32 {
        self.pos.y + self.size.y * 0.5
    }
    fn right(&self) -> f32 {
        self.pos.x + self.size.x * 0.5
    }
    /// `+y` is down, so a negative `y` velocity is a body on the way UP.
    fn rising(&self) -> bool {
        self.vel.y < 0.0
    }
}

fn body(app: &mut App) -> Body {
    let mut q = app
        .world_mut()
        .query_filtered::<(&ae::BodyKinematics, &ae::BodyGroundState), With<PrimaryPlayer>>();
    let (kin, ground) = q
        .iter(app.world())
        .next()
        .expect("the course has a playable body");
    Body {
        pos: kin.pos,
        vel: kin.vel,
        size: kin.size,
        on_ground: ground.on_ground,
    }
}

// ── Input vocabulary ──────────────────────────────────────────────────────

fn move_x(dir: f32) -> ControlFrame {
    ControlFrame {
        axis_x: dir,
        right_pressed: dir > 0.0,
        left_pressed: dir < 0.0,
        ..ControlFrame::default()
    }
}

fn with_jump(mut frame: ControlFrame) -> ControlFrame {
    frame.jump_pressed = true;
    frame.jump_held = true;
    frame
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

/// Step the real schedule until the beat's own condition says it is finished.
///
/// `choose` returns `None` when the course has answered and `Some(frame)` for
/// what to press otherwise, so a beat is written as *"press this until that is
/// true"* and never as *"press this for N frames"*. Overrunning [`LIVENESS_CAP`]
/// is a hard failure naming the beat — the run is wedged, and a silent
/// fall-through here is how a route goes green by not happening.
fn drive_until(
    app: &mut App,
    beat: &str,
    mut choose: impl FnMut(&mut App) -> Option<ControlFrame>,
) {
    for _ in 0..LIVENESS_CAP {
        match choose(app) {
            None => return,
            Some(frame) => step(app, frame),
        }
    }
    panic!(
        "the run wedged on `{beat}` — {LIVENESS_CAP} frames without the course \
         answering. She is at {:?}",
        body(app)
    );
}

// ── Reading the course ────────────────────────────────────────────────────

/// The course's one ?-block, as the room authors it — so the bonk is checked
/// against the block's own durable id rather than against a rebuilt one.
fn course_power_block() -> ae::world::Block {
    test_course()
        .world
        .blocks
        .iter()
        .find(|block| block_look_of(&block.name) == Some(MaryOBlockLook::Question))
        .expect("the course authors one ?-block")
        .clone()
}

fn block_is_spent(app: &App) -> bool {
    app.world()
        .get_resource::<SpentPowerBlocks>()
        .is_some_and(|spent| spent.is_spent(&course_power_block().id))
}

/// Every loose pickup in the room, as `(sprite, position)`.
fn world_items(app: &mut App) -> Vec<(String, ae::Vec2)> {
    let mut q = app.world_mut().query::<&WorldItem>();
    q.iter(app.world())
        .map(|item| {
            (
                item.sprite.clone().unwrap_or_else(|| "?".to_string()),
                item.pos,
            )
        })
        .collect()
}

/// The STOMPABLE snake — the one wearing the demo's own shell marker — with its
/// live box and phase.
///
/// keyed by the marker, not by "the first enemy the query yields". The
/// course stages one authored snake, and the engine's generic placement path was
/// found to build a SECOND, unmarked actor from the same placement.
/// The unmarked twin carries no shell mechanic at all, so "did the snake react"
/// has to be asked of the body that carries the mechanic.
fn shelled_snake(app: &mut App) -> Option<(ae::Aabb, SnakeShell)> {
    let mut q = app.world_mut().query::<(&CenteredAabb, &SnakeShell)>();
    q.iter(app.world())
        .map(|(aabb, shell)| (aabb.aabb(), *shell))
        .next()
}

fn wears_wand(app: &mut App) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<&WornEquipment, With<PrimaryPlayer>>();
    q.iter(app.world())
        .next()
        .is_some_and(|worn| worn.wears(STAR_WAND_ID))
}

fn worn_form(app: &mut App) -> Option<String> {
    let mut q = app
        .world_mut()
        .query_filtered::<&WornCharacter, With<PrimaryPlayer>>();
    q.iter(app.world()).next().map(|worn| worn.id().to_string())
}

fn health(app: &mut App) -> i32 {
    let mut q = app
        .world_mut()
        .query_filtered::<&BodyHealth, With<PrimaryPlayer>>();
    q.iter(app.world())
        .next()
        .expect("she has a health pool")
        .current()
}

fn lives(app: &mut App) -> i8 {
    let mut q = app.world_mut().query::<&MaryOLevelState>();
    q.iter(app.world())
        .next()
        .expect("the level owner exists once the course is playable")
        .lives
}

fn flag_phase(app: &mut App) -> Option<FlagPhase> {
    let mut q = app.world_mut().query::<&FlagSequence>();
    q.iter(app.world()).next().map(|seq| seq.phase)
}

/// `ManualDuration` pins the sim clock, but it does not pin BOOT: session
/// activation and asset loading advance on real I/O over a variable number of
/// frames, so "eight frames is enough to be playing" is a bet on machine load and
/// it loses under a parallel suite. A body standing on the ground is the honest
/// readiness signal.
fn settle_until_playable(app: &mut App) {
    for _ in 0..LIVENESS_CAP {
        app.update();
        let mut q = app
            .world_mut()
            .query_filtered::<(&ae::BodyKinematics, &ae::BodyGroundState), With<PrimaryPlayer>>();
        if q.iter(app.world()).next().is_some_and(|(_, g)| g.on_ground) {
            return;
        }
    }
    panic!("the course never put a playable body on its ground");
}

/// Assert that scripted input survives the composed participant pipeline.
/// Probe `ControlFrame` immediately after `Update`: the scripted delivery counter
/// observes the slot table from `FixedUpdate` and can lag the first press. The
/// scripted writer is ordered after normal input routing so a composed input
/// feature cannot erase the test press.
#[track_caller]
fn assert_scripted_input_reaches_the_sim(app: &mut App) {
    // `ControlFrame` is seat zero's OUTPUT mirror since, so the same line finally measures what
    // it claims.
    let probe = ControlFrame {
        aim_x: 1.0,
        ..ControlFrame::default()
    };
    for _ in 0..20 {
        step(app, probe);
        if app.world().resource::<ControlFrame>().aim_x > 0.5 {
            step(app, ControlFrame::default());
            return;
        }
    }
    panic!(
        "a scripted press did not survive into the simulation, so the whole \
         playthrough would report a course nobody walked"
    );
}

/// She plays the course: walk, bonk, take what pops, stomp, finish.
///
/// Every mechanic here has its own focused probe against the authored level; what only a run can
/// see is that they are CONNECTED, and that a body driven by nothing but a stick gets from one end
/// of a level to the other.
#[test]
fn she_plays_the_course_from_spawn_to_the_goal() {
    let mut app = boot_course_scripted();
    settle_until_playable(&mut app);

    assert_scripted_input_reaches_the_sim(&mut app);

    let spawn = body(&mut app);
    let lives_at_spawn = lives(&mut app);
    let block = course_power_block().aabb;
    eprintln!("SPAWN {spawn:?} block {block:?} pole x {}", course_pole_x());

    // ── 1. She WALKS ──────────────────────────────────────────────────────
    //
    // Held right moves her right, along the ground, and costs her nothing on the
    // way. On forty tiles of unbroken floor there is nothing else it could mean —
    // which is the fixture's whole point.
    let mut left_the_ground = false;
    let mut went_backwards = false;
    let mut furthest = spawn.pos.x;
    drive_until(&mut app, "walk to the ?-block", |app| {
        let b = body(app);
        left_the_ground |= !b.on_ground;
        went_backwards |= b.pos.x < furthest - 1.0;
        furthest = furthest.max(b.pos.x);
        (b.pos.x < block.center().x).then(|| move_x(1.0))
    });
    let walked = body(&mut app);
    assert!(
        walked.pos.x > spawn.pos.x,
        "holding right walks her right: {:?} -> {:?}",
        spawn.pos,
        walked.pos
    );
    assert!(
        !left_the_ground,
        "and she WALKS it — flat ground with no jump pressed must not put her in \
         the air, so an airborne frame here is a hole or a shove, not a walk"
    );
    assert!(
        !went_backwards,
        "and she never travels backwards while the stick is held right"
    );
    assert_eq!(
        lives(&mut app),
        lives_at_spawn,
        "walking the course costs nothing"
    );

    // ── 2. She BONKS the ?-block, and takes what pops out ─────────────────
    //
    // hold jump only until her HEAD reaches the underside, then release. A
    // held classic jump rises far past a row-four block and she sails over it; a
    // bare tap comes up short. Steering off the measurement — her head against the
    // block's own `max.y` — rather than off a frame count is what keeps this true
    // if her tuning or the block's row ever moves.
    assert!(!block_is_spent(&app), "the block starts unstruck");
    let items_before = world_items(&mut app).len();
    drive_until(&mut app, "bonk the ?-block", |app| {
        if block_is_spent(app) {
            return None;
        }
        let b = body(app);
        let under_it = (b.pos.x - block.center().x).abs() < 8.0;
        if b.on_ground && !under_it {
            return Some(move_x((block.center().x - b.pos.x).clamp(-1.0, 1.0)));
        }
        if b.on_ground {
            // Straight up. Steering while airborne is what carries her OVER the
            // block instead of into its underside.
            return Some(with_jump(move_x(0.0)));
        }
        Some(if b.head() > block.max.y {
            with_jump(move_x(0.0))
        } else {
            move_x(0.0)
        })
    });
    assert!(
        block_is_spent(&app),
        "a head contact under the ?-block must SPEND it — that resource is the \
         game's own record that a block gave up its reward, and without it nothing \
         was ever spawned for her to take"
    );

    // The reward starts INSIDE the block and rises out of it. Watching it rise is
    // what separates "an item exists" from "the pop happened".
    let popped = world_items(&mut app);
    assert!(
        popped.len() > items_before,
        "the bonk pops a pickup: {popped:?}"
    );
    let mut highest = f32::MAX;
    drive_until(&mut app, "watch the reward rise", |app| {
        let items = world_items(app);
        let Some(pos) = items.first().map(|(_, pos)| *pos) else {
            return None; // she caught it on the way up; nothing left to watch
        };
        highest = highest.min(pos.y);
        (highest > block.min.y - 1.0).then(|| move_x(0.0))
    });
    assert!(
        highest < block.min.y,
        "the reward RISES out of the block rather than sitting inside it: best y \
         {highest:.1}, block top {:.1}",
        block.min.y
    );

    // …and it has to actually reach her. It travels once it is out, so she walks
    // at it — she is several times faster than it is, so this converges from
    // either side without anybody timing anything.
    drive_until(&mut app, "take the wand", |app| {
        if wears_wand(app) {
            return None;
        }
        let b = body(app);
        let target = world_items(app)
            .first()
            .map(|(_, pos)| pos.x)
            .unwrap_or(b.pos.x);
        Some(move_x((target - b.pos.x).clamp(-1.0, 1.0)))
    });
    assert!(
        wears_wand(&mut app),
        "the wand equips through the shared equipment path — a reward that pops \
         and cannot be picked up is not a powerup"
    );
    assert_eq!(
        worn_form(&mut app).as_deref(),
        Some(TALL_ID),
        "and the worn form follows the equipment"
    );
    eprintln!("POWERED UP at {:?}", body(&mut app));

    // ── 3. She STOMPS the snake, and takes nothing for it ─────────────────
    //
    // From above it is a stomp; from the side, with the wand on, it costs her the
    // wand. So the armor she is wearing IS the assertion: she must come off this
    // beat still wearing it.
    let (snake_box, phase) = shelled_snake(&mut app).expect("the course stages one snake");
    assert_eq!(
        phase,
        SnakeShell::Walking,
        "the snake starts as a live walker"
    );
    let hp_before_stomp = health(&mut app);
    drive_until(&mut app, "stomp the snake", |app| {
        let Some((snake, phase)) = shelled_snake(app) else {
            panic!("the snake left the world mid-approach");
        };
        if phase != SnakeShell::Walking {
            return None;
        }
        let b = body(app);
        // the stomp is decided by her FEET against the snake's top face, not
        // by a distance she is allowed to jump from. Above that face, steer for the
        // middle of its back and come down on it; anywhere else, back out of its
        // reach — a body that is level with a walker and moving into it is taking a
        // side hit however the jump was timed. Both halves are geometry, so neither
        // goes stale when her tuning or the snake's box changes.
        let toward = (snake.center().x - b.pos.x).clamp(-1.0, 1.0);
        let above_its_back = b.feet() < snake.min.y;
        let within_reach = snake.min.x - b.right() < b.size.x;
        let steer = if above_its_back || !within_reach {
            toward
        } else {
            -toward
        };
        // SHE MAY NOT CHASE PAST THE GOAL, because in real play she cannot — walking into the
        // pole starts the flag sequence and ends the level with the snake still alive.
        //
        // this stays GEOMETRY, which is the fixture's whole design rule: hold
        // two tiles short of the pole and let the snake come back. It bounces off
        // `course_wall_right` and walks west again — that is what makes waiting a
        // bounded thing to do rather than a timing assumption.
        //
        // Walking BACK is the only input that guarantees she ends up west of the line she must not
        // cross.
        let chase_limit = course_pole_x() - 2.0 * 32.0;
        let steer = if b.right() >= chase_limit {
            -1.0
        } else {
            steer
        };
        // Press on the ground, hold while rising, release on the way down: her
        // jump is variable, so a release mid-rise cuts the arc, and the edge has
        // to be given back before the next hop can start.
        Some(if b.on_ground || b.rising() {
            with_jump(move_x(steer))
        } else {
            move_x(steer)
        })
    });
    let (_, stomped) = shelled_snake(&mut app).expect("the snake is still a body");
    assert_ne!(
        stomped,
        SnakeShell::Walking,
        "landing on the snake must put it into its shell"
    );
    assert_eq!(
        health(&mut app),
        hp_before_stomp,
        "and she must not take a scratch for landing on it"
    );
    assert!(
        wears_wand(&mut app),
        "nor lose the armor — a stomp that costs her the wand is a side contact \
         wearing a stomp's clothes"
    );
    eprintln!(
        "STOMPED {stomped:?} (snake was at {snake_box:?}) — she is {:?}",
        body(&mut app)
    );

    // ── 4. She reaches the GOAL ───────────────────────────────────────────
    //
    // The victory signal in this codebase is the flag sequence settling into
    // `Tallied`: grab → slide → walk-off → tally. Running at the pole is the whole
    // input; everything after the grab is the sequence driving her.
    assert_eq!(
        flag_phase(&mut app),
        Some(FlagPhase::Idle),
        "the goal has not been touched yet"
    );
    drive_until(&mut app, "reach the goal", |app| {
        (!matches!(flag_phase(app), Some(FlagPhase::Tallied { .. }))).then(|| move_x(1.0))
    });
    let Some(FlagPhase::Tallied { score }) = flag_phase(&mut app) else {
        panic!(
            "touching the goal must run the flag sequence through to a settled \
             tally; phase is {:?} and she is at {:?}",
            flag_phase(&mut app),
            body(&mut app)
        );
    };
    assert!(score > 0, "the grab banks a score");
    assert_eq!(
        lives(&mut app),
        lives_at_spawn,
        "she finished the course without dying once — a run that spends a life \
         has shown the course is survivable, not that it is playable"
    );
    assert!(
        wears_wand(&mut app),
        "and arrives still wearing what she took off the ?-block"
    );
    eprintln!("TALLIED {score} at {:?}", body(&mut app));
}
