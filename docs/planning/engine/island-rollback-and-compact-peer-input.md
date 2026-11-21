# Simulation islands and compact peer input

**Status:** compact GGRS input is done (NET-INPUT-WIDTH, 2026-11-10: 80 bytes per handle at wire identity 9). **Direction set 2026-11-10** ([decision](../maintainer-decisions.md)): the Ambition metroidvania moves away from App-wide rollback toward a hybrid model; rollback stays inside a room boundary (one live room instance is the island) while that is easy to maintain, and its room-level cost is measured before more is built on it. SIMULATION-ISLANDS is selected work under that direction; ISL-1 is [measured](#isl-1-result-2026-11-10-what-has-a-room). The Smash demo keeps its single rollback domain.
**Execution:** [SIMULATION-ISLANDS](../queue.md#simulation-islands--room-level-rollback-inside-a-hybrid-world) (selected 2026-11-10); [NET-AUTHORITY-EXPERIMENT](../queue.md#net-authority-experiment--compare-hosted-world-replication-with-ggrs) builds the hosted half; [NET-INPUT-WIDTH](../queue.md#net-input-width--remove-idle-room-consensus-from-every-frame) (done) compacted the current GGRS hot path.
**Owners:** [netcode](netcode.md), [peer room commits](peer-room-commit-protocol.md), [open-world residency](open-world-runtime-and-residency.md), [simulation authority](simulation-authority-and-determinism.md), [multiplayer/multiview](multiplayer-and-multiview.md).

## Why this work was selected, and what it measured

Until 2026-11-10 `ambition_platformer2d_rollback_ggrs/src/peer_input.rs` serialized a fixed-width `PeerInput` of **168 bytes per player per frame** at wire identity 8: **72 bytes** of `ControlFrame` plus **96 bytes** of `PeerVerdict` (operation, two content identities, plan digest, verdict), paid even when no room operation was pending. Wire identity 9 replaces the verdict with an 8-byte `PeerVerdictToken`, so a `PeerInput` is **80 bytes**. See [the result below](#result-2026-11-10).

Separately, multiple live rooms already have room-scoped spatial/collision behavior, but the selected GGRS path still snapshots/resimulates an **App-wide timeline**. November 4's ordinary peer-commit fix eliminated the old global *hold/rebase* for travel; it did **not** implement island-specific rollback or interest-scoped input exchange. Compact input is a near-term performance/correctness task **independent of** the larger rollback partition.

## Candidate topology and physical boundary

The `activeArea`/loading-zone distinction remains a useful physical-world boundary regardless of whether the region is an independently rollbackable island or a host-authoritative simulation/interest domain. The following describes the **GGRS-island branch**, not the mandatory network model for every game.

1. **Default island = one runtime `activeArea` instance**, not one LDtk level. Seamlessly joined LDtk levels in the same active area share continuous collision, projectiles and a speculative physics/rollback domain. Two separately instantiated rooms with the same authored area name must *not* share an island merely because their template identifiers match. Island identity includes live instance identity.
2. **Loading zones cross islands by confirmed, idempotent custody transfer.** The source relinquishes the body and attached inventory exactly once; the destination admits it exactly once from an agreed committed state. Transfer does not imply continuous cross-boundary collision, projectiles, grapples or rollback coupling. Two unrelated islands keep advancing during loading and preparation. Existing room-construction, identity, save and presentation contracts remain authoritative.
3. **Live physical portals are intra-island.** Both apertures of a real, continuously interactive portal must resolve within the same island. This is the scope of [canonical similarity portal parts](portal-parts-similarity-and-chirality.md), including scaled and reflected limbs and projectiles. Do not implement live cross-island portals, dynamic merging, causal bubbles, or view-only portal substitutes as dependencies of this campaign. A loading-zone transport remains a distinct mechanic, not a secretly nonphysical 'live portal'.
4. **Persistent world facts cross an explicit commit boundary.** A speculative island rewind cannot revoke another island's already-committed fact. Cross-island rewards, quest consequences, inventory transfers, entity custody and ordering use identified confirmed transactions, not unchecked cross-island ECS mutation.
5. **Clock, rendering, transport, residency and rollback are distinct.** Islands may initially share one logical tick counter without sharing history. A seat may move between islands, multiple local seats may coexist in one island, and an island may have zero visible views. `activeArea` is an *authoring/topology default*, not a license to assume physics is already partitioned.

## NET-INPUT-WIDTH: reduce serialized per-frame input first

**NPI-1 — Baseline and field census.** Rerun the 168-byte serialization test and inspect all producers/consumers of `PeerVerdict`, `ThisPeersVerdict`, `PeerVerdicts`, the ordinary frame-agreement path, and checkpoint/rebuild barriers. Measure actual P2P traffic under idle play, concurrent two-seat inputs, room crossings and induced latency/rollback; report transport bytes separately from bincode input length. Preserve the current golden-wire and multi-handle fixed-width tests as the baseline.

**NPI-2 — Separate rare agreement from hot-path controls.** Agree on a *frame-stamped*, operation-identified room-preparation protocol that moves full content hashes, plan identity and preparation verdicts out of every frame's `GgrsConfig::Input`. A reliable out-of-band exchange is an option, **not itself a rollback commit**: it must be deterministically admitted on the same simulation frame by the relevant peers, preserve cancellation/replacement on corrected inputs, refuse stale/mismatched plans, and never let local host readiness mutate speculative gameplay asynchronously. If a compact in-band acknowledgment/token is necessary, it remains fixed-width and carries no repeated 32-byte hashes. Ordinary crossings stay free of global holds/rebases; checkpoint and explicit whole-session rebuilds retain their distinct confirmed barrier.

**NPI-3 — Wire migration and measurement.** Bump `CONTROL_FRAME_WIRE_IDENTITY` and update `check_absence_contracts.py`'s recorded shape, the pinned input bytes and the exact-content/session handshake together. Verify all `PreparationVerdict` states and two-local-player packing. The target for the *normal frame* is **at most 80 serialized bytes per player**, with 72 bytes possible if no permanent compact token is needed. Reject a change that merely compresses repeated hashes while leaving the hot-path scaling behavior intact. Measure end-to-end traffic and rollback latency; do not equate per-frame size with datagram size, retransmissions or bandwidth.

**Acceptance:** nominal `PeerInput` no longer embeds the 96-byte `PeerVerdict`; fixed-width serialization still works across all handles, and source tests pin the new identity and bytes. Both peers agree on ordinary crossing application frames under delay, input correction, replacement, failure and concurrent preparation; plan/content mismatch refuses without duplicate arrival or custody. Checkpoint/rebuild barrier remains safe. Before/after input and transport byte measurements are recorded; no host rebuild/room hold is introduced solely by the optimization.

### Result (2026-11-10)

**Done at wire identity 9.** `PeerInput` is the 72-byte `ControlFrame` plus one 8-byte `PeerVerdictToken` per handle: **80 bytes**, pinned by `peer_input.rs`'s `the_bytes_a_peer_decodes_are_pinned` and `each_input_has_the_same_width`; `two_local_players_pack_and_divide_back` packs two handles into one GGRS payload and divides them back. `check_absence_contracts.py` records the identity-9 shape (`PeerInput::verdict: PeerVerdictToken`, `PeerVerdictToken::word: u64`).

- **Why an in-band token, not a side channel.** Agreement must stay frame-stamped and replay-stable: a crossing applies on the frame whose inputs say every handle prepared, and a corrected input must cancel it. GGRS already gives that to anything in the input. A side channel would need its own deterministic admission, and the measurement below shows no bandwidth to win.
- **Layout.** Bits 0..32 are a digest of the operation (recording frame, intent checksum, preparation checksum); 32..62 a digest of the frozen plan (both content identities and the plan data digest; zero unless `Prepared`, never zero when `Prepared`); 62..64 the verdict. `NotYet` is the zero word, so an idle input is 8 zero bytes.
- **The receiver never decodes a plan.** It compares each handle's token with the token its *own* operation and plan would produce (`PeerVerdicts::agreed`, `each_prepared`). A stale or replaced operation is read as `NotYet` unless its 32-bit digest collides with the current one; two different plans for one operation agree only if their 30-bit digests collide (about one in 10⁹ per mismatch). A plan mismatch needs different content or a desynchronised construction in the first place; the session handshake already refuses peers whose full content identity differs (`peer_contract_mismatch_refuses_simulation_on_both_peers`).
- **Kept.** The commit still `expect`s the exact retained plan through replay and checks the local plan against the agreed tokens; checkpoint/rebuild barriers read confirmed tokens only. All 35 `two_peers` agreement tests pass, including delay, correction, replacement, failure, concurrent preparation and `peers_refuse_different_room_plans_under_the_same_content_identity`.

**Measured link traffic** (`two_peers::how_many_bytes_the_link_carries_a_frame`, ignored by default; run with `--ignored --nocapture`). `LoopbackTransport::sent()` counts every GGRS message and its bincode size. Bytes per simulated frame per peer, Alice / Bob:

| Scene | Identity 8 (168-byte input) | Identity 9 (80-byte input) | Change |
|---|---|---|---|
| Steady play, 240 frames | 111.4 / 113.8 | 110.9 / 113.0 | −0.5% |
| Two crossings, 220 frames | 152.5 / 155.4 | 146.1 / 149.9 | −4% |

The input shrank by 52%, and the link traffic barely moved. GGRS XORs each input against the last acknowledged one and run-length encodes the result, so an unchanged verdict already cost almost nothing. The rest is about 3.9 messages a frame at about 28 bytes each; how much of that is headers, acknowledgements and quality reports was not split out. The crossing run also rolled back 405 times instead of 446; that was not investigated. **For NET-AUTHORITY-EXPERIMENT:** input width is not what drives this backend's bandwidth, so a comparison should measure message count and rollback work, not input bytes.

## SIMULATION-ISLANDS: independently rewindable active areas

**Selected 2026-11-10, inside a hybrid model.** The world outside a room (the save, quests, NPC lives, travel between rooms) moves toward hosted authority ([experiment](hosted-world-networking-experiment.md)); speculative rollback stays inside one live room while that is cheap. ISL-2 and ISL-4 below were written for distributed GGRS histories; read them as "a rollback history per room", whichever transport carries its inputs. Each step reports its cost against the App-wide baseline, because the decision to keep room-level rollback is a measured one.

**ISL-1 — Scope and causal census.** Inventory every rollback-registered resource, event, entity, physics query, input owner and confirmed-effect producer that can affect two active areas. Classify as island-owned, committed-world-owned, or presentation/host-owned. Demonstrate that cross-area collision, projectile contact, timed environment effects, and direct writes cannot silently bypass an island boundary. Keep the existing `activeArea` level stitching intact, including continuous projectile transit across a seam within one composed runtime room.

**ISL-2 — Partition history, not just visibility.** Implement one speculative rollback authority/history per live island instance, with participant membership and content/schema identity scoped appropriately. Evaluate feasible Bevy/GGRS scheduling and snapshot boundaries; *do not* label a per-room filter on one App-wide `GgrsSchedule` independent rollback. An island can rewind/resimulate while a different populated island advances and retains identical history, side-effect confirmation and committed facts. A bounded Smash arena may continue using a single GGRS rollback domain; a non-rollback game does not inherit this backend.

**ISL-3 — Committed loading-zone handoff.** Establish an operation identity, source confirmation horizon, destination preparation, transfer order, body/equipment/item custody, and exactly-once arrival. Explicitly handle changed intent, death, disconnect, checkpoint restores and failed preparation without stranded or duplicated entities. Moving the participant changes island input membership only at the agreed point. Retain distinct identity for simultaneous instances of the same authored active area. No unrelated island pauses, reloads, or has its rollback generation replaced because of that crossing.

**ISL-4 — Scope peer traffic and demonstrate scaling.** For a selected rollback topology, exchange the high-frequency input histories only among peers participating in the affected island; low-frequency authoritative world transactions still reach the necessary participants. Preserve multiple local seats, view independence and session/world ownership. A host or save owner may need to retain state for many islands, but need not replay their speculative physics as one monolith. Test two populated islands, two peers in different areas, then two peers rejoining one area. Measure per-island input subscriptions, actual transport bytes, rollback CPU/memory and recovery cost against the existing whole-App baseline. Do not imply that island-scoped GGRS alone supplies late-join state replication, remote authoritative snapshots or automatic host migration.

**Acceptance:** two distinct active areas and at least one stitched multi-LDtk active area pass continuous-versus-isolated physics witnesses; rewinding one populated island cannot rewind another or revoke committed world facts. A local or peer-driven loading-zone crossing transfers a controlled body, equipment and persistence exactly once and the destination becomes the only authority. A live portal within one island still supports continuous projectile/body interaction; authoring a live portal whose endpoints lie in different islands yields a clear deterministic refusal. Per-island P2P input subscriptions demonstrably omit unrelated island input streams. Whole-session checkpoint and forced rebuild remain separately correct, with measured cost rather than asserted zero overhead.

## ISL-1 result (2026-11-10): what has a room

Fixture: `game/ambition_app/tests/room_rollback_census.rs`,
`probe_which_rollback_state_belongs_to_one_room` (ignored; a measurement). Run:

```
cargo test -p ambition_app --test app_it --no-run
RUSTC_BOOTSTRAP=1 target/debug/deps/app_it-<hash> room_rollback_census --ignored --nocapture --test-threads=1
```

Scene: two live rooms. Alice crossed to `central_hub_complex`, and Bob stayed
in `switch_lab`. The census runs once on a plain host and once under a
sync-test rollback session (the control: there `bevy_ggrs::Rollback` marks
exactly what the snapshot takes). Population: every entity carrying a
component of a registered rollback row (298 component rows at schema 440),
grouped by `InRoomInstance`.

**Entities: the snapshot already partitions by room.**

| Where | Entities |
|---|---|
| hub (Alice) | 22 |
| `switch_lab` (Bob) | 2 |
| no room | 4 |

The control agrees: under the rollback session 28 entities carry `Rollback`,
and none carries it without a registered row. Of the four roomless entities, two
are the live room roots (each *is* its room: `root.room_instance` names it),
one is the session root (25 rows of session and checkpoint bookkeeping:
custody and occurrence baselines, checkpoint operations, party travel, room
preparation), and one is a boss encounter (`encounter.*`, session-scoped). Of
the 298 component rows, 92 occur only on room entities, 30 only on the four
roomless ones, 4 on both (`entity.sim_id`, `entity.name`, `scope.session`,
`body.sim_id_counter`: identity, which travels with its entity), and **172 on
no entity in this scene**.

**Cost: the snapshot is paid per registered type, not per room or body.** One
`SaveWorld` run under the session takes **16.1 ms over 1,201 systems for 28
entities** (debug build). That agrees with NA-1 (16–17 ms with 2 bodies or 18).
Almost all of it is per-type dispatch, so room-level rollback built from the
same per-type `bevy_ggrs` systems would pay that fixed cost **once per room
history per simulated frame**: room-level rollback is only cheap with a snapshot
whose cost follows the room's entities. The 172 empty rows are the measure of
that waste today (58% of the component rows in an ordinary two-room scene).

**Resources and messages: where rooms are coupled.** The 46 rollback resources
have no entity, so the census cannot place them. From their owners:

| Owner class | Resources | What room-level rollback needs |
|---|---|---|
| Keyed by live room already | `BaseGravity` (turned rooms), `PortalFrameHistory` (per room and channel), `GatePortalPhases` (per portal), Mary-O `BrokenBricks` / `SpentPowerBlocks`, Sanic `SpentMonitors` (`PerLiveRoom`) | Partition by key; no design change. |
| Keyed by body or participant | `PendingPlayerHitEvents`, `PossessionState`, `SlotInteractionState` (and their entity mappings) | Partition by the body's or participant's room. |
| Shared world clock | `SimTick`, `WorldTime`, `DayClock`, `ClockState` (its tick), `RegimePolicy`, `RequestedClockScale` | One logical clock (ADR 0037); a room's rewind reads it and must not write it. |
| **Couplings: one room's event reaches every room** | `ImpactHitstop` (a connect, a death beat or a boss beat anywhere freezes the shared clock), the scale in `ClockState` (requested from Player 1's body: their hitstop or bullet-time slows every room), `ProjectileSeqCounter` (one projectile id sequence for all rooms), `ActiveConversation`, `ActiveCutscene` / `CutsceneSkipHold` (one each per world) | Must become per room (or per participant) before a room rewinds alone. |
| Committed world (hosted authority) | `AmbitionGameSave`, `SaveRestored`, `OwnedItems`, `QuestRegistry`, `LifeLedger`, `AuthoredOccurrences`, the four `*SinceCheckpoint` deltas, `GateNetworkState`, `PendingLifecycleCommit`, `FactionRelations`, `FriendlyFire` | Written only by committed transactions out of a room (principle 4 above); not rewound by a room's history. |
| Single-arena (Smash) | `ActiveMatch`, `ActiveRoundScope`, `LiveMatchTicks`, `SessionMatchOrdinal`, `StocksMatchSettled`, `SuddenDeathEntered`, `VersusMatch` | None: Smash keeps one rollback domain. |
| Single-room demo content | `CutRopeHeavyObjectCycle`, `PendingCutRopeRoomReplay` | None while the demo has one room. |

The 91 `message-clear` rows are App-wide message buffers, cleared on a rewind.
A room rewinding alone may clear only the messages its own room wrote, so a
message that crosses a frame boundary needs a room (or a body that has one),
or must be drained within the frame it is written in.

**What this says.** The entity half of the state has a room today, with no
design change. The cost of keeping rollback at room level is decided by the
snapshot mechanism, not by the state: the per-type `SaveWorld` costs the same
for 28 entities as for a world, so a history per room on that mechanism
multiplies it by the number of live rooms. The design work is the five
coupling resources, the committed-world writes and the message buffers.

### ISL-2a result (2026-11-10): what a room snapshot costs

`crates/ambition_platformer2d_runtime/src/rollback/room_snapshot.rs` keeps, for
every registered component row, a type-erased save and restore that touch only
the entities they are given (`RoomSnapshotRows`). Both registrars record it:
the `bevy_ggrs` one on a rollback host (all 298 component rows) and the schema
one on a hosted world. A room snapshot visits only the rows its entities carry. The
census fixture times it beside `SaveWorld` in the same two-room scene, after a
premise check that a restore puts a moved body back.

| Snapshot | Entities | Rows visited | Save | Restore |
|---|---|---|---|---|
| App-wide `SaveWorld` (1,201 systems) | 28 | all | 16.3 ms | — |
| Room: hub (Alice) | 22 | 95 | 1.35 ms | 1.26 ms |
| Room: `switch_lab` (Bob) | 2 | 72 | 0.31 ms | 0.31 ms |

Debug build, one run; compare ratios. The room snapshot holds **no resources
and no checksum**, and `SaveWorld` holds both, so the ratio overstates the
saving. Even so, both rooms together cost about a tenth of one App-wide save,
and the quiet room's cost follows its two entities. **Keeping rollback at room
level is affordable on this mechanism and is not affordable on the per-type
one.** What it does not do yet: re-create an entity despawned after the save
or despawn one spawned after it, map entity references, save the room's
slices of the keyed resources, or checksum.

### ISL-2b, first witness (2026-11-10): a room rewinds alone

`game/ambition_app/tests/a_room_rewinds_alone.rs`. A hosted world (one App, no
rollback session) with Alice in the hub and Bob in `switch_lab`. At frame N
the hub's 23 entities (its root among them) are saved through the rows (100
rows), with the clock (`SimTick`, `WorldTime`, `DayClock`, `ClockState`). Both
rooms run 60 frames (Alice walks and jumps, Bob walks). Then the hub alone goes
back to N and replays Alice's inputs, with Bob's room hidden by Bevy's
`Disabled` (every default query skips it).

- **Passes:** the hub ends with every body (the hub's NPCs among them) exactly
  where the first run left it, Bob's room is exactly as it was before the
  replay, and the tick agrees.
- **The control** (`without_hiding_the_other_room_the_replay_moves_it_too`):
  the same replay with Bob's room left in the world changes Bob's body (his
  seat is idle in the replay), so hiding the room is what keeps it.

So on this scene a room can be rewound and resimulated in the host's own App,
with no second `World`: the schedule runs once per replayed frame and the
other rooms simply are not in it. **Limits of this witness:** the window
spawns and despawns nothing (the snapshot cannot yet); Alice only walks, so no
coupling resource is exercised (no connect, no hitstop, no projectile, no
conversation); room-keyed resources are not saved; the replay re-runs systems
that write committed-world resources (the save, the lives ledger) with nothing
to stop a replayed write from landing twice.

### ISL-2c, started (2026-11-10): spawn and despawn across the snapshot

**Restored again (review, 2026-11-10):** a snapshot keeps the ids it saved
and, beside them, the entity that stands for each now. Every restore maps
each saved id to its stand-in, so a snapshot restored twice (its stand-in
alive) or three times (the stand-in gone too) still puts back references to
live entities. It used to replace the saved id with the stand-in while the
saved values kept the old id. Witness:
`room_snapshot::tests::a_snapshot_restored_again_keeps_its_references_live`
(poisoned: the old per-restore remap fails it).

`RoomSnapshotRows::restore` now takes the room's entities at the restore:
one spawned after the save is despawned, a saved one that is gone is spawned
again, and the references the room's components hold to it are rewritten
through the registered entity-mapping rows (`record_map_row`, recorded by both
registrars; resource mappings are not). Unit-tested in the owner module.

Witness `a_rewound_room_fires_its_shots_again` (same fixture): Alice fires two
shots in the window, one that ends inside it and one still in flight when it
closes. The restore despawns the live one; the replay fires both again, and
the hub ends with the same shot at the same age and position. The shot's
`SimId` (`slot:0/1`) comes out the same because its counter
(`body.sim_id_counter`) is on Alice and rewinds with her. The global
`ProjectileSeqCounter` does not rewind, so the replayed shot's sequence number
differs; behaviour does not depend on it within one room, but a room checksum
would.

Witness `a_rewound_room_hurts_its_enemy_again` (same fixture): an enemy is
spawned in Alice's way in the hub before the save, and her shots hurt it in
the window. The restore gives back its health; the replay hurts it again by
as much, and the hub ends as it did.

The first run of this witness failed. The replay hides Bob's room with
`Disabled`, and the freeze gate (ISL-1b) counted live rooms with a default
query, which skips disabled entities. So in the replay the gate saw one room,
and the hit froze the shared clock. In the first run, with two rooms, it had
not. Every hub body ended offset. The gate now counts hidden roots too
(`Allow<Disabled>`): a room hidden for another room's replay is still live.
The same rule holds for any future reader that asks how many rooms are live.

**Projectile sequence numbers stay global, for now.** `ProjectileSeq` is
only a total order: the step, the clash sweep and the `SimId` mint sort by it,
and a shot's identity comes from its owner's counter. In a rewound room the
replayed shots get larger numbers, but their order within the room is the
same. Rooms do not interact, so behaviour does not change. The number becomes
a coupling when room state crosses machines: a guest that receives a host's
shots and then mints its own from a lower counter would order its new shots
first. The per-room counter belongs with that transfer (NA-2b's guest App).
It would be a counter on the room root, saved with the room, sorted by
(room, seq).

### ISL-2d, measured (2026-11-10): a world write in a rewound window

Probe `probe_a_coin_taken_in_a_rewound_window` (ignored): Alice takes the coin
in `basement_treasure` while Bob stays in the hub; her room rewinds to before
the pickup and replays it.

| After | Wallet (on Alice's body) | `RewardGrantsSinceCheckpoint` (world) | Save |
|---|---|---|---|
| frame N | 0 | 0 grants | — |
| the pickup | 25 | 1 | one flag set |
| the restore | 0 | 1 | unchanged |
| the replay | 25 | **2** | unchanged (the flag set again) |

What lives on the room's entities rewinds with the room. An append-only world
record does not: the replay records the grant a second time, and a death
restore would then take back two grants for one coin. An idempotent world write
(the save's flag) survives a replay that repeats it, but would stay wrong after
a replay in which the pickup no longer happens.

**Design (chosen, not built):** a world record written from a room carries the
room and the tick (`SimTick`) it was written on. A room rewind to tick N
announces itself (room, N), and each world store that takes room writes
retracts that room's records after N; the replay writes them again. An
overwrite (a save flag) keeps the value it replaced, so the retraction restores
it. The alternative, holding a room's world writes in an outbox until its
frames are confirmed, delays every world consequence by the rollback window
and needs read-your-own-writes; retraction keeps world facts immediate. It is
built with the room rollback host (the code that decides when a room rewinds),
not before it: today only tests restore a room. Its first customers are the
grant record and the save's pickup flags; the census of world stores written
from rooms comes with it.

**Built, first customer (review 2026-11-10, finding 7):** the retraction
mechanism is in `ambition_platformer2d_shared_tangle::lifecycle::room_writes`:
a world record written from a room carries a `RoomWrite` (room, `SimTick`),
a store registers a retractor (`register_room_write_retractor`), and whatever
rewinds a room calls `retract_room_writes(world, room, saved_at)`; each store
drops that room's records with a tick after `saved_at` (the tick advances at
the head of a step, so the window's first frame writes `saved_at + 1`). The
grant record is the first customer: pickups and chests stamp each grant, and
its retractor drops the room's later grants. Witness:
`a_rewound_room_takes_back_the_grant_its_replay_writes_again` (one grant
after the replay); the probe, now the control, still measures two without
the retraction.

**Census of the world stores a room writes (2026-11-10, read from the
writers):** two shapes, and they need different retractions.

| store | where | written by | shape |
| --- | --- | --- | --- |
| `RewardGrantsSinceCheckpoint` | items | pickups, chests | append (`Vec`): a replay duplicates. **Retracted** (above). |
| `BagSpendsSinceCheckpoint` | held items | bag spends | append (`Vec`): a replay duplicates, as the grants did |
| `ConsumedSinceCheckpoint` | items | pickups' consume and end, chests, breakables' spend | keyed by occurrence |
| `AuthoredOccurrences` | shared tangle | the same steps (`consume`, `end`, `spend`) and custody | keyed rows (whereabouts) |
| `BossDefeatsSinceCheckpoint` | boss encounter | a defeat | keyed by placement; "a later defeat replaces the earlier" |
| `WorldTimeSchedule` | world returns (session world) | regrowth, breakable returns | keyed by (room, feature): `record` / `forget` |
| the save's pickup and life sections | persistence | `persist_*_to_save` mirrors | keyed overwrite |
| `LifeLedger` | actor monolith | `keep_life_ledger` | keyed by character, rewritten from the live lives each tick |

An append store needs only "drop the room's rows written after the saved
tick" (the grants' retractor). A keyed store does not duplicate under an
identical replay, but a room rewinds because its input was late, so the
replay usually DIFFERS: a coin taken in the mispredicted future and not in
the corrected one stays consumed in `ConsumedSinceCheckpoint` and in
`AuthoredOccurrences` (and the corrected room, whose snapshot brought the
coin back, then disagrees with the ledger). Dropping the row is wrong when
the window overwrote an older value (a boss defeated before the window and
again in it), so a keyed store needs an undo log: each write in a room
records (room, tick, key, the value it replaced), and retracting a window
restores those values newest first. `ConsumedSinceCheckpoint` and
`AuthoredOccurrences` are written in the same steps and must be retracted
together. `LifeLedger` needs nothing: it is rewritten from the restored
bodies on the next tick. The save sections follow their ledgers (the
mirrors run after them).

**The census, second pass (2026-11-10): the population, not the
spelling.** The first pass followed the "since the checkpoint" stores and
missed the bag. This one starts from every world resource registered for
rollback (58 types, `rollback_resource_*::<T>`) and every session-root
component (14, `require_on_session_root::<T>`), and asks which a room's
gameplay writes:

| store | written from a room by | the key | retraction it needs |
| --- | --- | --- | --- |
| `OwnedItems` (the shared bag) | pickups and chests (grants), throws (spends), narrative and conditions | an item kind, SHARED: Alice and Bob add to one count | the inverse of each write (subtract its delta) |
| `QuestRegistry` | encounters, flag and quest effects, room entry (queued, drained each frame) | a quest, shared by rooms | the prior state only if no other room advanced that quest in the window |
| the save's flags | `SetFlagRequested` (pickups, chests, deaths, talk) | a flag, set-only, shareable | as the quests: a flag another room also set in the window stays set (done: `FlagWritesInRooms`) |
| `PossessionState` | possession | a participant (one room) | the prior value |
| `GateNetworkState` | the gate network | a body, a dialing endpoint | none alone: a link joins two rooms, an ISL-1b coupling |
| `BrokenBricks`, `SpentPowerBlocks`, `SpentMonitors` | the Mary-O and Sanic demos | a feature (one room) | the prior value, if those demos get room rollback |
| `SessionRecords` (a module's `attachment: "session"` records; found 2026-11-20) | any room's script entry | a record a module shares across rooms | none yet: a room rewind does not retract it. Shared world state goes through world values instead, which write only on a confirmed tick (`world_values.rs`) |

Not records a room keeps: `ProjectileSeqCounter` (a replay draws new ids;
they must be unique and ordered, not equal, and
`a_rewound_room_fires_its_shots_again` passes), `SwitchActivationQueue`
(drained on the next tick), the clocks (shared: the witnesses set them),
the checkpoint baselines (written at a commit), the Smash match state (one
rollback domain), the portals (the portal owner's), and the conversation,
the cutscene and the bullet-time scale (ISL-1b couplings).

⭐ **TWO KINDS OF KEYED STORE.** A key one room owns (an occurrence, a
feature, a participant) can be put back to the value it had: nothing else
writes it in the window. A key several rooms share (a count in the shared
bag, a quest, a flag) cannot: an item Bob picks up in the hub during Alice's
window would be erased by restoring her room's prior count of it (a coin
would not: the wallet is on the body). A shared key needs the
inverse of each write, which a count has (its delta) and a quest stage and a
set-only flag do not, unless the store keeps who set it.

**The first keyed customer (2026-11-10):** `ConsumedSinceCheckpoint` is
the undo log of the occurrence ledger's terminal rows. Each record written
from a live room (`record_consumed_pickups`, `record_ended_occurrences`,
`open_ecs_chests`, `record_spent_breakables`) carries a `ConsumedWrite`: the
room and tick, and the ledger row it replaced (none for a taken pickup or an
opened chest; the `Placed` row of an ended occurrence). Its retractor
(registered in `ambition_platformer2d_runtime/src/world_schedule.rs`) puts
the record and the row back, newest first (`AuthoredOccurrences::restore_row`
keeps the derived indexes). The witness is
`a_rewound_room_untakes_the_heart_its_replay_does_not_take`
(`game/ambition_app/tests/a_room_rewinds_alone.rs`): Alice takes the
one-time heart in `basement_breakables` while Bob holds the hub, the room
rewinds, and the replay does not take it; both stores forget it. Its control
(ignored) keeps both rows; the arithmetic (an ended occurrence is `Placed`
again, a write before the save or from another room stays) is
`a_rewind_puts_back_the_rows_its_room_wrote_after_the_save` (items). Limits:
the log is forgotten with the records at a committed checkpoint, so a window
cannot reach across one, and it is not in the value checksum (as the grants'
stamps are not): it changes no value until a rewind uses it.

**The shared bag (2026-11-10):** the room's own writes are inverted, not
the bag restored. A grant's record already held the quantity it added
(`PickupGranted::item`), so the grants' retractor takes it back out of the
bag; the coins stay, because the wallet is on the collector's body and the
room's snapshot rewound it. A throw's spend (`BagSpend`) now carries its room
write, and its retractor puts the quantity back (`OwnedItems::put_back`, the
uncapped inverse of `take`). The probe found a third write: the throw MINTS an
object, and the ledger admitted its `Placed` row in the window; after the
rewind the replay saw that row with no object and wrote it `Consumed`, so a
mint that never happened was remembered ended. The ledger now notes each mint
with the room and tick it entered from (`note_mints_entered`, at its mark in
`record_placed_ground_items`), and one retractor
(`resource.authored_occurrences`) first puts back the terminal rows, then
takes out the room's mints (`retract_room_mints`), in that order so a mint
made and ended in the window leaves. The witness is
`a_rewound_room_puts_back_the_throw_its_replay_does_not_make` (Alice throws
a gun sword in `basement_breakables` while Bob holds the hub; the replay does
not throw); with the mint retraction poisoned it reads `(1, 0, ["slot:0/0
Consumed"])`, and its ignored control keeps the bag spent.

The other bag writers, read 2026-11-10: a body's **cell use**
(`apply_item_uses`) is a room write, now recorded with the body's room and
tick in `BagUsesInRooms` (rollback state, `item.bag_uses_in_rooms`, bounded by
the rewind horizon) and put back by its retractor; witness
`a_rewound_room_puts_back_the_cell_its_replay_does_not_use`, with an ignored
control. A **conversation's grant** and a **shop transaction**
(`apply_item_grants`, `apply_shop_transactions`) waited for the
conversation to be a room's; done 2026-11-11 (see "Its grants and sales"
under ISL-1b below). The census's "conditions' take" was a test's
(`conditions.rs`'s test module), not a writer.

**Boss defeats (2026-11-10):** each defeat record carries the tick it fell
on, and a room's rewind takes back the defeats that fell there after the
saved tick, through the consequences the death replay already applies
(`retract_defeat_records`: the save row back to `Untouched`, the looted flag,
the quest steps it caused), the reward chest, and `BossDefeatRetracted` for
the item domain's mints. The witness is
`a_rewound_room_takes_back_the_boss_defeat_its_replay_does_not_make` (a
mockingbird in Alice's room killed in the window); its control shows why it
matters beyond the record: the restored boss stays dead, because its save row
still says `Cleared`.

**The world-time schedule (2026-11-10):** for a live room it is a mirror of
live state, republished every tick, so a replay that does not take a pickup
forgets the window's record by itself. What it does not heal: a record keeps
the first departure's due time, so a replay that takes the regrowing heart on
its first tick, before the mirror forgets, kept the window's (measured: due at
6.22 s for a heart the replay took at 1.57 s, 4 s regrowth). The two mirrors
now write through `record_from`/`forget_from` with their room write, and the
schedule keeps an undo log that a rewind replays newest first. The schedule is
not cleared at a checkpoint, so the log is bounded by the rewind horizon,
`ROOM_REWIND_HORIZON_TICKS` (600 ticks, beside `RoomWrite`): the room
rollback host must not rewind further. Witness:
`a_rewound_room_keeps_the_regrowth_time_of_its_replay`; unit:
`a_rewind_puts_back_the_records_its_room_wrote_and_forgot` (and the horizon).

**Flags and the quest steps they move (2026-11-10):** the decision above,
for a set-only flag: the store keeps who set it. `SetFlagRequested` carries
`written: Option<RoomWrite>`, filled by the room-level producers (a story-flag
pickup or chest reward through `grant_pickup`, a chest's looted flag, an
enemy's death flag in `actor_hit.rs`, and since 2026-11-11 a talk's flags,
a switch's quest flags and an authored `world.set_flag` asked in a room) and
`None` for the rest (a cutscene, a conversation's own grants, the intro chain
and the room look are no one room's). `apply_flag_effects` logs each room's
write in `FlagWritesInRooms` (rollback state, `resource.flag_writes_in_rooms`,
bounded by the rewind horizon) with the value it replaced, and queues the
flag's quest advance CAUSED by it (`write_flag_caused_by`, the boss road's
`push_event_caused_by`). A write no room owns is logged too while the flag has
a room's write in the log. The retractor
(`effect_bus::retract_room_flag_writes`) takes the room's writes back newest
first: the flag gets its prior value and `retract_caused_by` puts back the
quest steps, unless a later write to the same flag still stands, which keeps
both. A cause that leaves the horizon is forgotten
(`QuestRegistry::forget_cause`). Witness:
`a_rewound_room_puts_back_the_flag_and_quest_step_its_replay_does_not_set`
(the one-time heart made a story flag, a quest waiting on it); its ignored
control keeps both. Still open: `QuestAdvanceRequested` (an item collected, an
NPC talked) carries no room, so a quest step moved by an event that is not a
flag is not taken back yet.

**Wave encounter clears (2026-11-10):** the quest events that are not flags,
read in `quests.ron`: `EncounterCleared` (3 steps), `BossDefeated` (4, done
above), `RoomEntered` (5: a room's entry, before any window of it),
`ItemCollected` and `NpcTalked` (1 each). A wave clear
(`apply_wave_encounter_effects`) greens the save's switches linked to the
encounter and queues `EncounterCleared`; the encounter's own save row is a
mirror of its lifecycle, which the room's snapshot rewinds. The clear is now
recorded with its room and tick, and each switch's prior value, in
`EncounterClearsInRooms` (`ambition_encounter_features::room_writes`,
rollback state `resource.encounter_clears_in_rooms`, horizon-bounded), and its
quest event is caused by it. The retractor puts back the switches and the
quest steps, newest first, unless a later clear of the same encounter stands.
Witness `a_rewound_room_puts_back_the_wave_clear_its_replay_does_not_make`
(the goblin wave's `Completed`, written as the reducer would, with Alice in
`goblin_encounter` and Bob in the hub), with an ignored control; units in
`room_writes.rs`. `NpcTalked` is the talker's room's (below).

**A pickup's `ItemCollected` (2026-11-11):** `QuestAdvanceRequested` now
carries the cause its event is queued under (`None` for an event nothing
retracts; `QuestAdvanceRequested::new`). The pickup stamps a cause naming
its room and tick and logs it in `QuestCausesInRooms` (rollback state
`resource.quest_causes_in_rooms`); `apply_quest_effects` queues the event
under it and forgets the causes past the rewind horizon. The retractor
(`retract_room_quest_causes`) retracts the room's causes newest first, except
where a later event with the same label stands. Witness
`a_rewound_room_puts_back_the_quest_step_of_the_pickup_its_replay_does_not_take`,
with an ignored control. Every shipped quest event a room's act queues is now
taken back by that room's rewind: flags, boss defeats, wave clears, pickups.

**Next:** the room rollback host that calls `retract_room_writes` in
production (today the witnesses and the ISL-2e probe host
`room_rollback_host.rs` call it after each restore).

### ISL-2e, measured (2026-11-10): what room-level rollback costs a hosted world

Probe `probe_what_room_rollback_costs_a_hosted_world`
(`game/ambition_app/tests/room_rollback_host.rs`, ignored). The NA-2 scenes,
with a third topology: the host predicts the guest's input (his last arrived
one), saves the guest's room every frame through `RoomSnapshotRows`, and when
the real input for frame g arrives three frames late and differs, restores the
room to g (with the shared clock and the rollback-cleared message channels)
and replays it to now. An oracle run with the guest's input on time gives the
right ending.

| Scene | Host, late input applied (NA-2) | Host with room rollback | GGRS, each peer (NA-2) | Rollbacks / replayed frames | Ends as the oracle |
|---|---|---|---|---|---|
| `switch_lab`, inputs never change | 8.4 ms | 9.4 ms | 32.6–34.7 ms | 1 / 3 | yes |
| `switch_lab`, Alice fights, Bob turns | 8.9 ms | 14.5 ms | 37.7–48.0 ms | 43 / 129 | yes (2 bodies) |
| `central_hub_complex` with its NPCs | 9.9 ms | 16.3 ms | 45.2–47.6 ms | 43 / 129 | yes (18 bodies) |

Milliseconds of host update per simulated frame, debug build, one run each, so
compare ratios. A room save costs 1.3 ms (`switch_lab`) to 2.3 ms (the hub)
a frame and a restore 2.0–3.0 ms; the rest is the replayed updates. **Keeping
a room correct by rollback costs the host about 1.1–1.6 times a host that does
not, and about a third of what each GGRS peer pays**, and only the room whose
guest was mispredicted pays it.

One divergence was found and fixed in the probe: the dog in the hub ended
8 px off, because its rest ends at a time read from `GameplayElapsed` (on the
session root), which the first probe did not rewind. The session's elapsed
time is part of the shared clock a room rewind restores, beside `SimTick`,
`WorldTime`, `DayClock` and `ClockState`.

What this does not show: a guest's own prediction and correction (NA-3); two
rooms with a rewind in one (the witness above shows the other is untouched,
but not its cost); world writes in the window (ISL-2d); bytes.

### Next (ISL-2c and ISL-1b)

1. **ISL-2c — the witness under load.** A hit on an NPC: done
   (`a_rewound_room_hurts_its_enemy_again`). **An NPC life that writes the
   ledger (2026-11-11):** a life's row in `LifeLedger` is a mirror of its
   `Life` (`keep_life_ledger` compares and writes each tick), which the
   room's snapshot rewinds, so the row heals by itself. The bookings do not:
   they are shared by rooms and summed, so a hold or a catch-up booked in the
   window would be booked again by the replay. Each booking a body adds from
   a live room is now logged with the room and tick
   (`LifeLedger::room_bookings`, horizon-bounded), and the retractor
   (`retract_room_bookings`, `resource.life_ledger_bookings`) subtracts the
   rewound room's additions after the saved tick. Witness
   `a_rewound_room_books_its_replayed_hold_once` (`eureka_coil_lab` from five
   past ten: the room saved while a scientist holds the bench, the hold ends
   and is booked in the window, the replay holds again); its ignored control
   books the hold twice. Unit:
   `a_rewound_room_takes_back_the_bookings_its_bodies_made`. Still open: a
   connect (hitstop: ISL-1b below). A failure of the witness
   names the state that could not be kept to one room. These run as probes or
   one suite witness each, not a matrix.
2. **ISL-1b — remove the couplings.** One change per coupling resource, each
   with a witness that an event in one room leaves another room's state
   unchanged. **Done 2026-11-10, the freeze:** with several live rooms neither
   the impact hold nor Player 1's hitlag stops the shared clock
   (`emit_primary_observer_time_intent`), as a conversation is not modal
   there; one live room keeps the freeze. Witness:
   `a_hit_in_one_room_does_not_freeze_the_other`. **This is provisional
   (review 2026-11-10, finding 5):** it isolates the other room by giving up
   the freeze in the struck room too (the struck body keeps its own hitlag,
   but the rest of its room runs on), so a fight played while another room
   is live does not feel like the same fight alone. The fix is a freeze
   scoped to the connect's room, which waits for a room clock (body motion
   reads the shared `sim_dt`). Independent room rollback does not ship until
   that freeze is back in the struck room. **Back in the struck room
   (2026-11-11), without a room clock:** each room a connect lands in (by
   the victim's room) keeps the hold's expiry in `RoomPace::held_until` on
   its root, beside the shared expiry, and `RoomFreeze` answers a body's pace: `0.0` while its
   room is held. The readers that carry a fight's feel multiply the `dt` they
   give that body by it:
   - body motion (`integrate_sim_bodies`, players and actors);
   - actor brains (`tick_actor_brains`, after the actor's room is known);
   - move playback, buffered presses, reactions, guards and pose clocks;
   - a boss's frames;
   - projectiles (`step_projectiles`, by the shot's room).

   **And the fight's own clocks (2026-11-11):** a strike's window
   (`tick_and_despawn_hitboxes`), the melee cooldown floor, the ranged
   refire, the ability cooldown, and the bomb and gravity-grenade fuses tick
   at their body's room's pace too, so a held room's strike does not expire,
   and its fuse does not burn, during the hold. Witness:
   `a_cooldown_waits_while_its_room_is_held`.

   **And a kill's beat (2026-11-11):** the death beat (`kill_beat` in a
   room's rules, 0.083 s in Ambition's) wrote only the shared hold, which
   stops no clock while two rooms are live, so a kill there had no freeze.
   It now holds each room a kill lands in through `RoomHolds` (the same
   writer a connect uses), and a dying body's beat runs at its room's pace.
   Witness: `a_kill_in_the_hub_holds_the_hub_and_not_bobs_room` (poisoned:
   with the room hold dropped, the hub has no hold).

   With one live room the shared clock is already stopped, so nothing
   changes there. The other ~80 `sim_dt` reads (day clock, encounter timers,
   room departure, cutscene, trails, demos) still run in a held room, as they
   ran in every room before; a held room is a few ticks long. Witness:
   `an_impact_freeze_in_the_hub_holds_the_hub_and_not_bobs_room` (Alice,
   driven, stands still for the hold while Bob walks on, and moves again when
   it runs out). Schema 450 (the checksum folds each held room). **And the
   clock scale per room (2026-11-11):** with several live rooms, Player 1's
   bullet time (a blink aim or a blink hold) no longer scales the shared
   clock. `emit_primary_observer_time_intent` asks the shared clock for 1
   (`room_clock`) and eases Player 1's room toward the bullet scale in
   `RoomPace::clock_scale` on its root, at the shared clock's ramp rates;
   every other room eases home. `RoomFreeze` multiplies the room's scale into
   the same readers as the freeze. **On the root, not in a resource (schema
   452):** the first cut kept both in global resources keyed by room (schemas
   450, 451), and `a_rewound_room_hurts_its_enemy_again` went red: a room
   rewound to before a hit kept the hit's later hold, and its replay ran
   frozen. The room snapshot restores every rollback component of the room's
   entities, its root included, so `RoomPace` (registered as
   `combat.room_pace`) rewinds with its room. Witnesses:
   `bullet_time_slows_only_player_ones_room_while_two_rooms_are_live` and
   `a_body_runs_at_its_own_rooms_pace`. **One clock at a time (review
   2026-11-11, P2):** when a second room unloaded during bullet time, the
   shared clock took the slow-down back while the room's own scale eased
   home, and the readers applied both (0.25 on each ran the room at
   0.0625). `RoomFreeze` now applies a room's scale only while more than
   one room is live (`rooms_keep_their_own_clocks`, counting a room hidden
   for another's replay), and when the count changes the emit system hands
   the slow-down from one clock to the other where it stands, each easing on
   from there. Witnesses:
   `the_only_live_room_runs_at_the_shared_clock_not_its_own_scale_too` and
   `the_slow_down_passes_between_the_clocks_when_the_room_count_changes`
   (each poisoned). The same ~90 other `sim_dt` reads
   keep the shared pace, as for the freeze. Still open: projectile ids
   sequenced per room (deferred to a guest that mints its own shots, NA-3's
   prediction: NA-2b's guest builds the host's shots from spawn rows and
   mints none; see ISL-2c above), and one conversation and one cutscene per
   participant or room. The room-local `sim_dt` reads follow their room's
   pace (2026-11-11): the fight's, trails, encounter waves and, since then,
   a room's cutscene (`tick_active_cutscene` reads `RoomFreeze` for its
   room). The day clock is the world's and keeps the shared pace. A module
   entry's `dt` follows its scope's room (2026-11-11: the extension host
   takes a `ScopePace` hook, which the runtime fills from `RoomFreeze`; the
   host itself knows no rooms). The clocks that stay shared, by ruling: the
   match clock (the session's), banners (presentation), room transitions,
   passages and the departure watchdog (a trip spans two rooms), and the
   gate network (a link joins two rooms, an ISL-1b coupling).

   **The 91 message rows, by what they carry (2026-11-11,**
   `scripts/measure/message_rows_by_room_carrier.py`**):** 13 carry their room
   (the encounter's four, `HitEvent`, `VfxInRoom`, `DebrisBurstMessage`,
   `FxRequest`, `FireworksRequest`, `KnockoutBeatRequested`,
   `SwitchActivated`, `RoomReplayAdmitted`, `SetFlagRequested` by its
   `RoomWrite`); 51 carry an entity, a peer id or a participant, from which
   the room is derivable; 27 carry neither. The 27, read one by one:
   - **one per world by meaning (7):** `NewGameRequested`,
     `CheckpointCommitted`, `ResetToCheckpoint`, `StocksMatchDecided`,
     `SuddenDeathBegan` (the match's), `NpcProvocationChanged` (a durable
     world fact), `RoomLoaded` (a load, not a room's act);
   - **the portal owner's (4):** `ClearPortals`, `PortalBodyEntered`,
     `PortalShotFired`, `TogglePortalGunsActive`;
   - **the one conversation's, ISL-1b (3):** `ConversationEnded`,
     `ItemGrantRequested`, `ShopTransactionRequested`;
   - **the clock's, ISL-1b (2):** `ClockScaleRequest`, `ClockResetRequest`
     (a domain and a requester, no room: the scale coupling);
   - **staging, not a room's act (1):** `SpawnActorRequest` (the harness and
     publication staging write it; it spawns session-scoped);
   - **a room's act that drops its room (10), the coupling list:**
     `CameraShakeRequest` and `FinishZoomRequest` (every view shakes or
     zooms for one room's hit), `GameplayBannerRequested` (one banner for
     every player), `GameplaySfxRequested` (a position with no room) and
     `OwnedSfxMessage` (an owner context, no room), `AmbientGravityRequest`
     (a cycle with no room), `RoomReplayRequested` and
     `CutRopeRoomReplayRequested` ("the" room), `RunAuthoredCommand` (its
     `AuthoredAsk` names a kind and subject, no room), and
     `QuestAdvanceRequested` (its cause names a room as text, 2026-11-11; an
     uncaused event does not).
   The ten are the ISL-1b work list for messages, beside the clock scale,
   the conversation and the cutscene.

   **The camera shake (2026-11-11):** `CameraShakeRequest` carries the room
   whose views shake (`None` shakes every view), and `CameraShakeState`
   keeps each room's shake beside the shared one; a camera reads
   `offset_in` the room its view frames. Every writer names its room: the
   hardest landed hit and the loudest kill of each room
   (`shake_camera_on_landed_hits`, `advance_death_beats`), a boss's defeat and
   phase change, a conducted boss's port, a bomb, and a hard landing.
   Witnesses: `a_hit_shakes_only_the_views_of_its_own_room` (combat) and
   `a_room_s_kick_shakes_only_that_room_s_views` (the state);
   `hit_shakes_the_camera` reads any view's shake (`loudest`).

   **The finish zoom (2026-11-11):** `FinishZoomRequest` carries its room the
   same way, and `FinishZoomState` keeps each room's closeness and hold;
   a camera scales by `scale_factor_in` the room its view frames. A
   finishing blow and a boss's defeat name their room; a decided match
   zooms every view (`None`), since the match is not one room's. The
   finishing blow's cast freeze still reaches every fighter: Smash runs
   one match in one room. Witness:
   `a_room_s_finish_zoom_pulls_in_only_that_room_s_views`. A session
   replaced by `rest_the_camera_on_activation` now forgets every room's
   shake and zoom, not only the shared ones.

   **The banner (2026-11-11):** `GameplayBanner` keeps each live room's
   banner beside the one every player sees (`show_in`, `visible_in`), and
   `GameplayBannerRequested` carries its room (`in_room`). Its one reader,
   the inspector HUD, shows the banner of the primary seat's room. In a
   networked session a peer's process runs every live room, so a chest opened
   in Alice's room had announced itself in Bob's HUD. The room's acts name
   their room: pickups, chests, breakables (a break, a collapse, a respawn),
   a body's defeat and fall, a boss's phase and defeat, an encounter script's
   banner, a wave's clear, the symmetry attunement, the cut-rope flattening,
   a talk, a switch, a provocation, a blocked or armoured hit. Two stay
   every player's: a quest payout (the quest book is the session's) and the
   sandbox reset. Witness: `a_room_s_banner_shows_only_in_that_room`; tests
   that asked "was a banner shown" read `shown()`, every room's, so a room's
   banner cannot slip past them.

   **Sounds (2026-11-11):** `ambition_sfx` gains an opaque listening space
   (`SfxSpace`). A spatial emitter carries one, so a voice keeps it while its
   gain is recomputed each frame. `SfxListeners` carries each listener's
   space, and `OwnedSfxMessage` carries the space it was made in
   (`SfxWriter::in_space`, `BodySfxWriter::in_space`). A listener hears a
   spatial sound only from its own space, or from a sound with none. The host
   names the spaces: `ambition_vfx::sfx_space(room)` maps a live room to its
   space, and the sfx crate never names a room. Each view listens in the
   space of the room it frames (`publish_audio_listeners`). Before this, two
   live rooms at the same coordinates (two instances of one room) heard each
   other's hits as if they were beside the listener.
   Writers that name their room:
   - movement (`emit_movement_fx`);
   - hit feedback (`emit_hit_feedback`), a body's or boss's guard, death and
     strike, a breakable struck or broken, a guard deflect;
   - pogo bounces (on-hit, extension contact, attack support);
   - pickups and chests, switches (`apply_switch_effects`);
   - projectile parry, splash and bounce, a brain's shot report;
   - bombs and the gravity grenade, the puppy-slug gun, vitality pulses,
     extension body sounds;
   - a boss's defeat and phase change, a character's beat cues, a pet;
   - teleport, trapdoor and flyline;
   - a move's authored cue (`MoveEventKind::Sfx`), an `FxRequest`'s cue
     (`process_fx_requests`), a knockout jet;
   - the cut-rope slash, lock walls, wormhole gates;
   - `GameplaySfxRequested`, which now carries a room (it has no production
     writer today).

   Still heard in every space (no room named):
   - the shell menus, pause menu, kaleidoscope menu, and the dialog
     blips (`ambition_dialog`, `yarn_vocabulary`): UI, which belongs to a
     seat rather than a room, and the one conversation of ISL-1b;
   - the sandbox reset;
   - door crossings (`room_transition/commit.rs`, `rooms/passage.rs`,
     `rooms/systems.rs`): a crossing spans two rooms, and the session crate
     does not reach `ambition_vfx`;
   - the shrine (its crate does not reach `ambition_vfx`);
   - the smash-charge and parry cues in `body_cues.rs`, read from the feature
     view index, which carries no room;
   - the demo crates (Sanic, Mary-O, Twin Track);
   - the portal adapters (the portal owner's).
   Witnesses: `a_sound_in_one_space_is_not_heard_from_another_at_the_same_place`
   (the gain rule) and
   `a_landing_in_one_room_is_not_heard_from_another_at_the_same_place`
   (`ambition_audio`, under `--features kira`: playback applies the
   message's space; poisoned by dropping it, the assertion "another room's
   listener heard it" fails).

   **Three of the remaining five were misfiled (2026-11-11):**
   - `AmbientGravityRequest` is a developer tool's ask (a hotkey, the
     developer menu). The room is named where it is applied:
     `apply_ambient_gravity_requests` cycles `BaseGravity` for
     `PrimaryLiveRoom`, the room the developer looks at, one tick after the
     key. So it is a seat's act with its room resolved on application, not a
     room's act that loses its room.
   - `RoomReplayRequested` is the same: `admit_room_replay` replays the
     controlled subject's room, and `close_death_interlude` asks only when
     the primary's room goes back (Q153: a second seat that falls comes back
     beside the primary). One residual: with no primary (a match) and two
     rooms live, it reaches the replay only in a composition with no
     checkpoint horizon. There the request names no room, `definition_named(None)`
     finds no sole room, and the replay is dropped.
   - `CutRopeRoomReplayRequested` is the conversation's ("try again"), so it
     is the one-conversation item's. It replays the controlled subject's
     room, which is the talker's room only while the primary is the talker.

   **Authored commands (2026-11-11):** `RunAuthoredCommand` carries the live
   room it was asked in (`in_room`), and every `CommandRunner` is handed it.
   - A switch's verb is asked in the switch's room.
   - A room's `while_live` line is now asked once per live instance, in that
     instance (it used to be asked once per room id).
   - The conversation's verbs are no room's (`None`).
   - `encounter.signal` and `encounter.start` resolve only the occurrence in
     the asking room. Before, they refused whenever two live rooms each held
     the encounter.
   - `world.set_flag`, asked in a room, dates its write in that room, so a
     rewind of that room takes it back (ISL-2d). It dates only a write that
     changes the flag, so a line asked every tick logs nothing.

   Witness: `a_signal_asked_in_a_room_reaches_that_room_s_occurrence_of_two`
   (poisoned by dropping the room filter: refused).

   **A switch's quest flags (2026-11-11, ISL-2d):** `drive_wave_encounters`
   wrote `test_switch_toggled` and the switch's `switch_<id>_used` straight
   into the save, so a rewound room's switch press kept them. They now go
   through `SetFlagRequested`, dated in the switch's room (only a write that
   changes the flag), so the flag log retracts them and the quest steps they
   moved. The driver no longer writes the save at all. Witness:
   `a_switch_dates_the_flags_it_turns_on_in_its_own_room`.

   **A talk (2026-11-11, ISL-2d):** pressing interact on an NPC is the
   talker's room's act, done in the simulation. `interact_ecs_actors_and_switches`
   now dates its two flags (`met_any_hub_npc`, the dialogue's talked flag) in
   the talker's room at that tick and queues `NpcTalked` under a recorded
   cause, so a rewind of that room takes them back and the replay talks
   again. What the conversation then grants or sells (`ItemGrantRequested`,
   `ShopTransactionRequested`, from the Yarn runner through the narrative
   ledger) stays the one conversation's (ISL-1b). Witness:
   `a_talk_is_dated_in_the_talkers_room`.

   **The one conversation (2026-11-11, ISL-1b):** `ActiveConversation` is
   one world value, and a room rewinds alone. A talk opened in the rewound
   window stayed open at the rewind (the separation rule closed it a frame
   into the replay, with a cut bark the replay never earned), and a
   conversation that ended in the window stayed ended though the replay kept
   talking. A conversation is two bodies of one room, so each change is that
   room's act: `ConversationsInRooms` (rollback state,
   `resource.conversations_in_rooms`, schema 453) logs each change at the
   tail of the tick with the room of its bodies, the tick and the value it
   replaced, and its retractor puts back what the room had at the save,
   unless a later change of another room's stands. Witnesses:
   `a_rewound_room_takes_back_the_conversation_its_replay_does_not_open` and
   `a_rewound_room_puts_back_the_conversation_its_replay_does_not_leave`
   (read straight after the rewind, before a replayed frame, so the
   continuity rule cannot stand in), with an ignored control. Still one
   conversation at a time: Bob cannot open his own while Alice talks, and
   the text box is one; two at once need a box per view. A participant
   despawned and re-created by the room restore comes back as a new entity,
   which the restored conversation does not name, so the continuity rule
   ends it. **Its grants and sales (2026-11-11, ISL-2d):**
   `ItemGrantRequested` and `ShopTransactionRequested` now name the seat
   whose conversation made them (its input owner, filled when the Yarn
   command runs). A sale is paid from that seat's body's wallet (it was
   always Player 1's, so Bob buying from a merchant in his room spent
   Alice's coins), and the bag's change is dated in that body's room in
   `BagUsesInRooms` (a purchase or grant as a gain, a sale's item as a use;
   schema 455), so a rewind of the room takes it back. The wallet is on the
   body and rewinds with the room. Witnesses:
   `a_merchant_deals_with_the_seat_whose_conversation_it_is` (red with the
   payer poisoned back to Player 1, after a first cut whose buy and sell at
   one price cancelled on either wallet) and
   `a_rewound_room_takes_back_the_grant_its_conversation_gave`, with an
   ignored control.

   **The one cutscene (2026-11-11, ISL-1b):** a playing cutscene captured
   every participant's input, so a room's entry cutscene (or a boss's intro)
   in Alice's room held Bob in his. A cutscene now keeps the live room it
   plays for (`ActiveCutscene::room`, an ordinal, so the cutscene crate names
   no room type; schema 454): a room's entry trigger and a boss intro queue
   it in their room (`CutsceneTriggerQueue::request_in`), a world cutscene
   (`request`) holds everyone as before. The input contexts hold only the
   participants whose bodies are in its room, and its skip and dismiss are
   read from the lowest seat in its room, not Player 1's. Witness:
   `a_room_cutscene_holds_only_the_seats_in_its_room` (the input contexts of
   two seats whose bodies are in two rooms; red with `holds` poisoned to
   every room). The app harness cannot show it: a driven seat's frame skips
   the device road the claim gates, and the harness resolves no seat
   contexts. Still one cutscene at a time: a second
   room's entry cutscene waits in the queue until the first ends, then
   plays. **The drain dropped it (review 2026-11-11, P1):** it took the
   whole queue and started the first, so two rooms entered on one tick lost
   the second room's cutscene for good (each room is announced once). It
   now takes only the trigger it starts, and a trigger already waiting is
   not queued twice (the queue is not rollback state, so a replay asks
   again). Witness: `a_second_rooms_cutscene_waits_its_turn` (poisoned: the
   second is dropped). **Its flags (2026-11-11, ISL-2d):** a cutscene's `SetFlag` beats
   and its seen flag were written straight to the save; they now go through
   `SetFlagRequested`, dated in the room it plays for, so a rewind of that
   room takes them back with the quest steps they moved (a world cutscene's
   stay undated). Witness: `a_room_cutscene_dates_its_flags_in_its_room`.
   **Its playback (2026-11-11, ISL-1b):** `CutscenesInRooms` (rollback
   state, `cutscene.changes_in_rooms`, schema 456) logs each tick's change to
   the playing cutscene (its playback, its room, the skip hold) under the
   room it plays for, with the state it replaced; a room's retractor puts
   back what it had at the save, as for the conversation. Witness:
   `a_rewound_room_puts_back_its_cutscene` (read straight after the rewind),
   with an ignored control. **Without a script per tick (review 2026-11-11,
   P2):** each logged state held its own copy of the authored script, cloned
   every tick the cutscene advanced and written once per change. The
   runtime's script is now shared (`Arc`), and the log's codec writes each
   distinct script once in a table its states name by place (schema 457).
   Witness: `the_room_cutscene_log_keeps_one_copy_of_each_script` (poisoned:
   twenty copies for twenty ticks).

   The last message on the list, `QuestAdvanceRequested` with no cause, now
   has one writer: `RoomEntered`, a crossing, which spans two rooms and
   belongs to hosted authority, not to one room's rollback. So the
   room-dropping list is empty.

### The room rollback host in production (2026-11-14)

The hosted world now carries ISL-2e's host as code:
`ambition_hosted_world::room_rollback`, switched on by the host resource
`RewindsGuestRooms` (the visible app's `--host-world` sets it). A guest
stamps each input with the host tick it is for: the host replicates its
tick (`HostTick`), and the guest adds the frames it has run since it last
saw it. At the head of each tick, before the clock, the host drives every
guest seat with its input for that tick, or else the guest's latest arrived
input (the prediction), and saves each room only guests stand in, with the
shared clock. When an input arrives for a tick already run and differs from
the one used, that room alone is restored to its save. The rollback-cleared
channels are emptied, the clock is put back, the room's world writes are
retracted (ISL-2d), the other rooms are hidden (`Disabled`), and the
simulation replays to now through the simulation schedule (`FixedMain`, as
Bevy's fixed loop runs it, on a fixed-tick host). A host seat in the room is
replayed with what its devices gave each tick: the head records the seat's
latch, or on a host without latches its raw row, and the replay drives the
seat with that record; the input tables are put back after the replay. The
impact freeze (`ImpactHitstop`) is saved with the clock: while one room is
live it stops the shared clock, so a connect the replay takes back must take
its freeze back. Inputs and saves are kept for `REWIND_HORIZON` (60 ticks).

Witness `a_guests_late_input_rewinds_only_its_room_on_the_host`: Alice (the
host's seat 0) leaves for the hub, and Bob, a guest at seat 1, stays in
`switch_lab`. Bob's inputs reach the host 3 frames after the tick they name.
The host rewinds his room 26 times and replays 78 ticks, and all 19 bodies
end exactly where the run with his inputs on time ends (that run rewinds
nothing). The control, the same late inputs applied as they arrive, ends
with Bob elsewhere.

Witness `a_guests_late_input_rewinds_a_room_the_host_shares`: the same with
Alice beside Bob, fighting him. 26 rewinds, 78 ticks replayed, and both
bodies end exactly where the on-time run ends. Two misses, measured on the
way: with only latches recorded, a headless host (no latches) replayed Alice
with her current frame on every tick, and with her inputs right she still
ended 22.5 units behind, because a mispredicted connect's freeze outlived the
replay (the freeze merges expiries with `max`).

A replay presents nothing. Simulation and presentation messages use the
same replay boundary (`74e86f475`, 2026-11-16). The host keeps live message
buffers and reader positions outside replay, then restores them. A pending
heal in another room survives successive rewinds. Replay output is discarded.
Do not clear live channels for a room replay. See the
[room replay contract](../../systems/room-replay.md).

A guest's late crossing joins the live room (2026-11-14). Measured with a
probe (deleted): Bob, a guest on the hub door, presses interact at frame 30
while Alice is in the hub. On time he joins her hub at frame 32. Late, the
press arrived in a replay, which hides every room but his, so the crossing's
plan did not find the hub and built a second one (instance 2). Two rules fix
it. A replay publishes a `ConfirmedFrameBoundary` with its ticks unconfirmed,
and the eager crossing path honours a published boundary (it confirmed
everything before; it still does on a host that publishes none), so a
crossing the replay records begins on the first tick after it. And no room
is rewound while a crossing is recorded or loading: its commit plans against
the live rooms, and a replay would hide them. Late, Bob now joins Alice's hub
at frame 34 and ends where the on-time run ends (2 rewinds; 4 late inputs
applied from the next tick during the crossing).

The other confirmed-frame observers (checkpoints, deaths, passages, the
conversation ledger, host intents) treated each live tick as confirmed when
it first ran, because outside a replay the hosted host published no
boundary: an act on a tick a rewind replayed was not taken back. A boundary
published every tick would never have been fully confirmed while a guest
played (its inputs arrived a round trip late), and the sandbox autosave
waits for that. Both are answered below (2026-11-18): the guest runs ahead,
and the host publishes a live boundary. Still open: an edge exit walked by
a mispredicted guest commits at once and cannot be taken back.

## A rewind's cost on a release build over a real link (2026-11-18)

Measurement `a_guests_room_rewinds_over_a_real_link` (ignored; run with
`--release --ignored --nocapture`): the guest drives Bob over renet's
netcode on loopback, through a relay that holds each datagram a fixed delay
each way (`udp::host_on` names the relay as the host's public address, as a
host behind a router names its router). Both Apps are paced at 60 Hz of wall
time for 600 frames; Bob turns every 13 frames and jumps every 29, so about
one frame in seven brings an input that differs from the prediction. The
guest stamps its input with its own estimate of the host tick, so the
lateness is the ticks a rewind replays (renet's round trip is timed on the
App clock the harness steps, and reads 0.1 ms whatever the relay holds).

| Scene | Each way | Ticks a rewind | Host update, no rewind (mean, p95) | With a rewind (mean, p95, max) |
|---|---|---|---|---|
| Alice in the hub | 0 ms | 1.2 | 4.1, 5.0 ms | 8.8, 10.6, 20.9 ms |
| Alice in the hub | 25 ms | 4.0 | 4.2, 5.5 ms | 13.9, 16.3, 31.8 ms |
| Alice in the hub | 50 ms | 7.0 | 3.9, 5.0 ms | 18.6, 22.1, 29.0 ms |
| Alice beside Bob | 0 ms | 1.4 | 3.5, 4.6 ms | 8.9, 11.9, 19.6 ms |
| Alice beside Bob | 25 ms | 4.0 | 3.8, 5.2 ms | 16.7, 18.9, 26.2 ms |
| Alice beside Bob | 50 ms | 6.7 | 3.8, 5.2 ms | 21.9, 31.0, 33.2 ms |

A rewind costs about 2–2.6 ms of restore and bookkeeping plus 1.7 ms a
replayed tick when the room holds only the guest, 2.4 ms when the host's
player fights in it (the debug build: about 9 ms a tick). Every late input
that differed was rewound (none refused). At a 100 ms round trip a rewind
frame costs 19–22 ms on average and up to 31 ms at the 95th percentile,
more than a 60 Hz frame's 16.7 ms, on about one frame in seven while the
guest plays. So the late-input rewind is affordable on a LAN-like link and
not on an internet one: a guest that stamps its input ahead of the host
(NA-3's run-ahead) removes the rewinds of a guest that does not predict
itself, and is also what the confirmed-frame observers above wait on.

## Run-ahead: a guest's input arrives before its tick (2026-11-18)

A guest now stamps its input `lead` ticks past its estimate of the host tick
(`ambition_hosted_world::input_clock`). The host notes how early each
admitted input arrived (its stamped tick less the next tick to run) and
every `MARGIN_REPORT_TICKS` (15) sends each guest its latest-arriving input
of the window, by sequence (`InputMargin`, unordered). The guest reads the
report against the lead that sample was stamped with, so a report that
crossed a change of lead does not move it twice, and sets its lead so its
inputs arrive `ARRIVAL_SLACK` (1) tick early, up to `MAX_RUN_AHEAD` (30
ticks, about a 500 ms round trip). A stamp never goes back: a lead that
shrinks waits for the estimate to catch up. The host admits stamps up to
38 ticks ahead and already buffered inputs for ticks it has not run, so
nothing else changed on the host. A guest that sleeps sees its input's
result a round trip and the slack after the press either way: with
rewinds the input lands in the past and is seen a round trip later; with
run-ahead it lands on time and is seen a round trip and one tick later.

Unit test `the_lead_settles_where_inputs_arrive_with_the_slack`: a simulated
link of 0 to 12 ticks each way settles every input one tick early, with
stamps that never go back.

The real-link measurement again (release, 600 frames, Bob turning every 13
frames):

| Scene | Each way | Lead settled | Rewinds (before → now) | Host update without a rewind (mean, p95) |
|---|---|---|---|---|
| Alice in the hub | 0 ms | 3 | 84 → 3 | 4.4, 5.1 ms |
| Alice in the hub | 25 ms | 5 | 85 → 3 | 4.3, 5.7 ms |
| Alice in the hub | 50 ms | 9 | 83 → 4 | 4.7, 5.8 ms |
| Alice beside Bob | 0 ms | 3 | 83 → 3 | 3.9, 5.0 ms |
| Alice beside Bob | 25 ms | 5 | 85 → 3 | 4.0, 5.3 ms |
| Alice beside Bob | 50 ms | 8 | 84 → 4 | 4.4, 5.5 ms |

The 3 or 4 rewinds left are the same count at every delay, as the window
before the first report (15 ticks and a round trip) would leave; a link
with jitter beyond the slack would add more, and the rewind stays as the
correction for them. The confirmed-frame observers can now be given a
boundary that keeps up while a guest plays: that is the next step.

## The live confirmed-frame boundary on a hosted host (2026-11-18)

A host that rewinds guest rooms now publishes `ConfirmedFrameBoundary` at
the head of every live tick (`publish_live_boundary`, in
`ConfirmedFrameBoundaryPublished`, before `GameplaySimulationRoot`), so the
irreversible observers that already read it for GGRS (checkpoints, deaths,
passages, the conversation ledger, host intents, persistence's
`world_state_is_confirmed`) act on a tick only once no late input can make
the host replay it. A tick is confirmed for a guest seat once an input
stamped after it has arrived: the host clamps a stamp that goes back to the
seat's latest, as it clamps one before the seat's ownership, so nothing for
an earlier tick can come after it. A seat with no input yet holds the line
before its ownership began; a tick past the rewind horizon is final
whatever arrives. The line is the least over the guest seats.

A replay keeps its own boundary: the ticks it runs are unconfirmed (a
crossing it records waits for the first live tick, as before), and so is
anything the live boundary had not confirmed, since another guest's input
can still be late. The live boundary stands again after the replay.

An input stamped for the very next tick leaves that tick unconfirmed while
it runs (a second sample for it could still arrive after it), so a guest is
fully confirmed only when its inputs arrive a tick early, which is the
run-ahead's slack. Witnessed in both late-input scenes
(`a_guests_late_input_rewinds_only_its_room_on_the_host`,
`..._a_room_the_host_shares`): on time, the boundary trails the live tick
by 1; three frames late, by up to 4; no rewind in either replays a tick the
host had published as confirmed; and the ends still match the on-time run.
Those scenes now rewind 22 times over 66 ticks, not 26 over 78 as written
above: the same at the commit before this change, so an earlier change to
the input path moved it, not the boundary.

Over the real link with run-ahead (release, 600 frames): the host's boundary
is fully confirmed after 582, 575 and 547 frames with Alice in the hub and
580, 566 and 560 with her beside Bob, at 0, 25 and 50 ms each way. Fewer as
the round trip grows, as the window before the guest's lead settles grows
with it; so persistence and the other observers that wait for a fully
confirmed world keep up while a guest plays.

## Review fixes, 2026-11-16

`74e86f475` keeps other rooms' saved history during replay, isolates message
streams and reader positions, and suspends foreign narrative work. The guards
are the two-guest-room, pending-heal, and active or queued narrative tests in
`a_guests_room_rewinds_on_the_host`. The narrative API still permits one active
conversation and one active cutscene per world.

`dc98fbc71` gives input samples a sequence separate from their tick estimate.
Samples in one tick keep all button edges and the latest held levels. Timestamp
bounds use saturating arithmetic. Seat retirement clears prediction and input
history. Allocation consults the session roster and retries waiting clients.
The reconnect witness uses a fresh sequence after a held movement input.

**Open: no guard enforces the room-owner rule (filed 2026-11-16).** Room-local
simulation is being adopted faster than global gameplay resources are being
partitioned (Jon, 2026-11-16). The [room replay contract](../../systems/room-replay.md)
says a room-owned resource's writer asks `RoomReplay::allows(owner)` (47 call
sites today). Nothing finds a sim-schedule writer of a global `Resource` that
does neither that nor a shared-store retraction, so the next one is found by a
cross-room desync, as these fixes were.

**Census and ratchet, done (2026-11-17).**
`scripts/checks/check_sim_schedule_resource_writers_are_room_classified.py`
(a `run_tests.py` job) classifies each `ResMut` binding of a sim-schedule
system: room-gated (`.allows(`), room-stamped (`RoomWrite`), retracted (a
registered room-write retractor takes the write back: the retractor's own log,
a store this writer also logs to, or a store diffed whole at the tick's tail),
derived (a ruled type the scan checks is rebuilt each run, not accumulated),
instrument (a census, trace, capture tool or presentation-only state), or
world-owned (host state a rewind does not change, by the type's own doc), or
unclassified. Measured: 228 bindings; 11 room-gated, 14 room-stamped, 22
retracted, 59 derived, 26 instrument, 2 world-owned and 94 unclassified over
37 types. The ceiling is 94 and may only fall. **Open:** those 37 types are
real state, not projections, so each needs a code change that the room-replay
acceptance tests (`a_guests_room_rewinds_on_the_host`) must witness: a
`RoomReplay::allows` gate, a room-stamped log, or a world-owned writer gated
`not(resource_exists::<RoomReplay>)` as the cutscene runtime is. The largest
are `AmbitionGameSave` (writers other than the boss-defeat retraction),
`QuestRegistry`, `PendingLifecycleCommit` and `SlotInteractionState`; the
clocks (`SimTick`, `ClockState`, `DayClock`) and the seat input frames
(`SlotControls`, `SeatRawFrames`) want a ruling on whether a room replay runs
them.

**Gravity turns rewind with their room (2026-11-17).** The census missed one
writer because it writes through a deferred world command: a `FlipGravity` or
`SetGravity` switch turned its room's `BaseGravity` entry, and a rewind of
that room kept the turn while the replay pressed the switch again, so a flip
was applied twice. The switch arms now write through `BaseGravity::turn_from`,
which logs the room, the tick and the replaced entry in `GravityTurnsInRooms`
(rollback-registered, schema 478); `retract_room_gravity_turns` puts a rewound
room's turns back. Witness: `a_rewound_room_puts_back_its_own_gravity_turns`.
The census reads only `ResMut` parameters, so a write through
`commands.queue` is still invisible to it.

**Death fees rewind with their room (2026-11-17).** A room replay restores a
body's wallet, but `DeathFees` was one global ledger: a speculative death the
replay does not repeat left its fee or parcel behind, so the coins were both
in the wallet and on the floor. Each change to a participant's fee or parcel
now logs what it replaced in `DeathFeesInRooms` (schema 479), and
`retract_room_death_fees` puts a rewound room's changes back. Witness:
`a_rewound_room_puts_back_its_own_death_fees`. Census: 91 unclassified.

**The host's clock, the gate portals, and each session's logs (2026-11-17).**
The census now reads the room host itself: the types in `room_rollback.rs`'s
saved `Clock` and the seat inputs `rewind_room` copies out and puts back are
"host-rewound". `RequestedClockScale` joined the saved clock: a slowdown is a
target the clock holds, so a rewind that took back the request kept the
target. Gate portal phases were one integral for every room: a room replay
ticked the other rooms' portals again and did not set its own back. The tick
now runs only the replayed room's portals (`RoomReplay::allows`, the room
whose spec holds the zone) and logs each change in `GatePortalPhasesInRooms`
(schema 480) for `retract_room_gate_portal_phases`. A room's reset of its
gravity is logged like a switch's turn. And no `*InRooms` log was emptied
when a session began, while the room numbering and `SimTick` start again:
`register_room_write_log` installs each log and empties it at
`SessionScopeSet::Activate`. The portal straddle eviction's
`PortalFrameHistory` was replaced whole each tick: a replay dropped the other
rooms' frames, and its first tick compared the room's restored portals with
the frames the rewind left, so a portal moved in between read as one that
closed under its bodies. During a replay it now compares and replaces only
that room's frames, and logs each change in `PortalFramesInRooms` (schema
481) for `retract_room_portal_frames`. Rulings: the developer's gravity cycle
(an outside act a replay does not read again), `ProjectileSeqCounter` (an
order, not an identity), `ControlFrame` (the primary seat's mirror),
`RoomTransitionContentEpoch` (content identity a restore does not bump), the
scene capture tool, and a system registered with
`run_if(not(resource_exists::<RoomReplay>))` counts as room-gated. Census: 60
unclassified over 22 types.

**Hits on players and seat gestures (2026-11-17).** `PendingPlayerHitEvents`
crosses one tick, and the first tick of a replay drained it whole: the other
rooms' victims were disabled, so their hits were lost. A replay now keeps the
hits whose victim it cannot see for the live tick they were staged for, and
`retract_room_pending_player_hits` drops the rewound room's (they are the
discarded timeline's; the replay stages its own). The host's saved clock
gains `SlotInteractionState` (interact buffers and double taps are timed on
the clock); after the replay `rewind_room` puts back the live gestures of the
seats outside the room. Census: 52 unclassified over 20 types. Open:
`PossessionState` holds entities, so it cannot ride with the clock without
the snapshot's entity remap; `AmbitionGameSave` (13 writers),
`PendingLifecycleCommit`, `QuestRegistry`.

**Switch presses rewind with their room (2026-11-17).** A press toggles a
save switch, and a replay presses it again, so a rewind toggled it back. Each
press's write now logs its room, tick and replaced value in
`SwitchPressesInRooms` (schema 482); `retract_room_switch_presses` puts a
rewound room's switches back. Census rulings by system: the save's four
`persist_*` mirrors (derived), `track_room_visits` (a liveness fact), the
session-start loaders behind their own flag, and `ResolvedSwitchActivations`
(replaced each tick). Census: 38 unclassified over 16 types.

**Room entry and custody through a replay (2026-11-17).** During a replay only
the replayed room is live, so `push_room_entered_quest_events` shrank its
memory of the live rooms to that one and announced the others as entered
again afterwards; it is now registered off replays, as the cutscene entry
trigger is (the census reads `run_if(not(resource_exists::<RoomReplay>))`
however it is wrapped). `project_custody_onto_authored_occurrences` rebuilt
the custody set from the carriers it could see, so the other rooms' custody
left the ledger for the length of a replay; during a replay it now counts the
hidden (`Disabled`) carriers too. Rulings: `keep_life_ledger` and
`record_placed_bodies` mirror bodies the room snapshot restores. Census: 34
unclassified over 14 types. **Open, a design question:** a checkpoint commit
is a world-owned operation (`commit_authored_checkpoint`,
`heal_save_shrine_system`, `run_gate_network`'s save writes,
`PendingLifecycleCommit`'s crossings and returns). The contract says such
operations wait for the host's next normal tick, but the shrine's rest heals
a body (room state) in the same press, so gating the system would drop the
replayed press. It wants a split of each into its room half and a queued
world half.

**No room replays alone during a match (2026-11-17).** A match is one contest
across every seat: `count_the_live_match_ticks` would count a replayed room's
ticks a second time (and its item cadence would see the last live step's
crossing on every replayed tick), and `decide_stocks_match` and
`settle_versus_round` settle the whole cast. None of it is any one room's.
`register_room_replay_refusal` (beside the retractors in `room_writes.rs`)
names the states in which the host may not replay a room;
`take_arrived_inputs` drives a late input from the next tick instead, as it
does during a crossing. The match lifecycle refuses while a match is prepared
or active, the versus stage while its route is played. Versus netplay rewinds
the whole world instead, so nothing is lost. Census: 29 unclassified over 10
types.

**Two more off replays (2026-11-17).** `trigger_cutscenes_in_volumes` is
registered off replays as its two siblings are: it asks on every tick a body
stands inside, so the first live tick after a replay asks again (it loses only
a body that crossed a whole volume inside the replayed frames).
`reset_cut_rope_boss_arena_on_room_reset` advances the heavy-object cycle, a
world value a dialogue line branches on; a reset is admitted only on a live
tick, and the system is now registered off replays so that does not rest on
the schedule's order. The census reads a replay gate after other chained
conditions. Census: 27 unclassified over 8 types.

**Checkpoint commits are live-only, and a horizon (2026-11-17).** This settles
the checkpoint part of the open question above. A commit is the world's
agreement on every room at once, and every domain takes its baseline from it;
run in a replay it would capture those baselines with the other rooms hidden.
So: (1) the room rollback host drops every room's history when a commit lands
on a live tick, so no replay runs a live commit again, and a late input for an
earlier tick drives its seat from the next one; (2) during a replay the shrine
heals (the room half replays) and holds its rest in `PendingAuthoredCheckpoint`
(`rest: true`), and `commit_authored_checkpoint`, now registered off replays,
commits it on the next live tick with the revival of dead-until-rest bodies. A
temporary probe measured both halves: healed in the replay with the flag still
set and no commit, then one commit and the flag cleared on the live tick. Only
the shrine crate writes `CheckpointCommitted`, so its readers
(`compact_ended_mints_at_checkpoint` and the baseline captures) act on live
ticks only. Census: 22 unclassified over 6 types. Still open: `run_gate_network`'s
save writes and `PendingLifecycleCommit`'s crossings.

**Provocations rewind with their room (2026-11-17).** An NPC's durable
provocation (`npc_<id>_hostile`) was written with no room, so a provocation a
replay took back left the NPC hostile in the save. `NpcProvocationChanged`
carries the NPC's live room, and the runtime applies it through the same
logged write as `apply_flag_effects` (`apply_npc_provocations`); the
persistence reducer stays for compositions without room rewinds. Probed: a
room-stamped provocation is taken back by its room's rewind, an unstamped one
is not. `apply_quest_advance_events` is retracted through its events' causes;
the census now names such reducers (`CAUSE_RETRACTED`). Census: 18
unclassified over 6 types.

**Two more room flags logged, and the payout follows its quest (2026-11-17).**
`celebrate_symmetry_attunement` set its flag directly and
`retire_rewards_for_rearmed_encounters` cleared a looted flag directly, so a
replay that took back the clear or the switch press left the save changed.
Both now log the write in the act's room through
`FlagWritesInRooms::log_write` (the celebration through the runtime's
`write_logged_flag`, in the same tick: a flag request written in Progression
would be read in the next tick's GameplayEffects, and a replay's last tick
drops it). `grant_quest_completion_rewards` follows the registry both ways and
counts as a mirror. Census: 14 unclassified over 5 types.

**The census misread three writers as room-gated (2026-11-17).** Its gate test
was a substring: `RoomReplayRequested` and `RoomReplayAdmitted` (a room's own
reset) read as the replay's scope, so `admit_room_replay`, `drive_departures`
and `emit_cut_rope_room_replay_after_the_conversation_ends` counted as gated,
and any `.allows(` (`PortalRooms`, `BarkDraw`) counted as `RoomReplay::allows`.
The test is now the whole word. The honest count is 17 over 6 types, and the
ceiling rose to it. What it shows: every lifecycle operation (a room reset, a
crossing, a party return) can be admitted inside a replay. A death's
consequence fires once and is spent on the body, so its request must not die
with the replay's private message stream.

**Lifecycle intents in a replay commit on the next live tick (2026-11-17).**
Read again, the design already holds: during a replay `rewind_room` publishes
a `ConfirmedFrameBoundary` for the replayed ticks, so an intent the replay
records (a crossing, a reset admitted from a death's resume) is recorded at its
frame and commits on the first live tick after it, with every room visible.
The requests are read inside the replay, so none is lost with its stream. What
was missing was the other direction: a live intent's tick could fall inside a
later replay's window. The host now keeps no room history while an intent is
recorded or loading, as it already refused rewinds then, so no replay runs a
live intent again. (Holding the recorders off replays was tried and dropped:
it loses a door or return press that the late input carried.) The census
rules `PendingLifecycleCommit` and the cut-rope retry world-owned. Census: 8
unclassified (the gate network, possession, the Hall of Bosses switches).

**Every writer in the rewinding schedule is classified (2026-11-17).**
`run_gate_network` runs off replays: a link joins two rooms and runs out on
world time, which a replay's ticks counted a second time; its dials are the
host's intents, read on live ticks. A traversal only sets a link's idle timer
back to full (`GateNetworkState::used`). `PossessionState` is each seat's (the
body it drives and the hold that takes another), so the host saves it with the
clock and, after a replay, keeps the live records of the seats outside the
room, as it does their gestures. The Hall of Bosses' switches set the life the
press publishes rather than toggling it, so a repeated press sets the same
state; a press a replay takes back keeps its effect, a recorded limit of those
practice switches. Census: **0 unclassified**, and the ceiling is 0: a new
global writer in the rewinding schedule must ask `RoomReplay::allows`, stamp a
`RoomWrite`, or get a ruling.

## Order, exclusions and integration

1. Ship **NET-INPUT-WIDTH independently**. Do not make packet savings wait for the entire island runtime; preserve the working global GGRS session until island rollback is genuinely proven.
2. **SIMULATION-ISLANDS** is selected (2026-11-10) as room-level rollback inside a hybrid world: rollback per live room while it stays cheap and easy to maintain, hosted authority for what crosses rooms. If its measured cost or upkeep stops paying, the fallback is hosted authority inside the room too (guest prediction and correction, NA-3), not the App-wide timeline. Do not reintroduce the retired barrier.
3. Do not turn cross-island live portals into a hidden prerequisite for scaled portals. Only the same-island live portal path needs continuous causal geometry; no dynamic island merging/splitting is scheduled.
4. Preserve existing game rules and deterministic gameplay, and distinguish local simulation from optional GGRS and from a future player-hosted authoritative replication backend. The old [ADR 0037](../../adr/0037-causally-isolated-simulation-islands.md) compared broader possibilities; this maintainer-selected implementation scope is deliberately narrower.
5. Retire each selected queue task when its *production* acceptance is met. Do not claim NET-INPUT-WIDTH complete from a byte-count test alone; do not claim conditional GGRS islands complete from local-only room filtering.

## Narrative per room: the plan (2026-11-20)

The last open coupling is narrative. `ActiveConversation` holds one
conversation for the world, one `DialogueRunner` runs it
(`ambition_dialog::bridge`), and one text box draws it. So Bob, in another
room, cannot open a talk while Alice is in one, and he waits on her. That is
what the customer rules out ("must not wait on each other's rooms"). The
cutscene has the same shape (`ActiveCutscene`, `CutscenesInRooms`).

Slices, each with a witness:

- **N1, the simulation.** `ActiveConversation` keeps at most one live
  conversation per room: the room of its bodies. Opening a second in the same
  room is refused as now; one in another room opens. Every reader asks
  for the conversation of a body, a seat or a room, not "the" conversation:
  the hold, membership, the break rules, input routing (`input.rs`, the seat
  answers its own conversation), the ledger and `yarn_vocabulary`. Then
  `ConversationsInRooms` logs and retracts per room. The rule "a later change
  of another room's stands" goes away, because rooms no longer write one value.
  Witness: Alice and Bob talk at once in two rooms, each advances only on
  its own seat's input, and a rewind of Bob's room leaves Alice's talk as an
  on-time run has it.

  **N1 done 2026-11-20.** The key is the seat and the body, not the room:
  two live conversations never share a seat or a body, and opening one
  replaces any that does. Per room would still make two seats in one room
  wait on each other, and N3 draws a box per seat, so the seat is the
  natural key. `ActiveConversation` holds a list. Readers ask by body
  (`of_body`), seat (`of_owner`, `of_seat`) or instance (`of_instance`). The
  break and interrupt rules, the talk spacing, the crossing, input routing
  and the durable-horizon visit count go over every live conversation.
  `ConversationsInRooms` logs a change per seat and a room's retraction puts
  back only its own seats' talks. A runner command, the ledger's end stamp
  and membership act on `PresentedConversation`: the instance the one runner
  presents, read through `RunnerConversation`. Opening a second seat's talk
  is still refused in `DialogueDispatch::open_between`. With one runner and
  one box, that talk would hold its seat's input with nothing to answer it,
  so the refusal stays until N2 and N3. Witnesses (`ambition_conversation`
  tests): `two_seats_talk_at_once_and_one_ending_leaves_the_other` and
  `a_rooms_rewind_puts_back_only_its_own_seats_conversation`.
- **N2, the runner.** There is one `DialogueRunner` per live conversation.
  They share one variable storage, because Yarn variables are the save's
  flags. Commands and functions are registered on each runner
  (`ambition_dialog::bindings`). A command names its conversation's seat and
  room, as grants and sales already do.

  **N2 done 2026-11-20.** The main runner presents through the `DialogState`
  resource, as before; each further live conversation runs on a `DialogSlot`
  entity (its own `DialogState` and its own `DialogueRunner`), named by
  `SlotConversation`. The projection keeps each runner's conversation until it
  ends, gives the main runner the first that needs one, and reuses a free slot
  before it makes another. The runners share one Yarn variable store
  (`ConversationStorage`), except the participants (`$speaker_id`,
  `$listener_id`, `$speaker_is_self`), which are each runner's own; a runner
  made later fills only the declared values the store lacks, so it does not
  reset another conversation's `<<set>>`s. Every command and function a runner
  holds is wrapped to name it as the `SpeakingRunner` while it runs, so
  `RunnerConversation`, `NarrativeInputWriter` and the world-access functions
  resolve the conversation of the runner that fired them, with no change to
  content's vocabulary. A slot's line, options and end route to its own state;
  its input comes from its own seat (`slot_dialog_input`), and its end is
  stamped per instance (`write_once_for`). Opening a second seat's talk is still
  refused in `DialogueDispatch::open_between` until N3 draws a slot's box.
  Witnesses: `two_conversations_run_at_once_each_on_its_own_runner`
  (`ambition_dialog`: two runners at once, the shared `$met`, each runner's own
  listener, each command naming its runner) and
  `a_second_conversation_runs_on_its_own_slot_and_its_commands_are_its_own`
  (`ambition_conversation`: the slot, the command's conversation, a freed slot
  reused).
- **N3, the presentation.** `DialogState` is per seat. Each view draws the
  conversation of its own seat, so a split view draws two boxes and a hosted
  host draws its own seat's. A guest's box is the hosted-world experiment's
  question (replicated narrative), not this slice's.

  **N3 done 2026-11-20.** Each slot has a `SlotDialogView` beside the main
  `DialogView` (`rebuild_slot_dialog_views`). Each overlay root carries a
  `DialogBoxSource` naming the view it draws, and `DialogViews` gives a
  presenter every view; the default box and Ambition's box keep their layout
  and portrait playback per box, and each box sits over the view of the seat
  that holds it. The pointer region answers the main box. The refusal in
  `DialogueDispatch::open_between` is now
  `ActiveConversation::held_by_another_seat`: a talk is refused only while one
  of its bodies is in another seat's conversation. The `Dialogue` game mode is
  entered only with one driving seat and one live room, so a shared session
  never pauses for a talk. Witnesses:
  `each_seats_conversation_draws_its_own_box_over_its_own_view`
  (`ambition_render`) and `two_players_talk_at_once_each_in_its_own_box`
  (`app_it`: player one talks to a hen while player two is at Alice's
  choices; two boxes, each answering only its own pad, and the hen's box goes
  with its talk).
- **N4, the cutscene.** The same for `ActiveCutscene`: one per room, each
  holding only its own room's players. It already ticks on its room's pace.

  **N4 done 2026-11-20.** `ActiveCutscene` holds a list of `PlayingCutscene`s
  in room order: at most one per live room, or one of the whole world, which
  plays alone. Each carries its own skip hold (the `CutsceneSkipHold` resource
  is gone; schema 522). The drain starts a trigger when its room has none
  playing (`can_start`); a second for the same room waits, and a world
  cutscene waits for every room's, holding back the triggers behind it. The
  tick plays each on its own answering seat and room pace, and in a room's
  replay only that room's. The hold projection holds a body when any playing
  cutscene holds its room, and in a replay touches only the replaying room's
  bodies. `CutscenesInRooms` logs each room's cutscene apart, so a room's
  rewind puts back its own and no later change of another room's blocks it.
  The camera frames a view by the cutscene of its room. The overlay draws one
  per cutscene, over the view of its answering seat on a split screen, and
  the line of each cutscene beyond the one the main box shows gets a
  `CutsceneLineView` box of its own. Witnesses:
  `a_rooms_rewind_puts_back_only_its_own_cutscene` (`ambition_cutscene`),
  `each_rooms_cutscene_plays_at_once_and_a_second_waits_its_turn` and
  `each_rooms_cutscene_answers_only_its_own_rooms_seat` (runtime), and
  `two_rooms_play_their_cutscenes_at_once_and_a_rewind_puts_back_only_its_own`
  (`app_it`).

Smash keeps one match narrative; nothing here applies to a match.
