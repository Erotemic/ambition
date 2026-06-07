//! `DialogState` — the dialogue UI's read-model.
//!
//! Yarn-driven (post-phase 5): the runner is the authority. The
//! existing UI (`sync_dialog_ui`) and input systems (`dialog_input`,
//! `dialog_pointer_input`) continue to call the legacy accessors
//! (`active()`, `body()`, `options()`, `title()`,
//! `confirm_or_advance()`, `select_delta()`), but those now read /
//! write fields populated by the Yarn bridge.
//!
//! ## How writes flow
//!
//! - Caller (interact system) → `state.start(dialogue_id, npc_name)`.
//!   The method stashes a `pending_start` request on the state; a
//!   bridge system reads it, increments `dialog_visit_count` in
//!   save, and calls `runner.start_node(id)`.
//! - Runner triggers `PresentLine` → bridge observer writes
//!   `current_speaker` + `current_line`; the reveal tick exposes
//!   it to the UI over time.
//! - Runner triggers `PresentOptions` → bridge observer writes
//!   `current_options` + `yarn_option_ids`.
//! - Player input → `state.confirm_or_advance()` stashes
//!   `pending_select` or `pending_advance`; the dispatch system
//!   calls `runner.select_option(id)` or
//!   `runner.continue_in_next_update()`.
//! - Runner triggers `DialogueCompleted` → bridge observer flips
//!   `active = false`.
//!
//! ## Why a "pending request" indirection
//!
//! Callers of `state.start(...)` hold a `&mut DialogState`, not a
//! full Bevy `World`. The runner lives on an entity and needs a
//! `Query<&mut DialogueRunner>`. The pending-request fields are
//! the seam that lets pure callers stay pure and a single dispatch
//! system own the runner access. Same pattern for close + select.

use bevy::prelude::Resource;

use super::content::DialogChoice;
use crate::engine_core::Vec2;
use crate::ui_nav::MenuFocusState;

#[cfg(feature = "ui")]
use bevy_yarnspinner::prelude::OptionId;

#[derive(Clone, Debug, Default, Resource)]
pub struct DialogState {
    /// Whether the dialogue UI is currently visible. Flipped to `true`
    /// when `start()` is called (so the UI shows even on the first
    /// frame, before `PresentLine` lands), flipped to `false` when
    /// the runner reports `DialogueCompleted`. Bridge-write access
    /// inside the crate; readers go through `active()`.
    pub(in crate::dialog) active: bool,
    /// Display name of the NPC that started this conversation.
    /// Yarn lines carry their own `character_name`; this is the
    /// fallback shown when a line has no speaker prefix.
    npc_name: String,
    /// Dialogue id (== Yarn root node name) for the active
    /// conversation. Empty when no conversation is active.
    dialogue_id: String,

    /// Latest speaker from `PresentLine`. May differ from
    /// `npc_name` mid-conversation (e.g. an off-screen voice or a
    /// second character).
    pub(in crate::dialog) current_speaker: String,
    /// Latest line text (with character-name prefix stripped).
    pub(in crate::dialog) current_line: String,
    /// Typewriter reveal state for the current line.
    pub(in crate::dialog) line_reveal: LineRevealState,
    /// Whether the line was marked by Yarn as the last line before
    /// an options block. This is the explicit "auto-advance into
    /// options" signal, so plain lines still require a confirm.
    pub(in crate::dialog) line_last_before_options: bool,
    /// Latest options from `PresentOptions`. Empty when the
    /// player is reading a non-branching line.
    pub(in crate::dialog) current_options: Vec<DialogChoice>,
    /// Parallel-index Yarn option ids, used by the dispatch
    /// system to call `runner.select_option(...)`.
    #[cfg(feature = "ui")]
    pub(in crate::dialog) yarn_option_ids: Vec<OptionId>,

    pub(in crate::dialog) selected_option: usize,
    /// Android/touch row activation is deliberately two-step:
    /// first tap selects, second tap or a Confirm button
    /// activates. This prevents a finger press that turns into a
    /// small drag from accidentally advancing dialogue.
    pub(in crate::dialog) pointer_armed: Option<usize>,
    /// Which input source currently owns selection focus, plus the
    /// last row the pointer actually hovered.
    pub(in crate::dialog) focus: MenuFocusState,
    /// Last cursor position that successfully owned dialog hover.
    /// Used to ignore stationary hover when the option list scrolls
    /// underneath the mouse.
    pub(in crate::dialog) last_pointer_position: Option<Vec2>,

    /// Pending request: `Some((dialogue_id, npc_name))` until a
    /// dispatch system drains it and calls `runner.start_node`.
    pub(in crate::dialog) pending_start: Option<(String, String)>,
    /// Pending request: `true` until a dispatch system drains it
    /// and calls `runner.stop`. Set on `state.close()`.
    pub(in crate::dialog) pending_close: bool,
    /// Pending request: `Some(option_index_into_current_options)`
    /// until a dispatch system drains it and calls
    /// `runner.select_option(yarn_option_ids[i])`.
    pub(in crate::dialog) pending_select: Option<usize>,
    /// Pending request: `true` until a dispatch system drains it
    /// and calls `runner.continue_in_next_update()`. Set EITHER by
    /// the player confirming a no-option line OR by the bridge
    /// auto-advancing after a `PresentLine` (so the player sees
    /// `line + options` together without a Continue press between).
    pub(in crate::dialog) pending_advance: bool,
    /// Set by the `DialogueCompleted` observer when the runner
    /// finishes a node chain but `current_line` still has text to
    /// read. The UI keeps showing the dialog with a "press to
    /// continue" hint; the player's next confirm flips this to a
    /// `pending_close`. Without this flag, the auto-advance flow
    /// would race past the final line before the player could read.
    pub(in crate::dialog) runner_done_pending_close: bool,
}

#[derive(Clone, Debug)]
pub(in crate::dialog) struct LineRevealState {
    full_line_byte_ends: Vec<usize>,
    revealed_chars: usize,
    elapsed_s: f32,
    chars_per_second: f32,
}

impl Default for LineRevealState {
    fn default() -> Self {
        Self {
            full_line_byte_ends: Vec::new(),
            revealed_chars: 0,
            elapsed_s: 0.0,
            chars_per_second: 225.0,
        }
    }
}

impl DialogState {
    /// Begin a conversation with the named Yarn node. Activates the
    /// UI immediately (so the player sees the dialog box even on
    /// the first frame, before `PresentLine` lands) and stashes a
    /// `pending_start` request that the dispatch system drains to
    /// call `runner.start_node`. Increments
    /// `SandboxSave.dialog_visits[id]` so Yarn's `visit_count(id)`
    /// function reflects the new visit.
    pub fn start(&mut self, dialogue_id: &str, npc_name: &str) {
        self.active = true;
        self.dialogue_id = dialogue_id.to_string();
        self.npc_name = npc_name.to_string();
        self.current_speaker.clear();
        self.current_line.clear();
        self.line_reveal = LineRevealState::default();
        self.line_last_before_options = false;
        self.current_options.clear();
        #[cfg(feature = "ui")]
        self.yarn_option_ids.clear();
        self.selected_option = 0;
        self.pointer_armed = None;
        self.focus = MenuFocusState::default();
        self.last_pointer_position = None;
        self.pending_start = Some((dialogue_id.to_string(), npc_name.to_string()));
        // Clear any stale pending close from a previous session.
        self.pending_close = false;
        self.pending_select = None;
        self.pending_advance = false;
        self.runner_done_pending_close = false;
    }

    /// Close the dialogue. Hides the UI immediately and stashes a
    /// `pending_close` request that the dispatch system drains to
    /// call `runner.stop()`.
    pub fn close(&mut self) {
        self.active = false;
        self.pending_close = true;
        self.current_speaker.clear();
        self.current_line.clear();
        self.line_reveal = LineRevealState::default();
        self.line_last_before_options = false;
        self.current_options.clear();
        #[cfg(feature = "ui")]
        self.yarn_option_ids.clear();
        self.pointer_armed = None;
        self.focus = MenuFocusState::default();
        self.last_pointer_position = None;
    }

    pub fn active(&self) -> bool {
        self.active
    }

    /// Active dialogue id (matches the LDtk `NpcSpawn.dialogue_id`
    /// + the Yarn root node name). Empty when inactive.
    pub fn dialogue_id(&self) -> &str {
        &self.dialogue_id
    }

    pub fn title(&self) -> String {
        if self.current_speaker.is_empty() {
            format!("{} — dialogue", self.npc_name)
        } else {
            format!("{} — {}", self.current_speaker, self.npc_name)
        }
    }

    pub fn body(&self) -> String {
        if self.current_line.is_empty() && self.current_options.is_empty() {
            // Either we haven't received the first PresentLine yet
            // (the start request is in flight) or we just exhausted
            // the node. Empty body is the cleanest read — the UI
            // shows the title bar with no body, which reads as
            // "loading" to the player and resolves within one
            // frame.
            String::new()
        } else {
            self.visible_line().to_string()
        }
    }

    pub(in crate::dialog) fn start_revealing_line(&mut self, text: String) {
        self.current_line = text;
        self.line_reveal = LineRevealState::from_line(&self.current_line);
    }

    pub(in crate::dialog) fn tick_reveal(&mut self, delta_s: f32) {
        self.line_reveal.tick(delta_s, &self.current_line);
    }

    pub(in crate::dialog) fn reveal_full_line(&mut self) {
        self.line_reveal.reveal_full_line(&self.current_line);
    }

    pub(in crate::dialog) fn line_reveal_complete(&self) -> bool {
        self.line_reveal.complete(&self.current_line)
    }

    pub(in crate::dialog) fn visible_line(&self) -> &str {
        self.line_reveal.visible_line(&self.current_line)
    }

    pub(in crate::dialog) fn set_line_last_before_options(&mut self, is_last: bool) {
        self.line_last_before_options = is_last;
    }

    pub(in crate::dialog) fn line_last_before_options(&self) -> bool {
        self.line_last_before_options
    }

    pub fn options(&self) -> &[DialogChoice] {
        &self.current_options
    }

    pub fn selected_option(&self) -> usize {
        self.selected_option
    }

    pub(in crate::dialog) fn select_delta(&mut self, delta: isize) {
        self.focus.mark_keyboard();
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

    /// Player commits the current selection (or advances past a
    /// non-branching line). Stashes the appropriate pending request;
    /// the dispatch system drains it next tick. Returns the legacy
    /// "was the dialogue closed" bool, which under Yarn always
    /// returns `false` — closure now flows through the
    /// `DialogueCompleted` observer. Callers that needed the return
    /// value (legacy `if closed { next_mode.set(Playing) }`) get
    /// their game-mode transition from the observer instead.
    pub(in crate::dialog) fn confirm_or_advance(&mut self) -> bool {
        if !self.line_reveal_complete() {
            self.reveal_full_line();
            return false;
        }
        if self.runner_done_pending_close {
            // Runner already finished; this press dismisses the
            // final accumulated text and closes the dialog.
            self.runner_done_pending_close = false;
            self.pending_close = true;
            self.active = false;
        } else if self.current_options.is_empty() {
            self.pending_advance = true;
        } else {
            self.pending_select = Some(
                self.selected_option
                    .min(self.current_options.len().saturating_sub(1)),
            );
        }
        false
    }
}

impl LineRevealState {
    fn from_line(line: &str) -> Self {
        // Precompute safe byte ends for each revealed character.
        // Unicode grapheme segmentation would be better long-term,
        // but char boundaries are a safe incremental step for the
        // typewriter effect.
        let mut full_line_byte_ends = Vec::with_capacity(line.chars().count() + 1);
        full_line_byte_ends.push(0);
        for (idx, ch) in line.char_indices() {
            full_line_byte_ends.push(idx + ch.len_utf8());
        }
        Self {
            full_line_byte_ends,
            revealed_chars: 0,
            elapsed_s: 0.0,
            chars_per_second: 225.0,
        }
    }

    fn tick(&mut self, delta_s: f32, line: &str) {
        if line.is_empty() || self.complete(line) {
            self.revealed_chars = self.full_line_byte_ends.len().saturating_sub(1);
            return;
        }
        self.elapsed_s = (self.elapsed_s + delta_s.max(0.0)).max(0.0);
        // Future extension points:
        // - punctuation pauses
        // - metadata-based speed like `#slow`, `#fast`, `#instant`
        // - optional typing cursor / continue indicator
        let chars = (self.elapsed_s * self.chars_per_second).floor() as usize;
        self.revealed_chars = chars.min(self.full_line_byte_ends.len().saturating_sub(1));
    }

    fn reveal_full_line(&mut self, line: &str) {
        self.elapsed_s = 0.0;
        self.revealed_chars = self.full_line_byte_ends.len().saturating_sub(1);
        if line.is_empty() {
            self.revealed_chars = 0;
        }
    }

    fn complete(&self, line: &str) -> bool {
        line.is_empty() || self.revealed_chars >= self.full_line_byte_ends.len().saturating_sub(1)
    }

    fn visible_line<'a>(&self, line: &'a str) -> &'a str {
        let end = self
            .full_line_byte_ends
            .get(self.revealed_chars)
            .copied()
            .unwrap_or_else(|| line.len());
        &line[..end]
    }
}
