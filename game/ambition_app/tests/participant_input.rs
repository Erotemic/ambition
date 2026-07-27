#![cfg(all(feature = "input", feature = "mobile_touch", feature = "visible"))]

//! Assembled participant-input behavior: the REAL shell-host composition plus
//! the REAL host input stack (leafwing bindings on the persistent
//! participant, the touch virtual device, the shell's semantic consumers),
//! headless, with NO gameplay actor at boot.
//!
//! This is the startup/launcher acceptance for the participant-centered
//! input architecture: keyboard events, gamepad events, and virtual touch all
//! travel device → bindings → participant `ActionState` → semantic menu
//! frame → shell consumers, and the explicit input contexts (startup card →
//! launcher → gameplay) decide routing — never `GameMode`, never actor
//! presence. Source-ownership and context-transition proofs ride the same
//! harness: the participant survives session activation and teardown, held
//! confirmation edges do not retrigger across context switches, and raw
//! screen-frame axes reach the gameplay `ControlFrame` untransformed from
//! every device.
//!
//! Devices are driven through their REAL event paths (`KeyboardInput`
//! messages, `RawGamepadEvent`s): leafwing computes a gamepad button's value
//! from the ANALOG side of the `Gamepad` component and releases any button
//! whose value is ~0, so poking `digital_mut()` alone silently produces
//! nothing — the event path fills both halves exactly like a physical pad.

use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::input::gamepad::{RawGamepadButtonChangedEvent, RawGamepadEvent};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

use ambition::game_shell::{ActiveShellSequence, ShellLauncherState, ShellRouter};
use ambition::input::{
    ActiveInputContext, ControlFrame, InputParticipant, SandboxAction, LAUNCHER_CONTEXT,
    STARTUP_ACKNOWLEDGE_CONTEXT,
};
use ambition::platformer::lifecycle::PlayerVisual;
use ambition::sim_view::ControlPrompt;
use ambition::touch_input::{bevy_plugin::MobileTouchState, TouchControlsPlugin};
use ambition_app::app::shell_host;
use leafwing_input_manager::prelude::{ActionState, Buttonlike};

/// The real shell-host composition + the real host input stack, headless.
/// `with_startup_cards` additionally composes the vanity/startup sequence
/// (the same `compose_ambition_startup_sequence` the visible binary runs).
fn shell_input_app(with_startup_cards: bool) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    // The host input face: leafwing + the participant spawn + the context
    // resolver + the populate chain (self-sufficient headless).
    app.add_plugins(ambition::host::PlatformerHostPlugins);
    // The touch virtual device. Its HUD spawn orders after the app's font
    // load, so register that Startup system exactly as the app does (the
    // Font asset type must exist for it headless; no TextPlugin runs here).
    app.init_asset::<bevy::text::Font>();
    app.add_systems(Startup, ambition::render::ui_fonts::load_ui_fonts);
    app.add_plugins(TouchControlsPlugin);
    shell_host::compose_ambition_shell_host(&mut app);
    if with_startup_cards {
        shell_host::compose_ambition_startup_sequence(&mut app);
    }
    app
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

fn owner(app: &App) -> Option<ambition::input::InputContextId> {
    app.world().resource::<ActiveInputContext>().owner()
}

fn launcher_selected(app: &App) -> usize {
    app.world().resource::<ShellLauncherState>().selected
}

fn tap_key(app: &mut App, key: KeyCode) {
    Buttonlike::press(&key, app.world_mut());
    app.update();
    Buttonlike::release(&key, app.world_mut());
    app.update();
}

/// Drive a gamepad button through the REAL event path (see module docs).
fn pad_set(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
    app.world_mut()
        .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            pad, button, value,
        )));
}

fn pad_tap(app: &mut App, pad: Entity, button: GamepadButton) {
    pad_set(app, pad, button, 1.0);
    app.update();
    pad_set(app, pad, button, 0.0);
    app.update();
}

fn touch_stick(app: &mut App, x: f32, y: f32) {
    let mut state = app.world_mut().resource_mut::<MobileTouchState>();
    state.0.move_x = x;
    state.0.move_y = y;
}

/// Press/release the on-screen touch Jump button through its REAL collect
/// seam: the button entity's `Interaction`, exactly what a finger or mouse
/// press produces (the collect system rebuilds `MobileTouchState` from
/// interactions every frame, so writing the state directly is a no-op).
fn touch_jump(app: &mut App, held: bool) {
    use ambition::touch_input::layout::TouchActionButton;
    let jump = {
        let mut q = app
            .world_mut()
            .query_filtered::<(Entity, &TouchActionButton), With<Button>>();
        q.iter(app.world())
            .find_map(|(entity, action)| {
                matches!(action, TouchActionButton::Jump).then_some(entity)
            })
            .expect("the touch HUD spawned a Jump button")
    };
    let interaction = if held {
        Interaction::Pressed
    } else {
        Interaction::None
    };
    app.world_mut().entity_mut(jump).insert(interaction);
}

fn participant_entity(app: &mut App) -> Entity {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<InputParticipant>>();
    q.single(app.world()).expect("exactly one participant")
}

fn current_segment(app: &App) -> Option<String> {
    let sequence = app.world().resource::<ActiveShellSequence>();
    sequence
        .runtime
        .as_ref()
        .and_then(|runtime| runtime.current())
        .map(|segment| segment.id.as_str().to_owned())
}

#[test]
fn startup_cards_and_launcher_run_on_the_participant_with_no_actor() {
    let mut app = shell_input_app(true);
    settle(&mut app);

    // No gameplay actor exists, yet input is fully alive: the persistent
    // participant owns the device state and the startup card owns the
    // context.
    let mut visuals = app.world_mut().query_filtered::<(), With<PlayerVisual>>();
    assert_eq!(visuals.iter(app.world()).count(), 0, "no actor at boot");
    let participant = participant_entity(&mut app);
    assert!(app
        .world()
        .entity(participant)
        .contains::<ActionState<SandboxAction>>());
    assert_eq!(owner(&app), Some(STARTUP_ACKNOWLEDGE_CONTEXT));
    assert_eq!(
        app.world()
            .resource::<ControlPrompt>()
            .menu_confirm
            .as_deref(),
        Some("Continue"),
        "the startup cue names the acknowledge verb"
    );

    // Tap-anywhere: pressing the card's full-screen surface advances ONE
    // card through the same semantic command keyboard confirm uses.
    let first_segment = current_segment(&app).expect("a card is up");
    let card_surface = {
        let mut q = app
            .world_mut()
            .query_filtered::<(Entity, &Name), With<Button>>();
        q.iter(app.world())
            .find_map(|(entity, name)| {
                (name.as_str() == "basic shell sequence presentation").then_some(entity)
            })
            .expect("the card root is a pressable tap-anywhere surface")
    };
    app.world_mut()
        .entity_mut(card_surface)
        .insert(Interaction::Pressed);
    app.update();
    app.update();
    let second_segment = current_segment(&app).expect("the next card is up");
    assert_ne!(
        first_segment, second_segment,
        "a direct tap advanced the card"
    );

    // Keyboard confirm advances the remaining card; the launcher context
    // takes over and its cue names the focused verb. Enter stays HELD across
    // the transition: the consumed edge must not re-fire as a launch.
    Buttonlike::press(&KeyCode::Enter, app.world_mut());
    app.update();
    settle(&mut app);
    assert!(
        app.world().resource::<ShellLauncherState>().active,
        "confirm dismissed the last card into the launcher"
    );
    assert_eq!(owner(&app), Some(LAUNCHER_CONTEXT));
    assert_eq!(
        app.world()
            .resource::<ControlPrompt>()
            .menu_confirm
            .as_deref(),
        Some("Play"),
        "the launcher cue names the focused row's verb"
    );
    for _ in 0..5 {
        app.update();
        assert!(
            app.world().resource::<ShellRouter>().active.is_some()
                && app.world().resource::<ShellLauncherState>().active,
            "a confirmation held across the card->launcher transition must not launch"
        );
    }
    Buttonlike::release(&KeyCode::Enter, app.world_mut());
    app.update();

    // Keyboard navigation moves the launcher selection...
    let before = launcher_selected(&app);
    tap_key(&mut app, KeyCode::ArrowDown);
    assert_eq!(launcher_selected(&app), before + 1, "ArrowDown moves down");
    tap_key(&mut app, KeyCode::ArrowUp);
    assert_eq!(launcher_selected(&app), before, "ArrowUp moves back up");

    // ...gamepad navigation drives the same selection...
    let pad = app.world_mut().spawn(Gamepad::default()).id();
    app.update();
    pad_tap(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(launcher_selected(&app), before + 1, "D-pad down moves down");
    pad_tap(&mut app, pad, GamepadButton::DPadUp);
    assert_eq!(launcher_selected(&app), before, "D-pad up moves back up");

    // ...and the virtual touch stick drives it too (through the same
    // participant bindings, with the shared analog repeat).
    touch_stick(&mut app, 0.0, 1.0);
    app.update();
    touch_stick(&mut app, 0.0, 0.0);
    app.update();
    assert_eq!(
        launcher_selected(&app),
        before + 1,
        "a touch-stick flick down moves the launcher selection"
    );
    // Return to the first row (proven launchable) so the confirm below is
    // deterministic about WHAT it activates.
    touch_stick(&mut app, 0.0, -1.0);
    app.update();
    touch_stick(&mut app, 0.0, 0.0);
    app.update();
    assert_eq!(launcher_selected(&app), before, "flick up returns");

    // The launcher CAPTURES gameplay actions: a gameplay-only key produces
    // no gameplay input while the launcher owns the participant.
    Buttonlike::press(&KeyCode::KeyX, app.world_mut()); // preset 0 attack
    app.update();
    assert_eq!(
        *app.world().resource::<ControlFrame>(),
        ControlFrame::default(),
        "gameplay input stays neutral under the launcher's capture"
    );
    Buttonlike::release(&KeyCode::KeyX, app.world_mut());
    app.update();

    // A contextual virtual touch confirmation activates the selected route —
    // the same semantic activation keyboard confirm produces.
    touch_jump(&mut app, true);
    app.update();
    touch_jump(&mut app, false);
    let mut launched = false;
    for _ in 0..30 {
        app.update();
        if !app.world().resource::<ShellLauncherState>().active {
            launched = true;
            break;
        }
    }
    assert!(
        launched,
        "the touch confirm button activates the selected route"
    );
}

/// Source ownership + context transitions: the participant survives session
/// activation, actor replacement, and teardown; entering gameplay produces
/// no false press edges; the same participant later feeds gameplay input
/// with raw screen-frame axes from every device.
#[test]
fn the_participant_survives_sessions_and_feeds_gameplay_raw_axes() {
    let mut app = shell_input_app(false);
    settle(&mut app);
    assert!(app.world().resource::<ShellLauncherState>().active);
    let participant = participant_entity(&mut app);

    // Launch the selected route with gamepad South, and KEEP IT HELD across
    // the launcher -> gameplay transition. South is also bound to Jump: the
    // context switch must not manufacture a jump press edge from the held
    // button.
    let pad = app.world_mut().spawn(Gamepad::default()).id();
    app.update();
    pad_set(&mut app, pad, GamepadButton::South, 1.0);
    let mut gameplay_frames = 0;
    for _ in 0..600 {
        app.update();
        assert!(
            !app.world().resource::<ControlFrame>().jump_pressed,
            "a held confirmation must never surface as a gameplay jump PRESS edge"
        );
        if app
            .world()
            .resource::<ActiveInputContext>()
            .gameplay_owned()
        {
            gameplay_frames += 1;
            if gameplay_frames > 5 {
                break;
            }
        }
    }
    assert!(
        gameplay_frames > 5,
        "South activated the route and gameplay took the context"
    );
    pad_set(&mut app, pad, GamepadButton::South, 0.0);
    app.update();

    // Ownership: device state lives on the SAME participant entity — never
    // on the spawned actor/visual entities.
    assert_eq!(
        participant_entity(&mut app),
        participant,
        "session activation did not recreate the participant"
    );
    let mut stray = app.world_mut().query_filtered::<Entity, (
        With<ActionState<SandboxAction>>,
        Or<(
            With<PlayerVisual>,
            With<ambition::actors::actor::PrimaryPlayer>,
        )>,
    )>();
    assert_eq!(
        stray.iter(app.world()).count(),
        0,
        "no ActionState on actor or visual entities"
    );

    // Raw screen-frame axis parity: keyboard right, gamepad d-pad right, and
    // a touch-stick right all reach the gameplay ControlFrame as the SAME
    // raw screen-space axis — nothing rotates or cardinalizes upstream of
    // the body's own frame resolution.
    let axis_after = |app: &mut App| {
        app.update();
        let frame = app.world().resource::<ControlFrame>();
        (frame.axis_x, frame.axis_y)
    };

    Buttonlike::press(&KeyCode::ArrowRight, app.world_mut());
    let keyboard_axis = axis_after(&mut app);
    Buttonlike::release(&KeyCode::ArrowRight, app.world_mut());
    app.update();

    pad_set(&mut app, pad, GamepadButton::DPadRight, 1.0);
    let pad_axis = axis_after(&mut app);
    pad_set(&mut app, pad, GamepadButton::DPadRight, 0.0);
    app.update();

    touch_stick(&mut app, 1.0, 0.0);
    let touch_axis = axis_after(&mut app);
    touch_stick(&mut app, 0.0, 0.0);
    app.update();

    assert!(
        keyboard_axis.0 > 0.9 && keyboard_axis.1.abs() < 1e-3,
        "keyboard right is raw screen-right: {keyboard_axis:?}"
    );
    assert_eq!(
        keyboard_axis, pad_axis,
        "gamepad right produces the identical raw screen axis"
    );
    assert_eq!(
        keyboard_axis, touch_axis,
        "virtual touch right produces the identical raw screen axis"
    );

    // Teardown: quitting to home destroys the session and its actor but the
    // participant persists — same entity, device state intact — and the
    // launcher reclaims the context.
    app.world_mut()
        .write_message(ambition::game_shell::ShellCommand::QuitToHome);
    let mut back_home = false;
    for _ in 0..120 {
        app.update();
        if app.world().resource::<ShellLauncherState>().active {
            back_home = true;
            break;
        }
    }
    assert!(back_home, "QuitToHome returns to the launcher");
    assert_eq!(owner(&app), Some(LAUNCHER_CONTEXT));
    assert_eq!(
        participant_entity(&mut app),
        participant,
        "destroying the controlled actor does not destroy the participant"
    );
    assert!(
        app.world()
            .entity(participant)
            .contains::<ActionState<SandboxAction>>(),
        "participant device state survives session teardown"
    );

    // The SAME participant produces gameplay input again on a fresh session.
    tap_key(&mut app, KeyCode::Enter);
    let mut gameplay_again = false;
    for _ in 0..600 {
        app.update();
        if app
            .world()
            .resource::<ActiveInputContext>()
            .gameplay_owned()
        {
            gameplay_again = true;
            break;
        }
    }
    assert!(gameplay_again, "relaunch reaches gameplay again");
    Buttonlike::press(&KeyCode::ArrowRight, app.world_mut());
    app.update();
    assert!(
        app.world().resource::<ControlFrame>().axis_x > 0.9,
        "the same participant drives the replacement actor"
    );
    Buttonlike::release(&KeyCode::ArrowRight, app.world_mut());
}
