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
            // ⚠ **ASCII only, and the REASON is still open.** This read
            // `select · Enter` and drew a hollow TOFU BOX on the title screen.
            // Every `ambition_menu` surface in every game is affected,
            // invisibly, until a string steps outside ASCII.
            //
            // ⛔ Do NOT re-derive it from this crate. NINE hypotheses are dead,
            // including the two that read as obvious: the bundled
            // `JetBrainsMono-Regular.ttf` HAS U+00B7 (fontTools), and the string
            // arrives intact (`footer.clone()` → `MenuNode::Text` →
            // `Text::new`, no byte slicing anywhere). Removing all four spawn
            // components at once still tofus.
            //
            // ⚠ and the live clue says the cause is NOT source-level in
            // `ambition_menu` at all: adding an unrelated SIBLING `Text` to the
            // same parent makes this title render CORRECTLY (the inserted child
            // still tofus). An extra entity changing another entity's glyph
            // resolution points at Bevy's text pipeline — atlas keying, font-face
            // resolution, `ComputedTextBlock` sharing.
            //
            // Full elimination list and the one-command repro live in the Z1 row
            // of `docs/planning/queue-24h-2026-07-26.md` (grep `TOFU BOX`).
            //
            // Found by photographing the route (`capture_scene --route
            // ambition_launcher`) while checking an unrelated change — the
            // argument for drawing blind work: every test here was green and
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
