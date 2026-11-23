use super::*;

/// Initialize native music state. Source demand starts at selection.
pub fn load_music_cues(mut commands: Commands) {
    commands.insert_resource(LoadedMusicCueAssets::default());
    commands.insert_resource(MusicDirectorState::default());
}
