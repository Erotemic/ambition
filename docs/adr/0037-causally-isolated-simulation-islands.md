# ADR 0037: Investigate causally isolated simulation islands

## Status

**Proposed research boundary (2026-10-26), not an implemented architecture.** Region-scoped
rollback is a candidate, not a commitment to automatic per-room GGRS sessions.

## Context

Ambition permits several live rooms and players exploring independently, but its current GGRS
host saves and resimulates one App-wide timeline. Ordinary peer room commits can hold the whole
simulation. Full-world rollback can scale with unrelated live state. A room is a useful spatial
unit but may not be a valid simulation isolation unit.

## Proposed decision

- A **simulation island** is a set of authoritative mutable facts closed under immediate causal
  interactions; it may include one room, part of a room, or several connected rooms.
- Independent histories are permitted only when a rollback or resimulation in one island cannot
  alter another island's already-committed facts.
- Cross-island writes, transfers of bodies and carried objects, persistent rewards, quest events
  and ownership changes occur through **identified, ordered, idempotent committed
  transactions**, with one canonical owner before and after transfer.
- Immediate cross-boundary physical interactions (e.g. pushing a body or attacking through a
  portal) require either a shared island or an explicit delayed/committed interaction policy;
  merely drawing a remote portal view does not imply shared simulation.
- A common logical world clock is the initial simplification for ordering events, while tick
  rate, room residency, host assignment and synchronization backend remain separate axes.
- Ordinary room travel should not require all unrelated participants to pause, rewind or restart
  their synchronization sessions.
- Region-specific rollback, isolated Bevy `World` instances, scoped snapshots, authority
  partitioning and dynamic island merging/splitting are **engineering options to compare**, not
  accepted implementations.

## Rejected shortcuts

Do not equate `InRoomInstance` with an automatically safe rollback partition, filter a global
physics solver by room and call its state independent, or simply scope `Rollback` markers while
running one App-wide `GgrsSchedule`. Do not invent zero-cost handoffs between speculative
histories. Confirmed state exported from one island cannot be retroactively revoked without an
explicit compensation protocol.

## Feasibility gates

- Two real regions advance without sharing rollback history; rewind one while the other and its
  committed world facts remain unchanged.
- Two bodies cross different room boundaries while a third continues fighting, with no lost or
  duplicated authority, identities, custody or state.
- A portal-mediated cross-room interaction reveals whether islands must merge or can use a
  delayed committed event.
- Measure memory, scheduler overhead, physics compatibility and snapshot cost of isolated worlds
  versus scoped snapshots before choosing one.

See [ADR 0035](0035-distributed-multiplayer-without-required-services.md) and [regional architecture investigation](../planning/engine/architecture-reassessment-and-composable-multiplayer.md).
