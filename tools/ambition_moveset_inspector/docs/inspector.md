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
- **Engine takes** — recorded playback of the real simulation: the fighter, its
  live hitboxes, its projectiles, and anything its move spawned, frame by frame.
  Recorded by `moveset_takes`; absent until it has been run.

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
