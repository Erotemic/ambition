//! Bounded music occurrences and exact voice control.

use super::*;
use bevy::asset::AssetApp;

mod native;
pub use native::NativeMusicGroups;

const MAX_GROUPS: usize = crate::music_policy::MAX_MUSIC_PLAYBACK_GROUPS;
const ADMISSION_TIMEOUT_SECONDS: f64 = 5.0;
const MAX_RETRIES: u8 = 1;

/// A native music clock with one tick per second.
#[derive(Resource)]
pub struct MusicTransportClock(pub(crate) bevy_kira_audio::kira::clock::ClockHandle);

impl MusicTransportClock {
    fn seconds(&self) -> f64 {
        let time = self.0.time();
        time.ticks as f64 + time.fraction
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MusicPlaybackOccurrence(pub u64);

/// A selected entry has its own completion and retry scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MusicEntry(pub u64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MusicStartAfter {
    pub occurrence: MusicPlaybackOccurrence,
    pub seconds: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MusicProduct {
    Track {
        provider: String,
        generation: u64,
        id: String,
    },
    Section {
        provider: String,
        generation: u64,
        cue: String,
        section: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MusicSourceShape {
    pub sample_rate: u32,
    pub frames: u64,
}

impl MusicSourceShape {
    fn decoded(asset: &KiraAudioSource) -> Self {
        let sample_rate = asset.sound.sample_rate;
        let frames = (asset.sound.duration().as_secs_f64() * f64::from(sample_rate)).round() as u64;
        Self {
            sample_rate,
            frames,
        }
    }
    fn seconds(self) -> f64 {
        self.frames as f64 / f64::from(self.sample_rate)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MusicVoiceSource {
    pub expected_shape: Option<MusicSourceShape>,
    pub source: Handle<KiraAudioSource>,
    pub gain: f32,
    pub slot: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MusicPlaybackRequest {
    pub start_after: Option<MusicStartAfter>,
    pub entry: MusicEntry,
    pub goal_revision: u64,
    pub product: MusicProduct,
    /// The state submitted with these voices.
    pub state_id: Option<String>,
    pub phase_policy: MusicPhasePolicy,
    pub voices: Vec<MusicVoiceSource>,
    pub looped: bool,
    pub fade_seconds: f32,
}

impl MusicPlaybackRequest {
    fn can_continue(&self, other: &Self) -> bool {
        self.entry == other.entry
            && self.start_after == other.start_after
            && self.product == other.product
            && self.looped == other.looped
            && self.voices.len() == other.voices.len()
            && self
                .voices
                .iter()
                .zip(&other.voices)
                .all(|(a, b)| a.source == b.source)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicTerminalReason {
    Completed,
    Replaced,
    Canceled,
    AdmissionAbsent,
    AdmissionTimeout,
    UnexpectedStop,
    SourceMismatch,
    MissedStart,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MusicRequestOutcome {
    #[default]
    Pending,
    Completed,
    Failed(MusicTerminalReason),
}
impl MusicRequestOutcome {
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicGroupPhase {
    Submitted,
    Running,
    Retiring,
    Canceled,
}

#[derive(Clone, Debug)]
struct MusicVoice {
    shape: Option<MusicSourceShape>,
    handle: Handle<AudioInstance>,
    gain: f32,
    last_gain: Option<f32>,
    stop_sent: bool,
}

#[derive(Clone, Copy, Debug)]
struct Envelope {
    from: f32,
    to: f32,
    began: f64,
    seconds: f32,
}

impl Envelope {
    fn value(self, now: f64) -> f32 {
        let fraction = if self.seconds <= 0.0 {
            1.0
        } else {
            ((now - self.began) as f32 / self.seconds).clamp(0.0, 1.0)
        };
        self.from + (self.to - self.from) * fraction
    }

    fn done(self, now: f64) -> bool {
        now - self.began >= f64::from(self.seconds)
    }
}

#[derive(Clone, Debug)]
pub struct MusicPlaybackGroup {
    retiring_for: Option<MusicPlaybackOccurrence>,
    /// The requested start time on the output's transport clock.
    pub(crate) transport_origin: Option<f64>,
    pub entry: MusicEntry,
    pub occurrence: MusicPlaybackOccurrence,
    pub product: MusicProduct,
    pub phase: MusicGroupPhase,
    /// The last common file cursor. No cursor is available until every voice plays.
    /// Native output supplies Kira positions; recording output uses its host clock.
    pub position_seconds: Option<f64>,
    request: MusicPlaybackRequest,
    voices: Vec<MusicVoice>,
    submitted_at: f64,
    envelope: Envelope,
    terminal: Option<MusicTerminalReason>,
}

impl MusicPlaybackGroup {
    pub(super) fn scheduled_state(&self) -> Option<&str> {
        self.request
            .start_after
            .and(self.request.state_id.as_deref())
    }
    pub(super) fn file_duration_seconds(&self) -> Option<f64> {
        let mut voices = self.voices.iter();
        let mut duration = voices.next()?.shape?.seconds();
        for voice in voices {
            duration = duration.min(voice.shape?.seconds());
        }
        Some(duration)
    }

    pub fn goal_revision(&self) -> u64 {
        self.request.goal_revision
    }

    pub fn reserved_voices(&self) -> usize {
        self.voices.len()
    }

    pub fn instances(&self) -> impl Iterator<Item = &Handle<AudioInstance>> {
        self.voices.iter().map(|voice| &voice.handle)
    }

    pub fn nonzero_gain_voice_count(&self) -> usize {
        if self.phase != MusicGroupPhase::Running {
            return 0;
        }
        self.voices
            .iter()
            .filter(|voice| voice.last_gain.is_some_and(|gain| gain > 0.001))
            .count()
    }
}

/// One pending choice and at most two submitted, running, or retiring groups.
#[derive(Resource, Default, Debug)]
pub struct MusicPlayback {
    desired: Option<MusicPlaybackRequest>,
    groups: [Option<MusicPlaybackGroup>; MAX_GROUPS],
    current: Option<MusicPlaybackOccurrence>,
    current_completion: Option<(MusicEntry, MusicProduct)>,
    next_occurrence: u64,
    next_entry: u64,
    pub revision: u64,
    retries: u8,
    outcome: MusicRequestOutcome,
    pub last_terminal: Option<(MusicPlaybackOccurrence, MusicTerminalReason)>,
}

impl MusicPlayback {
    pub(crate) fn new_entry(&mut self) -> MusicEntry {
        self.next_entry = self
            .next_entry
            .checked_add(1)
            .expect("music entry identity exhausted");
        MusicEntry(self.next_entry)
    }

    pub fn outcome(&self) -> MusicRequestOutcome {
        self.outcome
    }

    pub(super) fn completed_product(&self) -> Option<(MusicEntry, &MusicProduct)> {
        self.current_completion
            .as_ref()
            .map(|(entry, product)| (*entry, product))
    }

    /// Withdraw unsent work. An admitted outgoing group can finish or be replaced.
    pub(crate) fn defer(&mut self) {
        if self.desired.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
        }
        self.outcome = MusicRequestOutcome::Pending;
        for group in self
            .groups
            .iter_mut()
            .flatten()
            .filter(|group| group.phase == MusicGroupPhase::Submitted)
        {
            group.phase = MusicGroupPhase::Canceled;
            group.terminal = Some(MusicTerminalReason::Canceled);
        }
    }

    pub fn groups(&self) -> impl Iterator<Item = &MusicPlaybackGroup> {
        self.groups.iter().filter_map(Option::as_ref)
    }

    pub fn current(&self) -> Option<&MusicPlaybackGroup> {
        self.groups()
            .find(|group| Some(group.occurrence) == self.current)
    }

    pub fn desired_is_running(&self) -> bool {
        self.current().is_some_and(|group| {
            group.phase == MusicGroupPhase::Running
                && self
                    .desired
                    .as_ref()
                    .is_some_and(|request| request.can_continue(&group.request))
        })
    }

    pub fn reserved_voices(&self) -> usize {
        self.groups().map(MusicPlaybackGroup::reserved_voices).sum()
    }

    pub(crate) fn request(&mut self, request: MusicPlaybackRequest) {
        assert!(!request.voices.is_empty() && request.voices.len() <= MAX_LAYERS);
        if self
            .desired
            .as_ref()
            .is_some_and(|old| old.can_continue(&request))
        {
            for group in self.groups.iter_mut().flatten() {
                if group.request.can_continue(&request)
                    && matches!(
                        group.phase,
                        MusicGroupPhase::Running | MusicGroupPhase::Submitted
                    )
                {
                    group.request.goal_revision = request.goal_revision;
                    group.request.state_id = request.state_id.clone();
                    group.request.phase_policy = request.phase_policy;
                    for (voice, source) in group.voices.iter_mut().zip(&request.voices) {
                        voice.gain = source.gain;
                    }
                }
            }
            self.desired = Some(request);
            return;
        }
        self.desired = Some(request);
        self.revision = self.revision.wrapping_add(1);
        self.retries = 0;
        self.outcome = MusicRequestOutcome::Pending;
    }

    pub(crate) fn set_section_gains(
        &mut self,
        entry: MusicEntry,
        goal_revision: u64,
        provider: &str,
        cue: &str,
        section: &str,
        gains: &[f32],
        association: Option<(&str, MusicPhasePolicy)>,
    ) {
        if association.is_some() && self.desired.as_ref().is_none_or(|request| {
            request.entry != entry || !matches!(&request.product,
                MusicProduct::Section { provider: p, cue: c, section: s, .. } if p == provider && c == cue && s == section)
        }) {
            if let Some(request) = self.current().filter(|group| group.entry == entry && matches!(&group.product,
                MusicProduct::Section { provider: p, cue: c, section: s, .. } if p == provider && c == cue && s == section))
                .map(|group| group.request.clone()) {
                self.request(request);
            }
        }
        let Some(request) = &mut self.desired else {
            return;
        };
        if request.entry != entry
            || !matches!(&request.product, MusicProduct::Section { provider: p, cue: c, section: s, .. }
            if p == provider && c == cue && s == section)
        {
            return;
        }
        request.goal_revision = goal_revision;
        if let Some((state, policy)) = association {
            request.state_id = Some(state.to_owned());
            request.phase_policy = policy;
        }
        for voice in &mut request.voices {
            voice.gain = gains[voice.slot];
        }
        for group in self.groups.iter_mut().flatten() {
            if group.request.can_continue(request)
                && matches!(
                    group.phase,
                    MusicGroupPhase::Running | MusicGroupPhase::Submitted
                )
            {
                group.request.goal_revision = goal_revision;
                group.request.state_id = request.state_id.clone();
                group.request.phase_policy = request.phase_policy;
                for (voice, source) in group.voices.iter_mut().zip(&request.voices) {
                    voice.gain = source.gain;
                }
            }
        }
    }

    pub(crate) fn silence(&mut self) {
        self.current_completion = None;
        self.outcome = MusicRequestOutcome::Pending;
        if self.desired.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
        }
        self.current = None;
        for group in self.groups.iter_mut().flatten() {
            group.phase = MusicGroupPhase::Canceled;
            group.terminal = Some(MusicTerminalReason::Canceled);
        }
    }

    fn failed(&mut self, occurrence: MusicPlaybackOccurrence, reason: MusicTerminalReason) {
        self.last_terminal = Some((occurrence, reason));
        if self.retries < MAX_RETRIES {
            self.retries += 1;
        } else {
            self.outcome = MusicRequestOutcome::Failed(reason);
        }
    }

    fn advance(&mut self, output: &mut impl MusicVoiceOutput, now: f64, master: f32) {
        let transport_now = output.transport_time();
        let envelope_now = transport_now.unwrap_or(now);
        for slot in 0..MAX_GROUPS {
            let Some(mut group) = self.groups[slot].take() else {
                continue;
            };
            let desired = self
                .desired
                .as_ref()
                .is_some_and(|request| request.can_continue(&group.request));
            if group.phase == MusicGroupPhase::Submitted && !desired {
                group.phase = MusicGroupPhase::Canceled;
                group.terminal = Some(MusicTerminalReason::Canceled);
            }
            let mut observations = [VoiceObservation::Absent; MAX_LAYERS];
            for ((state, voice), source) in observations
                .iter_mut()
                .zip(&mut group.voices)
                .zip(&group.request.voices)
            {
                *state = output.observe(&voice.handle);
                // Keep the first backend source facts through asset reload.
                if voice.shape.is_none()
                    && matches!(
                        *state,
                        VoiceObservation::Paused | VoiceObservation::Playing(_)
                    )
                {
                    voice.shape = output.shape(&voice.handle, &source.source);
                }
            }
            let observations = &observations[..group.voices.len()];
            let shapes_ready = group.voices.iter().all(|voice| voice.shape.is_some());
            if group.phase == MusicGroupPhase::Submitted && shapes_ready {
                let common = group.voices[0].shape;
                let mismatch =
                    group
                        .voices
                        .iter()
                        .zip(&group.request.voices)
                        .any(|(voice, source)| {
                            voice
                                .shape
                                .is_some_and(|shape| shape.frames == 0 || shape.sample_rate == 0)
                                || voice.shape != common
                                || source
                                    .expected_shape
                                    .is_some_and(|expected| Some(expected) != voice.shape)
                        });
                if mismatch {
                    group.phase = MusicGroupPhase::Canceled;
                    group.terminal = Some(MusicTerminalReason::SourceMismatch);
                    self.failed(group.occurrence, MusicTerminalReason::SourceMismatch);
                }
            }
            if !output.ready_to_start() && group.phase == MusicGroupPhase::Submitted {
                group.submitted_at = now;
            }
            if group.phase == MusicGroupPhase::Submitted {
                if observations.iter().any(|state| {
                    matches!(state, VoiceObservation::Absent | VoiceObservation::Stopped)
                }) {
                    group.phase = MusicGroupPhase::Canceled;
                    group.terminal = Some(MusicTerminalReason::AdmissionAbsent);
                    self.failed(group.occurrence, MusicTerminalReason::AdmissionAbsent);
                } else if group.transport_origin.is_none()
                    && observations
                        .iter()
                        .all(|state| matches!(state, VoiceObservation::Paused))
                    && output.ready_to_start()
                    && shapes_ready
                {
                    let origin = group.request.start_after.and_then(|start| {
                        self.groups()
                            .find(|source| source.occurrence == start.occurrence)
                            .and_then(|source| source.transport_origin)
                            .map(|origin| origin + start.seconds)
                    });
                    if group.request.start_after.is_some()
                        && origin.zip(transport_now).is_none_or(|(origin, current)| {
                            origin < current + output.start_lead_seconds()
                        })
                    {
                        group.phase = MusicGroupPhase::Canceled;
                        group.terminal = Some(MusicTerminalReason::MissedStart);
                        self.failed(group.occurrence, MusicTerminalReason::MissedStart);
                    } else {
                        group.transport_origin = origin.or_else(|| {
                            transport_now.map(|time| time + output.start_lead_seconds())
                        });
                        group.envelope.began = group.transport_origin.unwrap_or(envelope_now);
                        for voice in &group.voices {
                            output.gain(&voice.handle, master * voice.gain);
                            output.resume(
                                &voice.handle,
                                group.transport_origin,
                                group.request.fade_seconds,
                            );
                        }
                        if output.native_envelopes() {
                            if let Some((start, origin)) =
                                group.request.start_after.zip(group.transport_origin)
                            {
                                for previous in self
                                    .groups
                                    .iter_mut()
                                    .flatten()
                                    .filter(|previous| previous.occurrence == start.occurrence)
                                {
                                    previous.retiring_for = Some(group.occurrence);
                                    output.retire_group(
                                        previous.occurrence,
                                        Some(origin),
                                        group.request.fade_seconds,
                                    );
                                }
                            }
                        }
                        if group.transport_origin.is_none() {
                            // Outputs without a native clock retain immediate admission.
                            group.phase = MusicGroupPhase::Running;
                        }
                    }
                } else if now - group.submitted_at >= ADMISSION_TIMEOUT_SECONDS
                    && group.transport_origin.is_none()
                {
                    group.phase = MusicGroupPhase::Canceled;
                    group.terminal = Some(MusicTerminalReason::AdmissionTimeout);
                    self.failed(group.occurrence, MusicTerminalReason::AdmissionTimeout);
                }
                if group
                    .transport_origin
                    .zip(transport_now)
                    .is_some_and(|(origin, time)| time > origin + ADMISSION_TIMEOUT_SECONDS)
                    && !observations
                        .iter()
                        .all(|state| matches!(state, VoiceObservation::Playing(_)))
                {
                    group.phase = MusicGroupPhase::Canceled;
                    group.terminal = Some(MusicTerminalReason::AdmissionTimeout);
                    self.failed(group.occurrence, MusicTerminalReason::AdmissionTimeout);
                }
            }
            let start_observed = group.transport_origin.is_some_and(|origin| {
                transport_now.is_some_and(|time| time >= origin)
                    && (!output.native_envelopes()
                        || observations
                            .iter()
                            .all(|state| matches!(state, VoiceObservation::Playing(_))))
            });
            if desired
                && (start_observed || group.phase == MusicGroupPhase::Running)
                && self.current != Some(group.occurrence)
                && !matches!(
                    group.phase,
                    MusicGroupPhase::Canceled | MusicGroupPhase::Retiring
                )
            {
                if let Some(old) = self.current {
                    for previous in self
                        .groups
                        .iter_mut()
                        .flatten()
                        .filter(|other| other.occurrence == old)
                    {
                        let scheduled = previous.retiring_for == Some(group.occurrence);
                        previous.retiring_for = None;
                        previous.phase = MusicGroupPhase::Retiring;
                        previous.terminal = Some(MusicTerminalReason::Replaced);
                        previous.envelope = Envelope {
                            from: previous.envelope.value(envelope_now),
                            to: 0.0,
                            began: if scheduled {
                                group.transport_origin.unwrap_or(envelope_now)
                            } else {
                                envelope_now
                            },
                            seconds: group.request.fade_seconds,
                        };
                        if output.native_envelopes() && !scheduled {
                            output.retire_group(
                                previous.occurrence,
                                None,
                                group.request.fade_seconds,
                            );
                        }
                    }
                }
                group.phase = MusicGroupPhase::Running;
                self.current_completion = None;
                self.current = Some(group.occurrence);
            }
            if matches!(
                group.phase,
                MusicGroupPhase::Running | MusicGroupPhase::Retiring
            ) {
                group.position_seconds =
                    observations
                        .iter()
                        .try_fold(f64::INFINITY, |position, state| match state {
                            VoiceObservation::Playing(next) | VoiceObservation::Stopping(next)
                                if next.is_finite() && *next >= 0.0 =>
                            {
                                Some(position.min(*next))
                            }
                            _ => None,
                        });
            }
            if group.phase == MusicGroupPhase::Running {
                let terminal = observations.iter().any(|state| {
                    matches!(state, VoiceObservation::Absent | VoiceObservation::Stopped)
                });
                if terminal {
                    let complete = !group.request.looped
                        && observations
                            .iter()
                            .all(|state| matches!(state, VoiceObservation::Stopped));
                    let reason = if complete {
                        MusicTerminalReason::Completed
                    } else {
                        MusicTerminalReason::UnexpectedStop
                    };
                    group.phase = MusicGroupPhase::Canceled;
                    group.terminal = Some(reason);
                    if self.current == Some(group.occurrence) {
                        if complete {
                            self.current_completion = Some((group.entry, group.product.clone()));
                        }
                        self.current = None;
                    }
                    if desired {
                        if complete {
                            self.outcome = MusicRequestOutcome::Completed;
                            self.last_terminal = Some((group.occurrence, reason));
                        } else {
                            self.failed(group.occurrence, reason);
                        }
                    }
                }
            }
            if group.phase == MusicGroupPhase::Retiring && group.envelope.done(envelope_now) {
                group.phase = MusicGroupPhase::Canceled;
            }
            if group.phase == MusicGroupPhase::Canceled {
                // A canceled successor cannot retain its predecessor's stop.
                for previous in self
                    .groups
                    .iter_mut()
                    .flatten()
                    .filter(|previous| previous.retiring_for == Some(group.occurrence))
                {
                    previous.retiring_for = None;
                    output.cancel_group_retirement(previous.occurrence);
                }
                let mut accounted = true;
                for (voice, observation) in group.voices.iter_mut().zip(observations) {
                    match observation {
                        VoiceObservation::Queued => accounted = false,
                        VoiceObservation::Absent | VoiceObservation::Stopped => {
                            output.release(&voice.handle)
                        }
                        _ => {
                            accounted = false;
                            if !voice.stop_sent {
                                output.stop(&voice.handle);
                                voice.stop_sent = true;
                            }
                        }
                    }
                }
                if accounted {
                    if self.current == Some(group.occurrence) {
                        self.current = None;
                    }
                    if let Some(reason) = group.terminal {
                        self.last_terminal = Some((group.occurrence, reason));
                    }
                    output.end_group(group.occurrence);
                    continue;
                }
            } else if matches!(
                group.phase,
                MusicGroupPhase::Running | MusicGroupPhase::Retiring | MusicGroupPhase::Submitted
            ) {
                let envelope = if output.native_envelopes() {
                    1.0
                } else {
                    group.envelope.value(envelope_now)
                };
                for voice in &mut group.voices {
                    let gain = master * voice.gain * envelope;
                    if voice.last_gain != Some(gain) {
                        output.gain(&voice.handle, gain);
                        voice.last_gain = Some(gain);
                    }
                }
            }
            self.groups[slot] = Some(group);
        }
        let Some(request) = self.desired.as_ref() else {
            return;
        };
        if !output.ready_to_start()
            || self.outcome.is_terminal()
            || self.groups().any(|group| {
                group.request.can_continue(request)
                    && matches!(
                        group.phase,
                        MusicGroupPhase::Running | MusicGroupPhase::Submitted
                    )
            })
        {
            return;
        }
        let Some(slot) = self.groups.iter().position(Option::is_none) else {
            return;
        };
        if !request
            .voices
            .iter()
            .all(|voice| output.ready(&voice.source))
        {
            return;
        }
        self.next_occurrence = self
            .next_occurrence
            .checked_add(1)
            .expect("music occurrence space exhausted");
        let occurrence = MusicPlaybackOccurrence(self.next_occurrence);
        if !output.begin_group(occurrence) {
            self.failed(occurrence, MusicTerminalReason::AdmissionAbsent);
            return;
        }
        let request = self.desired.as_ref().expect("a selected request");
        let voices = request
            .voices
            .iter()
            .map(|source| MusicVoice {
                shape: None,
                handle: output.submit(occurrence, &source.source, request.looped),
                gain: source.gain,
                last_gain: None,
                stop_sent: false,
            })
            .collect();
        self.groups[slot] = Some(MusicPlaybackGroup {
            retiring_for: None,
            transport_origin: None,
            entry: request.entry,
            occurrence,
            product: request.product.clone(),
            phase: MusicGroupPhase::Submitted,
            position_seconds: None,
            request: request.clone(),
            voices,
            submitted_at: now,
            envelope: Envelope {
                from: 0.0,
                to: 1.0,
                began: now,
                seconds: request.fade_seconds,
            },
            terminal: None,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum VoiceObservation {
    Queued,
    Absent,
    Paused,
    Waiting,
    Playing(f64),
    Stopping(f64),
    Stopped,
}

trait MusicVoiceOutput {
    fn transport_time(&self) -> Option<f64> {
        None
    }
    fn start_lead_seconds(&self) -> f64 {
        0.0
    }
    fn native_envelopes(&self) -> bool {
        false
    }
    fn begin_group(&mut self, _id: MusicPlaybackOccurrence) -> bool {
        true
    }
    fn end_group(&mut self, _id: MusicPlaybackOccurrence) {}
    fn retire_group(&mut self, _id: MusicPlaybackOccurrence, _start: Option<f64>, _seconds: f32) {}
    fn cancel_group_retirement(&mut self, _id: MusicPlaybackOccurrence) {}
    fn ready_to_start(&self) -> bool {
        true
    }
    fn ready(&self, source: &Handle<KiraAudioSource>) -> bool;
    fn submit(
        &mut self,
        id: MusicPlaybackOccurrence,
        source: &Handle<KiraAudioSource>,
        looped: bool,
    ) -> Handle<AudioInstance>;
    fn observe(&self, handle: &Handle<AudioInstance>) -> VoiceObservation;
    fn shape(
        &self,
        handle: &Handle<AudioInstance>,
        source: &Handle<KiraAudioSource>,
    ) -> Option<MusicSourceShape>;
    fn resume(&mut self, handle: &Handle<AudioInstance>, start: Option<f64>, fade: f32);
    fn gain(&mut self, handle: &Handle<AudioInstance>, gain: f32);
    fn stop(&mut self, handle: &Handle<AudioInstance>);
    fn release(&mut self, handle: &Handle<AudioInstance>);
}

#[derive(Debug)]
struct RecordedVoice {
    started: Option<f64>,
    shape: MusicSourceShape,
    looped: bool,
    stopped: bool,
}

/// Bounded output state for hosts without a device drain.
#[derive(Resource, Default, Debug)]
pub struct RecordedMusicOutput {
    voices: HashMap<bevy::asset::AssetId<AudioInstance>, RecordedVoice>,
    pub submitted: u64,
    pub resumed: u64,
    pub gain_changes: u64,
    pub stopped: u64,
}

struct KiraMusicOutput<'a> {
    clock: Option<&'a MusicTransportClock>,
    native: &'a mut NativeMusicGroups,
    backend: Option<&'a mut bevy_kira_audio::AudioOutput>,
    decoded: &'a Assets<KiraAudioSource>,
    instances: &'a mut Assets<AudioInstance>,
    recording: &'a mut RecordedMusicOutput,
    mode: crate::output::AudioOutputMode,
    now: f64,
    can_start: bool,
}

impl MusicVoiceOutput for KiraMusicOutput<'_> {
    fn transport_time(&self) -> Option<f64> {
        if self.mode.emits_to_device() {
            self.clock.map(MusicTransportClock::seconds)
        } else {
            Some(self.now)
        }
    }

    fn start_lead_seconds(&self) -> f64 {
        // This lead defines a new origin. It does not move an existing exit.
        if self.mode.emits_to_device() {
            0.03
        } else {
            0.0
        }
    }

    fn native_envelopes(&self) -> bool {
        self.mode.emits_to_device()
    }

    fn begin_group(&mut self, id: MusicPlaybackOccurrence) -> bool {
        !self.mode.emits_to_device() || self.native.begin(id, self.backend.as_deref_mut())
    }

    fn end_group(&mut self, id: MusicPlaybackOccurrence) {
        self.native.end(id);
    }

    fn retire_group(&mut self, id: MusicPlaybackOccurrence, start: Option<f64>, seconds: f32) {
        let start = self.clock.zip(start).map_or(
            bevy_kira_audio::kira::StartTime::Immediate,
            |(clock, start)| {
                bevy_kira_audio::kira::clock::ClockTime::from_ticks_f64(&clock.0, start).into()
            },
        );
        self.native.retire(id, start, seconds);
    }

    fn cancel_group_retirement(&mut self, id: MusicPlaybackOccurrence) {
        self.native.cancel_retirement(id);
    }

    fn shape(
        &self,
        handle: &Handle<AudioInstance>,
        _source: &Handle<KiraAudioSource>,
    ) -> Option<MusicSourceShape> {
        if self.mode.emits_to_device() {
            self.native.shape(handle)
        } else {
            self.recording
                .voices
                .get(&handle.id())
                .map(|voice| voice.shape)
        }
    }

    fn ready_to_start(&self) -> bool {
        self.can_start
    }
    fn ready(&self, source: &Handle<KiraAudioSource>) -> bool {
        self.decoded.contains(source.id())
    }

    fn submit(
        &mut self,
        id: MusicPlaybackOccurrence,
        source: &Handle<KiraAudioSource>,
        looped: bool,
    ) -> Handle<AudioInstance> {
        self.recording.submitted += 1;
        if self.mode.emits_to_device() {
            let handle = self.instances.reserve_handle();
            if let Some(instance) = self.native.submit(
                id,
                &handle,
                self.decoded.get(source).expect("decoded before submission"),
                looped,
            ) {
                self.instances
                    .insert(handle.id(), instance)
                    .expect("a reserved native instance");
            }
            handle
        } else {
            let handle = self.instances.reserve_handle();
            let asset = self.decoded.get(source).expect("decoded before submission");
            self.recording.voices.insert(
                handle.id(),
                RecordedVoice {
                    started: None,
                    shape: MusicSourceShape::decoded(asset),
                    looped,
                    stopped: false,
                },
            );
            handle
        }
    }

    fn observe(&self, handle: &Handle<AudioInstance>) -> VoiceObservation {
        if !self.mode.emits_to_device() {
            return match self.recording.voices.get(&handle.id()) {
                None => VoiceObservation::Absent,
                Some(voice) if voice.stopped => VoiceObservation::Stopped,
                Some(voice) => match voice.started {
                    None => VoiceObservation::Paused,
                    Some(started) if self.now < started => VoiceObservation::Waiting,
                    Some(started)
                        if !voice.looped && self.now - started >= voice.shape.seconds() =>
                    {
                        VoiceObservation::Stopped
                    }
                    Some(started) => VoiceObservation::Playing(if voice.looped {
                        (self.now - started).rem_euclid(voice.shape.seconds())
                    } else {
                        self.now - started
                    }),
                },
            };
        }
        match self.instances.get(handle).map(AudioInstance::state) {
            Some(PlaybackState::Playing { position } | PlaybackState::Resuming { position }) => {
                VoiceObservation::Playing(position)
            }
            Some(PlaybackState::Paused { .. } | PlaybackState::Pausing { .. }) => {
                VoiceObservation::Paused
            }
            Some(PlaybackState::WaitingToResume { .. }) => VoiceObservation::Waiting,
            Some(PlaybackState::Stopping { position }) => VoiceObservation::Stopping(position),
            Some(PlaybackState::Stopped) => VoiceObservation::Stopped,
            _ => VoiceObservation::Absent,
        }
    }

    fn resume(&mut self, handle: &Handle<AudioInstance>, start: Option<f64>, fade: f32) {
        self.recording.resumed += 1;
        if self.mode.emits_to_device() {
            if let Some(mut voice) = self.instances.get_mut(handle) {
                voice.set_playback_rate(1.0, AudioTween::linear(std::time::Duration::ZERO));
                let tween = AudioTween::linear(std::time::Duration::from_secs_f32(fade));
                if let Some((clock, start)) = self.clock.zip(start) {
                    voice.resume_at(
                        bevy_kira_audio::kira::clock::ClockTime::from_ticks_f64(&clock.0, start)
                            .into(),
                        tween,
                    );
                } else {
                    voice.resume(tween);
                }
            }
        } else if let Some(voice) = self.recording.voices.get_mut(&handle.id()) {
            voice.started = Some(start.unwrap_or(self.now));
        }
    }

    fn gain(&mut self, handle: &Handle<AudioInstance>, gain: f32) {
        self.recording.gain_changes += 1;
        if self.mode.emits_to_device() {
            if let Some(mut voice) = self.instances.get_mut(handle) {
                voice.set_decibels(amplitude_to_decibels(gain), AudioTween::default());
            }
        }
    }

    fn stop(&mut self, handle: &Handle<AudioInstance>) {
        self.recording.stopped += 1;
        if self.mode.emits_to_device() {
            if let Some(mut voice) = self.instances.get_mut(handle) {
                voice.stop(AudioTween::default());
            }
        } else if let Some(voice) = self.recording.voices.get_mut(&handle.id()) {
            voice.stopped = true;
        }
    }

    fn release(&mut self, handle: &Handle<AudioInstance>) {
        self.recording.voices.remove(&handle.id());
        if self.mode.emits_to_device() {
            self.instances.remove(handle.id());
        }
    }
}

/// Observe the last drain and admit bounded work on the output's clock.
pub fn advance_music_playback(
    mut playback: ResMut<MusicPlayback>,
    mut recording: ResMut<RecordedMusicOutput>,
    decoded: Res<Assets<KiraAudioSource>>,
    mut instances: ResMut<Assets<AudioInstance>>,
    time: Res<Time<Real>>,
    mix: Res<MusicMix>,
    output: Option<Res<crate::output::AudioOutputMode>>,
    mut base: Option<ResMut<MusicPlaybackState>>,
    mut director: Option<ResMut<MusicDirectorState>>,
    unlock: Option<Res<crate::web_unlock::AudioUnlockState>>,
    conditions: Option<Res<crate::output::AudioPlaybackConditions>>,
    clock: Option<Res<MusicTransportClock>>,
    mut native: ResMut<NativeMusicGroups>,
    mut backend: Option<NonSendMut<bevy_kira_audio::AudioOutput>>,
) {
    let mode = output.as_deref().copied().unwrap_or_default();
    let can_start = !mode.emits_to_device()
        || (unlock.as_ref().is_none_or(|state| state.unlocked)
            && conditions
                .as_ref()
                .is_none_or(|conditions| !conditions.device_suspended));
    let mut sink = KiraMusicOutput {
        clock: clock.as_deref(),
        native: &mut native,
        backend: backend.as_deref_mut(),
        decoded: &decoded,
        instances: &mut instances,
        recording: &mut recording,
        mode,
        can_start,
        now: time.elapsed_secs_f64(),
    };
    let previous = playback.current().map(|group| group.occurrence);
    playback.advance(&mut sink, time.elapsed_secs_f64(), mix.effective_music());
    if let Some(director) = director.as_deref_mut() {
        if let Some(group) = playback.current().filter(|group| {
            group.phase == MusicGroupPhase::Running && Some(group.entry) == director.entry
        }) {
            director.position_seconds = group.position_seconds;
            if let Some(mut selection) = director.selection.clone() {
                // A queued section cannot replace the admitted section's state or policy.
                if let MusicProduct::Section { cue, section, .. } = &group.product {
                    if previous != Some(group.occurrence) && group.request.start_after.is_some() {
                        director.continuation_state_id = group.request.state_id.clone();
                    }
                    if director.current_section_id.as_ref() != Some(section) {
                        director.pending_state = None;
                    }
                    director.active_cue_id = Some(cue.clone());
                    director.current_section_id = Some(section.clone());
                    director.current_state_id = group.request.state_id.clone();
                    director.active_phase_policy = group.request.phase_policy;
                    director.mode = if director.prepared_cue.as_ref().is_some_and(|cue| {
                        cue.outro_state.is_some() && cue.outro_state == group.request.state_id
                    }) {
                        MusicDirectorMode::AdaptiveOutro
                    } else if group.request.looped {
                        MusicDirectorMode::AdaptiveLoop
                    } else {
                        MusicDirectorMode::AdaptiveIntro
                    };
                    if let MusicChoice::Cue { state, entry, .. } = &mut selection.choice {
                        *state = group
                            .request
                            .state_id
                            .clone()
                            .expect("a section has its state");
                        *entry = None;
                    }
                    selection.phase_policy = group.request.phase_policy;
                }
                director.admitted = Some(AdmittedMusic {
                    entry: group.entry,
                    selection,
                });
            }
        }
        if director.admitted.as_ref().is_some_and(|admitted| {
            !playback.groups().any(|group| {
                group.entry == admitted.entry && group.phase == MusicGroupPhase::Running
            })
        }) {
            director.admitted = None;
        }
        director.transport_completed = playback.completed_product().is_some_and(|(entry, product)|
            Some(entry) == director.entry && matches!(product,
                MusicProduct::Section { cue, section, .. } if director.active_cue_id.as_ref() == Some(cue) && director.current_section_id.as_ref() == Some(section)));
    }
    if let Some(base) = base.as_deref_mut() {
        let track = base.active_track().to_owned();
        base.set_instance(playback.current().and_then(|group| match &group.product {
            MusicProduct::Track { id, .. } if *id == track => group.instances().next().cloned(),
            _ => None,
        }));
    }
}

pub fn add_music_playback(app: &mut App) {
    app.init_resource::<MusicPlayback>()
        .init_resource::<RecordedMusicOutput>()
        .init_resource::<NativeMusicGroups>()
        .init_resource::<MusicMix>()
        .add_systems(
            PostUpdate,
            advance_music_playback
                .after(bevy_kira_audio::AudioSystemSet::PlayTypedChannels)
                .after(bevy_kira_audio::AudioSystemSet::PlayDynamicChannels),
        );
    if !app.world().contains_resource::<Assets<AudioInstance>>() {
        app.init_asset::<AudioInstance>();
    }
}

#[cfg(test)]
mod tests;
