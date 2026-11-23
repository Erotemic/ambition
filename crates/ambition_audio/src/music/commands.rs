use super::playback::{MusicPlayback, MusicPlaybackRequest, MusicProduct, MusicVoiceSource};
use super::*;

pub(super) type LayerGains = [f32; MAX_LAYERS];

/// Submit musical choices to the common occurrence owner.
#[derive(SystemParam)]
pub struct MusicPlaybackCommands<'w> {
    playback: ResMut<'w, MusicPlayback>,
}

impl MusicPlaybackCommands<'_> {
    pub(super) fn silence(&mut self) {
        self.playback.silence();
    }

    pub(super) fn set_section_gains(
        &mut self,
        provider: &str,
        cue: &str,
        section: &str,
        gains: &[f32],
    ) {
        self.playback
            .set_section_gains(provider, cue, section, gains);
    }

    pub(super) fn play_section(
        &mut self,
        provider: &str,
        cue: &MusicCueSpec,
        section: &MusicSectionSpec,
        assets: &LoadedMusicCueAssets,
        gains: LayerGains,
        fade_seconds: f32,
    ) {
        let voices = section
            .sources
            .iter()
            .filter_map(|source| {
                let slot = cue.layer(&source.layer_id)?.slot;
                Some(MusicVoiceSource {
                    source: assets.get(provider, &cue.id, &section.id, &source.layer_id)?,
                    gain: gains[slot],
                    slot,
                })
            })
            .collect::<Vec<_>>();
        if voices.len() != section.sources.len() || voices.is_empty() {
            return;
        }
        self.playback.request(MusicPlaybackRequest {
            product: MusicProduct::Section {
                provider: provider.into(),
                cue: cue.id.clone(),
                section: section.id.clone(),
            },
            voices,
            looped: section.looped,
            fade_seconds,
        });
    }

    pub(crate) fn play_track(
        &mut self,
        track: &str,
        source: Handle<KiraAudioSource>,
        one_shot: bool,
        fade_seconds: f32,
    ) {
        self.playback.request(MusicPlaybackRequest {
            product: MusicProduct::Track(track.into()),
            voices: vec![MusicVoiceSource {
                source,
                gain: 1.0,
                slot: 0,
            }],
            looped: !one_shot,
            fade_seconds,
        });
    }
}
