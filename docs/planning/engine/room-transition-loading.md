# Room-transition loading

**State:** OPEN. The readiness/authorization transaction is implemented for
room transitions. Remaining work: transition latency and residency quality, and
the external peer lifecycle barrier.
The [checkpoint protocol](checkpoint-restoration-protocol.md) is normative for
restore inputs, cancellation, prefetch coherence and domain mutation timing.

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

A crossing publication names the live room it leaves and carries a
`LiveRoomSuccession`: it replaces that room, opens a second live room while
another seat stays, or joins a live room another seat holds. A crossing does
not stop or reset another live room. Both hosts keep simulating other live rooms
while one room loads. The succession rules are owned by
[`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md).

### Eager host

An eager host may commit a ready transition at its lifecycle boundary because no
speculative rollback history can restore an earlier room.

### Rollback host

A rollback host does **not** mutate the room inside speculative `GgrsSchedule`.
`commit_confirmed_lifecycle` waits for the lifecycle intent to be confirmed and
for the same authorized construction plan/readiness transaction. It commits the
room outside speculative execution and installs a new GGRS frame-zero baseline.
The old rollback ring is no longer a source of room state.

## Session ownership

Rollback confirmation is read for the current gameplay `SessionScopeId` through
`SessionRollbackConfirmation`. A stale/unrelated session's rollback authority
cannot make the current session's transition unhealthy.

Acceptance covers Smash -> title -> Ambition under the rollback host and
adverse local-session/shell ordering. ADR 0027 owns the lifetime rule.

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

When the player waits, and what a loading screen may be made of, is
[`loading-screens-only-for-real-work.md`](loading-screens-only-for-real-work.md)
(Jon, 2026-10-04: a loading screen only when the room is not ready at the door).

A room transition requests what the next room needs through the asset
preparation/residency authority, not ad hoc eager loading. Do not prefetch
every neighbour.

Current shape:

- Neighbour prefetch is bounded: `NEIGHBOR_PREFETCH_ROOM_BUDGET = 4`
  (`game/ambition_app/src/app/world_flow/room_transition_assets.rs`). Excess
  neighbours are skipped as whole rooms, because cached manifests are promoted
  only when complete.
- Parallax has an eviction counterpart: `ParallaxLayerSet::retain_themes` (API)
  and `retire_departed_parallax_themes` (policy: keep the active room's theme
  and its one-hop neighbours', the same set the prefetch loads).
  `scripts/measure_parallax_retire.sh` verifies that retired images leave
  `Assets<Image>`.

Open: character pages, FX sheets and boss sheets have no retire. One stated
residency authority does not exist yet. That is open work 4 in
[`asset-preparation-and-residency.md`](asset-preparation-and-residency.md).

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

## Admission and publication are separate failure boundaries

[A1](actor-monolith-work-frontier.md) requires a checkpoint route attempt to
observe lifecycle admission before it latches progress. F9 also requires domain
restorers to stop interpreting a raw reset as admitted authority. A request can be
refused; it is not evidence that either a room transition or domain restore should
happen. Prepare a checkpoint transition from its pinned continuity input; do not
modify live ledgers just to make the ordinary builder see checkpoint state.
Preserve the startup road's non-gameplay-gated installation and reset's public
phase ancestry.

The word transaction has a limit here:
raw Bevy construction commands can have effects that post-commit verification
cannot undo. Verification must prevent normal publication of a failed candidate,
but retaining the prior room needs a separately staged and constrained operation.
Do not promise that stronger guarantee from the existing recipe API.

Record failures by stage: preparation, authorization, command application,
verification, baseline installation and presentation reveal. A timeout/retry must
not publish a stale attempt after a later room selection. A10 only broadens
rollback/cleanup guarantees when a concrete customer and fixture require it.
