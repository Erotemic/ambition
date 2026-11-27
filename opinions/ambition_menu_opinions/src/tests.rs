//! Open the optional pages through the shell and edit live preferences.

use ambition_game_shell::{
    ActiveGameplaySession, ShellAbandonRequested, ShellCommand, ShellHostConfiguration,
    ShellPauseMenu, ShellPauseMenuPlugin, ShellRouter,
};
use ambition_input::MenuControlFrame;
use ambition_persistence::settings::UserSettings;
use bevy::prelude::*;

fn pause_app() -> App {
    let mut app = App::new();
    app.init_resource::<MenuControlFrame>()
        .insert_resource(ActiveGameplaySession(None))
        .init_resource::<ShellRouter>()
        .init_resource::<ShellHostConfiguration>()
        .add_plugins(ShellPauseMenuPlugin)
        .add_message::<ShellCommand>()
        .add_message::<ShellAbandonRequested>();
    app
}

fn intent(app: &mut App, set: impl FnOnce(&mut MenuControlFrame)) {
    let mut frame = MenuControlFrame::default();
    set(&mut frame);
    app.insert_resource(frame);
    app.update();
    app.insert_resource(MenuControlFrame::default());
}

#[cfg(feature = "video")]
#[test]
fn the_pause_video_page_edits_only_the_selected_option() {
    use crate::{VideoMenuOption, VideoPausePage};

    let mut app = pause_app();
    app.add_plugins(VideoPausePage(vec![VideoMenuOption::Vsync]));
    let before = app.world().resource::<UserSettings>().video.clone();

    intent(&mut app, |frame| frame.start = true);
    intent(&mut app, |frame| frame.select = true);
    intent(&mut app, |frame| frame.down = true);
    intent(&mut app, |frame| frame.right = true);

    let video = &app.world().resource::<UserSettings>().video;
    assert_ne!(video.vsync, before.vsync);
    assert_eq!(video.view_shape, before.view_shape);
    assert_eq!(video.frame_cap, before.frame_cap);

    intent(&mut app, |frame| {
        frame.start = true;
        frame.back = true;
    });
    let menu = app.world().resource::<ShellPauseMenu>();
    assert!(menu.open);
    assert_eq!(menu.page(), None);
}

#[cfg(feature = "rebind")]
#[test]
fn the_pause_rebind_page_captures_the_next_raw_press() {
    use ambition_input::{
        bind, publish_seat_bindings, BindingDefaults, BindingScheme, InputAction, InputParticipant,
        InputProfile, OverrideControl, PresetId, SeatBindings,
    };

    let preset = PresetId::ArrowsZxc;
    let recipe = BindingDefaults::new(vec![(
        preset,
        BindingScheme {
            buttons: vec![bind(
                InputProfile::Gameplay,
                OverrideControl::Key(KeyCode::KeyZ),
                InputAction::Jump,
            )],
            ..default()
        },
    )])
    .recipe(preset);
    let mut app = pause_app();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<SeatBindings>()
        .add_systems(PreUpdate, publish_seat_bindings)
        .add_plugins(crate::RebindPausePage);
    app.world_mut()
        .spawn((InputParticipant::primary(), recipe.build(), recipe));

    intent(&mut app, |frame| frame.start = true);
    intent(&mut app, |frame| frame.select = true);
    intent(&mut app, |frame| frame.down = true);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    intent(&mut app, |frame| frame.select = true);
    assert!(app
        .world()
        .resource::<UserSettings>()
        .controls
        .binding_overrides
        .is_empty());

    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.clear();
    keys.press(KeyCode::Escape);
    intent(&mut app, |frame| {
        frame.start = true;
        frame.back = true;
    });
    let settings = app.world().resource::<UserSettings>();
    assert_eq!(settings.controls.binding_overrides.len(), 1);
    assert_eq!(settings.controls.binding_overrides[0].action, "Jump");
    assert_eq!(
        settings.controls.binding_overrides[0].control,
        OverrideControl::Key(KeyCode::Escape)
    );
    assert_eq!(
        app.world().resource::<ShellPauseMenu>().page(),
        Some("rebind")
    );
}
