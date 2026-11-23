use super::playback::{MusicPlayback, MusicPlaybackRequest, MusicProduct, MusicVoiceSource};
use super::*;

/// Submit musical choices to the common occurrence owner.
#[derive(SystemParam)]
pub struct MusicPlaybackCommands<'w> {
    playback: ResMut<'w, MusicPlayback>,
}

impl MusicPlaybackCommands<'_> {
    pub(super) fn scheduling_window(&self) -> Option<super::playback::MusicSchedulingWindow> {
        self.playback.scheduling
    }
    pub(super) fn new_entry(&mut self) -> super::playback::MusicEntry {
        self.playback.new_entry()
    }

    pub(super) fn current(&self) -> Option<&super::playback::MusicPlaybackGroup> {
        self.playback.current()
    }

    pub(super) fn section_group(
        &self,
        entry: super::playback::MusicEntry,
        cue: &str,
        section: &str,
    ) -> Option<&super::playback::MusicPlaybackGroup> {
        self.playback.groups().find(|group| {
            group.entry == entry
                && matches!(group.phase, super::playback::MusicGroupPhase::Running | super::playback::MusicGroupPhase::Retiring)
                && matches!(&group.product, MusicProduct::Section { cue: c, section: s, .. } if c == cue && s == section)
        })
    }

    pub(super) fn outcome(&self) -> super::playback::MusicRequestOutcome {
        self.playback.outcome()
    }
    pub(super) fn pending_for(&self, entry: super::playback::MusicEntry) -> bool {
        self.playback.groups().any(|group| {
            group.entry == entry && group.phase == super::playback::MusicGroupPhase::Submitted
        })
    }

    pub(super) fn scheduled_state(&self, entry: super::playback::MusicEntry, state: &str) -> bool {
        self.playback.groups().any(|group| {
            group.entry == entry
                && group.phase == super::playback::MusicGroupPhase::Submitted
                && group.scheduled_state() == Some(state)
        })
    }

    pub(super) fn defer(&mut self) {
        self.playback.defer();
    }

    pub(super) fn silence(&mut self) {
        self.playback.silence();
    }

    pub(super) fn set_section_gains(
        &mut self,
        entry: super::playback::MusicEntry,
        goal_revision: u64,
        provider: &str,
        cue: &str,
        section: &str,
        gains: &[f32],
        association: Option<(&str, MusicPhasePolicy)>,
    ) {
        self.playback.set_section_gains(
            entry,
            goal_revision,
            provider,
            cue,
            section,
            gains,
            association,
        );
    }

    pub(super) fn play_section(
        &mut self,
        entry: super::playback::MusicEntry,
        goal_revision: u64,
        selection: &MusicSelection,
        cue: &PreparedMusicCue,
        section: &PreparedMusicSection,
        assets: &LoadedMusicCueAssets,
        state: &PreparedMusicState,
        fade_seconds: f32,
        start_after: Option<super::playback::MusicStartAfter>,
    ) {
        let voices = section
            .sources
            .iter()
            .filter_map(|source| {
                let slot = source.slot;
                Some(MusicVoiceSource {
                    expected_shape: source.product.as_ref().map(|product| {
                        super::playback::MusicSourceShape {
                            sample_rate: product.sample_rate,
                            frames: product.frames,
                        }
                    }),
                    source: assets.get(
                        &selection.provider,
                        selection.generation,
                        &cue.id,
                        &section.id,
                        &source.layer_id,
                    )?,
                    gain: state.gains[slot],
                    slot,
                })
            })
            .collect::<Vec<_>>();
        if voices.len() != section.sources.len() || voices.is_empty() {
            return;
        }
        self.playback.request(MusicPlaybackRequest {
            start_after,
            entry,
            goal_revision,
            product: MusicProduct::Section {
                provider: selection.provider.clone(),
                generation: selection.generation,
                cue: cue.id.clone(),
                section: section.id.clone(),
            },
            state_id: Some(state.id.clone()),
            phase_policy: selection.phase_policy,
            voices,
            looped: section.looped,
            fade_seconds,
        });
    }

    pub(crate) fn play_track(
        &mut self,
        entry: super::playback::MusicEntry,
        goal_revision: u64,
        provider: &str,
        generation: u64,
        track: &str,
        source: Handle<KiraAudioSource>,
        one_shot: bool,
        fade_seconds: f32,
    ) {
        self.playback.request(MusicPlaybackRequest {
            start_after: None,
            entry,
            goal_revision,
            product: MusicProduct::Track {
                provider: provider.into(),
                generation,
                id: track.into(),
            },
            state_id: None,
            phase_policy: MusicPhasePolicy::default(),
            voices: vec![MusicVoiceSource {
                expected_shape: None,
                source,
                gain: 1.0,
                slot: 0,
            }],
            looped: !one_shot,
            fade_seconds,
        });
    }
}
