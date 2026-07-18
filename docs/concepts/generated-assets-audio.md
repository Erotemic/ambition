---
id: generated-assets-audio
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
implemented_by:
  - tools/ambition_sprite2d_renderer
  - tools/ambition_sfx_renderer
  - tools/ambition_music_renderer
  - crates/ambition_audio
  - game/ambition_content/src/audio_registries.rs
related_docs:
  - docs/tools/generated-audio-tools.md
  - docs/tools/generated-visual-tools.md
  - docs/recipes/generated-music-workflow.md
---

# Generated assets and audio

Generated art/audio follows the same contract as hand-authored assets: source
specification, deterministic build, explicit publish/install step, provider-owned
logical identity, and reusable runtime loading/playback.

## Ownership

- Generator code and source specifications live under `tools/`.
- Root `regen_*.sh` scripts are the supported orchestration front doors.
- Generated files remain local/build output until explicitly published into a
  provider's runtime assets.
- Provider content owns named sprite/music/SFX registrations and IDs.
- `ambition_audio` owns content-free catalogs, selection, loading, mixing,
  web-unlock handling, and final playback.
- Render/sprite crates own reusable runtime presentation machinery, not a game's
  named roster.

## Invariants

- The same source/spec and tool version produce reproducible output.
- Generators do not silently overwrite runtime assets without an explicit
  install/publish command.
- Runtime code references logical/provider IDs rather than generator filenames
  where a catalog exists.
- A generated file is not authoritative if the source spec cannot reproduce it.
- Web/Android packaging is validated after publication.
- Audio/VFX absence cannot change simulation outcomes.

## Workflow shape

```text
source spec / generator target
    -> local render
    -> inspect/validate
    -> explicit publish/install
    -> provider catalog/registry
    -> reusable runtime loader/playback
```

Use the focused tool docs for exact commands; do not copy command inventories
into this concept page.
