//! Which input source is CURRENTLY active — the last one to produce
//! GENUINE input.
//!
//! This is a *marker*, not a mode switch. No source is ever disabled:
//! a player may steer with the arrow keys while clicking buttons with
//! the mouse (mouse-as-touchpad), or push a gamepad stick while tapping
//! keyboard shortcuts. [`ActiveInputKind`] only records the most recent
//! genuine source so specific behaviours can be gated on it — most
//! importantly the menu mouse-hover-select, which must NOT re-claim the
//! cursor while the player is navigating with the keyboard / gamepad /
//! touch.
//!
//! ## Why "genuine" matters (the snap-back bug)
//!
//! The menus republish (despawn + respawn their controls) on every
//! cursor move. When a fresh control spawns under a STATIONARY mouse,
//! `bevy_ui` picking fires a `Pointer<Over>` for it. If the hover
//! handler reacted to that, it would snap the cursor back to the mouse
//! on every arrow-key move. The fix: a `Pointer<Over>` is NOT genuine
//! mouse input, so it never flips [`ActiveInputKind`] to `Mouse`. Only a
//! real [`CursorMoved`] (actual pointer motion) or a mouse-button press
//! does. The hover handlers gate on `active == Mouse`, so a
//! rebuild-induced `Over` while the player is on the keyboard is ignored.
//!
//! ## Touch
//!
//! Touch detection lives in the sandbox (it owns the on-screen joystick /
//! [`Touches`] reading), so the sandbox flips this resource to
//! [`ActiveInputKind::Touch`] directly via [`ActiveInputKind::mark`]
//! when the touch stick / touch buttons produce input. The
//! [`update_active_input_kind`] system here covers keyboard / mouse /
//! gamepad, which it can detect from Bevy's own input resources.

use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;
use bevy_window::CursorMoved;

/// The input source that most recently produced GENUINE input.
///
/// Last-writer-wins each frame: whichever source fired real input last
/// (in system order) owns the value. If nothing fires this frame the
/// previous value is kept, so an idle frame never resets the marker.
///
/// Defaults to [`ActiveInputKind::Keyboard`] so a fresh launch (or a
/// headless build with no input resources) behaves as keyboard-first.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActiveInputKind {
    #[default]
    Keyboard,
    Mouse,
    Gamepad,
    Touch,
}

impl ActiveInputKind {
    /// Mark this kind active iff it differs from the current value.
    ///
    /// Used by both the keyboard/mouse/gamepad detector here and the
    /// sandbox-side touch detector. Skipping the no-op write keeps
    /// `Changed<ActiveInputKind>` honest for any future change-gated
    /// reader and avoids needless resource-change churn.
    pub fn mark(&mut self, kind: ActiveInputKind) {
        if *self != kind {
            *self = kind;
        }
    }
}

/// Detect the most recent GENUINE keyboard / mouse / gamepad input and
/// record it in [`ActiveInputKind`].
///
/// Last-writer-wins WITHIN this system: keyboard is checked first, then
/// mouse, then gamepad, so on a frame where (say) both a key and a real
/// cursor move happen, the later-checked source (gamepad > mouse >
/// keyboard) wins. Touch is handled by the sandbox and is not touched
/// here, so a touch-active frame survives this system untouched (none of
/// the three desktop sources fire on a pure-touch frame).
///
/// **`Pointer<Over>` is deliberately NOT consulted** — only a real
/// [`CursorMoved`] or a mouse-button press counts as mouse input, so a
/// rebuild-induced `Over` event under a stationary mouse can never flip
/// the active kind to `Mouse` (the menu snap-back root cause).
///
/// All inputs are `Option<Res<…>>` / drain-free readers so the system is
/// a harmless no-op under `MinimalPlugins` (headless / RL), where Bevy's
/// input resources are absent.
pub fn update_active_input_kind(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    mut cursor_moved: MessageReader<CursorMoved>,
    pads: Query<&Gamepad>,
    mut active: ResMut<ActiveInputKind>,
) {
    // Keyboard: any key newly pressed this frame.
    if let Some(keys) = keys.as_deref() {
        if keys.get_just_pressed().next().is_some() {
            active.mark(ActiveInputKind::Keyboard);
        }
    }

    // Mouse: a REAL cursor move (actual motion) OR a mouse button press.
    // NOT `Pointer<Over>` (that fires on menu rebuild under a stationary
    // mouse and must not flip the active kind — the snap-back bug).
    let real_cursor_motion = cursor_moved.read().next().is_some();
    let mouse_pressed = mouse_buttons
        .as_deref()
        .is_some_and(|b| b.get_just_pressed().next().is_some());
    if real_cursor_motion || mouse_pressed {
        active.mark(ActiveInputKind::Mouse);
    }

    // Gamepad: any button just-pressed OR any axis past a generous
    // deflection. Iterating connected `Gamepad` components matches Bevy
    // 0.18's per-pad input shape.
    const GAMEPAD_AXIS_DEFLECTION: f32 = 0.5;
    for pad in pads.iter() {
        let button = pad.get_just_pressed().next().is_some();
        let axis = pad.get_analog_axes().any(|axis| {
            pad.get(*axis)
                .is_some_and(|v| v.abs() >= GAMEPAD_AXIS_DEFLECTION)
        });
        if button || axis {
            active.mark(ActiveInputKind::Gamepad);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `CursorMoved` requires a window entity; spawn a throwaway one.
    fn dummy_window(app: &mut App) -> Entity {
        app.world_mut().spawn_empty().id()
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<ActiveInputKind>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.add_message::<CursorMoved>();
        app.add_systems(Update, update_active_input_kind);
        app
    }

    fn active(app: &App) -> ActiveInputKind {
        *app.world().resource::<ActiveInputKind>()
    }

    #[test]
    fn defaults_to_keyboard() {
        assert_eq!(ActiveInputKind::default(), ActiveInputKind::Keyboard);
    }

    #[test]
    fn key_press_flips_to_keyboard() {
        let mut app = app();
        // Start from a non-keyboard value so the flip is observable.
        *app.world_mut().resource_mut::<ActiveInputKind>() = ActiveInputKind::Mouse;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        app.update();
        assert_eq!(active(&app), ActiveInputKind::Keyboard);
    }

    #[test]
    fn real_cursor_move_flips_to_mouse() {
        let mut app = app();
        let window = dummy_window(&mut app);
        app.world_mut().write_message(CursorMoved {
            window,
            position: Vec2::new(10.0, 10.0),
            delta: Some(Vec2::new(3.0, 0.0)),
        });
        app.update();
        assert_eq!(active(&app), ActiveInputKind::Mouse);
    }

    #[test]
    fn pointer_over_does_not_flip_to_mouse() {
        // `Pointer<Over>` is an entity-picking event, NOT a `CursorMoved`
        // and NOT a mouse-button press, so this system never reads it.
        // Driving the system with no CursorMoved + no mouse button (the
        // exact state during a rebuild-induced `Over`) must leave the
        // active kind on Keyboard.
        let mut app = app();
        // Sit on Keyboard, then run a frame with nothing but a (simulated)
        // hover — i.e. no genuine mouse input. Active must stay Keyboard.
        app.update();
        assert_eq!(
            active(&app),
            ActiveInputKind::Keyboard,
            "a frame with no CursorMoved / mouse press (the rebuild Over case) keeps the prior kind"
        );
    }

    #[test]
    fn mouse_button_flips_to_mouse() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert_eq!(active(&app), ActiveInputKind::Mouse);
    }

    #[test]
    fn idle_frame_keeps_previous_value() {
        let mut app = app();
        *app.world_mut().resource_mut::<ActiveInputKind>() = ActiveInputKind::Touch;
        app.update();
        assert_eq!(
            active(&app),
            ActiveInputKind::Touch,
            "nothing fired -> the previous (touch) value survives"
        );
    }
}
