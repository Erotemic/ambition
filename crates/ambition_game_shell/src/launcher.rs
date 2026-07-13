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
            footer: "Arrow keys select · Enter launches".to_owned(),
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
}
