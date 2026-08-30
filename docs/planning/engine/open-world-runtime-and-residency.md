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
[`bevy-plugin-and-crate-strategy.md`](bevy-plugin-and-crate-strategy.md)
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
