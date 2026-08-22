//! Authored-audio playback library: typed SFX cue table, lazily-loaded
//! pre-rendered music tracks, the music/SFX Kira channels, and the
//! track-switch/radio/default-start helpers. Game-side adapters (which
//! requests apply, where the bank bytes come from) stay in the host.

use crate::render::{audio_source_from_sfx_clip, silent_audio_source};
use crate::spec::{MusicRegistry, SfxRegistry, SoundCueKey};
use crate::web_unlock::AudioUnlockState;
use ambition_sfx::{SfxId, SfxMessage, SfxProvider};
use bevy::platform::collections::HashMap;
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

/// Maps the typed [`SfxMessage`] variants to the sandbox's [`SoundCue`]
/// table. `SfxMessage` now lives in the `ambition_sfx` crate (so reusable
/// mechanics can request sound without naming a sandbox module), but
/// `SoundCue` is a sandbox-internal mapping — hence this consumer-side
/// extension trait rather than an inherent method on the foreign type.
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

/// The [`SfxId`] a message resolves to for provider-authority checks: the id of
/// its typed cue, or the open-ended `Play { id }`'s id. The gate uses the id the
/// emitter *requested* — before the `Play`→cue rescue in the consumer — so a
/// provider is judged on what it actually authored.
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
    /// any. Lets the open-ended `SfxMessage::Play { id }` path (used by the
    /// data-driven moveset, whose events carry a string cue) resolve to a
    /// guaranteed procedural cue when the string names one — instead of only
    /// ever hitting the packed bank and silently no-op-ing on a bank miss.
    pub fn from_sfx_id(id: SfxId) -> Option<Self> {
        Self::ALL.into_iter().find(|cue| cue.sfx_id() == id)
    }

    pub fn sfx_id(self) -> SfxId {
        // Delegate to the kira-free [`SoundCueKey::sfx_id`] table so the
        // authority projection (used to gate provider-relative playback) and
        // the playback handle lookup can never drift out of sync.
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
    /// This cue plays once and stops rather than looping — see
    /// [`crate::spec::MusicTrack::one_shot`].
    pub one_shot: bool,
    source: TrackSource,
}

impl MusicTrackRuntime {
    /// Logical asset path the AssetServer was (or will be) asked to
    /// load. Used in diagnostics / log lines; the live handle lives
    /// behind the lazy `resolve_track_handle` call.
    pub fn asset_path(&self) -> &str {
        &self.source.asset_path
    }
}

#[derive(Resource)]
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

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.selected_track = None;
    }
}

impl AudioLibrary {
    /// Build the resident audio cache and the playback-state read model from the
    /// same registry in one operation. Hosts may still preload the returned
    /// state's active track before inserting both resources.
    pub fn new_with_playback_state(
        audio_sources: &mut Assets<KiraAudioSource>,
        sfx_registry: &SfxRegistry,
        music_registry: &MusicRegistry,
        asset_server: Option<&AssetServer>,
        sfx_provider: Option<&dyn SfxProvider>,
        resolve_track_path: Option<&dyn Fn(&str) -> Option<String>>,
    ) -> (Self, MusicPlaybackState) {
        let library = Self::new(
            audio_sources,
            sfx_registry,
            music_registry,
            asset_server,
            sfx_provider,
            resolve_track_path,
        );
        let playback = MusicPlaybackState::from_music_registry(music_registry, &library);
        (library, playback)
    }

    /// Build the audio library + music track table.
    ///
    /// `resolve_track_path` (when `Some`) resolves each music track id
    /// through the host's asset catalog so the runtime stores the
    /// catalog-blessed path instead of the track's conventional
    /// `audio/music/generated/{id}/full.ogg`. A genuinely missing OGG
    /// surfaces as a load warning later — there is no procedural fallback.
    ///
    /// `resolve_track_path = None` is the test-fixture / pre-catalog seam: the
    /// library reads each track's [`MusicTrack::resolved_asset_path`] directly.
    pub fn new(
        audio_sources: &mut Assets<KiraAudioSource>,
        sfx_registry: &SfxRegistry,
        music_registry: &MusicRegistry,
        _asset_server: Option<&AssetServer>,
        sfx_provider: Option<&dyn SfxProvider>,
        resolve_track_path: Option<&dyn Fn(&str) -> Option<String>>,
    ) -> Self {
        if let Err(error) = sfx_registry.validate() {
            warn!("invalid sfx registry: {error}");
        }
        if let Err(error) = music_registry.validate() {
            warn!("invalid music registry: {error}");
        }
        let sample_rate = sfx_registry.sample_rate.max(8_000);

        // SFX: every cue tries the bank; missing entries get a short
        // silent stub so the playback path stays uniform without
        // surfacing per-call warnings (the bank-cache layer logs once
        // when it sees the gap).
        let silent_handle = audio_sources.add(silent_audio_source(sample_rate));
        let mut sfx_handles = HashMap::default();
        let mut missing_cues: Vec<SoundCue> = Vec::new();
        for cue in SoundCue::ALL {
            let from_bank = sfx_provider
                .and_then(|provider| provider.provide_clip(cue.sfx_id()))
                .and_then(|clip| match audio_source_from_sfx_clip(clip) {
                    Ok(source) => Some(audio_sources.add(source)),
                    Err(error) => {
                        warn!("bank entry for {cue:?} failed to decode ({error})");
                        None
                    }
                });
            let handle = match from_bank {
                Some(handle) => handle,
                None => {
                    missing_cues.push(cue);
                    silent_handle.clone()
                }
            };
            sfx_handles.insert(cue, handle);
        }
        if !missing_cues.is_empty() {
            warn!(
                "audio library: no SFX bank entry for {} cue(s): {:?} — playing silent stubs. \
                 Repack the bank via `tools/ambition_sfx_pack` or check the active asset profile.",
                missing_cues.len(),
                missing_cues
            );
        }
        let fallback_sfx = silent_handle;

        // Music: each track resolves to a pre-rendered OGG path — the
        // catalog-blessed path when a resolver is supplied, otherwise the
        // track's conventional `audio/music/generated/{id}/full.ogg`
        // (see `MusicTrack::resolved_asset_path`). A genuinely missing OGG
        // surfaces later as a load warning; there is no fundsp fallback.
        let mut music_tracks = Vec::with_capacity(music_registry.tracks.len());
        for track in &music_registry.tracks {
            let asset_path = resolve_track_path
                .and_then(|resolve| resolve(&track.id))
                .unwrap_or_else(|| track.resolved_asset_path());
            music_tracks.push(MusicTrackRuntime {
                id: track.id.clone(),
                display_name: track.display_name.clone(),
                one_shot: track.one_shot,
                source: TrackSource {
                    asset_path,
                    handle: None,
                },
            });
        }

        Self {
            sfx: sfx_handles,
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

    /// Return an already-resolved playable handle without starting a load.
    ///
    /// Hosts that explicitly preload a track can use this to include the exact
    /// handle in a broader activation-readiness manifest without taking mutable
    /// ownership of the audio library every frame.
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

    /// The handle plus whether this cue loops. A sting must not repeat, and the
    /// spec is the only thing that knows which cues are stings.
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

    /// Warm a file-backed track's handle ahead of likely use (e.g. when
    /// the radio menu highlights it).
    pub fn preload_track(&mut self, track_id: &str, asset_server: &AssetServer) {
        let _ = self.resolve_track_handle(track_id, asset_server);
    }
}

/// What the base music channel is playing, and WHICH PLAY it is.
///
/// the generation is the difference between "still playing" and "started again". The track
/// name alone cannot tell those apart — a screen transition that stops the title theme and
/// immediately restarts the identical file leaves exactly the state it found.
///
/// This is deliberately NOT an event log. Wwise and FMOD ship capture logs and
/// Unreal has trace channels, but that is a debugging surface for a whole audio
/// department; the question here — *did this song restart?* — is a fact about
/// the state, and the state should be able to answer it. If audio ever earns a
/// timeline it belongs in `ambition_causal` as a domain, not in a second
/// observability authority.
///
/// The fields are private so the counter cannot be forgotten: starting a track
/// and recording that it started are ONE operation ([`Self::begin_track`]), not
/// a write plus a follow-up call somebody remembers.
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

    /// How many times the base channel has been started. Only equality across
    /// two observations is meaningful — same generation means the same
    /// uninterrupted play.
    pub fn play_generation(&self) -> u64 {
        self.play_generation
    }

    /// A track STARTED on the base channel. Bumps the generation even when the
    /// track id is unchanged, because restarting the same song is precisely the
    /// event this records.
    pub fn begin_track(&mut self, track_id: impl Into<String>) {
        self.active_track = track_id.into();
        self.play_generation = self.play_generation.wrapping_add(1);
    }

    /// The base channel stopped. A later play of the same track is a NEW play,
    /// so the generation moves here too.
    pub fn silence(&mut self) {
        self.active_track.clear();
        self.play_generation = self.play_generation.wrapping_add(1);
    }

    pub fn active_display_name<'a>(&self, library: &'a AudioLibrary) -> &'a str {
        library.display_name(&self.active_track)
    }
}

/// Tracks whether [`start_default_music_when_ready`] has actually
/// kicked off the music playback. Inserted at startup, flipped to
/// `true` the frame the deferred play call lands. Exposed so other
/// systems / overlays can show "audio: waiting for asset…" instead
/// of "audio: silent".
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct DefaultMusicStarted(pub bool);

/// The set [`start_default_music_when_ready`] runs in.
///
/// Startup loading must settle before the default track is allowed to begin, so
/// the loader pins `.before` this.
///
/// ONE member. The system is already gated by a run condition
/// (`music_auto_start_when_ungated`); a run condition on the only member does
/// NOT propagate to the set, so `.before` this set holds whether or not the
/// music actually starts — which is the behaviour the pin wants.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DefaultMusicStart;

/// Important on web, where the music OGG is fetched over HTTP and may not be ready until
/// several frames after startup — calling `play(handle)` on a not-yet-loaded handle either
/// drops the request silently or fires a soft warning, depending on bevy_kira_audio's internal
/// state, and the music never starts.
///
/// Also gated by [`AudioUnlockState::unlocked`]: on web the AudioContext is `suspended` until a
/// user gesture, and Kira's `play()` call on a suspended context schedules sounds that never
/// audibly play.
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
        // No track at all in the library (e.g. all music tracks
        // missing asset_path); nothing to do, but log once so the
        // browser console / log shows why music never starts.
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
    if library.track(next_track).is_none() {
        warn!("cannot switch to missing music track '{next_track}'");
        return;
    }
    state.begin_track(next_track);
    if output.emits_to_device() {
        music_channel.stop().fade_out(AudioTween::new(
            Duration::from_millis(180),
            AudioEasing::OutPowi(2),
        ));
    }
    play_music_track(library, asset_server, next_track, music_channel, output);
}

pub fn set_radio_track(
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    radio: &mut RadioStationState,
    state: &mut MusicPlaybackState,
    music_channel: &AudioChannel<MusicChannel>,
    output: crate::output::AudioOutputMode,
    next_track: &str,
) {
    if library.track(next_track).is_none() {
        warn!("cannot set radio to missing music track '{next_track}'");
        return;
    }
    radio.set_selected_track(next_track);
    switch_to_music_track(
        library,
        asset_server,
        state,
        music_channel,
        output,
        next_track,
    );
}

fn play_music_track(
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    track_id: &str,
    music_channel: &AudioChannel<MusicChannel>,
    output: crate::output::AudioOutputMode,
) {
    let Some((handle, one_shot)) = library.resolve_track(track_id, asset_server) else {
        warn!("cannot play missing music track '{track_id}'");
        return;
    };
    if output.emits_to_device() {
        let fade = AudioTween::new(Duration::from_millis(220), AudioEasing::InPowi(2));
        // A sting ends. Looping one turns a three-second fanfare into a nag that
        // runs until whatever ends the sequence gets round to it.
        if one_shot {
            music_channel.play(handle).fade_in(fade);
        } else {
            music_channel.play(handle).looped().fade_in(fade);
        }
    }
}
