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
  - tools/ambition_sfx_renderer
  - tools/ambition_sfx_pack
  - crates/ambition_gameplay_core/src/audio/mod.rs
  - crates/ambition_gameplay_core/src/music/mod.rs
  - crates/ambition_asset_manager/src
  - assets/
related_docs:
  - docs/recipes/generated-music-workflow.md
  - docs/tools/generated-audio-tools.md
  - docs/tools/generated-visual-tools.md
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


## Runtime wiring convention

When adding generated audio, wire runtime catalogs to the **generator cue id**
and the path that cue publishes to. Do not invent a temporary runtime-only id
unless you also add a generator spec with the same id.

For single-track music cues, the convention is:

```text
source: tools/ambition_music_renderer/scores/active/<cue>.music.yaml
runtime id: <cue>
runtime path: crates/ambition_gameplay_core/assets/audio/music/generated/<cue>/full.ogg
```

For SFX cues, the convention is:

```text
source: tools/ambition_sfx_renderer/sounds/active/<cue>.sfx.yaml
runtime id: <cue>
staged output: tools/ambition_sfx_renderer/output/<cue>/<cue>.ogg
runtime bank: crates/ambition_gameplay_core/assets/audio/sfx.bank
```

Generated OGG/WAV outputs are ignored by default. Check in the source YAML,
renderer code, runtime catalog ids, and SFX constants; then regenerate locally.

## Edit protocol

1. Identify the source spec, generator, runtime manifest, and playback adapter involved.
2. Preserve generator reproducibility and clear terminal diagnostics.
3. Search dev memory for audio/director/asset/platform symptoms.
4. Update docs or recipes when a generator invocation or asset authority changes.

## Validation

```bash
cargo test -p ambition_gameplay_core --lib music
./regen_music.sh
./regen_sfx.sh
cargo run -p ambition_gameplay_core --bin headless
```

Use generator-specific commands when working on sprites/backgrounds instead of audio.
