# Procedural Lo-Fi Music — RETIRED

> **End-of-life note.** The "render lo-fi music at Rust startup" pipeline
> has been removed from the runtime. Music in Ambition is now authored:
> every `MusicTrackSpec` declares an `asset_path` pointing at a
> pre-rendered OGG, which `bevy_kira_audio` loads through Bevy's asset
> server. See `docs/archive/retired/fundsp-audio.md` for the broader context.

## Where the OGGs come from

`tools/ambition_music_renderer` is the offline authoring tool. It takes
a YAML cue file describing the arrangement and writes
`assets/audio/music/generated/<id>/full.ogg`. The RON manifest
(`crates/ambition_sandbox/assets/ambition/sandbox.ron`) points at those
files via `asset_path: Some("audio/music/generated/<id>/full.ogg")`.

To add or revise a track:

1. Author / edit the YAML cue under
   `tools/ambition_music_renderer/scores/`.
2. Run the renderer to produce the OGG under
   `assets/audio/music/generated/<id>/`.
3. Add or update the matching `MusicTrackSpec` row in `sandbox.ron`,
   making sure `asset_path` is set (the runtime warns and skips tracks
   with `asset_path: None`).
4. The asset manager catalog
   (`crates/ambition_sandbox/src/sandbox_assets.rs::extend_with_music_entries`)
   picks up every track with an `asset_path` automatically — no
   additional Rust changes needed.

## What the `arrangement` field is for now

`MusicTrackSpec::arrangement` (BPM, chord progression, bass roots,
gains, etc.) is **retained as documentation** of how the OGG was
authored. The runtime no longer renders from it; `duration_seconds()`
is the only reader.

It's safe to leave existing entries as-is. New tracks may carry minimal
`arrangement` placeholders if they're authored entirely in the YAML
cue rather than in RON.

## Adaptive cues

Section/layer adaptive cues (e.g. the goblin combat music) work the
same way — every section/layer references a pre-rendered audio file
via `crate::music::MusicLayerSourceSpec`. The music director crossfades
those layers per the cue catalog. No procedural synthesis anywhere in
the runtime path.

## Future: realtime DSP

Live effects (filter sweeps, underwater muffling, reverb-ish ambience)
are still on the roadmap and would re-introduce something like FunDSP
behind an `audio_fx` feature, **but only as a processing layer over
authored playback** — not as a content generator.
