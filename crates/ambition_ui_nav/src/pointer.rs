//! Pointer/touch row activation: `MenuFocusOwner` / `MenuFocusState` track which
//! source owns focus, and `resolve_selectable_row_interaction` applies the host's
//! `MenuTapMode` (from `ambition_input::settings`) to a Bevy `Interaction` to
//! decide hover-vs-select-vs-activate.

use bevy::prelude::{Interaction, Vec2};

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

/// What a pointer went DOWN on, and where.
///
/// Arm on press, cancel past the drag threshold, and activate on release. This
/// permits one-tap selection without turning a drag gesture into activation.
///
/// The target identity must survive control-entity rebuilds between press and
/// release. Flat lists use a row index ([`RowPress`]); other surfaces may use a
/// stable action identity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressArm<T> {
    /// What the pointer went down on.
    pub target: Option<T>,
    /// Where it went down, for the drag test. `None` when the backend could not
    /// say, in which case a drag cannot be detected and release still activates —
    /// no position is not evidence of a drag.
    pub origin: Option<Vec2>,
    /// The pointer travelled past the threshold: this press is dead and will not
    /// activate, however it ends.
    pub cancelled: bool,
}

/// A press armed on a flat list's row index — the original shape, and still the
/// right one for anything with rows.
pub type RowPress = PressArm<usize>;

// `#[derive(Default)]` would demand `T: Default`, which a menu-action enum has no reason to
// satisfy.
impl<T> Default for PressArm<T> {
    fn default() -> Self {
        Self {
            target: None,
            origin: None,
            cancelled: false,
        }
    }
}

/// How far a pointer may travel between press and release and still count as a
/// tap, in logical pixels.
///
/// Generous enough for normal thumb movement during a deliberate tap while
/// still distinguishing a drag.
pub const ROW_TAP_SLOP_PX: f32 = 16.0;

impl<T: Clone + PartialEq> PressArm<T> {
    /// A pointer went down on `target` at `origin`.
    pub fn press(&mut self, target: T, origin: Option<Vec2>) {
        self.target = Some(target);
        self.origin = origin;
        self.cancelled = false;
    }

    /// Whether anything is armed.
    pub fn is_armed(&self) -> bool {
        self.target.is_some()
    }

    /// What is armed, if anything.
    pub fn armed(&self) -> Option<&T> {
        self.target.as_ref()
    }

    /// The pointer moved. Past [`ROW_TAP_SLOP_PX`] this press stops being a tap.
    pub fn moved(&mut self, to: Option<Vec2>) {
        if let (Some(origin), Some(to)) = (self.origin, to) {
            if origin.distance(to) > ROW_TAP_SLOP_PX {
                self.cancelled = true;
            }
        }
    }

    /// The pointer came up. Returns the target to activate, if this press
    /// survived as a tap ON THE SAME TARGET, and clears the arm either way.
    pub fn release(&mut self, target: T, at: Option<Vec2>) -> Option<T> {
        self.moved(at);
        let armed = self.target.take();
        let cancelled = std::mem::take(&mut self.cancelled);
        self.origin = None;
        (!cancelled && armed.as_ref() == Some(&target)).then_some(target)
    }

    /// The pointer came up SOMEWHERE, and the surface cannot say where.
    ///
    ///  weaker than [`Self::release`] on purpose, and only correct for a
    /// surface that rebuilds under the finger. A perspective cube respawns
    /// its cells continuously, so "which control is under the pointer now" is
    /// not evidence about which one the press began on — the press already
    /// captured that. A flat list HAS that evidence and must use it, or
    /// pressing one row and releasing on another activates the first.
    pub fn release_anywhere(&mut self) -> Option<T> {
        let armed = self.target.take();
        let cancelled = std::mem::take(&mut self.cancelled);
        self.origin = None;
        armed.filter(|_| !cancelled)
    }

    /// The pointer LEFT this target without coming up.
    ///
    ///  a leave is not a release, and the two are the same Bevy signal.
    /// `Interaction::None` is raised both when a finger lifts and when a held
    /// pointer stops covering the row, and [`Self::release`] cannot tell them
    /// apart — its only guards are the drag threshold and the target, so a
    /// press that slides a few pixels off a short row would activate under a
    /// finger that never came up. The caller decides which event it is from the
    /// live button/touch state and calls this one for a leave.
    ///
    /// Only the armed target can be left, so a sibling row going `None` in the
    /// same frame does not take the arm down with it.
    pub fn left(&mut self, target: T) {
        if self.target.as_ref() == Some(&target) {
            self.clear();
        }
    }

    /// Abandon the press without activating — the row went away, the menu
    /// closed, the finger left the control.
    pub fn clear(&mut self) {
        *self = Self::default();
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
            let press = tap_mode.resolve_press(index, &*selected, destructive, pointer_armed);
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

/// Marker on a rendered dialog choice-row entity, carrying its option index.
///
/// The render layer's dialog UI spawns these; the sim-side dialog
/// pointer-input system reads them to map a click to a choice. Content-free
/// pointer-row vocabulary (this crate's concern), so the renderer and the
/// input system both name it without either depending on the other.
#[derive(bevy::prelude::Component, Clone, Copy, Debug)]
pub struct DialogChoiceSlot {
    pub index: usize,
}
