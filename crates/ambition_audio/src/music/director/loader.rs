use super::*;

/// Load file-backed cue sources for the HOST-provided cue catalog
/// (inserted by the host's audio plugin before startup systems run).
pub fn load_music_cues(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    catalog: Res<MusicCueCatalog>,
) {
    let mut sources = HashMap::new();
    for cue in catalog.cues.values() {
        for section in &cue.sections {
            for source in &section.sources {
                let rel = format!("{}/{}", cue.asset_root.trim_end_matches('/'), source.path);
                sources.insert(
                    MusicSourceKey::new(&cue.id, &section.id, &source.layer_id),
                    asset_server.load(rel),
                );
            }
        }
        info!(
            target: MUSIC_LOG_TARGET,
            "loaded music cue id={} sections={} layers={}",
            cue.id,
            cue.sections.len(),
            cue.layers.len(),
        );
    }

    commands.insert_resource(LoadedMusicCueAssets { sources });
    commands.insert_resource(MusicDirectorState::default());
}
