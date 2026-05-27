use bevy::prelude::*;

use super::runtime::DialogState;
use super::ui::DialogChoiceSlot;
use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::MenuControlFrame;
#[cfg(feature = "input")]
use crate::ui_nav::apply_vertical_scroll;

#[cfg(feature = "input")]
pub fn dialog_pointer_input(
    mut dialogue: ResMut<DialogState>,
    mode: Res<State<GameMode>>,
    next_mode: ResMut<NextState<GameMode>>,
    choices: Query<(&Interaction, &DialogChoiceSlot), Changed<Interaction>>,
) {
    if !dialogue.active() {
        return;
    }
    if !matches!(mode.get(), GameMode::Dialogue) {
        return;
    }

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
                if dialogue.selected_option != index {
                    dialogue.pointer_armed = None;
                }
                dialogue.selected_option = index;
            }
            Interaction::Pressed => {
                let index = slot.index.min(option_count.saturating_sub(1));

                #[cfg(target_os = "android")]
                {
                    let confirm =
                        dialogue.selected_option == index && dialogue.pointer_armed == Some(index);
                    dialogue.selected_option = index;
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
