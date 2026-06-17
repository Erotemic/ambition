//! `DialogState` — the dialogue UI read model.
//!
//! The Yarn runner owns dialogue progression. `DialogState` stores the frame-local
//! UI projection: active conversation id, speaker/line text, revealed options,
//! selected option, presentation cue, and pending requests from input/UI callers.
//!
//! Callers mutate `DialogState` through pure methods such as `start`, `close`,
//! `confirm_or_advance`, and `select_delta`. The bridge drains the pending request
//! fields into the live `DialogueRunner`, then writes runner events back into this
//! read model.
//!
//! The pending-request seam keeps UI/gameplay callers independent of Bevy runner
//! queries while giving one system ownership of runner access.

use bevy::prelude::{Component, Resource};

use super::content::DialogChoice;
use crate::engine_core::Vec2;
use crate::ui_nav::MenuFocusState;

/// Marker on a rendered dialog choice-row entity, carrying its option index.
///
/// The render layer's `dialog_ui` spawns these; the sim-side
/// [`super::systems::dialog_pointer_input`] reads them to map a click to a
/// choice. Content-free, so it lives in the sandbox dialog module — both the
/// renderer and the input system name it without crossing the seam backwards.
#[derive(Component, Clone, Copy, Debug)]
pub struct DialogChoiceSlot {
    pub index: usize,
}

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
    /// Typewriter reveal state for the current options list.
    pub(in crate::dialog) options_reveal: OptionsRevealState,
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
    /// and calls `runner.continue_in_next_update()`. Set by the
    /// player confirming a plain line, or by the reveal tick when a
    /// line explicitly marked as `lastline` finishes and needs to
    /// hand off to an options block immediately.
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
            chars_per_second: 112.5,
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
        self.options_reveal = OptionsRevealState::default();
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
        self.options_reveal = OptionsRevealState::default();
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

    pub(in crate::dialog) fn tick_options_reveal(&mut self, delta_s: f32) {
        self.options_reveal
            .tick(delta_s, self.current_options.len());
    }

    pub(in crate::dialog) fn reveal_full_options(&mut self) {
        self.options_reveal.reveal_full(self.current_options.len());
    }

    pub(in crate::dialog) fn options_reveal_complete(&self) -> bool {
        self.options_reveal.complete(self.current_options.len())
    }

    pub fn options(&self) -> &[DialogChoice] {
        let visible = self
            .options_reveal
            .visible_count
            .min(self.current_options.len());
        &self.current_options[..visible]
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
        if !self.options_reveal_complete() {
            self.reveal_full_options();
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
            chars_per_second: 112.5,
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

#[derive(Clone, Debug)]
pub(in crate::dialog) struct OptionsRevealState {
    visible_count: usize,
    elapsed_s: f32,
    options_per_second: f32,
}

impl Default for OptionsRevealState {
    fn default() -> Self {
        Self {
            visible_count: 0,
            elapsed_s: 0.0,
            options_per_second: 1.0,
        }
    }
}

impl OptionsRevealState {
    fn tick(&mut self, delta_s: f32, total_count: usize) {
        if total_count == 0 || self.complete(total_count) {
            self.visible_count = total_count;
            return;
        }
        self.elapsed_s = (self.elapsed_s + delta_s.max(0.0)).max(0.0);
        let visible = (self.elapsed_s * self.options_per_second).floor() as usize;
        self.visible_count = visible.min(total_count);
    }

    fn reveal_full(&mut self, total_count: usize) {
        self.visible_count = total_count;
        self.elapsed_s = 0.0;
    }

    fn complete(&self, total_count: usize) -> bool {
        self.visible_count >= total_count
    }
}
