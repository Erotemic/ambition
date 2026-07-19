//! **Startup sequence** — the optional "Powered by Ambition" vanity card that
//! opens the production windowed host and hands off to the launcher.
//!
//! Drives the real composition (`build_visible_app(NoWindow)` + the opt-in
//! startup) and proves: boot lands on the startup route with the sequence
//! running and NO gameplay session; confirming (skip) hands off to exactly one
//! launcher authority, still with no gameplay session; and the same host WITHOUT
//! the startup composition boots straight to the launcher (the direct/test
//! bypass).

use bevy::prelude::*;

use ambition::game_shell::{
    ActiveFrontendAuthority, ActiveGameplaySession, ActiveShellSequence, ShellLauncherState,
    ShellRouter, ShellSequenceCommand,
};
use ambition_app::app::shell_host;
use ambition_app::app::{build_visible_app, VisibleRenderMode};

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

    let controller_confirm = std::mem::take(&mut input.controller_confirm);
    for mut gamepad in &mut gamepads {
        if controller_confirm {
            gamepad
                .digital_mut()
                .press(bevy::input::gamepad::GamepadButton::South);
        } else {
            gamepad
                .digital_mut()
                .release(bevy::input::gamepad::GamepadButton::South);
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
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
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
            .resource::<ambition::audio::selection::ActiveAudioSelection>()
            .owner(),
        Some(ambition::sfx::AudioContextOwner::Frontend(_)),
    ));
    assert!(
        !app.world()
            .resource::<ambition::audio::AudioBackendState>()
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
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
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
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    shell_host::compose_ambition_startup_sequence(&mut app);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
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
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
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

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
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

/// The run-in is TWO vanity cards: the engine card, then the authored comic
/// sequence composed straight from the committed content manifest.
#[test]
fn the_startup_run_in_plays_the_engine_card_then_the_authorship_card() {
    use ambition::game_shell::{image_sequence_total, ShellSegmentPresentation};

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
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
    let ShellSegmentPresentation::ImageSequence { frames, .. } = &segment.presentation else {
        panic!("expected the second card to be an image sequence");
    };
    assert!(
        frames.len() >= 2,
        "an animated card needs more than one frame"
    );
    assert!(
        frames
            .iter()
            .all(|frame| frame.asset_path.starts_with("game://")),
        "frames must address the content crate's own asset source",
    );

    // The card's lifetime is DERIVED from the frame holds, so retiming the
    // animation retimes the card — there is no second number to keep in sync.
    assert_eq!(
        segment.policy.auto_advance_after,
        Some(image_sequence_total(frames)),
        "the segment's duration must be the sum of its frame holds",
    );
}

/// The real authored frame data plays through and terminates.
///
/// Timing lives in the shell's pure sequence logic (unit-tested there); this
/// drives that logic with the ACTUAL content manifest, which is what would catch
/// a manifest that exports zero-length or wrongly-ordered holds.
#[test]
fn the_authored_card_advances_through_its_frames_and_finishes() {
    use ambition::game_shell::{
        ShellSegmentSpec, ShellSequenceFrame, ShellSequenceRuntime, ShellSequenceSpec,
    };
    use std::time::Duration;

    let frames = ambition_content::vanity_card::vanity_card_frames();
    let total: Duration = frames.iter().map(|(_, hold)| *hold).sum();
    let segment = ShellSegmentSpec::image_sequence_timed(
        "startup",
        frames
            .into_iter()
            .map(|(path, hold)| ShellSequenceFrame::new(path, hold)),
        "",
    );
    let mut runtime = ShellSequenceRuntime::new(ShellSequenceSpec {
        segments: vec![segment],
    });

    // Step in small slices and record which frame is showing at each moment.
    let step = Duration::from_millis(25);
    let mut seen = Vec::new();
    let mut elapsed = Duration::ZERO;
    while !runtime.finished {
        if let Some(index) = current_frame_index(&runtime) {
            if seen.last() != Some(&index) {
                seen.push(index);
            }
        }
        runtime.tick(step);
        elapsed += step;
        assert!(
            elapsed < total + Duration::from_secs(2),
            "the card must terminate on its own, near its authored length",
        );
    }

    assert!(
        seen.len() >= 2,
        "the card must actually advance through frames, saw {seen:?}",
    );
    assert!(
        seen.windows(2).all(|pair| pair[0] < pair[1]),
        "frames must advance forward and never wrap, saw {seen:?}",
    );
    // Auto-advance fires once the holds are spent, so the card ends near its
    // authored length rather than running long or cutting the punchline short.
    assert!(
        elapsed >= total && elapsed <= total + Duration::from_millis(200),
        "card ran {elapsed:?}, authored length {total:?}",
    );
}

fn current_frame_index(runtime: &ambition::game_shell::ShellSequenceRuntime) -> Option<usize> {
    use ambition::game_shell::{image_sequence_frame_at, ShellSegmentPresentation};
    let segment = runtime.current()?;
    let ShellSegmentPresentation::ImageSequence { frames, .. } = &segment.presentation else {
        return None;
    };
    Some(image_sequence_frame_at(frames, runtime.elapsed))
}
