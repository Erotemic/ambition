# Moveset balance inspector

Frame data, hitbox geometry and cross-roster comparison for Ambition's smash
cast, without loading the game.

## Launch

```bash
tools/ambition_moveset_inspector/serve_inspector.sh --open
```

The wrapper re-exports the bundle (`cargo run -p ambition_app_tools --bin
moveset_export`) and then serves the UI at <http://127.0.0.1:8777>. Pass
`--no-export` to look at the bundle already on disk.

## What it reads

`moveset_export` boots `build_visible_app` and reports what the **composed
host** resolves, not what an authoring file writes. Those differ whenever a
repertoire slot overwrites a move (`UpSpecial::into_spec` sets `gates.recovery`)
or a provider composes one fighter out of another's table — and the composed
answer is the only one a balance decision can be made against.

The bundle is generated, so it is gitignored. `check_bundle_contract.mjs`
asserts every field the UI reads is present; run it after changing either side.

## The four views

- **Roster** — every fighter with a real moveset, filterable, grid-only by default.
- **Fighter** — the moveset as a sortable frame-data table, plus per-move
  timeline, hitbox diagram and the body's own numbers.
- **Compare** — one slot across the whole roster. Cells more than two median
  absolute deviations from the roster median for that slot are flagged high or
  low. This is the view that answers *"is this move out of line"*.
- **Engine takes** — recorded playback of the real simulation **in the real art**:
  the fighter's own sprite, animated by the move that is playing, with its live
  hitboxes over the top and anything its move spawned drawn beside it. Recorded
  by `moveset_takes`; absent until it has been run. The `Art` button turns the
  sprites off when a volume is easier to read on its own.

## How the art gets there

`moveset_export` emits an atlas table (`sheets`) beside the fighters: frame size,
page images, the body rectangle, the feet pixel, and every row's per-frame
sub-rects with trim offsets. The server exposes the engine's own sprite directory
read-only at `/art/`, so nothing is copied and the page always shows what the
build would draw.

A take records `[sheet_key, row_index]` per body per tick. The FRAME within the
row is derived in the viewer by counting consecutive ticks on that row — one
clock (the sim tick the take was recorded at) instead of two that can drift.

⛔ Two traps are already paid for here, and both cost a rebuild:

- **`CharacterAnimator` is the wrong source.** It is what the renderer uses, and
  the render layer only inserts it once a sprite ASSET has loaded — which never
  happens under `NoWindow`. Asking for it recorded 14446 bodies with art on
  exactly zero of them.
- **A summon wears no catalog character.** Joining the sheet on `WornCharacter`
  alone drew the pirate in full art and his shark as an empty box, which is the
  one pairing this view exists to show. `ActorConfig::sprite_character_id` is the
  fallback the renderer itself uses.

The viewer places a frame on the body's own centre (`feet_pixel.x`), not the
frame's. A sheet cell is sized by the widest pose and the art sits wherever the
crop left it — `projectile_polygon` is 17% of a 377px frame left of centre — so
centring the cell would reproduce, in the viewer, the exact defect the engine's
own sprite anchor had until 2026-08-27.

```bash
cargo run -p ambition_app_tools --bin moveset_takes -- --characters npc_pirate_admiral
```

It drives the nineteen repertoire presses through the real control frame into a
real seated match — a **fresh match per take**, because `afford_recovery` refuses
a recovery whose airtime already spent one and an instrument whose answer depends
on what ran before it is not measuring the thing it names.

Read the report line. `MISMATCH: drove <id> but the engine played {...}` means
the press did not reach the move it is bound to, and that is a finding rather
than noise: it is how D252 (the back air is unreachable for the whole cast) and
D253 (`player_robot_v3` cannot throw any of its five specials) were both found.
The per-frame `gesture` field says what the engine understood the input to be,
which is usually the reason.

## Feedback

Every fighter and every move takes a 1–10 score, issue tags and free notes.
Reviews are durable authoring data under `reviews/<character>/<move>.yaml`, keyed
by the stable id rather than by the numbers — the point is that *"the pirate's
forward smash is too strong"* survives the next tuning pass. The numbers as they
stood when the note was written are recorded beside it as `snapshot`.

For an agent asked to *address the feedback on the pointed polygon*:

```bash
python3 -m ambition_moveset_inspector.server --report
```

prints every standing note that asks for a change (scored below 6, or tagged).
