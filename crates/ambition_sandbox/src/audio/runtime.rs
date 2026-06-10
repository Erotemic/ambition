#[cfg(feature = "audio")]
use super::render::{audio_source_from_sfx_clip, silent_audio_source};
use super::*;

pub const ORIGINAL_TRACK_ID: &str = "original_lofi_loop";

#[cfg(feature = "audio")]
#[derive(Resource)]
pub struct MusicChannel;

#[cfg(feature = "audio")]
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
    bank: Option<Res<crate::runtime::setup::SfxBankResource>>,
    mut cache: ResMut<SfxBankHandleCache>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    mut first_play_logged: Local<bool>,
) {
    for message in messages.read() {
        if !*first_play_logged {
            info!(
                target: super::web_unlock::AUDIO_LOG_TARGET,
                "ambition audio: first SFX play attempt (cue={:?}, bank_loaded={})",
                message.cue(),
                bank.is_some()
            );
            *first_play_logged = true;
        }
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
    mut library: ResMut<AudioLibrary>,
    asset_server: Res<AssetServer>,
    mut music_state: ResMut<MusicPlaybackState>,
    music_channel: Res<AudioChannel<MusicChannel>>,
    mut request: ResMut<crate::encounter::EncounterMusicRequest>,
    mut boss_request: ResMut<crate::encounter::BossEncounterMusicRequest>,
    room_music: Res<crate::rooms::RoomMusicRequest>,
    sandbox_data: Res<crate::runtime::data::SandboxDataSpec>,
) {
    let resolved_default = room_music
        .desired_track
        .as_ref()
        .filter(|id| library.track(id).is_some())
        .cloned()
        .unwrap_or_else(|| sandbox_data.audio.default_music_track.clone());
    // Priority: boss encounter > regular encounter > room default.
    // Boss music has its own resource so the regular encounter
    // tick can't clobber it by writing `desired_track = None`
    // every frame there's no in-flight encounter.
    let target = boss_request
        .desired_track
        .clone()
        .or_else(|| request.desired_track.clone())
        .unwrap_or(resolved_default);
    let last_applied_matches = boss_request.last_applied.as_ref() == Some(&target)
        || request.last_applied.as_ref() == Some(&target);
    let already_applied = last_applied_matches && music_state.active_track == target;
    if !already_applied && library.track(&target).is_some() {
        switch_to_music_track(
            &mut library,
            &asset_server,
            &mut music_state,
            &music_channel,
            &target,
        );
    }
    request.last_applied = Some(target.clone());
    boss_request.last_applied = Some(target);
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

/// Backing storage for a music track's audio source. Always file-backed
/// now (procedural fallback removed); the handle is lazily allocated on
/// the first `resolve_track_handle` call so the catalog's 25+ OGGs don't
/// all hit the asset IO queue at startup.
#[derive(Clone)]
#[cfg(feature = "audio")]
struct TrackSource {
    asset_path: String,
    handle: Option<Handle<KiraAudioSource>>,
}

#[derive(Clone)]
#[cfg(feature = "audio")]
pub struct MusicTrackRuntime {
    pub id: String,
    pub display_name: String,
    pub duration_seconds: f32,
    source: TrackSource,
}

#[cfg(feature = "audio")]
impl MusicTrackRuntime {
    /// Logical asset path the AssetServer was (or will be) asked to
    /// load. Used in diagnostics / log lines; the live handle lives
    /// behind the lazy `resolve_track_handle` call.
    pub fn asset_path(&self) -> &str {
        &self.source.asset_path
    }
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
    /// Build the audio library + music track table.
    ///
    /// `catalog` (when `Some`) resolves each music track id through the
    /// `music_track` id helper in `crate::assets::sandbox_assets::ids`
    /// (private) so the runtime stores the catalog-blessed path
    /// instead of the raw `MusicTrackSpec::asset_path`. Tracks without an `asset_path`
    /// (and with no catalog-resolved path) are skipped with a loud
    /// warning — there is no procedural fallback anymore. Author a
    /// pre-rendered OGG at the path the spec points to, or drop the
    /// track from the list.
    ///
    /// `catalog = None` is the test-fixture / pre-catalog seam: the
    /// library reads paths directly from `spec.music_tracks`.
    pub fn new(
        audio_sources: &mut Assets<KiraAudioSource>,
        spec: &AudioSpec,
        _asset_server: Option<&AssetServer>,
        sfx_provider: Option<&dyn SfxProvider>,
        catalog: Option<&crate::assets::sandbox_assets::SandboxAssetCatalog>,
    ) -> Self {
        if let Err(error) = spec.validate() {
            warn!("invalid audio spec: {error}");
        }
        let sample_rate = spec.sample_rate.max(8_000);

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

        // Music: every track must have an `asset_path` (either authored
        // on `MusicTrackSpec` or resolved by the catalog). Skip silently
        // missing ones with a loud warning — no fundsp fallback exists.
        let mut music_tracks = Vec::with_capacity(spec.music_tracks.len());
        let mut skipped: Vec<String> = Vec::new();
        for track in &spec.music_tracks {
            let catalog_path = catalog.and_then(|catalog| {
                catalog.path_for(&crate::assets::sandbox_assets::ids::music_track(&track.id))
            });
            let effective_path = catalog_path.or_else(|| track.asset_path.clone());
            let Some(asset_path) = effective_path else {
                skipped.push(track.id.clone());
                continue;
            };
            music_tracks.push(MusicTrackRuntime {
                id: track.id.clone(),
                display_name: track.display_name.clone(),
                duration_seconds: track.arrangement.duration_seconds(),
                source: TrackSource {
                    asset_path,
                    handle: None,
                },
            });
        }
        if !skipped.is_empty() {
            warn!(
                "audio library: skipped {} music track(s) with no asset_path: {:?}. \
                 Author a pre-rendered OGG (see tools/ambition_music_renderer) or remove from sandbox.ron.",
                skipped.len(),
                skipped
            );
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

    /// Replace the per-cue SFX handles with freshly-decoded clips from
    /// `sfx_provider`. Used when an async-loaded SFX bank arrives
    /// (`audio/bank_asset.rs::promote_loaded_sfx_bank`) after the
    /// library was already constructed with no bank — without this
    /// refresh the typed cues would stay silent for the whole session.
    /// Missing entries keep the existing handle (silent stub by default).
    pub fn refresh_sfx_from_bank(
        &mut self,
        audio_sources: &mut Assets<KiraAudioSource>,
        sfx_provider: &dyn SfxProvider,
    ) {
        let mut refreshed = 0usize;
        for cue in SoundCue::ALL {
            let Some(clip) = sfx_provider.provide_clip(cue.sfx_id()) else {
                continue;
            };
            match audio_source_from_sfx_clip(clip) {
                Ok(source) => {
                    let handle = audio_sources.add(source);
                    self.sfx.insert(cue, handle);
                    refreshed += 1;
                }
                Err(error) => {
                    warn!("bank entry for {cue:?} failed to decode ({error})");
                }
            }
        }
        if refreshed > 0 {
            info!("audio library: refreshed {refreshed} typed SFX cue handle(s) from bank");
        }
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

    /// Resolve a music track's playable handle, loading from disk on the
    /// first request. Returns `None` only for missing IDs.
    pub fn resolve_track_handle(
        &mut self,
        track_id: &str,
        asset_server: &AssetServer,
    ) -> Option<Handle<KiraAudioSource>> {
        let track = self
            .music_tracks
            .iter_mut()
            .find(|track| track.id == track_id)?;
        if track.source.handle.is_none() {
            track.source.handle = Some(asset_server.load(track.source.asset_path.clone()));
        }
        track.source.handle.clone()
    }

    /// Warm a file-backed track's handle ahead of likely use (e.g. when
    /// the radio menu highlights it).
    pub fn preload_track(&mut self, track_id: &str, asset_server: &AssetServer) {
        let _ = self.resolve_track_handle(track_id, asset_server);
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

/// Tracks whether [`start_default_music_when_ready`] has actually
/// kicked off the music playback. Inserted at startup, flipped to
/// `true` the frame the deferred play call lands. Exposed so other
/// systems / overlays can show "audio: waiting for asset…" instead
/// of "audio: silent".
#[derive(Resource, Default, Clone, Copy, Debug)]
#[cfg(feature = "audio")]
pub struct DefaultMusicStarted(pub bool);

/// Update-loop variant of the old `start_default_music`. Polls the
/// asset server until the default music track's handle finishes
/// loading, then issues the `play` call once. Important on web,
/// where the music OGG is fetched over HTTP and may not be ready
/// until several frames after startup — calling `play(handle)` on a
/// not-yet-loaded handle either drops the request silently or fires
/// a soft warning, depending on bevy_kira_audio's internal state,
/// and the music never starts.
///
/// Also gated by [`AudioUnlockState::unlocked`]: on web the
/// AudioContext is `suspended` until a user gesture, and Kira's
/// `play()` call on a suspended context schedules sounds that never
/// audibly play. Deferring the first `play` until after the gesture
/// gives the JS unlock shim in `web/index.html` a chance to resume
/// the context first. Desktop builds flip `unlocked` to `true` on
/// the first `Update` frame, so behavior matches the old startup
/// system there.
#[cfg(feature = "audio")]
pub fn start_default_music_when_ready(
    mut started: ResMut<DefaultMusicStarted>,
    unlock: Res<super::web_unlock::AudioUnlockState>,
    mut library: ResMut<AudioLibrary>,
    asset_server: Res<AssetServer>,
    state: Res<MusicPlaybackState>,
    music_channel: Res<AudioChannel<MusicChannel>>,
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
                 nothing to start (check sandbox.ron music_tracks + catalog)",
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
    music_channel.play(handle).looped().fade_in(AudioTween::new(
        Duration::from_millis(220),
        AudioEasing::InPowi(2),
    ));
    started.0 = true;
}

#[cfg(feature = "audio")]
pub fn switch_to_music_track(
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
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
    play_music_track(library, asset_server, next_track, music_channel);
}

#[cfg(feature = "audio")]
pub fn set_radio_track(
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
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
    switch_to_music_track(library, asset_server, state, music_channel, next_track);
}

#[cfg(feature = "audio")]
fn play_music_track(
    library: &mut AudioLibrary,
    asset_server: &AssetServer,
    track_id: &str,
    music_channel: &AudioChannel<MusicChannel>,
) {
    let Some(handle) = library.resolve_track_handle(track_id, asset_server) else {
        warn!("cannot play missing music track '{track_id}'");
        return;
    };
    music_channel.play(handle).looped().fade_in(AudioTween::new(
        Duration::from_millis(220),
        AudioEasing::InPowi(2),
    ));
}
