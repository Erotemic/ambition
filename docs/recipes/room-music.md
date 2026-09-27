---
status: current
last_verified: 2026-09-27
---

# Room music: what plays where, and how to change it

Music is chosen in one place, `simple_track_candidates` in
`crates/ambition_platformer2d_actor_monolith/src/music/intent.rs`. Each frame
it builds a priority list, and the first track id that exists plays.

| priority | source | where you author it |
|---|---|---|
| 1 | the room's **fight** track, only while a fight is on | LDtk level field `fight_music_track` |
| 2 | the fight's own track: a boss by phase, or a wave | `music_intro` / `music_phase1` / `music_phase2` / `music_enrage` in `game/ambition_content/assets/data/boss_encounters/<boss>.ron` |
| 3 | a track a conversation asked for | dialogue (`NarrativeMusicRequest`) |
| 4 | the radio | the in-game radio |
| 5 | the room's own track | LDtk level field `music_track` |
| 6 | the provider's default track | `default_track` in the provider's music registry |

Adaptive cues (the goblin fight) are a separate road: see
[`goblin-encounter.md`](goblin-encounter.md) and
[`generated-music-workflow.md`](generated-music-workflow.md).

## Which to edit

- **"The music in room X should be Y."** Set the room's `music_track`.
- **"The boss fight in room X should play Y."** Set the room's
  `fight_music_track`. It changes every fight in that room and leaves the same
  boss's music alone in every other room.
- **"Boss B should sound like Y wherever it is fought."** Edit the four
  `music_*` fields in B's `boss_encounters/<boss>.ron`.

Do not edit a boss's RON to change one room's fight: another room that hosts the
same boss changes with it.

## Track ids

A track id is a `MusicTrack.id` in
`game/ambition_content/assets/audio/music_registry.ron` (the `id:` column). Use
the id, not the display name.

## Setting a room field

Use the tool, not a hand edit of the `.ldtk` JSON:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools.edit.level_set_field \
    --ldtk game/ambition_content/assets/worlds/sandbox.ldtk \
    --level mode_collapse_arena \
    --set fight_music_track=crooked_ascent_boss \
    --in-place
```

Run it from the repository root. It re-runs the LDtk repair and validation
passes after the edit. The LDtk editor shows both fields under each level's
fields as well.

## Check it

```bash
cargo test -p ambition_content --lib embedded_content_graph_validates
```

An unknown id in `music_track`, `fight_music_track` or a boss's `music_*` field
fails this test with the level or boss and the field named.
