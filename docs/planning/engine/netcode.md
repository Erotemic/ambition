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

⛔ **AND ONE PIECE OF N1 IS NETCODE'S OWN AND CANNOT BE FINISHED WITHOUT A
RULING — [`Q128`](../awaiting-maintainer-decision.md#q128--should-the-simulation-tick-be-rebased-when-peers-agree-to-start-or-stay-an-absolute-per-app-count).**
`ambition_time::SimTick` is registered `resource-canonical`, so its ABSOLUTE
value is inside the checksum two peers compare, and it counts every sim step this
App has run including menu frames. Two Apps running for different lengths of time
disagree from the first compared frame, before anything else matters. It is the
last open road of the ID-PEER campaign and the only one engineering cannot close
alone: a projection excluding the tick would exclude the TIMELINE, so what is
needed is a session-relative tick rebased when peers agree to start — and where
that agreement comes from is N2's transport, not a refactor.

⚠ **THE ORDERING BETWEEN N1 AND N2 IS THEREFORE NOT STRICT HERE.** This section's
premise is that transport should not hide local deterministic defects, and that
still holds for the other nine roads. But this one defect cannot be observed
locally at all: the only sessions in use are `SyncTestSession`, one machine
rewinding itself, and a canary comparing a machine against its own past is
structurally incapable of catching a two-peer disagreement. So it will not
announce itself before N2, and N2 is what supplies the agreement it needs.

### N2 — first real external/P2P session

When Smash or Ambition has an actual online slice, install a real transport
through the existing session seam. `bevy_matchbox` remains a likely candidate;
transport choice must not change simulation/input ontology.

Do not build signaling/deployment infrastructure solely to satisfy this plan.

⛔⛤ **AND N2 NOW HAS A COUNTED POPULATION WAITING ON IT, NOT ONLY A DEFECT.**
`simulation-authority-and-determinism.md`'s **S7** ranks the rollback rows that
are outside the peer checksum, read every unfiltered tick, and float-bearing: 25
rows, 12 of them mutably written in production. Two have been measured clean
against a local resimulation and that is ALL a `SyncTestSession` can establish
about them — a value nothing compares between peers is reproducible locally and
divergent across peers at the same time, and the second half is invisible from
inside one App. ⇒ So N2 is not only what makes an online slice possible; it is
the only thing that can ask the question those 25 rows pose. S7 owns the list and
this row owns the session; neither duplicates the other.

### N3 — content/schema negotiation

Before external peers begin play, negotiate exact prepared-content identity and
rollback schema fingerprint. A peer with mismatched simulation content must fail
before speculative play rather than discovering incompatibility after divergence.

✔ **THE SCHEMA HAD TWO RECORDINGS CHECKED IN DIFFERENT LANES, THEY DID NOT
AGREE, AND THE DUPLICATE IS NOW COLLAPSED** (the measurement and what replaced
it are below; the negotiation this row is really about is still open).
`game/ambition_app/tests/rollback_schema_baseline.txt`
is read by the Rust lane and `scripts/baselines/rollback-schema-baseline.json` by
`scripts/check_absence_contracts.py` in the repo-tooling lane. A single new
registration owes both, and on 2026-09-10 one landed with only the first
updated: the Rust lane was green, which is precisely what made the other
invisible.

Measured 2026-09-16, the two are not copies that drifted — they are different
instruments with different reach, and the smaller one is the peer-facing risk:

- the `.txt` is the runtime dump: 493 rows of name/kind/detail from a live
  registry, and the fingerprint (`ggrs-rollback-schema-v194`) is a hash of the
  whole dump, `detail` prose included.
- the `.json` was source-scanned: 423 `stable_schema_names` and 129
  `encoded_types`. It recorded no fingerprint at all.
- **73 of the 493 runtime rows were invisible to the source scan, and 21 of
  those feed the peer checksum — 15% of the 144 checksum-feeding
  registrations.**
  (`feeds_peer_checksum`'s own TRUE arm, read from the source rather than from a
  hand-kept list: 14 `component-canonical`, 6 `resource-clone-custom-checksum`,
  1 `component-clone-cursor`.)

Three causes, all structural rather than drift — and the first two are why the
fix below is a collapse and not a wider regex:

1. **47 are registered under `game/`.** `rollback_schema_usage` globs
   `crates/*/src/**/*.rs`. The function's own comments describe twice how
   hand-listing registration FILES failed and was replaced by following the
   `R: RollbackRegistrar` marker — but the marker is only followed inside a
   hand-listed ROOT, and every demo and content crate lives outside it.
2. **26 are colon-form** (`entity:*`, `root:room_set`) spelled as plain literals
   in a file the scan does read and whose marker bound it matches
   (`crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs`).
   The name pattern is `"([a-z_]+\.[a-z_.]+)"`, which requires a dot, so a
   colon-delimited name cannot match however well the file is reached.
3. **The 3 names the JSON holds and the dump lacks are not stale** — the
   `message.causal_*` trio is filtered out of the dump BY THE TEST, on both
   sides of the comparison, so that compiling the causal recorder cannot move
   the state-schema baseline: those channels carry no snapshot bytes. The
   source scan has no such rule and records the literals. This one is a
   deliberate divergence, and the surviving N3 question is where it should be
   stated once rather than in each lane's own dialect.

⇒ Negotiating an identity the repo keeps twice is negotiating which copy, and
here the copies answered differently about 15% of what peers actually compare.

✔ **RESOLVED 2026-09-16 — the duplicate is collapsed and the runtime dump owns
the names.** `stable_schema_names` is gone from the JSON and from
`rollback_schema_usage`: a source scan cannot own this fact, because a
registration is a runtime call and the four spellings above are not a closed
set. What replaced it is the one question about the dump that the tree cannot
answer alone, because the dump has no memory of its previous self:

> **`the-peer-visible-schema-may-not-move-without-the-version`** — if the set of
> rows whose kind answers `feeds_peer_checksum()` changes, the version on the
> dump's first line must change in the same commit. 144 rows at
> `ggrs-rollback-schema-v194`.

⛔⛤ **AND THAT SLICE DROPPED `detail` AT FIRST AND WAS BLIND TO 48 OF THE 144.**
Q122's own measurement on this page's neighbour caught it: a
`resource-clone-custom-checksum` row's sentence records what its
`fn(&T) -> u64` actually covers, and 22 of them say 22 different things, so
NARROWING A PROJECTION MOVES NO NAME, NO KIND AND NO TYPE. The same is true of
the 18 `resource-canonical` rows, where `rollback_resource_optional_canonical`
adds a presence term to the checksum under an unchanged name/kind/type.

⇒ The rule that fits both halves is the artifact's own, and it is the same SPLIT
Q122 proposes, applied at the granularity the dump already has: **`detail` is
kept exactly where it distinguishes rows of the same kind, and dropped where it
does not.** A kind whose rows all carry one sentence has a `detail` the `kind`
column already implies; a kind whose rows differ is using it to say something
`kind` cannot. 48 of 144 rows carry theirs. The control and the positive differ
only in their subject: rewording the 7 uniform `component-clone-cursor` rows
stays green, rewording one of the 22 varying ones reddens.

⚠ The honest cost: a genuine reword of a VARYING kind's sentence still reddens
when the projection did not change. That is 48 rows of exposure instead of 493,
and it fails in the safe direction.

⭐ THE REPOSITORY ALREADY OBEYED THIS AND HAD NEVER SAID SO. Of the commits that
touched the dump, 14 changed the checksum-feeding set and all 14 moved the
version; 2 changed the wider row set and held it, and both added a single row of
a kind nothing hashes — the case `ambition_mount`'s own registration documents
as deliberate. So the guard was landed green against history rather than
imposed on it, and a non-hashed registration still lands without a bump, which
is what keeps the version from becoming a number people bump to pass a check.

Also measured and fixed in the same pass: `encoded_types` had the same
`crates/`-only root and was blind to nine `SnapshotState` sites in
`ambition_content`'s boss specials — 129 types became 137. That widening is safe
where the name census's was not, because it matches a plain `impl` beside the
type rather than following a registration road.

✔ **AND THE COMPOSITION QUESTION IS NOW MEASURED RATHER THAN ASSUMED.** The
baseline is recorded from `Platformer2dSimHarness`; the player runs
`build_visible_app`. Everything reading the baseline — the ratchet above, the
fingerprint two peers would negotiate, the Rust lane's own byte-for-byte arm —
was describing the sandbox, and the sandbox arm stays green precisely because it
never asks the shipped app. `the_shipped_app_registers_the_same_schema_as_the_sandbox`
now asks: **the two dumps are identical.** It carries the anti-vacuity floor that
two empty registries are also identical, and both poisons fire on their own
message path (a `deterministic_dump` on one side hits the diff; emptying a side
hits the floor).

⚠ Ordering is not a hazard here and this is why: `entries` is a `BTreeMap`, and
`canonical_section` preserves iteration order rather than sorting — so the
determinism comes from the container, and a future change to a `HashMap` would
make the fingerprint vary per process with nothing watching.

What this does NOT settle, and N3 still owes: **no peer handshake reads the dump
or its version.** The invariant makes the identity honest and the arm makes it
the shipped one; neither makes it EXCHANGED, which is the row's actual subject
and waits on N2's absent P2P session. `message.causal_*` is also still filtered
by the test rather than negotiated — a whole-schema identity that varies with a
cargo feature is an N3 question nobody has ruled on.

The [extension contract](extension-state-and-execution.md) extends this same
compatibility manifest with module code, port versions, complete extension schema
digests and numeric/runtime execution policy. App-local epochs are stale-plan
stamps, not peer identities. The existing content digest remains the mechanical
root. Unknown/mismatched required inputs refuse before speculative play.

Initially a remote session pins its mechanical generation. Local authoring reload
uses the existing supported reconstruction/rebase road and preserves unhealthy
same-session diagnostics. It does not implement coordinated mid-session online
code migration or a second snapshot timeline. Those require N4 plus an explicit
migration requirement; they do not block local data iteration.

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

⛔⛤ **AND THE RULE ABOVE IS NOT ENOUGH, BECAUSE THE SUBSTITUTIONS THAT ACTUALLY
HAPPENED WERE NOT SUBSTITUTIONS OF THE TYPE — THEY WERE OF ITS VALUE.** Nobody
wrote `SessionScopeId` where `SimId` belonged. What happened ten times is that a
host-local COUNT was read out of one of these and used to derive a canonical
identity: the session root was minted `SimId::singleton("session",
activation_id)`, a match item was `SimId::match_spawn(activation_tick, ..)`, and
a settlement verdict was checksummed with the whole `MatchInstance` in it. Each
one type-checks, reads correctly, and makes two hosts that agree completely about
a session disagree about the world.

⇒ **THE RULE THAT CATCHES THOSE:** a value that counts something THIS PROCESS did
— activations, sessions, sim steps, load transactions, content epochs — may name
a thing for cleanup, staleness rejection and correlation, and may never be an
input to authoritative RNG, deterministic construction provenance, rollback
identity, contact/projectile identity, or a peer checksum. The acceptance test
is: two Apps with arbitrary different prior local history, entering the same
peer-agreed session, must reach the same canonical mechanical identities and the
same rollback-visible state.

⚠ **NO TYPE CENSUS CAN SEE THIS CLASS**, which is why it is stated here rather
than left to a guard. `game/ambition_app/tests/id_peer_audit.rs` reads the live
rollback registry and asks whether a host-local TYPE is registered; the two
worst instances were a canonical type whose PROVENANCE was local — a counter
inside a constructor argument, and a counter inside a singleton's key. Those are
held by value-level arms in the crate that MINTS each identity.

⇒ Nine roads are closed and two are open; the table and the arm holding each one
are the ID-PEER row in [`../queue.md`](../queue.md). One of the two is netcode's,
below; the other is `Q122`, the snapshot schema fingerprint hashing prose.

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

## Decomposition and effect-delivery limits

The current same-build rollback contract is retained by the
[reassessment](architecture-reassessment.md). Moving a checkpoint progress type or
installer must preserve its wire identity, baseline/reset lifetime and confirmed
admission phase. A crate/module path is not an instruction to change the wire ID.
A new serialization schema or executable build is a separate compatibility change.

Confirmed in-process effect release is not a durable exactly-once protocol.
Packet A10 does not retrofit one through the construction interface. When a real
external transport or persistent side effect is added, name the idempotency scope,
restart recovery, duplicate delivery and acknowledgement owner explicitly.

Room publication remains outside speculative execution with a new frame-zero
baseline. Concurrent world residency, cross-room snapshots and two independent
live matches are separate capabilities; A8 requires a real two-instance witness
before generalizing every identity or rollback resource.

## Generation activation and active population are separate network promises

[generation/reload](content-generation-and-reload.md) is initially local development
reconstruction with a fresh timeline; active remote sessions pin their generation.
A code/schema match alone does not authorize old history to run under new content.
Port/execution profiles and the host compatibility policy remain part of admission.

Multi-instance membership changes preserve unaffected instance state under the
session's accepted baseline/confirmation policy. Do not invent independent room
rollback clocks or clear remote actors to reuse the single-room reset. A future
real-transport barrier must coordinate those decisions; local GGRS sync tests are
necessary controls, not proof of remote lifecycle coordination.
