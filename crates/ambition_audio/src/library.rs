//! Authored-audio playback library: typed SFX cue table, lazily-loaded
//! pre-rendered music tracks, the music/SFX Kira channels, and the
//! track-switch/radio/default-start helpers. Game-side adapters (which
//! requests apply, where the bank bytes come from) stay in the host.

use crate::spec::{MusicRegistry, SoundCueKey};
use crate::web_unlock::AudioUnlockState;
use ambition_sfx::{SfxId, SfxMessage};
use bevy::prelude::*;
use bevy_kira_audio::prelude::{
    AudioChannel, AudioControl, AudioEasing, AudioSource as KiraAudioSource, AudioTween,
};
use std::time::Duration;

pub const ORIGINAL_TRACK_ID: &str = "original_lofi_loop";

#[derive(Resource)]
pub struct MusicChannel;

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
    handle: Option<Handle<KiraAudioSource>>,
}

#[derive(Clone)]
pub struct MusicTrackRuntime {
    pub id: String,
    pub display_name: String,
    /// Plays once and stops instead of looping; see
    /// [`crate::spec::MusicTrack::one_shot`].
    pub one_shot: bool,
    source: TrackSource,
}

impl MusicTrackRuntime {
    /// Logical asset path for diagnostics. The live handle comes from
    /// `resolve_track_handle`.
    pub fn asset_path(&self) -> &str {
        &self.source.asset_path
    }
}

#[derive(Resource)]
pub struct AudioLibrary {
    music_tracks: Vec<MusicTrackRuntime>,
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
    /// Build the resident audio cache and the playback-state read model from the
    /// same registry in one operation. Hosts may still preload the returned
    /// state's active track before inserting both resources.
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
        let music_tracks = music_registry.tracks.iter().map(|track| MusicTrackRuntime {
            id: track.id.clone(),
            display_name: track.display_name.clone(),
            one_shot: track.one_shot,
            source: TrackSource {
                asset_path: resolve_track_path.and_then(|resolve| resolve(&track.id))
                    .unwrap_or_else(|| track.resolved_asset_path()),
                handle: None,
            },
        }).collect();
        Self { music_tracks }
    }

    /// Replace one provider's tracks, for a reload of that provider's pack:
    /// `previous` is the music the provider stated before, `next` what it
    /// states now. A track of another provider is kept as it is, and so is a
    /// track the provider did not change (with the path the host resolved
    /// for it). A changed or new track takes its own path
    /// ([`crate::spec::MusicTrack::resolved_asset_path`]) and loads on first
    /// use; a dropped track is removed.
    pub fn revise_provider_tracks(&mut self, previous: Option<&MusicRegistry>, next: Option<&MusicRegistry>) {
        let previous: &[crate::spec::MusicTrack] = previous.map_or(&[], |music| &music.tracks);
        let next: &[crate::spec::MusicTrack] = next.map_or(&[], |music| &music.tracks);
        self.music_tracks
            .retain(|track| !previous.iter().any(|old| old.id == track.id) || next.iter().any(|new| new.id == track.id));
        for track in next {
            if previous.contains(track) && self.track(&track.id).is_some() {
                continue;
            }
            let asset_path = track.resolved_asset_path();
            let known = self.music_tracks.iter_mut().find(|known| known.id == track.id);
            // A track whose file did not move keeps its loaded handle.
            let handle = known
                .as_ref()
                .filter(|known| known.source.asset_path == asset_path)
                .and_then(|known| known.source.handle.clone());
            let runtime = MusicTrackRuntime {
                id: track.id.clone(),
                display_name: track.display_name.clone(),
                one_shot: track.one_shot,
                source: TrackSource { asset_path, handle },
            };
            match known {
                Some(known) => *known = runtime,
                None => self.music_tracks.push(runtime),
            }
        }
    }

    pub fn track(&self, id: &str) -> Option<&MusicTrackRuntime> {
        self.music_tracks.iter().find(|track| track.id == id)
    }

    /// Return an already-resolved playable handle without starting a load.
    ///
    /// A host that preloads a track can put the exact handle in an
    /// activation-readiness manifest without mutable access to the library.
    pub fn resolved_track_handle(&self, id: &str) -> Option<Handle<KiraAudioSource>> {
        self.track(id).and_then(|track| track.source.handle.clone())
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
        let track = self
            .music_tracks
            .iter_mut()
            .find(|track| track.id == track_id)?;
        if track.source.handle.is_none() {
            track.source.handle = Some(asset_server.load(track.source.asset_path.clone()));
        }
        Some((track.source.handle.clone()?, track.one_shot))
    }

    /// Warm a file-backed track's handle before likely use (for example when
    /// the radio menu highlights it).
    pub fn preload_track(&mut self, track_id: &str, asset_server: &AssetServer) {
        let _ = self.resolve_track_handle(track_id, asset_server);
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
        }
    }

    /// What is playing, or empty for silence.
    pub fn active_track(&self) -> &str {
        &self.active_track
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
        self.play_generation = self.play_generation.wrapping_add(1);
    }

    /// The base channel stopped. A later play of the same track is a new play,
    /// so the generation moves here too.
    pub fn silence(&mut self) {
        self.active_track.clear();
        self.play_generation = self.play_generation.wrapping_add(1);
    }

    pub fn active_display_name<'a>(&self, library: &'a AudioLibrary) -> &'a str {
        library.display_name(&self.active_track)
    }
}

/// Whether [`start_default_music_when_ready`] has started music. Set `true`
/// on the frame the deferred play call lands, so overlays can show
/// "waiting for asset" instead of "silent".
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct DefaultMusicStarted(pub bool);

/// The set [`start_default_music_when_ready`] runs in.
///
/// Startup loading must finish before the default track starts, so the loader
/// runs `.before` this set.
///
/// One member. The system's run condition (`music_auto_start_when_ungated`)
/// does not propagate to the set, so `.before` this set holds whether or not
/// music starts.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DefaultMusicStart;

/// Start the default music once its handle is loaded. On web the OGG arrives
/// over HTTP several frames after startup, and `play(handle)` on an unloaded
/// handle can drop the request, so music never starts.
///
/// Also gated by [`AudioUnlockState::unlocked`]: on web the AudioContext is
/// suspended until a user gesture, and Kira schedules sounds on a suspended
/// context that never play.
pub fn start_default_music_when_ready(
    mut started: ResMut<DefaultMusicStarted>,
    unlock: Res<AudioUnlockState>,
    mut library: ResMut<AudioLibrary>,
    asset_server: Res<AssetServer>,
    state: Res<MusicPlaybackState>,
    music_channel: Res<AudioChannel<MusicChannel>>,
    output: Option<Res<crate::output::AudioOutputMode>>,
    mut waiting_logged: Local<bool>,
) {
    if started.0 {
        return;
    }
    if !unlock.unlocked {
        return;
    }
    let track_id = state.active_track.clone();
    let Some(_track) = library.track(&track_id) else {
        // No track in the library (for example every track lacks
        // `asset_path`). Log once, so the console shows why music never
        // starts.
        if !*waiting_logged {
            warn!(
                "default music: track '{}' not present in AudioLibrary; \
                 nothing to start (check platformer_defaults.ron music_tracks + catalog)",
                track_id
            );
            *waiting_logged = true;
            started.0 = true; // stop polling — there's nothing to wait for
        }
        return;
    };
    // Resolve / allocate the handle so we have an asset id to query.
    let Some(handle) = library.resolve_track_handle(&track_id, &asset_server) else {
        return;
    };
    let asset_path = library
        .track(&track_id)
        .map(|t| t.asset_path().to_string())
        .unwrap_or_default();
    if !asset_server.is_loaded(&handle) {
        if !*waiting_logged {
            info!(
                "default music: waiting for asset `{}` (track `{}`) to load \
                 before first play",
                asset_path, track_id
            );
            *waiting_logged = true;
        }
        return;
    }
    info!(
        "default music: track `{}` asset `{}` loaded; starting playback",
        track_id, asset_path
    );
    if crate::output::emits_to_device(output.as_deref()) {
        music_channel.play(handle).looped().fade_in(AudioTween::new(
            Duration::from_millis(220),
            AudioEasing::InPowi(2),
        ));
    }
    started.0 = true;
}

pub fn switch_to_music_track(
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    state: &mut MusicPlaybackState,
    music_channel: &AudioChannel<MusicChannel>,
    output: crate::output::AudioOutputMode,
    next_track: &str,
) {
    switch_to_music_track_over(library, asset_server, state, music_channel, output, next_track, None);
}

/// [`switch_to_music_track`], with the old track fading out and the new one
/// fading in over `crossfade` together: a change that is part of the same
/// place (the time of day turning) rather than a cut to another. `None` is
/// the ordinary swap (a 180 ms fade out, a 220 ms fade in).
pub fn switch_to_music_track_over(
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    state: &mut MusicPlaybackState,
    music_channel: &AudioChannel<MusicChannel>,
    output: crate::output::AudioOutputMode,
    next_track: &str,
    crossfade: Option<Duration>,
) {
    if library.track(next_track).is_none() {
        warn!("cannot switch to missing music track '{next_track}'");
        return;
    }
    state.begin_track(next_track);
    if output.emits_to_device() {
        let fade_out = crossfade.unwrap_or(Duration::from_millis(180));
        music_channel.stop().fade_out(AudioTween::new(fade_out, AudioEasing::OutPowi(2)));
    }
    let fade_in = crossfade.unwrap_or(Duration::from_millis(220));
    play_music_track(library, asset_server, next_track, music_channel, output, fade_in);
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

fn play_music_track(
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    track_id: &str,
    music_channel: &AudioChannel<MusicChannel>,
    output: crate::output::AudioOutputMode,
    fade_in: Duration,
) {
    let Some((handle, one_shot)) = library.resolve_track(track_id, asset_server) else {
        warn!("cannot play missing music track '{track_id}'");
        return;
    };
    if output.emits_to_device() {
        let fade = AudioTween::new(fade_in, AudioEasing::InPowi(2));
        // A sting ends. Looping a three-second fanfare would repeat it until
        // the sequence ends.
        if one_shot {
            music_channel.play(handle).fade_in(fade);
        } else {
            music_channel.play(handle).looped().fade_in(fade);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::MusicTrack;

    fn track(id: &str, asset_path: Option<&str>) -> MusicTrack {
        MusicTrack { id: id.into(), display_name: id.into(), asset_path: asset_path.map(Into::into), one_shot: false }
    }

    fn music(tracks: Vec<MusicTrack>) -> MusicRegistry {
        MusicRegistry { default_track: tracks[0].id.clone(), tracks }
    }

    fn library(tracks: &[(&str, &str)]) -> AudioLibrary {
        AudioLibrary {
            music_tracks: tracks
                .iter()
                .map(|(id, path)| MusicTrackRuntime {
                    id: id.to_string(),
                    display_name: id.to_string(),
                    one_shot: false,
                    source: TrackSource { asset_path: path.to_string(), handle: None },
                })
                .collect(),
        }
    }

    fn paths(library: &AudioLibrary) -> Vec<(String, String)> {
        let mut paths: Vec<_> =
            library.music_tracks.iter().map(|track| (track.id.clone(), track.asset_path().to_string())).collect();
        paths.sort();
        paths
    }

    /// ⭐ A reload of one provider's music changes its tracks and no other
    /// provider's, and keeps the host's path for a track it did not change.
    #[test]
    fn a_provider_reload_revises_its_own_tracks_only() {
        let before = music(vec![track("kept", None), track("moved", None), track("dropped", None)]);
        let after = music(vec![track("kept", None), track("moved", Some("audio/other.ogg")), track("added", None)]);
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
                ("added".to_string(), "audio/music/generated/added/full.ogg".to_string()),
                ("host_theme".to_string(), "host/theme.ogg".to_string()),
                ("kept".to_string(), "catalog/kept.ogg".to_string()),
                ("moved".to_string(), "audio/other.ogg".to_string()),
            ]
        );
    }
}
