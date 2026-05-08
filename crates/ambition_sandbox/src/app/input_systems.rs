use bevy::prelude::*;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::SandboxAction;
use crate::input::{ControlFrame, PlayerDashTriggerState};
#[cfg(feature = "input")]
use crate::rendering::PlayerVisual;
use crate::SandboxRuntime;

/// Presentation-side companion to `setup_simulation_system`: attach
/// leafwing's `ActionState` and the active preset's `InputMap` to the
/// player entity. Sim-only setup spawns the player without these so the
/// sim path stays leafwing-free per the ADR 0012 input seam.
#[cfg(feature = "input")]
pub(super) fn attach_player_input_components(
    mut commands: Commands,
    runtime: Res<SandboxRuntime>,
    scene: Res<crate::rendering::SceneEntities>,
) {
    let input_map = runtime.preset().input_map();
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
/// Dialogue mode also resets leafwing's pressed/just-pressed edges so
/// action edges from the moment dialogue opened don't leak into the
/// next gameplay frame.
#[cfg(feature = "input")]
pub fn populate_control_frame_from_actions(
    mode: Res<State<GameMode>>,
    mut player_input: Query<&mut ActionState<SandboxAction>, With<PlayerVisual>>,
    mut frame: ResMut<ControlFrame>,
    user_settings: Res<crate::settings::UserSettings>,
    mut dash_state: ResMut<PlayerDashTriggerState>,
    cutscene: Res<crate::cutscene::ActiveCutscene>,
    mut cutscene_request: ResMut<crate::cutscene::CutsceneAdvanceRequest>,
    time: Res<Time>,
) {
    if matches!(mode.get(), GameMode::Dialogue) {
        if let Ok(mut action_state) = player_input.single_mut() {
            action_state.reset_all();
        }
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
