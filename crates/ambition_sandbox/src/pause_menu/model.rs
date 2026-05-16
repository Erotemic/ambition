use super::*;

/// Mutable bundle of the resources the Developer settings page toggles.
/// Packed into a single `SystemParam` so `pause_menu_navigate` stays
/// under Bevy's 16-param budget.
#[derive(SystemParam)]
pub struct DevToggleParams<'w, 's> {
    pub dev_state: ResMut<'w, SandboxDevState>,
    pub developer: ResMut<'w, DeveloperTools>,
    pub editable_tuning: ResMut<'w, crate::dev_tools::EditableMovementTuning>,
    pub ldtk_reload: ResMut<'w, LdtkHotReloadState>,
    pub player_q: Query<'w, 's, &'static mut crate::player::PlayerMovementAuthority, With<crate::player::PlayerEntity>>,
}

/// Read-only counterpart used by `sync_pause_menu` to sample the
/// developer-toggle snapshot for label rendering.
#[derive(SystemParam)]
pub struct DevToggleView<'w> {
    pub dev_state: Res<'w, SandboxDevState>,
    pub developer: Res<'w, DeveloperTools>,
    pub ldtk_reload: Res<'w, LdtkHotReloadState>,
}

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

pub(super) const SETTINGS_VISIBLE_ROWS: usize = 6;
/// Max row slots pre-spawned in the settings/radio panel. Hidden slots toggle
/// `Display::None` so they don't reserve layout — the panel sizes to the
/// actual visible row count. Bump only if a single page needs more
/// simultaneously visible rows than this cap.
pub(super) const MAX_ROWS: usize = 32;
/// Radio keeps a phone-friendly row count and windows through the full catalog.
pub(super) const RADIO_VISIBLE_ROWS: usize = 8;

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
    pub(super) fn enter_page(&mut self, page: PauseMenuPage) {
        if self.page != page {
            self.stack.push(self.page);
            self.page = page;
            self.selected = 0;
        }
    }

    pub(super) fn pop_page(&mut self) {
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
