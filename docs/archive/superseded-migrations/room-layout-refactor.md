# Archived: room-layout-refactor.md

Superseded migration or transition note. Preserve as historical evidence; do not treat as current procedure.

Original path: `docs/recipes/room-layout-refactor.md`

---

# Room Layout Refactor Notes

This pass keeps the current hand-authored sandbox rooms, but starts moving the
layout code toward a safer data-driven workflow.

## Loading-zone conventions

- `EdgeExit` zones should live on an actual room boundary and align with a
  visible hole in the wall. They are cyan and automatic.
- `Door` zones may be inside a room, require pressing up, and are drawn in gold.
- Active fixtures such as rebound pads, pogo orbs, and hazards should not overlap
  loading zones.

## New validation hook

`RoomSet::layout_warnings()` performs a lightweight authoring check for active
fixtures that overlap loading zones. It runs at startup and prints warnings
through Bevy logging. This is intentionally non-fatal while the sandbox is still
experimental.

Later, generated room specs should run through a stronger validator before they
are accepted. Useful checks would include:

- loading zones are reachable;
- edge exits touch the correct boundary;
- destination spawns do not overlap solids or immediately re-trigger a loading zone;
- active fixtures do not overlap exits unless explicitly marked as intentional;
- room graph connectivity is valid;
- doors have readable approach platforms;
- blink-passable walls and hard blink blockers are visually distinguishable.

## Enemy fixtures

The current dummies are intentional test fixtures. `spawn_dummies(world)` places
near-spawn sandbags in every room so attack, knockback, particles, audio, and
collision can be checked quickly after each transition. Later rooms should move
from this global fixture rule to explicit enemy spawn specs in the room data.
