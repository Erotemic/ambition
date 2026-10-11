# Falling sand — removed; not a work item

**Status (2026-10-11): the code is deleted.** Jon's ruling: *"We can remove
falling sand ... at this point it would need a reimplementation anyway to be
anything like what I originally wanted with it. It might be better to just not
have it as a distraction."*

⛔ **Do not work on falling sand, fix its tests or restore its code unless Jon
asks.** It is not a queue row and not a correctness customer. A
reimplementation is a new design, not a revert.

## What was deleted

- `ambition_content::falling_sand` (the `bevy_falling_sand` 0.8 bridge for
  water and oil, the presentation and the room plugin) and
  `ambition_content::falling_sand_sim` (the bespoke deterministic sand grid,
  the settled-sand ledger, the spout and room state, and the room's lent
  `swim`).
- The `falling_sand` feature in `ambition_content` and `ambition_app`, and the
  `bevy_falling_sand` dependency.
- `app_it::falling_sand_room`, the two-live-rooms sand test, the room's
  rollback-coverage sweep and the guard-script rows that adjudicated its
  systems and resources.

## What stays, dormant

- The LDtk level `falling_sand_room` in `worlds/sandbox.ldtk`. It is an
  ordinary room now: its switch, loading zones and camera zone still work,
  and nothing pours.
- `FeatureEcsWorldOverlay::water_regions`, the overlay slot for additive
  liquid. It has no contributor today. It is engine vocabulary, so it stays
  (Q74).

## What a reimplementation must meet

These are the lessons the deleted version paid for. Read them before any new
design.

- **Rollback first.** The sand grid and ledger were not rollback snapshots.
  Gating their step to authoritative passes stopped double stepping, but a
  rewind could not reconstruct historical sand. A new material model must be
  rollback-owned state from the start.
- **No external frame-driven solver.** `bevy_falling_sand` runs private,
  parallel, RNG-driven, query-ordered `PostUpdate` systems. It cannot be
  driven one step per simulation tick without a fork. Jon: *"We will have to
  rewrite bevy_falling_sand if we want netcode level determinism AND falling
  sand."*
- **One representation.** Matter exists in one place. Settled matter becomes
  geometry only through the overlay, and the transfer is atomic.
- **Conservation and settling.** Total matter equals emitted minus drained,
  every tick, and a finite pour reaches a fixed point.
- **A spout is an authored placement**, lowered through the placement seam,
  not a hard-coded table.

The parked design that used it: the character **Oiler** sprays oil that coats
surfaces (a movement modifier) and can be ignited, and flame spreads across
connected coated cells. See also `docs/storylines/cannon-elevated.md`.
