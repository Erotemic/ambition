#[cfg(feature = "audio")]
use super::render::render_lofi_theme;
#[cfg(feature = "audio")]
use super::render::{
    add_rendered_audio, audio_source_from_sfx_clip, fallback_sfx, find_sfx, render_sfx,
};
use super::*;

pub const ORIGINAL_TRACK_ID: &str = "original_lofi_loop";

#[cfg(feature = "audio")]
#[derive(Resource)]
pub struct MusicChannel;

#[cfg(feature = "audio")]
#[derive(Resource)]
pub struct SfxChannel;

#[derive(Message, Clone, Copy, Debug)]
pub enum SfxMessage {
    Jump { pos: ae::Vec2 },
    DoubleJump { pos: ae::Vec2 },
    Dash { pos: ae::Vec2 },
    Blink { pos: ae::Vec2, precision: bool },
    Pogo { pos: ae::Vec2 },
    Slash { pos: ae::Vec2 },
    Hit { pos: ae::Vec2 },
    Death { pos: ae::Vec2 },
    Reset { pos: ae::Vec2 },
    Play { id: SfxId, pos: ae::Vec2 },
}

impl SfxMessage {
    pub fn cue(self) -> Option<SoundCue> {
        Some(match self {
            SfxMessage::Jump { .. } => SoundCue::Jump,
            SfxMessage::DoubleJump { .. } => SoundCue::DoubleJump,
            SfxMessage::Dash { .. } => SoundCue::Dash,
            SfxMessage::Blink {
                precision: false, ..
            } => SoundCue::Blink,
            SfxMessage::Blink {
                precision: true, ..
            } => SoundCue::PrecisionBlink,
            SfxMessage::Pogo { .. } => SoundCue::Pogo,
            SfxMessage::Slash { .. } => SoundCue::Slash,
            SfxMessage::Hit { .. } => SoundCue::Hit,
            SfxMessage::Death { .. } => SoundCue::Death,
            SfxMessage::Reset { .. } => SoundCue::Reset,
            SfxMessage::Play { .. } => return None,
        })
    }
}

#[cfg(feature = "audio")]
pub fn audio_play_sfx_messages(
    mut messages: MessageReader<SfxMessage>,
    library: Res<AudioLibrary>,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
    bank: Option<Res<crate::setup::SfxBankResource>>,
    mut cache: ResMut<SfxBankHandleCache>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
) {
    for message in messages.read() {
        if let Some(cue) = message.cue() {
            sfx_channel.play(library.sfx_handle(cue));
            continue;
        }
        let SfxMessage::Play { id, .. } = *message else {
            continue;
        };
        let Some(handle) = cache.handle_for(id, bank.as_deref(), audio_sources.as_mut()) else {
            continue;
        };
        sfx_channel.play(handle);
    }
}

#[cfg(feature = "audio")]
pub fn amplitude_to_decibels(linear: f32) -> f32 {
    let clamped = linear.clamp(0.0, 1.0);
    if clamped < 0.001 {
        return -60.0;
    }
    20.0 * clamped.log10()
}

#[cfg(feature = "audio")]
pub fn apply_encounter_music(
    library: Res<AudioLibrary>,
    mut music_state: ResMut<MusicPlaybackState>,
    music_channel: Res<AudioChannel<MusicChannel>>,
    mut request: ResMut<crate::encounter::EncounterMusicRequest>,
    room_music: Res<crate::rooms::RoomMusicRequest>,
    sandbox_data: Res<crate::data::SandboxDataSpec>,
) {
    let resolved_default = room_music
        .desired_track
        .as_ref()
        .filter(|id| library.track(id).is_some())
        .cloned()
        .unwrap_or_else(|| sandbox_data.audio.default_music_track.clone());
    let target = match &request.desired_track {
        Some(track) => track.clone(),
        None => resolved_default,
    };
    let already_applied =
        request.last_applied.as_ref() == Some(&target) && music_state.active_track == target;
    if !already_applied && library.track(&target).is_some() {
        switch_to_music_track(&library, &mut music_state, &music_channel, &target);
    }
    request.last_applied = Some(target);
}

#[cfg(feature = "audio")]
pub fn apply_audio_settings(
    settings: Res<crate::settings::UserSettings>,
    music_channel: Res<AudioChannel<MusicChannel>>,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
    mut last: Local<Option<crate::settings::AudioSettings>>,
) {
    let current = settings.audio;
    if last.as_ref() == Some(&current) {
        return;
    }
    let music_db = amplitude_to_decibels(current.effective_music());
    let sfx_db = amplitude_to_decibels(current.effective_sfx());
    music_channel.set_volume(music_db);
    sfx_channel.set_volume(sfx_db);
    *last = Some(current);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

impl SoundCue {
    pub(super) const ALL: [Self; 11] = [
        Self::Jump,
        Self::DoubleJump,
        Self::Dash,
        Self::Blink,
        Self::PrecisionBlink,
        Self::Slash,
        Self::Hit,
        Self::Pogo,
        Self::Reset,
        Self::Death,
        Self::Respawn,
    ];

    #[cfg(feature = "audio")]
    pub fn sfx_id(self) -> SfxId {
        match self {
            Self::Jump => sfx::ids::PLAYER_JUMP,
            Self::DoubleJump => sfx::ids::PLAYER_DOUBLE_JUMP,
            Self::Dash => sfx::ids::PLAYER_DASH,
            Self::Blink => sfx::ids::PLAYER_BLINK,
            Self::PrecisionBlink => sfx::ids::PLAYER_PRECISION_BLINK,
            Self::Slash => sfx::ids::PLAYER_SLASH,
            Self::Hit => sfx::ids::PLAYER_HIT,
            Self::Pogo => sfx::ids::PLAYER_POGO,
            Self::Reset => sfx::ids::PLAYER_RESET,
            Self::Death => sfx::ids::PLAYER_DEATH,
            Self::Respawn => sfx::ids::PLAYER_RESPAWN,
        }
    }
}

impl From<SoundCue> for SoundCueKey {
    fn from(value: SoundCue) -> Self {
        match value {
            SoundCue::Jump => Self::Jump,
            SoundCue::DoubleJump => Self::DoubleJump,
            SoundCue::Dash => Self::Dash,
            SoundCue::Blink => Self::Blink,
            SoundCue::PrecisionBlink => Self::PrecisionBlink,
            SoundCue::Slash => Self::Slash,
            SoundCue::Hit => Self::Hit,
            SoundCue::Pogo => Self::Pogo,
            SoundCue::Reset => Self::Reset,
            SoundCue::Death => Self::Death,
            SoundCue::Respawn => Self::Respawn,
        }
    }
}

#[derive(Clone)]
#[cfg(feature = "audio")]
pub struct MusicTrackRuntime {
    pub id: String,
    pub display_name: String,
    pub handle: Handle<KiraAudioSource>,
    pub duration_seconds: f32,
}

#[derive(Resource)]
#[cfg(feature = "audio")]
pub struct AudioLibrary {
    sfx: HashMap<SoundCue, Handle<KiraAudioSource>>,
    fallback_sfx: Handle<KiraAudioSource>,
    music_tracks: Vec<MusicTrackRuntime>,
}

/// Player-selected simple music track. The music director treats this as the
/// sandbox radio station: rooms can still provide a default when no station is
/// set, and adaptive encounter cues can temporarily take over, but the chosen
/// radio track resumes afterward.
#[derive(Resource, Clone, Debug, Default)]
#[cfg(feature = "audio")]
pub struct RadioStationState {
    selected_track: Option<String>,
}

#[cfg(feature = "audio")]
impl RadioStationState {
    pub fn selected_track(&self) -> Option<&str> {
        self.selected_track.as_deref()
    }

    pub fn set_selected_track(&mut self, track_id: impl Into<String>) {
        self.selected_track = Some(track_id.into());
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.selected_track = None;
    }
}

#[cfg(feature = "audio")]
impl AudioLibrary {
    pub fn new(
        audio_sources: &mut Assets<KiraAudioSource>,
        spec: &AudioSpec,
        asset_server: Option<&AssetServer>,
        sfx_provider: Option<&dyn SfxProvider>,
    ) -> Self {
        if let Err(error) = spec.validate() {
            warn!("invalid audio spec: {error}");
        }
        let sample_rate = spec.sample_rate.max(8_000);
        let mut sfx_handles = HashMap::default();
        for cue in SoundCue::ALL {
            let from_bank = sfx_provider
                .and_then(|provider| provider.provide_clip(cue.sfx_id()))
                .and_then(|clip| match audio_source_from_sfx_clip(clip) {
                    Ok(source) => Some(audio_sources.add(source)),
                    Err(error) => {
                        warn!(
                            "bank entry for {cue:?} failed to decode ({error}); falling back to fundsp"
                        );
                        None
                    }
                });
            let handle = match from_bank {
                Some(handle) => handle,
                None => {
                    let sfx_spec = find_sfx(spec, cue);
                    add_rendered_audio(audio_sources, render_sfx(sfx_spec, sample_rate))
                }
            };
            sfx_handles.insert(cue, handle);
        }
        let sfx = sfx_handles;
        let fallback_sfx = sfx.get(&SoundCue::Jump).cloned().unwrap_or_else(|| {
            add_rendered_audio(
                audio_sources,
                render_sfx(fallback_sfx(SoundCueKey::Jump), sample_rate),
            )
        });

        let music_tracks = spec
            .music_tracks
            .iter()
            .map(|track| {
                let handle = match (&track.asset_path, asset_server) {
                    (Some(path), Some(server)) => server.load(path),
                    _ => add_rendered_audio(
                        audio_sources,
                        render_lofi_theme(&track.arrangement, sample_rate),
                    ),
                };
                MusicTrackRuntime {
                    id: track.id.clone(),
                    display_name: track.display_name.clone(),
                    handle,
                    duration_seconds: track.arrangement.duration_seconds(),
                }
            })
            .collect();

        Self {
            sfx,
            fallback_sfx,
            music_tracks,
        }
    }

    pub fn sfx_handle(&self, cue: SoundCue) -> Handle<KiraAudioSource> {
        self.sfx
            .get(&cue)
            .cloned()
            .unwrap_or_else(|| self.fallback_sfx.clone())
    }

    pub fn track(&self, id: &str) -> Option<&MusicTrackRuntime> {
        self.music_tracks.iter().find(|track| track.id == id)
    }

    pub fn track_count(&self) -> usize {
        self.music_tracks.len()
    }

    pub fn track_at(&self, index: usize) -> Option<&MusicTrackRuntime> {
        self.music_tracks.get(index)
    }

    pub fn track_index(&self, id: &str) -> Option<usize> {
        self.music_tracks.iter().position(|track| track.id == id)
    }

    pub fn display_name_at(&self, index: usize) -> Option<&str> {
        self.track_at(index)
            .map(|track| track.display_name.as_str())
    }

    pub fn radio_label(&self, index: usize, active: &str) -> Option<String> {
        let track = self.track_at(index)?;
        let marker = if track.id == active { "▶" } else { " " };
        Some(format!(
            "{marker} {:02}/{:02} {}",
            index + 1,
            self.track_count().max(1),
            track.display_name
        ))
    }

    pub fn default_track_id<'a>(&'a self, configured: &'a str) -> Option<&'a str> {
        if self.track(configured).is_some() {
            Some(configured)
        } else if self.track(ORIGINAL_TRACK_ID).is_some() {
            warn!(
                "default music track '{configured}' is missing; falling back to '{ORIGINAL_TRACK_ID}'"
            );
            Some(ORIGINAL_TRACK_ID)
        } else {
            let fallback = self.music_tracks.first().map(|track| track.id.as_str());
            if let Some(fallback) = fallback {
                warn!(
                    "default music track '{configured}' is missing; falling back to '{fallback}'"
                );
            }
            fallback
        }
    }

    pub fn display_name(&self, id: &str) -> &str {
        self.track(id)
            .map(|track| track.display_name.as_str())
            .unwrap_or("Unknown Track")
    }

    pub fn next_track_id(&self, active: &str) -> Option<&str> {
        self.track_offset(active, 1)
    }

    pub fn previous_track_id(&self, active: &str) -> Option<&str> {
        self.track_offset(active, -1)
    }

    fn track_offset(&self, active: &str, offset: isize) -> Option<&str> {
        if self.music_tracks.is_empty() {
            return None;
        }
        let index = self
            .music_tracks
            .iter()
            .position(|track| track.id == active)
            .unwrap_or(0);
        let len = self.music_tracks.len() as isize;
        let next = (index as isize + offset).rem_euclid(len) as usize;
        Some(self.music_tracks[next].id.as_str())
    }
}

#[derive(Resource, Clone, Debug)]
#[cfg(feature = "audio")]
pub struct MusicPlaybackState {
    pub active_track: String,
}

#[cfg(feature = "audio")]
impl MusicPlaybackState {
    pub fn from_audio_spec(spec: &AudioSpec, library: &AudioLibrary) -> Self {
        let active_track = library
            .default_track_id(&spec.default_music_track)
            .unwrap_or_default()
            .to_string();
        Self { active_track }
    }

    pub fn active_display_name<'a>(&self, library: &'a AudioLibrary) -> &'a str {
        library.display_name(&self.active_track)
    }
}

#[cfg(feature = "audio")]
pub fn start_default_music(
    library: Res<AudioLibrary>,
    state: Res<MusicPlaybackState>,
    music_channel: Res<AudioChannel<MusicChannel>>,
) {
    play_music_track(&library, &state.active_track, &music_channel);
}

#[cfg(feature = "audio")]
pub fn switch_to_music_track(
    library: &AudioLibrary,
    state: &mut MusicPlaybackState,
    music_channel: &AudioChannel<MusicChannel>,
    next_track: &str,
) {
    if library.track(next_track).is_none() {
        warn!("cannot switch to missing music track '{next_track}'");
        return;
    }
    state.active_track = next_track.to_string();
    music_channel.stop().fade_out(AudioTween::new(
        Duration::from_millis(180),
        AudioEasing::OutPowi(2),
    ));
    play_music_track(library, next_track, music_channel);
}

#[cfg(feature = "audio")]
pub fn set_radio_track(
    library: &AudioLibrary,
    radio: &mut RadioStationState,
    state: &mut MusicPlaybackState,
    music_channel: &AudioChannel<MusicChannel>,
    next_track: &str,
) {
    if library.track(next_track).is_none() {
        warn!("cannot set radio to missing music track '{next_track}'");
        return;
    }
    radio.set_selected_track(next_track);
    switch_to_music_track(library, state, music_channel, next_track);
}

#[cfg(feature = "audio")]
fn play_music_track(
    library: &AudioLibrary,
    track_id: &str,
    music_channel: &AudioChannel<MusicChannel>,
) {
    let Some(track) = library.track(track_id) else {
        warn!("cannot play missing music track '{track_id}'");
        return;
    };
    music_channel
        .play(track.handle.clone())
        .looped()
        .fade_in(AudioTween::new(
            Duration::from_millis(220),
            AudioEasing::InPowi(2),
        ));
}
