# Open-world runtime and residency — Engine 1.0 program

**State:** OPEN — world-first direction is settled; residency granularity and background simulation policy are not.

## Goal

Support one persistent Ambition world that can be much larger than the set of
rooms currently instantiated in Bevy ECS.

A participant may move through many regions over a long session. Multiplayer may
place participants in different rooms. Important actors/items/world changes may
continue to exist conceptually while their room is not fully simulated or
rendered.

The runtime should distinguish:

```text
world exists
    != room is resident
    != room is fully simulated
    != room is visible in a local view
```


## Foundation dependency

Open-world expansion should build on the current lifetime/reconstitution model
rather than inventing another residency manager first. In particular:

- gameplay-session and rollback-timeline ownership remain distinct (ADR 0027);
- room/session reconstruction should converge through
  [`construction-and-reconstitution.md`](construction-and-reconstitution.md);
- persistent item/actor location is an occurrence/lifetime policy, not evidence
  that every nonresident entity should remain instantiated;
- existence, residency, simulation activity and visibility remain separate axes.

This is a sequencing dependency, not a requirement to solve a universal world
scheduler before adding another room/customer.

## Required capabilities

- stable world/room identity independent of ECS entities;
- explicit residency/loading state for one or many rooms;
- multiple simultaneously resident rooms when participants or mechanics require
  them;
- room unload that does not silently erase persistent instances or world facts;
- deterministic reconstitution of resident simulation from authoritative world
  state;
- cross-room transitions that use the same readiness/commit model in solo and
  multiplayer;
- explicit background-simulation policy for actors/mechanisms that are not in a
  fully resident room;
- resource budgets for resident rooms, views, assets and background work;
- headless queries that can explain which regions are resident and why.

### ⭐ Where three of these stand, measured `baebe16f3` (2026-09-02)

⛔ **Three only. The other capabilities above were NOT measured** and nothing
here should be read as a verdict on them — an unmeasured row and a row measured
as absent look identical once a summary is written, which is the mistake this
note is trying not to make.

- ⛔ **"multiple simultaneously resident rooms" is UNBUILT, and the model is
  singular by type.** `RoomSet` carries `pub active: usize`
  (`crates/ambition_platformer2d_world/src/rooms/room_graph.rs:142`) — ONE index,
  not a set — and no residency-state type exists anywhere in `crates/`. Portals
  do not change this: `ambition_portal2d` is room-scoped state that is cleared on
  room reset, not a second resident room.
- ◐ **"resource budgets for resident rooms, views, assets and background work" —
  ONE of the four is runtime policy and three are TEST RATCHETS**, which is a
  distinction worth keeping: a test budget fails a build, it does not bound a
  running game. Runtime: `NEIGHBOR_PREFETCH_ROOM_BUDGET = 4`
  (`game/ambition_app/src/app/world_flow/room_transition_assets.rs:1271`).
  Test-only: `BOOT_MEGAPIXEL_BUDGET` and `SYSTEM_COUNT_BUDGET`
  (`game/ambition_app/tests/boot_budget.rs:25`, `:28`) and
  `SESSION_STAGED_CAST_BUDGET` (`game/ambition_app/tests/boot_budget.rs:247`).
  ⭐ Assets gained their first eviction rule the same day — see
  [`room-transition-loading.md`](room-transition-loading.md) T2 — so the asset
  axis now has a bound AND a retire. Views and background work have neither.
- ◐ **"headless queries that can explain which regions are resident and why"
  exist for ASSETS, not regions.** `resident_by_road` and
  `resident_never_drawn` (`crates/ambition_asset_manager/src/image_stages.rs:502`)
  answer "which images are resident and by which road", which is the same
  question one axis down. Nothing answers it for rooms, because of the first
  bullet: with one active room the question has a trivial answer and the query
  has never been needed.

⇒ **The three compose into one observation.** Residency is currently a property
of ASSETS, not of ROOMS — the engine can already say what art is resident and
why, and has begun bounding it, while room residency remains a single index
because nothing has yet required two. This program's granularity question is
therefore still genuinely open, and the asset work does not prejudge it.

## Relationship to existing architecture

`ambition_platformer2d_world` already owns backend-neutral room/world records and
room graph concepts. `ambition_load` / room-transition machinery already models
readiness and commit. The multiview program already requires participants to be
able to occupy different rooms.

Do not create a second "open world" representation beside those. Extend or
factor the existing ownership according to measured dependencies.

## Candidate crate / Bevy shape

A reusable residency scheduler is a plausible standalone Bevy plugin only if it
can operate over game-provided world records without importing Ambition content.
The likely shape is a small domain plugin that owns residency state/messages and
lets a game/world provider supply preparation and instantiation adapters.

Whether this belongs in `ambition_platformer2d_world`, `ambition_load`, a new
world-runtime crate, or a split between them is **not decided**. Use
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md)
before carving.

## Acceptance pressure

- Ambition: travel across a large connected world, leave persistent state behind,
  save/reload and return coherently.
- Ambition multiplayer: two participants can occupy different rooms without
  duplicating the world or forcing both rooms into one camera contract.
- TwinTrack: independent views may stress different resident presentation state.

## Open design questions — deliberately unresolved

- Is the unit of residency a room, region, chunk, or a hierarchy of these?
- How much background simulation should occur for dormant persistent actors?
- Should dormant simulation advance continuously, event-wise, or only when a
  relevant world fact changes?
- How are time-dependent mechanisms reconciled when a room becomes resident
  again?
- Which room-to-room interactions are legal without both rooms resident?
- What is the memory/CPU budget policy for N local/remote participants spread
  across the world?
- How do save checkpoints interact with a room currently preparing/loading?

Do not hide these questions behind an implicit "current room" singleton.
