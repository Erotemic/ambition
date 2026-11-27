//! Pages that a host can add to the basic pause menu.

use ambition_input::ParticipantId;
use bevy::prelude::*;

pub type ShellPausePageId = &'static str;

/// One option on a page. Keep its id the same when its label or position changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellPauseRow {
    pub id: String,
    pub label: String,
    pub value: String,
}

pub(super) struct PausePage {
    pub id: ShellPausePageId,
    pub title: &'static str,
    pub owner: Option<ParticipantId>,
    pub rows: Vec<ShellPauseRow>,
    pub capture_hint: Option<&'static str>,
}

/// The pages that installed plugins supply to the pause menu.
#[derive(Resource, Default)]
pub struct ShellPausePages(pub(super) Vec<PausePage>);

impl ShellPausePages {
    /// Add a page at the end of the current page list.
    pub fn register(&mut self, id: ShellPausePageId, title: &'static str) {
        assert!(self.get(id).is_none(), "duplicate pause page {id}");
        self.0.push(PausePage {
            id,
            title,
            owner: None,
            rows: Vec::new(),
            capture_hint: None,
        });
    }

    /// Supply the options for the foreground participant. Use a capture hint
    /// to let the page read raw input until the operation ends.
    pub fn publish(
        &mut self,
        id: ShellPausePageId,
        owner: ParticipantId,
        rows: Vec<ShellPauseRow>,
        capture_hint: Option<&'static str>,
    ) {
        let page = self
            .0
            .iter_mut()
            .find(|page| page.id == id)
            .expect("registered pause page");
        page.owner = Some(owner);
        page.rows = rows;
        page.capture_hint = capture_hint;
    }

    pub(super) fn get(&self, id: ShellPausePageId) -> Option<&PausePage> {
        self.0.iter().find(|page| page.id == id)
    }
}

/// A request from the pause menu to the owner of a page.
#[derive(Message, Debug, Clone)]
pub struct ShellPauseOption {
    pub page: ShellPausePageId,
    pub owner: ParticipantId,
    pub row: String,
    /// Zero confirms the option. A nonzero value requests a directional step.
    pub direction: i32,
}

/// Publish page data after input and before the pause menu draws.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublishShellPausePages;
