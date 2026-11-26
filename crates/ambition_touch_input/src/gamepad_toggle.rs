//! Q178: the small button that hides and restores the virtual gamepad.
//!
//! The gamepad is not hidden because a menu takes direct touch: menus' touch
//! handling is often incomplete, and the gamepad stays usable for menu
//! navigation. A player who wants it out of the way presses this button, at
//! the top left of the safe area, beside (never over) the HUD and the other
//! controls; pressing it again brings the gamepad back. The setting **Toggle Gamepad
//! Button** says when the button shows (`ToggleGamepadButton`). Whatever it
//! says, a hidden gamepad always has its button: hiding can never remove the
//! way back, and choosing "never visible" brings a hidden gamepad back.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use ambition_persistence::settings::controls::ToggleGamepadButton;
use ambition_persistence::settings::UserSettings;
use ambition_platformer2d_shared_tangle::gameplay_presentation::{
    ResolvedGameplayPresentation, ScreenOcclusion, ScreenOcclusionPurpose, ScreenRect,
};

/// The player hid the virtual gamepad with its toggle. Session-local: a new
/// run starts with the gamepad shown.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GamepadHiddenByToggle(pub bool);

/// Where the button was drawn last frame; `None` while it is not shown, so
/// only a drawn button takes a press.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct GamepadToggleRect(pub Option<ScreenRect>);

/// The toggle button's root. It is not a `MobileTouchUiRoot`, so hiding the
/// gamepad does not hide it.
#[derive(Component)]
pub struct GamepadToggleButton;

/// Its size and its margin from the safe area's top-left corner, in px.
pub const TOGGLE_SIZE: f32 = 40.0;
pub const TOGGLE_MARGIN: f32 = 8.0;

/// Whether the button shows: by the setting, over a menu or dialogue in
/// `Auto`, and always while the gamepad is hidden. Nothing shows when the
/// touch overlay itself is off.
pub fn toggle_shown(setting: ToggleGamepadButton, overlay_on: bool, hidden: bool, ui_owns_controls: bool) -> bool {
    if !overlay_on { return false; }
    if hidden { return true; }
    match setting {
        ToggleGamepadButton::Always => true,
        ToggleGamepadButton::Never => false,
        ToggleGamepadButton::Auto => ui_owns_controls,
    }
}

/// Where the button goes: the safe area's top-left corner, moved right past
/// whatever already occupies it (the HUD, the system buttons, the touch
/// clusters), and below them when the row is full to the safe area's middle.
/// Dialogue is not avoided: it comes and goes, and the button must not jump.
pub fn toggle_corner(safe: ScreenRect, occupied: &[ScreenOcclusion]) -> Vec2 {
    let size = Vec2::splat(TOGGLE_SIZE);
    let start = safe.min + Vec2::splat(TOGGLE_MARGIN);
    let blockers: Vec<ScreenRect> = occupied.iter()
        .filter(|occlusion| occlusion.purpose != ScreenOcclusionPurpose::Dialogue)
        .map(|occlusion| occlusion.rect)
        .collect();
    let mut at = start;
    // Each step clears one blocker, so this ends within twice their count.
    for _ in 0..=2 * blockers.len() {
        let candidate = ScreenRect::from_min_size(at, size);
        let Some(blocker) = blockers.iter().find(|rect| rect.overlaps(candidate)) else { return at };
        at.x = blocker.max.x + TOGGLE_MARGIN;
        if at.x + TOGGLE_SIZE > safe.center().x {
            at = Vec2::new(start.x, blocker.max.y + TOGGLE_MARGIN);
        }
    }
    start
}

pub(crate) fn spawn_gamepad_toggle(mut commands: Commands) {
    commands.spawn((
        GamepadToggleButton,
        Button,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(TOGGLE_MARGIN),
            top: Val::Px(TOGGLE_MARGIN),
            width: Val::Px(TOGGLE_SIZE),
            height: Val::Px(TOGGLE_SIZE),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.20, 0.16, 0.22, 0.60)),
        Visibility::Hidden,
        GlobalZIndex(20),
        Name::new("TouchGamepadToggle"),
    )).with_children(|button| {
        // A small gamepad, drawn: the fonts have no gamepad glyph.
        let ink = Color::srgba(0.12, 0.10, 0.14, 1.0);
        let at = |left: f32, top: f32, width: f32, height: f32| Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left), top: Val::Px(top), width: Val::Px(width), height: Val::Px(height),
            ..default()
        };
        button.spawn((
            Node { width: Val::Px(28.0), height: Val::Px(16.0), border_radius: BorderRadius::all(Val::Px(7.0)), ..default() },
            BackgroundColor(Color::srgba(0.92, 0.90, 0.95, 0.95)),
        )).with_children(|pad| {
            pad.spawn((at(4.0, 7.0, 8.0, 2.0), BackgroundColor(ink)));
            pad.spawn((at(7.0, 4.0, 2.0, 8.0), BackgroundColor(ink)));
            for (left, top) in [(18.0, 4.0), (21.0, 8.0)] {
                pad.spawn((
                    Node { border_radius: BorderRadius::all(Val::Px(1.5)), ..at(left, top, 3.0, 3.0) },
                    BackgroundColor(ink),
                ));
            }
        });
    });
}

/// A touch that starts on the drawn button, or a left click on it, toggles
/// the gamepad. Like the overlay's own buttons this hit-tests the raw input
/// against the drawn rect: `Interaction` is unreliable while another finger
/// holds the stick. "Never visible" brings a hidden gamepad back.
pub(crate) fn press_gamepad_toggle(
    settings: Res<UserSettings>,
    drawn: Res<GamepadToggleRect>,
    touches: Option<Res<Touches>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut hidden: ResMut<GamepadHiddenByToggle>,
) {
    let pressed = drawn.0.is_some_and(|rect| {
        let touched = touches.as_deref().is_some_and(|touches| {
            touches.iter_just_pressed().any(|touch| rect.contains(touch.position()))
        });
        let clicked = mouse.as_deref().is_some_and(|mouse| mouse.just_pressed(MouseButton::Left))
            && windows.single().ok().and_then(Window::cursor_position).is_some_and(|cursor| rect.contains(cursor));
        touched || clicked
    });
    let next = if settings.controls.toggle_gamepad_button == ToggleGamepadButton::Never { false }
        else if pressed { !hidden.0 } else { hidden.0 };
    if hidden.0 != next { hidden.0 = next; }
}

/// Place the button by [`toggle_corner`] and show it by [`toggle_shown`].
pub(crate) fn place_gamepad_toggle(
    settings: Res<UserSettings>,
    hidden: Res<GamepadHiddenByToggle>,
    active_context: Option<Res<ambition_input::SeatInputContexts>>,
    presentation: Option<Res<ResolvedGameplayPresentation>>,
    mut buttons: Query<(&mut Node, &mut Visibility), With<GamepadToggleButton>>,
    mut drawn: ResMut<GamepadToggleRect>,
) {
    let ui_owns_controls = active_context.as_deref().is_some_and(|seats| !seats.primary().gameplay_owned());
    let shown = toggle_shown(settings.controls.toggle_gamepad_button, settings.controls.touch_controls_visible,
        hidden.0, ui_owns_controls);
    let corner = presentation.as_deref().map_or(Vec2::splat(TOGGLE_MARGIN), |presentation| {
        toggle_corner(presentation.display_safe_rect, &presentation.occlusions)
    });
    let target = if shown { Visibility::Inherited } else { Visibility::Hidden };
    let rect = shown.then(|| ScreenRect::from_min_size(corner, Vec2::splat(TOGGLE_SIZE)));
    if drawn.0 != rect { drawn.0 = rect; }
    for (mut node, mut visibility) in &mut buttons {
        visibility.set_if_neq(target);
        let (left, top) = (Val::Px(corner.x), Val::Px(corner.y));
        if node.left != left { node.left = left; }
        if node.top != top { node.top = top; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rule: Always and Never as named, Auto over a UI that owns the
    /// controls; a hidden gamepad always has its button; with the overlay off
    /// there is nothing to toggle.
    #[test]
    fn the_toggle_shows_by_its_setting_and_always_while_the_gamepad_is_hidden() {
        use ToggleGamepadButton::*;
        assert!(toggle_shown(Always, true, false, false));
        assert!(!toggle_shown(Never, true, false, true));
        assert!(!toggle_shown(Auto, true, false, false), "Auto in gameplay: out of the way");
        assert!(toggle_shown(Auto, true, false, true), "Auto over a menu or dialogue");
        for setting in ToggleGamepadButton::ALL {
            assert!(toggle_shown(setting, true, true, false), "{setting:?}: a hidden gamepad keeps its way back");
            assert!(!toggle_shown(setting, false, true, true), "{setting:?}: no overlay, no toggle");
        }
    }

    /// The corner when it is free; beside the HUD when the HUD holds it; below
    /// a band that fills the row; a dialogue box is not dodged.
    #[test]
    fn the_toggle_sits_beside_what_already_holds_the_corner() {
        let safe = ScreenRect::from_min_size(Vec2::new(0.0, 20.0), Vec2::new(640.0, 340.0));
        let occlusion = |purpose, min: Vec2, max: Vec2| ScreenOcclusion { purpose, rect: ScreenRect::from_corners(min, max) };
        assert_eq!(toggle_corner(safe, &[]), Vec2::new(8.0, 28.0));
        let hud = occlusion(ScreenOcclusionPurpose::PersistentHud, Vec2::new(10.0, 30.0), Vec2::new(185.0, 95.0));
        assert_eq!(toggle_corner(safe, &[hud]), Vec2::new(193.0, 28.0));
        let band = occlusion(ScreenOcclusionPurpose::PersistentHud, Vec2::new(0.0, 20.0), Vec2::new(400.0, 60.0));
        assert_eq!(toggle_corner(safe, &[band]), Vec2::new(8.0, 68.0));
        let dialogue = occlusion(ScreenOcclusionPurpose::Dialogue, Vec2::ZERO, Vec2::new(640.0, 360.0));
        assert_eq!(toggle_corner(safe, &[dialogue]), Vec2::new(8.0, 28.0));
        let placed = toggle_corner(safe, &[hud, band]);
        let rect = ScreenRect::from_min_size(placed, Vec2::splat(TOGGLE_SIZE));
        assert!(!rect.overlaps(hud.rect) && !rect.overlaps(band.rect), "{placed:?}");
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin);
        app.init_resource::<UserSettings>();
        app.init_resource::<GamepadHiddenByToggle>();
        app.init_resource::<GamepadToggleRect>();
        app.add_systems(Update, (press_gamepad_toggle, place_gamepad_toggle).chain());
        app.add_systems(Startup, spawn_gamepad_toggle);
        app
    }

    fn touch(app: &mut App, at: Vec2, phase: bevy::input::touch::TouchPhase) {
        app.world_mut().write_message(bevy::input::touch::TouchInput {
            phase, position: at, window: Entity::PLACEHOLDER, force: None, id: 7,
        });
        app.update();
    }

    /// A finger down and up on the button's centre.
    fn press(app: &mut App) {
        let centre = Vec2::splat(TOGGLE_MARGIN + TOGGLE_SIZE * 0.5);
        touch(app, centre, bevy::input::touch::TouchPhase::Started);
        touch(app, centre, bevy::input::touch::TouchPhase::Ended);
    }

    /// A touch on the button hides the gamepad and the button stays; a second
    /// brings it back; a touch beside it does nothing. Choosing "never
    /// visible" while it is hidden brings it back.
    #[test]
    fn a_press_hides_the_gamepad_and_the_next_restores_it() {
        let mut app = app();
        app.world_mut().resource_mut::<UserSettings>().controls.toggle_gamepad_button = ToggleGamepadButton::Always;
        app.update();
        press(&mut app);
        assert!(app.world().resource::<GamepadHiddenByToggle>().0);
        let mut q = app.world_mut().query_filtered::<&Visibility, With<GamepadToggleButton>>();
        assert_eq!(*q.single(app.world()).unwrap(), Visibility::Inherited, "its way back is shown");
        press(&mut app);
        assert!(!app.world().resource::<GamepadHiddenByToggle>().0);
        // A finger that lands beside the button does nothing.
        touch(&mut app, Vec2::splat(TOGGLE_MARGIN + TOGGLE_SIZE + 4.0), bevy::input::touch::TouchPhase::Started);
        touch(&mut app, Vec2::splat(TOGGLE_MARGIN + TOGGLE_SIZE + 4.0), bevy::input::touch::TouchPhase::Ended);
        assert!(!app.world().resource::<GamepadHiddenByToggle>().0);
        press(&mut app);
        assert!(app.world().resource::<GamepadHiddenByToggle>().0);
        app.world_mut().resource_mut::<UserSettings>().controls.toggle_gamepad_button = ToggleGamepadButton::Never;
        app.update();
        assert!(!app.world().resource::<GamepadHiddenByToggle>().0, "never visible brings it back");
        assert_eq!(*q.single(app.world()).unwrap(), Visibility::Hidden);
    }
}
