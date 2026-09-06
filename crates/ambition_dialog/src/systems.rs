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
use ambition_input::{ActiveDevice, MenuControlFrame, SeatActiveDevices};
#[cfg(feature = "input")]
use ambition_persistence::settings::{MenuTapMode, UserSettings};
use ambition_sfx::{SfxMessage, SfxWriter};
#[cfg(feature = "input")]
use ambition_ui_nav::DialogChoiceSlot;
#[cfg(feature = "input")]
use ambition_ui_nav::{resolve_selectable_row_interaction, RowPointerOutcome};
#[cfg(feature = "input")]
use bevy::input::mouse::MouseButton;
#[cfg(feature = "input")]
use bevy::input::touch::Touches;
#[cfg(feature = "input")]
use bevy::input::ButtonInput;
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
    devices: Option<Res<SeatActiveDevices>>,
    touches: Option<Res<Touches>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
) {
    if !dialogue.active() {
        return;
    }
    let cursor_position = windows.single().ok().and_then(Window::cursor_position);
    let configured_tap_mode = settings
        .as_deref()
        .map(|settings| settings.controls.menu_tap_mode)
        .unwrap_or_default();
    // `SeatActiveDevices::machine` is the shared last-genuine-input policy, while the
    // live touch resource closes the same-frame ordering gap for a finger that
    // presses a row before the touch fold has published `Touch`.
    let direct_touch_active = touches
        .as_deref()
        .is_some_and(|touches| touches.iter().next().is_some());
    let pointer_input = if direct_touch_active {
        Some(ActiveDevice::Touch)
    } else {
        devices.as_deref().map(SeatActiveDevices::machine)
    };
    let tap_mode = effective_dialog_tap_mode(configured_tap_mode, pointer_input);

    //  a touch device never writes the window cursor, so `RowPress`'s drag
    // test — the thing that makes one-tap activation safe — was being fed `None`
    // on the only platform it was built for, and every phone drag passed it. The
    // finger's own position is the drag evidence. Read the JUST-RELEASED finger
    // too: on the frame it lifts, Bevy has already moved it out of `pressed`
    // (observed against `bevy_input` 0.18, not assumed).
    let pointer_position = touches
        .as_deref()
        .and_then(|touches| {
            touches
                .iter()
                .next()
                .or_else(|| touches.iter_just_released().next())
                .map(|touch| touch.position())
        })
        .or(cursor_position);
    let pointer_up = resolve_pointer_up(touches.as_deref(), mouse_buttons.as_deref());

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
                if devices
                    .as_deref()
                    .is_some_and(|devices| devices.machine() != ActiveDevice::Mouse)
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
                // A mouse button released over the row it pressed reports
                // `Hovered`, not `None` — so this is where an ordinary click
                // completes.  but `Hovered` is also raised by mere motion, and a
                // press that survives a stray hover must not be spent by it: ask
                // whether a pointer actually came up this frame first.
                if pointer_up.came_up()
                    && dialogue.row_press.release(index, pointer_position) == Some(index)
                {
                    dialogue.selected_option = index;
                    dialogue.confirm_or_advance();
                    return;
                }
            }
            Interaction::Pressed => {
                //  press SELECTS and ARMS; it no longer confirms. Activating
                // on press is what forced touch into two-tap mode: a finger that
                // lands on a row and then slides has already fired it. Now the
                // row highlights, the press is remembered, and a drag can still
                // cancel it — see `RowPress`.
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
                //  the tap mode decides WHETHER this tap activates; the release
                // decides WHEN. `SingleTapWithDestructiveGuard` — the default on
                // every platform — returns `Confirmed` from the press itself, so
                // acting on it here would activate on press for every ordinary
                // dialogue row and leave the whole release path governing nothing
                // but an explicitly-configured two-tap mode. Arming instead keeps
                // the policy exactly as configured (one tap for a plain row, two
                // for a guarded one) while making that tap a real tap: down, then
                // up, on the same row.
                //
                //  this is the split `MenuTapMode::default()` already argues for
                // in its own comment — a confirmation policy answers "how many
                // taps", and drag-cancellation belongs to the gesture layer.
                if update.outcome != RowPointerOutcome::Confirmed {
                    // This press only moved the selection; the mode wants another
                    // tap. Nothing is armed, so its release activates nothing.
                    dialogue.row_press.clear();
                    return;
                }
                dialogue.row_press.press(index, pointer_position);
                //  a whole tap can fit inside one frame. `ui_focus_system` sets
                // `Pressed` on the just-pressed frame and defers the reset to
                // `None` to the NEXT frame (its `entities_to_reset` list), by which
                // time the release edge is gone from `Touches`/`ButtonInput`. A
                // pointer that is already up on the frame the row went down has
                // pressed AND released here, so it completes here.
                if pointer_up.came_up() {
                    dialogue.row_press.clear();
                    dialogue.confirm_or_advance();
                }
                return;
            }
            //  `Interaction::None` is NOT a release. Bevy raises it for two
            // events this arm cannot tell apart on its own: the pointer came UP
            // over this row, or the pointer LEFT the row while still held. A
            // finger that presses near a short row's edge and slides a few pixels
            // off it is the second one, and activating there fires a choice under
            // a finger that never lifted.
            //
            //  the live input state is the discriminator, and it is the same one
            // `ui_focus_system` uses to force `Pressed` → `None`: a finger came up
            // this frame. A MOUSE release over the row it pressed reports
            // `Hovered` (see above), so a mouse `None` is always the pointer
            // leaving — which is why only a touch lift activates here.
            Interaction::None => {
                if pointer_up != PointerUp::Touch {
                    dialogue.row_press.left(index);
                    continue;
                }
                if dialogue.row_press.release(index, pointer_position) == Some(index) {
                    dialogue.selected_option = index;
                    dialogue.confirm_or_advance();
                    return;
                }
            }
        }
    }
}

#[cfg(not(feature = "input"))]
pub fn dialog_pointer_input() {}

/// Whether a pointer actually came UP this frame, and which one.
///
/// Bevy's `Interaction` carries no release event, so every activation rule here
/// has to ask the live input state instead. The two answers are not
/// interchangeable: a mouse release over the row it pressed arrives as
/// `Hovered`, a finger lift arrives as `None`.
#[cfg(feature = "input")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointerUp {
    /// A finger came up this frame and none is left down.
    Touch,
    /// A mouse button was released this frame.
    Mouse,
    /// Whatever moved this row, it was not a pointer coming up.
    Nothing,
    /// Neither backend is present to ask.
    Unknown,
}

#[cfg(feature = "input")]
impl PointerUp {
    /// Did a pointer come up — treating "cannot say" as yes.
    ///
    ///  this is the direction [`ambition_ui_nav::RowPress::origin`] already argues for: *no
    /// position is not evidence of a drag*.
    ///
    ///  the `Interaction::None` arm deliberately does NOT use this. There,
    /// "cannot say" resolves the other way, and for a reason rather than a
    /// convention: no `Touches` means no touch backend, so no finger can have
    /// lifted, so the only pointer that could have produced `None` is a mouse
    /// leaving the row.
    fn came_up(self) -> bool {
        !matches!(self, Self::Nothing)
    }
}

/// Classify the frame's release edge from the live input resources.
///
/// Both arguments are `None` when the backend is absent, which is a different
/// answer from "present and quiet" — see [`PointerUp::came_up`].
#[cfg(feature = "input")]
fn resolve_pointer_up(
    touches: Option<&Touches>,
    mouse_buttons: Option<&ButtonInput<MouseButton>>,
) -> PointerUp {
    //  the lift must be the LAST finger up. Bevy's own focus system forces
    // every pressed node to `None` on any `any_just_released()` frame, so with a
    // second finger still down that `None` says nothing about the row's own
    // finger. Requiring the screen to be empty keeps a multi-touch frame from
    // activating a row nobody released.
    let touch_lifted = touches.map(|touches| {
        touches.iter_just_released().next().is_some() && touches.iter().next().is_none()
    });
    let mouse_released = mouse_buttons.map(|buttons| buttons.get_just_released().next().is_some());

    if touch_lifted == Some(true) {
        PointerUp::Touch
    } else if mouse_released == Some(true) {
        PointerUp::Mouse
    } else if touch_lifted.is_none() && mouse_released.is_none() {
        PointerUp::Unknown
    } else {
        PointerUp::Nothing
    }
}

/// Resolve the configured pointer policy for the device that actually issued
/// the interaction.
#[cfg(feature = "input")]
fn effective_dialog_tap_mode(
    configured: MenuTapMode,
    _active_input: Option<ActiveDevice>,
) -> MenuTapMode {
    // Device type does not override the configured tap policy. Drag rejection is
    // handled by the press/release gesture state.
    configured
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
    use bevy::app::{App, Update};
    use bevy::input::mouse::MouseButtonInput;
    use bevy::input::touch::{TouchInput, TouchPhase};
    use bevy::input::{ButtonState, InputPlugin};
    use bevy::prelude::Entity;

    /// The real system, driven through Bevy's real input plugin.
    ///
    ///  `Touches` and `ButtonInput` are populated by `InputPlugin`, not by
    /// hand. The whole rule under test turns on what those resources report on
    /// the frame a pointer comes up, and a fixture that set the flags itself
    /// would be asserting this test's model of Bevy rather than Bevy. What the
    /// plugin actually reports on a release frame, observed: the finger is gone
    /// from `pressed` and present in `just_released`, and the mouse button reads
    /// `just_released` with `pressed` already false.
    ///
    /// `Interaction` IS set by hand — `ui_focus_system` needs a laid-out UI tree
    /// and a camera, and the contract this system owns is its response to a given
    /// interaction plus the live input state, not the layout that produced it.
    struct Rows {
        app: App,
        rows: Vec<Entity>,
        window: Entity,
    }

    impl Rows {
        fn new(count: usize) -> Self {
            let mut app = App::new();
            app.add_plugins(InputPlugin);
            app.insert_resource(dialogue_with_options(count));
            app.add_systems(Update, dialog_pointer_input);
            let window = app.world_mut().spawn_empty().id();
            let rows = (0..count)
                .map(|index| {
                    app.world_mut()
                        .spawn((Interaction::None, DialogChoiceSlot { index }))
                        .id()
                })
                .collect();
            let mut rows = Self { app, rows, window };
            // Settle the spawn-time `Changed<Interaction>` on every row.
            rows.step();
            rows
        }

        /// Force a row's interaction. Always marks it changed, which is what the
        /// system's `Changed<Interaction>` filter reads.
        fn interaction(&mut self, row: usize, interaction: Interaction) -> &mut Self {
            let entity = self.rows[row];
            *self
                .app
                .world_mut()
                .get_mut::<Interaction>(entity)
                .expect("row keeps its Interaction") = interaction;
            self
        }

        fn touch(&mut self, phase: TouchPhase, position: Vec2) -> &mut Self {
            self.finger(1, phase, position)
        }

        /// A second finger, somewhere else on the screen.
        fn other_finger(&mut self, phase: TouchPhase) -> &mut Self {
            self.finger(2, phase, Vec2::new(600.0, 40.0))
        }

        fn finger(&mut self, id: u64, phase: TouchPhase, position: Vec2) -> &mut Self {
            let window = self.window;
            self.app.world_mut().write_message(TouchInput {
                phase,
                position,
                window,
                force: None,
                id,
            });
            self
        }

        fn mouse(&mut self, state: ButtonState) -> &mut Self {
            let window = self.window;
            self.app.world_mut().write_message(MouseButtonInput {
                button: MouseButton::Left,
                state,
                window,
            });
            self
        }

        fn step(&mut self) -> &mut Self {
            self.app.update();
            self
        }

        fn chosen(&self) -> Option<usize> {
            self.app.world().resource::<DialogState>().pending_select
        }
    }

    /// A tap is down THEN up, and it is still one tap.
    ///
    /// The guarantee the release rule is not allowed to cost: a finger that
    /// presses a row and lifts on it chooses that row, without a second tap.
    #[test]
    fn a_finger_that_presses_and_lifts_on_a_row_chooses_it_in_one_tap() {
        let mut rows = Rows::new(3);
        rows.touch(TouchPhase::Started, Vec2::new(100.0, 100.0))
            .interaction(1, Interaction::Pressed)
            .step();
        assert_eq!(
            rows.chosen(),
            None,
            "the press only arms — a finger that is still down has chosen nothing"
        );

        rows.touch(TouchPhase::Ended, Vec2::new(101.0, 103.0))
            .interaction(1, Interaction::None)
            .step();
        assert_eq!(
            rows.chosen(),
            Some(1),
            "the lift completes the tap; a thumb rolling a few pixels is still a tap"
        );
    }

    /// A whole tap can land inside one frame, and `ui_focus_system` defers the
    /// `Pressed` → `None` reset for exactly that case, so the release edge is gone
    /// by the time `None` arrives. The press frame has to complete it.
    #[test]
    fn a_tap_that_opens_and_closes_in_one_frame_still_chooses() {
        let mut rows = Rows::new(3);
        rows.touch(TouchPhase::Started, Vec2::new(100.0, 100.0))
            .touch(TouchPhase::Ended, Vec2::new(100.0, 100.0))
            .interaction(2, Interaction::Pressed)
            .step();
        assert_eq!(rows.chosen(), Some(2));
    }

    ///  the defect: `Interaction::None` under a finger that never lifted.
    ///
    /// Dialogue rows are short, so a press near an edge leaves the row after a
    /// slide far smaller than the tap slop — which means the drag threshold
    /// cannot catch this one and only the live touch state can. The press dies at
    /// the leave, and the genuine lift that follows must not resurrect it.
    #[test]
    fn a_finger_that_slides_off_a_row_while_still_down_chooses_nothing() {
        let mut rows = Rows::new(3);
        rows.touch(TouchPhase::Started, Vec2::new(100.0, 100.0))
            .interaction(1, Interaction::Pressed)
            .step();

        // Eight pixels — half the tap slop, and off the row.
        rows.touch(TouchPhase::Moved, Vec2::new(100.0, 108.0))
            .interaction(1, Interaction::None)
            .step();
        assert_eq!(
            rows.chosen(),
            None,
            "the finger is still down, so this `None` is the row being left"
        );

        rows.touch(TouchPhase::Ended, Vec2::new(100.0, 108.0))
            .interaction(1, Interaction::None)
            .step();
        assert_eq!(
            rows.chosen(),
            None,
            "the press was cancelled at the leave; lifting later does not revive it"
        );
    }

    /// A second finger lifting says nothing about this row's finger.
    ///
    /// `ui_focus_system` forces EVERY pressed node to `None` on any frame where
    /// `Touches::any_just_released()` is true, so an unrelated finger coming up
    /// is a real way for a held row to report `None` — the leave, arriving with a
    /// release edge sitting right next to it.
    #[test]
    fn a_second_finger_lifting_does_not_choose_the_row_the_first_is_holding() {
        let mut rows = Rows::new(3);
        rows.touch(TouchPhase::Started, Vec2::new(100.0, 100.0))
            .interaction(1, Interaction::Pressed)
            .step();

        rows.other_finger(TouchPhase::Started).step();
        rows.other_finger(TouchPhase::Ended)
            .interaction(1, Interaction::None)
            .step();
        assert_eq!(
            rows.chosen(),
            None,
            "the row's own finger is still down; somebody else's lift is not its release"
        );
    }

    /// A drag that never leaves the row is still a drag.
    ///
    /// Scrolling a list starts on whatever row the thumb landed on, and on a tall
    /// row the gesture can run its whole length without the row ever reporting
    /// `None`. Only the travel threshold catches that one — and it can only catch
    /// it if the press is measured against the FINGER, since a touch device never
    /// writes the window cursor.
    #[test]
    fn a_finger_that_drags_the_length_of_a_row_before_lifting_chooses_nothing() {
        let mut rows = Rows::new(3);
        rows.touch(TouchPhase::Started, Vec2::new(100.0, 100.0))
            .interaction(1, Interaction::Pressed)
            .step();

        rows.touch(TouchPhase::Moved, Vec2::new(100.0, 300.0))
            .step();
        rows.touch(TouchPhase::Ended, Vec2::new(100.0, 300.0))
            .interaction(1, Interaction::None)
            .step();
        assert_eq!(rows.chosen(), None);
    }

    /// A mouse release over the row it pressed arrives as `Hovered` — so
    /// `Hovered` is where a click completes, and a `Hovered` with no release
    /// behind it is just the cursor moving and must not spend the press.
    #[test]
    fn a_click_completes_on_hovered_and_a_bare_hover_does_not_spend_the_press() {
        let mut rows = Rows::new(3);
        rows.mouse(ButtonState::Pressed)
            .interaction(0, Interaction::Pressed)
            .step();

        rows.interaction(0, Interaction::Hovered).step();
        assert_eq!(
            rows.chosen(),
            None,
            "the button is still down — nothing was released"
        );

        rows.mouse(ButtonState::Released)
            .interaction(0, Interaction::Hovered)
            .step();
        assert_eq!(
            rows.chosen(),
            Some(0),
            "the arm survived the bare hover and the real release completes it"
        );
    }

    /// A mouse `Interaction::None` is never a release. A release over the row
    /// reports `Hovered`, so `None` on a mouse-release frame means the button came
    /// up somewhere else — the classic "drag off the button to cancel".
    #[test]
    fn a_mouse_release_away_from_the_row_chooses_nothing() {
        let mut rows = Rows::new(3);
        rows.mouse(ButtonState::Pressed)
            .interaction(2, Interaction::Pressed)
            .step();

        rows.mouse(ButtonState::Released)
            .interaction(2, Interaction::None)
            .step();
        assert_eq!(rows.chosen(), None);
    }

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

    /// Touch keeps the configured policy — and drag safety comes from RELEASE.
    ///
    /// That promotion existed only because activation happened on PRESS, so a finger that
    /// landed and slid had already chosen.
    ///
    ///  rewritten rather than deleted, because the guarantee it protected is
    /// still owed — a drag must not activate. That is now `RowPress`'s job, and
    /// the second half of this test is where it is asserted, so removing the
    /// promotion cannot quietly remove the safety with it.
    #[test]
    fn touch_keeps_its_configured_tap_policy_and_a_drag_still_cannot_activate() {
        for kind in [ActiveDevice::Touch, ActiveDevice::Mouse] {
            for configured in [
                MenuTapMode::SingleTapWithDestructiveGuard,
                MenuTapMode::SingleTap,
                MenuTapMode::TapToSelectThenConfirm,
            ] {
                assert_eq!(
                    effective_dialog_tap_mode(configured, Some(kind)),
                    configured,
                    "{kind:?} must not have a tap policy imposed on it — a game \
                     that wants two-tap says so in its configured mode"
                );
            }
        }

        let mut press = ambition_ui_nav::RowPress::default();
        press.press(2, Some(bevy::prelude::Vec2::new(100.0, 100.0)));
        assert_eq!(
            press.release(2, Some(bevy::prelude::Vec2::new(101.0, 102.0))),
            Some(2),
            "a thumb rolls a pixel or two on a real tap and that is still a tap"
        );

        let mut dragged = ambition_ui_nav::RowPress::default();
        dragged.press(2, Some(bevy::prelude::Vec2::new(100.0, 100.0)));
        assert_eq!(
            dragged.release(2, Some(bevy::prelude::Vec2::new(100.0, 400.0))),
            None,
            "a finger that pressed a row and then scrolled the list chose nothing"
        );

        let mut wandered = ambition_ui_nav::RowPress::default();
        wandered.press(2, Some(bevy::prelude::Vec2::new(100.0, 100.0)));
        assert_eq!(
            wandered.release(5, Some(bevy::prelude::Vec2::new(100.0, 100.0))),
            None,
            "releasing over a DIFFERENT row than the one pressed activates neither"
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
