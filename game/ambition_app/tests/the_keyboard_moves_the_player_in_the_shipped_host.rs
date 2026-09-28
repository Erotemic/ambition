//! **The keyboard moves the player's body in the shipped host.**
//!
//! A player boots the game, presses Enter through the startup cards and the
//! launcher, dismisses the hub's first-boot intro, and walks with the arrow
//! keys. Every step here is a key press on the device road that leafwing
//! reads, in the composition the game binary builds (`build_visible_app` plus
//! the startup sequence `run_visible` adds). Nothing is written into the
//! simulation by the test.
//!
//! ⚠ The other input arms stop at `ControlFrame`. A frame is not a body: a
//! defect between the frame and the body's motion leaves every one of them
//! green while the player cannot move. This file asserts the displacement of
//! the body the seat controls.

use ambition_platformer2d::cutscene::ActiveCutscene;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::input::SeatInputContexts;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;
use leafwing_input_manager::prelude::Buttonlike;

/// Updates for a held key. At the shipped walk speed this is several body
/// widths, so a threshold of one tile cannot be met by drift or a knockback.
const HOLD_FRAMES: usize = 60;
const MIN_TRAVEL_PX: f32 = 16.0;
/// Frames the seat must stay walkable before the hold, so an intro that starts
/// a few frames after arrival is not mistaken for gameplay.
const SETTLED_FRAMES: usize = 90;

fn player_x(app: &mut App) -> Option<f32> {
    let world = app.world_mut();
    let mut bodies = world.query_filtered::<&BodyKinematics, With<PrimaryPlayer>>();
    bodies.iter(world).next().map(|body| body.pos.x)
}

fn gameplay_owns_the_seat(app: &App) -> bool {
    app.world()
        .get_resource::<SeatInputContexts>()
        .is_some_and(|contexts| contexts.primary().gameplay_owned())
}

fn cutscene_playing(app: &App) -> bool {
    app.world()
        .get_resource::<ActiveCutscene>()
        .is_some_and(ActiveCutscene::is_playing)
}

fn ready_to_walk(app: &mut App) -> bool {
    gameplay_owns_the_seat(app) && !cutscene_playing(app) && player_x(app).is_some()
}

fn tap(app: &mut App, key: KeyCode) {
    Buttonlike::press(&key, app.world_mut());
    app.update();
    Buttonlike::release(&key, app.world_mut());
    app.update();
}

/// Presses Enter at a human pace until the seat has been in gameplay, with a
/// body and no cutscene, for [`SETTLED_FRAMES`] frames in a row. Enter confirms
/// a startup card, launches the selected launcher route and dismisses the
/// intro's dialogue beat, so one key covers the road. It is pressed only while
/// something other than gameplay holds the seat.
fn enter_until_walkable(app: &mut App) {
    let mut settled = 0;
    for _ in 0..2400 {
        app.update();
        if ready_to_walk(app) {
            settled += 1;
            if settled >= SETTLED_FRAMES {
                return;
            }
            continue;
        }
        settled = 0;
        tap(app, KeyCode::Enter);
        for _ in 0..6 {
            app.update();
        }
    }
    panic!(
        "Enter never reached a walkable state: gameplay owns the seat = {}, \
         cutscene playing = {}, player body = {:?}",
        gameplay_owns_the_seat(app),
        cutscene_playing(app),
        player_x(app)
    );
}

/// Which GGRS timeline is live. A hold that spans more than one timeline lost
/// its input to the reinstall, which is how this defect showed itself.
fn timeline(app: &App) -> Option<u64> {
    app.world()
        .get_resource::<ambition_platformer2d::engine_core::confirmed_frame::ConfirmedFrameBoundary>()
        .map(|boundary| boundary.session)
}

/// Travel along x while `key` is held, and the timelines at both ends.
fn travel_while_held(app: &mut App, key: KeyCode) -> (f32, Option<u64>, Option<u64>) {
    let before = player_x(app).expect("the seat's body exists before the hold");
    let first_timeline = timeline(app);
    Buttonlike::press(&key, app.world_mut());
    for _ in 0..HOLD_FRAMES {
        app.update();
    }
    Buttonlike::release(&key, app.world_mut());
    for _ in 0..10 {
        app.update();
    }
    let after = player_x(app).expect("the seat's body exists after the hold");
    (after - before, first_timeline, timeline(app))
}

fn assert_arrows_walk_both_ways(app: &mut App, road: &str) {
    let (right, from, to) = travel_while_held(app, KeyCode::ArrowRight);
    assert!(
        right > MIN_TRAVEL_PX,
        "{road}: {HOLD_FRAMES} frames of a held Right arrow moved the player \
         {right:.2} px (needs > {MIN_TRAVEL_PX}); rollback timeline {from:?} -> {to:?}"
    );
    let (left, from, to) = travel_while_held(app, KeyCode::ArrowLeft);
    assert!(
        left < -MIN_TRAVEL_PX,
        "{road}: {HOLD_FRAMES} frames of a held Left arrow moved the player \
         {left:.2} px (needs < -{MIN_TRAVEL_PX}); rollback timeline {from:?} -> {to:?}"
    );
}

/// The road `run_game.sh` takes: shell-hosted, startup cards, launcher, and the
/// hub's first-boot intro, all from the keyboard.
#[test]
fn a_first_boot_from_the_keyboard_ends_with_a_player_who_walks() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    ambition_app::app::shell_host::compose_ambition_startup_sequence(&mut app);
    enter_until_walkable(&mut app);
    assert_arrows_walk_both_ways(&mut app, "shell-hosted first boot");
}

/// The `--direct` road: the host boots straight into the gameplay route.
#[test]
fn a_direct_entry_player_walks_on_the_arrow_keys() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, false);
    enter_until_walkable(&mut app);
    assert_arrows_walk_both_ways(&mut app, "direct entry");
}
