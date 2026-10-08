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
- the GGRS backend in its own crate, `ambition_platformer2d_rollback_ggrs`;
- exact content/schema binding and invalidation;
- real `SyncTestSession` rewind and resimulation over the actual `GgrsSchedule`
  (`game/ambition_app/tests/desync_canary.rs`,
  `gameplay_presentation_ggrs.rs`);
- multi-seat local input through rollback, not seat zero replayed into every
  handle;
- runtime-created rollback entity recreation for covered families;
- external/presentation effects quarantined to the confirmed host-side boundary
  where implemented;
- a confirmed room transition waits for the authorized construction plan and
  rebases to a new frame-zero baseline;
- cross-game shell lifecycle acceptance: one retired game's rollback health
  cannot block another game's room transition, including misordered retirement
  (`shell_host_lifecycle.rs`);
- two players in two live rooms resimulate to the same checksums under a GGRS
  sync test (`two_players_in_two_live_rooms_resimulate_to_the_same_world`).

## Remaining netcode work

### N1 — finish deterministic/runtime-state correctness before transport

Transport should not hide local deterministic defects.
[`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md)
owns the remaining deterministic selection/composition sites and
scenario-populated dynamic-state coverage.

One piece of N1 was netcode's own: the canonical timeline. Decided 2026-10-03
(`Q128`, option (a)): `ambition_time::SimTick` is session-relative. The
session-scope activation sets it to `0`, so two Apps that ran for different
times agree from their first compared frame. N2 must activate the session scope
at the start the peers agree on; that activation is the rebase.

So N1 and N2 are not strictly ordered for this road: a `SyncTestSession` compares
one machine with its own past and cannot see a two-peer disagreement.

### N2 — first real external/P2P session

When Smash or Ambition has an online slice, install a real transport through the
existing session seam. `bevy_matchbox` is a likely candidate. Transport choice
must not change simulation or input ontology. Do not build signaling or
deployment infrastructure only to satisfy this plan.

The P2P road exists without a network
(`ambition_platformer2d_rollback_ggrs::peer`): `start_peer_session` builds a
GGRS P2P session over the socket of a `PeerTransport` and installs it with
`install_rebased_session`, and `loopback_transports` is an in-memory link with
a latency in updates, for two Apps in one process. A transport supplies a
socket for each session generation. Nothing here is signaling or deployment.

**A session generation has its own socket on the link (2026-10-04).** A peer
session ends at a lifecycle commit and the next one starts at frame zero. A new
GGRS endpoint accepts each message until its handshake is complete (its
`remote_magic` is zero until then), so an Input parcel of the old session that
is still on the link would be read as an input of the new timeline.
`PeerTransport::socket_for(generation)` (in memory:
`LoopbackTransport::socket(generation)`) gives the socket of one generation, and
a socket receives only its own generation: a parcel of an older one is dropped,
and a parcel of a later one stays on the link for its socket
(`peer::tests::a_parcel_of_an_older_session_is_not_delivered_to_a_new_endpoint`,
its control `the_same_parcel_is_delivered_to_the_session_it_was_sent_to`, and
`a_parcel_of_the_next_session_waits_for_its_socket`). A real transport owes
the same rule: a generation number in its envelope, or a channel for each
generation.

N2 was the only instrument for two open questions:

- the unchecksummed float rows (S7 in
  [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md)).
  Measured 2026-10-03 by `game/ambition_app/tests/two_peers.rs` over the
  in-memory link: two peers agree on every probed row at every confirmed frame,
  the float rows by value, in eight rooms that together carry every float row
  with a production writer.
- whether the session rebase at a room crossing fits a remote peer's rollback
  window. Answered 2026-10-04: it does not have to fit. Under a peer session a
  crossing freezes the simulation, each peer commits alone on the frozen world,
  and each peer starts the next generation of its session at frame zero, so no
  rollback crosses the commit. The cost is the hold: 23 to 43 updates measured
  at a link latency of 3 updates, most of it the GGRS handshake (Q155). See the
  "Remote peers" row of
  [`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md).

**A preparation that fails on one peer had no rule (measured 2026-10-04,
Q156; the default below now ends it).** Each peer prepares the room of an operation on its own machine, in
`Update`, and commits alone. When one machine's preparation fails, the other
commits and waits in the handshake of a session that never starts, and the
machine that failed stays held. No error is reported. When each machine fails,
both stay held. The measurement is in Q156 of
[`awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md).

The engineering half is built (2026-10-05): **the verdict of each peer
travels in the peer input.** `PeerInput` is a `ControlFrame` and a
`PeerVerdict` (the frame that recorded the operation, and `NotYet`,
`Prepared` or `Failed`). `decide_this_peers_verdict` writes it in
`ReadInputs`: `Prepared` is the commit's own test (`authorized_plan`), and
`Failed` comes from `RoomTransitionLoadState::failed_to_prepare`: a host
with no loading screen retires a failed transaction at once, so the state
keeps its intent (`last_failure`) until a transaction for it is authorized.
`record_confirmed_verdicts` (in `GgrsSchedule`, outside the frozen root)
keeps the newest CONFIRMED verdict of each handle in `PeerVerdicts`, which
frame zero of each session resets. The peer arm of the commit then also needs
each handle to have said `Prepared` for the operation. The reasons for the
input: it keeps one protocol (a second message beside GGRS is a second
ordering to reason about), and an input is confirmed with its frame. The
price is one more link delay inside the freeze, and 8 more bytes for each
handle and frame. Witness:
`two_peers::a_peer_does_not_commit_a_crossing_the_other_peer_could_not_prepare`
(poison: the commit with no verdict check commits alone, generation 1 on one
peer and 0 on the other).

**A `Failed` ends the operation on each peer (2026-10-05, the default in force
until Q156 is ruled).** `end_an_operation_a_peer_could_not_prepare` (in
`GgrsSchedule`, before the gameplay root) retracts the waiting operation when
a handle's input of the frame says `Failed` for it. It reads the frame's
inputs, not the confirmed record: each peer reads the same input for a frame
(a wrong prediction is corrected by a rollback), so each peer ends the
operation on the same frame and simulates again from it. Ended from the
confirmed record, which arrives at a different frame on each peer, the
peers desynced from frame 69 (the poison of the witness). The rest follows
from the slot: the transaction is cancelled, and a checkpoint restore
publishes `Cancelled`. A door a peer could not prepare does not open; a new
attempt clears the last failure, so a press of the door tries again.

⚠ The verdict made two equal machines commit at ONE frame (measured: 36 and
36), because each waits one link delay for the other's verdict. The freeze is
still needed when one machine prepares later than the other: then the faster
peer commits a link delay after the slower one's verdict arrives, and the
slower peer commits at once. The door tests of `two_peers.rs` run with Bob's
machine 6 updates slower (`bobs_machine_is_slower`) so that the no-freeze
poison still starts two worlds.

A plan must also be lowered from a durable horizon that the peers agree on.
Today `begin_room_transition_load_system` lowers the mints of a door from this
machine's save (`minted_baseline_from_save`), so two peers with different
saves build different plans, and no check sees it. That is a divergence, not a
policy question (read 2026-10-04, not measured).

**The start is a new timeline, and that choice is made.** Each peer calls
`start_peer_session` at the same point of the same world, so the world is frame
zero and the carrier order is rebased (`install_rebased_session`, the frame-zero
half of `install_rebased_sync_test_session`). `install_session`, which does not
rebase, stays the seam for a session that continues a timeline the peers
already share.

**The first frame of a peer timeline carries no input.** With no frame
confirmed, GGRS lets a session run `max_prediction` frames past frame 0 and
saves `max_prediction + 1` frames, while bevy_ggrs keeps `max_prediction`
snapshots. The peer that synchronized first (ten updates earlier, measured)
runs that far ahead and evicts frame 0, and the first wrong prediction of a
remote's frame-0 input rolls back to a frame with no snapshot: bevy_ggrs
panics. GGRS predicts that input as the default, so `publish_local_inputs`
publishes the default for every local seat at frame 0 of a session it does not
own. The defect is upstream (the pinned bevy_ggrs rev, which is its current
main); the rule can go when the snapshot depth covers the start.

**A desync reaches the session's health.** GGRS reports a peer checksum
mismatch only as a queued event. `record_peer_events` drains them each update
and records a desync on `ActiveRollbackAuthority` as a sync-test mismatch is
recorded, so `session_health` fails with the frame.

### N3 — content/schema negotiation

Before external peers begin play, negotiate exact prepared-content identity,
the rollback schema fingerprint, and the input payload identity. A peer with
mismatched simulation content fails before speculative play.

**State identity.** The runtime dump (`schema_dump()`, recorded in
`game/ambition_app/tests/rollback_schema_baseline.txt`) owns the schema. A
source scan cannot own it, because a registration is a runtime call with no
closed set of spellings.

- `the-peer-visible-schema-may-not-move-without-the-version`: if the set of rows
  whose kind answers `feeds_peer_checksum()` changes, the version on the dump's
  first line changes in the same commit. `detail` is kept where it
  distinguishes rows of the same kind and dropped where the kind already implies
  it.
- `the_shipped_app_registers_the_same_schema_as_the_sandbox`: the baseline is
  recorded from `Platformer2dSimHarness`, and the shipped `build_visible_app`
  registers the same dump (with an anti-vacuity floor).
- Determinism of the dump comes from its container (`entries` is a `BTreeMap`).
  A `HashMap` there would make the fingerprint vary per process.
- **A local instrument is not peer identity.** `RollbackEntryKind::MessageClearInstrument`
  answers `in_peer_schema_identity() == false`, and `schema_dump()` filters on
  it. The causal recorder's channels register through
  `clear_instrument_message_on_rollback`, so `--features causal` and the default
  build advertise one fingerprint. The instrument is still cleared on rewind.
  `the_causal_instrument_is_registered_and_outside_the_peer_schema` first
  requires the channels to be present in `deterministic_dump`.
- Open (`Q122` ruled): the fingerprint hashes prose `detail`. Split each row's
  `detail` into the mechanical facts the fingerprint hashes and the explanation
  it does not. Do not just drop `detail`. Tracked as ID-PEER in
  [`../queue.md`](../queue.md).

**Input identity.** See "The input payload two peers exchange".

**Still owed:** no peer handshake reads the dump, its version, or the input
shape. That waits on N2.

The [extension contract](extension-state-and-execution.md) extends this
compatibility manifest with module code, port versions, extension schema digests
and numeric/runtime execution policy. App-local epochs are stale-plan stamps,
not peer identities. The content digest stays the mechanical root. Unknown or
mismatched required inputs refuse before speculative play.

A remote session pins its mechanical generation. Local authoring reload uses
the supported reconstruction/rebase road and keeps unhealthy same-session
diagnostics. Coordinated mid-session online code migration needs N4 and an
explicit requirement; it does not block local data iteration.

### N4 — coordinated lifecycle barrier

The local rollback host commits a confirmed room transition and rebases at once,
because it has no remote corrected-input frontier. A real external host needs a
peer-coordinated barrier around the same construction/rebase seam. The barrier
answers:

- which lifecycle intent/frame is committed;
- that every peer confirms the required input/content horizon;
- that every peer has the same authorized construction plan and content identity;
- how corrected input arriving before the barrier cancels or replaces a pending
  intent;
- when old rollback history is discarded and the new frame-zero baseline
  installed.

This is an authorization protocol around canonical construction, not a second
room constructor. With several live rooms, a crossing in one room rebases the
whole session; the barrier must preserve other rooms' state under that
baseline.

### N5 — disconnect/reconnect/spectator/deployment policy

Defer until the first two-peer deterministic lifecycle path is green. These are
product and network-service concerns.

## The input payload two peers exchange

`AmbitionGgrsConfig = GgrsConfig<PeerInput>`, and `PeerInput` is a
`ControlFrame` followed by the peer's verdict (78 bytes for each handle at wire
identity 4). `ControlFrame` is `derived` (rebuilt from the input stream, not snapshotted), so the
rollback dump, the fingerprint and `rollback_codec_shape.txt` do not describe its
fields. `INPUT_STREAM_VERSION` versions recorded replay files and exempts added
fields by design; it does not cover the peer payload.

- **Identity and ratchet.** `CONTROL_FRAME_WIRE_IDENTITY` names the shape.
  `the-peer-input-payload-may-not-move-without-its-identity`
  (`scripts/check_absence_contracts.py`) ratchets the field list in declaration
  order plus nested enum variants with payloads, `PeerInput` and its verdict
  included. Order is part of the shape, because bincode is positional. An
  unrecognized field type raises. The baseline records the shape of EACH
  identity, and the live identity must be one of them. ⛔ From 2026-09-28 to
  2026-10-04 the baseline held identity 2 while the source said 3, and a
  different identity was the exemption, so every shape read green (measured by
  poison); `test_a_bumped_identity_with_no_recorded_shape_is_caught`.
- **Bytes.** `control_frame.rs`'s `the_bytes_two_peers_exchange` pins the exact
  bincode bytes of a deliberately legible frame (a default frame is all zeros),
  and `peer_input.rs`'s module of the same name pins the whole `PeerInput`.
  The bytes cannot see a `bool`/`u8` swap; the field census can. They
  complement each other.
- **Fixed width is Ambition's contract, not GGRS's.** `ggrs` 0.13
  `InputBytes::from_inputs` concatenates each local player's encoding with no
  per-player length, and `to_player_inputs` divides the total by the player
  count. A `String`, `Vec`, `Option` or data-carrying enum makes one player's
  width depend on what they pressed, and the next player's slice starts mid-frame.
  A single-player payload is immune; local multiplayer sharing one packet is
  where it would appear. `refuse_variable_width` refuses variable-width and
  unrecognized types at every level `ControlFrame` reaches. A version bump buys a
  different fixed-width protocol, not a variable-width one.
- **`#[serde(default)]` does nothing on the wire.** Bincode carries no field
  names, so a field is never missing. The attribute serves replay compatibility
  only.
- Adding a field already fails to compile (`ControlFrame::merge_sample` builds
  an exhaustive literal and each field declares LEVEL or EDGE). A reorder, a
  width change, or an enum gaining a variant ahead of an existing one compiles
  and moves the bytes; the ratchet catches those.
- Not established: whether a mismatched field set fails loudly or decodes into
  garbage. No layer compares two builds' input shape yet. A negotiated input
  version is not owed until N2.

## Identity rules

- `RollbackId` is GGRS frame-history identity.
- `SimId` is Ambition semantic simulation identity. With several live rooms, a
  live occurrence is (`SimId`, live room).
- `SessionScopeId` owns one gameplay activation.
- `RollbackTimelineGeneration` distinguishes successive rollback timelines,
  including rebases, across the process.

Do not use one as a substitute for another.

**No host-local count in canonical identity.** A value that counts something
this process did (activations, sessions, sim steps, load transactions, content
epochs) may name a thing for cleanup, staleness rejection and correlation. It
is never an input to authoritative RNG, deterministic construction provenance,
rollback identity, contact/projectile identity, or a peer checksum. Acceptance:
two Apps with different prior local history, entering the same peer-agreed
session, reach the same canonical identities and rollback-visible state.

No type census sees this class, because the defect is in a value's provenance,
not its type. Hold each road with a value census over a built world across two
local histories (`two_local_histories_name_every_simulated_entity_identically`,
which first asserts the local tokens differ), or by shape where no argument can
carry a local term (`SimId::match_spawn`). Test the road the shipped composition
calls, not the minting crate's helper. The closed roads are listed in the
ID-PEER receipt in [`../queue.md`](../queue.md#receipts);
`game/ambition_app/tests/id_peer_audit.rs` holds the arms.

Standing prohibitions:

- Keep the session term in the rendered `TransactionId`. Only its peer
  projection drops it, because the construction scope's gather filter and A10's
  candidate-vs-live separation read the local stamp.
- Do not project `TransactionId` to `{room}` alone. The peer content term
  (`PeerContentIdentity`) must stay in the projection.
- Do not replace a raw `SessionScopeId` with the nearest canonical-looking
  value. Before Q128, `SimTick` looked canonical and was host-local.
- Do not mint a local `PeerSessionIdentity`. A peer session identity comes only
  from a session handshake.
- An id that crosses to a peer is a function of content or of a canonically
  sorted set, never of insertion or allocation order (`RollbackOrdered`, an
  `Entity` index).
- `id_peer_audit` censuses type names, so it cannot see a provenance defect.
  Hold provenance with value-level arms in the crate that mints the identity.
  `scripts/one_owner_per_canonical_identity.py` keeps each constant identity to
  one production mint.

## What GGRS actually folds into the peer checksum

From the pinned `bevy_ggrs` rev:

| plugin | what it folds |
| --- | --- |
| `ComponentChecksumPlugin<C>` | per carrier, `hash(RollbackOrdered.order(id), projection(value))`, XORed, then hashed once more |
| `ResourceChecksumPlugin<R>` | `hash(value)` |
| `EntityChecksumPlugin` | `hash(active carrier count, RollbackOrdered.len())` |

`ChecksumPlugin` XORs every `ChecksumPart`. XOR cancels a value that appears an
even number of times; that blind spot is upstream's.

`RollbackOrdered` is host-local: it assigns each `RollbackId` an index on first
`Rollback`, keeps every index ever handed out, and is snapshotted. So process
history reaches the checksum twice. `rebase_rollback_carrier_order` runs where a
session declares frame zero and re-orders the live population by
(`SimId`, live room), so the order is a fact about the session.

A projection census (`RollbackChecksumProbes`) ignores which entity carried a
value, so it cannot see this. Only an arm that reads `ChecksumPart` from a
running session measures the checksum:
`two_local_histories_compute_the_same_ggrs_component_checksums`.

## Confirmed effects

Irreversible host effects are not emitted because a speculative tick ran.
Reconstructable audio/VFX may replay from confirmed simulation state. Persistence
writes, analytics, network-side effects and file output require an explicit
confirmation boundary. Developer tracing may keep historical-resimulation
observations as diagnostics only when they cannot feed authoritative behavior.

Confirmed in-process effect release is not a durable exactly-once protocol. When
a real external transport or persistent side effect is added, name the
idempotency scope, restart recovery, duplicate delivery and acknowledgement
owner.

## Verification

Before online transport is considered healthy:

1. local `SyncTestSession` repeatedly rewinds/resimulates representative
   gameplay without checksum divergence;
2. multiple seats keep independent input streams across rewind;
3. runtime-created authoritative populations used in play survive recreation
   and deterministic composition;
4. session retirement/startup cannot transfer rollback authority across
   `SessionScopeId`;
5. content/schema mismatch refuses before play;
6. a real two-peer host proves corrected input, confirmation, and one
   coordinated lifecycle/rebase.

## Non-goals

- custom snapshot/history machinery beside GGRS;
- persistence implemented as rollback snapshots;
- multiplayer-specific actor/control ontology;
- Matchbox/deployment work before a product customer exists;
- treating a two-seat local sync-test session as proof of a two-peer protocol;
- independent per-room rollback clocks.

## Wire identity across refactors

Moving a checkpoint progress type or installer keeps its wire identity,
baseline/reset lifetime and confirmed admission phase. A crate or module path is
not an instruction to change a wire ID. A new serialization schema or executable
build is a separate compatibility change.

[Generation/reload](content-generation-and-reload.md) is local development
reconstruction with a fresh timeline; active remote sessions pin their
generation. A code/schema match alone does not authorize old history to run under
new content.
