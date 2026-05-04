//! Pause menu overlay (UI shell + navigation).
//!
//! `GameMode::Paused` already gates gameplay. This module is the
//! visible side: a translucent overlay with a small action menu and
//! a focused selection that responds to keyboard / gamepad through
//! `SandboxAction`.
//!
//! The menu has two pages:
//!
//! * `Top` — Resume / Settings / Music / Inventory / Quit.
//! * `Settings` — Display Mode / Back. The vocabulary for this page
//!   lives in [`crate::settings`]; this module is only the renderer
//!   and controller.
//!
//! When `audio` is disabled the Music row is replaced with a
//! placeholder and the navigation system uses the audio-free path so
//! `--no-default-features --features input` still compiles.

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::AudioChannel;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

#[cfg(feature = "audio")]
use crate::audio::{switch_to_music_track, AudioLibrary, MusicChannel, MusicPlaybackState};
use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::SandboxAction;
use crate::inventory::InventoryUiState;
use crate::settings::{
    handle_action as handle_settings_action, SettingsAction, SettingsItem, SettingsOutcome,
    SettingsView,
};
use crate::windowing::DisplayModeState;

/// Re-export the settings-row component so other modules that want to
/// query menu rows by tag don't need to remember which module owns it.
pub use crate::settings::SettingsItem as MenuSettingsItem;

#[derive(Component)]
pub struct PauseMenuRoot;

#[derive(Component)]
pub struct PauseMenuTopPanel;

#[derive(Component)]
pub struct PauseMenuSettingsPanel;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PauseMenuPage {
    #[default]
    Top,
    Settings,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseMenuItem {
    Resume,
    Settings,
    MusicTrack,
    Inventory,
    Quit,
}

impl PauseMenuItem {
    pub fn static_label(self) -> &'static str {
        match self {
            Self::Resume => "Resume",
            Self::Settings => "Settings",
            Self::MusicTrack => "Music",
            Self::Inventory => "Inventory",
            Self::Quit => "Quit to Desktop",
        }
    }

    #[cfg(feature = "audio")]
    pub fn label(
        self,
        music_state: Option<&MusicPlaybackState>,
        library: Option<&AudioLibrary>,
    ) -> String {
        match self {
            Self::MusicTrack => {
                let display = music_state
                    .zip(library)
                    .map(|(state, library)| state.active_display_name(library))
                    .unwrap_or("Unavailable");
                format!("Music: {display}  < / >")
            }
            _ => self.static_label().to_string(),
        }
    }

    /// Audio-off label: Music row stays visible (so menu indices match)
    /// but its current value collapses to a placeholder.
    #[cfg(not(feature = "audio"))]
    pub fn label(self) -> String {
        match self {
            Self::MusicTrack => "Music: <audio disabled>".into(),
            _ => self.static_label().to_string(),
        }
    }

    pub const ALL: [Self; 5] = [
        Self::Resume,
        Self::Settings,
        Self::MusicTrack,
        Self::Inventory,
        Self::Quit,
    ];
}

#[derive(Resource, Default)]
pub struct PauseMenuState {
    /// Selected index inside the active page. Reset on page enter so the
    /// cursor lands on the first row consistently.
    pub selected: usize,
    pub page: PauseMenuPage,
}

impl PauseMenuState {
    fn enter_page(&mut self, page: PauseMenuPage) {
        self.page = page;
        self.selected = 0;
    }
}

/// `MenuToggle` input opens/closes the pause menu by toggling `GameMode`.
#[cfg(feature = "input")]
pub fn pause_menu_toggle(
    action_state: Query<&ActionState<SandboxAction>>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut state: ResMut<PauseMenuState>,
    mut inventory: ResMut<InventoryUiState>,
) {
    let Ok(actions) = action_state.single() else {
        return;
    };
    if !actions.just_pressed(&SandboxAction::Start) {
        return;
    }
    match mode.get() {
        GameMode::Playing => {
            state.enter_page(PauseMenuPage::Top);
            next_mode.set(GameMode::Paused);
        }
        GameMode::Paused => {
            inventory.visible = false;
            next_mode.set(GameMode::Playing);
        }
        _ => {}
    }
}

/// Compact navigation actions decoded from the leafwing `ActionState`.
/// Sharing this type between the audio-on and audio-off navigators
/// keeps the menu logic in one place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct NavInput {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    confirm: bool,
}

#[cfg(feature = "input")]
fn read_nav_input(actions: &ActionState<SandboxAction>) -> NavInput {
    NavInput {
        up: actions.just_pressed(&SandboxAction::MoveUp),
        down: actions.just_pressed(&SandboxAction::MoveDown),
        left: actions.just_pressed(&SandboxAction::MoveLeft),
        right: actions.just_pressed(&SandboxAction::MoveRight),
        confirm: actions.just_pressed(&SandboxAction::Jump),
    }
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub fn pause_menu_navigate(
    action_state: Query<&ActionState<SandboxAction>>,
    mode: Res<State<GameMode>>,
    mut state: ResMut<PauseMenuState>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut inventory: ResMut<InventoryUiState>,
    mut exit: MessageWriter<AppExit>,
    mut display_state: ResMut<DisplayModeState>,
    windows: Query<&mut Window, With<PrimaryWindow>>,
    #[cfg(feature = "audio")] library: Res<AudioLibrary>,
    #[cfg(feature = "audio")] mut music_state: ResMut<MusicPlaybackState>,
    #[cfg(feature = "audio")] music_channel: Res<AudioChannel<MusicChannel>>,
) {
    if !matches!(mode.get(), GameMode::Paused) {
        return;
    }
    if inventory.visible {
        return;
    }
    let Ok(actions) = action_state.single() else {
        return;
    };
    let nav = read_nav_input(actions);

    match state.page {
        PauseMenuPage::Top => {
            handle_top_input(
                nav,
                &mut state,
                &mut next_mode,
                &mut inventory,
                &mut exit,
                #[cfg(feature = "audio")]
                &library,
                #[cfg(feature = "audio")]
                &mut music_state,
                #[cfg(feature = "audio")]
                &music_channel,
            );
        }
        PauseMenuPage::Settings => {
            handle_settings_input(nav, &mut state, &mut display_state, windows);
        }
    }
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn handle_top_input(
    nav: NavInput,
    state: &mut PauseMenuState,
    next_mode: &mut NextState<GameMode>,
    inventory: &mut InventoryUiState,
    exit: &mut MessageWriter<AppExit>,
    #[cfg(feature = "audio")] library: &AudioLibrary,
    #[cfg(feature = "audio")] music_state: &mut MusicPlaybackState,
    #[cfg(feature = "audio")] music_channel: &AudioChannel<MusicChannel>,
) {
    let items = PauseMenuItem::ALL;
    if nav.up {
        state.selected = (state.selected + items.len() - 1) % items.len();
    }
    if nav.down {
        state.selected = (state.selected + 1) % items.len();
    }

    let item = items[state.selected];

    #[cfg(feature = "audio")]
    if item == PauseMenuItem::MusicTrack {
        let next_track = if nav.left {
            library.previous_track_id(&music_state.active_track)
        } else if nav.right {
            library.next_track_id(&music_state.active_track)
        } else {
            None
        };
        if let Some(next_track) = next_track.map(str::to_string) {
            switch_to_music_track(library, music_state, music_channel, &next_track);
        }
    }

    if nav.confirm {
        match item {
            PauseMenuItem::Resume => {
                inventory.visible = false;
                next_mode.set(GameMode::Playing);
            }
            PauseMenuItem::Settings => {
                state.enter_page(PauseMenuPage::Settings);
            }
            PauseMenuItem::MusicTrack => {
                #[cfg(feature = "audio")]
                {
                    if let Some(next_track) = library
                        .next_track_id(&music_state.active_track)
                        .map(str::to_string)
                    {
                        switch_to_music_track(library, music_state, music_channel, &next_track);
                    }
                }
            }
            PauseMenuItem::Inventory => {
                inventory.visible = true;
                inventory.selected = 0;
                inventory.opened_from_pause = true;
            }
            PauseMenuItem::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

#[cfg(feature = "input")]
fn handle_settings_input(
    nav: NavInput,
    state: &mut PauseMenuState,
    display_state: &mut DisplayModeState,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let items = SettingsItem::ALL;
    if nav.up {
        state.selected = (state.selected + items.len() - 1) % items.len();
    }
    if nav.down {
        state.selected = (state.selected + 1) % items.len();
    }

    let item = items[state.selected];
    let action = if nav.left {
        Some(SettingsAction::Prev)
    } else if nav.right {
        Some(SettingsAction::Next)
    } else if nav.confirm {
        Some(SettingsAction::Confirm)
    } else {
        None
    };
    if let Some(action) = action {
        let outcome = handle_settings_action(item, action, display_state, &mut windows);
        if matches!(outcome, SettingsOutcome::Back) {
            state.enter_page(PauseMenuPage::Top);
        }
    }
}

pub fn spawn_pause_menu(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.78)),
            ZIndex(50),
            Visibility::Hidden,
            PauseMenuRoot,
            Name::new("Pause menu"),
        ))
        .id();

    let top_panel = commands
        .spawn((
            Node {
                width: Val::Px(360.0),
                padding: UiRect::all(Val::Px(28.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.94)),
            BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.85)),
            PauseMenuTopPanel,
            Name::new("Pause panel — top"),
        ))
        .id();
    commands.entity(root).add_child(top_panel);

    let title = commands
        .spawn((
            Text::new("Paused"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgba(0.92, 0.96, 1.0, 0.98)),
            Name::new("Pause title"),
        ))
        .id();
    commands.entity(top_panel).add_child(title);

    for item in PauseMenuItem::ALL {
        let label = item.static_label();
        let entity = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Text::new(label),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96)),
                item,
                Name::new(format!("Pause item: {label}")),
            ))
            .id();
        commands.entity(top_panel).add_child(entity);
    }

    let settings_panel = commands
        .spawn((
            Node {
                width: Val::Px(380.0),
                padding: UiRect::all(Val::Px(28.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.0),
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.94)),
            BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.85)),
            PauseMenuSettingsPanel,
            Name::new("Pause panel — settings"),
        ))
        .id();
    commands.entity(root).add_child(settings_panel);

    let settings_title = commands
        .spawn((
            Text::new("Settings"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgba(0.92, 0.96, 1.0, 0.98)),
            Name::new("Settings title"),
        ))
        .id();
    commands.entity(settings_panel).add_child(settings_title);

    for item in SettingsItem::ALL {
        let label = item.static_label();
        let entity = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Text::new(label),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96)),
                item,
                Name::new(format!("Settings item: {label}")),
            ))
            .id();
        commands.entity(settings_panel).add_child(entity);
    }
}

/// Show/hide the pause overlay based on `GameMode` and update item highlights.
#[cfg(feature = "audio")]
#[allow(clippy::too_many_arguments)]
pub fn sync_pause_menu(
    mode: Res<State<GameMode>>,
    state: Res<PauseMenuState>,
    inventory: Res<InventoryUiState>,
    library: Res<AudioLibrary>,
    music_state: Res<MusicPlaybackState>,
    display_state: Res<DisplayModeState>,
    mut roots: Query<&mut Visibility, With<PauseMenuRoot>>,
    mut top_panels: Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    mut settings_panels: Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
    mut top_items: Query<
        (
            &PauseMenuItem,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        Without<SettingsItem>,
    >,
    mut settings_items: Query<
        (
            &SettingsItem,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        Without<PauseMenuItem>,
    >,
) {
    let visible = matches!(mode.get(), GameMode::Paused) && !inventory.visible;
    for mut visibility in &mut roots {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        return;
    }

    apply_page_visibility(state.page, &mut top_panels, &mut settings_panels);
    let view = SettingsView::from_state(&display_state);
    if matches!(state.page, PauseMenuPage::Top) {
        let selected_item = PauseMenuItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut top_items {
            **text = item.label(Some(&music_state), Some(&library));
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
    } else {
        let selected_item = SettingsItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut settings_items {
            **text = item.label(&view);
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
    }
}

#[cfg(not(feature = "audio"))]
#[allow(clippy::too_many_arguments)]
pub fn sync_pause_menu(
    mode: Res<State<GameMode>>,
    state: Res<PauseMenuState>,
    inventory: Res<InventoryUiState>,
    display_state: Res<DisplayModeState>,
    mut roots: Query<&mut Visibility, With<PauseMenuRoot>>,
    mut top_panels: Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    mut settings_panels: Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
    mut top_items: Query<
        (
            &PauseMenuItem,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        Without<SettingsItem>,
    >,
    mut settings_items: Query<
        (
            &SettingsItem,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        Without<PauseMenuItem>,
    >,
) {
    let visible = matches!(mode.get(), GameMode::Paused) && !inventory.visible;
    for mut visibility in &mut roots {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        return;
    }
    apply_page_visibility(state.page, &mut top_panels, &mut settings_panels);
    let view = SettingsView::from_state(&display_state);
    if matches!(state.page, PauseMenuPage::Top) {
        let selected_item = PauseMenuItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut top_items {
            **text = item.label();
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
    } else {
        let selected_item = SettingsItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut settings_items {
            **text = item.label(&view);
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
    }
}

fn apply_page_visibility(
    page: PauseMenuPage,
    top_panels: &mut Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    settings_panels: &mut Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
) {
    let on_top = matches!(page, PauseMenuPage::Top);
    for mut node in &mut *top_panels {
        node.display = if on_top { Display::Flex } else { Display::None };
    }
    for mut node in &mut *settings_panels {
        node.display = if on_top { Display::None } else { Display::Flex };
    }
}

fn apply_item_highlight(color: &mut TextColor, bg: &mut BackgroundColor, is_selected: bool) {
    *color = if is_selected {
        TextColor(Color::srgba(0.18, 0.06, 0.04, 1.0))
    } else {
        TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96))
    };
    *bg = if is_selected {
        BackgroundColor(Color::srgba(0.95, 0.78, 0.32, 0.96))
    } else {
        BackgroundColor(Color::NONE)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_menu_state_default_is_top_page_zero() {
        let s = PauseMenuState::default();
        assert!(matches!(s.page, PauseMenuPage::Top));
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn enter_page_resets_selected_to_zero() {
        let mut s = PauseMenuState {
            selected: 3,
            page: PauseMenuPage::Top,
        };
        s.enter_page(PauseMenuPage::Settings);
        assert!(matches!(s.page, PauseMenuPage::Settings));
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn pause_menu_item_all_includes_settings() {
        assert!(PauseMenuItem::ALL.contains(&PauseMenuItem::Settings));
    }

    /// `MenuSettingsItem` is the public re-export so other modules can
    /// query rows by tag without crossing the private boundary.
    #[test]
    fn menu_settings_item_is_settings_item() {
        let _ = MenuSettingsItem::DisplayMode;
    }
}
