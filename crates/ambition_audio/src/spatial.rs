//! Shared distance mixing for active one-shot and sustained sound voices.

use std::time::Duration;

use ambition_sfx::{AudioContextOwner, SfxEmitter, SfxListeners};
use bevy::{
    asset::{Assets, Handle},
    ecs::system::SystemParam,
    prelude::*,
};
use bevy_kira_audio::prelude::{
    AudioChannel, AudioControl, AudioInstance, AudioSource, AudioTween, PlaybackState,
};

use crate::{
    library::{amplitude_to_decibels, SfxChannel},
    selection::ActiveAudioSelection,
};

/// One accepted voice. Its source stays at its emission position; a loop's
/// next hold can move or reshape it without restarting the clip.
#[derive(Clone, Debug)]
pub struct SfxVoice {
    pub owner: AudioContextOwner,
    pub emitter: SfxEmitter,
    pub gain: f32,
    source_gain: f32,
    state: VoiceOutput,
    source: Handle<AudioSource>,
    loop_fade: Option<f32>,
    stop_requested: Option<f32>,
    began: Duration,
    duration: Option<Duration>,
    applied_gain: f32,
}

#[derive(Clone, Debug)]
enum VoiceOutput {
    Recording,
    Pending,
    Playing(Handle<AudioInstance>),
}

/// Accepted one-shots that have not ended. Recording uses the clip duration.
#[derive(Resource, Default, Debug)]
pub struct SfxOneShots(pub Vec<SfxVoice>);

#[derive(SystemParam)]
pub struct SfxVoiceOutput<'w> {
    pub(crate) channel: Res<'w, AudioChannel<SfxChannel>>,
    listeners: Res<'w, SfxListeners>,
    mix: Res<'w, crate::SfxMix>,
    output: Option<Res<'w, crate::output::AudioOutputMode>>,
    time: Res<'w, Time<Real>>,
}

impl SfxVoiceOutput<'_> {
    pub(crate) fn now(&self) -> Duration {
        self.time.elapsed()
    }

    fn gain(&self, emitter: SfxEmitter) -> f32 {
        self.listeners.gain(emitter) * self.mix.effective_sfx.clamp(0.0, 1.0)
    }

    pub(crate) fn start(
        &self,
        owner: AudioContextOwner,
        source: Handle<AudioSource>,
        emitter: SfxEmitter,
        duration: Duration,
        loop_fade: Option<f32>,
        source_gain: f32,
    ) -> SfxVoice {
        let source_gain = if source_gain.is_finite() { source_gain.clamp(0.0, 1.0) } else { 0.0 };
        let gain = self.gain(emitter) * source_gain;
        let state = if crate::output::emits_to_device(self.output.as_deref()) {
            VoiceOutput::Pending
        } else {
            VoiceOutput::Recording
        };
        SfxVoice {
            owner,
            emitter,
            gain,
            source_gain,
            state,
            source,
            loop_fade,
            stop_requested: None,
            began: self.time.elapsed(),
            duration: loop_fade.is_none().then_some(duration),
            applied_gain: gain,
        }
    }
}

impl SfxVoice {
    pub fn instance(&self) -> Option<&Handle<AudioInstance>> {
        match &self.state {
            VoiceOutput::Playing(handle) => Some(handle),
            _ => None,
        }
    }

    pub(crate) fn request_stop(&mut self, fade: f32, now: Duration) {
        let fade = Duration::from_secs_f32(fade.max(0.0));
        let remaining = self
            .duration
            .map(|duration| duration.saturating_sub(now.saturating_sub(self.began)));
        self.duration = Some(remaining.map_or(fade, |remaining| remaining.min(fade)));
        self.began = now;
        self.stop_requested = Some(fade.as_secs_f32());
    }

    fn update(
        &mut self,
        output: &SfxVoiceOutput,
        selection: &ActiveAudioSelection,
        instances: &mut Assets<AudioInstance>,
    ) -> bool {
        let live = selection.accepts_request_owner(Some(self.owner));
        self.gain = if live { output.gain(self.emitter) * self.source_gain } else { 0.0 };
        match &self.state {
            VoiceOutput::Recording => {
                return live
                    && self
                        .duration
                        .is_none_or(|duration| output.now().saturating_sub(self.began) < duration)
            }
            VoiceOutput::Pending => {
                if !live || self.stop_requested.is_some() {
                    return false;
                }
                let mut command = output.channel.play(self.source.clone());
                command.with_volume(amplitude_to_decibels(self.gain));
                if let Some(fade) = self.loop_fade {
                    command
                        .looped()
                        .fade_in(AudioTween::linear(Duration::from_secs_f32(fade.max(0.0))));
                }
                self.state = VoiceOutput::Playing(command.handle());
                self.applied_gain = self.gain;
                return true;
            }
            VoiceOutput::Playing(_) => {}
        }
        let handle = self.instance().expect("this voice was started").clone();
        let Some(instance) = instances.get(&handle) else {
            return output.channel.state(&handle) == PlaybackState::Queued;
        };
        if instance.state() == PlaybackState::Stopped {
            return false;
        }
        if !live {
            instances
                .get_mut(&handle)
                .expect("the voice exists")
                .stop(AudioTween::linear(Duration::from_millis(30)));
            return false;
        }
        if let Some(fade) = self.stop_requested.take() {
            instances
                .get_mut(&handle)
                .expect("the voice exists")
                .stop(AudioTween::linear(Duration::from_secs_f32(fade)));
        }
        if (self.gain - self.applied_gain).abs() > 0.0001 {
            instances
                .get_mut(&handle)
                .expect("the voice exists")
                .set_decibels(
                    amplitude_to_decibels(self.gain),
                    AudioTween::linear(Duration::from_millis(30)),
                );
            self.applied_gain = self.gain;
        }
        true
    }
}

/// Keep attenuation current even when no new cue is emitted.
pub fn update_sfx_attenuation(
    output: SfxVoiceOutput,
    selection: Res<ActiveAudioSelection>,
    mut instances: ResMut<Assets<AudioInstance>>,
    mut shots: ResMut<SfxOneShots>,
    mut loops: ResMut<crate::sfx_loops::SfxLoopPlayback>,
) {
    shots
        .0
        .retain_mut(|voice| voice.update(&output, &selection, &mut instances));
    loops
        .held
        .retain(|_, held| held.voice.update(&output, &selection, &mut instances));
    loops
        .fading
        .retain_mut(|voice| voice.update(&output, &selection, &mut instances));
}
