//! Participant → frame populate systems: the schedule-anchored input vocabulary.
//!
//! Bridges the persistent participant's leafwing `ActionState<SandboxAction>`
//! into the sim-side `ControlFrame` ([`populate_control_frame_from_actions`])
//! and the menu-side [`MenuControlFrame`]
//! ([`populate_menu_control_frame_from_actions`]), the device-agnostic seam
//! the sim/menu read instead of raw devices (ADR 0012). Also:
//! [`MenuNavConsume`] (the set menu-nav consumers join so touch/joystick
//! writers can pin `.before` it), cutscene advance/skip routing,
//! [`spawn_primary_input_participant`] (the boot-time participant spawn), and
//! [`declare_gameplay_input_context`] (the session lifecycle's context
//! claim). All gated behind the `input` feature except the context claim.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

use ambition_input::participant::{context_priority, ContextClaim};
use ambition_input::{
    analog_to_dir, ActiveInputContext, ControlFrame, InputParticipant, KeyboardPreset,
    MenuControlFrame, MenuInputState, ParticipantContexts, PlayerDashTriggerState,
    GAMEPLAY_CONTEXT,
};
#[cfg(feature = "input")]
use ambition_input::{
    read_gameplay_control_frame_with_settings, read_menu_control_frame, SandboxAction,
};
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionGatedSimulation, SessionRoot,
};
use ambition_platformer_primitives::schedule::GameMode;

/// Item 3 (optional guard): whether input should be SUPPRESSED this frame because
/// the "Pause input when window unfocused" setting is ON and the OS window is not
/// focused. Default OFF, so this returns `false` and nothing changes unless the
/// player opts in. When ON, it returns `true` while no window is focused, and the
/// input population systems clear their frames (same shape as the existing
/// pause/dialogue/cutscene suppression). Reading `Window.focused` keeps the gate
/// minimal — it never touches the leafwing `ActionState`, so the input abstraction
/// is untouched; only the device-agnostic frames are zeroed.
#[cfg(feature = "input")]
fn input_suppressed_by_unfocus(
    settings: &ambition_persistence::settings::UserSettings,
    window_focus: impl IntoIterator<Item = bool>,
) -> bool {
    if !settings.gameplay.pause_input_when_unfocused {
        return false;
    }
    // Suppress when NO window reports focus. A missing window (headless / between
    // frames) is treated as "not focused" only when the guard is enabled, which is
    // the safe direction for this opt-in.
    !window_focus.into_iter().any(|focused| focused)
}

/// The menu-nav CONSUMERS of [`MenuControlFrame`].
///
/// Both inventory backends' directional nav — the bevy_ui Grid
/// (`grid_menu_nav`) and the 3D cube (`kaleidoscope_focus_nav`) — join
/// this set so any writer that must land in the frame BEFORE it is
/// consumed (notably the touch-joystick fold in the mobile_input plugin)
/// can pin `.before(MenuNavConsume)` without naming each backend's
/// private system. Without that ordering the touch stick reached the
/// frame only after the consumers had already read (and reset) it, so
/// the on-screen joystick never moved either menu's cursor (Bug 2).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MenuNavConsume;

/// Spawn the persistent primary input participant at boot.
///
/// The participant is the person in front of the controller: it owns the
/// leafwing `ActionState`/`InputMap` and the declared input contexts, exists
/// before any gameplay session (startup cards, launcher), and survives every
/// session teardown/relaunch — device state is never attached to actors or
/// presentation entities. Idempotent by the `With<InputParticipant>` guard.
#[cfg(feature = "input")]
pub fn spawn_primary_input_participant(
    mut commands: Commands,
    // The persisted setting is the ONE preset authority (`Option` so headless
    // fixtures without a settings resource fall back to preset 0).
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    existing: Query<(), With<InputParticipant>>,
) {
    if !existing.is_empty() {
        return;
    }
    let preset = KeyboardPreset::by_index(settings.map_or(0, |s| s.controls.keyboard_preset_index));
    commands.spawn((
        InputParticipant::primary(),
        ParticipantContexts::default(),
        ActionState::<SandboxAction>::default(),
        preset.input_map(),
    ));
}

/// The session lifecycle's context claim: a live gameplay session owns the
/// participant's actions.
///
/// Mirrors `session_world_exists` (the canonical [`SessionRoot`] must exist
/// and, on shell-gated hosts, match the active scope). The SESSION is the
/// surface that owns gameplay input, so the claim follows the session —
/// never `GameMode`, never controlled-body presence.
pub fn declare_gameplay_input_context(
    gate: Option<Res<SessionGatedSimulation>>,
    active_scope: Option<Res<ActiveSessionScope>>,
    roots: Query<&SessionRoot>,
    mut participants: Query<&mut ParticipantContexts, With<InputParticipant>>,
) {
    let session_live = roots.single().is_ok_and(|root| {
        gate.is_none()
            || active_scope
                .as_deref()
                .and_then(ActiveSessionScope::current)
                == Some(root.0)
    });
    for mut contexts in &mut participants {
        // Touch the component only when the claim actually moves.
        if contexts.is_declared(GAMEPLAY_CONTEXT) != session_live {
            contexts.sync(
                ContextClaim::capturing(GAMEPLAY_CONTEXT, context_priority::GAMEPLAY),
                session_live,
            );
        }
    }
}

/// Toggle player-trail emission from the logical input action.
///
/// The physical key or button belongs to `KeyboardPreset::input_map`; this bridge
/// only consumes the semantic `SandboxAction` and flips the simulation resource
/// that the trail system reads.
#[cfg(feature = "input")]
pub fn toggle_player_trail_emission_from_actions(
    mode: Res<State<GameMode>>,
    active_context: Res<ActiveInputContext>,
    player_input: Query<&ActionState<SandboxAction>, With<InputParticipant>>,
    enabled: Option<ResMut<crate::avatar::trail::PlayerTrailEnabled>>,
) {
    // The participant exists at the launcher too; only a session that owns
    // input (and is actually in a gameplay mode) may consume the toggle.
    if !active_context.gameplay_owned() || !mode.get().allows_gameplay() {
        return;
    }
    let Some(mut enabled) = enabled else {
        return;
    };
    let Ok(actions) = player_input.single() else {
        return;
    };
    if actions.just_pressed(&SandboxAction::TrailToggle) {
        enabled.enabled = !enabled.enabled;
    }
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
    active_context: Res<ActiveInputContext>,
    player_input: Query<&ActionState<SandboxAction>, With<InputParticipant>>,
    mut frame: ResMut<ControlFrame>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    mut dash_state: ResMut<PlayerDashTriggerState>,
    cutscene: Res<ambition_cutscene::ActiveCutscene>,
    mut cutscene_request: ResMut<ambition_cutscene::CutsceneAdvanceRequest>,
    world_time: Option<Res<ambition_time::WorldTime>>,
    windows: Query<&Window>,
) {
    let wall_dt = world_time.as_deref().map_or(0.0, |time| time.wall_dt());

    // The participant persists across the whole app lifetime, so "no player
    // spawned yet" no longer implies "no ActionState". The resolved input
    // context is the gate: while the launcher/startup (or nothing) owns the
    // participant's actions, gameplay input stays neutral. In-session UI
    // states (pause/dialogue/cutscene) keep their own suppressions below —
    // the session still owns input there.
    if !active_context.gameplay_owned() {
        dash_state.edge = crate::persistence::settings::TriggerEdgeState::default();
        *frame = ControlFrame::default();
        return;
    }

    // Optional unfocus guard: clear gameplay input while the window is unfocused
    // (and the setting is on). Reset the dash edge too so the post-refocus re-press
    // starts clean, mirroring the pause path.
    if input_suppressed_by_unfocus(&user_settings, windows.iter().map(|w| w.focused)) {
        dash_state.edge = crate::persistence::settings::TriggerEdgeState::default();
        *frame = ControlFrame::default();
        return;
    }
    if matches!(mode.get(), GameMode::Dialogue) {
        // Dialogue is a UI state: gameplay input is suppressed, but the
        // underlying `ActionState` must remain intact so a held arrow/D-pad
        // key does not become `just_pressed` again on every frame.
        dash_state.edge = crate::persistence::settings::TriggerEdgeState::default();
        *frame = ControlFrame::default();
        return;
    }
    // Cutscene takes precedence over gameplay input. We snapshot
    // interact_pressed into the dismiss request and zero out the
    // gameplay frame so movement / attack can't fire while a beat
    // plays. Holding `Reset` (Delete/pad-Select) for
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
                cutscene_request.skip_hold_seconds += wall_dt;
                if cutscene_request.skip_hold_seconds >= ambition_cutscene::SKIP_HOLD_THRESHOLD_SECS
                {
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
    let mut player_inputs = player_input.iter();
    let action_state = player_inputs.next();
    if player_inputs.next().is_some() {
        // Two input-bearing participants are never a benign transition: they
        // would compete to author the single simulation ControlFrame. (Real
        // multi-participant support keys frames by ParticipantId → slot.)
        bevy::log::warn_once!(
            "populate_control_frame_from_actions: multiple participant ActionState \
             components are active; gameplay input is NEUTRAL until exact participant \
             ownership is restored."
        );
        *frame = ControlFrame::default();
        return;
    }
    *frame = match action_state {
        Some(action_state) => {
            if mode.get().allows_gameplay() {
                let (next_frame, next_state) = read_gameplay_control_frame_with_settings(
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
                dash_state.edge = crate::persistence::settings::TriggerEdgeState::default();
                read_menu_control_frame(action_state)
            }
        }
        // No participant exists only in minimal fixtures that never ran the
        // boot spawn. Neutral input is the contract there, not a warning.
        None => ControlFrame::default(),
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
    world_time: Option<Res<ambition_time::WorldTime>>,
    player_input: Query<&ActionState<SandboxAction>, With<InputParticipant>>,
    mut menu_frame: ResMut<MenuControlFrame>,
    mut menu_input_state: ResMut<MenuInputState>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    windows: Query<&Window>,
) {
    let wall_dt = world_time.as_deref().map_or(0.0, |time| time.wall_dt());
    let mut next = MenuControlFrame::default();

    // Optional unfocus guard: leave the menu frame cleared while the window is
    // unfocused (and the setting is on). Drain the wheel so a buffered scroll
    // doesn't fire on refocus.
    if input_suppressed_by_unfocus(&user_settings, windows.iter().map(|w| w.focused)) {
        mouse_wheel.clear();
        *menu_frame = next;
        return;
    }

    if let Ok(actions) = player_input.single() {
        let edge_up = actions.just_pressed(&SandboxAction::MenuNavigateUp);
        let edge_down = actions.just_pressed(&SandboxAction::MenuNavigateDown);
        let edge_left = actions.just_pressed(&SandboxAction::MenuNavigateLeft);
        let edge_right = actions.just_pressed(&SandboxAction::MenuNavigateRight);

        let raw = actions.clamped_axis_pair(&SandboxAction::MenuStick);
        let (sx, sy) = ambition_persistence::settings::ControlSettings::apply_deadzone(
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
            wall_dt,
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
        // Paged-menu page-turn bumpers (Fix 2): just-pressed edge so one bumper tap
        // turns exactly one page, independent of the arrow/d-pad item cursor.
        next.page_left = actions.just_pressed(&SandboxAction::MenuPageLeft);
        next.page_right = actions.just_pressed(&SandboxAction::MenuPageRight);
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
    world_time: Option<Res<ambition_time::WorldTime>>,
    menu_frame: Res<MenuControlFrame>,
    cutscene: Res<ambition_cutscene::ActiveCutscene>,
    mut cutscene_request: ResMut<ambition_cutscene::CutsceneAdvanceRequest>,
) {
    let wall_dt = world_time.as_deref().map_or(0.0, |time| time.wall_dt());
    if !cutscene.is_playing() {
        return;
    }
    if menu_frame.select || menu_frame.select_held {
        cutscene_request.dismiss_dialogue = true;
    }
    if menu_frame.back_held {
        cutscene_request.skip_hold_seconds += wall_dt;
        if cutscene_request.skip_hold_seconds >= ambition_cutscene::SKIP_HOLD_THRESHOLD_SECS {
            cutscene_request.skip_cutscene = true;
            cutscene_request.skip_hold_seconds = 0.0;
        }
    }
}

#[cfg(all(test, feature = "input"))]
mod focus_gate_tests {
    use super::{
        declare_gameplay_input_context, input_suppressed_by_unfocus,
        spawn_primary_input_participant,
    };
    use ambition_input::{
        resolve_active_input_context, ActiveInputContext, InputParticipant, SandboxAction,
    };
    use ambition_persistence::settings::UserSettings;
    use ambition_platformer_primitives::lifecycle::{SessionRoot, SessionScopeId};
    use bevy::prelude::*;
    use leafwing_input_manager::prelude::{ActionState, InputMap};

    #[test]
    fn the_participant_spawns_once_and_owns_device_state() {
        let mut app = App::new();
        app.add_systems(Update, spawn_primary_input_participant);

        app.update();
        app.update();

        let mut participants = app
            .world_mut()
            .query_filtered::<Entity, With<InputParticipant>>();
        let all: Vec<Entity> = participants.iter(app.world()).collect();
        assert_eq!(all.len(), 1, "the spawn is idempotent across frames");
        let participant = all[0];
        assert!(
            app.world()
                .entity(participant)
                .contains::<ActionState<SandboxAction>>(),
            "the participant owns the leafwing action state"
        );
        assert!(
            app.world()
                .entity(participant)
                .contains::<InputMap<SandboxAction>>(),
            "the participant owns the active input map"
        );
    }

    #[test]
    fn the_session_lifecycle_claims_and_releases_the_gameplay_context() {
        let mut app = App::new();
        app.init_resource::<ActiveInputContext>();
        app.add_systems(
            Update,
            (
                spawn_primary_input_participant,
                declare_gameplay_input_context,
                resolve_active_input_context,
            )
                .chain(),
        );

        // Before any session (startup cards, launcher): nothing claims
        // gameplay, so the participant's actions do not route to the sim.
        app.update();
        assert!(
            !app.world()
                .resource::<ActiveInputContext>()
                .gameplay_owned(),
            "no session -> gameplay context is not owned"
        );

        // A live session claims the context; teardown releases it. The
        // participant entity itself is untouched either way.
        let root = app.world_mut().spawn(SessionRoot(SessionScopeId(7))).id();
        app.update();
        assert!(app
            .world()
            .resource::<ActiveInputContext>()
            .gameplay_owned());
        let participant = {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<InputParticipant>>();
            q.single(app.world()).expect("participant exists")
        };
        app.world_mut().despawn(root);
        app.update();
        assert!(
            !app.world()
                .resource::<ActiveInputContext>()
                .gameplay_owned(),
            "session teardown retracts the gameplay claim"
        );
        assert!(
            app.world().get_entity(participant).is_ok(),
            "destroying the session does not destroy the participant"
        );
        assert!(
            app.world()
                .entity(participant)
                .contains::<ActionState<SandboxAction>>(),
            "participant device state survives session teardown"
        );
    }

    #[test]
    fn unfocus_gate_is_off_by_default() {
        let settings = UserSettings::default();
        assert!(!settings.gameplay.pause_input_when_unfocused);
        // With the setting OFF, input is never suppressed regardless of focus.
        assert!(!input_suppressed_by_unfocus(&settings, [false]));
        assert!(!input_suppressed_by_unfocus(&settings, [true]));
        assert!(!input_suppressed_by_unfocus(&settings, std::iter::empty()));
    }

    #[test]
    fn unfocus_gate_suppresses_only_when_on_and_no_window_focused() {
        let mut settings = UserSettings::default();
        settings.gameplay.pause_input_when_unfocused = true;
        // Some window focused → not suppressed.
        assert!(!input_suppressed_by_unfocus(&settings, [false, true]));
        assert!(!input_suppressed_by_unfocus(&settings, [true]));
        // No window focused → suppressed.
        assert!(input_suppressed_by_unfocus(&settings, [false, false]));
        // No window at all (headless) → suppressed (safe direction for the opt-in).
        assert!(input_suppressed_by_unfocus(&settings, std::iter::empty()));
    }
}
