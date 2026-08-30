# Construction and reconstitution

**State:** OPEN — canonical construction exists; lifecycle consumers still have
residual independent reconstruction/reset logic.

## Goal

A room or authoritative population should be reconstructable from one semantic
model:

```text
prepared immutable content
        + durable occurrence / progression facts
        + explicit lifecycle policy
                    |
                    v
        canonical construction plan
                    |
      +-------------+-------------+-------------+
      |             |             |             |
  new session   room transition   replay     durable restore
```

The lifecycle operations may retain different populations and may require
different authorization barriers, but they should not invent different ways to
build the same authoritative state.

## Current architecture at `26ec7b19`

### Prepared construction is transactional and federated

Room construction is already split into typed domain-owned lanes. The room
adapter translates authored world data into each domain's vocabulary; domains do
not depend upward on the world-spec crate merely to construct themselves.

One room transaction owns:

```text
plan -> preflight -> commit -> verify -> publish
```

The construction schema/fingerprint is metadata. It is not a string/`TypeId`
service locator and it does not select arbitrary constructors.

### Confirmed rollback transitions use the same readiness transaction

The rollback host no longer commits a room transition speculatively. A confirmed
lifecycle intent waits for the authorized prepared construction plan, commits the
room, then rebases GGRS onto a new frame-zero baseline. Earlier rollback frames
cannot restore the pre-transition room.

Therefore a "rollback snapshot that crosses the room boundary" is not an engine
requirement. Persistence/checkpoint state is a separate durable product boundary.

### Provenance/lifetime vocabulary exists

`SessionScopeId`, `SessionRoot`, spawn provenance/lifetime components, occurrence
records and room/session scope helpers already provide the vocabulary needed to
decide which populations a lifecycle operation may retire or reconstruct.

The open work is to use that vocabulary consistently in each reconstruction
road, not to invent another universal instance manager.

## The remaining second-constructor problem

Same-room replay/reset still has behavior that has historically been expressed as
a hand-maintained reset/rebuild ledger. That is dangerous because adding an
authoritative room-scoped population to fresh construction does not automatically
add it to replay reconstruction.

The target is not "make the reset list complete." The target is:

> replay chooses a retention policy, then invokes the same domain construction
> semantics used by a fresh room.

A lifecycle operation may retain session-owned or persistent occurrences while
retiring room-scoped state. Those retention decisions are policy and should be
explicit inputs to reconstitution rather than implicit omissions from a reset
list.

## Required convergence

### C1 — name retention classes at the lifecycle boundary

For each authoritative family, decide whether the operation retains or
reconstructs it because of its declared lifetime/provenance, not because a reset
function happened to remember the component.

At minimum distinguish:

- process-only diagnostics/services;
- gameplay-session authority;
- room-resident authoritative population;
- persistent occurrences whose durable facts outlive residency;
- rollback-timeline history, which is discarded/rebased at confirmed room
  boundaries rather than restored across them;
- presentation/read models, which may be rebuilt downstream.

### C2 — make replay consume canonical constructors

Replace family-specific replay/reset reconstruction with calls into the same
typed construction lanes used by fresh room construction. Keep only the explicit
retention/destroy policy at the replay boundary.

### C3 — make durable restore consume facts, not ECS snapshots

Save/checkpoint loading should reconstruct from authored/prepared content plus
saved facts such as occurrence disposition, encounter/quest/switch state and
inventory/custody. Do not grow a second ephemeral rollback snapshot engine.

### C4 — keep transition preparation and commit singular

Eager/headless and rollback hosts may authorize commitment differently, but they
must consume the same prepared construction semantics. The rollback host's
confirmed-frame barrier is a lifecycle authorization layer, not a second room
constructor.

### C5 — external/P2P lifecycle barrier only with a real transport customer

Local sync testing cannot exercise corrected remote input at a lifecycle barrier.
When external/P2P netplay is real, add a peer-coordinated confirmation/content
barrier around the existing construction/rebase seam. Do not build Matchbox
ceremony merely to satisfy a local planning checkbox.

## Invariants

- A domain owns the constructor for the state it owns.
- Prepared construction is deterministic for equal prepared content and durable
  facts.
- A lifecycle operation cannot publish a partially verified room.
- Replay/restore do not maintain independent semantic constructors.
- Rollback history does not cross a confirmed room boundary; a new baseline is
  installed after the lifecycle commit.
- Durable persistence stores product facts, not allocator-local ECS history.
- A relationship is persisted only when the durable road can restore both sides
  of its authority.
- Presentation is reconstructed from authoritative state rather than preserved as
  hidden authority.

## Acceptance

A representative suite should eventually prove the same authored room and
ledger facts through:

1. fresh session construction;
2. in-session room transition;
3. same-room replay/reset;
4. fresh-process durable restore;

and compare the authoritative populations/facts that the operation promises to
be equivalent, while explicitly allowing lifecycle-specific retained state.

The test does not need byte-identical worlds. It needs semantic equivalence of
the authorities the lifecycle claims to reconstruct.

## Open design questions — deliberately unresolved

- Which session-scoped populations should a same-room replay retain rather than
  reconstruct?
- Which persistent occurrence states are terminal, resettable, or recoverable?
- How should persistent actor relocation outside authored home rooms be expressed?
- Which durable relationships require stable IDs across a fresh process?
- What exact peer barrier authorizes an external/P2P lifecycle commit once real
  transport exists?

## Related durable/current authorities

- ADR 0027 — GGRS and gameplay-session rollback authority.
- [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md) —
  existence/residency/simulation/visibility policy.
- [`item-custody-and-accounting.md`](item-custody-and-accounting.md) — physical
  item occurrence/custody semantics.
- [`netcode.md`](netcode.md) — confirmed external effects and eventual P2P
  lifecycle coordination.
