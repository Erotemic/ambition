//! Native tracks and source facts owned by bounded music groups.

use super::*;
use bevy_kira_audio::kira::{
    track::{TrackBuilder, TrackHandle},
    Decibels, Tween,
};

struct NativeGroup {
    track: TrackHandle,
    sources: HashMap<bevy::asset::AssetId<AudioInstance>, MusicSourceShape>,
}

/// Native output for the executor's live groups.
#[derive(Resource, Default)]
pub struct NativeMusicGroups {
    groups: HashMap<MusicPlaybackOccurrence, NativeGroup>,
}

impl NativeMusicGroups {
    pub(super) fn begin(
        &mut self,
        id: MusicPlaybackOccurrence,
        backend: Option<&mut bevy_kira_audio::AudioOutput>,
    ) -> bool {
        if self.groups.len() >= MAX_GROUPS {
            return false;
        }
        let Some(track) = backend
            .and_then(|backend| backend.manager())
            .and_then(|manager| {
                manager
                    .add_sub_track(
                        TrackBuilder::new()
                            .sound_capacity(MAX_LAYERS)
                            .sub_track_capacity(0),
                    )
                    .ok()
            })
        else {
            return false;
        };
        self.groups.insert(
            id,
            NativeGroup {
                track,
                sources: HashMap::new(),
            },
        );
        true
    }

    pub(super) fn submit(
        &mut self,
        id: MusicPlaybackOccurrence,
        handle: &Handle<AudioInstance>,
        source: &KiraAudioSource,
        looped: bool,
    ) -> Option<AudioInstance> {
        let group = self.groups.get_mut(&id)?;
        let shape = MusicSourceShape::decoded(source);
        let mut sound = source.sound.clone();
        sound.settings.volume = Decibels::SILENCE.into();
        sound.settings.playback_rate = 0.0.into();
        sound.settings.loop_region = looped.then(Default::default);
        let mut voice = group.track.play(sound).ok()?;
        voice.pause(Tween {
            duration: std::time::Duration::ZERO,
            ..Default::default()
        });
        group.sources.insert(handle.id(), shape);
        Some(AudioInstance::from_native(voice))
    }

    pub(super) fn shape(&self, handle: &Handle<AudioInstance>) -> Option<MusicSourceShape> {
        self.groups
            .values()
            .find_map(|group| group.sources.get(&handle.id()).copied())
    }

    pub(super) fn retire(
        &mut self,
        id: MusicPlaybackOccurrence,
        start: bevy_kira_audio::kira::StartTime,
        seconds: f32,
    ) {
        if let Some(group) = self.groups.get_mut(&id) {
            group.track.set_volume(
                Decibels::SILENCE,
                Tween {
                    start_time: start,
                    duration: std::time::Duration::from_secs_f32(seconds),
                    ..Default::default()
                },
            );
        }
    }

    pub(super) fn cancel_retirement(&mut self, id: MusicPlaybackOccurrence) {
        if let Some(group) = self.groups.get_mut(&id) {
            // Cancellation and retirement use the same latest-value command.
            group.track.set_volume(
                Decibels::IDENTITY,
                Tween {
                    duration: std::time::Duration::ZERO,
                    ..Default::default()
                },
            );
        }
    }

    pub(super) fn end(&mut self, id: MusicPlaybackOccurrence) {
        self.groups.remove(&id);
    }
}
