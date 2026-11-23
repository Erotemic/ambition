use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicDirectorMode {
    Idle,
    SimpleTrack,
    AdaptiveIntro,
    AdaptiveLoop,
    AdaptiveOutro,
}

/// Runtime entitlement. Authors name content, not these tokens.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MusicClaim {
    pub context: ambition_sfx::AudioContextOwner,
    pub session: Option<u64>,
    pub scope: Option<u32>,
    pub source: String,
    pub attempt: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MusicChoice {
    Track(String),
    Cue {
        id: String,
        state: String,
        entry: Option<String>,
    },
}

impl MusicChoice {
    pub fn program(&self) -> &str {
        match self {
            Self::Track(id) | Self::Cue { id, .. } => id,
        }
    }
    pub fn same_program(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Track(a), Self::Track(b)) => a == b,
            (Self::Cue { id: a, .. }, Self::Cue { id: b, .. }) => a == b,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicContinuity {
    SharedProgram,
    FreshOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicEntryPolicy {
    FreshOccurrence,
    ContinueCompatible,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MusicSelection {
    pub provider: String,
    pub generation: u64,
    pub claim: MusicClaim,
    /// The accepted lifecycle binding can differ from the music source.
    pub release_owner: Option<MusicClaim>,
    pub choice: MusicChoice,
    pub continuity: MusicContinuity,
    pub entry_policy: MusicEntryPolicy,
    pub priority: u16,
    pub crossfade_s: f32,
    pub completed_state: Option<String>,
}

impl MusicSelection {
    pub fn can_continue(&self, next: &Self) -> bool {
        self.provider == next.provider
            && self.generation == next.generation
            && self.choice.same_program(&next.choice)
            && (self.claim == next.claim
                || (self.claim.session == next.claim.session
                    && self.continuity == MusicContinuity::SharedProgram
                    && next.continuity == MusicContinuity::SharedProgram
                    && next.entry_policy == MusicEntryPolicy::ContinueCompatible
                    && (self.claim.context == next.claim.context
                        || matches!(
                            (self.claim.context, next.claim.context),
                            (
                                ambition_sfx::AudioContextOwner::Frontend(_),
                                ambition_sfx::AudioContextOwner::Frontend(_)
                            )
                        ))))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MusicSilenceReason {
    NoContext,
    ProviderHasNoMusic,
    FrontendHasNoMusic,
    NoEligibleChoice,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MusicGoal {
    Play(MusicSelection),
    Silence {
        context: Option<ambition_sfx::AudioContextOwner>,
        reason: MusicSilenceReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicReleaseReason {
    Completed,
    Failed,
    Reset,
    AwaitingSettlement,
    Withdrawn,
}

/// A disposition can affect only the playback entry it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicDisposition {
    pub claim: MusicClaim,
    pub entry: super::playback::MusicEntry,
    pub reason: MusicReleaseReason,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct MusicIntent {
    pub context: Option<ambition_sfx::AudioContextOwner>,
    pub provider_id: Option<String>,
    pub authority: crate::selection::MusicAuthority,
    pub goal: Option<MusicGoal>,
    pub revision: u64,
    pub outgoing: Option<MusicDisposition>,
}

impl MusicIntent {
    /// Missing projection keeps the last goal. Silence must be explicit.
    pub fn publish(&mut self, goal: MusicGoal) {
        if self.goal.as_ref() != Some(&goal) {
            self.revision = self.revision.wrapping_add(1);
            self.goal = Some(goal);
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdmittedMusic {
    pub entry: super::playback::MusicEntry,
    pub selection: MusicSelection,
}

#[derive(Debug, Clone)]
pub(super) struct MusicEnding {
    pub admitted: AdmittedMusic,
    pub cue: std::sync::Arc<PreparedMusicCue>,
    pub started: bool,
    pub elapsed: f64,
    pub duration: f64,
    pub waiting_seconds: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicEndingResult {
    Finished,
    Interrupted,
    MediaUnavailable,
    BackendFailure,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct MusicDirectorState {
    pub mode: MusicDirectorMode,
    pub active_cue_id: Option<String>,
    pub current_state_id: Option<String>,
    pub current_section_id: Option<String>,
    pub goal_revision: u64,
    pub last_ending_result: Option<(super::playback::MusicEntry, MusicEndingResult)>,
    pub position_seconds: Option<f64>,
    pub(super) selection: Option<MusicSelection>,
    pub(super) prepared_cue: Option<std::sync::Arc<PreparedMusicCue>>,
    pub(super) entry: Option<super::playback::MusicEntry>,
    pub(super) transport_completed: bool,
    pub(super) admitted: Option<AdmittedMusic>,
    pub(super) ending: Option<MusicEnding>,
    pub(super) requested_state_id: Option<String>,
    pub(super) continuation_state_id: Option<String>,
    pub(super) pending_state: Option<PendingMusicStateTransition>,
    pub(super) debug_log_timer: f32,
}

impl Default for MusicDirectorMode {
    fn default() -> Self {
        Self::Idle
    }
}

impl MusicDirectorState {
    pub fn admitted(&self) -> Option<&AdmittedMusic> {
        self.admitted.as_ref()
    }
    pub fn selection(&self) -> Option<&MusicSelection> {
        self.selection.as_ref()
    }
    pub fn has_ending(&self) -> bool {
        self.ending.is_some()
    }
    pub(super) fn clear_active_cue(&mut self) {
        self.transport_completed = false;
        self.prepared_cue = None;
        self.active_cue_id = None;
        self.current_state_id = None;
        self.current_section_id = None;
        self.requested_state_id = None;
        self.continuation_state_id = None;
        self.pending_state = None;
        self.position_seconds = None;
    }
}

#[derive(Debug, Clone)]
pub(super) struct PendingMusicStateTransition {
    pub(super) state_id: String,
    pub(super) occurrence: super::playback::MusicPlaybackOccurrence,
    pub(super) exit_seconds: f64,
    pub(super) observed_seconds: f64,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MusicGoalPublication;
