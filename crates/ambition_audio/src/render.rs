//! Provider-relative SFX source resolution and Kira adapters.
//!
//! The combined App may cache many providers' authored sources, but a playback
//! request resolves through the active provider first. Procedural definitions
//! synthesize from that provider's [`SfxRegistry`](crate::spec::SfxRegistry);
//! packed entries decode from that provider's bank. Authorization can therefore
//! never accidentally select an Ambition handle for a Sanic cue with the same
//! logical id.

use ambition_sfx::{self as sfx, AudioContextOwner, SfxId, SfxProvider};
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

/// Deterministically synthesize one provider-authored procedural cue.
pub fn audio_source_from_sfx_spec(spec: &SfxSpec, sample_rate: u32) -> KiraAudioSource {
    let sample_rate = sample_rate.max(8_000);
    let frame_count = (spec.duration.max(0.01) * sample_rate as f32).ceil() as usize;
    let attack = spec.attack.max(0.0);
    let release = spec.release.max(0.0);
    let duration = spec.duration.max(0.01);
    let mut phase = 0.0_f32;
    let mut noise_state = 0x6d2b_79f5_u32;
    let mut frames = Vec::with_capacity(frame_count.max(2));
    for index in 0..frame_count.max(2) {
        let t = index as f32 / sample_rate as f32;
        let progress = (t / duration).clamp(0.0, 1.0);
        let frequency = spec.frequency + (spec.frequency_end - spec.frequency) * progress;
        phase = (phase + TAU * frequency.max(1.0) / sample_rate as f32) % TAU;
        let tone = match spec.waveform {
            WaveformSpec::Sine => phase.sin(),
            WaveformSpec::Square => if phase.sin() >= 0.0 { 1.0 } else { -1.0 },
            WaveformSpec::Triangle => 2.0 * (2.0 * (phase / TAU - (phase / TAU + 0.5).floor())).abs() - 1.0,
            WaveformSpec::Saw => 2.0 * (phase / TAU) - 1.0,
        };
        noise_state = noise_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let noise = ((noise_state >> 8) as f32 / 0x00ff_ffff as f32) * 2.0 - 1.0;
        let attack_gain = if attack > 0.0 {
            (t / attack).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let release_start = (duration - release).max(0.0);
        let release_gain = if release > 0.0 && t > release_start {
            ((duration - t) / release).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let noise_mix = spec.noise.clamp(0.0, 1.0);
        let sample = ((1.0 - noise_mix) * tone + noise_mix * noise)
            * spec.volume.clamp(0.0, 1.0)
            * attack_gain
            * release_gain;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SfxSourceKind {
    Procedural,
    Bank,
}

/// Stable identity of the authored source that produced one playback handle.
///
/// This is deliberately independent of Bevy's opaque `Handle` allocation so
/// lifecycle tests can prove that Sanic's procedural Dash did not accidentally
/// resolve to Ambition's resident sample with the same logical id.
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

/// Lazy provider-qualified handle cache. Missing sources are not cached so a
/// bank that arrives after activation becomes usable immediately.
#[derive(Resource, Default)]
pub struct ProviderSfxHandleCache {
    handles: HashMap<(String, SfxId), ResolvedSfxHandle>,
}

impl ProviderSfxHandleCache {
    pub fn handle_for(
        &mut self,
        provider_id: &str,
        id: SfxId,
        procedural: Option<&SfxRegistry>,
        bank: Option<&dyn SfxProvider>,
        bank_fingerprint: Option<u64>,
        audio_sources: &mut Assets<KiraAudioSource>,
    ) -> Option<ResolvedSfxHandle> {
        let key = (provider_id.to_owned(), id);
        if let Some(handle) = self.handles.get(&key) {
            if cached_sfx_source_is_current(handle.source, bank_fingerprint) {
                return Some(handle.clone());
            }
            // A procedural fallback may have been rendered before this
            // provider's packed bank finished loading. Do not let that fallback
            // become sticky: the first request after bank publication upgrades
            // the cache to the provider's higher-fidelity authored clip.
            self.handles.remove(&key);
        }
        // Packed provider content is the highest-fidelity authored source.
        // Procedural specs are provider-local fallbacks and the complete source
        // for providers such as Sanic that intentionally ship no packed bank.
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
                    warn!("provider '{provider_id}' SFX id {id} failed to decode ({error})");
                    None
                }
            });
        let resolved = from_bank.or_else(|| {
            procedural
                .and_then(|registry| registry.spec_for_id(id).map(|spec| (registry, spec)))
                .map(|(registry, spec)| ResolvedSfxHandle {
                    handle: audio_sources.add(audio_source_from_sfx_spec(spec, registry.sample_rate)),
                    source: SfxSourceIdentity {
                        kind: SfxSourceKind::Procedural,
                        fingerprint: procedural_sfx_fingerprint(registry.sample_rate, spec),
                    },
                })
        });
        if let Some(resolved) = resolved.as_ref() {
            self.handles.insert(key, resolved.clone());
        }
        resolved
    }

    pub fn clear_provider(&mut self, provider_id: &str) {
        self.handles.retain(|(provider, _), _| provider != provider_id);
    }
}


fn cached_sfx_source_is_current(
    cached: SfxSourceIdentity,
    bank_fingerprint: Option<u64>,
) -> bool {
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
    pub provider_id: String,
    pub id: SfxId,
    pub source: SfxSourceIdentity,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct SfxPlaybackState {
    pub last_played: Option<SfxPlaybackRecord>,
    /// Number of requests accepted by the real playback decision path.
    ///
    /// Unlike `last_played`, this monotonic counter is not cleared on an
    /// audio-context transition, so tests and diagnostics can prove that a
    /// rejected delayed request did not reach playback even when the fresh
    /// session legitimately emitted another cue during activation.
    pub accepted_playbacks: u64,
    pub rejected_wrong_owner: u64,
    pub rejected_unauthorized: u64,
    pub missing_source: u64,
}

impl SfxPlaybackState {
    pub fn clear_if_owner(&mut self, owner: AudioContextOwner) {
        if self.last_played.as_ref().is_some_and(|record| record.owner == owner) {
            self.last_played = None;
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
        assert!(
            cache
                .handle_for("late", id, None, None, None, &mut assets)
                .is_none(),
            "the source is unavailable before its provider bank arrives"
        );
        assert!(
            cache.handles.get(&("late".to_owned(), id)).is_none(),
            "a miss is not cached, so late provider content remains observable"
        );
    }
}
