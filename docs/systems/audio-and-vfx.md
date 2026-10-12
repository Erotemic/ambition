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

A provider says its audio in content-pack data, and `ambition_audio` owns the
three schemas: `music_registry` (tracks), `sfx_registry` (procedural cues) and
`music_cue_catalog` (adaptive cues: sections, layers, states, and the
encounters and rooms that bind to a cue). The compiler refuses a cue file whose
states, layers or bindings name nothing, and it requires each section's audio
file. Ambition's cues are in `game/ambition_content/assets/audio/music_cues.ron`.

Two bindings choose a cue state. An encounter binding follows the phases and
waves of one fight. A room binding gives a room one state of a cue, in place of
the room's `music_track`; rooms that bind the same cue share its playback, so a
walk from room to room changes the state and does not start the cue again. States
on one section change only the layer gains. A room binding can also name an entry
state (a section that plays one time when the cue starts) and a fight state.

A change of gains is continuous; a change of section is not (the new section
starts from its top). So a level whose music must never be cut puts every state
its rooms and its fight can ask for on ONE looped section, and gives each of its
rooms a binding: a room with none plays its `music_track` from the top. The
primeval trail is the example, and a test of `ambition_content` holds it to
that. An entry section always plays to its end; the section after it starts on
the frame nearest that end, at the state the game asks for then.

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
