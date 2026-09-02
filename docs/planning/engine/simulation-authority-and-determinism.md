# Simulation authority and determinism

**State:** OPEN — rollback registration/backend ownership is largely settled;
remaining work is runtime authoritative-state correctness, deterministic
composition, lifetime boundaries, and explicit phase ownership.

> **Guard pointer, added ce25540b1 (2026-09-02).** This program has a mechanical
> guard that no planning file previously named:
> `scripts/check_rollback_mutators_run_in_sim.py` — *"does anything mutate
> ROLLBACK state from a schedule that never rewinds?"* A component registered
> for rollback is restored on every rewind, so a system mutating it must run in
> the schedule GGRS resimulates (`app.sim_schedule()`), not a literal `Update`.
> ⛔ Its own docstring names why the bug is invisible locally: **under a
> fixed-tick host the two are the same schedule**, so the mistake costs nothing
> and shows nothing — it only diverges under GGRS, where the value rewinds and
> the mutation does not replay with it. Green at `ce25540b1`: 4 systems mutate
> rollback state, none registered into a non-rewinding schedule.

## Goal

A simulation result should be determined by explicit authoritative data and
semantic phase/composition rules, not by:

- whether a component happened to be registered but its entity was not on the
  rollback timeline;
- Bevy query/entity iteration order;
- a `Local<T>` or process-global value remembering a future state across rewind;
- ambiguous gameplay-session ownership;
- multiple mutable representations of one fact;
- scheduler topology among otherwise unordered writers;
- tuple/SystemParam packing that hides one system owning unrelated domains.

## Current model

An authoritative dynamic object can require five independent guarantees.

### 1. Rewind codec

What mutable component/resource state is saved, restored, remapped and included
in deterministic checksum policy?

### 2. Rollback participation

Does the authoritative entity itself participate in GGRS entity
creation/destruction history? A registered component on an entity without the
appropriate authoritative-family `Rollback` anchor is not enough.

### 3. Stable semantic identity

Which logical object is this across construction/reconstruction and when a rule
needs to compare peers? `SimId` is Ambition's semantic identity;
`bevy_ggrs::RollbackId` remains internal frame-history identity.

A stable identity is not required merely because an entity exists. It is required
when behavior, relationships, reconstruction, diagnostics or deterministic
selection depend on that logical identity.

### 4. Deterministic selection and composition

If multiple valid entities can affect one result, define the rule. Sorting by a
stable key is appropriate for deterministic selection, but not every operation
is order-independent. Where effects do not commute, define precedence or a
canonical state-first/identity-last composition rather than laundering ECS query
order into gameplay semantics.

### 5. Lifetime ownership

Which gameplay session and rollback timeline may treat the state as authority?

Current rollback lifetime is:

```text
process
  -> gameplay session           SessionScopeId
       -> rollback timeline     RollbackTimelineGeneration
```

`ActiveRollbackAuthority` owns the current rollback contract/status together.
Health carries across timeline generations only when the gameplay-session owner
is the same. A foreign session reads `Unavailable`, never the other session's
health. Historical diagnostics are process-lifetime evidence and gate no
simulation/lifecycle work.

ADR 0027 is the durable authority for this rule.

## Landed architecture that should not be reopened

### Domain-owned rollback declarations

Every gameplay domain owns its `register_rollback_state` declaration beside the
types it owns. The generic runtime collects backend-neutral declarations without
naming every concrete gameplay component.

The concrete GGRS schedule/session/backend lives in
`ambition_platformer2d_rollback_ggrs`; the generic runtime has no `bevy_ggrs`
dependency. Do not reconstruct the old runtime `rollback/domains/*` census.

### Explicit schedule phases

`GgrsSchedule` uses explicit Ambition phase sets and the single-threaded
executor. Current measurement found parallel dispatch expensive for the present
many-small-system workload; parallelization is not a determinism or performance
objective without new evidence.

### Controlled-actor observation/decision decomposition

Large actor decision code has already shed several unrelated authorities:
observation, target selection and pre-decision maintenance have clearer owners;
old primary-player combat-slot arbitration was deleted after proving it had no
production consumer.

Continue phase decomposition only when it removes real authority coupling.

### Gameplay-session rollback ownership

`26ec7b19` fixed the demonstrated Smash -> title -> Ambition contamination by
making rollback confirmation session-owned, resolving contracts against their
own session root, and re-establishing session-mirrored resources at activation.

Do not replace this with ad hoc "clear health on quit" calls. Same-session poison
must survive a timeline rebase; different gameplay sessions must not inherit it.

## Current work

### S1 — scenario-populated rollback coverage — the timeline half landed 2026-09-02

Boot-time/static registration checks cannot prove runtime-created authoritative
families. Representative scenarios should create the populations real gameplay
creates, then assert the required combination of:

- rollback participation;
- registered mutable state/remapping;
- stable semantic identity where needed;
- deterministic composition/selection;
- correct gameplay-session ownership.

Prefer domain-specific authoritative constructors/request types that make the
required invariants hard to omit. Do **not** introduce a universal
`spawn_sim_entity` wrapper merely so a source scanner can ban raw `spawn`.

Where it stands (re-verified 2026-09-02):

- **participation + registration, populated**: `rollback_coverage.rs` builds a
  live match, a boss arena, a strike volume, a mounted pair, the falling-sand
  room and the event-created set (sentry, vortex well, temporary gravity zone,
  falling hazard, portal shot) and sweeps each for unaccounted components and
  INERT registrations (a registered type on an entity with no anchor).
- ✔ **rewind stability while those families STEP**:
  `rollback_populated_timeline.rs` makes that same event-created population
  (plus a held-item bolt created by play) a fresh SyncTest baseline and
  resimulates 150 frames at check distance 7 — 852 replay comparisons — under
  TWO oracles. ⛔⛔ The session checksum alone was not one: 47 registrations are
  probed-only ("not in the session checksum"), and a sentry stepping from a
  process-global counter stayed GREEN under `rollback_health` for all 150
  frames. `RollbackRestoreAudit` is the oracle that sees them; the same poison
  fails it at frame 2 naming `Sentry`. A poison periodic in the check window
  (`n % 7` at distance 7) cancels and proves nothing — measured.
- ✔ **semantic identity across a rewind** is covered by the same timeline, and
  measured rather than inferred: `SimId` is canonical AND checksummed, the
  fixture fires a bolt every nine frames inside seven-frame check windows, so
  every mint is replayed — and minting from a process-global counter instead
  of the spawner's `SimIdCounter` (poison in `sim_identity.rs`) desyncs the
  session at frame 2. **Session ownership** is covered by
  `rollback_lifecycle_reset.rs` / `session_ownership_tests.rs`;
  **selection/composition** is S3.

### S2 — remove authoritative non-rewinding memory — ✔ CLOSED 2026-09-02

Every authoritative case this row named is now registered state, and the
remaining `Local<T>`s on sim-schedule systems were censused and classified:

```text
cutscene trigger room      ambition_cutscene::LastCutsceneRoom        registered (cutscene.last_room)
Mary-O active-room follower LevelDeparture.seen_room (component)     registered; rollback_room_memory.rs
quest room-entry edge      ambition_persistence::quest::LastQuestRoom registered+checksummed (2d73d9d94, schema v149)
```

Census of `Local<T>` on systems in the sim schedule after that (grep, sim-side
crates, tests/dev/census excluded): `tick_npc_idle_barks`'s bark timers —
presentation cadence that writes only `VfxMessage` (cleared on rollback), so a
resim re-emits bubbles and mutates nothing authoritative; `contact_scratch` and
`empty_relations` — per-run scratch cleared before use; `gated_lock_walls`'s
cached query state; everything else is host/presentation (`time_control`,
audio, menu input, the runtime's prefetch memory, the causal recorder's epoch).
None remembers an authoritative future.

⚠ Known limit of the quest fix's PIN: a confirmed room transition rebases GGRS
onto a new frame-zero baseline, so today no rewind crosses the room flip and the
old `Local`'s divergence was unreachable (the mary-o file records the same for
its follower). The guard is therefore structural — the resource restores the
producer's behaviour (`restoring_the_last_room_makes_the_producer_announce_the_
room_again`, red with the `Local` back) — not a desync reproduction. The move is
still right: a correctness only the rebase provides moves when the rebase does.

The original row, for the record:

Current review evidence still identifies gameplay logic whose `Local<T>`/edge
memory can remember a future state after rollback. Examples include the Mary-O
active-room follower and quest/room-visit edge detection.

For each case, decide whether the memory is:

- authoritative history that must rewind;
- derivable from rewound state and should be recomputed;
- presentation/diagnostic memory that must not mutate authoritative state during
  historical resimulation.

Move authoritative memory into registered state or eliminate it. Do not add it
to checksums while leaving the actual restore semantics unchanged.

### S3 — close remaining deterministic selection/composition sites — ✔ the three named sites, 2026-09-02

Current known sites include projectile-victim ties, possession candidates and
pickup-magnet ownership. Use the existing deterministic-selection vocabulary
where the operation is a true selection.

For composition problems, first state whether the operation is commutative. A
stable sort is not a semantic answer when reversing the same valid influences
changes the result.

Re-verified 2026-09-02:

- possession candidates (`abilities/traversal/possession.rs:226`) and the pickup
  magnet / collector (`features/ecs/pickups.rs`, `items/world_item.rs`) go
  through `sim_selection::winner_by`, whose final key is `SimId`;
- ✔ projectile victims: `step_projectiles` ordered its first-wins loop by
  distance-along-the-leg then the victim's position — and two bodies on one
  spawn point tie on all of it, so a stable sort handed the decision back to
  query order. `StrikeVictim` now carries the victim's `SimId` and the sort ends
  on it. Guard: `two_stacked_victims_are_struck_in_identity_order_whatever_the_
  archetype_order` (spawns `[a, b]` and `[b, a]`; red without the key). The
  boss/breakable arms are `any()` predicates, order-free by construction.
- censused the same day: every `.iter().find/min_by/max_by` in the sim-side
  crates (11 sites) is a lookup by unique key (`enc.id == target`, `entity ==
  body`, the portal partner) or a walk over an AUTHORED `Vec` (a boss's
  damageable parts, `world.blocks`) — none selects among peers by query order.
  The melee victim loop hits every overlapping body and has no first-wins to
  order. `for … in query { … break }` shapes outside these were not swept; the
  grep for them is noisy and the next one found should be added here.

### S4 — dynamic identity and provenance — the census half landed 2026-09-02

Runtime-spawned authoritative entities must use the runtime identity/provenance
road rather than borrowing authored-placement identity. Required component
relationships should make paired facts difficult to forget where the type system
can express the invariant.

Do not attempt to enforce semantic authored-vs-runtime provenance with a grep of
all `SimId::placement` call sites. A future typed `PlacementId` seam may be
appropriate when the refactor is justified by a concrete failure/customer.

✔ **Every rollback-anchored entity on the populated timeline carries one
unique `SimId`** — `every_rollback_anchored_entity_has_a_unique_sim_id_on_the_
populated_timeline` (`rollback_populated_timeline.rs`), with no waiver list.
The first run found four anonymous anchors and one collision, all repaired at
their seams: the grenade's gravity well and the encounter script's falling
hazard now mint `SimId::spawned(spawner, counter)` under the grenade / the
encounter (`open_temporary_gravity_well` and `drop_hazard` take the id); a
placed portal and the gameplay session's world root carry the new DERIVED
`SimId::singleton(kind, key)` (`portal:blue`, `session:<activation>`) — at
most one per key, so re-placing is the same object. The collision was the
fixture's: hand-minted `slot:0/0` met the subject's first bolt, which is why
the fixture now draws from the subject's own `SimIdCounter` like every
production spawner. Poison: drop the portal's id and the census names it.
The semantic half — a resimulation re-mints the SAME ids — is proven by the
S1 timeline (see S1: a process-global mint counter desyncs it at frame 2).

### S5 — phase and ownership decomposition

Continue breaking high-authority systems where a split produces a real semantic
phase or domain owner. Useful criteria are:

- independent authoritative domains read/written;
- ordering constraints;
- query breadth;
- rollback participation;
- mutation during what should be proposal/decision;
- duplicated derivations of the same authority.

See [`../../architecture/bevy-system-boundaries.md`](../../architecture/bevy-system-boundaries.md) for the durable ECS-boundary rule.

A cohesive `SystemParam` or `QueryData` is good when it names one concept. It is
not a fix when it only hides the parameter ceiling.

### S6 — session-scoped process-resource residue

The current engine still has process resources that mirror one live gameplay
session. `SessionScopedResources` now re-establishes the known set on activation,
so skipped/misordered retirement is not the correctness boundary.

Treat additions to this set as migration pressure: decide whether the fact is
truly process-global, should remain a session mirror with activation semantics,
or belongs structurally under session-owned state. Do not mandate one storage
shape without a concrete ownership case.

The remaining `LocalSessionPolicy::check_distance` F9-proof-pulse question is a
maintainer/productivity decision, not evidence that all such resources need an
entity-based rewrite.

Addition 2026-09-02: the two room-entry edge memories S2 moved out of `Local`s
(`LastQuestRoom`, `LastCutsceneRoom`) are in the set now. Each remembers "the
room I last announced" and fires only on a change, so inherited across sessions
a new game starting in the room the previous session ended in — quit at the
start, start over — skipped its first room's quest events and cutscene
trigger. The `Local`s had the same defect; S2 made it registered state and S6
makes it session-scoped. Guard: `retirement_clears_every_session_scoped_mirror`.

## State projection rule

Read models are allowed. They must be one-way projections from authoritative
state. If a projected component contains fields that another system mutates as
authority, split the representation or move the authority; do not preserve
exceptions by saving/restoring selected fields around a projection rebuild.

## Test topology

Use the host needed by the invariant:

| Property | Required proof shape |
|---|---|
| deterministic simulation | headless real `GgrsSchedule` / `SyncTestSession` |
| runtime-created rollback population | scenario that actually creates it before rewind |
| cross-game/session isolation | shell/app host that creates, retires and creates real sessions |
| physical input/rebinding lifecycle | real input/session host |
| rendered materialization/raster behavior | rendered hardware measurement |
| durable persistence | fresh-process reconstruction |
| capability combinations | explicit supported feature/product matrix |

A simplified host is useful only if it still contains the composition property
being asserted.

## Acceptance

- adding a rewindable domain type changes its domain declaration, not a central
  concrete-type census;
- representative dynamic authoritative families survive rewind/recreation with
  correct identity and deterministic behavior;
- no known authoritative edge/history uses non-rewinding `Local<T>` memory;
- peer selection/composition does not depend on raw ECS iteration order;
- one gameplay session cannot consume another session's rollback health or live
  session mirror state;
- same-session rollback rebases cannot clear a real desync by accident;
- major simulation systems have named authority/phase contracts rather than
  parameter packing disguising breadth.

## Explicit non-goals for the current program

- another custom rollback/snapshot engine;
- pushing `bevy_ggrs` into every leaf domain;
- a universal raw-spawn wrapper;
- exhaustive pairwise schedule edges duplicating semantic phase structure;
- scheduler parallelism as an architecture objective;
- source-text policy where a type/API/runtime test can make the invariant
  structural.
