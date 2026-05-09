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

#[derive(Debug, Clone, PartialEq)]
pub(super) enum AdaptiveCueDirective {
    Play { cue_id: String, state_id: String },
    StopNow,
}
