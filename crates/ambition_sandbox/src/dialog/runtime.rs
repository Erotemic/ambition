use bevy::prelude::Resource;

use super::content::{tree_for, DialogChoice, DialogNode, DialogTree, GENERIC_DIALOGUE_ID};

#[derive(Clone, Debug, Default, Resource)]
pub struct DialogState {
    active: bool,
    npc_name: String,
    /// Authored dialogue id (matches the LDtk `NpcSpawn.dialogue_id`
    /// field and the registry key in `registry.ron`). Empty when the
    /// state is inactive.
    dialogue_id: String,
    node_index: usize,
    pub(in crate::dialog) selected_option: usize,
    /// Android/touch row activation is deliberately two-step: first tap selects,
    /// second tap or a Confirm button activates. This prevents a finger press
    /// that turns into a small drag from accidentally advancing dialogue.
    pub(in crate::dialog) pointer_armed: Option<usize>,
    last_note: String,
}

impl DialogState {
    pub fn start(&mut self, dialogue_id: &str, npc_name: &str) {
        self.active = true;
        self.dialogue_id = dialogue_id.to_string();
        self.npc_name = npc_name.to_string();
        self.node_index = 0;
        self.selected_option = 0;
        self.pointer_armed = None;
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

    /// Swap the live dialogue tree mid-conversation, resetting node /
    /// option indices so the next render shows the new branch from
    /// its first node. Intended for redirect systems (e.g. quest
    /// state changes the dialog tree). Callers must guarantee the
    /// new id resolves to a non-empty tree in the registry.
    pub fn set_dialogue_id(&mut self, dialogue_id: &str) {
        if self.dialogue_id == dialogue_id {
            return;
        }
        self.dialogue_id = dialogue_id.to_string();
        self.node_index = 0;
        self.selected_option = 0;
        self.pointer_armed = None;
        self.last_note.clear();
    }

    /// Current dialogue id. Returns `""` when the state is inactive.
    pub fn dialogue_id(&self) -> &str {
        &self.dialogue_id
    }

    pub fn title(&self) -> String {
        let tree = self.current_tree();
        if let Some(node) = self.current_node() {
            let label = tree
                .map(|t| t.label.as_str())
                .unwrap_or("dialogue");
            format!("{} — {}", node.speaker, label)
        } else {
            format!("{} — dialogue", self.npc_name)
        }
    }

    pub fn body(&self) -> String {
        let Some(node) = self.current_node() else {
            return "The conversation data is missing; this is a dialogue routing bug.".to_string();
        };
        let mut body = node.line.clone();
        if !self.last_note.is_empty() {
            body.push_str("\n\n");
            body.push_str(&self.last_note);
        }
        body
    }

    pub fn options(&self) -> &'static [DialogChoice] {
        self.current_node()
            .map(|node| node.options.as_slice())
            .unwrap_or(&[])
    }

    pub fn selected_option(&self) -> usize {
        self.selected_option
    }

    fn current_tree(&self) -> Option<&'static DialogTree> {
        tree_for(&self.dialogue_id).or_else(|| tree_for(GENERIC_DIALOGUE_ID))
    }

    fn current_node(&self) -> Option<&'static DialogNode> {
        self.current_tree()
            .and_then(|tree| tree.nodes.get(self.node_index))
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
        if let Some(note) = &choice.note {
            self.last_note = note.clone();
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
