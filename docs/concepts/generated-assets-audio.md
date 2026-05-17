---
id: generated-assets-audio
aliases:
  - procedural audio
  - generated music
  - generated sprites
  - asset generation
  - reproducible assets
implemented_by:
  - tools/ambition_music_renderer
  - crates/ambition_sandbox/src/audio.rs
  - crates/ambition_sandbox/src/music.rs
  - crates/ambition_asset_manager/src
  - assets/
related_docs:
  - docs/recipes/music-generation-pipeline-notes.md
  - docs/recipes/music-transition-notes.md
  - docs/systems/ai-generation-contract.md
  - docs/systems/asset-manager.md
related_memory:
  - dev/benchmark-candidates/procedural-audio-questions.md
  - dev/journals/music-director-refactor-lessons-2026-05-11.md
last_verified: 2026-05-17
---

# Generated assets and audio

## Definition

Ambition uses code-owned and data-owned generation for music, SFX, sprites, backgrounds, and asset catalogs. Generated outputs should remain inspectable and reproducible rather than becoming opaque binary state.

## Core invariants

- Source specs and generator code are the authority; generated `assets/` output may be ignored or reproducible.
- Audio generator default behavior should skip unchanged YAML/spec inputs, with an explicit force path when generator code changes.
- Adaptive music state must not allow simple base tracks and adaptive cue layers to play as mutually-exclusive authorities at the same time.
- Asset behavior must be platform-aware: Android APK assets are not host filesystem paths.

## Edit protocol

1. Identify the source spec, generator, runtime manifest, and playback adapter involved.
2. Preserve generator reproducibility and clear terminal diagnostics.
3. Search dev memory for audio/director/asset/platform symptoms.
4. Update docs or recipes when a generator invocation or asset authority changes.

## Validation

```bash
cargo test -p ambition_sandbox --lib music
./regen_music.sh
./regen_sfx.sh
cargo run -p ambition_sandbox --bin headless
```

Use generator-specific commands when working on sprites/backgrounds instead of audio.
