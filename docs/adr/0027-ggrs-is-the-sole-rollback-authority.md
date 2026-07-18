# ADR 0027: GGRS is the sole rollback authority

## Status

**Accepted; implemented for the simulation harness** (2026-07-18).

- **Supersedes:** Ambition-owned ephemeral snapshot/restore machinery described by the earlier N3 planning state

## Decision

Ambition uses `ggrs` and `bevy_ggrs` as the sole authority for ephemeral rollback:

- GGRS owns input synchronization, prediction, frame history, save/load requests,
  rollback-window selection, resimulation, confirmed-frame tracking, and sync-test
  checksum comparison.
- `bevy_ggrs` owns Bevy world snapshots, rollback entity creation/destruction,
  component/resource restoration, and allocator-local `Entity` remapping.
- Ambition owns only the typed registration contract, deterministic domain codec /
  checksum projections, the input bridge, exact prepared-content/schema identity,
  and session invalidation policy.

The deleted `ambition_runtime::snapshot` subsystem is not retained behind a
compatibility facade. Persistence/checkpoint serialization, when required, will
be a separate product boundary and must not become a second rollback engine.

## Identity and ownership

`bevy_ggrs::RollbackId` is GGRS's frame-history identity. `SimId` remains
Ambition's semantic authored/runtime identity for construction, diagnostics,
relationships, replay, observations, and future persistence. A `SimId` does not
by itself opt a presentation-only entity into rollback; authoritative family
anchors install `Rollback` explicitly.

A GGRS session is bound to the exact `PreparedContentIdentity` and deterministic
rollback-registration fingerprint present when it starts. A changed content
epoch or registration schema invalidates and removes the active session before
another GGRS frame can run. LDtk hot reload therefore cannot commit while a
rollback session is active; a coordinated session restart is required.

## Registration policy

Authoritative mutable components/resources use one of:

- an explicit canonical byte strategy, also used for checksums;
- an exact clone strategy plus an explicit canonical checksum projection;
- an exact clone strategy for immutable/structural shell data whose behavior is
  already bound by prepared-content identity.

Allocator-local relationships use `MapEntities`. Frame-derived values are
registered as derived and rebuilt by their ordinary maintenance systems. Sim
message buffers are cleared on `LoadWorld`; replayed inputs regenerate the
accepted future. Presentation/external side effects must later be released only
from confirmed frames.

Registration names, owners, kinds, concrete type names, and policy details form
an order-independent, versioned schema fingerprint. Conflicting duplicate names
fail during App construction.

## Harness and networking sequence

The simulation harness uses `SyncTestSession` first: real game inputs drive the
real `GgrsSchedule`, and GGRS repeatedly saves, loads, resimulates, and compares
checksums. Future native/Matchbox P2P hosts construct another GGRS `Session` and
install it through the same exact-content/schema seam; transport does not alter
simulation ownership.

`GgrsSchedule` uses Bevy's single-threaded executor. Ambition's explicit phase
sets define the semantic ordering; systems intentionally unordered inside one
phase use stable same-build App construction order. Bevy's exhaustive ambiguity
diagnostic is disabled only for `GgrsSchedule` because emitting hundreds of
pairwise edges would duplicate that phase architecture without strengthening the
same-build contract. `SyncTestSession` remains the behavioral determinism oracle:
it repeatedly restores and re-executes the actual schedule and rejects divergent
checksums.

## Consequences

- Thousands of lines of custom history, blob dispatch, room staging for rollback,
  entity reconciliation, compatibility preflight, and restore tests are deleted.
- Ordinary construction/transition/reset remain canonical game architecture, but
  no longer masquerade as rollback implementation.
- The next rollback-networking slice is confirmed-frame side-effect quarantine
  plus a Matchbox-backed two-peer handshake. The independent construction-plan
  track remains the next world-construction milestone.


## Current implications for agents

- Put authoritative gameplay systems in `GgrsSchedule`; never add another rollback driver or frame-history store.
- Register every mutable authoritative component/resource through `AmbitionRollbackApp`, including entity remapping where needed.
- Use `SyncTestSession` for deterministic rollback verification before adding transport.
- Keep presentation and irreversible host effects outside speculative execution until the confirmed-frame effect boundary is implemented.
- Keep `SimId` as semantic identity and let `bevy_ggrs::RollbackId` remain an internal frame-history identity.
