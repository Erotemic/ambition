use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::SandboxAction;
use crate::input::{
    analog_to_dir, ControlFrame, MenuControlFrame, MenuInputState, PlayerDashTriggerState,
};
#[cfg(feature = "input")]
use crate::rendering::PlayerVisual;
use crate::SandboxDevState;

/// Presentation-side companion to `setup_simulation_system`: attach
/// leafwing's `ActionState` and the active preset's `InputMap` to the
/// player entity. Sim-only setup spawns the player without these so the
/// sim path stays leafwing-free per the ADR 0012 input seam.
#[cfg(feature = "input")]
pub(super) fn attach_player_input_components(
    mut commands: Commands,
    dev_state: Res<SandboxDevState>,
    scene: Res<crate::rendering::SceneEntities>,
) {
    let input_map = dev_state.preset().input_map();
    commands
        .entity(scene.player)
        .insert((ActionState::<SandboxAction>::default(), input_map));
}

/// Bridge leafwing's `ActionState` into the sim-side `ControlFrame` resource.
///
/// This is the visible-binary half of the ADR 0012 input seam. The sim
/// reads `Res<ControlFrame>` only — it never queries `ActionState` —
/// which means headless / RL drivers can populate the resource directly
/// without an `InputManagerPlugin` in scope.
///
/// Non-gameplay modes suppress the sim-side `ControlFrame` without
/// mutating leafwing's `ActionState`. Menu systems read their own
/// semantic `MenuControlFrame`, so clearing gameplay here must not
/// make held keyboard/menu buttons look newly pressed every frame.
#[cfg(feature = "input")]
pub fn populate_control_frame_from_actions(
    mode: Res<State<GameMode>>,
    player_input: Query<&ActionState<SandboxAction>, With<PlayerVisual>>,
    mut frame: ResMut<ControlFrame>,
    user_settings: Res<crate::settings::UserSettings>,
    mut dash_state: ResMut<PlayerDashTriggerState>,
    cutscene: Res<crate::cutscene::ActiveCutscene>,
    mut cutscene_request: ResMut<crate::cutscene::CutsceneAdvanceRequest>,
    time: Res<Time>,
) {
    if matches!(mode.get(), GameMode::Dialogue) {
        // Dialogue is a UI state: gameplay input is suppressed, but the
        // underlying `ActionState` must remain intact so a held arrow/D-pad
        // key does not become `just_pressed` again on every frame.
        dash_state.edge = crate::settings::TriggerEdgeState::default();
        *frame = ControlFrame::default();
        return;
    }
    // Cutscene takes precedence over gameplay input. We snapshot
    // interact_pressed into the dismiss request and zero out the
    // gameplay frame so movement / attack can't fire while a beat
    // plays. Holding `Reset` (Backspace/Delete/pad-Select) for
    // `SKIP_HOLD_THRESHOLD_SECS` requests a full cutscene skip so a
    // mistap can't burn through scripted content. Reset is chosen
    // (not Start) so the pause toggle still works during cutscenes
    // and a held button doesn't fight the existing
    // press-to-advance-dialogue mapping on Interact / Jump.
    if cutscene.is_playing() {
        if let Ok(action_state) = player_input.single() {
            let interact = action_state.pressed(&SandboxAction::Interact)
                || action_state.pressed(&SandboxAction::Jump);
            if interact {
                cutscene_request.dismiss_dialogue = true;
            }
            if action_state.pressed(&SandboxAction::Reset) {
                cutscene_request.skip_hold_seconds += time.delta_secs();
                if cutscene_request.skip_hold_seconds >= crate::cutscene::SKIP_HOLD_THRESHOLD_SECS {
                    cutscene_request.skip_cutscene = true;
                    cutscene_request.skip_hold_seconds = 0.0;
                }
            } else {
                cutscene_request.skip_hold_seconds = 0.0;
            }
        }
        *frame = ControlFrame::default();
        return;
    }
    // Outside cutscenes, decay the skip-hold counter so a stale
    // mid-cutscene press can't carry over.
    cutscene_request.skip_hold_seconds = 0.0;
    *frame = match player_input.single() {
        Ok(action_state) => {
            if mode.get().allows_gameplay() {
                let (next_frame, next_state) = ControlFrame::read_gameplay_with_settings(
                    action_state,
                    &user_settings.controls,
                    dash_state.edge,
                );
                dash_state.edge = next_state;
                next_frame
            } else {
                // While paused, suppress gameplay input AND reset the
                // dash trigger state so the post-pause re-press starts
                // from a clean Released edge.
                dash_state.edge = crate::settings::TriggerEdgeState::default();
                ControlFrame::read_menu(action_state)
            }
        }
        Err(_) => ControlFrame::default(),
    };
}

/// Bridge keyboard/gamepad/menu-wheel input into the device-agnostic menu frame.
///
/// Menu systems should read this resource instead of reading raw
/// `ActionState<SandboxAction>`. Touch folds into the same resource from
/// `mobile_input`, so phone menus, desktop keyboard/gamepad, and mouse wheel
/// scrolling share one semantic seam.
#[cfg(feature = "input")]
pub fn populate_menu_control_frame_from_actions(
    time: Res<Time>,
    player_input: Query<&ActionState<SandboxAction>, With<PlayerVisual>>,
    mut menu_frame: ResMut<MenuControlFrame>,
    mut menu_input_state: ResMut<MenuInputState>,
    user_settings: Res<crate::settings::UserSettings>,
    mut mouse_wheel: MessageReader<MouseWheel>,
) {
    let mut next = MenuControlFrame::default();

    if let Ok(actions) = player_input.single() {
        let edge_up = actions.just_pressed(&SandboxAction::MenuNavigateUp);
        let edge_down = actions.just_pressed(&SandboxAction::MenuNavigateDown);
        let edge_left = actions.just_pressed(&SandboxAction::MenuNavigateLeft);
        let edge_right = actions.just_pressed(&SandboxAction::MenuNavigateRight);

        let raw = actions.clamped_axis_pair(&SandboxAction::MenuStick);
        let (sx, sy) = crate::settings::ControlSettings::apply_deadzone(
            raw.x,
            raw.y,
            user_settings.controls.left_stick_deadzone,
        );
        let analog_dir = analog_to_dir(sx, sy, 0.5);

        let input = menu_input_state.step(
            edge_up,
            edge_down,
            edge_left,
            edge_right,
            analog_dir,
            actions.just_pressed(&SandboxAction::MenuSelect),
            actions.just_pressed(&SandboxAction::MenuBack),
            actions.just_pressed(&SandboxAction::Start),
            time.delta_secs(),
            user_settings.controls.menu_repeat_initial_delay,
            user_settings.controls.menu_repeat_interval,
        );
        next = MenuControlFrame::from_menu_input(input);
        next.select_held = actions.pressed(&SandboxAction::MenuSelect)
            || actions.pressed(&SandboxAction::Jump)
            || actions.pressed(&SandboxAction::Interact);
        next.back_held =
            actions.pressed(&SandboxAction::MenuBack) || actions.pressed(&SandboxAction::Reset);
        next.inventory = actions.just_pressed(&SandboxAction::Inventory);
        next.map = actions.just_pressed(&SandboxAction::Map);
    }

    for ev in mouse_wheel.read() {
        next.scroll_y += ev.y;
    }

    *menu_frame = next;
}

/// Cutscene controls are UI/menu intent, not gameplay movement. Keep this
/// small bridge beside the menu frame so touch Confirm/Back can advance or
/// skip cutscenes without teaching the gameplay `ControlFrame` about menu
/// gestures.
#[cfg(feature = "input")]
pub fn apply_menu_frame_to_cutscene_request(
    time: Res<Time>,
    menu_frame: Res<MenuControlFrame>,
    cutscene: Res<crate::cutscene::ActiveCutscene>,
    mut cutscene_request: ResMut<crate::cutscene::CutsceneAdvanceRequest>,
) {
    if !cutscene.is_playing() {
        return;
    }
    if menu_frame.select || menu_frame.select_held {
        cutscene_request.dismiss_dialogue = true;
    }
    if menu_frame.back_held {
        cutscene_request.skip_hold_seconds += time.delta_secs();
        if cutscene_request.skip_hold_seconds >= crate::cutscene::SKIP_HOLD_THRESHOLD_SECS {
            cutscene_request.skip_cutscene = true;
            cutscene_request.skip_hold_seconds = 0.0;
        }
    }
}
