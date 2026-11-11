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
(`apply_item_grants`, `apply_shop_transactions`) are consequences of the one
conversation, an ISL-1b coupling, not of a room: they wait for the
conversation to be a room's. The census's "conditions' take" was a test's
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
enemy's death flag in `actor_hit.rs`) and `None` for the rest (a conversation's
talk flags wait on ISL-1b; a cutscene, an authored command, the intro chain
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
`room_writes.rs`. `NpcTalked` is a conversation's (ISL-1b).

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
   that freeze is back in the struck room. Still open: projectile ids
   sequenced per room (deferred to the state transfer; see ISL-2c above), a clock scale per room (Player 1's bullet-time slows
   Player 1's room; the scale reaches gameplay through `WorldTime::sim_dt`, read at
   about a hundred non-test sites), one conversation and one cutscene per participant or room.

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

## Order, exclusions and integration

1. Ship **NET-INPUT-WIDTH independently**. Do not make packet savings wait for the entire island runtime; preserve the working global GGRS session until island rollback is genuinely proven.
2. **SIMULATION-ISLANDS** is selected (2026-11-10) as room-level rollback inside a hybrid world: rollback per live room while it stays cheap and easy to maintain, hosted authority for what crosses rooms. If its measured cost or upkeep stops paying, the fallback is hosted authority inside the room too (guest prediction and correction, NA-3), not the App-wide timeline. Do not reintroduce the retired barrier.
3. Do not turn cross-island live portals into a hidden prerequisite for scaled portals. Only the same-island live portal path needs continuous causal geometry; no dynamic island merging/splitting is scheduled.
4. Preserve existing game rules and deterministic gameplay, and distinguish local simulation from optional GGRS and from a future player-hosted authoritative replication backend. The old [ADR 0037](../../adr/0037-causally-isolated-simulation-islands.md) compared broader possibilities; this maintainer-selected implementation scope is deliberately narrower.
5. Retire each selected queue task when its *production* acceptance is met. Do not claim NET-INPUT-WIDTH complete from a byte-count test alone; do not claim conditional GGRS islands complete from local-only room filtering.
