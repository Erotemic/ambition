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

`ambition_runtime::SimulationHost` is the construction-time simulation owner:
`RenderFrame`, `Fixed60Hz`, or `Ggrs`. Only `Ggrs` installs rollback schedules,
snapshot storage, checksum machinery, and session/request handling. Games that
do not require rollback choose one of the lighter hosts before content plugins
build and pay no GGRS runtime tax.

`ambition_runtime::rollback::AmbitionRollbackSchemaPlugin` records the exact
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

`ambition_sim_harness::SandboxSimOptions::with_sync_test_rollback*` selects
`GgrsSchedule` before game/content plugins are built, starts a real
`SyncTestSession` after startup has published the canonical prepared session,
and submits one local `ControlFrame` per harness step. Instrumentation proves
that GGRS issues real load and extra advance requests; mismatch events and
content/schema invalidation are surfaced through `rollback_health()`.

The retired `ambition_runtime::snapshot` tree is deleted. There is no blob
registry, manual snapshot queue, room-staging restore, dynamic respawn decoder,
or compatibility wrapper behind the new API.

## Identity

- `RollbackId` is GGRS's allocator-local rollback-history identity.
- `SimId` is Ambition's semantic authored/runtime identity. It remains the key
  for construction, relationships, observation, replay, diagnostics, and future
  persistence.
- Only authoritative family anchors require `Rollback`; `SimId` alone does not
  pull presentation-only entities into frame history.
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

1. Quarantine external effects behind confirmed-frame release.
2. Add a two-peer native/loopback GGRS acceptance test.
3. Add `bevy_matchbox` signaling/WebRTC transport through the existing
   `install_session` seam.
4. Negotiate exact prepared-content and rollback-schema identities before play.
5. Add disconnect/reconnect, spectator, and deployment policy only after the
   two-peer deterministic oracle is green.

Persistent save/checkpoint serialization is a separate product concern. It may
reuse semantic codecs, but must never become a second ephemeral rollback driver.
