//! Host-provided launch catalog and the cursor used by the minimal `ambition_menu` adapter.

use bevy::prelude::{Message, Resource};

use crate::{ShellExperienceId, ShellRouteId};

pub const BASIC_LAUNCHER_EXPERIENCE: &str = "ambition.shell.basic-launcher";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellLaunchEntry {
    pub route_id: ShellRouteId,
    pub label: String,
    pub description: String,
    pub available: bool,
    /// Player-facing reason this entry is disabled, when `available` is false.
    pub unavailable_reason: Option<String>,
}

#[derive(Resource, Default)]
pub struct ShellLaunchCatalog {
    pub entries: Vec<ShellLaunchEntry>,
}

impl ShellLaunchCatalog {
    pub fn register(&mut self, entry: ShellLaunchEntry) -> Option<ShellLaunchEntry> {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|existing| existing.route_id == entry.route_id)
        {
            return Some(std::mem::replace(existing, entry));
        }
        self.entries.push(entry);
        None
    }

    pub fn basic_experience_id() -> ShellExperienceId {
        ShellExperienceId::new(BASIC_LAUNCHER_EXPERIENCE)
    }
}

#[derive(Resource, Clone, Debug, Eq, PartialEq)]
pub struct ShellLauncherPresentation {
    pub title: String,
    pub empty_message: String,
    pub footer: String,
    /// Label of the built-in Exit entry appended after the experience rows.
    /// `None` hides it (kiosk builds / hosts that own quit elsewhere).
    /// Selecting it emits `ShellCommand::ExitProcess`; the host acts on the
    /// resulting `ShellEvent::ExitRequested`.
    pub exit_label: Option<String>,
}

impl Default for ShellLauncherPresentation {
    fn default() -> Self {
        Self {
            title: "Ambition".to_owned(),
            empty_message: "No experiences registered".to_owned(),
            // With Bevy's default font, the menu draws `·` here as a box; the
            // menu font (`MenuFont`) fixes it. The cause is not known:
            // `ambition_demo_smash`'s select screen renders `·` with the
            // default font. A composition with no menu font still shows a box.
            footer: "Arrow keys select · Enter launches".to_owned(),
            exit_label: Some("Exit".to_owned()),
        }
    }
}

#[derive(Resource, Default, Clone, Debug, Eq, PartialEq)]
pub struct ShellLauncherState {
    pub active: bool,
    pub selected: usize,
    /// Which tab the title screen is showing: the game list or settings. The
    /// title screen has nothing to pause, so settings is a tab here, not the
    /// pause menu.
    pub tab: LauncherTab,
}

/// The title screen's tabs, in the order the bumpers cycle them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LauncherTab {
    /// The game picker.
    #[default]
    Home,
    /// The same global audio controls as the shell pause menu.
    Settings,
}

impl LauncherTab {
    pub const ALL: [Self; 2] = [Self::Home, Self::Settings];

    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Choose Game",
            Self::Settings => "Settings",
        }
    }

    /// The tab at an exact index, clamped to the strip. This is what a click
    /// means; [`Self::cycled`] is a step from the current tab.
    ///
    /// Clamped, not `Option`: the tab strip is built from `ALL`, so an
    /// out-of-range index is a renderer bug. The last tab is a safe answer.
    pub fn at_index(index: usize) -> Self {
        Self::ALL[index.min(Self::ALL.len() - 1)]
    }

    /// Cycle by `bump`, with wraparound. Same bumper rule as the kaleidoscope
    /// tab strip.
    pub fn cycled(self, bump: i32) -> Self {
        let n = Self::ALL.len() as i32;
        let at = Self::ALL.iter().position(|t| *t == self).unwrap_or(0) as i32;
        Self::ALL[((at + bump).rem_euclid(n)) as usize]
    }
}

#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellLauncherCommand {
    Previous,
    Next,
    /// Cycle the tab strip. `-1` left, `+1` right; wraps.
    CycleTab(i32),
    /// Put the strip on an exact tab: what a click or tap on a tab means.
    /// Sent by the pointer handler from `MenuTabActivated`.
    ///
    /// Not a `CycleTab` delta, which would need the current tab and duplicate
    /// tab arithmetic in the pointer handler.
    SelectTab(usize),
    /// Put the cursor on an exact row. The caller computes the settings-tab
    /// cursor.
    SelectRow(usize),
    /// Adjust the focused settings control. `-1` left, `+1` right.
    AdjustSetting(i32),
    LaunchSelected,
    /// Select and launch one row from the launcher's semantic selectable space.
    Activate(usize),
    /// Move the cursor to one row without launching it: what a mouse hover
    /// means. Separate from [`Self::Activate`], because pointing is not
    /// choosing. It moves the same cursor as `Previous`/`Next`.
    Focus(usize),
}
