# Netcode and rollback host

**State:** OPEN at the transport/lifecycle frontier. Ephemeral rollback ownership
and the local sync-test host are established.

## Durable boundary

GGRS/bevy_ggrs are the sole ephemeral rollback authority. Ambition owns:

- typed backend-neutral rollback registration declarations;
- deterministic checksum/projection policy;
- input bridging;
- exact prepared-content/schema identity;
- gameplay-session ownership and invalidation policy;
- confirmed-frame/lifecycle authorization around irreversible host work.

Persistence/checkpoint serialization is a separate durable product concern and
must not become another rollback engine.

ADR 0027 is authoritative for the rollback backend and gameplay-session lifetime
rule.

## Current host model

```text
Gameplay session                  SessionScopeId
    |
    +-- prepared content identity
    +-- rollback schema fingerprint
    +-- ActiveRollbackAuthority
            |
            +-- RollbackTimelineGeneration
            +-- timeline contract/status
            +-- confirmation for this session only
```

A new GGRS timeline inside the same gameplay session inherits an existing
unhealthy diagnosis so a desync cannot be laundered by rebasing. A new gameplay
session starts fresh because the previous session's timeline discovered nothing
about the new session's world.

Gameplay reads confirmation through `SessionRollbackConfirmation`; process-level
diagnostic history is evidence only and cannot gate gameplay.

## What is already implemented

- domain-owned rollback registration declarations;
- GGRS backend extracted into `ambition_platformer2d_rollback_ggrs`;
- exact content/schema binding and invalidation;
- real `SyncTestSession` rewind/resimulation over the actual `GgrsSchedule`;
- multi-seat local input through rollback rather than replaying seat zero into
  every handle;
- runtime-created rollback entity recreation for covered families;
- external/presentation effects quarantined to the confirmed host-side boundary
  where implemented;
- confirmed room lifecycle transition waits for the authorized construction plan
  and rebases to a new frame-zero baseline;
- cross-game shell lifecycle acceptance proving one retired game's rollback
  health cannot block another game's room transition.

> **Re-checked against `8b0731706` (2026-09-03): the three load-bearing claims above
> are ACCURATE, and one is stronger than written.**
>
> - **"real `SyncTestSession` rewind/resimulation over the actual `GgrsSchedule`"**
>   — `game/ambition_app/tests/desync_canary.rs` and
>   `game/ambition_app/tests/gameplay_presentation_ggrs.rs:18` both drive one,
>   the latter explicitly "on a live `SyncTestSession` that genuinely rewinds".
> - **"cross-game shell lifecycle acceptance"** — TWO tests, not one:
>   `a_smash_session_does_not_take_ambitions_doors_with_it`
>   (`game/ambition_app/tests/shell_host_lifecycle.rs`) and
>   `a_smash_session_does_not_take_ambitions_doors_even_when_retirement_is_misordered`.
>   The second covers the ordering case the prose does not mention, and the file
>   states the rule in place: "a value inherited from the retired Smash scope is
>   not B's to read".
> - **"GGRS backend extracted"** — `crates/ambition_platformer2d_rollback_ggrs`
>   exists as its own crate.
>
> ⚠ **A note on how this was checked, because the first attempt failed.** A grep
> for `SyncTestSession` filtered with `grep -v 'tests.rs'` returned nothing and
> would have supported "this claim is stale" — the filter dropped exactly the
> files a sync-test session lives in. The pattern was fine; the exclusion was
> not. See
> [`../../recipes/re-measuring-a-planning-claim.md`](../../recipes/re-measuring-a-planning-claim.md).

## Remaining netcode work

### N1 — finish deterministic/runtime-state correctness before transport

Transport should not hide local deterministic defects. The simulation-authority
program still owns the remaining deterministic selection/composition sites and
scenario-populated dynamic-state coverage (non-rewinding authoritative memory
closed 2026-09-02, S2).

Use [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md).

### N2 — first real external/P2P session

When Smash or Ambition has an actual online slice, install a real transport
through the existing session seam. `bevy_matchbox` remains a likely candidate;
transport choice must not change simulation/input ontology.

Do not build signaling/deployment infrastructure solely to satisfy this plan.

### N3 — content/schema negotiation

Before external peers begin play, negotiate exact prepared-content identity and
rollback schema fingerprint. A peer with mismatched simulation content must fail
before speculative play rather than discovering incompatibility after divergence.

### N4 — coordinated lifecycle barrier

The local rollback host can commit a confirmed room transition and immediately
rebase because there is no remote corrected-input frontier. A real external/P2P
host needs a peer-coordinated barrier around the same construction/rebase seam.

The barrier must answer:

- which lifecycle intent/frame is being committed;
- that every peer confirms the required input/content horizon;
- that every peer has the same authorized construction plan/content identity;
- how corrected input arriving before the barrier cancels/replaces a pending
  intent;
- when the old rollback history can be discarded and the new frame-zero
  baseline installed.

This is an authorization protocol around canonical construction. It is not a
second room constructor.

### N5 — disconnect/reconnect/spectator/deployment policy

Defer until the first two-peer deterministic lifecycle path is green. These are
product/network-service concerns and should not distort the simulation model in
advance.

## Identity rules

- `RollbackId` is GGRS frame-history identity.
- `SimId` is Ambition semantic simulation identity.
- `SessionScopeId` owns one gameplay activation.
- `RollbackTimelineGeneration` distinguishes successive rollback timelines,
  including rebases, across the process.

Do not use one of these as a substitute for another because all happen to be
stable integers.

## Confirmed effects

Irreversible host effects must not be emitted merely because a speculative tick
ran. Audio/VFX that are purely reconstructable presentation may replay from
confirmed simulation state; persistence writes, analytics, network-side effects,
file output and similar irreversible work require an explicit confirmation
boundary.

Developer tracing may retain historical-resimulation observations as diagnostics
only when it cannot feed authoritative behavior.

## Verification

Before online transport is considered healthy:

1. local `SyncTestSession` repeatedly rewinds/resimulates representative gameplay
   without checksum divergence;
2. multiple seats preserve independent input streams across rewind;
3. runtime-created authoritative populations used by product play survive
   recreation and deterministic composition;
4. session retirement/startup cannot transfer rollback authority across
   `SessionScopeId`;
5. content/schema mismatch refuses before play;
6. a real two-peer host eventually proves corrected input, confirmation, and one
   coordinated lifecycle/rebase.

## Non-goals

- custom snapshot/history machinery beside GGRS;
- persistence implemented as rollback snapshots;
- multiplayer-specific actor/control ontology;
- Matchbox/deployment work before a product customer exists;
- treating a two-seat local sync-test session as proof of a two-peer network
  protocol.
