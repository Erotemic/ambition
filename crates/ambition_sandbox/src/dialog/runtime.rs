use bevy::prelude::Resource;

use super::content::{DialogChoice, DialogMode, DialogNode};

#[derive(Clone, Debug, Default, Resource)]
pub struct DialogState {
    active: bool,
    node_id: String,
    npc_name: String,
    node_index: usize,
    pub(in crate::dialog) selected_option: usize,
    /// Android/touch row activation is deliberately two-step: first tap selects,
    /// second tap or a Confirm button activates. This prevents a finger press
    /// that turns into a small drag from accidentally advancing dialogue.
    pub(in crate::dialog) pointer_armed: Option<usize>,
    mode: DialogMode,
    last_note: String,
}

impl DialogState {
    pub fn start(&mut self, dialogue_id: &str, npc_name: &str) {
        self.active = true;
        self.node_id = dialogue_id.to_string();
        self.npc_name = npc_name.to_string();
        self.node_index = 0;
        self.selected_option = 0;
        self.pointer_armed = None;
        self.mode = DialogMode::from_dialogue_id(dialogue_id);
        self.last_note.clear();
    }

    pub fn close(&mut self) {
        self.active = false;
        self.pointer_armed = None;
        self.last_note.clear();
    }

    pub fn active(&self) -> bool {
        self.active
    }

    /// Swap the dialog's mode mid-conversation, resetting node /
    /// option indices so the next render shows the new branch from
    /// its first node. Intended for redirect systems (e.g. quest
    /// state changes the dialog tree) — callers must guarantee the
    /// new mode's node list is non-empty.
    pub fn set_mode(&mut self, mode: DialogMode) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.node_index = 0;
        self.selected_option = 0;
        self.pointer_armed = None;
        self.last_note.clear();
    }

    /// Read the current dialog branch — used by redirect systems to
    /// decide whether a remap is needed without forcing a write.
    pub fn mode(&self) -> DialogMode {
        self.mode
    }

    pub fn title(&self) -> String {
        if let Some(node) = self.current_node() {
            format!("{} — {}", node.speaker, self.mode.label())
        } else {
            format!("{} — dialogue", self.npc_name)
        }
    }

    pub fn body(&self) -> String {
        let Some(node) = self.current_node() else {
            return "The conversation data is missing; this is a dialogue routing bug.".to_string();
        };
        let mut body = node.line.to_string();
        if !self.last_note.is_empty() {
            body.push_str("\n\n");
            body.push_str(&self.last_note);
        }
        body
    }

    pub fn options(&self) -> &'static [DialogChoice] {
        self.current_node().map(|node| node.options).unwrap_or(&[])
    }

    pub fn selected_option(&self) -> usize {
        self.selected_option
    }

    fn current_node(&self) -> Option<&'static DialogNode> {
        self.mode.nodes().get(self.node_index)
    }

    pub(in crate::dialog) fn select_delta(&mut self, delta: isize) {
        let len = self.options().len();
        if len == 0 {
            self.selected_option = 0;
            return;
        }
        let next = (self.selected_option as isize + delta).rem_euclid(len as isize) as usize;
        if self.selected_option != next {
            self.pointer_armed = None;
        }
        self.selected_option = next;
    }

    pub(in crate::dialog) fn confirm_or_advance(&mut self) -> bool {
        let Some(node) = self.current_node() else {
            self.close();
            return true;
        };
        if node.options.is_empty() {
            if let Some(next) = node.default_next {
                self.node_index = next;
                self.selected_option = 0;
                self.pointer_armed = None;
                return false;
            }
            self.close();
            return true;
        }
        let choice = &node.options[self
            .selected_option
            .min(node.options.len().saturating_sub(1))];
        if let Some(note) = choice.note {
            self.last_note = note.to_string();
        } else {
            self.last_note.clear();
        }
        if choice.close_after {
            self.close();
            return true;
        }
        if let Some(next) = choice.next_node {
            self.node_index = next;
            self.selected_option = 0;
            self.pointer_armed = None;
            return false;
        }
        self.close();
        true
    }
}
