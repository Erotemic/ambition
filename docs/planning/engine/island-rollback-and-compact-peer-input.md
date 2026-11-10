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
`which_rollback_state_belongs_to_one_room` (ignored; a measurement). Run:

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

### Next (ISL-2a and ISL-1b)

1. **ISL-2a — what a room snapshot costs.** Save and restore one room's
   entities (archetypes holding `Rollback` and that room's `InRoomInstance`)
   plus its keyed resource slices, through the registered rows' own clone and
   checksum functions, and time it beside `SaveWorld` in the same scene. This
   is the number the keep-or-drop decision on room-level rollback needs. A
   lever that helps the App-wide timeline too: skip a row whose type no
   rollback entity carries this frame.
2. **ISL-1b — remove the couplings.** One change per coupling resource, each
   with a witness that an event in one room leaves another room's state
   unchanged: hitstop scoped to the room of the connect, projectile ids
   sequenced per room, a clock scale per room (Player 1's bullet-time slows
   Player 1's room), one conversation and one cutscene per participant or
   room. Then classify the 91 message rows by writer and reader room.

## Order, exclusions and integration

1. Ship **NET-INPUT-WIDTH independently**. Do not make packet savings wait for the entire island runtime; preserve the working global GGRS session until island rollback is genuinely proven.
2. **SIMULATION-ISLANDS** is selected (2026-11-10) as room-level rollback inside a hybrid world: rollback per live room while it stays cheap and easy to maintain, hosted authority for what crosses rooms. If its measured cost or upkeep stops paying, the fallback is hosted authority inside the room too (guest prediction and correction, NA-3), not the App-wide timeline. Do not reintroduce the retired barrier.
3. Do not turn cross-island live portals into a hidden prerequisite for scaled portals. Only the same-island live portal path needs continuous causal geometry; no dynamic island merging/splitting is scheduled.
4. Preserve existing game rules and deterministic gameplay, and distinguish local simulation from optional GGRS and from a future player-hosted authoritative replication backend. The old [ADR 0037](../../adr/0037-causally-isolated-simulation-islands.md) compared broader possibilities; this maintainer-selected implementation scope is deliberately narrower.
5. Retire each selected queue task when its *production* acceptance is met. Do not claim NET-INPUT-WIDTH complete from a byte-count test alone; do not claim conditional GGRS islands complete from local-only room filtering.
