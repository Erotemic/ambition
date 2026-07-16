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

    // Confirm/skip the card (the same command the Enter/South mapping emits).
    let activation_id = app
        .world()
        .resource::<ActiveShellSequence>()
        .activation_id
        .expect("startup sequence has an activation");
    app.world_mut()
        .write_message(ShellSequenceCommand::Skip { activation_id });
    settle(&mut app);

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
    // The vanity card holds 3600ms (see `compose_ambition_startup_sequence`),
    // i.e. ~216 ticks at 60fps, before auto-advancing; step past that with
    // margin so the natural handoff to the launcher is observed.
    for _ in 0..240 {
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
    app.world_mut()
        .resource_mut::<SyntheticStartupInput>()
        .keyboard_confirm = true;
    app.update();
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE.to_owned()),
    );
}

#[test]
fn controller_acknowledgement_uses_the_neutral_shell_action() {
    use bevy::input::gamepad::Gamepad;

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    install_synthetic_startup_input(&mut app);
    shell_host::compose_ambition_startup_sequence(&mut app);
    settle(&mut app);
    app.world_mut().spawn(Gamepad::default());
    app.world_mut()
        .resource_mut::<SyntheticStartupInput>()
        .controller_confirm = true;
    app.update();
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE.to_owned()),
    );
}
