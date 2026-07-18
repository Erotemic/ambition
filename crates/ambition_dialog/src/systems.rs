//! Dialogue Bevy systems: input translation + the typewriter reveal tick.
//!
//! These read [`crate::runtime::DialogState`] and write its `pending_*`
//! request fields (which [`crate::bridge`] later drains into the runner):
//! - [`dialog_reveal_tick`] — advances the visible substring of the line/options.
//! - [`dialog_input`] — semantic menu nav from keyboard, gamepad, touch controls, wheel, and drag.
//! - [`dialog_pointer_input`] — mouse/touch choice-row selection, `input`-gated.
//!
//! Presentation only; the Yarn runner owns the line/option state machine.

use bevy::prelude::*;

use crate::runtime::DialogState;
use crate::speech_sfx::{should_play_talk_blip, talk_blip_id_for_speaker, DialogueVoiceCatalog};
#[cfg(feature = "input")]
use ambition_input::{ActiveInputKind, MenuControlFrame};
#[cfg(feature = "input")]
use ambition_persistence::settings::{MenuTapMode, UserSettings};
use ambition_sfx::{SfxMessage, SfxWriter};
use ambition_ui_nav::DialogChoiceSlot;
#[cfg(feature = "input")]
use ambition_ui_nav::{resolve_selectable_row_interaction, RowPointerOutcome};
#[cfg(feature = "input")]
use bevy::window::PrimaryWindow;

/// Advance the active dialogue line's typewriter reveal.
///
/// This is presentation only: Yarn still owns the line/option state
/// machine, while the Bevy side owns the timing of what substring is
/// visible right now.
pub fn dialog_reveal_tick(
    time: Res<Time>,
    voice_catalog: Option<Res<DialogueVoiceCatalog>>,
    mut dialogue: ResMut<DialogState>,
    mut sfx: SfxWriter,
) {
    if !dialogue.active() || dialogue.current_line.is_empty() {
        return;
    }
    if !dialogue.line_reveal_complete() {
        let previous_visible_chars = dialogue.visible_line_char_count();
        dialogue.tick_reveal(time.delta_secs());
        let visible_chars = dialogue.visible_line_char_count();
        if should_play_talk_blip(
            &dialogue.current_line,
            previous_visible_chars,
            visible_chars,
        ) {
            sfx.write(SfxMessage::Play {
                id: talk_blip_id_for_speaker(
                    voice_catalog.as_deref(),
                    dialogue.speaker_label_for_sfx(),
                    dialogue.dialogue_id(),
                    dialogue.speech_style(),
                ),
                pos: Vec2::ZERO,
            });
        }
        return;
    }
    if dialogue.current_options.is_empty() {
        if dialogue.line_last_before_options()
            && !dialogue.runner_done_pending_close
            && !dialogue.pending_advance
        {
            dialogue.pending_advance = true;
        }
        return;
    }
    if !dialogue.options_reveal_complete() {
        dialogue.tick_options_reveal(time.delta_secs());
    }
}

#[cfg(feature = "input")]
pub fn dialog_pointer_input(
    mut dialogue: ResMut<DialogState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    choices: Query<(&Interaction, &DialogChoiceSlot), Changed<Interaction>>,
    settings: Option<Res<UserSettings>>,
    active_input: Option<Res<ActiveInputKind>>,
    touches: Option<Res<bevy::input::touch::Touches>>,
) {
    if !dialogue.active() {
        return;
    }
    let cursor_position = windows.single().ok().and_then(Window::cursor_position);
    let configured_tap_mode = settings
        .as_deref()
        .map(|settings| settings.controls.menu_tap_mode)
        .unwrap_or_default();
    // `ActiveInputKind` is the shared last-genuine-input policy, while the
    // live touch resource closes the same-frame ordering gap for a finger that
    // presses a row before the touch fold has published `Touch`.
    let direct_touch_active = touches
        .as_deref()
        .is_some_and(|touches| touches.iter().next().is_some());
    let pointer_input = if direct_touch_active {
        Some(ActiveInputKind::Touch)
    } else {
        active_input.as_deref().copied()
    };
    let tap_mode = effective_dialog_tap_mode(configured_tap_mode, pointer_input);

    let option_count = dialogue.options().len();
    for (interaction, slot) in &choices {
        let valid_slot = if option_count == 0 {
            slot.index == 0
        } else {
            slot.index < option_count
        };
        if !valid_slot {
            continue;
        }
        let index = slot.index.min(option_count.saturating_sub(1));

        match interaction {
            Interaction::Hovered => {
                // A freshly rebuilt windowed list can spawn under a stationary
                // cursor. Only genuine mouse motion owns hover selection; touch,
                // keyboard, physical gamepad, and the touch gamepad keep their
                // newer semantic selection until the mouse actually moves.
                if active_input
                    .as_deref()
                    .is_some_and(|kind| *kind != ActiveInputKind::Mouse)
                {
                    continue;
                }
                let update = handle_dialog_choice_hover(
                    index,
                    dialogue.selected_option,
                    dialogue.pointer_armed,
                    dialogue.focus,
                    dialogue.last_pointer_position,
                    cursor_position,
                );
                dialogue.selected_option = update.selected;
                dialogue.pointer_armed = update.pointer_armed;
                dialogue.focus = update.focus;
                dialogue.last_pointer_position = update.last_pointer_position;
            }
            Interaction::Pressed => {
                let update = resolve_selectable_row_interaction(
                    interaction,
                    index,
                    dialogue.selected_option,
                    tap_mode,
                    false,
                    dialogue.pointer_armed,
                    dialogue.focus,
                );
                dialogue.selected_option = update.selected;
                dialogue.pointer_armed = update.pointer_armed;
                dialogue.focus = update.focus;
                dialogue.last_pointer_position = cursor_position;
                if update.outcome == RowPointerOutcome::Confirmed {
                    dialogue.confirm_or_advance();
                }
                return;
            }
            Interaction::None => {}
        }
    }
}

#[cfg(not(feature = "input"))]
pub fn dialog_pointer_input() {}

/// Resolve the configured pointer policy for the device that actually issued
/// the interaction.
///
/// The desktop default (`SingleTapWithDestructiveGuard`) intentionally confirms
/// ordinary mouse rows immediately. A direct touch press, however, may still
/// become a drag-scroll gesture, so that default is promoted to
/// `TapToSelectThenConfirm`. Users who explicitly choose unconditional
/// `SingleTap` retain that behavior on every device.
#[cfg(feature = "input")]
fn effective_dialog_tap_mode(
    configured: MenuTapMode,
    active_input: Option<ActiveInputKind>,
) -> MenuTapMode {
    if active_input == Some(ActiveInputKind::Touch)
        && configured == MenuTapMode::SingleTapWithDestructiveGuard
    {
        MenuTapMode::TapToSelectThenConfirm
    } else {
        configured
    }
}

#[cfg(feature = "input")]
pub fn dialog_input(menu: Res<MenuControlFrame>, mut dialogue: ResMut<DialogState>) {
    apply_dialog_menu_input(&menu, &mut dialogue);
}

#[cfg(feature = "input")]
fn apply_dialog_menu_input(menu: &MenuControlFrame, dialogue: &mut DialogState) {
    if !dialogue.active() {
        return;
    }
    if menu.back || menu.start {
        // Back-button close: the dispatch system tells the runner to stop.
        // `close()` flips `DialogState.active` this same frame so every
        // presentation/input backend observes the same immediate closure.
        dialogue.close();
        return;
    }

    // Directional semantic navigation is shared by keyboard arrows, D-pad,
    // physical analog stick, and the on-screen touch joystick. It retains the
    // familiar wrapping cursor behavior.
    if menu.up {
        dialogue.select_delta(-1);
    }
    if menu.down {
        dialogue.select_delta(1);
    }

    // Mouse wheel, touchpad, and touch drag are scroll gestures. Preserve their
    // discrete magnitude and clamp at list edges rather than wrapping from the
    // bottom to the top of a long dialogue choice list.
    let scroll_steps = menu.vertical_scroll_steps();
    if scroll_steps != 0 {
        dialogue.select_delta_clamped(-(scroll_steps as isize));
    }

    if menu.select {
        // The same semantic Confirm edge comes from keyboard, physical gamepad,
        // touch gamepad, or the on-screen Interact/Jump buttons.
        dialogue.confirm_or_advance();
    }
}

#[cfg(not(feature = "input"))]
pub fn dialog_input() {}

#[cfg(feature = "input")]
fn handle_dialog_choice_hover(
    index: usize,
    selected: usize,
    pointer_armed: Option<usize>,
    focus: ambition_ui_nav::MenuFocusState,
    last_pointer_position: Option<Vec2>,
    cursor_position: Option<Vec2>,
) -> DialogHoverUpdate {
    if focus.owner == ambition_ui_nav::MenuFocusOwner::Keyboard
        && last_pointer_position.is_some()
        && (cursor_position.is_none() || last_pointer_position == cursor_position)
    {
        return DialogHoverUpdate {
            selected,
            pointer_armed,
            focus,
            last_pointer_position,
        };
    }

    let update = resolve_selectable_row_interaction(
        &Interaction::Hovered,
        index,
        selected,
        ambition_persistence::settings::MenuTapMode::TapToSelectThenConfirm,
        false,
        pointer_armed,
        focus,
    );
    DialogHoverUpdate {
        selected: update.selected,
        pointer_armed: update.pointer_armed,
        focus: update.focus,
        last_pointer_position: cursor_position.or(last_pointer_position),
    }
}

#[cfg(feature = "input")]
#[derive(Clone, Copy, Debug, PartialEq)]
struct DialogHoverUpdate {
    selected: usize,
    pointer_armed: Option<usize>,
    focus: ambition_ui_nav::MenuFocusState,
    last_pointer_position: Option<Vec2>,
}

#[cfg(all(test, feature = "input"))]
mod tests {
    use super::*;
    use ambition_ui_nav::MenuFocusOwner;

    fn dialogue_with_options(count: usize) -> DialogState {
        let mut dialogue = DialogState::default();
        dialogue.active = true;
        dialogue.current_options = (0..count)
            .map(|index| crate::DialogChoice {
                label: format!("Option {index}"),
                ..default()
            })
            .collect();
        dialogue.reveal_full_options();
        dialogue
    }

    #[test]
    fn direct_touch_uses_drag_safe_tap_policy_without_changing_mouse_or_explicit_single_tap() {
        assert_eq!(
            effective_dialog_tap_mode(
                MenuTapMode::SingleTapWithDestructiveGuard,
                Some(ActiveInputKind::Touch),
            ),
            MenuTapMode::TapToSelectThenConfirm,
        );
        assert_eq!(
            effective_dialog_tap_mode(
                MenuTapMode::SingleTapWithDestructiveGuard,
                Some(ActiveInputKind::Mouse),
            ),
            MenuTapMode::SingleTapWithDestructiveGuard,
        );
        assert_eq!(
            effective_dialog_tap_mode(MenuTapMode::SingleTap, Some(ActiveInputKind::Touch)),
            MenuTapMode::SingleTap,
        );
    }

    #[test]
    fn wheel_and_touch_drag_scroll_preserve_magnitude_and_clamp() {
        let mut dialogue = dialogue_with_options(8);
        apply_dialog_menu_input(
            &MenuControlFrame {
                scroll_y: -3.0,
                ..default()
            },
            &mut dialogue,
        );
        assert_eq!(dialogue.selected_option(), 3);

        apply_dialog_menu_input(
            &MenuControlFrame {
                scroll_y: -6.0,
                ..default()
            },
            &mut dialogue,
        );
        assert_eq!(dialogue.selected_option(), 7);

        apply_dialog_menu_input(
            &MenuControlFrame {
                scroll_y: -1.0,
                ..default()
            },
            &mut dialogue,
        );
        assert_eq!(
            dialogue.selected_option(),
            7,
            "scroll gestures stop at the list edge rather than wrapping"
        );
    }

    #[test]
    fn directional_and_confirm_share_the_same_authoritative_selection() {
        let mut dialogue = dialogue_with_options(4);
        apply_dialog_menu_input(
            &MenuControlFrame {
                up: true,
                ..default()
            },
            &mut dialogue,
        );
        assert_eq!(dialogue.selected_option(), 3, "directional nav wraps");

        apply_dialog_menu_input(
            &MenuControlFrame {
                select: true,
                ..default()
            },
            &mut dialogue,
        );
        assert_eq!(dialogue.pending_select, Some(3));
    }

    #[test]
    fn keyboard_focus_blocks_stale_hover_on_same_row() {
        let update = handle_dialog_choice_hover(
            2,
            1,
            Some(1),
            ambition_ui_nav::MenuFocusState {
                owner: MenuFocusOwner::Keyboard,
                last_hovered_row: Some(2),
            },
            Some(Vec2::new(120.0, 240.0)),
            Some(Vec2::new(120.0, 240.0)),
        );

        assert_eq!(update.selected, 1);
        assert_eq!(update.pointer_armed, Some(1));
        assert_eq!(update.focus.owner, MenuFocusOwner::Keyboard);
    }

    #[test]
    fn keyboard_focus_blocks_stationary_hover_after_scroll() {
        let update = handle_dialog_choice_hover(
            5,
            1,
            Some(1),
            ambition_ui_nav::MenuFocusState {
                owner: MenuFocusOwner::Keyboard,
                last_hovered_row: Some(1),
            },
            Some(Vec2::new(220.0, 180.0)),
            Some(Vec2::new(220.0, 180.0)),
        );

        assert_eq!(update.selected, 1);
        assert_eq!(update.pointer_armed, Some(1));
        assert_eq!(update.focus.owner, MenuFocusOwner::Keyboard);
        assert_eq!(update.last_pointer_position, Some(Vec2::new(220.0, 180.0)));
    }
}
