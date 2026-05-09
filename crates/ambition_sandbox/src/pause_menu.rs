//! Pause menu overlay (UI shell + navigation).
//!
//! `GameMode::Paused` already gates gameplay. This module is the
//! visible side: a translucent overlay with a small action menu and a
//! focused selection that responds to keyboard / gamepad through the
//! `Menu*` actions on `crate::input::SandboxAction`.
//!
//! The menu is structured as a stack of pages (`SettingsPage`). The
//! top page lists Resume / Settings / Music / Inventory / Quit; the
//! Settings entry pushes onto a category page (Video / Audio /
//! Controls / Gameplay), which then push to the actual setting rows.
//!
//! When `audio` is disabled the Music row is replaced with a
//! placeholder and the navigation system uses the audio-free path so
//! `--no-default-features --features input` still compiles.

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
#[cfg(feature = "audio")]
use bevy_kira_audio::prelude::AudioChannel;

#[cfg(feature = "audio")]
use crate::audio::{
    set_radio_track, AudioLibrary, MusicChannel, MusicPlaybackState, RadioStationState,
};
use crate::game_mode::GameMode;
use crate::input::{KeyboardPreset, MenuControlFrame, MenuInputFrame};
use crate::inventory::InventoryUiState;
use crate::settings::{
    apply_action as handle_settings_action, MenuPointerPress, SettingsAction, SettingsItem,
    SettingsOutcome, SettingsPage, UserSettings,
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

/// Marker placed on every row entity inside the settings panel so the
/// renderer can rebuild row text from `SettingsItem::label`.
#[derive(Component, Clone, Copy, Debug)]
pub struct SettingsRowSlot {
    pub index: usize,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseMenuItem {
    Resume,
    Settings,
    MusicTrack,
    Inventory,
    /// Wipe the persisted save + rebuild the runtime — every NPC
    /// alive again, every encounter armed, player back at the start
    /// room. Triggered via `crate::reset::SandboxResetRequested`.
    ResetSandbox,
    Quit,
}

impl PauseMenuItem {
    pub fn static_label(self) -> &'static str {
        match self {
            Self::Resume => "Resume",
            Self::Settings => "Settings",
            Self::MusicTrack => "Radio",
            Self::Inventory => "Inventory",
            Self::ResetSandbox => "Reset Sandbox",
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
                format!("Radio: {display}")
            }
            _ => self.static_label().to_string(),
        }
    }

    /// Audio-off label: Music row stays visible (so menu indices match)
    /// but its current value collapses to a placeholder.
    #[cfg(not(feature = "audio"))]
    pub fn label(self) -> String {
        match self {
            Self::MusicTrack => "Radio: <audio disabled>".into(),
            _ => self.static_label().to_string(),
        }
    }

    pub const ALL: [Self; 6] = [
        Self::Resume,
        Self::Settings,
        Self::MusicTrack,
        Self::Inventory,
        Self::ResetSandbox,
        Self::Quit,
    ];

    /// Items that need a confirm tap under
    /// `MenuTapMode::SingleTapWithDestructiveGuard` so a stray touch
    /// can't wipe the save or exit the game.
    pub fn is_destructive(self) -> bool {
        matches!(self, Self::ResetSandbox | Self::Quit)
    }
}

/// Active page on the pause overlay. The pause overlay starts on
/// `Top`; entering Settings transitions through the settings page
/// stack (Top → Video / Audio / Controls / Gameplay).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PauseMenuPage {
    #[default]
    Top,
    Settings(SettingsPage),
    Radio,
}

const SETTINGS_VISIBLE_ROWS: usize = 6;
const RADIO_VISIBLE_ROWS: usize = 6;

#[derive(Resource, Default)]
pub struct PauseMenuState {
    pub selected: usize,
    pub page: PauseMenuPage,
    /// Stack of pages we can pop back to. The current page is NOT in
    /// this stack; it is the live `page` field.
    pub stack: Vec<PauseMenuPage>,
    /// Set to `Some(index)` when a pointer press selected a row that
    /// requires a confirmation tap (destructive item under guard mode,
    /// or any item under tap-then-confirm mode). Cleared when the user
    /// taps a different row, navigates with kbd/gamepad, or confirms.
    pub pointer_armed: Option<usize>,
    /// Set by the pointer system on a click that should activate the
    /// currently selected row. Consumed by the navigate system on the
    /// same frame and folded into `MenuInputFrame.select`.
    pub pointer_confirm: bool,
}

impl PauseMenuState {
    fn enter_page(&mut self, page: PauseMenuPage) {
        if self.page != page {
            self.stack.push(self.page);
            self.page = page;
            self.selected = 0;
        }
    }

    fn pop_page(&mut self) {
        if let Some(prev) = self.stack.pop() {
            self.page = prev;
            self.selected = 0;
        } else {
            // Already at root — close the menu (caller decides).
            self.page = PauseMenuPage::Top;
            self.selected = 0;
        }
    }
}

/// `Start` input opens/closes the pause menu by toggling `GameMode`.
///
/// Reads from `MenuControlFrame`, the semantic menu-input seam. Keyboard,
/// gamepad, mouse/touch buttons, and touch gestures all fold into that
/// resource before this system runs.
#[cfg(feature = "input")]
pub fn pause_menu_toggle(
    menu: Res<MenuControlFrame>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut state: ResMut<PauseMenuState>,
    mut inventory: ResMut<InventoryUiState>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !menu.start {
        return;
    }
    match mode.get() {
        GameMode::Playing => {
            state.page = PauseMenuPage::Top;
            state.selected = 0;
            state.stack.clear();
            next_mode.set(GameMode::Paused);
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::UI_PAUSE_OPEN,
                pos: ambition_engine::Vec2::ZERO,
            });
        }
        GameMode::Paused => {
            inventory.visible = false;
            next_mode.set(GameMode::Playing);
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::UI_PAUSE_CLOSE,
                pos: ambition_engine::Vec2::ZERO,
            });
        }
        _ => {}
    }
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub fn pause_menu_navigate(
    menu: Res<MenuControlFrame>,
    mode: Res<State<GameMode>>,
    mut state: ResMut<PauseMenuState>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut inventory: ResMut<InventoryUiState>,
    mut exit: MessageWriter<AppExit>,
    mut display_state: ResMut<DisplayModeState>,
    mut user_settings: ResMut<UserSettings>,
    mut reset_request: ResMut<crate::reset::SandboxResetRequested>,
    windows: Query<&mut Window, With<PrimaryWindow>>,
    #[cfg(feature = "audio")] library: Res<AudioLibrary>,
    #[cfg(feature = "audio")] mut music_state: ResMut<MusicPlaybackState>,
    #[cfg(feature = "audio")] mut radio: ResMut<RadioStationState>,
    #[cfg(feature = "audio")] music_channel: Res<AudioChannel<MusicChannel>>,
) {
    if !matches!(mode.get(), GameMode::Paused) {
        return;
    }
    if inventory.visible {
        return;
    }
    let mut frame = MenuInputFrame {
        up: menu.up,
        down: menu.down,
        left: menu.left,
        right: menu.right,
        select: menu.select,
        back: menu.back,
        start: menu.start,
    };
    apply_vertical_scroll(&mut frame, menu.vertical_scroll_steps());

    // Fold pointer-driven confirms into the frame, and clear any
    // armed pointer state when the user navigates with kbd / gamepad
    // (touching a different row already cleared/replaced it).
    if state.pointer_confirm {
        frame.select = true;
        state.pointer_confirm = false;
    }
    if frame.any_directional() || frame.back || menu.scroll_y.abs() >= 0.5 {
        state.pointer_armed = None;
    }

    let preset_count = KeyboardPreset::presets().len();

    // MenuBack always pops; if we're already at Top it closes the menu.
    if frame.back {
        match state.page {
            PauseMenuPage::Top => {
                inventory.visible = false;
                next_mode.set(GameMode::Playing);
            }
            PauseMenuPage::Settings(SettingsPage::Top) | PauseMenuPage::Radio => {
                state.page = PauseMenuPage::Top;
                state.selected = 0;
                state.stack.clear();
            }
            _ => {
                state.pop_page();
            }
        }
        return;
    }

    match state.page {
        PauseMenuPage::Top => {
            handle_top_input(
                frame,
                &mut state,
                &mut next_mode,
                &mut inventory,
                &mut exit,
                &mut reset_request,
                #[cfg(feature = "audio")]
                &library,
                #[cfg(feature = "audio")]
                &mut music_state,
                #[cfg(feature = "audio")]
                &mut radio,
                #[cfg(feature = "audio")]
                &music_channel,
            );
        }
        PauseMenuPage::Settings(page) => {
            handle_settings_page_input(
                frame,
                page,
                &mut state,
                &mut user_settings,
                &mut display_state,
                windows,
                preset_count,
            );
        }
        PauseMenuPage::Radio => {
            #[cfg(feature = "audio")]
            handle_radio_input(
                frame,
                &mut state,
                &library,
                &mut radio,
                &mut music_state,
                &music_channel,
            );
            #[cfg(not(feature = "audio"))]
            {
                if frame.back || frame.select {
                    state.pop_page();
                }
            }
        }
    }
}

fn apply_vertical_scroll(frame: &mut MenuInputFrame, steps: i32) {
    if steps > 0 {
        frame.up = true;
    } else if steps < 0 {
        frame.down = true;
    }
}

fn visible_window_start(selected: usize, total: usize, capacity: usize) -> usize {
    if total <= capacity || capacity == 0 {
        return 0;
    }
    let half = capacity / 2;
    let start = selected.saturating_sub(half);
    start.min(total - capacity)
}

fn visible_row_index(
    slot_index: usize,
    selected: usize,
    total: usize,
    capacity: usize,
) -> Option<usize> {
    if total == 0 || slot_index >= capacity {
        return None;
    }
    let start = visible_window_start(selected, total, capacity);
    let absolute = start + slot_index;
    (absolute < total).then_some(absolute)
}

fn windowed_title(base: &str, selected: usize, total: usize, capacity: usize) -> String {
    if total > capacity {
        format!(
            "{base} — {}/{}",
            selected.min(total.saturating_sub(1)) + 1,
            total
        )
    } else {
        base.to_string()
    }
}

fn indexed_title(base: &str, selected: usize, total: usize) -> String {
    if total > 1 {
        format!(
            "{base} — {}/{}",
            selected.min(total.saturating_sub(1)) + 1,
            total
        )
    } else {
        base.to_string()
    }
}

fn decorate_windowed_label(
    label: String,
    index: usize,
    selected: usize,
    total: usize,
    capacity: usize,
) -> String {
    if total <= capacity || capacity == 0 {
        return label;
    }
    let start = visible_window_start(selected, total, capacity);
    let end = (start + capacity).min(total);
    let prefix = if index == start && start > 0 {
        "↑ "
    } else {
        "  "
    };
    let suffix = if index + 1 == end && end < total {
        " ↓"
    } else {
        ""
    };
    format!("{prefix}{label}{suffix}")
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn handle_top_input(
    nav: MenuInputFrame,
    state: &mut PauseMenuState,
    next_mode: &mut NextState<GameMode>,
    inventory: &mut InventoryUiState,
    exit: &mut MessageWriter<AppExit>,
    reset_request: &mut crate::reset::SandboxResetRequested,
    #[cfg(feature = "audio")] library: &AudioLibrary,
    #[cfg(feature = "audio")] music_state: &mut MusicPlaybackState,
    #[cfg(feature = "audio")] radio: &mut RadioStationState,
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
        let active = radio
            .selected_track()
            .unwrap_or(music_state.active_track.as_str());
        let next_track = if nav.left {
            library.previous_track_id(active)
        } else if nav.right {
            library.next_track_id(active)
        } else {
            None
        };
        if let Some(next_track) = next_track.map(str::to_string) {
            set_radio_track(library, radio, music_state, music_channel, &next_track);
        }
    }

    if nav.select {
        match item {
            PauseMenuItem::Resume => {
                inventory.visible = false;
                next_mode.set(GameMode::Playing);
            }
            PauseMenuItem::Settings => {
                state.enter_page(PauseMenuPage::Settings(SettingsPage::Top));
            }
            PauseMenuItem::MusicTrack => {
                #[cfg(feature = "audio")]
                {
                    state.enter_page(PauseMenuPage::Radio);
                    let active = radio
                        .selected_track()
                        .unwrap_or(music_state.active_track.as_str());
                    state.selected = library.track_index(active).unwrap_or(0);
                }
                #[cfg(not(feature = "audio"))]
                {
                    state.enter_page(PauseMenuPage::Radio);
                    state.selected = 0;
                }
            }
            PauseMenuItem::Inventory => {
                inventory.visible = true;
                inventory.selected = 0;
                inventory.opened_from_pause = true;
            }
            PauseMenuItem::ResetSandbox => {
                // Queue the reset and return to gameplay so the
                // processor system can run on the next frame. The
                // banner ("SANDBOX RESET") confirms the action.
                reset_request.request();
                inventory.visible = false;
                next_mode.set(GameMode::Playing);
            }
            PauseMenuItem::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

#[cfg(all(feature = "input", feature = "audio"))]
fn handle_radio_input(
    nav: MenuInputFrame,
    state: &mut PauseMenuState,
    library: &AudioLibrary,
    radio: &mut RadioStationState,
    music_state: &mut MusicPlaybackState,
    music_channel: &AudioChannel<MusicChannel>,
) {
    let count = library.track_count();
    if count == 0 {
        return;
    }
    if nav.up {
        state.selected = (state.selected + count - 1) % count;
    }
    if nav.down {
        state.selected = (state.selected + 1) % count;
    }
    if nav.left {
        state.selected = (state.selected + count - 1) % count;
    }
    if nav.right {
        state.selected = (state.selected + 1) % count;
    }
    if state.selected >= count {
        state.selected = 0;
    }
    if nav.select || nav.left || nav.right {
        if let Some(track) = library.track_at(state.selected) {
            let track_id = track.id.clone();
            set_radio_track(library, radio, music_state, music_channel, &track_id);
        }
    }
}

#[cfg(feature = "input")]
fn handle_settings_page_input(
    nav: MenuInputFrame,
    page: SettingsPage,
    state: &mut PauseMenuState,
    user_settings: &mut UserSettings,
    display_state: &mut DisplayModeState,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    preset_count: usize,
) {
    let rows = SettingsItem::rows_for(page);
    if rows.is_empty() {
        return;
    }
    if nav.up {
        state.selected = (state.selected + rows.len() - 1) % rows.len();
    }
    if nav.down {
        state.selected = (state.selected + 1) % rows.len();
    }
    if state.selected >= rows.len() {
        state.selected = 0;
    }
    let item = rows[state.selected];

    let action = if nav.left {
        Some(SettingsAction::Prev)
    } else if nav.right {
        Some(SettingsAction::Next)
    } else if nav.select {
        Some(SettingsAction::Confirm)
    } else {
        None
    };
    if let Some(action) = action {
        let outcome = handle_settings_action(
            item,
            action,
            user_settings,
            display_state,
            &mut windows,
            preset_count,
        );
        match outcome {
            SettingsOutcome::Stay => {}
            SettingsOutcome::OpenPage(next_page) => {
                state.enter_page(PauseMenuPage::Settings(next_page));
            }
            SettingsOutcome::PopPage => {
                state.pop_page();
            }
        }
    }
}

/// Mouse / touch input for the pause menu and its settings sub-pages.
///
/// Hover (mouse-over) moves the highlight; press routes through
/// `MenuTapMode::resolve_press` to decide whether to also confirm.
/// Confirms are deferred to `pause_menu_navigate` via
/// `state.pointer_confirm` so the rest of the menu pipeline keeps a
/// single confirm path.
#[cfg(feature = "input")]
pub fn pause_menu_pointer_input(
    mode: Res<State<GameMode>>,
    inventory: Res<InventoryUiState>,
    user_settings: Res<UserSettings>,
    mut state: ResMut<PauseMenuState>,
    top_items: Query<(&Interaction, &PauseMenuItem), Changed<Interaction>>,
    settings_rows: Query<(&Interaction, &SettingsRowSlot), Changed<Interaction>>,
    #[cfg(feature = "audio")] library: Res<AudioLibrary>,
) {
    if !matches!(mode.get(), GameMode::Paused) {
        return;
    }
    if inventory.visible {
        return;
    }
    let tap_mode = user_settings.controls.menu_tap_mode;

    match state.page {
        PauseMenuPage::Top => {
            let items = PauseMenuItem::ALL;
            for (interaction, item) in &top_items {
                let Some(index) = items.iter().position(|i| i == item) else {
                    continue;
                };
                match interaction {
                    Interaction::Hovered => {
                        // Mouse hover: just move the highlight. Don't
                        // disturb the armed-confirm state — the user
                        // may be hovering past a destructive item.
                        if state.selected != index {
                            state.selected = index;
                        }
                    }
                    Interaction::Pressed => {
                        let press = tap_mode.resolve_press(
                            index,
                            state.selected,
                            item.is_destructive(),
                            &mut state.pointer_armed,
                        );
                        state.selected = index;
                        if matches!(press, MenuPointerPress::Confirm) {
                            state.pointer_confirm = true;
                        }
                    }
                    Interaction::None => {}
                }
            }
        }
        PauseMenuPage::Settings(page) => {
            let rows = SettingsItem::rows_for(page);
            for (interaction, slot) in &settings_rows {
                let Some(index) = visible_row_index(
                    slot.index,
                    state.selected,
                    rows.len(),
                    SETTINGS_VISIBLE_ROWS,
                ) else {
                    continue;
                };
                handle_row_pointer_interaction(interaction, index, tap_mode, &mut state);
            }
        }
        PauseMenuPage::Radio => {
            #[cfg(feature = "audio")]
            let row_count = library.track_count();
            #[cfg(not(feature = "audio"))]
            let row_count = 1;
            for (interaction, slot) in &settings_rows {
                let Some(index) =
                    visible_row_index(slot.index, state.selected, row_count, RADIO_VISIBLE_ROWS)
                else {
                    continue;
                };
                handle_row_pointer_interaction(interaction, index, tap_mode, &mut state);
            }
        }
    }
}

fn handle_row_pointer_interaction(
    interaction: &Interaction,
    index: usize,
    tap_mode: crate::settings::MenuTapMode,
    state: &mut PauseMenuState,
) {
    match interaction {
        Interaction::Hovered => {
            if state.selected != index {
                state.selected = index;
            }
        }
        Interaction::Pressed => {
            let press =
                tap_mode.resolve_press(index, state.selected, false, &mut state.pointer_armed);
            state.selected = index;
            if matches!(press, MenuPointerPress::Confirm) {
                state.pointer_confirm = true;
            }
        }
        Interaction::None => {}
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
                padding: UiRect::all(Val::Px(14.0)),
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
                width: Val::Px(400.0),
                max_width: Val::Percent(92.0),
                max_height: Val::Percent(94.0),
                padding: UiRect::all(Val::Px(18.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
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
                font_size: 25.0,
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
                Button,
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(34.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Text::new(label),
                TextFont {
                    font_size: 19.0,
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
                width: Val::Px(500.0),
                max_width: Val::Percent(94.0),
                max_height: Val::Percent(94.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
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
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgba(0.92, 0.96, 1.0, 0.98)),
            SettingsTitle,
            Name::new("Settings title"),
        ))
        .id();
    commands.entity(settings_panel).add_child(settings_title);

    // Pre-spawn enough slot rows to hold the largest page. Each frame
    // the renderer fills `slot.index < rows.len()` slots with text and
    // hides the rest. This avoids respawning UI nodes per page swap,
    // which can cost a frame of layout instability.
    const MAX_ROWS: usize = 24;
    for index in 0..MAX_ROWS {
        let entity = commands
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(30.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Text::new(""),
                TextFont {
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96)),
                Visibility::Hidden,
                SettingsRowSlot { index },
                Name::new(format!("Settings row slot {index}")),
            ))
            .id();
        commands.entity(settings_panel).add_child(entity);
    }
}

#[derive(Component)]
pub struct SettingsTitle;

/// Show/hide the pause overlay based on `GameMode` and update item highlights.
#[cfg(feature = "audio")]
#[allow(clippy::too_many_arguments)]
pub fn sync_pause_menu(
    mode: Res<State<GameMode>>,
    state: Res<PauseMenuState>,
    inventory: Res<InventoryUiState>,
    library: Res<AudioLibrary>,
    music_state: Res<MusicPlaybackState>,
    radio: Res<RadioStationState>,
    user_settings: Res<UserSettings>,
    mut roots: Query<&mut Visibility, With<PauseMenuRoot>>,
    mut top_panels: Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    mut settings_panels: Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
    mut titles: Query<(&mut Text, &SettingsTitle), Without<SettingsRowSlot>>,
    mut top_items: Query<
        (
            &PauseMenuItem,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        (Without<SettingsRowSlot>, Without<SettingsTitle>),
    >,
    mut row_slots: Query<
        (
            &SettingsRowSlot,
            &mut Visibility,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        (
            Without<PauseMenuRoot>,
            Without<PauseMenuItem>,
            Without<SettingsTitle>,
        ),
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
    apply_page_visibility(on_top, &mut top_panels, &mut settings_panels);
    if matches!(state.page, PauseMenuPage::Top) {
        let selected_item = PauseMenuItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut top_items {
            **text = item.label(Some(&music_state), Some(&library));
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
        // Hide all settings rows.
        for (_, mut vis, _, _, _) in &mut row_slots {
            *vis = Visibility::Hidden;
        }
    } else if let PauseMenuPage::Settings(page) = state.page {
        let rows = SettingsItem::rows_for(page);
        for (mut text, _) in &mut titles {
            **text = windowed_title(
                page.title(),
                state.selected,
                rows.len(),
                SETTINGS_VISIBLE_ROWS,
            );
        }
        for (slot, mut vis, mut text, mut color, mut bg) in &mut row_slots {
            if let Some(row_index) = visible_row_index(
                slot.index,
                state.selected,
                rows.len(),
                SETTINGS_VISIBLE_ROWS,
            ) {
                if let Some(item) = rows.get(row_index) {
                    **text = decorate_windowed_label(
                        item.label(&user_settings),
                        row_index,
                        state.selected,
                        rows.len(),
                        SETTINGS_VISIBLE_ROWS,
                    );
                    let selected = state.selected == row_index;
                    apply_item_highlight(&mut color, &mut bg, selected);
                    *vis = Visibility::Visible;
                }
            } else {
                *vis = Visibility::Hidden;
            }
        }
    } else if matches!(state.page, PauseMenuPage::Radio) {
        let count = library.track_count();
        for (mut text, _) in &mut titles {
            **text = indexed_title("Radio", state.selected, count);
        }
        let active = radio
            .selected_track()
            .unwrap_or(music_state.active_track.as_str());
        for (slot, mut vis, mut text, mut color, mut bg) in &mut row_slots {
            if let Some(track_index) =
                visible_row_index(slot.index, state.selected, count, RADIO_VISIBLE_ROWS)
            {
                if let Some(label) = library.radio_label(track_index, active) {
                    **text = decorate_windowed_label(
                        label,
                        track_index,
                        state.selected,
                        count,
                        RADIO_VISIBLE_ROWS,
                    );
                    let selected = state.selected == track_index;
                    apply_item_highlight(&mut color, &mut bg, selected);
                    *vis = Visibility::Visible;
                }
            } else {
                *vis = Visibility::Hidden;
            }
        }
    }
}

#[cfg(not(feature = "audio"))]
#[allow(clippy::too_many_arguments)]
pub fn sync_pause_menu(
    mode: Res<State<GameMode>>,
    state: Res<PauseMenuState>,
    inventory: Res<InventoryUiState>,
    user_settings: Res<UserSettings>,
    mut roots: Query<&mut Visibility, With<PauseMenuRoot>>,
    mut top_panels: Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    mut settings_panels: Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
    mut titles: Query<(&mut Text, &SettingsTitle), Without<SettingsRowSlot>>,
    mut top_items: Query<
        (
            &PauseMenuItem,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        (Without<SettingsRowSlot>, Without<SettingsTitle>),
    >,
    mut row_slots: Query<
        (
            &SettingsRowSlot,
            &mut Visibility,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        (
            Without<PauseMenuRoot>,
            Without<PauseMenuItem>,
            Without<SettingsTitle>,
        ),
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
    apply_page_visibility(on_top, &mut top_panels, &mut settings_panels);
    if matches!(state.page, PauseMenuPage::Top) {
        let selected_item = PauseMenuItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut top_items {
            **text = item.label();
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
        for (_, mut vis, _, _, _) in &mut row_slots {
            *vis = Visibility::Hidden;
        }
    } else if let PauseMenuPage::Settings(page) = state.page {
        let rows = SettingsItem::rows_for(page);
        for (mut text, _) in &mut titles {
            **text = windowed_title(
                page.title(),
                state.selected,
                rows.len(),
                SETTINGS_VISIBLE_ROWS,
            );
        }
        for (slot, mut vis, mut text, mut color, mut bg) in &mut row_slots {
            if let Some(row_index) = visible_row_index(
                slot.index,
                state.selected,
                rows.len(),
                SETTINGS_VISIBLE_ROWS,
            ) {
                if let Some(item) = rows.get(row_index) {
                    **text = decorate_windowed_label(
                        item.label(&user_settings),
                        row_index,
                        state.selected,
                        rows.len(),
                        SETTINGS_VISIBLE_ROWS,
                    );
                    let selected = state.selected == row_index;
                    apply_item_highlight(&mut color, &mut bg, selected);
                    *vis = Visibility::Visible;
                }
            } else {
                *vis = Visibility::Hidden;
            }
        }
    } else if matches!(state.page, PauseMenuPage::Radio) {
        for (mut text, _) in &mut titles {
            **text = "Radio".to_string();
        }
        for (slot, mut vis, mut text, mut color, mut bg) in &mut row_slots {
            if slot.index == 0 {
                **text = "Audio feature disabled".to_string();
                apply_item_highlight(&mut color, &mut bg, state.selected == 0);
                *vis = Visibility::Visible;
            } else {
                *vis = Visibility::Hidden;
            }
        }
    }
}

fn apply_page_visibility(
    on_top: bool,
    top_panels: &mut Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    settings_panels: &mut Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
) {
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
    fn enter_page_pushes_onto_stack() {
        let mut s = PauseMenuState {
            selected: 3,
            page: PauseMenuPage::Top,
            stack: Vec::new(),
            pointer_armed: None,
            pointer_confirm: false,
        };
        s.enter_page(PauseMenuPage::Settings(SettingsPage::Top));
        assert!(matches!(s.page, PauseMenuPage::Settings(SettingsPage::Top)));
        assert_eq!(s.selected, 0);
        assert_eq!(s.stack.len(), 1);
        s.pop_page();
        assert!(matches!(s.page, PauseMenuPage::Top));
        assert!(s.stack.is_empty());
    }

    #[test]
    fn pause_menu_item_all_includes_settings() {
        assert!(PauseMenuItem::ALL.contains(&PauseMenuItem::Settings));
    }

    /// `ResetSandbox` is the user-facing entry point for the
    /// "wipe the save and rebuild the runtime" flow. Pin it here so
    /// a future menu-shape refactor can't silently drop it.
    #[test]
    fn pause_menu_item_all_includes_reset_sandbox() {
        assert!(PauseMenuItem::ALL.contains(&PauseMenuItem::ResetSandbox));
        assert_eq!(PauseMenuItem::ResetSandbox.static_label(), "Reset Sandbox");
    }

    /// `MenuSettingsItem` is the public re-export so other modules can
    /// query rows by tag without crossing the private boundary.
    #[test]
    fn menu_settings_item_is_settings_item() {
        let _ = MenuSettingsItem::DisplayMode;
    }

    #[test]
    fn visible_window_tracks_selected_row_without_overflow() {
        assert_eq!(visible_window_start(0, 12, 5), 0);
        assert_eq!(visible_window_start(4, 12, 5), 2);
        assert_eq!(visible_window_start(11, 12, 5), 7);
        assert_eq!(visible_row_index(0, 11, 12, 5), Some(7));
        assert_eq!(visible_row_index(4, 11, 12, 5), Some(11));
        assert_eq!(visible_row_index(5, 11, 12, 5), None);
    }
}
