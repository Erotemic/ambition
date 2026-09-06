---
status: current
last_verified: 2026-07-18
---

# Audio and VFX

Audio and visual effects are provider-extensible presentation consumers of
simulation facts. They may be absent in headless mode without changing outcomes.

## Audio ownership

- provider crates own named tracks, cues, SFX IDs/banks, voiceprints, and
  encounter bindings;
- `ambition_audio` owns content-free App-local catalogs, active provider
  selection, loading, mixing, adaptive music, web unlock, and output;
- `ambition_sfx` / `ambition_sfx_bank` own reusable cue/bank vocabulary where
  applicable;
- hosts provide device/backend and mix settings;
- simulation emits semantic sound intent rather than loading/playing assets.

Generated audio is rendered by author-time tools and explicitly published into
provider assets. Runtime does not synthesize the whole shipped bank at startup.

## VFX ownership

`ambition_vfx` provides presentation-neutral effect messages. Render/content
presentation plugins map those messages and simulation read models to sprites,
particles, trails, flashes, camera effects, and provider-specific looks.

Hitboxes, hurtboxes, projectile trajectories, shield state, and action timing
remain simulation authority. A particle/sprite may visualize them but cannot
define them.

## Invariants

- one semantic event should have one simulation emission site;
- providers may replace presentation without changing simulation;
- missing/degraded audio or VFX never changes game state;
- IDs resolve through the active provider/context;
- web audio readiness is explicit and not confused with game/content readiness;
- generated output has a reproducible source and explicit publish step.

## Validation

```bash
./run_tests.sh -p ambition_audio
./run_tests.sh -p ambition_content -k audio
./run_tests.sh -p ambition_render
./run_tests.sh -k gameplay_effect
```

Use [`../recipes/web-audio-manual-test.md`](../recipes/web-audio-manual-test.md)
for browser device acceptance.
