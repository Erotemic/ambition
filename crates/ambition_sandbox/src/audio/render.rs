//! SFX-bank byte → Kira asset adapter and lazy handle cache.
//!
//! After the fundsp procedural music + SFX synthesizer was retired the
//! runtime audio path is purely authored: music comes from pre-rendered
//! OGGs and SFX comes from the packed bank built by
//! `tools/ambition_sfx_pack`. This module owns the small wiring that
//! turns a [`sfx::SfxClip`] (raw bytes from a [`BankProvider`]) into a
//! [`KiraAudioSource`] Kira can play, and caches the resulting
//! handles per [`SfxId`] so each `SfxMessage::Play` only decodes once.

#[cfg(feature = "audio")]
use super::*;

#[cfg(feature = "audio")]
pub(super) fn audio_source_from_sfx_clip(clip: sfx::SfxClip) -> Result<KiraAudioSource, String> {
    let cursor = Cursor::new(clip.bytes.to_vec());
    let sound = StaticSoundData::from_cursor(cursor).map_err(|e| e.to_string())?;
    Ok(KiraAudioSource { sound })
}

#[cfg(feature = "audio")]
#[derive(Resource, Default)]
pub struct SfxBankHandleCache {
    handles: HashMap<SfxId, Option<Handle<KiraAudioSource>>>,
}

#[cfg(feature = "audio")]
impl SfxBankHandleCache {
    pub(super) fn handle_for(
        &mut self,
        id: SfxId,
        bank: Option<&crate::setup::SfxBankResource>,
        audio_sources: &mut Assets<KiraAudioSource>,
    ) -> Option<Handle<KiraAudioSource>> {
        if let Some(slot) = self.handles.get(&id) {
            return slot.clone();
        }
        let result = (|| {
            let bank = bank?;
            let clip = bank.0.provide_clip(id)?;
            match audio_source_from_sfx_clip(clip) {
                Ok(source) => Some(audio_sources.add(source)),
                Err(error) => {
                    warn!("sfx bank entry for id {id} failed to decode ({error})");
                    None
                }
            }
        })();
        if result.is_none() {
            warn!("sfx bank has no entry for id {id}");
        }
        self.handles.insert(id, result.clone());
        result
    }
}

/// A short silent stereo buffer used as the SFX fallback when no bank
/// provider is registered (e.g. WebStatic without `static_sfx_bank`).
/// Returning a real handle keeps the playback path uniform — Kira just
/// plays ~10ms of silence instead of warning per call.
#[cfg(feature = "audio")]
pub(super) fn silent_audio_source(sample_rate: u32) -> KiraAudioSource {
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
