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

### ⭐ Where FOUR of these stand — three measured `baebe16f3` (2026-09-02), the fourth `ba111c995` (2026-09-04)

⛔ **Four only. The other capabilities above were NOT measured** and nothing
here should be read as a verdict on them — an unmeasured row and a row measured
as absent look identical once a summary is written, which is the mistake this
note is trying not to make.

- ⛔ **"multiple simultaneously resident rooms" is UNBUILT, and the model is
  singular by type.** `RoomSet` carries `pub active: usize`
  (`crates/ambition_platformer2d_world/src/rooms/room_graph.rs:142`) — ONE index,
  not a set — and no ROOM-residency state type exists anywhere in `crates/`.
  Portals do not change this: `ambition_portal2d` is room-scoped state that is
  cleared on room reset, not a second resident room.
  ⚠ **Re-checked 2026-09-03 and narrowed from "no residency-state type" to "no
  ROOM-residency state type", because the looser wording is now falsifiable by a
  grep that proves nothing.** `crates/ambition_sprite_sheet/src/fx.rs` gained
  `FxResidency { Core, OwnedByCharacter }` (`06a494f4e`), which is ASSET
  residency — whether a sheet decodes at boot or with the character that names
  it — a different axis entirely. `RoomResidencyOwners` likewise exists, in
  `game/`, and is the room-commit retire set rather than a second resident room.
  ⇒ The argument here is untouched: `RoomSet.active` is still a `usize`, checked
  at the line cited above. Only the sentence needed narrowing, so a reader who
  greps `Residency` does not read a hit as a refutation.
- ◐ **"resource budgets for resident rooms, views, assets and background work" —
  ONE of the four is runtime policy and three are TEST RATCHETS**, which is a
  distinction worth keeping: a test budget fails a build, it does not bound a
  running game. Runtime: `NEIGHBOR_PREFETCH_ROOM_BUDGET = 4`
  (`game/ambition_app/src/app/world_flow/room_transition_assets.rs:1367`;
  ⚠ this said `:1271` until 2026-09-05, where the file now has `))` — the VALUE
  was and is 4, only the line drifted, and the citation checker validates the
  FILE so a drifted line passes it forever).
  ⭐ **AND THE AMBIGUITY CHECK CATCHES DRIFT BY ACCIDENT, observed 2026-09-05.**
  A citation on another page named a bare `integration` file at line 773 and was
  RED only because the suffix matched two tracked files. Disambiguating it meant
  opening `crates/ambition_platformer2d_core/src/movement/integration.rs:798` —
  where line 773 turned out to be a comment about jump height, the other
  candidate file is only 768 lines long, and the real line was 798.
  ⭐⭐ **AND THIS PARAGRAPH TRIPPED THE SAME GATE**, because a sentence about an
  ambiguous citation naturally contains one. The fighter session's fix hit it
  too, one line apart. ⇒ The checker is TEXTUAL, not semantic: it cannot tell a
  citation from prose quoting a citation, which is the price of a gate cheap
  enough to always run — and worth paying, since quoting a path is exactly when
  you should be spelling it in full anyway. ⇒ **The drift was
  invisible to every gate; the ambiguity is what got a human to re-read a line
  number at all.** That is not an argument for a line-drift gate (both its
  predicates were measured this same day and neither is gateable) — it is a
  reason to fix an ambiguous citation by RE-DERIVING the line rather than by
  pasting a longer path.
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

- ✔ **"room unload that does not silently erase persistent instances or world
  facts" is SATISFIED, and the mechanism is a three-way classification plus one
  ledger** — a positive verdict, which is worth writing down because an
  unmeasured row and a satisfied row also look identical in a summary. The room
  transition sweeps `RoomResident`
  (`crates/ambition_platformer2d_shared_tangle/src/lifecycle/markers.rs:23`,
  `With<RoomScopedEntity>, Without<InCustodyOf>`), and every occurrence it
  destroys is covered by exactly one of three answers:
  - **carried** — `InCustodyOf` exempts it from the sweep outright, and it rides
    across with whoever holds it;
  - **remembered** — an `AuthoredOccurrences` row survives the unload
    (`Placed { room, at }` freezes at the boundary) and `outlook_for` rebuilds it
    where it lies while every other room suppresses it;
  - **as authored** — no row *means* "as authored", so the room's own spec
    re-authors it. Verified by the first arm of
    `a_room_reinstates_an_occurrence_whose_record_lives_next_door`
    (`world/rooms/stage.rs`): an untouched record is built at its own
    coordinates.

  ⭐ **And the class that IS destroyed is `SpawnOrigin::Dynamic`, which is not a
  persistent instance by definition** — the enum's own words, *"the running
  simulation minted this: a projectile, a summoned minion, a dropped item"*
  (`crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs:76`). Nothing authored it and no hand carried it, so
  there is no durable record to erase. The three-way `SpawnOrigin`
  classification and the ledger's entry rule are the same distinction seen from
  two sides, which is why this comes out clean rather than lucky.
  ⚠ **The erasure is no longer SILENT as of `ba111c995`**, and that is the word
  the capability turns on: `republish_placements` refuses an id it never held
  and returns the refusal `#[must_use]`, so a future producer that tries to make
  a dynamic drop durable is told rather than ignored. Whether one ever should is
  question 51, not a defect here.
  ⚠ Reasoned from the sweep filter and the enum rather than run for the dynamic
  arm specifically; the carried and as-authored arms have tests named above.

⇒ **The four compose into one observation.** Residency is currently a property
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

⭐⭐ **TWO OF THESE HAVE A DE-FACTO ANSWER IN THE CODE ALREADY — measured
2026-09-05 — and reading the list without that makes the design space look four
times wider than it is.**

**The unit of residency today is the ROOM, and there are TWO TIERS, not one:**

| tier | what exists | authority |
|---|---|---|
| RESIDENT | entities, simulated | `RoomScopedEntity` — *"despawn when the current authored room unloads"* (`platformer2d_shared_tangle/src/lifecycle/markers.rs:7`) |
| PREPARED | a `RoomConstructionPlan`, DATA only | `room_transition/prefetch.rs` — keyed by content epoch, session scope, and the room you are standing in |

⇒ **A prepared neighbour is not a resident room.** It is a cached construction
artifact that the transition promotes *"only if every identity term still
matches, so a hot reload, a provider swap, or a session change is a safe MISS
rather than a stale promotion"*. No entity exists for it and nothing about it
ticks.

⇒ **So "room, region, chunk, or hierarchy" is not open four ways.** One room is
resident; a two-level structure already exists; and the live question is whether
the PREPARED tier should ever become *simulated* rather than remain data — which
is a much smaller question than the bullet implies.

**And background simulation for dormant actors is ZERO today.** ⚠ Checked
against the thing that looks like a counter-example: `features/ecs/dormancy.rs`
is *"optional distance-based brain dormancy"* — it sleeps the BRAIN of an actor
inside the resident room while *"body physics continues"*. That is an intra-room
cost optimisation, not simulation of a non-resident room. Nothing outside the
current room ticks at all.
ⓘ Both statements are MEASURED. What each *should* be remains a design question,
and the bullets below are unchanged — they are now asked against a known
baseline instead of a blank page.

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
