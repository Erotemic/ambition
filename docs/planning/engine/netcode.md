# Deterministic simulation, rollback, and networking

## Binding direction

Ambition uses `ggrs` plus `bevy_ggrs`; it does not maintain a second rollback
engine. ADR 0027 is authoritative.

```text
ControlFrame per player
    → ggrs Session (sync test now; P2P later)
    → GGRS save/load/advance requests
    → bevy_ggrs SaveWorld / LoadWorld / GgrsSchedule
    → Ambition deterministic simulation
```

GGRS owns prediction, rollback-window selection, frame history, confirmed frames,
load/resimulation requests, and checksum comparison. `bevy_ggrs` owns typed Bevy
component/resource snapshots, rollback entity recreation, and `Entity` remapping.

Ambition owns:

- fixed-step deterministic simulation and per-player `ControlFrame` input;
- the explicit authoritative component/resource registration contract;
- canonical float-safe codecs/checksum projections where clone/`Hash` is not a
  meaningful contract;
- semantic `SimId`, construction, and authored relationships;
- immutable prepared-content identity and schema fingerprinting;
- session startup/invalidation policy;
- eventual confirmed-frame release of external effects.

## Current implementation

`ambition_platformer2d_runtime::SimulationHost` is the construction-time simulation owner:
`RenderFrame`, `Fixed60Hz`, or `Ggrs`. Only `Ggrs` installs rollback schedules,
snapshot storage, checksum machinery, and session/request handling. Games that
do not require rollback choose one of the lighter hosts before content plugins
build and pay no GGRS runtime tax.

`ambition_platformer2d_runtime::rollback::AmbitionRollbackSchemaPlugin` records the exact
typed component/resource contract for every host so prepared-content identity
remains inspectable and stable. On non-GGRS hosts this is only a small descriptor
registry. `AmbitionRollbackPlugin` is GGRS-only and installs
`GgrsPlugin<ControlFrame>`, deterministic GGRS time, snapshot/checksum runtime
machinery, relationship mapping, message-buffer cleanup on load, and exact
session-content/schema enforcement.

The actual `GgrsSchedule` runs with Bevy's single-threaded executor. Ordered
simulation phase sets remain the semantic schedule contract; stable same-build
plugin registration order resolves systems intentionally unordered inside a
phase. The exhaustive Bevy ambiguity diagnostic is disabled for this schedule,
while `SyncTestSession` supplies the stronger behavioral check by repeatedly
rewinding and resimulating the real world.

`ambition_sim_harness::Platformer2dSimHarnessOptions::with_sync_test_rollback*` selects
`GgrsSchedule` before game/content plugins are built, starts a real
`SyncTestSession` after startup has published the canonical prepared session,
and submits one local `ControlFrame` per harness step. Instrumentation proves
that GGRS issues real load and extra advance requests; mismatch events and
content/schema invalidation are surfaced through `rollback_health()`.

The retired `ambition_platformer2d_runtime::snapshot` tree is deleted. There is no blob
registry, manual snapshot queue, room-staging restore, dynamic respawn decoder,
or compatibility wrapper behind the new API.

## Identity

- `RollbackId` is GGRS's allocator-local rollback-history identity.
- `SimId` is Ambition's semantic authored/runtime identity. It remains the key
  for construction, relationships, observation, replay, diagnostics, and future
  persistence.
- Only authoritative family anchors require `Rollback`; `SimId` alone does not
  pull presentation-only entities into frame history.

⛔ **The runtime-spawn half of this contract was violated THREE ways, and all
three surfaced on 2026-08-08 from unrelated investigations.** N3.1 has said since
2026-07-06 that *"dynamically-spawned sim entities (projectiles, dropped items,
spawned adds) get a deterministic sequence id minted at spawn — `(spawner SimId,
per-spawner counter)`"*. What the tree actually did:

| site | what it minted | symptom |
|---|---|---|
| enemy drops (coin/heart/ability) | **no id, no provenance at all** | every drop drew as a magenta stand-in — the player collected a diagnosis box |
| `spawn_split_offspring` | `SimId::placement(..)` — the **authored** namespace | none visible; offspring are claimed as staged actors |
| construction executor (every authored boss) | a `SimId` with **no counter** | a shipped boss's summon warned and built nothing |

⭐ **the contract was right and the enforcement was absent.** Each site was
correct about the half it remembered and silent about the half it did not, and
each failure was invisible in a different way — one was loudly wrong on screen,
one was silently wrong in the data, one was a feature that simply never happened.
None of the three tests covering these paths could see it: they hand-built their
fixtures, supplying exactly what production omitted.

⭐⭐ **the durable fix was to make the pairing unforgettable rather than
remembered**: `SimId` is now `#[require(SimIdCounter)]`, so *"identified"* and
*"able to be descended from"* are one condition rather than two facts six mint
sites had to keep in step.

## ⛔⛔ A rollback registration reddens TWO baselines, and one is invisible per-crate

Measured 2026-08-17, after `rollback-wire-format-is-frozen` sat RED on `main` for
hours and nobody's per-crate run could see it.

```text
game/ambition_app/tests/rollback_schema_baseline.txt      inside `app_it`
scripts/baselines/rollback-schema-baseline.json           the ABSENCE CONTRACTS
                                                          (check_absence_contracts.py --check)
```

**Neither appears in any `cargo test -p <crate>` run.** Registering a component,
*or moving a registered type between crates*, moves both — D147 moved
`StocksMatchSettled` across a crate boundary and reddened the json alone, which
is how it stayed red.

⚠ **and the `.txt` is `include_str!`d**: after editing it, `touch` the test file
or the run compares against the stale embedded copy and fails reporting
`0 added, 0 removed` — which reads like an ordering bug and is not one.

⇒ when you register rollback state, or move a registered type, check **both**
before pushing. See also [`../queue.md`] D147/D150.

⚠ **rollback cost, verified independently rather than taken on report** — this is
the widest-reaching change of 2026-08-08 and it touches every `SimId` carrier:

* `SimIdCounter` was **already** snapshot-registered
  (`ambition_platformer2d_shared_tangle::rollback_registration`, `rollback_component_canonical`), so **no
  new type entered the schema.** Confirmed: the schema version is still **18** and
  neither `rollback_schema_baseline.txt` nor `scripts/baselines/rollback-schema-baseline.json` has moved
  since `a7013ef82`, which predates the change.
* What changed is how many ENTITIES carry an already-registered component — a
  snapshot-payload cost (8 bytes each), not a schema or determinism change.
* ⭐ **the restore question is the one that mattered**: a required component is
  supplied only when the component is **absent**, so a restore putting back
  `SimIdCounter(7)` keeps 7 rather than re-minting a default. Evidenced by
  `rollback_coverage`'s inert sweep not firing (its poison test still passes) and
  the rollback tests in `app_it` staying green. ⚠ **the namespace half is still unenforced**, and ⛔ **it cannot be enforced the
same way.** Measured: **70 `SimId::placement` call sites**, and nearly all are
correct — the construction executor naming authored features, room staging,
probes, tests. A grep contract cannot separate them from a runtime spawn wearing
the authored namespace, because the distinction is semantic: *was this entity
authored in a room spec, or minted while the sim ran?* Only the call site knows,
and a checker that cannot tell them apart is noise a reader learns to waive.

⭐ **the type-level move that WOULD work, by analogy with the counter fix**: make
`SimId::placement` take something only authored content can produce — a
`PlacementId` newtype carried out of the room spec — rather than a `&str` anyone
can hand it. Then a runtime spawner cannot spell an authored id because it has
nothing to spell it with, exactly as `#[require(SimIdCounter)]` means no mint site
can forget the counter. ⚠ that is a real refactor across 70 sites, not a slice;
recorded as the shape of the answer rather than as a plan.
- A session captures the exact `PreparedContentIdentity` and deterministic
  rollback-registration fingerprint. Any change removes the active session
  before another GGRS frame runs.
- Local developer LDtk hot reload stops the owned SyncTest session, commits or
  rejects the prepared-content transaction, and starts a fresh zero-distance
  baseline at frame zero. External/P2P sessions still require a coordinated
  peer content barrier and reject unilateral reload.

## State policy

Authoritative state is registered as one of:

1. canonical GGRS strategy plus the same canonical checksum;
2. exact clone strategy plus a domain checksum projection;
3. exact clone strategy for immutable/structural shell data bound by prepared
   content identity;
4. derived state rebuilt by the ordinary per-frame maintenance path.

Every allocator-local relationship registered for exact cloning also registers
`MapEntities`. Dynamic bodies/projectiles/encounters are rollback entities, so
`bevy_ggrs` recreates their entity population and registered component shape.
Ordinary room construction is no longer part of rollback restore.

Rollback-sensitive message buffers are cleared during `LoadWorld`; replayed
inputs regenerate the accepted future. The remaining production boundary is
external/presentation effects: audio, VFX, persistence writes, analytics, and
similar irreversible work must be buffered by frame and released only when GGRS
confirms the frame. Developer trace recorders skip passes marked as historical
resimulation, and file output is flushed only outside `GgrsSchedule`, so replay
cannot synthesize or write duplicate anomaly dumps.

## Verification

The narrow gate is the real headless simulation, not a toy counter:

- repeated `SyncTestSession` rewinds/resimulation complete without checksum
  mismatch;
- two independent harnesses driven by the same controls retain equal
  observations;
- dynamically spawned actor and projectile families survive rollback entity
  recreation;
- every authoritative anchor carries `Rollback`;
- deterministic registration dumps/fingerprints are insertion-order stable;
- ordinary simulation does not change the bound content epoch.

## Next online slice

1. ✔ Quarantine external effects behind confirmed-frame release.
2. ✔ **Add a two-peer native/loopback GGRS acceptance test.** Landed
   2026-07-28 as `two_seats_drive_independent_streams_through_a_rewind`: a
   two-seat sync-test session, both players local, the seats driven in OPPOSITE
   directions for 32 frames through the real save/rewind/resimulate loop.

   ⚠ **Two SEATS, not two peers, and the distinction is the point.** A sync test
   has no remote peer by definition. What this buys is that N input streams go
   through rewind and are checksum-compared — the precondition for any of them
   being remote later, and the thing that was missing: the session was built
   `with_num_players(1)` while C4 shipped a 2–4 player couch mode, so the oracle
   proved determinism for a quarter of what the game seated.

   ⛔ **What it found on the way is the real content of this row.**
   `publish_local_inputs` handed the PRIMARY seat's frame to every handle —
   correct with one handle, silently wrong with two, and it would have made a
   two-peer session checksum-compare a simulation nobody was playing. Seats 1–3
   were also written on the FEEL clock where GGRS never saw them, so a
   resimulated frame replayed seat zero faithfully and gave every other seat
   whatever the device happened to be doing at replay time. Both are fixed;
   neither was visible from a one-player session.
3. ⛔ Add `bevy_matchbox` signaling/WebRTC transport through the existing
   `install_session` seam. **DEFERRED by Jon until Smash** — do not reach for it.
4. Negotiate exact prepared-content and rollback-schema identities before play.
   ⚠ note for whoever takes this: the fingerprint EXISTS and moves on a schema
   change, but nothing captures a BEFORE and asserts an AFTER across a refactor.
   That harness is also the precondition the architecture campaign named for its
   rollback-relocation campaign, so building it once serves both.
5. Add disconnect/reconnect, spectator, and deployment policy only after the
   two-peer deterministic oracle is green.

Persistent save/checkpoint serialization is a separate product concern. It may
reuse semantic codecs, but must never become a second ephemeral rollback driver.
