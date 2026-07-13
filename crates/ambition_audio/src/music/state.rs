use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicDirectorMode {
    Idle,
    SimpleTrack,
    AdaptiveIntro,
    AdaptiveLoop,
    AdaptiveOutro,
    AdaptiveFinished,
}

#[derive(Resource, Debug, Clone)]
pub struct MusicDirectorState {
    pub mode: MusicDirectorMode,
    pub active_cue_id: Option<String>,
    pub current_state_id: Option<String>,
    pub current_section_id: Option<String>,
    pub seconds_in_mode: f32,
    pub seconds_in_loop: f32,
    pub(super) active_bank: MusicBank,
    pub(super) fading_bank: Option<MusicBank>,
    pub(super) fade_stop_seconds: f32,
    pub(super) current_gains: [LayerGains; 2],
    pub(super) target_gains: [LayerGains; 2],
    pub(super) pending_state: Option<PendingMusicStateTransition>,
    pub(super) default_resume_started: bool,
    pub(super) debug_log_timer: f32,
    pub(super) last_simple_track: Option<String>,
}

impl Default for MusicDirectorState {
    fn default() -> Self {
        Self {
            mode: MusicDirectorMode::Idle,
            active_cue_id: None,
            current_state_id: None,
            current_section_id: None,
            seconds_in_mode: 0.0,
            seconds_in_loop: 0.0,
            active_bank: MusicBank::A,
            fading_bank: None,
            fade_stop_seconds: 0.0,
            current_gains: [[0.0; MAX_LAYERS]; 2],
            target_gains: [[0.0; MAX_LAYERS]; 2],
            pending_state: None,
            default_resume_started: false,
            debug_log_timer: 0.0,
            last_simple_track: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct PendingMusicStateTransition {
    pub(super) state_id: String,
    pub(super) delay_seconds: f32,
}

/// A content-agnostic adaptive-cue intent handed to the director.
///
/// The director never decides *which* cue plays for *which* game event;
/// the game's content layer resolves that (e.g. encounter phase → cue
/// state) and pushes the resulting directive in via [`MusicIntent`]. The
/// directive only names cue/state ids that the director looks up in its
/// catalog, so it carries no game-specific vocabulary.
#[derive(Debug, Clone, PartialEq)]
pub enum AdaptiveCueDirective {
    /// Play `cue_id` in adaptive `state_id` this frame.
    Play { cue_id: String, state_id: String },
    /// Stop the active adaptive cue immediately (e.g. encounter failed).
    StopNow,
}

/// Per-frame music intent the game's content layer pushes to the director.
///
/// This is the decoupling seam: the music director is content-agnostic and
/// reads only this neutral resource plus its catalog/assets/audio backend.
/// The Ambition-specific mapping ("which track / cue for which encounter,
/// boss, room, or radio station") lives in [`super::intent`] and writes here.
/// A different game would supply its own `compute_music_intent` and reuse the
/// director unchanged.
#[derive(Resource, Default, Debug, Clone)]
pub struct MusicIntent {
    /// Provider whose actual adaptive definitions and track authority apply to
    /// this frame. `None` means no active music context.
    pub provider_id: Option<String>,
    /// Adaptive cue directive for this frame, if any content owns audio.
    /// `None` means "no adaptive claim — fall back to the simple track."
    pub adaptive: Option<AdaptiveCueDirective>,
    /// Simple (single-track) candidates in priority order. The director plays
    /// the first id that BOTH exists in its `AudioLibrary` AND is permitted by
    /// [`Self::authority`]. Empty means "nothing requested" (the director keeps
    /// whatever is playing, subject to authority).
    pub simple_track_candidates: Vec<String>,
    /// Provider-relative playback authority for this frame. The content layer
    /// derives it from the active audio selection; the director enforces it so a
    /// track present in the process-wide combined library but foreign to the
    /// active provider can never play, and so a provider that authored no music
    /// yields deliberate silence rather than a retained previous track.
    pub authority: crate::selection::MusicAuthority,
}
