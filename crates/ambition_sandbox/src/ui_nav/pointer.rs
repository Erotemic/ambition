use bevy::prelude::Interaction;

use crate::settings::{MenuPointerPress, MenuTapMode};

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
) -> RowPointerOutcome {
    match interaction {
        Interaction::Hovered => {
            if *selected != index {
                *selected = index;
            }
            RowPointerOutcome::Hovered
        }
        Interaction::Pressed => {
            let press = tap_mode.resolve_press(index, *selected, destructive, pointer_armed);
            *selected = index;
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
) -> RowPointerUpdate {
    let mut selected = selected;
    let mut pointer_armed = pointer_armed;
    let outcome = handle_selectable_row_interaction(
        interaction,
        index,
        &mut selected,
        tap_mode,
        destructive,
        &mut pointer_armed,
    );
    RowPointerUpdate {
        selected,
        pointer_armed,
        outcome,
    }
}
