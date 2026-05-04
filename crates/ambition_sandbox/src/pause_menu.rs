//! Pause menu overlay.
//!
//! The existing `GameMode::Paused` state already gates gameplay (input, sim,
//! and feature updates short-circuit when not in `Playing`). This module
//! adds the visible side: a translucent overlay with a small action menu
//! and a focused selection that responds to both keyboard and gamepad
//! navigation through the existing `SandboxAction` input map.
//!
//! The menu has two pages:
//!
//! * `Top` — Resume / Settings / Music / Inventory / Quit.
//! * `Settings` — Display Mode / Back. New settings rows go here.
//!
//! Adding a new settings option:
//!
//! 1. Add a variant to `SettingsItem`.
//! 2. Add it to `SettingsItem::ALL` (the navigation order).
//! 3. Implement `label` so the row knows how to render its current value.
//! 4. Handle Left/Right and Confirm in `handle_settings_input`.
//! 5. Add a presentation-side row entity in `spawn_pause_menu` (the
//!    existing loop over `SettingsItem::ALL` already does this; extending
//!    the enum is enough).
//!
//! Settings that affect simulation rules (input bindings, time-scale
//! preferences, etc.) should still live in their own modules; this menu
//! is only the surface that exposes them.

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, VideoModeSelection, WindowMode};
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
use crate::windowing::{DisplayModeKind, DisplayModeState};

/// Top-level entity tagging for the pause overlay.
#[derive(Component)]
pub struct PauseMenuRoot;

/// Tag for the settings sub-panel container; lets `sync_pause_menu`
/// show/hide the right page without despawning.
#[derive(Component)]
pub struct PauseMenuTopPanel;

#[derive(Component)]
pub struct PauseMenuSettingsPanel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseMenuPage {
    Top,
    Settings,
}

impl Default for PauseMenuPage {
    fn default() -> Self {
        Self::Top
    }
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

    #[cfg(not(feature = "audio"))]
    pub fn label(self) -> String {
        self.static_label().to_string()
    }

    pub const ALL: [Self; 5] = [
        Self::Resume,
        Self::Settings,
        Self::MusicTrack,
        Self::Inventory,
        Self::Quit,
    ];
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsItem {
    DisplayMode,
    Back,
}

impl SettingsItem {
    pub const ALL: [Self; 2] = [Self::DisplayMode, Self::Back];

    pub fn static_label(self) -> &'static str {
        match self {
            Self::DisplayMode => "Display Mode",
            Self::Back => "Back",
        }
    }

    /// Render the row text including the current value when relevant.
    /// Adding a new settings row that exposes a value: extend the match
    /// here with a `format!("Label: {value}  < / >")` so users know
    /// Left/Right cycles the value.
    pub fn label(self, display_mode: DisplayModeKind) -> String {
        match self {
            Self::DisplayMode => format!("Display Mode: {}  < / >", display_mode.label()),
            Self::Back => self.static_label().to_string(),
        }
    }
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
/// Runs before `sandbox_update` consumes the start press so the gameplay
/// loop's existing toggle path stays disabled while the menu is the
/// authoritative driver of pause/resume.
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
    let toggle = actions.just_pressed(&SandboxAction::Start);
    if !toggle {
        return;
    }
    match mode.get() {
        GameMode::Playing => {
            state.enter_page(PauseMenuPage::Top);
            next_mode.set(GameMode::Paused);
        }
        GameMode::Paused => {
            // Pressing pause again resumes immediately and closes the
            // inventory if it was open from the menu.
            inventory.visible = false;
            next_mode.set(GameMode::Playing);
        }
        _ => {}
    }
}

#[cfg(all(feature = "input", feature = "audio"))]
#[allow(clippy::too_many_arguments)]
pub fn pause_menu_navigate(
    action_state: Query<&ActionState<SandboxAction>>,
    mode: Res<State<GameMode>>,
    mut state: ResMut<PauseMenuState>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut inventory: ResMut<InventoryUiState>,
    mut exit: MessageWriter<AppExit>,
    library: Res<AudioLibrary>,
    mut music_state: ResMut<MusicPlaybackState>,
    music_channel: Res<AudioChannel<MusicChannel>>,
    mut display_state: ResMut<DisplayModeState>,
    windows: Query<&mut Window, With<PrimaryWindow>>,
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

    match state.page {
        PauseMenuPage::Top => handle_top_input(
            actions,
            &mut state,
            &mut next_mode,
            &mut inventory,
            &mut exit,
            &library,
            &mut music_state,
            &music_channel,
        ),
        PauseMenuPage::Settings => {
            handle_settings_input(actions, &mut state, &mut display_state, windows)
        }
    }
}

#[cfg(all(feature = "input", feature = "audio"))]
#[allow(clippy::too_many_arguments)]
fn handle_top_input(
    actions: &ActionState<SandboxAction>,
    state: &mut PauseMenuState,
    next_mode: &mut NextState<GameMode>,
    inventory: &mut InventoryUiState,
    exit: &mut MessageWriter<AppExit>,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    music_channel: &AudioChannel<MusicChannel>,
) {
    let items = PauseMenuItem::ALL;
    if actions.just_pressed(&SandboxAction::MoveUp) {
        state.selected = (state.selected + items.len() - 1) % items.len();
    }
    if actions.just_pressed(&SandboxAction::MoveDown) {
        state.selected = (state.selected + 1) % items.len();
    }

    let item = items[state.selected];
    if item == PauseMenuItem::MusicTrack {
        let next_track = if actions.just_pressed(&SandboxAction::MoveLeft) {
            library.previous_track_id(&music_state.active_track)
        } else if actions.just_pressed(&SandboxAction::MoveRight) {
            library.next_track_id(&music_state.active_track)
        } else {
            None
        };
        if let Some(next_track) = next_track.map(str::to_string) {
            switch_to_music_track(library, music_state, music_channel, &next_track);
        }
    }

    if actions.just_pressed(&SandboxAction::Jump) {
        match item {
            PauseMenuItem::Resume => {
                inventory.visible = false;
                next_mode.set(GameMode::Playing);
            }
            PauseMenuItem::Settings => {
                state.enter_page(PauseMenuPage::Settings);
            }
            PauseMenuItem::MusicTrack => {
                if let Some(next_track) = library
                    .next_track_id(&music_state.active_track)
                    .map(str::to_string)
                {
                    switch_to_music_track(library, music_state, music_channel, &next_track);
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
    actions: &ActionState<SandboxAction>,
    state: &mut PauseMenuState,
    display_state: &mut DisplayModeState,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let items = SettingsItem::ALL;
    if actions.just_pressed(&SandboxAction::MoveUp) {
        state.selected = (state.selected + items.len() - 1) % items.len();
    }
    if actions.just_pressed(&SandboxAction::MoveDown) {
        state.selected = (state.selected + 1) % items.len();
    }

    let item = items[state.selected];
    let mut requested_mode: Option<DisplayModeKind> = None;
    if item == SettingsItem::DisplayMode {
        if actions.just_pressed(&SandboxAction::MoveLeft) {
            requested_mode = Some(prev_display_mode(display_state.mode));
        }
        if actions.just_pressed(&SandboxAction::MoveRight) {
            requested_mode = Some(next_display_mode(display_state.mode));
        }
    }

    if actions.just_pressed(&SandboxAction::Jump) {
        match item {
            SettingsItem::DisplayMode => {
                requested_mode = Some(next_display_mode(display_state.mode));
            }
            SettingsItem::Back => {
                state.enter_page(PauseMenuPage::Top);
            }
        }
    }

    if let Some(mode) = requested_mode {
        apply_display_mode(mode, display_state, &mut windows);
    }
}

fn next_display_mode(current: DisplayModeKind) -> DisplayModeKind {
    match current {
        DisplayModeKind::Windowed => DisplayModeKind::Borderless,
        DisplayModeKind::Borderless => DisplayModeKind::Fullscreen,
        DisplayModeKind::Fullscreen => DisplayModeKind::Windowed,
    }
}

fn prev_display_mode(current: DisplayModeKind) -> DisplayModeKind {
    match current {
        DisplayModeKind::Windowed => DisplayModeKind::Fullscreen,
        DisplayModeKind::Borderless => DisplayModeKind::Windowed,
        DisplayModeKind::Fullscreen => DisplayModeKind::Borderless,
    }
}

/// Apply a `DisplayModeKind` to the primary window. Shared between the
/// settings menu and `crate::windowing::window_mode_hotkeys` so both
/// surfaces produce the same WindowMode mapping.
pub(crate) fn apply_display_mode(
    mode: DisplayModeKind,
    state: &mut DisplayModeState,
    windows: &mut Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    window.mode = match mode {
        DisplayModeKind::Windowed => WindowMode::Windowed,
        DisplayModeKind::Borderless => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        DisplayModeKind::Fullscreen => {
            WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
        }
    };
    state.mode = mode;
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

    let on_top = matches!(state.page, PauseMenuPage::Top);
    for mut node in &mut top_panels {
        node.display = if on_top { Display::Flex } else { Display::None };
    }
    for mut node in &mut settings_panels {
        node.display = if on_top { Display::None } else { Display::Flex };
    }

    if on_top {
        let selected_item = PauseMenuItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut top_items {
            **text = item.label(Some(&music_state), Some(&library));
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
    } else {
        let selected_item = SettingsItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut settings_items {
            **text = item.label(display_state.mode);
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
    }
}

/// Audio-off variant: same visibility logic, but item labels stay static
/// (no music-track display) since the music subsystem is gone.
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
    let on_top = matches!(state.page, PauseMenuPage::Top);
    for mut node in &mut top_panels {
        node.display = if on_top { Display::Flex } else { Display::None };
    }
    for mut node in &mut settings_panels {
        node.display = if on_top { Display::None } else { Display::Flex };
    }
    if on_top {
        let selected_item = PauseMenuItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut top_items {
            **text = item.label();
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
    } else {
        let selected_item = SettingsItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut settings_items {
            **text = item.label(display_state.mode);
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
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
    fn next_display_mode_cycles_forward() {
        assert_eq!(
            next_display_mode(DisplayModeKind::Windowed),
            DisplayModeKind::Borderless
        );
        assert_eq!(
            next_display_mode(DisplayModeKind::Borderless),
            DisplayModeKind::Fullscreen
        );
        assert_eq!(
            next_display_mode(DisplayModeKind::Fullscreen),
            DisplayModeKind::Windowed
        );
    }

    #[test]
    fn prev_display_mode_cycles_backward() {
        assert_eq!(
            prev_display_mode(DisplayModeKind::Windowed),
            DisplayModeKind::Fullscreen
        );
        assert_eq!(
            prev_display_mode(DisplayModeKind::Fullscreen),
            DisplayModeKind::Borderless
        );
        assert_eq!(
            prev_display_mode(DisplayModeKind::Borderless),
            DisplayModeKind::Windowed
        );
    }

    #[test]
    fn settings_item_label_includes_current_value() {
        let label = SettingsItem::DisplayMode.label(DisplayModeKind::Borderless);
        assert!(label.contains("borderless"));
        assert!(SettingsItem::Back
            .label(DisplayModeKind::Windowed)
            .eq("Back"));
    }

    #[test]
    fn pause_menu_item_all_includes_settings() {
        assert!(PauseMenuItem::ALL.contains(&PauseMenuItem::Settings));
    }
}
