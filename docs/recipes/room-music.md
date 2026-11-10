---
status: current
last_verified: 2026-11-10
---

# Room music: selecting and editing tracks

Ambition's music intent is built by `compute_music_intent` in
`crates/ambition_platformer2d_audio/src/music/intent.rs`. That system selects
which live room is heard and publishes a provider-scoped `MusicIntent`.
`ambition_audio` resolves the chosen track or adaptive cue and plays it. A
second live room must not change the music of the room currently heard.

## Simple-track priority

`simple_track_candidates` in the same module builds this ordered list:

| Priority | Candidate | Where to edit |
| --- | --- | --- |
| 1 | Player-selected radio station | Radio selection |
| 2 | Room's fight override, when an encounter requests music | LDtk level `fight_music_track` |
| 3 | Encounter's own track | Encounter or boss phase music content |
| 4 | Conversation's requested track | Dialogue music command |
| 5 | Room or time-of-day track | LDtk level `music_track`; `audio/music_cues.ron` room-phase mappings |
| 6 | Active provider's default | Its generated music registry |

The audio director uses the first available permitted track. A radio selection
also suppresses adaptive cues. Adaptive encounter, boss, and room cues use
`MusicIntent::adaptive` and are resolved separately; they are not additional
rows in the simple-track list. A conversation request takes precedence over a
room's adaptive cue. For exact current selection rules, inspect
`compute_music_intent` and the `ambition_audio` director.

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
