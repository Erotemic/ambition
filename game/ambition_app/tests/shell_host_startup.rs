//! Startup sequence — the optional "Powered by Ambition" vanity card that
//! opens the production windowed host and hands off to the launcher.
//!
//! Drives the real composition (`build_visible_app(NoWindow)` + the opt-in
//! startup) and proves: boot lands on the startup route with the sequence
//! running and NO gameplay session; confirming (skip) hands off to exactly one
//! launcher authority, still with no gameplay session; and the same host WITHOUT
//! the startup composition boots straight to the launcher (the direct/test
//! bypass).

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition_app::app::shell_host;
use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::game_shell::{
    ActiveFrontendAuthority, ActiveGameplaySession, ActiveShellSequence, ShellLauncherState,
    ShellRouter, ShellSequenceCommand,
};

/// The real startup composition, stepped on a PINNED timestep.
///
/// Startup cards advance on a TIMELINE, so these tests are time-sensitive by nature. Under
/// Bevy's default `TimeUpdateStrategy::Automatic`, `app.update()` advances the clock by real
/// elapsed wall-clock — meaning how much of that timeline a `settle()` covers depends on how
/// busy the machine is. Pinning the step makes every test here mean the same thing on any
/// machine.
fn startup_app() -> App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn active_route(app: &App) -> Option<String> {
    app.world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .map(|active| active.route_id.as_str().to_owned())
}

fn no_gameplay_session(app: &App) -> bool {
    app.world().resource::<ActiveGameplaySession>().0.is_none()
}

fn launcher_active(app: &App) -> bool {
    app.world().resource::<ShellLauncherState>().active
}

/// Confirm through every remaining vanity card. Confirm skips ONE card, so the
/// number of presses tracks the number of segments the host composed.
fn skip_remaining_cards(app: &mut App) -> usize {
    let mut skipped = 0;
    while let Some(activation_id) = app.world().resource::<ActiveShellSequence>().activation_id {
        app.world_mut()
            .write_message(ShellSequenceCommand::Skip { activation_id });
        settle(app);
        skipped += 1;
        assert!(skipped < 16, "startup sequence did not terminate");
    }
    skipped
}

#[derive(Resource, Default)]
struct SyntheticStartupInput {
    keyboard_confirm: bool,
    controller_confirm: bool,
}

fn drive_synthetic_startup_input(
    mut input: ResMut<SyntheticStartupInput>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut gamepads: Query<&mut bevy::input::gamepad::Gamepad>,
) {
    if std::mem::take(&mut input.keyboard_confirm) {
        keys.press(KeyCode::Enter);
    } else {
        keys.release(KeyCode::Enter);
    }

    // Set BOTH halves of the gamepad state, exactly as bevy's event
    // processing does for a physical pad. Leafwing computes a button's value
    // from the ANALOG side and releases any button whose value is ~0, so a
    // digital-only press is silently dead at the ActionState (see
    // dev/journals/lessons_learned.md, ).
    let controller_confirm = std::mem::take(&mut input.controller_confirm);
    for mut gamepad in &mut gamepads {
        let south = bevy::input::gamepad::GamepadButton::South;
        if controller_confirm {
            gamepad.digital_mut().press(south);
            gamepad.analog_mut().set(south, 1.0);
        } else {
            gamepad.digital_mut().release(south);
            gamepad.analog_mut().set(south, 0.0);
        }
    }
}

/// Hold confirm through the whole run-in, one press per card, and report how
/// many presses it took. Each press advances exactly one card, so this also
/// proves the neutral action is what drives the sequence forward.
fn confirm_until_launcher(app: &mut App, controller: bool) -> usize {
    let mut presses = 0;
    while !launcher_active(app) {
        {
            let mut input = app.world_mut().resource_mut::<SyntheticStartupInput>();
            if controller {
                input.controller_confirm = true;
            } else {
                input.keyboard_confirm = true;
            }
        }
        app.update();
        settle(app);
        presses += 1;
        assert!(presses < 16, "confirm never reached the launcher");
    }
    presses
}

fn install_synthetic_startup_input(app: &mut App) {
    app.init_resource::<SyntheticStartupInput>();
    app.add_systems(
        PreUpdate,
        drive_synthetic_startup_input.after(bevy::input::InputSystems),
    );
}

#[test]
fn startup_card_plays_then_hands_off_to_the_launcher() {
    let mut app = startup_app();
    shell_host::compose_ambition_startup_sequence(&mut app);
    settle(&mut app);

    // Boot lands on the startup card, not the launcher.
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_STARTUP_ROUTE.to_owned()),
        "boot opens on the startup route"
    );
    assert!(
        app.world()
            .resource::<ActiveShellSequence>()
            .runtime
            .is_some(),
        "the startup vanity sequence is running"
    );
    assert!(
        no_gameplay_session(&app),
        "no gameplay session exists during startup"
    );
    assert!(
        !launcher_active(&app),
        "the launcher is not yet the active frontend during startup"
    );
    assert_eq!(
        app.world()
            .resource::<ActiveFrontendAuthority>()
            .0
            .as_ref()
            .map(|active| active.route_id.as_str()),
        Some(shell_host::AMBITION_STARTUP_ROUTE),
        "startup owns the exact frontend authority",
    );
    assert!(matches!(
        app.world()
            .resource::<ambition_platformer2d::audio::selection::ActiveAudioSelection>()
            .owner(),
        Some(ambition_platformer2d::sfx::AudioContextOwner::Frontend(_)),
    ));
    assert!(
        !app.world()
            .resource::<ambition_platformer2d::audio::AudioBackendState>()
            .device_backend_installed,
        "no-window startup acceptance never opens the audio device",
    );

    // Confirm/skip each card (the same command the Enter/South mapping emits).
    // Confirm advances ONE card, so a multi-card run-in needs one per card.
    skip_remaining_cards(&mut app);

    // Handoff: exactly one launcher authority, still no gameplay session.
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE.to_owned()),
        "completing the startup card routes to the launcher"
    );
    assert!(
        launcher_active(&app),
        "the launcher owns the frontend after startup"
    );
    assert!(
        no_gameplay_session(&app),
        "the handoff introduces no gameplay session"
    );
    assert!(
        app.world()
            .resource::<ActiveShellSequence>()
            .runtime
            .is_none(),
        "the startup sequence is cleaned up after completion"
    );
}

#[test]
fn without_the_startup_composition_boot_bypasses_straight_to_the_launcher() {
    // Direct-entry / test bypass: the host without the opt-in startup opens on
    // the launcher immediately — startup is a host presentation CHOICE.
    let mut app = startup_app();
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE.to_owned()),
        "the plain host boots straight to the launcher"
    );
    assert!(no_gameplay_session(&app));
}

#[test]
fn startup_naturally_auto_advances_on_the_shipping_timeline() {
    let mut app = startup_app();
    shell_host::compose_ambition_startup_sequence(&mut app);
    // Step until the run-in hands off on its own. Deliberately NOT a tick count
    // derived from the current card timings: cards get retimed and added, and
    // the invariant under test is "startup reaches the launcher with no input",
    // not "startup takes N frames". The cap is a hang guard (60s of ticks).
    for _ in 0..3600 {
        if launcher_active(&app) {
            break;
        }
        app.update();
    }
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE.to_owned()),
    );
    assert!(launcher_active(&app));
    assert!(no_gameplay_session(&app));
}

#[test]
fn keyboard_acknowledgement_uses_the_neutral_shell_action() {
    let mut app = startup_app();
    install_synthetic_startup_input(&mut app);
    shell_host::compose_ambition_startup_sequence(&mut app);
    settle(&mut app);
    let presses = confirm_until_launcher(&mut app, false);
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE.to_owned()),
    );
    // One press per composed card — confirm skips a card, not the whole run-in.
    assert_eq!(presses, 2, "engine card then authorship card");
}

#[test]
fn controller_acknowledgement_uses_the_neutral_shell_action() {
    use bevy::input::gamepad::Gamepad;

    let mut app = startup_app();
    install_synthetic_startup_input(&mut app);
    shell_host::compose_ambition_startup_sequence(&mut app);
    settle(&mut app);
    app.world_mut().spawn(Gamepad::default());
    confirm_until_launcher(&mut app, true);
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE.to_owned()),
    );
}

/// The run-in is TWO vanity cards: the engine card, then the authorship card —
/// which is now DRAWN by the content crate rather than played back from
/// rendered frames, so the card exists in a checkout that never fetched the
/// git-ignored payload.
#[test]
fn the_startup_run_in_plays_the_engine_card_then_the_authorship_card() {
    use ambition_platformer2d::game_shell::ShellSegmentPresentation;

    let mut app = startup_app();
    shell_host::compose_ambition_startup_sequence(&mut app);
    settle(&mut app);

    let sequence = app.world().resource::<ActiveShellSequence>();
    let segments = &sequence
        .runtime
        .as_ref()
        .expect("the startup sequence is running")
        .spec
        .segments;
    assert_eq!(segments.len(), 2, "engine card then authorship card");

    // The engine card comes FIRST — built-with before built-by.
    assert!(
        matches!(
            &segments[0].presentation,
            ShellSegmentPresentation::TextCard { title, .. } if title.contains("Ambition")
        ),
        "the first card credits the engine, got {:?}",
        segments[0].presentation,
    );

    let segment = &segments[1];
    let ShellSegmentPresentation::Registered(kind) = &segment.presentation else {
        panic!(
            "expected the authorship card to be a REGISTERED segment drawn by the \
             content crate, got {:?}",
            segment.presentation,
        );
    };
    assert_eq!(
        kind.as_str(),
        ambition_content::presentation::vanity_card_made_this_meme::MADE_THIS_MEME_CARD_SEGMENT_KIND,
        "the host must name the kind the content crate actually draws — a kind \
         nothing registers spawns no scene and the run-in shows a blank card",
    );

    // The card's lifetime is DERIVED from the baked animation's own frame count
    // and rate, so re-timing it in the exporter re-times the segment — there is
    // no second number to keep in sync.
    assert_eq!(
        segment.policy.auto_advance_after,
        Some(ambition_content::presentation::vanity_card_made_this_meme::made_this_meme_card_duration()),
        "the segment's duration must be the baked card's own length",
    );
}

/// The title music plays THROUGH the handoff instead of restarting on it.
///
/// I want the music to play uninterrupted in the title sequence."*
///
/// The track NAME cannot answer this — it is the same either way — so the
/// question is asked of `play_generation`, which counts how many times the base
/// channel has been started. Same generation across the handoff means the same
/// uninterrupted play, which is precisely what "uninterrupted" means.
#[test]
fn the_title_music_survives_the_handoff_from_the_cards_to_the_launcher() {
    use ambition_platformer2d::audio::library::MusicPlaybackState;

    let mut app = startup_app();
    shell_host::compose_ambition_startup_sequence(&mut app);
    settle(&mut app);

    // vacuity check FIRST. If the frontend never selected a track, everything
    // below passes over an empty world and proves nothing.
    let playback = app.world().resource::<MusicPlaybackState>();
    let playing = playback.active_track().to_string();
    let generation = playback.play_generation();
    assert!(
        !playing.is_empty(),
        "the startup cards must have selected the title track for this to mean anything",
    );

    skip_remaining_cards(&mut app);
    assert!(launcher_active(&app), "the run-in reached the launcher");

    let playback = app.world().resource::<MusicPlaybackState>();
    assert_eq!(
        playback.active_track(),
        playing,
        "the launcher plays the same title track the cards did",
    );
    assert_eq!(
        playback.play_generation(),
        generation,
        "the title music was stopped and started again across the handoff",
    );
}

/// The card the run-in schedules is the one the content crate DRAWS, and it
/// terminates on its own.
///
/// What still matters — the segment auto-advances and hands off — is asserted by
/// `startup_card_plays_then_hands_off_to_the_launcher` and
/// `startup_naturally_auto_advances_on_the_shipping_timeline` above, against the real host.
#[test]
fn the_drawn_card_declares_a_length_the_run_in_can_schedule() {
    let total =
        ambition_content::presentation::vanity_card_made_this_meme::made_this_meme_card_duration();
    let run_in = shell_host::ambition_startup_duration();
    assert!(
        run_in > total,
        "the run-in ({run_in:?}) must budget for the card ({total:?}) plus the engine card",
    );
}

/// F1 changes the debug mode on the LAUNCHER, where there is no session.
///
/// ⛔⛔ IT DID NOT, AND THE FAILURE WAS SILENT. `handle_debug_hotkeys` reads only
/// `DeveloperRuntimeState` and `DeveloperTools` — two HOST resources — but it
/// was installed inside a `.chain()` gated by `session_world_exists`, beside
/// three systems that genuinely do need a session (LDtk reload, the trace
/// hotkey, the map menu). So pressing F1 on the launcher, the title screen or a
/// loading screen changed nothing at all, and the debug mode a developer thought
/// they had switched on before entering a game was never on.
///
/// ⭐ THE PRODUCER WAS NEVER GATED. `emit_developer_actions` runs in `PreUpdate`
/// unconditionally, so the message was always being written and only the
/// consumer refused to hear it — which is why nothing logged, nothing warned,
/// and the key simply did nothing.
///
/// ⭐ THE PREMISE IS ASSERTED FIRST: with a session present this would pass even
/// against the gated version, so the test states that there is no session before
/// it claims anything about the toggle.
#[test]
fn f1_toggles_the_debug_mode_with_no_gameplay_session() {
    use ambition_platformer2d::dev_tools::DeveloperRuntimeState;
    use ambition_platformer2d::platformer::developer_hotkeys::DeveloperAction;

    // ⭐ THE STARTUP ROUTE, NOT THE DIRECT BYPASS. The first draft of this used
    // `build_visible_app(.., false)` and its own premise guard caught that that
    // boot DOES carry a gameplay session — so the assertion below would have
    // been about a state this test is not about, and would have passed against
    // the very gate it exists to pin. This module's opening paragraph already
    // says the startup route is the one with no session.
    let mut app = startup_app();
    settle(&mut app);

    assert!(
        no_gameplay_session(&app),
        "premise: this test is about the state where NO session exists, and a \
         session here would make it pass against the defect it pins"
    );
    let before = app.world().resource::<DeveloperRuntimeState>().debug;

    app.world_mut()
        .write_message(DeveloperAction::ToggleDebugOverlay);
    app.update();

    assert_eq!(
        app.world().resource::<DeveloperRuntimeState>().debug,
        !before,
        "F1 must reach the host debug state without a gameplay session; it is a \
         HOST diagnostic toggle, not a session one"
    );

    // And back, so the test pins a TOGGLE rather than a one-way latch.
    app.world_mut()
        .write_message(DeveloperAction::ToggleDebugOverlay);
    app.update();
    assert_eq!(
        app.world().resource::<DeveloperRuntimeState>().debug,
        before,
        "the second press must undo the first"
    );
}
