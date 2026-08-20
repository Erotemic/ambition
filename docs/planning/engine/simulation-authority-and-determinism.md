# Simulation authority and determinism

**State:** OPEN successor program.

## Goal

Make simulation ownership explicit enough that deterministic behavior does not
depend on incidental Bevy scheduler topology, mirrored read-model state, giant
systems, or a central runtime census of every domain type.

⚠ **read that last clause precisely — it is about AUTHORITY, not about
discovery.** The bad shape is a low-level generic runtime that owns an
authoritative census of every gameplay domain and must be edited whenever a new
domain participates. A **derived, read-only** index that each domain contributes
descriptors to is a different thing and is actively wanted; see
[`inspection-diagnostics-and-workbench.md`](inspection-diagnostics-and-workbench.md).
⛔ do not cite this goal as an objection to discoverability.

## Current pressure points

The current tree has already exposed four recurring problems:

1. large systems such as actor-brain ticks approach Bevy's parameter ceiling and
   have historically hidden that pressure by tuple packing;
2. some actor facts have existed simultaneously in integrated cluster state and
   ECS components, requiring careful projection/synchronization;
3. adding otherwise unrelated systems has changed ordering among ambiguous
   writers in the past;
4. rollback registration is centralized high in the runtime, forcing generic
   composition code to know exact domain types.

D73 removed several instances of duplicate actor authority. This program starts
from that cleaner state rather than rebuilding the deleted mirrors.

## Target architecture

### Explicit simulation phases

Prefer named phases with clear inputs/outputs, for example:

```text
sample participant / AI intent
    -> resolve body-valid actions
    -> decision / targeting
    -> movement + contacts
    -> combat production
    -> reaction / death / lifecycle
    -> authoritative event publication
    -> presentation/read-model projection
```

The exact set may differ by subsystem. The rule is that dependencies should be
semantic, not an accidental by-product of which systems happen to share a
schedule.

### Named data contracts

Use `QueryData`, coherent `SystemParam`s and domain-owned resources to state what
a phase reads and writes. A parameter object is useful when it names a real
concept; it is not useful merely as a way to fit seventeen unrelated authorities
through Bevy's argument limit.

### Domain-owned rollback declaration

A gameplay domain should describe its rewindable state close to where that state
is defined. Runtime composition may collect/install declarations, but it should
not import every gameplay crate only to enumerate concrete types.

The design must avoid simply pushing `bevy_ggrs` into every leaf crate. Prefer a
small engine-owned registration vocabulary or capability fragment that domains
can implement without depending on the transport/integration backend.

#### DONE — semantics and installation both federate

`RollbackRegistrar` (`ambition_platformer2d_core::snapshot`, depends on
`bevy_ecs` only — no `bevy_app`, no `bevy_ggrs`) lets each domain call a
backend-neutral registration method with its own concrete type; the generic
runtime never names it. It is deliberately not object-safe (`&mut impl
RollbackRegistrar`), so monomorphisation lands at the host's call site rather
than in a central list — a generic API constrains where monomorphisation
happens, not where the list of types lives (the correction to the reopened
conclusion below). The orphan rule is why the implementor is a wrapper: `impl
RollbackRegistrar for App` inside the runtime is foreign-trait-on-foreign-type
(E0117), so `GgrsRollbackRegistrar<'a>(&'a mut App)` implements it instead.

`GatePortalPhases` was the representative customer: its snapshot/projection
moved to `ambition_platformer2d_world::snapshot_impls` (2026-07-30, −2,688
lines from the runtime), with zero new `bevy_ggrs` dependency edges and schema
unchanged (31).

**Migration completed 2026-08-18:** every gameplay crate now owns one
`register_rollback_state` declaration beside its types (canonical/clone
component and resource state, cursor/resolved projections, entity-reference
remapping, rollback anchors, message clearing, derived-state claims, value
probes); the former `runtime/rollback/domains/*` adapter census is deleted.

**Backend ownership extracted 2026-08-19:** the concrete GGRS schedule,
snapshot/history installation, session lifecycle, and checksum/restore probes
now live in `ambition_platformer2d_rollback_ggrs`. The generic
`ambition_platformer2d_runtime` no longer depends on `bevy_ggrs` at all; the
facade's `rollback` feature selects the backend explicitly, and a fixed-step
minimal consumer can omit it entirely. Schema metadata stays in the generic
runtime so prepared-content identity doesn't depend on whether a netcode
backend is linked.

⭐ **The boundary now:** the host may know an installed domain offers rollback
state; it does not know the concrete types or projections inside that offer.
Adding a rewindable type changes the owning domain, not a runtime census.
Extend the backend-neutral vocabulary only when a genuinely new rollback
semantic appears, rather than growing a parallel snapshot abstraction.

---

**Historical note.** An earlier pass argued the census was structural — that
`bevy_ggrs` registration being generic over `T` meant the runtime's source had
to own the list of `T`s. That did not survive review: genericity only
constrains where monomorphisation happens, not who holds the list, and the
migration above completed the resulting redesign. ⛔ **A rejected fix along the
way, worth keeping as a prohibition:** moving the registrations into a
`domains/rooms.rs` inside the same crate is not decentralization — its `mod.rs`
is still a facade holding the same list.

### One authoritative state, explicit projections

Read models are allowed and useful. They must be one-way projections from the
authoritative state. If a projection contains a field that another system also
mutates authoritatively, separate the fields or move the authority rather than
saving/restoring exceptions around a rebuild.

## Phases

### S1 — inventory ambiguous high-authority systems

Re-measure HEAD. Rank systems by:

- independent authoritative domains read/written;
- ordering constraints;
- query breadth;
- parameter count;
- rollback participation;
- number of callers that must understand the system's internal state.

Start with a system where decomposition removes real authority coupling, not the
largest function by LOC.

**Measured 2026-08-14 on `tick_actor_brains`, the ranking's top system.** Its
largest player-centric authority — a combat slot board anchored on
`PrimaryPlayer`, or the lowest `PlayerSlot` when a build had none — turned out to
drive nothing. `assign_slots` filled the board every tick and no production
reader consumed the assignment; the per-actor position it produced had been
discarded since before the monolith split, and the board was rewound as
registered rollback state on top of that. Actor spacing comes from the brain's
crowding signal, which reads positions and a ground/aerial kind and has no
anchor at all.

So the slice was a deletion, not a target-relative rewrite: arbitration that no
consumer observes does not become correct by being re-anchored. Gone with it are
`CombatSlotBoard`/`assign_slots`/`CombatSlotsRes`, the `PrimaryPlayer` query and
the position/`PlayerSlot` reads in `tick_actor_brains`, one rollback
registration (schema v28), and the room-transition and room-reset paths that
cleared the board. `ambition_combat::slots` is now `::crowd`, holding the one
fact the surviving signal needs.

⇒ **the remaining player-centrism in this system is gone**; what remains to
carve is phase structure, below.

**Then the contract for the phase above it already existed.** Phase 2's first
job is separating world observation from decision, and
`ambition_platformer2d_world::collision::CollisionWorld` — "the single collision
read-API" — already owns that composition. Eight systems had adopted it and the
six largest had not: they each carried the room, the moving-platform set and the
overlay as three parameters and wrote out the same
`world_with_sandbox_solids(...)` call. Migrating them deleted eight duplicate
compositions and let `tick_actor_brains` drop its seven-parameter tuple for ten
named ones — `PerceivedWorld` being the concept that three of the seven turned
out to share. **Count the adopters, not the capability.**

**A duplicate authority found on the way out.** `advance_moving_platforms` read
the primary body's `hitstop_timer` to decide whether world geometry may move,
while that same body's hitstop already drives the global clock to zero through
`emit_player_time_intent_system`. Two consequences, and the second is the one
this program is about: no home avatar meant no platform motion at all, and the
clock request lands a frame after the timer is armed, so the platforms froze one
frame before the bodies riding them did — a rider integrating on a nonzero `dt`
across a surface reporting no displacement. Reading the world's own clock fixes
both. ⇒ **when two systems derive the same freeze from one component, the
ordering difference between them is a defect waiting for a witness.**

### S2 — carve decision from mutation

For the selected system, produce a decision/result representation that can be
reasoned about independently, then apply it in a narrower mutation phase. Do not
create a giant `SimulationContext` bag.

### S3 — make ordering explicit

For each authoritative write-after-write or read-after-write dependency, encode
it as a phase edge or data dependency. Re-run deterministic replay/rollback
oracles after each slice.

### S4 — invert rollback registration ownership

Introduce the minimal declaration/composition seam, migrate one representative
domain, then remove the corresponding central runtime knowledge in the same
slice. Expand only after the shape proves useful.

### S5 — simplify projections

Use the cleaner phase model to delete remaining mixed-authority mirrors and
family-specific maintenance lists.

## Acceptance

- adding an unrelated read-only system cannot change authoritative results;
- a new rewindable domain can declare rollback participation without editing a
  central runtime type census;
- the largest actor decision systems have named domain contracts and explicit
  phase boundaries rather than tuple-packed authority bags;
- read-model projection code never preserves hidden authoritative fields around
  reconstruction;
- deterministic/headless tests exercise the same phase graph used by visible
  hosts.
