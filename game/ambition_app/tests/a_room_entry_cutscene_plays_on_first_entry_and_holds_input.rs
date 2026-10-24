//! **A room's entry cutscene plays on first entry, and holds the seat's input
//! until it is dismissed.**
//!
//! The room is `cutscene_lab`, whose `entry_cutscene` level field names
//! `cutscene_lab_intro`. A playing cutscene declares a CAPTURING
//! `CUTSCENE_CONTEXT` claim, so `gameplay_owned()` is false for as long as it
//! plays. An arm that measures gameplay input in a room with an entry cutscene
//! hangs on that claim, and its failure message points at the input road.
//!
//! ⚠ **WHAT THIS FILE IS NOT.** It is a claim about this composition — the
//! shell host with no rollback session — not about a rewinding one. The dismiss
//! rides the seat's `ControlFrame::confirm_pressed`, and the rewinding arm is
//! `a_cutscene_dismiss_on_the_seats_input_survives_the_rewind_once`. What IS
//! measured here is that a real key press dismisses at all, through the
//! device road, so a room with an entry cutscene is not a hang.

#![cfg(feature = "rl_sim")]

use ambition_app::app::shell_host;
use ambition_platformer2d::cutscene::ActiveCutscene;
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::input::SeatInputContexts;
use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

/// The room under test: its `entry_cutscene` level field in `sandbox.ldtk`
/// names [`SCRIPT`].
const ROOM: &str = "cutscene_lab";
const SCRIPT: &str = "cutscene_lab_intro";
/// The flag [`SCRIPT`] sets and consults (`seen_flag`).
const SEEN_FLAG: &str = "cutscene_lab_intro_seen";

/// The shipped shell host, started in [`ROOM`] and driven into gameplay by the
/// route command rather than by a device press — this file is about what
/// happens AFTER arrival, and a synthetic gamepad would add a second thing that
/// can fail.
fn room_on_arrival(cutscene_already_seen: bool) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    // Before `init_sandbox_resources`, which consumes both. The second makes a
    // renamed room fail here instead of a silent start in the authored room.
    app.insert_resource(ambition_app::app::StartRoomOverride(ROOM.to_string()));
    app.insert_resource(ambition_app::app::StartRoomMustResolve);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    shell_host::compose_ambition_shell_host(&mut app);
    // A fixed step, so "the timed beats have run out" is a frame count rather
    // than a wall-clock race: the script opens on a 1.0 s banner.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    if cutscene_already_seen {
        // The room is not built until the gameplay route is entered below, so
        // the flag is in the save before the cutscene can start.
        app.world_mut()
            .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data_mut()
            .set_flag(SEEN_FLAG.to_string(), true);
    }
    for _ in 0..8 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    app
}

fn playing(app: &App) -> bool {
    app.world().resource::<ActiveCutscene>().is_playing()
}

fn gameplay_owns_input(app: &App) -> bool {
    app.world()
        .resource::<SeatInputContexts>()
        .primary()
        .gameplay_owned()
}

/// ⭐ **THE ROAD: arrive, be held, dismiss, play.**
#[test]
fn a_room_entry_cutscene_plays_on_first_entry_and_captures_the_seat() {
    let mut app = room_on_arrival(false);

    let mut started_at = None;
    for frame in 0..400 {
        app.update();
        if playing(&app) {
            started_at = Some(frame);
            break;
        }
    }
    let started_at = started_at.unwrap_or_else(|| {
        panic!(
            "no cutscene played on first entry to {ROOM}. Its `entry_cutscene` level field in \
             sandbox.ldtk names {SCRIPT}; a script id that does not exist is caught earlier by \
             content_validation, so the live suspicion here is a field moved to some OTHER \
             level, or a trigger that no longer fires on room entry"
        )
    });

    // ⛔ THE CAPTURE IS THE POINT, not the playback.
    //
    // ⚠ **THERE IS EXACTLY ONE FRAME OF LEAK, AND IT IS MEASURED, NOT ALLOWED
    // FOR.** On the frame the cutscene starts, `gameplay_owned()` is still
    // true: `declare_in_session_input_contexts` runs in
    // `InputSet::ResolveContext`, the cutscene starts later that same frame in
    // the sim schedule, and the claim therefore lands on the next one. So this
    // asserts the capture one frame after the start. Whether one frame of
    // gameplay input at a cutscene boundary matters is filed under `queue.md`'s
    // `CUTSCENE-ROLLBACK-DECISION`, not answered here.
    app.update();
    assert!(
        !gameplay_owns_input(&app),
        "{SCRIPT} is playing (started at frame {started_at}) and one frame later the primary \
         seat STILL owns gameplay. A cutscene declares a CAPTURING CUTSCENE_CONTEXT claim, so \
         this reading means either the claim is not declared or it is no longer capturing"
    );

    // The banner is timed; the next beat is a `Dialogue` with no duration, so
    // it is the one still holding the seat here.
    for _ in 0..200 {
        app.update();
    }
    assert!(
        playing(&app),
        "{SCRIPT} ended on its own within 200 frames of starting. Its second beat is a \
         CutsceneBeat::Dialogue with no duration, which waits for a dismiss; a beat list that \
         runs out by itself makes the dismiss below untestable"
    );

    // The real device road: Enter is bound to MenuSelect, which the seat's frame
    // carries as `confirm_pressed` while the cutscene holds the seat. The script
    // has two dialogue beats, so one press per beat, with time between for the
    // timed beat that separates them.
    use leafwing_input_manager::prelude::Buttonlike;
    let mut ended = false;
    for _ in 0..4 {
        Buttonlike::press(&KeyCode::Enter, app.world_mut());
        app.update();
        Buttonlike::release(&KeyCode::Enter, app.world_mut());
        for _ in 0..60 {
            app.update();
            if !playing(&app) {
                ended = true;
                break;
            }
        }
        if ended {
            break;
        }
    }
    assert!(
        ended,
        "four dismisses did not end {SCRIPT}. While a cutscene plays nothing the player \
         presses reaches gameplay, so a dismiss that does not land is a hang"
    );
    // The lag is SYMMETRIC, and for the same reason as at the start: the
    // cutscene ends in the sim schedule, after `ResolveContext` has already run
    // that frame, so the retraction resolves on the next one.
    app.update();
    assert!(
        gameplay_owns_input(&app),
        "{SCRIPT} ended and one frame later the primary seat still does not own gameplay, so \
         the capturing claim was not retracted with it"
    );
}

/// **CONTROL: a save that has seen the cutscene walks straight into gameplay.**
///
/// The non-vacuity control for the arm above: the cutscene's absence here is
/// what makes its presence there a fact about first entry.
#[test]
fn a_room_entry_cutscene_is_skipped_for_a_save_that_has_seen_it() {
    let mut app = room_on_arrival(true);
    for _ in 0..400 {
        app.update();
        assert!(
            !playing(&app),
            "{SCRIPT} played even though the save carries `{SEEN_FLAG}`. Either the seen_flag \
             is no longer consulted by start_queued_cutscene, or the flag's id moved"
        );
        if gameplay_owns_input(&app) {
            return;
        }
    }
    panic!(
        "the player never got the input context within 400 frames in {ROOM}, and no cutscene \
         was holding it -- so the capture is not what is blocking gameplay here"
    );
}
