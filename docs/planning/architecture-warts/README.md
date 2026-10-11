# Local architecture wart index

This page lists small and medium ownership defects that the named architecture
campaigns do not cover. It is not a second queue. It is a re-measurement index.

Before you implement a row, re-check it against the current tree. When the work
is selected, put the executable task in [`queue.md`](../queue.md) and the durable
design in its owner page. Remove the row here when another live page owns it or
when the defect is fixed. The
[AUTHORITY-POLISH](../queue.md#authority-polish--one-owner-per-mechanical-fact-and-no-mirror-in-the-rollback-kernel---done-2026-10-09)
receipt closed the campaign on a fresh census (2026-10-09); an open row here is
fixed when a queue slice reaches it.

The index looks for four failure shapes:

1. authored controls that the runtime does not consume;
2. derived, diagnostic or presentation state promoted to rollback authority;
3. fields or components owned by the wrong subsystem;
4. fallback policy that silently invents capabilities or behaviour.

`STRUCTURAL` means source shows the ownership problem, but the repair can depend
on intended product behaviour.

## Index

| ID | Status | Area | Current defect | Smallest sound direction |
| --- | --- | --- | --- | --- |
| W017 | STRUCTURAL | actor tuning | `ActorTuning` (`crates/ambition_combat/src/actor_tuning.rs`) is one construction record with several owners: reusable body facts, placement/session facts and presentation facts. Its runtime half is already split out: the live brain policy is `ActorPolicy`, and `ActorConfig` is construction input with no runtime writer. | Split by owner when a real consumer boundary exists. Do not add unrelated fields to this record, and do not put a runtime-written field back in it. |
| W018 | OPEN | mount ground | Where a body stands on the giant gnu (`mount.saddle`, `mount.back`, `mount.ledges` of `npc_giant_gnu`) is typed by hand from the drawing, for one pose; the slope of its back is ten steps; `BackLedge` holds two facts. | The sheet's record says `seat` and `ground` in its pixels and the game derives the mount's places: [`engine/the-art-says-where-a-body-stands.md`](../engine/the-art-says-where-a-body-stands.md), steps 1 and 7. |
| W019 | OPEN | mount carry | `mount_backs_are_ground` gives a block `velocity: mount.vel` (per second) where a block's velocity is per frame. Read, not run. | Carry by the displacement the mount did make in the tick, with a test of a mount that moves: same page, step 2. |
| W020 | OPEN | placement | A room has copies of a body's size and of its art's height: the giant's `EnemySpawn` box, the scholar's `BossSpawn` place, the hall's `GIANT_SLOT_HEIGHT_PX`; `specs/gnu_ton_arena_area.ron` is behind its level. | A spawn says where the feet are; tools read the sheet's `reach`: same page, step 3. |
| W021 | OPEN | GNU-ton fight | The choreography's heights are numbers of the room, and the fists' rest is typed from the body's width; the fight was not tuned for the larger giant. | A conductor measures from its mount's places. With the fight's rework: same page, step 4. |
| W022 | OPEN | giant gnu art | The giant is posed by Python (`giant_art.py`), not an SVG with a rig; `_svg.py` is a copy of the reference kit; five pages of 6311 x 5301 are made and two are read; two stale JSON files are in git. | An SVG rig and one sheet per reader: same page, steps 5 and 6. |

## Owned elsewhere

Do not duplicate these here:

- Actor resources (Mana, the Smash Limit) are owned by
  [`engine/composable-actor-resources.md`](../engine/composable-actor-resources.md).
- Rollback-presence and deterministic-ingress defects stay in their owner
  documents and decision rows.

## Re-measurement rule

Search the current source for the named field and its readers before you act.
For an "unused" claim, check direct field reads and adapters that translate the
containing type. For an authoring claim, check shipped content as well as Rust.
For rollback-state cleanup, verify the real restore and checksum contract before
you delete a registration.
