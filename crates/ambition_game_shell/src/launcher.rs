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
    /// Selecting it emits the semantic `ShellCommand::ExitProcess`; acting on
    /// the resulting `ShellEvent::ExitRequested` remains the HOST's job.
    pub exit_label: Option<String>,
}

impl Default for ShellLauncherPresentation {
    fn default() -> Self {
        Self {
            title: "Ambition".to_owned(),
            empty_message: "No experiences registered".to_owned(),
            // Probed: forcing the default handle back at every menu text spawn and
            // re-capturing `--route ambition_launcher` puts the hollow box back on this exact
            // string.
            //
            // why the default handle fails here is NOT settled. The obvious story — "the
            // subset has no U+00B7" — is contradicted by `ambition_demo_smash`'s select screen,
            // which spawns `Text` with no `TextFont` AT ALL (so, the same handle) and renders
            // `·`, `—` and `…` correctly. Same codepoint, same font asset, different result;
            // something about the MENU's text path is the other half.
            //
            // a composition that loads no font at all still falls back to the
            // subset and this string tofus again — which is the honest outcome,
            // and why the fallback is `None` rather than a guess.
            footer: "Arrow keys select · Enter launches".to_owned(),
            exit_label: Some("Exit".to_owned()),
        }
    }
}

#[derive(Resource, Default, Clone, Debug, Eq, PartialEq)]
pub struct ShellLauncherState {
    pub active: bool,
    pub selected: usize,
    /// Which tab the title screen is showing.
    ///
    /// ⭐ Jon, 2026-09-05: *"A better design would probably have choose game be
    /// one menu tab, and then have the settings menu be in a second tab. There
    /// really isn't a notion of 'paused' in the title screen."* Settings used to
    /// arrive by opening the shell PAUSE menu over this one, which left both
    /// live and made settings unusable.
    pub tab: LauncherTab,
}

/// The title screen's tabs, in the order the bumpers cycle them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LauncherTab {
    /// The game picker.
    #[default]
    Home,
    /// The same global audio controls the shell pause menu offers, reached
    /// WITHOUT a pause menu because there is nothing here to pause.
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

    /// Cycle by `bump`, wrapping — the shared `MenuControlFrame` bumper
    /// contract, spelled the same way the kaleidoscope's tab strip spells it.
    /// The tab at an exact index, clamped to the strip.
    ///
    /// ⭐ WHAT A CLICK MEANS, and it is a different act from cycling. `cycled`
    /// answers "one step from where I am"; a pointer landing on a tab names the
    /// tab and knows nothing about where the cursor was. The same distinction
    /// `SelectRow` draws against `Previous`/`Next` one strip over.
    ///
    /// ⚠ Clamped rather than `Option`: the index comes from the rendered tab
    /// strip, which is built FROM `ALL`, so an out-of-range value means the
    /// renderer and this enum disagree — and falling back to the last real tab
    /// is a better answer than dropping the only gesture a mouse has.
    pub fn at_index(index: usize) -> Self {
        Self::ALL[index.min(Self::ALL.len() - 1)]
    }

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
    /// Put the strip on an EXACT tab — what clicking or tapping a tab means.
    ///
    /// ⛔⛔ THE TITLE SCREEN HAD NO POINTER ROAD TO ITS SETTINGS TAB AT ALL.
    /// The renderer already draws tabs as real `Button`s and
    /// `publish_bevy_ui_menu_tabs` already publishes `MenuTabActivated` — but its
    /// ONLY consumer was the kaleidoscope menu, so a click on the shell's
    /// Settings tab produced a message nobody read. Reported by Jon 2026-09-06:
    /// *"in the title screen there is no way for me to select the settings menu.
    /// I can't click, tap, nothing."*
    ///
    /// ⚠ Deliberately not expressed as a `CycleTab` delta: computing the delta
    /// needs the CURRENT tab, which would put tab arithmetic in the pointer
    /// handler as well as on `LauncherTab` — the second copy this command's
    /// neighbour warns about in as many words.
    SelectTab(usize),
    /// Put the cursor on an exact row — the settings tab's rows are a fixed
    /// list, so its cursor is computed by the caller rather than nudged.
    SelectRow(usize),
    /// Adjust the focused settings control. `-1` left, `+1` right.
    AdjustSetting(i32),
    LaunchSelected,
    /// Select and launch one row from the launcher's semantic selectable space.
    Activate(usize),
    /// Move the cursor to one row WITHOUT launching it.
    ///
    /// What a mouse hover means. It is deliberately a separate command from
    /// [`Self::Activate`] rather than a flag on it: pointing at a thing and
    /// choosing it are different acts, and a launcher that started a game
    /// because the pointer crossed a row on its way somewhere else would be
    /// unusable. Keyboard `Previous`/`Next` and this land in the same cursor,
    /// so hovering then pressing Enter does what it looks like it will.
    Focus(usize),
}
