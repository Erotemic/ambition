## Ideal end-state crate map

```text
crates/
  ambition_engine/
  ambition_audio/
  ambition_world/
  ambition_ldtk/
  ambition_bevy/
  ambition_devtools/
  ambition_actors/
  ambition_game/
```

I would **not** add all of these at once. The first useful split is probably `ambition_audio`, then `ambition_world`/`ambition_ldtk`, then `ambition_bevy` when `ambition_game` is real.

## 1. `ambition_engine`

This remains the reusable mechanics crate.

Owns:

```text
movement
collision semantics
abilities
combat hitboxes
actor/enemy/boss behavior kernels
world/block primitives
state-machine vocabulary
core geometry helpers
```

The current architecture already says `ambition_engine` owns movement, collision, abilities, combat hitboxes, enemy/test-target behavior, generated music/audio specs, and reusable mechanics, while the sandbox owns Bevy shell/presentation/input/debug/data. 

Over time, I’d probably move concrete generated audio out of `ambition_engine` and leave only gameplay-relevant symbolic concepts there.

## 2. `ambition_audio`

This should be the first new crate.

Owns:

```text
MusicTrackSpec
MusicSpec
NoteSpec
MusicGainsSpec
SfxSpec
SoundCueKey
procedural music/SFX validation
render-to-frames or render-to-WAV preview helpers
instrument/patch specs once added
```

Does **not** own:

```text
Kira channels
Bevy resources
pause-menu switching
game startup playback
```

Current docs already identify audio as a hard-coded area and say the next step is moving symbolic sound specs into `ambition_audio` if the sandbox audio layer becomes shared across game crates.  That is exactly where you are now: the sandbox and future game will both want generated tracks, tune previewing, SFX specs, and eventually different instruments.

## 3. `ambition_world`

This is for runtime world topology, not LDtk file details.

Owns:

```text
AreaId / RoomId / LoadingZoneId
RoomGraph
active-area composition
transition semantics
spawn repair policies
progression locks
non-Euclidean seam semantics
reachability validation
```

The current repo has LDtk as the sandbox world source, but it also has active-area composition, transition repair, room graph validity, loading-zone semantics, and hot reload behavior. Those should not be duplicated in `ambition_actors` and `ambition_game`.

## 4. `ambition_ldtk`

This should be the LDtk-native Ambition authoring adapter.

Owns:

```text
LDtk parsing/validation
Ambition LDtk entity definitions/field rules
LDtk -> ambition_world / ambition_engine runtime data
editor round-trip checks if moved from tools later
```

Does **not** mean:

```text
LDtk -> duplicate RON world manifest
```

The current state says the sandbox RON is no longer the room/world owner; LDtk is the checked-in sandbox world definition, and startup/hot reload build runtime data from the LDtk file.  So this crate should keep LDtk native, but centralize Ambition-specific semantics and validation.

## 5. `ambition_bevy`

This is the shared Bevy frontend layer used by both `ambition_actors` and `ambition_game`.

Owns reusable Bevy integration:

```text
app plugin groups
Leafwing -> engine input adapter
Kira playback adapter
common pause/settings framework
common camera controllers
asset loading state scaffolding
sim -> presentation event bridge
common HUD/debug primitives
LDtk runtime bridge systems if they stay Bevy-specific
```

Does **not** own:

```text
sandbox test rooms
campaign story
specific tune selection
specific maps
specific NPC content
```

This crate should appear once `ambition_game` exists or is imminent. Before that, it may be premature.

## 6. `ambition_devtools`

Optional, but likely useful.

Owns:

```text
inspector plugins
debug overlays
hot reload UI
room graph visualizers
music/tune debug panels
feature lab tools
```

The current docs explicitly say debug overlays should stay presentation-only and out of `ambition_engine`; the Bevy adapter should decide how to visualize deterministic engine state.  That can remain in `ambition_actors` for now, then move to `ambition_devtools` when the full game wants dev builds without inheriting every sandbox lab.

## 7. `ambition_actors`

After the split, this becomes thinner:

```text
testbed app
feature labs
all-abilities playground
sandbox-specific LDtk file
sandbox-specific RON tuning
experimental generated assets
debug-first UX
```

It should depend on shared crates, not own reusable systems.

## 8. `ambition_game`

This is the future full game crate.

Owns:

```text
campaign app shell
title/save/settings flow
curated world content
story/NPC selection
biome ordering
progression pacing
boss roster
release-facing UI skin
game-specific audio track selection
```

It should not copy movement, input mapping, Kira setup, LDtk semantics, room graph validation, generated audio rendering, or debug infrastructure.

## Phased refactor plan

### Phase 1: `ambition_audio`

Move only pure audio/data/rendering things.

```text
from ambition_actors:
  data audio structs
  procedural music/SFX renderer
  WAV preview writer
  tune validation
  tune preview tests

stay in ambition_actors:
  Kira AudioLibrary
  MusicChannel / SfxChannel
  pause menu track switching
  audio_play_sfx_messages
```

This is high-value because you are actively iterating on procedural music and instruments.

### Phase 2: `ambition_world`

Extract room graph and transition semantics that are not Bevy presentation.

```text
move:
  RoomId / AreaId
  LoadingZone semantics
  RoomGraph
  transition validation
  spawn repair policies
```

Leave LDtk parsing where it is until the boundary is clearer.

### Phase 3: `ambition_ldtk`

Move LDtk-specific validation/conversion once `ambition_world` exists.

```text
LDtk native file
  -> ambition_ldtk validation/conversion
  -> ambition_world topology
  -> ambition_engine collision/world data
```

This avoids duplicating LDtk behavior in sandbox and full game.

### Phase 4: `ambition_bevy`

Create when `ambition_game` starts.

Move shared visible runtime plumbing:

```text
input adapter
Kira adapter
asset loading scaffolding
common app states
sim/presentation event bridge
camera/pause/settings foundations
```

Then:

```text
ambition_actors = lab content + debug UX
ambition_game    = campaign content + release UX
```
