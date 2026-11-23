---
status: current
last_verified: 2026-11-23
---

# Room music: selecting and editing tracks

Ambition's music intent is built by `compute_music_intent` in
`crates/ambition_platformer2d_audio/src/music/intent.rs`. That system selects
which live room is heard and publishes a provider-scoped `MusicIntent`.
`ambition_audio` resolves the chosen track or adaptive cue and plays it. A
second live room must not change the music of the room currently heard.

## Music priority

Tracks and adaptive programs use one ordered candidate list:

| Priority | Candidate | Where to edit |
| --- | --- | --- |
| 1 | Selected radio station | Radio selection |
| 2 | Focused fight | Encounter or boss phase music content |
| 3 | Other fight | Encounter or boss phase music content |
| 4 | Conversation's requested track | Dialogue music command |
| 5 | Room or time-of-day track | LDtk level `music_track`; `audio/music_cues.ron` room-phase mappings |
| 6 | Active provider's default | Its generated music registry |

Apply a room's fight override to that fight before selection. The winning
goal keeps its priority while media prepares. For equal priorities across
local listening rooms, the primary participant's room wins. Source recency
orders equal candidates within one room. Track and adaptive representations
use the same rule.

## Which source to edit

- To change one room's normal track, edit its LDtk `music_track` field.
- To override music while an encounter is active in that room, edit the room's
  `fight_music_track`.
- To change a boss's music across rooms, edit that boss's encounter fields in
  `game/ambition_content/assets/data/boss_encounters/`.
- To change room or encounter adaptive cues, edit the provider's
  `audio/music_cues.ron`, then validate that pack.
- To add or rename a track, edit the authored score/selection policy and run the
  music publication tools. The music registry is generated; do not hand-edit it.

`game/ambition_content/assets/pack.ron` declares `audio/music_registry.ron`
from the `ambition_shared` library. It is **not** an ordinary file under
`game/ambition_content/assets/audio/`. The generator is
`scripts/regen/regen_music_registry.py`; the active library manifest in
`game/ambition_content_library/library.json` controls the generated location.
Use a track's `id`, not its display name.

## Cue media

The `music_cue_catalog` schema is version 2. Each cue declares `media` as
`Inline(...)` or `Published(...)`. Inline media gives a declared musical grid
and body length; it does not establish the encoded file end. Published media
references a renderer report in the selected library and selects
`MasteredFull` or `GroupedLayers`. It uses the report's clock, loop flags,
and product paths. States, continuations, layer slots, and intensity gains
remain game policy. A program cannot combine a mastered mix with its stems.

## Edit a level field

Use LDtk tooling instead of editing its JSON by hand. For example:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools.edit.level_set_field \
    --ldtk game/ambition_content/assets/worlds/sandbox.ldtk \
    --level mode_collapse_arena \
    --set fight_music_track=crooked_ascent_boss \
    --in-place
```

The LDtk map-assets submodule must be initialized before editing that world.
The command changes authored world content. Confirm the semantic diff, and
validate the pack or use the focused content check:

```bash
cargo test -p ambition_content --lib embedded_content_graph_validates
```

See [`ldtk-authoring.md`](ldtk-authoring.md) and
[`generated-music-workflow.md`](generated-music-workflow.md).
