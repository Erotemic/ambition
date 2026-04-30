//! Procedural audio for Ambition sandbox feedback.
//!
//! The current implementation synthesizes tiny WAV files into in-memory Bevy
//! `AudioSource` handles. This keeps the project assetless while leaving a clear
//! seam for a future Kira/CPAL-backed `ambition_audio` crate.

use bevy::audio::AudioSource;
use bevy::prelude::*;
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundCue {
    Jump,
    DoubleJump,
    Dash,
    Slash,
    Hit,
    Pogo,
    Reset,
    Death,
    Respawn,
}

#[derive(Resource)]
pub struct SoundBank {
    jump: Handle<AudioSource>,
    double_jump: Handle<AudioSource>,
    dash: Handle<AudioSource>,
    slash: Handle<AudioSource>,
    hit: Handle<AudioSource>,
    pogo: Handle<AudioSource>,
    reset: Handle<AudioSource>,
    death: Handle<AudioSource>,
    respawn: Handle<AudioSource>,
}

impl SoundBank {
    pub fn new(audio_sources: &mut Assets<AudioSource>) -> Self {
        let mut add = |spec: SynthSpec| audio_sources.add(AudioSource { bytes: synth_wav_bytes(spec, 44_100).into() });
        Self {
            jump: add(SynthSpec::jump()),
            double_jump: add(SynthSpec::double_jump()),
            dash: add(SynthSpec::dash()),
            slash: add(SynthSpec::slash()),
            hit: add(SynthSpec::hit()),
            pogo: add(SynthSpec::pogo()),
            reset: add(SynthSpec::reset()),
            death: add(SynthSpec::death()),
            respawn: add(SynthSpec::respawn()),
        }
    }

    pub fn get(&self, cue: SoundCue) -> Handle<AudioSource> {
        match cue {
            SoundCue::Jump => self.jump.clone(),
            SoundCue::DoubleJump => self.double_jump.clone(),
            SoundCue::Dash => self.dash.clone(),
            SoundCue::Slash => self.slash.clone(),
            SoundCue::Hit => self.hit.clone(),
            SoundCue::Pogo => self.pogo.clone(),
            SoundCue::Reset => self.reset.clone(),
            SoundCue::Death => self.death.clone(),
            SoundCue::Respawn => self.respawn.clone(),
        }
    }
}

pub fn play_sound(commands: &mut Commands, bank: &SoundBank, cue: SoundCue) {
    commands.spawn((
        AudioPlayer::new(bank.get(cue)),
        PlaybackSettings::DESPAWN,
    ));
}

#[derive(Clone, Copy, Debug)]
enum Waveform {
    Sine,
    Square,
    Triangle,
    Saw,
}

#[derive(Clone, Copy, Debug)]
struct SynthSpec {
    waveform: Waveform,
    frequency: f32,
    frequency_end: f32,
    duration: f32,
    volume: f32,
    attack: f32,
    release: f32,
    noise: f32,
}

impl SynthSpec {
    fn jump() -> Self {
        Self::tone(Waveform::Sine, 460.0, 720.0, 0.085, 0.22)
    }
    fn double_jump() -> Self {
        Self::tone(Waveform::Triangle, 520.0, 940.0, 0.115, 0.22)
    }
    fn dash() -> Self {
        Self::tone(Waveform::Saw, 260.0, 110.0, 0.105, 0.18)
    }
    fn slash() -> Self {
        Self::tone(Waveform::Square, 620.0, 340.0, 0.075, 0.16)
    }
    fn hit() -> Self {
        Self {
            noise: 0.44,
            ..Self::tone(Waveform::Triangle, 220.0, 88.0, 0.105, 0.26)
        }
    }
    fn pogo() -> Self {
        Self::tone(Waveform::Sine, 360.0, 880.0, 0.105, 0.22)
    }
    fn reset() -> Self {
        Self::tone(Waveform::Sine, 160.0, 90.0, 0.150, 0.16)
    }
    fn death() -> Self {
        Self {
            noise: 0.18,
            ..Self::tone(Waveform::Saw, 140.0, 48.0, 0.220, 0.24)
        }
    }
    fn respawn() -> Self {
        Self::tone(Waveform::Triangle, 440.0, 660.0, 0.145, 0.20)
    }
    fn tone(waveform: Waveform, frequency: f32, frequency_end: f32, duration: f32, volume: f32) -> Self {
        Self {
            waveform,
            frequency,
            frequency_end,
            duration,
            volume,
            attack: 0.003,
            release: 0.045,
            noise: 0.0,
        }
    }
}

fn sample_wave(phase: f32, waveform: Waveform) -> f32 {
    let p = phase.fract();
    match waveform {
        Waveform::Sine => (p * TAU).sin(),
        Waveform::Square => {
            if p < 0.5 { 1.0 } else { -1.0 }
        }
        Waveform::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
        Waveform::Saw => 2.0 * p - 1.0,
    }
}

fn envelope(index: usize, length: usize, attack: usize, release: usize) -> f32 {
    if attack > 0 && index < attack {
        return index as f32 / attack as f32;
    }
    if release > 0 && index >= length.saturating_sub(release) {
        return (length.saturating_sub(index)) as f32 / release as f32;
    }
    1.0
}

fn synth_wav_bytes(spec: SynthSpec, sample_rate: u32) -> Vec<u8> {
    let sample_count = ((spec.duration * sample_rate as f32).max(1.0)) as usize;
    let attack = (spec.attack * sample_rate as f32) as usize;
    let release = (spec.release * sample_rate as f32) as usize;
    let mut pcm: Vec<i16> = Vec::with_capacity(sample_count * 2);
    let mut phase = 0.0f32;
    let mut noise_state = 0x1234_5678u32;
    for i in 0..sample_count {
        let t = if sample_count > 1 {
            i as f32 / (sample_count - 1) as f32
        } else {
            0.0
        };
        let freq = spec.frequency + (spec.frequency_end - spec.frequency) * t;
        phase += freq / sample_rate as f32;
        let mut sample = sample_wave(phase, spec.waveform);
        if spec.noise > 0.0 {
            noise_state = noise_state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (((noise_state >> 8) as f32 / 0x00ff_ffff as f32) * 2.0) - 1.0;
            sample = sample * (1.0 - spec.noise) + n * spec.noise;
        }
        sample *= envelope(i, sample_count, attack, release) * spec.volume;
        let v = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        pcm.push(v);
        pcm.push(v);
    }
    let data_bytes = (pcm.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_bytes as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2 * 2).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in pcm {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}
