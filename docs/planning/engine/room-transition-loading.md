# Room-transition loading

**State:** NARROW / OPEN — the canonical readiness/authorization transaction is
implemented. Remaining work is latency/residency quality and the future external
peer lifecycle barrier.

## Current contract

A room transition is not a direct mutation request. It progresses through one
prepared lifecycle transaction:

```text
intent
  -> resolve target / carry policy
  -> prepare target construction plan
  -> readiness / authorization
  -> commit authoritative room change
  -> verify / publish
```

The eager/headless and rollback hosts consume the same construction semantics.
They differ only in when commitment is authorized.

### Eager host

An eager host may commit a ready transition at its lifecycle boundary because no
speculative rollback history can restore an earlier room.

### Rollback host

A rollback host does **not** mutate the room inside speculative `GgrsSchedule`.
`commit_confirmed_lifecycle` waits for the lifecycle intent to be confirmed and
for the same authorized construction plan/readiness transaction. It commits the
room outside speculative execution and installs a new GGRS frame-zero baseline.
The old rollback ring is no longer a source of room state.

This means the prior planning requirement "make rollback transitions use the
readiness transaction" is complete.

## Session ownership

Rollback confirmation is read for the current gameplay `SessionScopeId` through
`SessionRollbackConfirmation`. A stale/unrelated session's rollback authority
cannot make the current session's transition unhealthy.

`26ec7b19` added acceptance coverage for Smash -> title -> Ambition under the
rollback host and for adverse local-session/shell ordering. ADR 0027 owns the
lifetime rule.

## Construction authority

Room transition does not own an alternate constructor. The prepared target plan
uses the same typed domain construction lanes as ordinary room construction.
Further convergence with replay/restore belongs to
[`construction-and-reconstitution.md`](construction-and-reconstitution.md).

## Current performance evidence

Transition quality is primarily an asset/readiness/residency question rather
than a reason to make speculative room mutation part of rollback. Existing
measurements showed material asset work around room/session activation and later
render-device materialization can dominate user-visible stalls.

Use [`asset-preparation-and-residency.md`](asset-preparation-and-residency.md) for
asset demand/materialization/residency architecture and
[`performance-and-iteration.md`](performance-and-iteration.md) for current
measurement guidance.

## Remaining work

### T1 — measure user-visible transition latency on representative rendered hosts

Keep the measurement stages separate:

- target resolution/preflight;
- source I/O and decode;
- asset insertion/preparation;
- render-device materialization;
- construction commit;
- presentation readiness.

Do not quote a headless preflight time as a rendered transition budget.

### T2 — make prefetch/residency policy explicit where measurements justify it

A room transition should request what the next room needs through the asset
preparation/residency authority rather than ad hoc eager loading. Avoid a broad
prefetch-every-neighbour policy that merely moves a hitch earlier and grows
resident memory without a budget.

### T3 — keep carry/retention semantics lifecycle-owned

Possession/body/item carry policy must derive from explicit lifetime/custody
semantics. Do not add special transition-only mirrors for populations that the
construction/reconstitution model should understand.

### T4 — external/P2P coordinated commit is trigger-based

When real external netplay exists, peers need a coordinated confirmed lifecycle
barrier around the existing plan/commit/rebase seam. Local sync testing cannot
prove corrected remote input or peer content agreement.

Owner: [`netcode.md`](netcode.md).

## Invariants

- a transition has one target plan and one readiness/authorization result;
- a failed preflight/readiness check does not publish a half-constructed room;
- rollback hosts do not cross room boundaries inside speculative history;
- eager and rollback hosts use the same semantic room constructor;
- session A's rollback health cannot block session B's room transition;
- carried/persistent populations follow lifetime/custody policy rather than
  transition-specific special cases;
- asset work is measured by stage before a residency/prefetch mechanism is
  generalized.

## Acceptance

- direct/eager and rollback-host transitions reach equivalent authoritative room
  state for the same prepared content and durable facts;
- a possessed/carried body and its legitimate custody state survive a real door
  transition according to policy;
- an invalid target/readiness condition retries/refuses without publishing a
  partial transition;
- cross-game shell lifecycle leaves the new session able to transition rooms;
- future external/P2P acceptance proves a peer-coordinated lifecycle rebase rather
  than a local-only substitute.
