//! Compare music requests with backend playback on the real clock.

use super::*;
use crate::selection::ActiveAudioSelection;
use crate::web_unlock::AudioUnlockState;
use bevy_kira_audio::{AudioInstance, PlaybackState};

/// A backend observation, separate from the director's requested track.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MusicPlaybackStatus {
    Playing,
    Recording,
    NoContext,
    DeliberateSilence,
    Muted,
    AwaitingGesture,
    OneShotComplete,
    DeviceSuspended,
    InitialPresentation,
    MissingRequest,
    MissingContent,
    Loading,
    BackendSilent,
}

impl MusicPlaybackStatus {
    fn unexpected(&self) -> bool {
        matches!(
            self,
            Self::MissingRequest | Self::MissingContent | Self::Loading | Self::BackendSilent
        )
    }
}

/// The latest observation and the duration of an unexplained music gap.
/// Backend playback does not prove that source samples or speakers are audible.
#[derive(Resource, Default, Debug)]
pub struct MusicPlaybackDiagnostics {
    pub expected: String,
    pub actual: String,
    pub section: Option<String>,
    pub status: Option<MusicPlaybackStatus>,
    pub base_playing: bool,
    pub layers_playing: usize,
    pub detail: String,
    pub gap_seconds: f64,
    pub longest_gap_seconds: f64,
    pub gap_count: u64,
    next_warning: f64,
    sampled_at: f64,
    observed_generation: Option<u64>,
}

impl MusicPlaybackDiagnostics {
    fn observe(&mut self, status: MusicPlaybackStatus, dt: f64) {
        if status.unexpected() {
            if self.gap_seconds == 0.0 {
                self.gap_count += 1;
                self.next_warning = 0.5;
            }
            self.gap_seconds += if self
                .status
                .as_ref()
                .is_some_and(MusicPlaybackStatus::unexpected)
            {
                dt
            } else {
                dt.min(0.25)
            };
            self.longest_gap_seconds = self.longest_gap_seconds.max(self.gap_seconds);
            if self.gap_seconds >= self.next_warning {
                warn!(target: MUSIC_LOG_TARGET,
                    "music_gap expected={} actual={} section={:?} status={:?} gap_s={:.3} base_playing={} layers_playing={} detail={}",
                    self.expected, self.actual, self.section, status, self.gap_seconds, self.base_playing, self.layers_playing, self.detail);
                self.next_warning = self.gap_seconds + 5.0;
            }
        } else if self.gap_seconds > 0.0 {
            info!(target: MUSIC_LOG_TARGET, "music_gap_end duration_s={:.3} expected={} status={:?}",
                self.gap_seconds, self.expected, status);
            self.gap_seconds = 0.0;
        }
        if self.status.as_ref() != Some(&status) {
            info!(target: MUSIC_LOG_TARGET,
                "music_playback expected={} section={:?} status={:?} base_playing={} layers_playing={}",
                self.expected, self.section, status, self.base_playing, self.layers_playing);
        }
        self.status = Some(status);
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct MusicObservation<'w> {
    intent: Res<'w, MusicIntent>,
    selection: Res<'w, ActiveAudioSelection>,
    playback: Option<Res<'w, MusicPlaybackState>>,
    library: Option<Res<'w, AudioLibrary>>,
    director: Option<Res<'w, MusicDirectorState>>,
    catalogs: Res<'w, AdaptiveMusicCatalogRegistry>,
    loaded: Option<Res<'w, LoadedMusicCueAssets>>,
    decoded: Res<'w, Assets<KiraAudioSource>>,
    asset_server: Res<'w, AssetServer>,
    base: Res<'w, AudioChannel<MusicChannel>>,
    instances: Res<'w, Assets<AudioInstance>>,
    mix: Res<'w, MusicMix>,
    unlock: Res<'w, AudioUnlockState>,
    output: Res<'w, crate::output::AudioOutputMode>,
    conditions: Option<Res<'w, crate::output::AudioPlaybackConditions>>,
}

/// Run outside gameplay gates so death, loading, and frontend gaps remain visible.
pub fn observe_music_playback(
    time: Res<Time<Real>>,
    mut report: ResMut<MusicPlaybackDiagnostics>,
    inputs: MusicObservation,
) {
    let MusicObservation {
        intent,
        selection,
        playback,
        library,
        director,
        catalogs,
        loaded,
        decoded,
        asset_server,
        base,
        instances,
        mix,
        unlock,
        output,
        conditions,
    } = inputs;
    let now = time.elapsed_secs_f64();
    let dt = now - report.sampled_at;
    if dt < 0.25 {
        return;
    }
    report.sampled_at = now;
    report.base_playing = playback
        .as_ref()
        .and_then(|p| p.instance())
        .and_then(|handle| instances.get(handle))
        .is_some_and(|voice| playing(voice.state()));
    if report.base_playing {
        report.observed_generation = playback.as_ref().map(|p| p.play_generation());
    }
    let old_actual = report.actual.clone();
    report.actual.clear();
    if let Some(playback) = playback.as_ref().filter(|_| report.base_playing) {
        report.actual = format!("track:{}", playback.active_track());
    }
    if let Some(director) = director.as_ref() {
        for (bank, voices) in director.instances.iter().enumerate() {
            if voices.iter().enumerate().any(|(slot, handle)| {
                director.current_gains[bank][slot] > 0.001
                    && handle
                        .as_ref()
                        .and_then(|h| instances.get(h))
                        .is_some_and(|voice| playing(voice.state()))
            }) {
                if let Some((provider, cue, section)) = &director.bank_sections[bank] {
                    report
                        .actual
                        .push_str(&format!(" provider:{provider} cue:{cue} section:{section}"));
                }
            }
        }
    }
    if old_actual != report.actual {
        info!(target: MUSIC_LOG_TARGET, "music_output actual={}", report.actual);
    }
    report.layers_playing = director.as_ref().map_or(0, |d| {
        d.instances
            .iter()
            .enumerate()
            .flat_map(|(bank, voices)| {
                voices
                    .iter()
                    .enumerate()
                    .map(move |(slot, voice)| (bank, slot, voice))
            })
            .filter(|(bank, slot, voice)| {
                d.current_gains[*bank][*slot] > 0.001
                    && voice
                        .as_ref()
                        .and_then(|h| instances.get(h))
                        .is_some_and(|voice| playing(voice.state()))
            })
            .count()
    });
    let old_expected = report.expected.clone();
    let old_section = report.section.clone();
    report.expected.clear();
    report.detail.clear();
    report.section = director.as_ref().and_then(|d| d.current_section_id.clone());

    let mut content_status = MusicPlaybackStatus::MissingRequest;
    if let Some(AdaptiveCueDirective::Play { cue_id, state_id }) = &intent.adaptive {
        report.expected = format!("cue:{cue_id} state:{state_id}");
        content_status = MusicPlaybackStatus::MissingContent;
        if intent.authority.allows_cue(cue_id) {
            if let Some((provider, cue)) = intent.provider_id.as_deref().and_then(|provider| {
                catalogs
                    .catalog_for(provider)
                    .and_then(|c| c.cue(cue_id))
                    .map(|cue| (provider, cue))
            }) {
                if cue
                    .state(state_id)
                    .and_then(|s| cue.section(&s.section_id))
                    .is_some()
                {
                    content_status = if loaded
                        .as_ref()
                        .is_some_and(|assets| assets.cue_ready(provider, cue, &decoded))
                    {
                        MusicPlaybackStatus::BackendSilent
                    } else {
                        if let Some(assets) = loaded.as_ref() {
                            report.detail = cue
                                .sections
                                .iter()
                                .find_map(|section| {
                                    section.sources.iter().find_map(|source| {
                                        let handle = assets.get(
                                            provider,
                                            cue_id,
                                            &section.id,
                                            &source.layer_id,
                                        )?;
                                        (!decoded.contains(handle.id())).then(|| {
                                            format!(
                                                "{} {:?}",
                                                cue.source_path(source),
                                                asset_server.get_load_state(handle.id())
                                            )
                                        })
                                    })
                                })
                                .unwrap_or_else(|| "cue sources not requested".into());
                        }
                        MusicPlaybackStatus::Loading
                    };
                }
            }
        }
    } else {
        let track = intent
            .simple_track_candidates
            .iter()
            .find(|id| {
                intent.authority.allows(id)
                    && library.as_ref().is_some_and(|l| l.track(id).is_some())
            })
            .map(String::as_str)
            .or_else(|| {
                intent
                    .simple_track_candidates
                    .iter()
                    .rev()
                    .find(|id| intent.authority.allows(id))
                    .map(String::as_str)
            })
            .or_else(|| selection.preferred_track())
            .or_else(|| {
                (!intent.authority.is_governed())
                    .then(|| {
                        playback
                            .as_ref()
                            .map(|p| p.active_track())
                            .filter(|id| !id.is_empty())
                    })
                    .flatten()
            });
        if let Some(id) = track {
            report.expected = format!("track:{id}");
            content_status = match library.as_ref().and_then(|l| l.track(id)) {
                None => MusicPlaybackStatus::MissingContent,
                Some(track) => match library.as_ref().and_then(|l| l.resolved_track_handle(id)) {
                    Some(handle) if decoded.contains(handle.id()) => {
                        report.detail = format!(
                            "base_instance={:?}",
                            playback
                                .as_ref()
                                .and_then(|p| p.instance())
                                .map(|h| base.state(h))
                        );
                        if track.one_shot
                            && playback.as_ref().is_some_and(|p| {
                                p.active_track() == id && report.observed_generation == Some(p.play_generation())
                            })
                            && !report.base_playing
                        {
                            MusicPlaybackStatus::OneShotComplete
                        } else {
                            MusicPlaybackStatus::BackendSilent
                        }
                    }
                    handle => {
                        report.detail = format!(
                            "{} {:?}",
                            track.asset_path(),
                            handle.map(|h| asset_server.get_load_state(h.id()))
                        );
                        MusicPlaybackStatus::Loading
                    }
                },
            };
        }
    }
    let deliberate = if intent.authority.is_governed() {
        intent.authority.is_deliberate_silence()
    } else {
        selection.music_authority().is_deliberate_silence()
    };
    let status = if !output.emits_to_device() {
        MusicPlaybackStatus::Recording
    } else if conditions.as_ref().is_some_and(|c| c.device_suspended) {
        MusicPlaybackStatus::DeviceSuspended
    } else if conditions
        .as_ref()
        .is_some_and(|c| !c.initial_presentation_ready)
    {
        MusicPlaybackStatus::InitialPresentation
    } else if deliberate {
        MusicPlaybackStatus::DeliberateSilence
    } else if !intent.authority.is_governed() && selection.owner().is_none() {
        MusicPlaybackStatus::NoContext
    } else if mix.effective_music() <= 0.0 {
        MusicPlaybackStatus::Muted
    } else if !unlock.unlocked {
        MusicPlaybackStatus::AwaitingGesture
    } else if report.base_playing || report.layers_playing > 0 {
        MusicPlaybackStatus::Playing
    } else {
        content_status
    };
    if old_expected != report.expected || old_section != report.section {
        info!(target: MUSIC_LOG_TARGET, "music_request expected={} section={:?}", report.expected, report.section);
    }
    report.observe(status, dt);
}

fn playing(state: PlaybackState) -> bool {
    matches!(
        state,
        PlaybackState::Playing { .. }
            | PlaybackState::Pausing { .. }
            | PlaybackState::Stopping { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gap_spans_loading_and_backend_silence_and_ends_on_playback() {
        let mut report = MusicPlaybackDiagnostics::default();
        report.observe(MusicPlaybackStatus::Loading, 0.25);
        report.observe(MusicPlaybackStatus::BackendSilent, 0.75);
        assert_eq!(report.gap_seconds, 1.0);
        assert_eq!(report.gap_count, 1);
        report.observe(MusicPlaybackStatus::Playing, 0.25);
        assert_eq!(report.gap_seconds, 0.0);
        assert_eq!(report.longest_gap_seconds, 1.0);
    }

    #[test]
    fn explicit_silence_and_non_device_output_do_not_count_as_gaps() {
        let mut report = MusicPlaybackDiagnostics::default();
        for status in [
            MusicPlaybackStatus::Recording,
            MusicPlaybackStatus::NoContext,
            MusicPlaybackStatus::DeliberateSilence,
            MusicPlaybackStatus::Muted,
            MusicPlaybackStatus::AwaitingGesture,
            MusicPlaybackStatus::OneShotComplete,
            MusicPlaybackStatus::DeviceSuspended,
            MusicPlaybackStatus::InitialPresentation,
        ] {
            report.observe(status, 5.0);
        }
        assert_eq!(report.gap_count, 0);
    }
}
