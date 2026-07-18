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

use bevy::prelude::{Entity, Resource};

use crate::content::DialogChoice;
use crate::context::DialogueContext;
use ambition_engine_core::Vec2;
use ambition_ui_nav::MenuFocusState;

#[cfg(feature = "ui")]
use bevy_yarnspinner::prelude::OptionId;

/// A `start()` request waiting for the Yarn bridge to drain it.
///
/// The [`DialogueContext`] rides along so the bridge can publish `$speaker_id`
/// / `$listener_id` / `$speaker_is_self` into the runner's variable storage
/// BEFORE the node begins — content's `<<if>>` reads them on its first line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PendingStart {
    pub(crate) dialogue_id: String,
    pub(crate) npc_name: String,
    pub(crate) context: DialogueContext,
}

#[derive(Clone, Debug, Default, Resource)]
pub struct DialogState {
    /// Whether the dialogue UI is currently visible. Flipped to `true`
    /// when `start()` is called (so the UI shows even on the first
    /// frame, before `PresentLine` lands), flipped to `false` when
    /// the runner reports `DialogueCompleted`. Bridge-write access
    /// inside the crate; readers go through `active()`.
    pub(crate) active: bool,
    /// Display name of the NPC that started this conversation.
    /// Yarn lines carry their own `character_name`; this is the
    /// fallback shown when a line has no speaker prefix.
    npc_name: String,
    /// Dialogue id (== Yarn root node name) for the active
    /// conversation. Empty when no conversation is active.
    dialogue_id: String,

    /// The actor entity that opened this conversation, if it came from an
    /// in-world NPC interaction. `None` for scripted / system-started
    /// dialogue with no speaker actor. Yarn commands that act on "the NPC
    /// I'm talking to" (e.g. `<<challenge>>` provoking it into a fight)
    /// read this. Cleared on every `start()` so a stale entity from a
    /// prior conversation can't leak into the next one.
    pub(crate) speaker_entity: Option<Entity>,

    /// Latest speaker from `PresentLine`. May differ from
    /// `npc_name` mid-conversation (e.g. an off-screen voice or a
    /// second character).
    pub(crate) current_speaker: String,
    /// Latest line text (with character-name prefix stripped).
    pub(crate) current_line: String,
    /// Typewriter reveal state for the current line.
    pub(crate) line_reveal: LineRevealState,
    /// Presentation style for the current line, derived from Yarn markup.
    /// Normal speech may use a provider-authored voiceprint; styled speech
    /// uses the generic whisper/shout blips.
    pub(crate) speech_style: DialogSpeechStyle,
    /// Whether the line was marked by Yarn as the last line before
    /// an options block. This is the explicit "auto-advance into
    /// options" signal, so plain lines still require a confirm.
    pub(crate) line_last_before_options: bool,
    /// Latest options from `PresentOptions`. Empty when the
    /// player is reading a non-branching line.
    pub(crate) current_options: Vec<DialogChoice>,
    /// Typewriter reveal state for the current options list.
    pub(crate) options_reveal: OptionsRevealState,
    /// Parallel-index Yarn option ids, used by the dispatch
    /// system to call `runner.select_option(...)`.
    #[cfg(feature = "ui")]
    pub(crate) yarn_option_ids: Vec<OptionId>,

    pub(crate) selected_option: usize,
    /// Pointer activation arm used by the shared menu tap policy. Depending
    /// on `MenuTapMode`, a first press may select and arm a row while a second
    /// press (or semantic Confirm from any device) activates it. This is input
    /// policy, not an Android-specific branch.
    pub(crate) pointer_armed: Option<usize>,
    /// Which input source currently owns selection focus, plus the
    /// last row the pointer actually hovered.
    pub(crate) focus: MenuFocusState,
    /// Last cursor position that successfully owned dialog hover.
    /// Used to ignore stationary hover when the option list scrolls
    /// underneath the mouse.
    pub(crate) last_pointer_position: Option<Vec2>,

    /// Pending request: `Some((dialogue_id, npc_name))` until a
    /// dispatch system drains it and calls `runner.start_node`.
    pub(crate) pending_start: Option<PendingStart>,
    /// Pending request: `true` until a dispatch system drains it
    /// and calls `runner.stop`. Set on `state.close()`.
    pub(crate) pending_close: bool,
    /// Pending request: `Some(option_index_into_current_options)`
    /// until a dispatch system drains it and calls
    /// `runner.select_option(yarn_option_ids[i])`.
    pub(crate) pending_select: Option<usize>,
    /// Pending request: `true` until a dispatch system drains it
    /// and calls `runner.continue_in_next_update()`. Set by the
    /// player confirming a plain line, or by the reveal tick when a
    /// line explicitly marked as `lastline` finishes and needs to
    /// hand off to an options block immediately.
    pub(crate) pending_advance: bool,
    /// Set by the `DialogueCompleted` observer when the runner
    /// finishes a node chain but `current_line` still has text to
    /// read. The UI keeps showing the dialog with a "press to
    /// continue" hint; the player's next confirm flips this to a
    /// `pending_close`. Without this flag, the auto-advance flow
    /// would race past the final line before the player could read.
    pub(crate) runner_done_pending_close: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
// The shout/whisper variants are constructed by the Yarn bridge when the
// `ui` feature is enabled. Default headless/dialog-model builds still match
// on them from the SFX selector, but do not construct them.
#[cfg_attr(not(feature = "ui"), allow(dead_code))]
pub(crate) enum DialogSpeechStyle {
    #[default]
    Normal,
    Whisper,
    Shout,
}

impl DialogSpeechStyle {
    #[cfg_attr(not(feature = "ui"), allow(dead_code))]
    pub(crate) fn from_markup(shout: bool, whisper: bool) -> Self {
        if shout {
            Self::Shout
        } else if whisper {
            Self::Whisper
        } else {
            Self::Normal
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LineRevealState {
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
    pub fn start(&mut self, dialogue_id: &str, npc_name: &str, context: DialogueContext) {
        self.active = true;
        self.dialogue_id = dialogue_id.to_string();
        self.npc_name = npc_name.to_string();
        // A fresh conversation has no known speaker actor until the caller
        // sets one via `set_speaker_entity`. Clearing here prevents a stale
        // entity from a prior dialogue leaking into a system-started one.
        self.speaker_entity = None;
        self.current_speaker.clear();
        self.current_line.clear();
        self.line_reveal = LineRevealState::default();
        self.speech_style = DialogSpeechStyle::Normal;
        self.line_last_before_options = false;
        self.current_options.clear();
        self.options_reveal = OptionsRevealState::default();
        #[cfg(feature = "ui")]
        self.yarn_option_ids.clear();
        self.selected_option = 0;
        self.pointer_armed = None;
        self.focus = MenuFocusState::default();
        self.last_pointer_position = None;
        self.pending_start = Some(PendingStart {
            dialogue_id: dialogue_id.to_string(),
            npc_name: npc_name.to_string(),
            context,
        });
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
        self.speech_style = DialogSpeechStyle::Normal;
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

    /// Record the actor entity that opened this conversation. Called by the
    /// interaction system right after `start()` so Yarn commands can act on
    /// "the NPC I'm talking to".
    pub fn set_speaker_entity(&mut self, entity: Entity) {
        self.speaker_entity = Some(entity);
    }

    /// The actor entity that opened this conversation, if any. `None` for
    /// scripted dialogue with no in-world speaker.
    pub fn speaker_entity(&self) -> Option<Entity> {
        self.speaker_entity
    }

    /// Human-facing speaker label for the current line. Falls back to the
    /// conversation partner while the first Yarn line is still in flight or
    /// when authored dialogue omits an explicit character prefix.
    ///
    /// This is raw presentation input, not a preformatted title: each game is
    /// free to place the name beside a portrait, in a nameplate, or nowhere.
    pub fn speaker_label(&self) -> &str {
        if self.current_speaker.is_empty() {
            &self.npc_name
        } else {
            &self.current_speaker
        }
    }

    /// Human-facing label of the NPC / conversation endpoint that started the
    /// dialogue. This may differ from [`Self::speaker_label`] when a Yarn node
    /// switches speakers mid-conversation.
    pub fn conversation_label(&self) -> &str {
        &self.npc_name
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

    // Called by the Yarn bridge when the `ui` feature is enabled; default
    // headless/dialog-model builds start from already-populated state in tests.
    #[cfg_attr(not(feature = "ui"), allow(dead_code))]
    pub(crate) fn start_revealing_line(&mut self, text: String) {
        self.current_line = text;
        self.line_reveal = LineRevealState::from_line(&self.current_line);
    }

    pub(crate) fn tick_reveal(&mut self, delta_s: f32) {
        self.line_reveal.tick(delta_s, &self.current_line);
    }

    pub(crate) fn visible_line_char_count(&self) -> usize {
        self.line_reveal.revealed_char_count()
    }

    pub(crate) fn speaker_label_for_sfx(&self) -> &str {
        self.speaker_label()
    }

    #[cfg_attr(not(feature = "ui"), allow(dead_code))]
    pub(crate) fn set_speech_style(&mut self, style: DialogSpeechStyle) {
        self.speech_style = style;
    }

    pub(crate) fn speech_style(&self) -> DialogSpeechStyle {
        self.speech_style
    }

    pub(crate) fn reveal_full_line(&mut self) {
        self.line_reveal.reveal_full_line(&self.current_line);
    }

    pub(crate) fn line_reveal_complete(&self) -> bool {
        self.line_reveal.complete(&self.current_line)
    }

    pub(crate) fn visible_line(&self) -> &str {
        self.line_reveal.visible_line(&self.current_line)
    }

    #[cfg_attr(not(feature = "ui"), allow(dead_code))]
    pub(crate) fn set_line_last_before_options(&mut self, is_last: bool) {
        self.line_last_before_options = is_last;
    }

    pub(crate) fn line_last_before_options(&self) -> bool {
        self.line_last_before_options
    }

    pub(crate) fn tick_options_reveal(&mut self, delta_s: f32) {
        self.options_reveal
            .tick(delta_s, self.current_options.len());
    }

    pub(crate) fn reveal_full_options(&mut self) {
        self.options_reveal.reveal_full(self.current_options.len());
    }

    pub(crate) fn options_reveal_complete(&self) -> bool {
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

    pub(crate) fn select_delta(&mut self, delta: isize) {
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

    /// Move selection without wrapping. Wheel and drag scrolling use this so
    /// reaching the end of a long list does not unexpectedly jump to the other
    /// edge; directional keyboard/gamepad/touch-stick navigation keeps the
    /// existing wrapping behavior through [`Self::select_delta`].
    pub(crate) fn select_delta_clamped(&mut self, delta: isize) {
        self.focus.mark_keyboard();
        let len = self.options().len();
        if len == 0 {
            self.selected_option = 0;
            return;
        }
        let next = (self.selected_option as isize + delta).clamp(0, len.saturating_sub(1) as isize)
            as usize;
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
    pub(crate) fn confirm_or_advance(&mut self) -> bool {
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
    #[cfg_attr(not(feature = "ui"), allow(dead_code))]
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

    fn revealed_char_count(&self) -> usize {
        self.revealed_chars
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
pub(crate) struct OptionsRevealState {
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
