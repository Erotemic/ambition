# Determinism, local multiplayer, and netcode ladder

This document records the durable contract and the remaining ladder. The detailed
N0/N3.1 execution ledger through 2026-07-16 is archived at
[`docs/archive/reviews/netcode-plan-through-2026-07-16.md`](../../archive/reviews/netcode-plan-through-2026-07-16.md).

## Decision

Ambition currently promises **managed same-build determinism**: the same binary,
content, configuration, and input stream should produce the same authoritative
simulation result. Cross-platform bit-exactness is not a current promise.

That level is sufficient for replay, desync detection, headless comparison, local
multiplayer, and development of rollback-safe state ownership. Stronger numeric
portability is reconsidered when online transport becomes an active product goal.

## The ladder

### N0 — deterministic simulation substrate

Landed foundations include:

- a fixed-tick simulation mode;
- typed control/input-stream capture and replay;
- deterministic-ordering discipline for authoritative systems;
- stable simulation identity (`SimId`) and snapshot registration;
- a desync-canary/replay comparison surface.

These are maintained as engine behavior, not as a permanent migration campaign.
New work should test meaningful replay/state properties rather than add source
scanners merely because determinism is involved.

### N1 — local multiplayer

Local multiplayer is an engine/customer requirement before online networking:

- N bodies are controlled through slots rather than a privileged player singleton;
- devices bind to slots through the common input layer;
- human, brain, RL, replay, and future network controllers produce the same control contract;
- session/UI ownership is explicit for joined/local observers.

Super Smash Siblings is the acceptance customer. Online transport is not required
for this phase.

### N2 — deterministic lockstep

Lockstep is a post-1.0 candidate after local-N and deterministic replay are proven.
It requires:

- an explicit transport/session shell;
- input delay and confirmation policy;
- simulation/content/config identity negotiation;
- deterministic failure reporting rather than silent divergence.

Do not introduce network-specific gameplay paths. Network input terminates at the
same slot/control seam as local and replay input.

### N3 — rollback

Rollback depends on exact reconstruction, not merely serializing many components.
Both N3.2 slices are landed: the same-room/session-ownership slice and the atomic
active-room transaction (a rollback window may span a room transition). A future
rollback driver should consume the snapshot/session and simulation-harness
surfaces, not create a second runtime assembly.

## N3.1 — snapshot substrate

The substrate owns:

- one stable identity vocabulary shared with `SimView`/replay;
- domain-registered codecs and versioned snapshot shapes;
- deterministic capture order;
- explicit unsupported-state and codec errors;
- transactional restore semantics where mutation begins only after preflight;
- reconstruction of derived runtime state through canonical domain/session paths.

Domain crates register the state they own. Runtime coordinates capture and restore
because the operation is cross-domain.

## N3.2 — exact reconstruction and resimulation

This is the active netcode-adjacent campaign and is coupled to session-root
exclusivity.

### Required properties

1. **One room/session construction authority.** Activation, reset, transition, and restore use the same App-installed placement-lowering registry and canonical session services.
2. **Preflight before mutation.** Unsupported identities, rooms, codecs, versions, or required provider data are rejected before authoritative state is partially changed.
3. **Exact ownership.** Snapshot-restored entities and relationships belong to the exact session scope; no process-global handle/cache points to a retired scope.
4. **Deterministic derived state.** Moving platforms, collision overlays, action-derived hitboxes, read models, and other reconstructible state are rebuilt from authoritative inputs in defined order.
5. **Losslessness where claimed.** Capture → restore produces the same canonical authoritative snapshot/observation for all registered state.
6. **Bounded resimulation proof.** After restoring an earlier tick and replaying the same input suffix, the result matches the uninterrupted run.

### Independent gates

Both are required:

- **Session-isolation gate:** activate A, exercise it, retire it, activate B (or A with a fresh scope), and prove no entity, relation, cache, view row, or raw handle refers to the old scope.
- **Exact-restore gate:** restore a captured state, rebuild derived state through the canonical paths, replay a bounded input suffix, and match the uninterrupted result.

A restore test that manually refreshes ambient global mirrors does not satisfy the
session-isolation gate.

### N3.2a — landed same-room/session slice

The process-global `SceneEntities` handle bag was removed; its responsibilities
are derived from canonical player/HUD/quest markers. A provider-installed
`SessionTeardownPlugin` resets the remaining active-session resource mirrors on
scope retirement, and `ambition_demo_sanic_app/tests/session_isolation.rs` proves
isolation through the real host lifecycle.

`MovingPlatformSet` is rebuilt by canonical room construction, registered as
snapshot state, and explicitly cleared on teardown. It is lifecycle-scoped under
the current one-live-session host contract; the type does not carry an independent
session key.

For supported snapshots that remain in the active room, the `desync_canary`
restore/replay oracle proves bounded resimulation equality with moving-platform
state included in the hash.

### N3.2b — atomic active-room transaction (landed 2026-07-16)

The active room is restored sim state. When a snapshot's room differs from the
live one, `restore` STAGES the snapshot's room before reconciling, through
`RoomStaging` (`ambition_actors::world::rooms`) — the same canonical
construction a room transition runs: the room-scoped entity sweep, the
active-spec/`RoomGeometry` swap, the moving-platform rebuild, and the
App-installed placement-lowering registry. Staging deliberately performs NO
arrival/clock/cooldown resets — the snapshot blobs applied afterwards are the
authority for everything registered.

How each required property is met:

1. **Preflight before mutation.** `RoomStaging::prepare` is mutation-free (it
   resolves the target room and clones every construction service); it runs
   with the other preflights (snapshot well-formedness, identity uniqueness,
   dynamic-reconstruction, standalone codec probes) before `apply`. Every
   refusal — `RoomNotStageable` (unknown room / missing service),
   `CrossRoomBoundary` (room-presence mismatch), `MalformedSnapshot`,
   `UnsupportedDynamicReconstruction`, standalone `DecodeFailed` — leaves the
   live room untouched (gated by
   `an_unstageable_room_refuses_with_the_world_untouched`).
2. **Canonical construction.** Staging shares `spawn_room_feature_entities_with_registry`,
   `moving_platforms_for_room`/`spawn_moving_platforms`, and the physics
   retirement path with session setup, transition, and sandbox reset. The
   staged bodies then receive identity through the SAME `ensure_sim_id` pass
   the sim runs — executed synchronously by `restore`, never a restore-only
   recipe.
3. **Reconciliation against the right `RoomSpec`.** After staging, survivors
   patch, the target room's authored entities rebuild + patch, and identities
   the snapshot never knew (including staged-but-then-dead ones) despawn.
   `RestoreReport::staged_room` names the staged room.
4. **Cross-room rewind/replay equality.** `portal_lab` — whose 60-tick window
   spans a room transition — is in the desync canary's `CLEAN` roster: restore
   reproduces the registered hash bit for bit, a re-taken snapshot equals the
   restored one, and the replayed future matches the abandoned one. The DIRTY
   ledger emptied and was deleted.

Two enabling invariants landed with it, found by the gate:

- **Identity is synchronous with the tick that spawns a body.** The
  `ensure_sim_id`/`mint_spawned_sim_ids` pair runs at the sim head AND after
  the last in-tick spawner, so a boundary snapshot never captures an authored
  body without identity.
- **Read-model syncs own no reset.** `sync_moving_platform` carried a
  `Local`-cached room-change reset that clobbered restored platform state with
  authored starts; platform state is now installed exclusively by construction
  (session setup, transition, reset, hot-reload, staging), and the sync is a
  pure resource→visual mirror.

Remaining honest boundaries, unchanged in kind: a snapshot holding a
dynamically-spawned entity that will not survive into reconciliation (for a
staged restore this includes room-scoped `spawned(..)` entities — a projectile
in flight at the snapshot tick) refuses with
`UnsupportedDynamicReconstruction` until spawn recipes land; a room-presence
mismatch refuses as `CrossRoomBoundary`; restores never span sessions.

## Identity and ordering rules

- Authoritative dynamic entities carry stable simulation identity; raw Bevy `Entity` is not a serialized/network identity.
- Ordered simulation outcomes never depend on hash-map iteration or allocation order.
- Time-sensitive state declares its clock domain; wall-clock time does not drive authoritative simulation.
- Provider/content identity is part of the session contract. A snapshot cannot be restored against a different prepared catalog/world by accident.

## Relationship to other architecture

- [`architecture.md`](architecture.md) defines session/provider/runtime ownership.
- [`decisions-2026-07-16.md`](decisions-2026-07-16.md) records the two required session gates.
- [`../tracks.md`](../tracks.md) orders placement unification before the broader N3.2/session campaign.
- The accepted `ambition_sim_harness` extraction supplies the reusable reset/step/replay consumer surface.

## Later work

Online rollback, transport selection, prediction policy, cross-platform numeric
contracts, spectator state, and authoritative server modes remain later product
choices. They do not justify premature networking abstractions in current domain
code.
