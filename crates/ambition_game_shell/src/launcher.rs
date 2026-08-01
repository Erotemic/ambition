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
            // ⚠ **ASCII only, and the REASON is an open question worth more than
            // this line.** It read `select · Enter` and drew a TOFU BOX on the
            // title screen — confirmed at 2560×1440, a hollow rectangle.
            //
            // ⛔ the cause is NOT "the shipped font lacks the glyph", and it is
            // narrowed to one crate. `JetBrainsMono-Regular.ttf` is bundled and
            // HAS U+00B7 (fontTools). Decisive experiment: the SAME string on
            // both text paths, on this route, in ONE frame —
            //
            //     bare `Text::new`   →  "A · B — C"   renders correctly
            //     `ambition_menu`    →  "A ▯ B ▯ C"   both tofu
            //
            // So it is the MENU RENDER PATH, not the font asset, not the string
            // and not the capture. `spawn.rs` sets no font handle
            // (`TextFont { font_size, ..default() }`), so the difference is
            // somewhere else in that crate — and it means every `ambition_menu`
            // surface in every game is affected, invisibly, until a string steps
            // outside ASCII.
            //
            // Found by photographing the route (`capture_scene --route
            // ambition_launcher`) while checking an unrelated change — which is
            // the argument for drawing blind work: every test here was green and
            // none of them can assert what a missing glyph looks like.
            footer: "Arrow keys select | Enter launches".to_owned(),
            exit_label: Some("Exit".to_owned()),
        }
    }
}

#[derive(Resource, Default, Clone, Debug, Eq, PartialEq)]
pub struct ShellLauncherState {
    pub active: bool,
    pub selected: usize,
}

#[derive(Message, Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellLauncherCommand {
    Previous,
    Next,
    LaunchSelected,
    /// Select and launch one row from the launcher's semantic selectable space.
    /// Pointer/touch rows carry this index directly, so they do not have to
    /// synthesize a sequence of Previous/Next commands before confirmation.
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
