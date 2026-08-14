# Simulation authority and determinism

**State:** OPEN successor program.

## Goal

Make simulation ownership explicit enough that deterministic behavior does not
depend on incidental Bevy scheduler topology, mirrored read-model state, giant
systems, or a central runtime census of every domain type.

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
