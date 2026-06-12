use bevy::prelude::*;

use super::runtime::DialogState;
use super::ui::DialogChoiceSlot;
use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::MenuControlFrame;
#[cfg(feature = "input")]
use crate::ui_nav::{apply_vertical_scroll, resolve_selectable_row_interaction};
#[cfg(feature = "input")]
use bevy::window::PrimaryWindow;

/// Advance the active dialogue line's typewriter reveal.
///
/// This is presentation only: Yarn still owns the line/option state
/// machine, while the Bevy side owns the timing of what substring is
/// visible right now.
pub fn dialog_reveal_tick(time: Res<Time>, mut dialogue: ResMut<DialogState>) {
    if !dialogue.active() || dialogue.current_line.is_empty() {
        return;
    }
    if !dialogue.line_reveal_complete() {
        dialogue.tick_reveal(time.delta_secs());
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
    mode: Res<State<GameMode>>,
    next_mode: ResMut<NextState<GameMode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    choices: Query<(&Interaction, &DialogChoiceSlot), Changed<Interaction>>,
) {
    if !dialogue.active() {
        return;
    }
    if !matches!(mode.get(), GameMode::Dialogue) {
        return;
    }
    let cursor_position = windows.single().ok().and_then(Window::cursor_position);

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

        match interaction {
            Interaction::Hovered => {
                let index = slot.index.min(option_count.saturating_sub(1));
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
                continue;
            }
            Interaction::Pressed => {
                let index = slot.index.min(option_count.saturating_sub(1));

                #[cfg(target_os = "android")]
                {
                    let confirm =
                        dialogue.selected_option == index && dialogue.pointer_armed == Some(index);
                    dialogue.selected_option = index;
                    dialogue.focus.mark_pointer(index);
                    dialogue.last_pointer_position = cursor_position;
                    if confirm {
                        dialogue.pointer_armed = None;
                        // Confirm advances via the Yarn dispatch
                        // (sets pending_select/advance); the
                        // dialog-completed observer flips
                        // GameMode back to Playing.
                        dialogue.confirm_or_advance();
                    } else {
                        dialogue.pointer_armed = Some(index);
                    }
                }

                #[cfg(not(target_os = "android"))]
                {
                    dialogue.selected_option = index;
                    dialogue.focus.mark_pointer(index);
                    dialogue.last_pointer_position = cursor_position;
                    dialogue.confirm_or_advance();
                }
                return;
            }
            Interaction::None => {}
        }
    }
    // `next_mode` is reserved for the back-button path below — keep
    // the parameter on the signature so the system schedule slot
    // doesn't have to change when we add back-button support to the
    // pointer surface.
    let _ = next_mode;
}

#[cfg(not(feature = "input"))]
pub fn dialog_pointer_input() {}

#[cfg(feature = "input")]
pub fn dialog_input(
    menu: Res<MenuControlFrame>,
    mut dialogue: ResMut<DialogState>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
) {
    if !dialogue.active() {
        return;
    }
    if !matches!(mode.get(), GameMode::Dialogue) {
        return;
    }
    if menu.back || menu.start {
        // Back-button close: the dispatch system will tell the
        // runner to stop, and the dialog-completed observer will
        // flip the GameMode. The explicit `next_mode.set` here is
        // belt-and-suspenders so the UI hides this same frame.
        dialogue.close();
        next_mode.set(GameMode::Playing);
        return;
    }
    let mut frame = crate::input::MenuInputFrame {
        up: menu.up,
        down: menu.down,
        left: menu.left,
        right: menu.right,
        select: menu.select,
        back: menu.back,
        start: menu.start,
    };
    apply_vertical_scroll(&mut frame, menu.vertical_scroll_steps());
    if frame.up {
        dialogue.select_delta(-1);
    }
    if frame.down {
        dialogue.select_delta(1);
    }
    if frame.select {
        // The Yarn runner closes the dialogue asynchronously via
        // the `DialogueCompleted` observer (which flips GameMode
        // back to Playing). `confirm_or_advance` now always returns
        // `false`; the legacy `if closed { ... }` branch is gone.
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
    focus: crate::ui_nav::MenuFocusState,
    last_pointer_position: Option<Vec2>,
    cursor_position: Option<Vec2>,
) -> DialogHoverUpdate {
    if focus.owner == crate::ui_nav::MenuFocusOwner::Keyboard
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
        crate::persistence::settings::MenuTapMode::TapToSelectThenConfirm,
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
    focus: crate::ui_nav::MenuFocusState,
    last_pointer_position: Option<Vec2>,
}

#[cfg(all(test, feature = "input"))]
mod tests {
    use super::*;
    use crate::ui_nav::MenuFocusOwner;

    #[test]
    fn keyboard_focus_blocks_stale_hover_on_same_row() {
        let update = handle_dialog_choice_hover(
            2,
            1,
            Some(1),
            crate::ui_nav::MenuFocusState {
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
            crate::ui_nav::MenuFocusState {
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
