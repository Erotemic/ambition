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
   `UnsupportedReconstruction`, standalone `DecodeFailed` — leaves the
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

### N3.2b closeout — complete roster, session binding, first spawn recipe (landed 2026-07-16)

The GPT-5.6 closeout review found the staged room INCOMPLETE: occupants created
by `RoomLoaded` consumers (the duel-arena fighters, Mary-O's cronies) were not
part of room construction, so a staged restore bare-spawned their identities
and patched blobs onto hollow entities. Landed corrections:

1. **Room-content staging is part of construction.** Providers/content register
   PURE stagers (`RoomSpec` → `SpawnActorRequest`s) into the App-installed
   `RoomContentStagingRegistry`; `spawn_room_feature_entities_with_registry`
   drains them on the sim side in every path — activation, transition, reset,
   hot-reload, and restore staging (which also runs the canonical request
   applier synchronously). `RoomLoaded` is now a pure notification (resource
   re-arms, presentation); it never creates snapshot-authoritative entities.
   This also fixed a latent determinism hole: the old consumers ran on the
   presentation schedule, so staging timing was frame-rate-relative.
2. **No bare-identity success in a room-backed world.** Restore preflights the
   snapshot roster against the PREDICTED roster — survivors ∪ the target room's
   authored lists ∪ its content-staged ids ∪ dynamic-anchor rows — and refuses
   (`UnsupportedReconstruction`) before mutation for anything outside it. The
   bare respawn survives only for room-less headless fixtures.
3. **Snapshots are session- and prepared-world-bound.** `SimSnapshot.session`
   captures the owning `SessionScopeId`; a mismatch (`SessionMismatch`) is the
   FIRST preflight. `SimSnapshot.world` independently captures the prepared
   provider ids and sorted room roster, so a local scope id reused by a different
   App/world is not sufficient for acceptance (`WorldMismatch`). Gate:
   `a_snapshot_never_restores_across_sessions` (shell_host_lifecycle) — A's
   snapshot into a same-provider, same-room session B refuses with B untouched.
   A future persisted wire format may strengthen the same-build world identity
   with a content fingerprint.
4. **The identity invariant is re-checked after staging** — a content stager
   colliding with an authored placement cannot silently win a map insert.
5. **The projectile family is the first spawn recipe** — the registered kind:
   every component an in-flight projectile carries is a registered row, including
   `RoomScopedEntity` and `SessionScopedEntity` (marker presence and exact scope
   are restored state). `ProjectileSeqCounter` is a registered resource, and
   `ProjectileOwner` (the one `Entity` handle) is declared derived, healed from
   the spawned id's parent by `heal_projectile_owners` beside the identity pair.
   `projectile_gameplay` is a DYNAMIC ANCHOR: a dead projectile rebuilds from
   blobs with its mechanical and lifetime shell intact, so a rollback window may
   span the projectile's whole life without leaking it across a room/session end.
6. **Pending cross-tick messages are restored state.** `SpawnActorRequest` and
   `RoomTransitionRequested` joined the restore-cleared channels (9 total): a
   spawn or door-walk queued in the abandoned future must not replay.
7. **Same-room content-staged deaths rebuild through the coordinated batch.** If
   one snapshot member is absent, restore replays the room's pure content-staging
   requests as one batch before reconciliation. This preserves authored
   cross-member relationships such as the duelists' mutual grudges instead of
   independently bare-spawning one fighter.
8. **Fight-aged mutable state is registered.** Smash-brain reaction history and
   tactical clocks, `ActorDisposition`, the mutable aggression policy cursor,
   and `BodyMelee` now rewind. The temporary per-entry duel diagnostic is gone.

Exit oracles:

- `a_staged_restore_rebuilds_the_duel_roster_completely` stages the complete
  authored duel roster and replays the identical suffix bit for bit;
- `same_room_restore_rebuilds_a_missing_content_staged_batch` spans one
  fighter's same-room death and restores both the roster and bilateral authored
  grudge relationships;
- the projectile dynamic-rebuild test proves room/session lifetime markers are
  restored with the mechanical projectile state.

**Transactionality, stated precisely:** every currently-preflightable refusal
occurs before room mutation — session binding, room resolution, snapshot
well-formedness, identity uniqueness, the predicted-roster check, and
standalone codec probes. Cursor/resolved codec application failures (and the
predicted-roster/construction disagreement) remain a post-mutation
internal-consistency boundary until transactional codecs land; the caller must
discard the world on those.

Remaining honest boundaries: a dead `spawned(..)` entity outside an anchored
family (a summoned minion; a lowering-spawned child like the giant hands under
a staged restore) refuses with `UnsupportedReconstruction` until a domain spawn
recipe lands; a room-presence mismatch refuses as `CrossRoomBoundary`;
`PlayerProjectileState` remains ledgered debt. Mutable relationships represented
as allocator-local `Entity` handles remain domain-derived/authored responsibilities
rather than snapshot bytes; raw entity handles are never serialized.

## Identity and ordering rules

- Authoritative dynamic entities carry stable simulation identity; raw Bevy `Entity` is not a serialized/network identity.
- Ordered simulation outcomes never depend on hash-map iteration or allocation order.
- Time-sensitive state declares its clock domain; wall-clock time does not drive authoritative simulation.
- Provider/content identity is part of the session contract. A snapshot cannot be restored against a different prepared catalog/world by accident.

## Relationship to other architecture

- [`architecture.md`](architecture.md) defines session/provider/runtime ownership.
- [`decisions-2026-07-16.md`](decisions-2026-07-16.md) records the two required session gates.
- [`../tracks.md`](../tracks.md) lists the remaining executable work after the N3.2/session campaign.
- The accepted `ambition_sim_harness` extraction supplies the reusable reset/step/replay consumer surface.

## Later work

Online rollback, transport selection, prediction policy, cross-platform numeric
contracts, spectator state, and authoritative server modes remain later product
choices. They do not justify premature networking abstractions in current domain
code.
