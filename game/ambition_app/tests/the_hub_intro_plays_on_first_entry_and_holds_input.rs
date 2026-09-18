//! **The hub's boot cutscene plays on first entry, and holds the seat's input
//! until it is dismissed.**
//!
//! ⛔⛤ **THIS BEHAVIOUR EXISTED ONLY ON PAPER UNTIL 2026-09-18, AND NOTHING
//! HELD IT AFTERWARDS.** `default_room_cutscene_bindings()` bound `test_intro`
//! to `central_hub_main`, an LDtk LEVEL id. `auto_trigger_room_cutscenes`
//! compares against the RUNTIME room id, and `central_hub_main` and
//! `central_hub_basement` both merge into `central_hub_complex` at LDtk
//! conversion, so the row could never match. `479d5a028` repointed it at the
//! runtime room and added a guard that every binding names a room that exists.
//!
//! ⇒ That guard asks whether the row RESOLVES. It cannot ask whether the
//! cutscene PLAYS, and the difference was not academic: repointing the binding
//! turned four arms red across three files, all with messages sending the
//! reader after the input road — because a playing cutscene declares a
//! CAPTURING `CUTSCENE_CONTEXT` claim and `gameplay_owned()` goes false for as
//! long as it lasts. The arms are right to opt out (`common::
//! the_hub_intro_has_already_played`), but an opt-out used in four places with
//! nothing asserting the behaviour would restore the world in which the
//! binding was still broken, and the next reader would have no way to tell.
//!
//! ⚠ **WHAT THIS FILE IS NOT.** It is a claim about this composition — the
//! shell host with no rollback session — not about a rewinding one. Q136
//! records that `CutsceneAdvanceRequest` is host-produced and sim-consumed, so
//! a dismiss can be lost across a rewind; that is a different arm and it is not
//! measured here. What IS measured is that the dismiss works at all, which is
//! what says the shipped game's first boot is not a hang.

#![cfg(feature = "rl_sim")]

use ambition_app::app::shell_host;
use ambition_platformer2d::cutscene::{ActiveCutscene, CutsceneAdvanceRequest};
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::input::SeatInputContexts;
use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

/// The shipped shell host, driven into gameplay by the route command rather
/// than by a device press — this file is about what happens AFTER arrival, and
/// a synthetic gamepad would add a second thing that can fail.
fn hub_on_arrival(intro_already_seen: bool) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    shell_host::compose_ambition_shell_host(&mut app);
    // A fixed step, so "the timed beats have run out" is a frame count rather
    // than a wall-clock race: `test_intro` is a 1.4 s banner and a 0.8 s fade.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    if intro_already_seen {
        crate::common::the_hub_intro_has_already_played(app.world_mut());
    }
    for _ in 0..8 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
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
///
/// ⚠ THE EDIT THAT MAKES THIS FALSE: bind `test_intro` to a DIFFERENT room
/// that exists — `("cutscene_lab", "test_intro")` — and this arm reports that
/// no cutscene ever started. Verified 2026-09-18.
///
/// ⛔⛤ **AND THE OBVIOUS POISON — RESTORING `central_hub_main` — CANNOT BE
/// BUILT, WHICH THIS ARM'S FIRST DRAFT PRESCRIBED ANYWAY.** `cf3cd7479` moved
/// the binding's room check into `content_validation`, so that edit does not
/// reach a test at all: the content graph refuses to load with *"cutscene
/// binding for 'test_intro' references unknown room 'central_hub_main'"* and
/// the process aborts before any arm runs. That is a stronger guard than this
/// file, and it is the reason the regression this arm was written for is now
/// unreachable by that route — so the prescription is corrected rather than
/// left pointing at an experiment whose red says nothing about this witness.
#[test]
fn the_hub_intro_plays_on_first_entry_and_captures_the_seat() {
    let mut app = hub_on_arrival(false);

    let mut started_at = None;
    for frame in 0..400 {
        app.update();
        if playing(&app) {
            started_at = Some(frame);
            break;
        }
    }
    let started_at = started_at.expect(
        "no cutscene played on first entry to central_hub_complex. The hub's intro is bound in \
         default_room_cutscene_bindings(); a row naming a room that does not exist is caught \
         earlier by content_validation, so the live suspicion here is a row repointed at some \
         OTHER real room, or a trigger that no longer fires on room entry",
    );

    // ⛔ THE CAPTURE IS THE POINT, not the playback. Four arms in three files
    // hung on exactly this and blamed the input road.
    //
    // ⚠ **THERE IS EXACTLY ONE FRAME OF LEAK, AND IT IS MEASURED, NOT ALLOWED
    // FOR.** On the frame the cutscene starts, `gameplay_owned()` is still
    // true: `declare_in_session_input_contexts` runs in
    // `InputSet::ResolveContext`, the cutscene starts later that same frame in
    // the sim schedule, and the claim therefore lands on the next one. So this
    // asserts the capture one frame after the start, and the window is named
    // here so a reader does not discover it as a surprise. Whether one frame of
    // gameplay input at a cutscene boundary matters is filed under `queue.md`'s
    // `CUTSCENE-ROLLBACK-DECISION`, not answered here.
    app.update();
    assert!(
        !gameplay_owns_input(&app),
        "the intro is playing (started at frame {started_at}) and one frame later the primary \
         seat STILL owns gameplay. A cutscene declares a CAPTURING CUTSCENE_CONTEXT claim, so \
         this reading means either the claim is not declared or it is no longer capturing — \
         and every arm that opts out of this cutscene is then opting out of nothing"
    );

    // The banner and the fade are timed; the third beat is a `Dialogue` with no
    // duration, so it is the one still holding the seat here.
    for _ in 0..200 {
        app.update();
    }
    assert!(
        playing(&app),
        "the intro ended on its own within 200 frames of starting. Its third beat is a \
         CutsceneBeat::Dialogue with no duration, which waits for a dismiss; a beat list that \
         runs out by itself makes the dismiss below untestable"
    );

    app.world_mut()
        .resource_mut::<CutsceneAdvanceRequest>()
        .dismiss_dialogue = true;
    let mut ended = false;
    for _ in 0..120 {
        app.update();
        if !playing(&app) {
            ended = true;
            break;
        }
    }
    assert!(
        ended,
        "a dismiss did not end the hub's intro within 120 frames. This is the arm that says \
         the shipped game's first boot is not a hang, because while it plays nothing the \
         player presses reaches gameplay"
    );
    // The lag is SYMMETRIC, and for the same reason as at the start: the
    // cutscene ends in the sim schedule, after `ResolveContext` has already run
    // that frame, so the retraction resolves on the next one.
    app.update();
    assert!(
        gameplay_owns_input(&app),
        "the intro ended and one frame later the primary seat still does not own gameplay, so \
         the capturing claim was not retracted with it"
    );
}

/// **CONTROL: a returning player walks straight into gameplay.**
///
/// ⛔⛤ WITHOUT THIS ARM THE OPT-OUT IS UNWITNESSED. Four arms call
/// `common::the_hub_intro_has_already_played` and then measure gameplay input;
/// if that helper stopped working they would fail with the same misleading
/// "input never reached gameplay" message that sent the last reader down the
/// wrong road. This says the helper does what its name claims, and it doubles
/// as the non-vacuity control for the arm above: the cutscene's absence here
/// is what makes its presence there a fact about first entry.
#[test]
fn the_intro_is_skipped_for_a_player_whose_save_has_seen_it() {
    let mut app = hub_on_arrival(true);
    for _ in 0..400 {
        app.update();
        assert!(
            !playing(&app),
            "the intro played even though the save carries `test_intro_seen`. Either the \
             seen_flag is no longer consulted by start_queued_cutscene, or the flag's id \
             moved and the helper is setting a name nothing reads"
        );
        if gameplay_owns_input(&app) {
            return;
        }
    }
    panic!(
        "a returning player never got the input context within 400 frames, and no cutscene \
         was holding it -- so the capture is not what is blocking gameplay here"
    );
}
