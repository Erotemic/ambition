//! Game-side audio adapters: map the sandbox's music REQUESTS
//! (encounter / boss / room) onto the `ambition_audio` library, and
//! play `SfxMessage`s through the bank-backed handle cache. The
//! reusable playback library itself lives in the `ambition_audio`
//! crate; `crate::audio` re-exports it.

#[cfg(feature = "audio")]
use super::*;
// `SfxMessage` no longer re-exported by the parent module (§D1) — name its
// real home directly.
#[cfg(feature = "audio")]
use ambition_sfx::SfxMessage;

#[cfg(feature = "audio")]
#[cfg(feature = "audio")]
pub fn apply_encounter_music(
    mut library: ResMut<AudioLibrary>,
    asset_server: Res<AssetServer>,
    mut music_state: ResMut<MusicPlaybackState>,
    music_channel: Res<AudioChannel<MusicChannel>>,
    mut request: ResMut<crate::encounter::EncounterMusicRequest>,
    mut boss_request: ResMut<crate::encounter::BossEncounterMusicRequest>,
    room_music: Res<crate::rooms::RoomMusicRequest>,
    music_registry: Res<crate::session::data::MusicRegistry>,
) {
    let resolved_default = room_music
        .desired_track
        .as_ref()
        .filter(|id| library.track(id).is_some())
        .cloned()
        .unwrap_or_else(|| music_registry.default_track.clone());
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

#[cfg(feature = "audio")]
#[cfg(feature = "audio")]
pub fn audio_play_sfx_messages(
    mut messages: MessageReader<SfxMessage>,
    library: Res<AudioLibrary>,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
    bank: Option<Res<crate::audio::SfxBankResource>>,
    mut cache: ResMut<SfxBankHandleCache>,
    mut audio_sources: ResMut<Assets<KiraAudioSource>>,
    mut first_play_logged: Local<bool>,
) {
    for message in messages.read() {
        if !*first_play_logged {
            info!(
                target: ambition_audio::web_unlock::AUDIO_LOG_TARGET,
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
        // A string-keyed `Play` that names a procedural cue (e.g. the moveset's
        // "player.slash" swing) resolves to the guaranteed procedural sound rather
        // than the bank — so a cue with no bank sample still plays instead of
        // silently no-op-ing. Genuine bank ids (no matching cue) fall through.
        if let Some(cue) = SoundCue::from_sfx_id(id) {
            sfx_channel.play(library.sfx_handle(cue));
            continue;
        }
        let bank_provider = bank
            .as_deref()
            .map(|bank| &*bank.0 as &dyn ambition_sfx::SfxProvider);
        let Some(handle) = cache.handle_for(id, bank_provider, audio_sources.as_mut()) else {
            continue;
        };
        sfx_channel.play(handle);
    }
}
