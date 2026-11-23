//! Authored-audio playback library: typed SFX cue table, lazily-loaded
//! pre-rendered music tracks, SFX channels, and radio requests. Game-side adapters (which
//! requests apply, where the bank bytes come from) stay in the host.

use crate::spec::{MusicRegistry, SoundCueKey};
use ambition_sfx::{SfxId, SfxMessage};
use bevy::prelude::*;
use bevy_kira_audio::prelude::AudioInstance;
pub use bevy_kira_audio::prelude::AudioSource as KiraAudioSource;

#[derive(Resource)]
pub struct SfxChannel;

/// Maps the typed [`SfxMessage`] variants to the [`SoundCue`] table.
/// `SfxMessage` lives in `ambition_sfx` so mechanics can request sound without
/// naming this crate, and `SoundCue` is this crate's mapping, so this is an
/// extension trait.
pub trait SfxMessageCue {
    /// The typed cue this message maps to, or `None` for the open-ended
    /// `Play { id }` variant (handled directly via its `SfxId`).
    fn cue(self) -> Option<SoundCue>;
}

impl SfxMessageCue for SfxMessage {
    fn cue(self) -> Option<SoundCue> {
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
            SfxMessage::Land { .. } => SoundCue::Land,
            SfxMessage::Slash { .. } => SoundCue::Slash,
            SfxMessage::Hit { .. } => SoundCue::Hit,
            SfxMessage::Death { .. } => SoundCue::Death,
            SfxMessage::Reset { .. } => SoundCue::Reset,
            SfxMessage::Play { .. } => return None,
        })
    }
}

/// The [`SfxId`] a message resolves to for provider-authority checks: its typed
/// cue's id, or the `Play { id }` id. The gate uses the id the emitter
/// requested (before the consumer maps `Play` to a cue), so a provider is
/// judged on what it authored.
pub fn sfx_message_target_id(message: SfxMessage) -> SfxId {
    match message.cue() {
        Some(cue) => cue.sfx_id(),
        None => match message {
            SfxMessage::Play { id, .. } => id,
            // `cue()` returns `None` only for `Play`, so this is unreachable.
            _ => unreachable!("every non-Play SfxMessage variant maps to a cue"),
        },
    }
}

pub fn amplitude_to_decibels(linear: f32) -> f32 {
    let clamped = linear.clamp(0.0, 1.0);
    if clamped < 0.001 {
        return -60.0;
    }
    20.0 * clamped.log10()
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
    Land,
    Reset,
    Death,
    Respawn,
}

impl SoundCue {
    pub const ALL: [Self; 12] = [
        Self::Jump,
        Self::DoubleJump,
        Self::Dash,
        Self::Blink,
        Self::PrecisionBlink,
        Self::Slash,
        Self::Hit,
        Self::Pogo,
        Self::Land,
        Self::Reset,
        Self::Death,
        Self::Respawn,
    ];

    /// Reverse of [`Self::sfx_id`]: the procedural cue an [`SfxId`] names, if
    /// any. Lets `SfxMessage::Play { id }` (used by the data-driven moveset,
    /// whose events carry a string cue) reach a procedural cue instead of
    /// doing nothing on a bank miss.
    pub fn from_sfx_id(id: SfxId) -> Option<Self> {
        Self::ALL.into_iter().find(|cue| cue.sfx_id() == id)
    }

    pub fn sfx_id(self) -> SfxId {
        // Use the kira-free [`SoundCueKey::sfx_id`] table, so the authority
        // projection and the playback lookup stay in sync.
        SoundCueKey::from(self).sfx_id()
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
            SoundCue::Land => Self::Land,
            SoundCue::Reset => Self::Reset,
            SoundCue::Death => Self::Death,
            SoundCue::Respawn => Self::Respawn,
        }
    }
}

/// Backing storage for a music track's audio source.
#[derive(Clone)]
struct TrackSource {
    asset_path: String,
    handle: Handle<KiraAudioSource>,
    requested: bool,
}

#[derive(Clone)]
pub struct MusicTrackRuntime {
    pub generation: u64,
    pub id: String,
    pub display_name: String,
    /// Plays once and stops instead of looping; see
    /// [`crate::spec::MusicTrack::one_shot`].
    pub one_shot: bool,
    asset_path: String,
}

impl MusicTrackRuntime {
    /// Logical asset path for diagnostics. The live handle comes from
    /// `resolve_track_handle`.
    pub fn asset_path(&self) -> &str {
        &self.asset_path
    }
}

#[derive(Resource)]
pub struct AudioLibrary {
    music_tracks: Vec<MusicTrackRuntime>,
    sources: std::collections::HashMap<String, TrackSource>,
}

/// Player-selected simple music track: the radio station. While a station is
/// set it is the music, above the room's and every situation's (a fight, a
/// conversation, an adaptive cue); with none, the room and the situation choose.
/// The music director is the only player: selecting a station plays nothing
/// here.
#[derive(Resource, Clone, Debug, Default)]
pub struct RadioStationState {
    selected_track: Option<String>,
}

impl RadioStationState {
    pub fn selected_track(&self) -> Option<&str> {
        self.selected_track.as_deref()
    }

    pub fn set_selected_track(&mut self, track_id: impl Into<String>) {
        self.selected_track = Some(track_id.into());
    }

    /// Turn the radio off: the room's and the situation's music return.
    pub fn clear(&mut self) {
        self.selected_track = None;
    }
}

impl AudioLibrary {
    /// Build track metadata and the playback read model from one registry.
    /// A preparation owner can retain a preload handle until playback starts.
    pub fn new_with_playback_state(
        music_registry: &MusicRegistry,
        resolve_track_path: Option<&dyn Fn(&str) -> Option<String>>,
    ) -> (Self, MusicPlaybackState) {
        let library = Self::new(music_registry, resolve_track_path);
        let playback = MusicPlaybackState::from_music_registry(music_registry, &library);
        (library, playback)
    }

    /// Build the music track table from declared paths.
    pub fn new(
        music_registry: &MusicRegistry,
        resolve_track_path: Option<&dyn Fn(&str) -> Option<String>>,
    ) -> Self {
        if let Err(error) = music_registry.validate() {
            warn!("invalid music registry: {error}");
        }
        let music_tracks = music_registry
            .tracks
            .iter()
            .map(|track| MusicTrackRuntime {
                generation: 1,
                id: track.id.clone(),
                display_name: track.display_name.clone(),
                one_shot: track.one_shot,
                asset_path: resolve_track_path
                    .and_then(|resolve| resolve(&track.id))
                    .unwrap_or_else(|| track.resolved_asset_path()),
            })
            .collect();
        Self { music_tracks, sources: Default::default() }
    }

    /// Replace one provider's tracks, for a reload of that provider's pack:
    /// `previous` is the music the provider stated before, `next` what it
    /// states now. A track of another provider is kept as it is, and so is a
    /// track the provider did not change (with the path the host resolved
    /// for it). A changed or new track takes its own path
    /// ([`crate::spec::MusicTrack::resolved_asset_path`]) and loads on first
    /// use; a dropped track is removed.
    pub fn revise_provider_tracks(
        &mut self,
        previous: Option<&MusicRegistry>,
        next: Option<&MusicRegistry>,
    ) {
        let previous: &[crate::spec::MusicTrack] = previous.map_or(&[], |music| &music.tracks);
        let next: &[crate::spec::MusicTrack] = next.map_or(&[], |music| &music.tracks);
        self.music_tracks.retain(|track| {
            !previous.iter().any(|old| old.id == track.id)
                || next.iter().any(|new| new.id == track.id)
        });
        for track in next {
            if previous.contains(track) && self.track(&track.id).is_some() {
                continue;
            }
            let asset_path = track.resolved_asset_path();
            let known = self
                .music_tracks
                .iter_mut()
                .find(|known| known.id == track.id);
            let generation = known.as_ref().map_or(1, |track| track.generation + 1);
            let runtime = MusicTrackRuntime {
                generation,
                id: track.id.clone(),
                display_name: track.display_name.clone(),
                one_shot: track.one_shot,
                asset_path,
            };
            match known {
                Some(known) => *known = runtime,
                None => self.music_tracks.push(runtime),
            }
        }
        self.sources.retain(|id, source| self.music_tracks.iter()
            .any(|track| track.id == *id && track.asset_path == source.asset_path));
    }

    pub fn track(&self, id: &str) -> Option<&MusicTrackRuntime> {
        self.music_tracks.iter().find(|track| track.id == id)
    }

    /// Return an already-resolved playable handle without starting a load.
    ///
    /// Explicit preparation uses `preload_track` and retains its returned handle.
    pub fn resolved_track_handle(&self, id: &str) -> Option<Handle<KiraAudioSource>> {
        self.sources.get(id).map(|source| source.handle.clone())
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

    pub fn default_track_id<'a>(&self, configured: &'a str) -> Option<&'a str> {
        self.track(configured).map(|_| configured)
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

    /// Resolve a music track's playable handle, loading from disk on the
    /// first request. Returns `None` only for missing IDs.
    pub fn resolve_track_handle(
        &mut self,
        track_id: &str,
        asset_server: &AssetServer,
    ) -> Option<Handle<KiraAudioSource>> {
        Some(self.resolve_track(track_id, asset_server)?.0)
    }

    /// The handle, plus whether this cue loops. Stings must not repeat, and
    /// only the spec knows which cues are stings.
    pub fn resolve_track(
        &mut self,
        track_id: &str,
        asset_server: &AssetServer,
    ) -> Option<(Handle<KiraAudioSource>, bool)> {
        let track = self.music_tracks.iter().find(|track| track.id == track_id)?;
        if self.sources.get(track_id).is_none_or(|source| source.asset_path != track.asset_path) {
            self.sources.insert(track_id.to_owned(), TrackSource {
                asset_path: track.asset_path.clone(),
                handle: asset_server.load(track.asset_path.clone()),
                requested: true,
            });
        }
        let source = self.sources.get_mut(track_id).expect("resolved track");
        source.requested = true;
        Some((source.handle.clone(), track.one_shot))
    }

    /// Retain this handle until playback or another preparation owner holds the source.
    pub fn preload_track(&self, track_id: &str, asset_server: &AssetServer) -> Option<Handle<KiraAudioSource>> {
        Some(asset_server.load(self.track(track_id)?.asset_path().to_owned()))
    }

    /// Keep the last director update's source demand.
    pub(crate) fn retire_unused(&mut self) {
        self.sources.retain(|_, source| std::mem::replace(&mut source.requested, false));
    }
}

/// What the base music channel is playing, and which play it is.
///
/// The generation tells "still playing" from "started again": a transition
/// that stops and immediately restarts the same file leaves the same track
/// name.
///
/// This is state, not an event log. If audio ever needs a timeline, it goes
/// in `ambition_causal` as a domain.
///
/// Fields are private: starting a track and recording the start are one
/// operation ([`Self::begin_track`]).
#[derive(Resource, Clone, Debug)]
pub struct MusicPlaybackState {
    active_track: String,
    play_generation: u64,
    instance: Option<Handle<AudioInstance>>,
}

impl MusicPlaybackState {
    pub fn from_music_registry(music: &MusicRegistry, library: &AudioLibrary) -> Self {
        let active_track = library
            .default_track_id(&music.default_track)
            .unwrap_or_default()
            .to_string();
        Self {
            active_track,
            play_generation: 0,
            instance: None,
        }
    }

    /// The selected base track. Use the instance state to confirm playback.
    pub fn active_track(&self) -> &str {
        &self.active_track
    }

    pub(crate) fn set_instance(&mut self, instance: Option<Handle<AudioInstance>>) {
        self.instance = instance;
    }

    pub fn instance(&self) -> Option<&Handle<AudioInstance>> {
        self.instance.as_ref()
    }

    /// How many times the base channel was started. Only equality between two
    /// readings is meaningful: the same generation means the same
    /// uninterrupted play.
    pub fn play_generation(&self) -> u64 {
        self.play_generation
    }

    /// A track started on the base channel. Bumps the generation even for the
    /// same track id, because a restart is the event this records.
    pub fn begin_track(&mut self, track_id: impl Into<String>) {
        self.active_track = track_id.into();
        self.instance = None;
        self.play_generation = self.play_generation.wrapping_add(1);
    }

    /// The base channel stopped. A later play of the same track is a new play,
    /// so the generation moves here too.
    pub fn silence(&mut self) {
        self.active_track.clear();
        self.instance = None;
        self.play_generation = self.play_generation.wrapping_add(1);
    }

    pub fn active_display_name<'a>(&self, library: &'a AudioLibrary) -> &'a str {
        library.display_name(&self.active_track)
    }
}

/// Select station `next_track`, if the library has it. The music director plays
/// it from the next frame (`compute_music_intent` puts a station first), so the
/// radio and an authored cue never sound at once.
pub fn select_radio_station(
    library: &AudioLibrary,
    radio: &mut RadioStationState,
    next_track: &str,
) -> bool {
    if library.track(next_track).is_none() {
        warn!("cannot set radio to missing music track '{next_track}'");
        return false;
    }
    radio.set_selected_track(next_track);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::MusicTrack;

    fn track(id: &str, asset_path: Option<&str>) -> MusicTrack {
        MusicTrack {
            id: id.into(),
            display_name: id.into(),
            asset_path: asset_path.map(Into::into),
            one_shot: false,
        }
    }

    fn music(tracks: Vec<MusicTrack>) -> MusicRegistry {
        MusicRegistry {
            default_track: tracks[0].id.clone(),
            tracks,
        }
    }

    fn library(tracks: &[(&str, &str)]) -> AudioLibrary {
        AudioLibrary {
            music_tracks: tracks
                .iter()
                .map(|(id, path)| MusicTrackRuntime {
                    generation: 1,
                    id: id.to_string(),
                    display_name: id.to_string(),
                    one_shot: false,
                    asset_path: path.to_string(),
                })
                .collect(),
            sources: Default::default(),
        }
    }

    fn paths(library: &AudioLibrary) -> Vec<(String, String)> {
        let mut paths: Vec<_> = library
            .music_tracks
            .iter()
            .map(|track| (track.id.clone(), track.asset_path().to_string()))
            .collect();
        paths.sort();
        paths
    }

    /// ⭐ A reload of one provider's music changes its tracks and no other
    /// provider's, and keeps the host's path for a track it did not change.
    #[test]
    fn a_provider_reload_revises_its_own_tracks_only() {
        let before = music(vec![
            track("kept", None),
            track("moved", None),
            track("dropped", None),
        ]);
        let after = music(vec![
            track("kept", None),
            track("moved", Some("audio/other.ogg")),
            track("added", None),
        ]);
        let mut library = library(&[
            ("host_theme", "host/theme.ogg"),
            ("kept", "catalog/kept.ogg"),
            ("moved", "catalog/moved.ogg"),
            ("dropped", "catalog/dropped.ogg"),
        ]);

        library.revise_provider_tracks(Some(&before), Some(&after));

        assert_eq!(
            paths(&library),
            [
                (
                    "added".to_string(),
                    "audio/music/generated/added/full.ogg".to_string()
                ),
                ("host_theme".to_string(), "host/theme.ogg".to_string()),
                ("kept".to_string(), "catalog/kept.ogg".to_string()),
                ("moved".to_string(), "audio/other.ogg".to_string()),
            ]
        );
    }
}
