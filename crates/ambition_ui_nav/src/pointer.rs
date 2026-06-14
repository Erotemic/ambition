use bevy::prelude::Interaction;

use ambition_input::settings::{MenuPointerPress, MenuTapMode};

/// Which input source currently owns menu focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuFocusOwner {
    Keyboard,
    Pointer,
}

impl Default for MenuFocusOwner {
    fn default() -> Self {
        Self::Keyboard
    }
}

/// Tracks the current menu focus owner plus the last row the pointer
/// actually hovered.
///
/// Keyboard/controller navigation may claim focus and keep it until the
/// pointer *moves to a different row*. A stationary hover should not keep
/// reasserting itself over newer directional navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuFocusState {
    pub owner: MenuFocusOwner,
    pub last_hovered_row: Option<usize>,
}

impl Default for MenuFocusState {
    fn default() -> Self {
        Self {
            owner: MenuFocusOwner::Keyboard,
            last_hovered_row: None,
        }
    }
}

impl MenuFocusState {
    pub fn mark_keyboard(&mut self) {
        self.owner = MenuFocusOwner::Keyboard;
    }

    pub fn mark_pointer(&mut self, index: usize) {
        self.owner = MenuFocusOwner::Pointer;
        self.last_hovered_row = Some(index);
    }
}

/// Semantic result of a pointer interaction with a selectable UI row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowPointerOutcome {
    None,
    Hovered,
    Confirmed,
}

/// Complete state update returned by a selectable-row pointer interaction.
///
/// Returning the updated values, instead of borrowing two fields from the same
/// parent state object, keeps callers on the right side of Rust's aliasing
/// rules when their menu state lives inside a single Bevy resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowPointerUpdate {
    pub selected: usize,
    pub pointer_armed: Option<usize>,
    pub focus: MenuFocusState,
    pub outcome: RowPointerOutcome,
}

/// Shared hover/tap behavior for menu-like selectable rows.
///
/// Hover only moves the selected index. Press resolves through the user's tap
/// mode so touch and mouse rows can share single-tap, tap-then-confirm, and
/// destructive-guard semantics.
pub fn handle_selectable_row_interaction(
    interaction: &Interaction,
    index: usize,
    selected: &mut usize,
    tap_mode: MenuTapMode,
    destructive: bool,
    pointer_armed: &mut Option<usize>,
    focus: &mut MenuFocusState,
) -> RowPointerOutcome {
    match interaction {
        Interaction::Hovered => {
            if focus.owner == MenuFocusOwner::Keyboard && focus.last_hovered_row == Some(index) {
                return RowPointerOutcome::None;
            }
            if *selected != index {
                *selected = index;
                // Once the pointer has drifted to a different row, a prior
                // tap-to-confirm arm should not survive. This matches mobile
                // expectations: a touch that becomes a drag is navigation, not
                // a latent activation waiting to fire on the next tap.
                *pointer_armed = None;
            }
            focus.mark_pointer(index);
            RowPointerOutcome::Hovered
        }
        Interaction::Pressed => {
            let press = tap_mode.resolve_press(index, *selected, destructive, pointer_armed);
            *selected = index;
            focus.mark_pointer(index);
            if matches!(press, MenuPointerPress::Confirm) {
                RowPointerOutcome::Confirmed
            } else {
                RowPointerOutcome::None
            }
        }
        Interaction::None => RowPointerOutcome::None,
    }
}

/// Value-oriented variant of [`handle_selectable_row_interaction`].
///
/// Prefer this form when the selected index and pointer-arm state are fields on
/// the same struct/resource. It avoids passing two simultaneous `&mut` borrows
/// of that parent into a helper call.
pub fn resolve_selectable_row_interaction(
    interaction: &Interaction,
    index: usize,
    selected: usize,
    tap_mode: MenuTapMode,
    destructive: bool,
    pointer_armed: Option<usize>,
    focus: MenuFocusState,
) -> RowPointerUpdate {
    let mut selected = selected;
    let mut pointer_armed = pointer_armed;
    let mut focus = focus;
    let outcome = handle_selectable_row_interaction(
        interaction,
        index,
        &mut selected,
        tap_mode,
        destructive,
        &mut pointer_armed,
        &mut focus,
    );
    RowPointerUpdate {
        selected,
        pointer_armed,
        focus,
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_to_new_row_clears_tap_arm() {
        let update = resolve_selectable_row_interaction(
            &Interaction::Hovered,
            2,
            1,
            MenuTapMode::TapToSelectThenConfirm,
            false,
            Some(1),
            MenuFocusState::default(),
        );
        assert_eq!(update.selected, 2);
        assert_eq!(update.pointer_armed, None);
        assert_eq!(update.focus.owner, MenuFocusOwner::Pointer);
        assert_eq!(update.outcome, RowPointerOutcome::Hovered);
    }

    #[test]
    fn tap_to_select_requires_second_press_same_row() {
        let first = resolve_selectable_row_interaction(
            &Interaction::Pressed,
            3,
            0,
            MenuTapMode::TapToSelectThenConfirm,
            false,
            None,
            MenuFocusState::default(),
        );
        assert_eq!(first.selected, 3);
        assert_eq!(first.pointer_armed, Some(3));
        assert_eq!(first.focus.owner, MenuFocusOwner::Pointer);
        assert_eq!(first.outcome, RowPointerOutcome::None);

        let second = resolve_selectable_row_interaction(
            &Interaction::Pressed,
            3,
            first.selected,
            MenuTapMode::TapToSelectThenConfirm,
            false,
            first.pointer_armed,
            MenuFocusState::default(),
        );
        assert_eq!(second.pointer_armed, None);
        assert_eq!(second.focus.owner, MenuFocusOwner::Pointer);
        assert_eq!(second.outcome, RowPointerOutcome::Confirmed);
    }

    #[test]
    fn keyboard_focus_blocks_stale_hover_on_same_row() {
        let update = resolve_selectable_row_interaction(
            &Interaction::Hovered,
            2,
            1,
            MenuTapMode::TapToSelectThenConfirm,
            false,
            Some(1),
            MenuFocusState {
                owner: MenuFocusOwner::Keyboard,
                last_hovered_row: Some(2),
            },
        );
        assert_eq!(update.selected, 1);
        assert_eq!(update.pointer_armed, Some(1));
        assert_eq!(update.focus.owner, MenuFocusOwner::Keyboard);
        assert_eq!(update.outcome, RowPointerOutcome::None);
    }
}
