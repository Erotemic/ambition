# Generic music director overlay

This overlay replaces the goblin-specific `generated_music.rs` path with a
first-pass generic `music.rs` director.

The new split is:

- `AudioLibrary` remains the source cache for existing procedural/simple tracks.
- `MusicDirectorState` owns music priority and current playback mode.
- `MusicCueCatalog` describes adaptive cues as data: sections, layers, states,
  and encounter bindings.
- `LoadedMusicCueAssets` maps cue/section/layer sources to Kira handles.
- Adaptive music uses two generic layer banks for crossfades.

The current built-in adaptive cue is `first_goblin_tune_v2`. The runtime now
loads it as a generic cue rather than as `GeneratedGoblinMusicAssets`.

Follow-up direction: move `MusicCueCatalog::builtin()` into RON/YAML manifests
and convert the existing `AudioLibrary` tracks into explicit one-section,
one-layer `MusicCueSpec`s.
