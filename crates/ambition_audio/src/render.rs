//! Provider-relative SFX source resolution and Kira adapters.
//!
//! The combined App may cache many providers' authored sources. Every
//! playback request carries a stable presentation-source id, and the active
//! session binds each authorized source to one provider registry or bank.
//! Procedural definitions synthesize from that source's
//! [`SfxRegistry`](crate::spec::SfxRegistry); packed entries decode from its
//! provider bank. So the primary provider cannot take over a same-named cue
//! from another cast member.

use ambition_sfx::{
    self as sfx, AudioContextOwner, PresentationSourceId, SfxId, SfxProvider,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_kira_audio::prelude::{
    AudioSource as KiraAudioSource, Frame, StaticSoundData, StaticSoundSettings,
};
use std::f32::consts::TAU;
use std::io::Cursor;
use std::sync::Arc;

use crate::spec::{SfxRegistry, SfxSpec, WaveformSpec};

pub fn audio_source_from_sfx_clip(clip: sfx::SfxClip) -> Result<KiraAudioSource, String> {
    let cursor = Cursor::new(clip.bytes.to_vec());
    let sound = StaticSoundData::from_cursor(cursor).map_err(|e| e.to_string())?;
    Ok(KiraAudioSource { sound })
}

/// Loudness target for procedural cues, matching the packed path, where
/// `tools/ambition_sfx_renderer` normalizes banked clips to a cue-family peak
/// ceiling between -6 and -11 dBFS.
///
/// Calibrated so the median procedural cue matches the median banked clip.
/// `python3 scripts/audio_levels.py` measures both and prints the gap; use it
/// to re-derive this value if the corpus changes.
pub const PROCEDURAL_CUE_REFERENCE_RMS_DBFS: f32 = -11.0;

/// Deterministically synthesize one provider-authored procedural cue.
///
/// [`SfxSpec::volume`] is a loudness trim, not a peak amplitude. The cue body
/// is rendered at unit scale, measured, and scaled so its RMS is `volume` of
/// [`PROCEDURAL_CUE_REFERENCE_RMS_DBFS`]. Two cues with the same `volume` are
/// equally loud whatever their waveform and noise mix. (All waveforms swing
/// ±1, but a square's RMS equals its peak, a sine's is 3 dB lower, and a
/// triangle's 4.8 dB lower.)
///
/// The target fixes the level of the body; the envelope applied afterwards is
/// sound design. The noise mix is included in the measurement, because noise
/// changes RMS.
pub fn audio_source_from_sfx_spec(spec: &SfxSpec, sample_rate: u32) -> KiraAudioSource {
    let sample_rate = sample_rate.max(8_000);
    let duration = spec.duration.max(0.01);
    let frame_count = ((duration * sample_rate as f32).ceil() as usize).max(2);
    let noise_mix = spec.noise.clamp(0.0, 1.0);

    // Pass 1: the cue's body — waveform and noise, at unit scale, unenveloped.
    let mut phase = 0.0_f32;
    let mut noise_state = 0x6d2b_79f5_u32;
    let mut body = Vec::with_capacity(frame_count);
    for index in 0..frame_count {
        let t = index as f32 / sample_rate as f32;
        let progress = (t / duration).clamp(0.0, 1.0);
        let frequency = spec.frequency + (spec.frequency_end - spec.frequency) * progress;
        phase = (phase + TAU * frequency.max(1.0) / sample_rate as f32) % TAU;
        let tone = match spec.waveform {
            WaveformSpec::Sine => phase.sin(),
            WaveformSpec::Square => {
                if phase.sin() >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            WaveformSpec::Triangle => {
                2.0 * (2.0 * (phase / TAU - (phase / TAU + 0.5).floor())).abs() - 1.0
            }
            WaveformSpec::Saw => 2.0 * (phase / TAU) - 1.0,
        };
        noise_state = noise_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let noise = ((noise_state >> 8) as f32 / 0x00ff_ffff as f32) * 2.0 - 1.0;
        body.push((1.0 - noise_mix) * tone + noise_mix * noise);
    }

    // Pass 2: one gain that puts that body on the target, then the envelope.
    let gain = procedural_body_gain(&body, spec.volume.clamp(0.0, 1.0));
    let attack = spec.attack.max(0.0);
    let release = spec.release.max(0.0);
    let release_start = (duration - release).max(0.0);
    let mut frames = Vec::with_capacity(frame_count);
    for (index, body) in body.iter().enumerate() {
        let t = index as f32 / sample_rate as f32;
        let attack_gain = if attack > 0.0 {
            (t / attack).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let release_gain = if release > 0.0 && t > release_start {
            ((duration - t) / release).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let sample = body * gain * attack_gain * release_gain;
        frames.push(Frame::new(sample, sample));
    }
    KiraAudioSource {
        sound: StaticSoundData {
            sample_rate,
            frames: Arc::from(frames.into_boxed_slice()),
            settings: StaticSoundSettings::default(),
            slice: None,
        },
    }
}

/// The single scalar that puts one rendered body on the loudness target.
///
/// Dividing by the body's own RMS makes `volume` mean loudness, whatever crest
/// factor the waveform and noise produce.
///
/// The `min` is a peak ceiling: an RMS target could ask a very peaky body for
/// more than full scale, and clipping is worse than a quiet cue. No shipped
/// cue reaches it (the loudest peaks at -11.4 dBFS; the peakiest possible body
/// at `volume = 1.0` stops near -3 dBFS).
fn procedural_body_gain(body: &[f32], volume: f32) -> f32 {
    if body.is_empty() {
        return 0.0;
    }
    let mean_square =
        body.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / body.len() as f64;
    let rms = mean_square.sqrt() as f32;
    let peak = body.iter().fold(0.0_f32, |max, s| max.max(s.abs()));
    if rms <= 0.0 || peak <= 0.0 {
        return 0.0;
    }
    let target_rms = volume * 10.0_f32.powf(PROCEDURAL_CUE_REFERENCE_RMS_DBFS / 20.0);
    (target_rms / rms).min(1.0 / peak)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SfxSourceKind {
    Procedural,
    Bank,
}

/// Stable identity of the authored source that produced one playback handle.
///
/// Independent of Bevy's opaque `Handle` allocation, so lifecycle tests can
/// prove that Sanic's procedural Dash did not resolve to Ambition's resident
/// sample with the same logical id.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SfxSourceIdentity {
    pub kind: SfxSourceKind,
    pub fingerprint: u64,
}

#[derive(Clone)]
pub struct ResolvedSfxHandle {
    pub handle: Handle<KiraAudioSource>,
    pub source: SfxSourceIdentity,
}

/// Why a cue produced no playable source.
///
/// The cases are different bugs with different fixes: a cue requested before
/// its bank loaded is not missing, and a clip that fails to decode is not
/// absent. The playback path reports them verbatim.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SfxSourceMiss {
    /// No bank is registered for this provider yet, and no procedural spec
    /// answers the cue. Possibly transient: the bank may still be loading, and
    /// [`ProviderSfxHandleCache`] caches nothing so the first request after
    /// it arrives succeeds.
    NoProviderBank,
    /// The provider declared a bank asset, but that bank file or its decoding
    /// failed before it could be registered. This is terminal for the current
    /// content, unlike [`Self::NoProviderBank`].
    BankLoadFailed,
    /// The provider's bank is loaded, has no entry for this cue, and no
    /// The bank is loaded, has no entry, and no procedural fallback exists.
    NotInBank,
    /// The bank has the clip but it does not decode, and there is no
    /// procedural fallback. The content exists and is broken.
    DecodeFailed,
}

impl std::fmt::Display for SfxSourceMiss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoProviderBank => "no bank is registered for this provider (yet)",
            Self::BankLoadFailed => "the provider's bank asset failed to load",
            Self::NotInBank => "the provider's bank has no entry for it",
            Self::DecodeFailed => {
                "the provider's bank entry failed to decode (the decoder's own \
                 message is at debug level)"
            }
        })
    }
}

/// Lazy provider-qualified handle cache. Missing sources are not cached so a
/// bank that arrives after activation becomes usable immediately.
#[derive(Resource, Default)]
pub struct ProviderSfxHandleCache {
    handles: HashMap<(String, SfxId), ResolvedSfxHandle>,
}

impl ProviderSfxHandleCache {
    /// The playable source for `id` under `provider_id`, or [`SfxSourceMiss`]
    /// saying which of the several distinct failures happened.
    pub fn handle_for(
        &mut self,
        provider_id: &str,
        id: SfxId,
        procedural: Option<&SfxRegistry>,
        bank: Option<&dyn SfxProvider>,
        bank_fingerprint: Option<u64>,
        audio_sources: &mut Assets<KiraAudioSource>,
    ) -> Result<ResolvedSfxHandle, SfxSourceMiss> {
        let key = (provider_id.to_owned(), id);
        if let Some(handle) = self.handles.get(&key) {
            if cached_sfx_source_is_current(handle.source, bank_fingerprint) {
                return Ok(handle.clone());
            }
            // Do not keep that fallback: the first request after the bank
            // arrives upgrades to the provider's authored clip.
            self.handles.remove(&key);
        }
        // Packed content is the highest-fidelity source. Procedural specs are
        // provider-local fallbacks, and the only source for providers (such as
        // Sanic) that ship no packed bank.
        let mut miss = match bank {
            None => SfxSourceMiss::NoProviderBank,
            Some(_) => SfxSourceMiss::NotInBank,
        };
        let from_bank = bank
            .and_then(|bank| bank.provide_clip(id))
            .and_then(|clip| match audio_source_from_sfx_clip(clip) {
                Ok(source) => Some(ResolvedSfxHandle {
                    handle: audio_sources.add(source),
                    source: SfxSourceIdentity {
                        kind: SfxSourceKind::Bank,
                        fingerprint: bank_fingerprint.unwrap_or_else(|| id.hash()),
                    },
                }),
                Err(error) => {
                    // `debug`, not `warn`: this runs on every request for a
                    // corrupt clip, and the caller reports it once by name.
                    debug!("provider '{provider_id}' SFX id {id} failed to decode ({error})");
                    miss = SfxSourceMiss::DecodeFailed;
                    None
                }
            });
        let resolved = from_bank.or_else(|| {
            procedural
                .and_then(|registry| registry.spec_for_id(id).map(|spec| (registry, spec)))
                .map(|(registry, spec)| ResolvedSfxHandle {
                    handle: audio_sources
                        .add(audio_source_from_sfx_spec(spec, registry.sample_rate)),
                    source: SfxSourceIdentity {
                        kind: SfxSourceKind::Procedural,
                        fingerprint: procedural_sfx_fingerprint(registry.sample_rate, spec),
                    },
                })
        });
        match resolved {
            Some(resolved) => {
                self.handles.insert(key, resolved.clone());
                Ok(resolved)
            }
            None => Err(miss),
        }
    }

    pub fn clear_provider(&mut self, provider_id: &str) {
        self.handles
            .retain(|(provider, _), _| provider != provider_id);
    }
}

fn cached_sfx_source_is_current(cached: SfxSourceIdentity, bank_fingerprint: Option<u64>) -> bool {
    match bank_fingerprint {
        Some(fingerprint) => {
            cached.kind == SfxSourceKind::Bank && cached.fingerprint == fingerprint
        }
        None => true,
    }
}

/// Observable fact written by the actual playback decision path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SfxPlaybackRecord {
    pub owner: AudioContextOwner,
    pub presentation_source: PresentationSourceId,
    pub provider_id: String,
    pub id: SfxId,
    pub source: SfxSourceIdentity,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct SfxPlaybackState {
    pub last_played: Option<SfxPlaybackRecord>,
    /// Number of requests accepted by the real playback decision path.
    ///
    /// Unlike `last_played`, not cleared on an audio-context transition, so a
    /// stable diagnostic of total accepted playback.
    ///
    /// Not a per-request check: a live session plays its own cues. To prove a
    /// request was refused, assert the matching rejection counter's exact
    /// increment; `audio_play_sfx_messages` sends each message down exactly
    /// one branch.
    pub accepted_playbacks: u64,
    pub rejected_wrong_owner: u64,
    pub rejected_unauthorized: u64,
    pub missing_source: u64,
    /// WHICH cues went silent under WHICH provider, and why.
    ///
    /// Keyed by `(provider, cue)`, not cue alone: providers hold separate
    /// banks, so the same cue can fail for one provider and not another.
    pub missing_sources: std::collections::BTreeMap<(String, ambition_sfx::SfxId), SfxSourceMiss>,
}

impl SfxPlaybackState {
    pub fn clear_if_owner(&mut self, owner: AudioContextOwner) {
        if self
            .last_played
            .as_ref()
            .is_some_and(|record| record.owner == owner)
        {
            self.last_played = None;
        }
    }

    /// Record a miss, answering whether it is worth saying out loud: a
    /// `(provider, cue)` pair not seen before, or one whose diagnosis changed.
    pub fn note_missing_source(
        &mut self,
        provider_id: &str,
        id: ambition_sfx::SfxId,
        miss: SfxSourceMiss,
    ) -> bool {
        self.missing_source = self.missing_source.saturating_add(1);
        match self.missing_sources.get(&(provider_id.to_owned(), id)) {
            Some(known) if *known == miss => false,
            _ => {
                self.missing_sources
                    .insert((provider_id.to_owned(), id), miss);
                true
            }
        }
    }
}

/// Stable fingerprint of one procedural definition. Float fields are hashed by
/// their exact authored bit patterns; this is an identity for diagnostics and
/// tests, not a perceptual audio hash.
fn procedural_sfx_fingerprint(sample_rate: u32, spec: &SfxSpec) -> u64 {
    let waveform = match spec.waveform {
        WaveformSpec::Sine => 0_u32,
        WaveformSpec::Square => 1,
        WaveformSpec::Triangle => 2,
        WaveformSpec::Saw => 3,
    };
    let words = [
        sample_rate,
        waveform,
        spec.frequency.to_bits(),
        spec.frequency_end.to_bits(),
        spec.duration.to_bits(),
        spec.volume.to_bits(),
        spec.attack.to_bits(),
        spec.release.to_bits(),
        spec.noise.to_bits(),
    ];
    let mut bytes = Vec::with_capacity(words.len() * 4);
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    ambition_sfx::fnv1a_64(&bytes)
}

/// A short silent stereo buffer retained for compatibility fixtures.
pub fn silent_audio_source(sample_rate: u32) -> KiraAudioSource {
    let frames = vec![Frame::new(0.0, 0.0); (sample_rate / 100).max(2) as usize];
    KiraAudioSource {
        sound: StaticSoundData {
            sample_rate,
            frames: Arc::from(frames.into_boxed_slice()),
            settings: StaticSoundSettings::default(),
            slice: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{SfxRegistry, SfxSpec, SoundCueKey, WaveformSpec};

    fn registry(frequency: f32) -> SfxRegistry {
        SfxRegistry {
            sample_rate: 44_100,
            sfx: vec![SfxSpec {
                cue: Some(SoundCueKey::Dash),
                id: None,
                waveform: WaveformSpec::Square,
                frequency,
                frequency_end: frequency * 0.5,
                duration: 0.1,
                volume: 0.4,
                attack: 0.0,
                release: 0.02,
                noise: 0.0,
            }],
        }
    }

    fn cue(waveform: WaveformSpec, volume: f32, noise: f32) -> SfxSpec {
        SfxSpec {
            cue: None,
            id: Some("probe".to_owned()),
            waveform,
            frequency: 600.0,
            frequency_end: 900.0,
            duration: 0.2,
            volume,
            attack: 0.005,
            release: 0.05,
            noise,
        }
    }

    fn rendered_rms_db(spec: &SfxSpec) -> f32 {
        let source = audio_source_from_sfx_spec(spec, 44_100);
        let frames = &source.sound.frames;
        let mean_square = frames
            .iter()
            .map(|frame| (frame.left as f64) * (frame.left as f64))
            .sum::<f64>()
            / frames.len() as f64;
        20.0 * (mean_square.sqrt() as f32).log10()
    }

    fn rendered_peak(spec: &SfxSpec) -> f32 {
        audio_source_from_sfx_spec(spec, 44_100)
            .sound
            .frames
            .iter()
            .fold(0.0_f32, |max, frame| max.max(frame.left.abs()))
    }

    /// Sanic uses square and saw; the engine's own cues use sine and triangle.
    /// Equal `volume` must mean equal loudness across waveforms.
    #[test]
    fn equal_volume_is_equal_loudness_whatever_the_cue_is_made_of() {
        let reference = rendered_rms_db(&cue(WaveformSpec::Sine, 0.5, 0.0));
        for waveform in [
            WaveformSpec::Sine,
            WaveformSpec::Square,
            WaveformSpec::Triangle,
            WaveformSpec::Saw,
        ] {
            for noise in [0.0, 0.35, 0.8] {
                let measured = rendered_rms_db(&cue(waveform, 0.5, noise));
                assert!(
                    (measured - reference).abs() < 0.25,
                    "{waveform:?} at noise {noise} measured {measured:.2} dBFS \
                     against the sine's {reference:.2}"
                );
            }
        }
    }

    /// Control for the test above: `volume` is still a relative trim, and half
    /// the trim is 6 dB quieter.
    #[test]
    fn volume_remains_a_relative_trim_in_the_loudness_domain() {
        let loud = rendered_rms_db(&cue(WaveformSpec::Square, 0.5, 0.0));
        let quiet = rendered_rms_db(&cue(WaveformSpec::Square, 0.25, 0.0));
        assert!(
            (loud - quiet - 6.02).abs() < 0.05,
            "halving volume moved the level by {:.2} dB, not 6",
            loud - quiet
        );
        // The absolute anchor: an unenveloped cue at volume 1.0 is the
        // reference level. `scripts/audio_levels.py` ports this synthesizer
        // and constant, so the two must agree.
        let mut full = cue(WaveformSpec::Saw, 1.0, 0.4);
        full.attack = 0.0;
        full.release = 0.0;
        let measured = rendered_rms_db(&full);
        assert!(
            (measured - PROCEDURAL_CUE_REFERENCE_RMS_DBFS).abs() < 0.05,
            "volume 1.0 measured {measured:.3} dBFS, not the \
             {PROCEDURAL_CUE_REFERENCE_RMS_DBFS} dBFS reference"
        );
    }

    /// An RMS target can clip where a peak rule could not, so the ceiling is
    /// asserted, including on the peakiest body the synthesizer can produce.
    #[test]
    fn no_authored_volume_can_drive_a_cue_past_full_scale() {
        for waveform in [
            WaveformSpec::Sine,
            WaveformSpec::Square,
            WaveformSpec::Triangle,
            WaveformSpec::Saw,
        ] {
            for noise in [0.0, 0.5, 1.0] {
                let mut spec = cue(waveform, 1.0, noise);
                // One partial cycle: the peakiest, lowest-RMS body, where an
                // RMS target strains most.
                spec.frequency = 1.0;
                spec.frequency_end = 1.0;
                spec.duration = 0.01;
                let peak = rendered_peak(&spec);
                assert!(peak <= 1.0, "{waveform:?}/{noise} clipped at {peak}");
            }
        }
        assert!(rendered_peak(&cue(WaveformSpec::Square, 1.0, 0.0)) <= 1.0);
    }

    /// `volume` is independent of the envelope: a longer tail must not change
    /// the body's level, only lower the whole-clip average.
    #[test]
    fn a_longer_release_shapes_the_cue_without_relevelling_its_body() {
        let short = cue(WaveformSpec::Triangle, 0.4, 0.0);
        let mut long = short.clone();
        long.release = 0.18;

        let body_level = |spec: &SfxSpec| {
            // The first 20 ms: past the 5 ms attack, before either release.
            let source = audio_source_from_sfx_spec(spec, 44_100);
            let window = &source.sound.frames[441..882];
            let mean_square = window
                .iter()
                .map(|frame| (frame.left as f64) * (frame.left as f64))
                .sum::<f64>()
                / window.len() as f64;
            20.0 * (mean_square.sqrt() as f32).log10()
        };
        assert!(
            (body_level(&short) - body_level(&long)).abs() < 0.05,
            "the release changed the body's level: {:.2} vs {:.2} dBFS",
            body_level(&short),
            body_level(&long)
        );
        assert!(
            rendered_rms_db(&long) < rendered_rms_db(&short) - 1.0,
            "a longer release must still take energy out of the whole clip"
        );
    }

    #[test]
    fn provider_qualified_cache_keeps_same_id_definitions_distinct() {
        let mut cache = ProviderSfxHandleCache::default();
        let mut assets = Assets::<KiraAudioSource>::default();
        let a_registry = registry(220.0);
        let b_registry = registry(880.0);
        let id = SoundCueKey::Dash.sfx_id();

        let a = cache
            .handle_for("a", id, Some(&a_registry), None, None, &mut assets)
            .expect("provider a authors Dash");
        let b = cache
            .handle_for("b", id, Some(&b_registry), None, None, &mut assets)
            .expect("provider b authors Dash");

        assert_eq!(a.source.kind, SfxSourceKind::Procedural);
        assert_eq!(b.source.kind, SfxSourceKind::Procedural);
        assert_ne!(
            a.source.fingerprint, b.source.fingerprint,
            "the actual authored procedural definitions remain provider-relative"
        );
        assert_ne!(
            a.handle, b.handle,
            "one provider must not reuse another provider's rendered handle"
        );
    }

    #[test]
    fn a_late_bank_invalidates_a_cached_procedural_fallback() {
        let procedural = SfxSourceIdentity {
            kind: SfxSourceKind::Procedural,
            fingerprint: 11,
        };
        assert!(cached_sfx_source_is_current(procedural, None));
        assert!(
            !cached_sfx_source_is_current(procedural, Some(22)),
            "a late packed bank must upgrade the provider's cached fallback"
        );
        let packed = SfxSourceIdentity {
            kind: SfxSourceKind::Bank,
            fingerprint: 22,
        };
        assert!(cached_sfx_source_is_current(packed, Some(22)));
        assert!(!cached_sfx_source_is_current(packed, Some(23)));
    }

    #[test]
    fn missing_sources_are_not_cached_before_a_late_bank_arrives() {
        let mut cache = ProviderSfxHandleCache::default();
        let mut assets = Assets::<KiraAudioSource>::default();
        let id = SfxId::from_static("late.bank.cue");
        assert_eq!(
            cache
                .handle_for("late", id, None, None, None, &mut assets)
                .err(),
            Some(SfxSourceMiss::NoProviderBank),
            "the source is unavailable before its provider bank arrives, and says \
             that rather than claiming nobody authored the cue"
        );
        assert!(
            cache.handles.get(&("late".to_owned(), id)).is_none(),
            "a miss is not cached, so late provider content remains observable"
        );
    }
}
