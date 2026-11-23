//! Authored exits and limits for the native music transport.

use serde::{Deserialize, Serialize};

pub const MAX_MUSIC_PLAYBACK_GROUPS: usize = 2;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MusicExitRule {
    Immediate,
    AssetEnd,
    MusicalEnd,
    Bars(usize),
    Markers(Vec<String>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicTimingRequirement {
    #[default]
    FrameDriven,
    SampleAccurate,
    PhasePreserving,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicExhaustionPolicy {
    #[default]
    StartReadyTarget,
    Finish,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MusicPlaybackSpec {
    pub section_exit: Option<MusicExitRule>,
    pub ending_exit: Option<MusicExitRule>,
    pub section_crossfade_seconds: Option<f32>,
    pub ending_crossfade_seconds: Option<f32>,
    pub continuation_crossfade_seconds: Option<f32>,
    pub return_overlap_seconds: Option<f32>,
    pub ending_media_timeout_seconds: Option<f32>,
    pub timing: Option<MusicTimingRequirement>,
    pub exhausted: Option<MusicExhaustionPolicy>,
}

impl MusicPlaybackSpec {
    pub fn resolve(&self, base: MusicPlaybackPolicy) -> MusicPlaybackPolicy {
        MusicPlaybackPolicy {
            section_exit: self.section_exit.clone().unwrap_or(base.section_exit),
            ending_exit: self.ending_exit.clone().unwrap_or(base.ending_exit),
            section_crossfade_seconds: self
                .section_crossfade_seconds
                .unwrap_or(base.section_crossfade_seconds),
            ending_crossfade_seconds: self
                .ending_crossfade_seconds
                .unwrap_or(base.ending_crossfade_seconds),
            continuation_crossfade_seconds: self
                .continuation_crossfade_seconds
                .unwrap_or(base.continuation_crossfade_seconds),
            return_overlap_seconds: self
                .return_overlap_seconds
                .unwrap_or(base.return_overlap_seconds),
            ending_media_timeout_seconds: self
                .ending_media_timeout_seconds
                .unwrap_or(base.ending_media_timeout_seconds),
            timing: self.timing.unwrap_or(base.timing),
            exhausted: self.exhausted.unwrap_or(base.exhausted),
        }
    }
}

impl From<MusicPlaybackPolicy> for MusicPlaybackSpec {
    fn from(policy: MusicPlaybackPolicy) -> Self {
        Self {
            section_exit: Some(policy.section_exit),
            ending_exit: Some(policy.ending_exit),
            section_crossfade_seconds: Some(policy.section_crossfade_seconds),
            ending_crossfade_seconds: Some(policy.ending_crossfade_seconds),
            continuation_crossfade_seconds: Some(policy.continuation_crossfade_seconds),
            return_overlap_seconds: Some(policy.return_overlap_seconds),
            ending_media_timeout_seconds: Some(policy.ending_media_timeout_seconds),
            timing: Some(policy.timing),
            exhausted: Some(policy.exhausted),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MusicPlaybackPolicy {
    pub section_exit: MusicExitRule,
    pub ending_exit: MusicExitRule,
    pub section_crossfade_seconds: f32,
    pub ending_crossfade_seconds: f32,
    pub continuation_crossfade_seconds: f32,
    pub return_overlap_seconds: f32,
    pub ending_media_timeout_seconds: f32,
    pub timing: MusicTimingRequirement,
    pub exhausted: MusicExhaustionPolicy,
}

impl Default for MusicPlaybackPolicy {
    fn default() -> Self {
        Self {
            section_exit: MusicExitRule::Bars(1),
            ending_exit: MusicExitRule::Bars(2),
            section_crossfade_seconds: 1.70,
            ending_crossfade_seconds: 1.65,
            continuation_crossfade_seconds: 0.0,
            return_overlap_seconds: 2.25,
            ending_media_timeout_seconds: 5.0,
            timing: MusicTimingRequirement::FrameDriven,
            exhausted: MusicExhaustionPolicy::StartReadyTarget,
        }
    }
}

impl MusicPlaybackPolicy {
    pub fn whole_assets() -> Self {
        Self {
            section_exit: MusicExitRule::Immediate,
            ending_exit: MusicExitRule::Immediate,
            ..Self::default()
        }
    }
}
