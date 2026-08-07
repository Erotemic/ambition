//! **The demo changes rooms.**
//!
//! This is the exit proof for moving the room-transition transaction into the
//! engine (2026-07-25). Before it, `RoomTransitionRequested` had exactly one
//! consumer and only `ambition_app` registered it — so in THIS binary, which
//! deliberately does not depend on `ambition_app`, the message went into a
//! registered channel that nothing drained. A second room was not merely
//! unauthored; it was unreachable.
//!
//! So the assertion that matters is not "a transition was requested" — the
//! request always worked. It is that the ACTIVE ROOM CHANGED and the body is
//! standing in the new room's geometry, which is only true if a consumer ran.
//! (Room-replay §2.5 shipped three green proofs of a beat that had no consumer
//! in-process, each asserting a value the emitter wrote one line earlier. Not
//! again.)
//!
//! ## Driving the stick under either composition
//!
//! The scripted stick used to run in `PreUpdate` behind
//! `#![cfg(not(feature = "input"))]`, which was the WRONG PREDICATE and made
//! this file red under `cargo test --workspace` for as long as that command has
//! existed. The cfg reads THIS crate's `input` feature, but what actually
//! decides whether the participant pipeline owns `ControlFrame` is
//! `ambition_platformer2d/input` — and workspace feature unification turns that on from
//! `ambition_app`'s defaults no matter what this crate asked for. So the guard
//! silently stopped guarding: the file compiled, the pipeline ran in `Update`
//! and overwrote every `PreUpdate` write, and Mary-O simply never moved.
//!
//! The fix is to stop guessing and order against the authority: the stick is
//! written in `Update` after `InputSet::Route`, which is where the pipeline
//! declares all its `ControlFrame`-writing systems live. That is last-writer-wins
//! by construction rather than by composition luck, so this proof holds under
//! `-p` and `--workspace` alike.

use ambition_demo_mary_o::level_1_2::LEVEL_1_2_ROOM_ID;
use ambition_demo_mary_o::LEVEL_1_1_ROOM_ID;
use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::RoomSet;
use bevy::prelude::*;

#[derive(Resource, Clone, Copy, Default)]
struct ScriptedStick(ControlFrame);

fn apply_scripted_stick(stick: Res<ScriptedStick>, mut frame: ResMut<ControlFrame>) {
    *frame = stick.0;
}

fn boot() -> App {
    let mut app = build_demo_app();
    app.init_resource::<ScriptedStick>();
    // AFTER the pipeline's routing stage and BEFORE the frame->tick latch reads
    // it. Both edges are load-bearing under a fixed-tick host: `InputSet::Route`
    // is where the participant pipeline writes `ControlFrame`, and
    // `accumulate_control_frame_latch` is what the sim actually consumes —
    // `publish_latched_control_frame` overwrites `ControlFrame` from the latch
    // inside the sim schedule, so a write that misses the latch never reaches
    // gameplay no matter how late it lands in `Update`.
    //
    // Ordering against a set or system nobody composed is a no-op, so the
    // headless frame-stepped composition (which has no latch) is unaffected.
    app.add_systems(
        Update,
        apply_scripted_stick
            .after(ambition_platformer2d::input::InputSet::Route)
            .before(ambition_platformer2d::engine_core::accumulate_control_frame_latch),
    );
    // Settle activation: the provider publishes its world over several frames.
    for _ in 0..90 {
        app.update();
    }
    app
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut().resource_mut::<ScriptedStick>().0 = frame;
    app.update();
}

fn hold_right() -> ControlFrame {
    let mut frame = ControlFrame::default();
    frame.axis_x = 1.0;
    frame.right_pressed = true;
    frame
}

fn press_down() -> ControlFrame {
    let mut frame = ControlFrame::default();
    frame.axis_y = 1.0;
    frame
}

fn player_pos(app: &mut App) -> Vec2 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, With<PrimaryPlayer>>(
        );
    query
        .iter(app.world())
        .next()
        .expect("gameplay has a primary player")
        .pos
}

/// The id of the room that is actually AUTHORITATIVE right now — the fact a
/// transition has to change for anything to have happened.
fn active_room(app: &mut App) -> String {
    let mut query = app.world_mut().query::<&RoomSet>();
    let world = app.world();
    let set = query.iter(world).next().expect("the session has a RoomSet");
    set.rooms[set.active].id.clone()
}

fn place_player(app: &mut App, pos: Vec2) {
    let mut query = app.world_mut().query_filtered::<(
        ambition_platformer2d::engine_core::BodyClusterQueryData,
        &mut ambition_platformer2d::actors::features::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut cluster_item, mut motion_model) = query
        .iter_mut(world)
        .next()
        .expect("gameplay has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ambition_platformer2d::engine_core::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        pos,
        ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
    );
}

#[test]
fn she_walks_out_of_one_room_and_into_another() {
    let mut app = boot();
    assert_eq!(
        active_room(&mut app),
        LEVEL_1_1_ROOM_ID,
        "the demo should start on the surface",
    );

    // ⛔ **this used to walk into a DESCENT SHAFT in 1-1's vault, and that shaft
    // is gone (2026-08-06).** Jon: *"she doesn't just get to go there in the
    // middle of 1-1 and come back."* The only route from 1-1 to 1-2 is finishing
    // 1-1, so that is the route this proof drives now — the assertion it was
    // written for is about the TRANSACTION (the active room changed and the body
    // is standing in the new geometry), not about which affordance started it.
    //
    // Set down a short walk from the pole, then WALK into it. The placement is
    // the same concession `level_circuit` makes and for the same reason — 1-1 is
    // 3328px of platforming and this is not a playthrough test — but the last
    // stretch is played, so "reachable by moving" still means something.
    let pole = ambition_demo_mary_o::pole_for_room(LEVEL_1_1_ROOM_ID);
    place_player(&mut app, Vec2::new(pole.x - 96.0, pole.base_y - 24.0));
    for _ in 0..8 {
        step(&mut app, ControlFrame::default());
    }

    let mut room = active_room(&mut app);
    for _ in 0..900 {
        step(&mut app, hold_right());
        room = active_room(&mut app);
        if room == LEVEL_1_2_ROOM_ID {
            break;
        }
    }

    assert_eq!(
        room, LEVEL_1_2_ROOM_ID,
        "walking into 1-1's goal pole did not change the active room — the \
         transition either never fired or nothing consumed it",
    );

    // And she is IN the new room, not merely bookkept into it. 1-2's corridor
    // floor is at the bottom of a 14-tile room; 1-1's vault floor is elsewhere,
    // so a body still standing in the old geometry fails this.
    //
    // ⚠ **the flip and the body are not the SAME frame on this route.** Under
    // the deleted descent shaft the transaction that changed the room also moved
    // the body, so reading the position on the flip frame was safe. The flag
    // route asks for the room while the sequence still holds her at the pole,
    // and she is placed at the target room's spawn when it commits — so this
    // read one frame early and reported her at 1-1's x=3240, which looks exactly
    // like "the body never moved".
    //
    // ⛔ so it WAITS rather than settling a fixed count: a bounded wait that
    // reports the last position it saw still fails if she never arrives, which a
    // blind `for 0..120` before the read would have hidden.
    let world_size = {
        let mut query = app.world_mut().query::<&RoomSet>();
        let world = app.world();
        let set = query.iter(world).next().expect("a RoomSet");
        set.rooms[set.active].world.size
    };
    let inside = |pos: Vec2| {
        pos.x >= 0.0 && pos.x <= world_size.x && pos.y >= 0.0 && pos.y <= world_size.y
    };
    let mut pos = player_pos(&mut app);
    for _ in 0..120 {
        if inside(pos) {
            break;
        }
        step(&mut app, ControlFrame::default());
        pos = player_pos(&mut app);
    }
    assert!(
        inside(pos),
        "she never landed inside 1-2's bounds — last seen at {pos:?} (room is \
         {world_size:?}) 120 frames after the active room became 1-2",
    );
    assert_eq!(
        active_room(&mut app),
        LEVEL_1_2_ROOM_ID,
        "she did not STAY in 1-2 — something bounced her back out",
    );
}

#[test]
fn the_two_rooms_are_linked_both_ways() {
    let mut app = boot();
    let mut query = app.world_mut().query::<&RoomSet>();
    let world = app.world();
    let set = query.iter(world).next().expect("the session has a RoomSet");

    let ids: Vec<&str> = set.rooms.iter().map(|room| room.id.as_str()).collect();
    assert!(ids.contains(&LEVEL_1_1_ROOM_ID), "1-1 missing from {ids:?}");
    assert!(ids.contains(&LEVEL_1_2_ROOM_ID), "1-2 missing from {ids:?}");

    // A one-way link is a trap door: you can reach 1-2 and never come back.
    //
    // ⛔ **this asked the LOADING ZONES, and neither room has one any more**
    // (2026-08-06 — the mid-1-1 round trip Jon rejected was four zones and they
    // are all deleted). Asking them now would be a check that cannot fail in the
    // worst way: it would pass on an empty `for`.
    //
    // ⭐ **so it asks the route that actually exists.** Finishing a level is what
    // moves you between them, `exit_for_room` is where each level says where
    // that goes, and the property worth holding is unchanged and stronger than
    // "has a zone": follow the exits and you come back to where you started.
    let mut seen = vec![LEVEL_1_1_ROOM_ID.to_string()];
    let mut at = LEVEL_1_1_ROOM_ID.to_string();
    for _ in 0..set.rooms.len() {
        let ambition_demo_mary_o::LevelDestination::Room(next) =
            ambition_demo_mary_o::exit_for_room(&at)
        else {
            panic!("room '{at}' replays instead of leading anywhere, so the demo dead-ends there");
        };
        assert!(
            ids.contains(&next.as_str()),
            "room '{at}' leads to '{next}', which is not in this world: {ids:?}",
        );
        at = next;
        if at == LEVEL_1_1_ROOM_ID {
            break;
        }
        seen.push(at.clone());
    }
    assert_eq!(
        at, LEVEL_1_1_ROOM_ID,
        "following each level's exit from 1-1 never came back to it (visited {seen:?})",
    );
    assert!(
        seen.contains(&LEVEL_1_2_ROOM_ID.to_string()),
        "the cycle out of 1-1 never passes through 1-2 (visited {seen:?})",
    );
}

/// The ferry in 1-2 is not decoration: the chasm has no stepping stone, so a
/// body that is not CARRIED cannot cross, and 1-2 is impassable.
///
/// Carrying is engine behavior — the platform advance runs once per frame ahead
/// of the body tick, and the ride/ledge-carry logic reads its delta — so this
/// asserts the invariant rather than a tuned speed: a body standing on the
/// platform moves by the platform's own displacement, with no input at all.
#[test]
fn a_body_standing_on_the_ferry_is_carried_by_it() {
    let mut app = boot();
    reach_level_1_2(&mut app);

    let (platform_pos, platform_size) = ferry(&mut app);
    // Stand her ON the deck: feet on its top face, not centre-on-centre.
    let feet_offset = {
        let mut query = app
            .world_mut()
            .query_filtered::<&ambition_platformer2d::engine_core::BodyKinematics, With<PrimaryPlayer>>();
        let size = query
            .iter(app.world())
            .next()
            .expect("a primary player")
            .size;
        size.y * 0.5
    };
    place_player(
        &mut app,
        Vec2::new(
            platform_pos.x + platform_size.x * 0.5,
            platform_pos.y - feet_offset - 1.0,
        ),
    );
    for _ in 0..6 {
        step(&mut app, ControlFrame::default());
    }

    let body_before = player_pos(&mut app).x;
    let deck_before = ferry(&mut app).0.x;
    for _ in 0..40 {
        step(&mut app, ControlFrame::default());
    }
    let body_moved = player_pos(&mut app).x - body_before;
    let deck_moved = ferry(&mut app).0.x - deck_before;

    assert!(
        deck_moved.abs() > 1.0,
        "the ferry never moved, so this proves nothing about riding it",
    );
    assert!(
        (body_moved - deck_moved).abs() <= 2.0,
        "she did not ride the ferry: it moved {deck_moved:.1}px, she moved {body_moved:.1}px",
    );
}

/// Where the 1-2 ferry is right now, out of the live platform set.
///
/// ⭐ found by its authored ID. It was found by NAME until the converter learned
/// to read one: a platform's runtime id was the LDtk iid, which the file mints
/// and nothing outside it can spell, so presentation was the only handle going —
/// and renaming the platform in the editor would have broken this silently.
fn ferry(app: &mut App) -> (Vec2, Vec2) {
    let set = app
        .world()
        .resource::<ambition_platformer2d::world::collision::MovingPlatformSet>();
    let platform = set
        .0
        .iter()
        .find(|platform| platform.id == ambition_demo_mary_o::level_1_2::FERRY_ID)
        .expect("1-2's ferry is in the live platform set");
    (platform.pos, platform.size)
}

/// Play from the surface into 1-2, the same way the walk proof does.
fn reach_level_1_2(app: &mut App) {
    let pole = ambition_demo_mary_o::pole_for_room(LEVEL_1_1_ROOM_ID);
    place_player(app, Vec2::new(pole.x - 96.0, pole.base_y - 24.0));
    for _ in 0..8 {
        step(app, ControlFrame::default());
    }
    for _ in 0..900 {
        step(app, hold_right());
        if active_room(app) == LEVEL_1_2_ROOM_ID {
            break;
        }
    }
    assert_eq!(
        active_room(app),
        LEVEL_1_2_ROOM_ID,
        "could not reach 1-2 to test its ferry",
    );
}

/// Walk the vault and collect what it pays out.
///
/// Into the vault through the real directional pipe — stand on the mouth and
/// press DOWN, the verb Jon asked for — then along its floor. The far end is
/// masonry (`vault_wall_1`) since the descent shaft was deleted, so holding
/// right simply stops there.
fn bank_the_vault_coins(app: &mut App) {
    let mouth = ambition_demo_mary_o::pipe_mouth();
    place_player(app, Vec2::new(mouth.center().x, mouth.center().y - 24.0));
    for _ in 0..8 {
        step(app, ControlFrame::default());
    }
    for _ in 0..60 {
        step(app, press_down());
        if player_pos(app).y > ambition_demo_mary_o::vault_bounds().min.y {
            break;
        }
    }
    assert!(
        player_pos(app).y > ambition_demo_mary_o::vault_bounds().min.y,
        "the pipe did not put her in the vault, so she cannot bank its coins",
    );
    for _ in 0..600 {
        step(app, hold_right());
    }
}

/// A room is a place, not a save file: crossing between them must not reset the
/// RUN.
///
/// This is the class of bug the level-1 gate already caught once — a body reset
/// redefined the body (`4e4bd0fd8`), silently, because every test asserted the
/// value an emitter had just written. So each clause here reads state that
/// crossing the boundary would plausibly clobber, on both sides of the crossing.
///
/// ⚠ NOT covered yet: crossing while GROWN. Getting her powered takes a real
/// ?-block bonk (`level_1_acceptance` owns that ladder), and a set-up equip
/// would prove the transition preserves something a player never obtained.
#[test]
fn the_run_survives_the_crossing() {
    let mut app = boot();

    // Bank the vault's coins FIRST — real currency through the shared economy,
    // not a number poked into a resource.
    //
    // ⚠ **this used to be a side effect of getting to 1-2, and it stopped being
    // one.** The route to 1-2 ran along the vault floor to the descent shaft, so
    // walking to the crossing collected the coins on the way. The shaft is gone
    // (2026-08-06) and the route is the goal pole, which is nowhere near the
    // vault — so the walk is its own beat now, and the assertion below is what
    // said so rather than a comment.
    bank_the_vault_coins(&mut app);
    let coins = wallet(&mut app);
    assert!(
        coins > 0,
        "she banked no coins walking the vault, so this proves nothing about \
         carrying them across",
    );

    // ...and only then cross.
    reach_level_1_2(&mut app);
    let (lives, score) = run_state(&mut app);
    assert_eq!(lives, 3, "she should not have spent a life getting here");

    // Settle well past the transition: a clobber that happens one frame later
    // than the commit is still a clobber.
    for _ in 0..120 {
        step(&mut app, ControlFrame::default());
    }

    assert_eq!(
        active_room(&mut app),
        LEVEL_1_2_ROOM_ID,
        "she did not stay in 1-2",
    );
    assert_eq!(wallet(&mut app), coins, "the crossing spent her coins");
    assert_eq!(
        run_state(&mut app),
        (lives, score),
        "the crossing reset her lives or her score",
    );
}

fn wallet(app: &mut App) -> i32 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::actor::BodyWallet, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .expect("the player has a wallet")
        .balance
}

fn run_state(app: &mut App) -> (u8, u32) {
    let mut query = app
        .world_mut()
        .query::<&ambition_demo_mary_o::MaryOLevelState>();
    let state = query
        .iter(app.world())
        .next()
        .expect("the mode owner exists in gameplay");
    (state.lives, state.score)
}
