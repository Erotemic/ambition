use super::*;

/// Initialize the music director resources. Cue sources are NOT loaded here —
/// authored cues are large pre-rendered `.ogg` assets, and loading every catalog
/// cue at startup pulls in boss/encounter music that may never play this session
/// (and logged a misleading "loaded music cue ..." line at boot). Instead the
/// director lazily `asset_server.load()`s a cue's sources the first time its
/// `Play` directive fires (see `LoadedMusicCueAssets::ensure_cue_loaded`).
pub fn load_music_cues(mut commands: Commands) {
    commands.insert_resource(LoadedMusicCueAssets::default());
    commands.insert_resource(MusicDirectorState::default());
}
