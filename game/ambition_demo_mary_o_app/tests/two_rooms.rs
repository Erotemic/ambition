//! Verify that the standalone Mary-O demo can complete an engine-owned room
//! transition.
//!
//! The proof requires the authoritative active room and player geometry to
//! change, not merely a transition request. Scripted input is routed through the
//! production participant pipeline so the test behaves the same under package
//! and workspace feature unification.

use ambition_demo_mary_o::level_1_2::LEVEL_1_2_ROOM_ID;
use ambition_demo_mary_o::powerups::{SpentPowerBlocks, STAR_WAND_ID};
use ambition_demo_mary_o::provider::MARY_O_CHARACTER_ID;
use ambition_demo_mary_o::LEVEL_1_1_ROOM_ID;
use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::RoomSet;
use bevy::prelude::*;

fn boot() -> App {
    let mut app = build_demo_app();
    // Use the shared production ordering for scripted participant input.
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    // Settle activation: the provider publishes its world over several frames.
    for _ in 0..90 {
        app.update();
    }
    app
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
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
        &mut ambition_platformer2d::actor::MotionModel,
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
    let world_size = {
        let mut query = app.world_mut().query::<&RoomSet>();
        let world = app.world();
        let set = query.iter(world).next().expect("a RoomSet");
        set.rooms[set.active].world.size
    };
    let inside =
        |pos: Vec2| pos.x >= 0.0 && pos.x <= world_size.x && pos.y >= 0.0 && pos.y <= world_size.y;
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
    // Asking them now would be a check that cannot fail in the worst way: it would pass on an
    // empty `for`.
    //
    //  so it asks the route that actually exists. Finishing a level is what
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
/// So each clause here reads state that crossing the boundary would plausibly clobber, on both
/// sides of the crossing.
#[test]
fn the_run_survives_the_crossing() {
    let mut app = boot();

    // Bank the vault's coins FIRST — real currency through the shared economy,
    // not a number poked into a resource.
    //
    // The shaft is gone and the route is the goal pole, which is nowhere near the vault — so the
    // walk is its own beat now, and the assertion below is what said so rather than a comment.
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

fn run_state(app: &mut App) -> (i8, u32) {
    let mut query = app
        .world_mut()
        .query::<&ambition_demo_mary_o::MaryOLevelState>();
    let state = query
        .iter(app.world())
        .next()
        .expect("the mode owner exists in gameplay");
    (state.lives, state.score)
}

// ── The fourth quantity: the form she is WEARING ────────────────────────────

/// She crosses in the form she is wearing.
///
/// The last of the four the continuity row asks about — *score, coins, lives,
/// and worn power across the transition* — and the only one nothing held.
/// [`the_run_survives_the_crossing`] covers the first three and named this one
/// as its own gap.
///
///  the form is a component ON THE BODY, and that is the whole design of the
/// row. `WornEquipment` is the authority: the exclusive `mary_o_form` slot she
/// wears IS her power state. Her tall sheet, her hurtbox and the HUD are all
/// re-derived from it every frame by `sync_grown_form` and
/// `reconcile_equipment_grants` — so asserting any of THOSE would be asserting
/// the emitter's bookkeeping, and would pass on a body that arrived naked for as
/// long as the deriving system had not yet run. The row is read off the body.
///
///  and she has to have EARNED it. Inserting a `WornEquipment` by hand
/// would prove the transition preserves something no player can obtain. So the
/// wand is knocked out of the level's OWN authored ?-block by a real head
/// contact — `bonk_power_blocks` mints it — and picked up by the engine's own
/// touch-to-collect, which is the only thing here that writes the component.
///
///  what is skipped is the WALK to the block, the same concession the crossing
/// proof above makes: 1-1 is 3328px of platforming and this is not a
/// playthrough. The strike and the pickup are played.
#[test]
fn she_crosses_wearing_the_form_she_earned() {
    let mut app = boot();

    earn_the_star_wand(&mut app);
    assert!(
        wears(&mut app, STAR_WAND_ID),
        "she never ended up wearing the wand she knocked out of the block, so \
         there is no form for the crossing to carry",
    );
    // The tall sheet is the VISIBLE consequence of the row — recorded here so
    // the post-crossing clause can say she is still the same shape, not merely
    // that a row survived in a body drawn small.
    let sheet_before = worn_character(&mut app);
    assert_ne!(
        sheet_before, MARY_O_CHARACTER_ID,
        "she wears the wand but is still drawn as small Mary-O, so the form \
         never actually took and this proves nothing about carrying it",
    );

    reach_level_1_2(&mut app);
    // Settle well past the commit, for the reason the crossing proof above gives:
    // a clobber one frame late is still a clobber.
    for _ in 0..120 {
        step(&mut app, ControlFrame::default());
    }

    assert_eq!(
        active_room(&mut app),
        LEVEL_1_2_ROOM_ID,
        "she did not stay in 1-2",
    );
    assert!(
        wears(&mut app, STAR_WAND_ID),
        "the crossing STRIPPED her form: `WornEquipment` on her body no longer \
         carries `{STAR_WAND_ID}` in 1-2, though she walked into the pole \
         wearing it. She is drawn as '{}' now (she left as '{sheet_before}'). \
         A room is a place, not a save file — and unlike her coins, her lives \
         and her score, the form she is wearing had nothing holding it.",
        worn_character(&mut app),
    );
    assert_eq!(
        worn_character(&mut app),
        sheet_before,
        "she still WEARS the wand after the crossing but is no longer drawn in \
         its form, so the row survived and the body it dresses did not — the \
         sheet is re-derived from the worn set every frame, so this is a \
         reconcile that stopped running, not a stale mirror",
    );
}

/// Knock the wand out of 1-1's own ?-block and put it on, entirely through the
/// shipped systems: the demo's block rule mints the reward, the engine's
/// touch-to-collect equips it, and nothing here writes any worn state.
/// How far above the target a placement starts so the body LANDS on the surface
/// instead of being inserted at a guessed height.
const DROP_HEIGHT_PX: f32 = 100.0;

fn earn_the_star_wand(app: &mut App) {
    let block = first_power_block();
    // `+y` is screen-down, so the block's `max.y` is the face she bonks.
    let underside = block.aabb.max.y;

    // Start above the target and let collision settle the body onto the surface;
    // direct placement at the guessed resting height can leave this fixture inert.
    let start_y = player_body(app).0.y;
    place_player(
        app,
        Vec2::new(block.aabb.center().x, start_y - DROP_HEIGHT_PX),
    );
    for _ in 0..90 {
        step(app, ControlFrame::default());
    }

    //  the variable jump, steered off GEOMETRY rather than a frame count.
    // A bare tap rises ~33px and the underside is 48px above her head, so a tap
    // falls short; a held jump rises ~145px and sails clean over. Hold while her
    // head is still below the underside and release the instant it arrives —
    // the measurement lives in `level_1_acceptance`'s bonk beat, and steering
    // off the geometry is what keeps this true if her jump tuning or the block's
    // row ever moves.
    let mut head_best = f32::MAX;
    let mut struck = false;
    for _ in 0..400 {
        let (pos, size) = player_body(app);
        let head = pos.y - size.y * 0.5;
        head_best = head_best.min(head);
        // Head still BELOW the face (larger y is lower)  keep rising.
        let frame = if head > underside {
            with_jump(ControlFrame::default())
        } else {
            ControlFrame::default()
        };
        step(app, frame);
        if block_is_spent(app, &block.id) {
            struck = true;
            break;
        }
    }
    assert!(
        struck,
        "she never struck 1-1's first ?-block, so no reward was ever minted and \
         there is no form to carry anywhere. Best head height reached: \
         {head_best:.1} (needs <= {underside:.1}, lower number is higher)",
    );

    // The wand rises out of the block and then TRAVELS, so it is chased rather
    // than waited for. Walking her onto it is what collects it: the equip is
    // `collect_world_items`, on overlap, exactly as a player gets it.
    let mut worn = false;
    for _ in 0..400 {
        if let Some(item) = pending_item_pos(app) {
            place_player(app, item);
        }
        step(app, ControlFrame::default());
        if wears(app, STAR_WAND_ID) {
            worn = true;
            break;
        }
    }
    assert!(
        worn,
        "the ?-block spent itself but she never picked up what it popped — \
         either nothing was spawned or touch-to-collect did not claim it",
    );
}

/// 1-1's first authored ?-block.
///
///  found by what the AUTHOR marked it, never by an id reconstructed from a
/// Rust constant — the same rule `power_loop`'s harness follows, and for the
/// same reason: a ?-block is wherever the level says it is.
fn first_power_block() -> ae::world::Block {
    let room = ambition_demo_mary_o::level_1_1();
    room.world
        .blocks
        .iter()
        .find(|block| {
            ambition_demo_mary_o::ldtk_vocabulary::block_look_of(&block.name)
                == Some(ambition_demo_mary_o::ldtk_vocabulary::MaryOBlockLook::Question)
        })
        .expect("1-1 authors a ?-block")
        .clone()
}

fn player_body(app: &mut App) -> (Vec2, Vec2) {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    let kin = query
        .iter(app.world())
        .next()
        .expect("gameplay has a primary player");
    (kin.pos, kin.size)
}

/// Hold jump. `jump_pressed` rides along with `jump_held` because her jump is an
/// EDGE — a frame that only holds continues an ascent, it cannot start one.
fn with_jump(mut frame: ControlFrame) -> ControlFrame {
    frame.jump_pressed = true;
    frame.jump_held = true;
    frame
}

fn block_is_spent(app: &App, id: &ae::GeoId) -> bool {
    app.world()
        .get_resource::<SpentPowerBlocks>()
        .is_some_and(|spent| spent.is_spent(id))
}

/// Where the reward the block popped is right now, if one is still uncollected.
fn pending_item_pos(app: &mut App) -> Option<Vec2> {
    let mut query = app
        .world_mut()
        .query::<&ambition_platformer2d::world_items::WorldItem>();
    query.iter(app.world()).next().map(|item| item.pos)
}

/// The form, read off the BODY. `WornEquipment` is the authority the row's
/// instruction names; everything else about being grown is derived from it.
fn wears(app: &mut App, row_id: &str) -> bool {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::equipment::WornEquipment, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .is_some_and(|worn| worn.wears(row_id))
}

/// Which sheet she is wearing — the derived consequence of the row above.
fn worn_character(app: &mut App) -> String {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::actor::WornCharacter, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .map(|worn| worn.id().to_string())
        .unwrap_or_default()
}
