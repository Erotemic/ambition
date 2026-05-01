//! Procedural audio for Ambition sandbox feedback and music.
//!
//! The current implementation synthesizes tiny WAV files into in-memory Bevy
//! `AudioSource` handles. This keeps the project assetless while leaving a clear
//! seam for a future Kira/CPAL-backed `ambition_audio` crate. The background
//! track is intentionally low-key, lo-fi, and loopable rather than busy or shrill.

use bevy::audio::AudioSource;
use bevy::prelude::*;
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundCue {
    Jump,
    DoubleJump,
    Dash,
    Blink,
    PrecisionBlink,
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
    blink: Handle<AudioSource>,
    precision_blink: Handle<AudioSource>,
    slash: Handle<AudioSource>,
    hit: Handle<AudioSource>,
    pogo: Handle<AudioSource>,
    reset: Handle<AudioSource>,
    death: Handle<AudioSource>,
    respawn: Handle<AudioSource>,
    ambience: Handle<AudioSource>,
}

impl SoundBank {
    pub fn new(audio_sources: &mut Assets<AudioSource>) -> Self {
        let mut add = |spec: SynthSpec| audio_sources.add(AudioSource { bytes: synth_wav_bytes(spec, 44_100).into() });
        Self {
            jump: add(SynthSpec::jump()),
            double_jump: add(SynthSpec::double_jump()),
            dash: add(SynthSpec::dash()),
            blink: add(SynthSpec::blink()),
            precision_blink: add(SynthSpec::precision_blink()),
            slash: add(SynthSpec::slash()),
            hit: add(SynthSpec::hit()),
            pogo: add(SynthSpec::pogo()),
            reset: add(SynthSpec::reset()),
            death: add(SynthSpec::death()),
            respawn: add(SynthSpec::respawn()),
            ambience: audio_sources.add(AudioSource { bytes: synth_lofi_theme_wav_bytes(44_100).into() }),
        }
    }

    pub fn ambience(&self) -> Handle<AudioSource> {
        self.ambience.clone()
    }

    pub fn get(&self, cue: SoundCue) -> Handle<AudioSource> {
        match cue {
            SoundCue::Jump => self.jump.clone(),
            SoundCue::DoubleJump => self.double_jump.clone(),
            SoundCue::Dash => self.dash.clone(),
            SoundCue::Blink => self.blink.clone(),
            SoundCue::PrecisionBlink => self.precision_blink.clone(),
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

/// Start the generated background music loop.
///
/// This intentionally uses Bevy's built-in audio for now. Once we need
/// cross-fades, parameter automation, or layered adaptive music, this is the
/// seam where Kira should replace the current simple playback path. The public
/// function keeps its original name so the call sites do not churn yet.
pub fn play_ambience(commands: &mut Commands, bank: &SoundBank) {
    commands.spawn((
        AudioPlayer::new(bank.ambience()),
        PlaybackSettings::LOOP,
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
    fn blink() -> Self {
        Self::tone(Waveform::Triangle, 740.0, 260.0, 0.090, 0.18)
    }
    fn precision_blink() -> Self {
        Self {
            noise: 0.08,
            ..Self::tone(Waveform::Sine, 880.0, 180.0, 0.160, 0.20)
        }
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

fn synth_lofi_theme_wav_bytes(sample_rate: u32) -> Vec<u8> {
    let bpm = 72.0f32;
    let seconds_per_beat = 60.0 / bpm;
    let total_beats = 32.0f32;
    let seconds = total_beats * seconds_per_beat;
    let sample_count = (seconds * sample_rate as f32).round() as usize;
    let mut pcm: Vec<i16> = Vec::with_capacity(sample_count * 2);
    let mut noise_state = 0x45d9_f3bu32;
    let mut lowpass_left = 0.0f32;
    let mut lowpass_right = 0.0f32;

    for i in 0..sample_count {
        let t = i as f32 / sample_rate as f32;
        let loop_beat = (t / seconds_per_beat).rem_euclid(total_beats);
        let seam_fade = (loop_beat.min(total_beats - loop_beat) * 3.0).clamp(0.0, 1.0);
        let mut left = 0.0f32;
        let mut right = 0.0f32;

        mix_stereo(
            &mut left,
            &mut right,
            lofi_chord_pad(loop_beat, seconds_per_beat, t),
            -0.10,
        );
        mix_stereo(
            &mut left,
            &mut right,
            note_sequence_voice(&LOFI_LEAD, loop_beat, seconds_per_beat, 220.0, Waveform::Triangle, 0.5, t),
            0.12,
        );
        mix_stereo(
            &mut left,
            &mut right,
            lofi_soft_keys(loop_beat, seconds_per_beat, t),
            0.18,
        );
        mix_stereo(
            &mut left,
            &mut right,
            lofi_bass(loop_beat, seconds_per_beat, t),
            -0.04,
        );
        mix_stereo(
            &mut left,
            &mut right,
            lofi_dusty_drums(loop_beat, seconds_per_beat, t, &mut noise_state),
            0.02,
        );

        left += tape_hiss(&mut noise_state) * 0.0025;
        right += tape_hiss(&mut noise_state) * 0.0025;

        // A one-pole low-pass and soft clip move the generated loop away from
        // bright chip leads and toward a warmer lo-fi demo-tape texture.
        lowpass_left += 0.075 * (left - lowpass_left);
        lowpass_right += 0.075 * (right - lowpass_right);
        left = soft_clip(lowpass_left * seam_fade * 0.78).clamp(-1.0, 1.0);
        right = soft_clip(lowpass_right * seam_fade * 0.78).clamp(-1.0, 1.0);
        pcm.push((left * i16::MAX as f32) as i16);
        pcm.push((right * i16::MAX as f32) as i16);
    }

    wav_bytes_from_stereo_i16(&pcm, sample_rate)
}

// Sparse A-minor-ish motif. Keep this understated: a few lower-register notes
// are enough when the sandbox is mostly about movement feel.
const LOFI_LEAD: [(f32, f32, i32, f32); 14] = [
    (2.00, 1.40, 0, 0.030),
    (4.00, 1.10, 3, 0.028),
    (6.00, 1.70, 7, 0.027),
    (10.00, 1.20, 5, 0.026),
    (12.00, 1.80, 3, 0.026),
    (15.00, 1.00, 0, 0.024),
    (18.00, 1.40, 7, 0.028),
    (20.00, 1.10, 10, 0.026),
    (22.00, 1.60, 7, 0.026),
    (25.00, 1.20, 5, 0.024),
    (27.00, 1.20, 3, 0.024),
    (29.00, 1.00, 0, 0.022),
    (30.50, 0.75, -2, 0.020),
    (31.25, 0.65, 0, 0.018),
];

fn note_sequence_voice(
    notes: &[(f32, f32, i32, f32)],
    loop_beat: f32,
    seconds_per_beat: f32,
    root_hz: f32,
    waveform: Waveform,
    duty: f32,
    time_seconds: f32,
) -> f32 {
    let mut sample = 0.0f32;

    for &(start_beat, duration_beats, semitone, volume) in notes {
        let end_beat = start_beat + duration_beats;
        if loop_beat < start_beat || loop_beat >= end_beat {
            continue;
        }

        let local_time = (loop_beat - start_beat) * seconds_per_beat;
        let duration = duration_beats * seconds_per_beat;
        let wow = 1.0 + 0.002 * (TAU * 0.31 * time_seconds).sin();
        let freq = semitone_frequency(root_hz, semitone) * wow;
        let phase = freq * local_time;
        let rounded = chip_wave(phase, waveform, duty) * 0.82 + (phase.fract() * TAU).sin() * 0.18;
        sample += rounded * note_envelope(local_time, duration, 0.045, 0.260) * volume;
    }

    sample
}

const LOFI_CHORDS: [[i32; 4]; 8] = [
    [0, 3, 7, 10],
    [-5, -2, 2, 7],
    [-3, 0, 3, 7],
    [-7, -4, 0, 5],
    [0, 3, 7, 12],
    [-5, -2, 2, 7],
    [-8, -5, -1, 3],
    [-7, -4, 0, 5],
];

const LOFI_BASS_ROOTS: [i32; 8] = [0, -5, -3, -7, 0, -5, -8, -7];

fn lofi_chord_pad(loop_beat: f32, seconds_per_beat: f32, time_seconds: f32) -> f32 {
    let bar = ((loop_beat / 4.0).floor() as usize).min(LOFI_CHORDS.len() - 1);
    let local_time = (loop_beat - (bar as f32 * 4.0)) * seconds_per_beat;
    let duration = 4.0 * seconds_per_beat;
    let mut sample = 0.0f32;

    for (voice, semitone) in LOFI_CHORDS[bar].iter().enumerate() {
        let detune = 1.0 + (voice as f32 - 1.5) * 0.0015;
        let wow = 1.0 + 0.0025 * (TAU * (0.09 + voice as f32 * 0.017) * time_seconds).sin();
        let freq = semitone_frequency(220.0, *semitone) * detune * wow;
        let phase = freq * local_time;
        let tri = chip_wave(phase, Waveform::Triangle, 0.5);
        let sine = (phase.fract() * TAU).sin();
        sample += (tri * 0.45 + sine * 0.55) * 0.012;
    }

    sample * note_envelope(local_time, duration, 0.180, 0.700)
}

fn lofi_soft_keys(loop_beat: f32, seconds_per_beat: f32, time_seconds: f32) -> f32 {
    // Lazy off-beat key stabs: less sparkle than an arpeggio, more pulse than a pad.
    let bar = ((loop_beat / 4.0).floor() as usize).min(LOFI_CHORDS.len() - 1);
    let half_step = (loop_beat * 2.0).floor() as i32;
    if half_step.rem_euclid(4) != 1 {
        return 0.0;
    }

    let step_start = half_step as f32 * 0.5;
    let local_time = (loop_beat - step_start) * seconds_per_beat;
    let step_index = ((half_step / 2) as usize + bar) % 4;
    let semitone = LOFI_CHORDS[bar][step_index];
    let freq = semitone_frequency(220.0, semitone + 12);
    let wobble = 1.0 + 0.0030 * (TAU * 0.65 * time_seconds).sin();
    let phase = freq * local_time * wobble;
    let rounded = chip_wave(phase, Waveform::Triangle, 0.5) * 0.70 + (phase.fract() * TAU).sin() * 0.30;
    rounded * note_envelope(local_time, 0.42 * seconds_per_beat, 0.025, 0.180) * 0.020
}

fn lofi_bass(loop_beat: f32, seconds_per_beat: f32, time_seconds: f32) -> f32 {
    let bar = ((loop_beat / 4.0).floor() as usize).min(LOFI_BASS_ROOTS.len() - 1);
    let beat_in_bar = loop_beat - (bar as f32 * 4.0);
    let beat_floor = beat_in_bar.floor();
    let local_time = (beat_in_bar - beat_floor) * seconds_per_beat;
    let chord_root = LOFI_BASS_ROOTS[bar];
    let semitone = match beat_floor as i32 {
        0 => chord_root,
        1 => chord_root,
        2 => chord_root + 7,
        _ => chord_root,
    };
    let freq = semitone_frequency(55.0, semitone) * (1.0 + 0.0015 * (TAU * 0.22 * time_seconds).sin());
    (freq * local_time * TAU).sin()
        * note_envelope(local_time, 0.86 * seconds_per_beat, 0.020, 0.210)
        * 0.055
}

fn lofi_dusty_drums(
    loop_beat: f32,
    seconds_per_beat: f32,
    time_seconds: f32,
    noise_state: &mut u32,
) -> f32 {
    *noise_state = (*noise_state).wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let noise = (((*noise_state >> 8) as f32 / 0x00ff_ffff as f32) * 2.0) - 1.0;
    let beat_floor = loop_beat.floor();
    let beat_frac = loop_beat - beat_floor;
    let beat_in_bar = beat_floor as i32 % 4;
    let mut sample = 0.0f32;

    if beat_frac < 0.20 && beat_in_bar == 0 {
        let local_time = beat_frac * seconds_per_beat;
        let env = (1.0 - beat_frac / 0.20).clamp(0.0, 1.0).powf(2.4);
        sample += (TAU * (52.0 - 12.0 * beat_frac) * local_time).sin() * env * 0.050;
    }

    if beat_frac < 0.18 && beat_in_bar == 2 {
        let env = (1.0 - beat_frac / 0.18).clamp(0.0, 1.0).powf(2.2);
        let body = (TAU * 145.0 * beat_frac * seconds_per_beat).sin() * env * 0.012;
        sample += noise * env * 0.018 + body;
    }

    let quarter_frac = loop_beat.fract();
    if quarter_frac < 0.10 {
        let env = (1.0 - quarter_frac / 0.10).clamp(0.0, 1.0).powf(2.0);
        let sway = 0.55 + 0.45 * (TAU * time_seconds / 10.0).sin().abs();
        sample += noise * env * 0.0035 * sway;
    }

    sample
}

fn tape_hiss(noise_state: &mut u32) -> f32 {
    *noise_state = (*noise_state).wrapping_mul(1_103_515_245).wrapping_add(12_345);
    (((*noise_state >> 8) as f32 / 0x00ff_ffff as f32) * 2.0) - 1.0
}

fn soft_clip(sample: f32) -> f32 {
    sample / (1.0 + sample.abs() * 0.35)
}

fn semitone_frequency(root_hz: f32, semitone: i32) -> f32 {
    root_hz * 2.0f32.powf(semitone as f32 / 12.0)
}

fn chip_wave(phase: f32, waveform: Waveform, duty: f32) -> f32 {
    let p = phase.fract();
    match waveform {
        Waveform::Sine => (p * TAU).sin(),
        Waveform::Square => {
            if p < duty.clamp(0.05, 0.95) { 1.0 } else { -1.0 }
        }
        Waveform::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
        Waveform::Saw => 2.0 * p - 1.0,
    }
}

fn note_envelope(local_time: f32, duration: f32, attack: f32, release: f32) -> f32 {
    if duration <= 0.0 || local_time < 0.0 || local_time > duration {
        return 0.0;
    }
    if attack > 0.0 && local_time < attack {
        return (local_time / attack).clamp(0.0, 1.0);
    }
    let release_start = (duration - release).max(attack);
    if release > 0.0 && local_time > release_start {
        return ((duration - local_time) / release).clamp(0.0, 1.0);
    }
    1.0
}

fn mix_stereo(left: &mut f32, right: &mut f32, sample: f32, pan: f32) {
    let pan = pan.clamp(-1.0, 1.0);
    *left += sample * (1.0 - pan * 0.35);
    *right += sample * (1.0 + pan * 0.35);
}

fn wav_bytes_from_stereo_i16(pcm: &[i16], sample_rate: u32) -> Vec<u8> {
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
