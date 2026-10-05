# Awaiting a maintainer decision

This file holds only open maintainer and product questions. Engineering work
that can proceed without a ruling goes in [`queue.md`](queue.md).

- Give each question one unique `Q<number>`. Do not reuse a retired number.
- Use the heading shape `## Q<number> — <question>`. Other documents link to
  these anchors, so do not change the text of a heading.
- Each entry states the question, the context needed to choose, the owner, and
  what changes for each answer.
- A queue row that waits on a question names it in a `**Blocked by:**` field.
- When a question is answered, record the ruling in
  [`maintainer-decisions.md`](maintainer-decisions.md), update the owner plan or
  source, and delete the question here.
- Before you file a question, make sure that no ruling already decides it. A
  decided direction with unfinished implementation is a queue row, not a
  question.
- Do not file "nothing uses X: keep or delete?". Current usage is not a deletion
  criterion (Q74 ruling, 2026-10-03); ask about meaning, authority or cost.

## What actually blocks architecture work today

This is the blocking set, not the important set. A question can matter and
block nothing.

**The blocking set is empty (2026-10-01).** No open P0/P1 queue row states a
`**Blocked by:**` question.

| question | what it blocks | and if it stays open |
|---|---|---|

`scripts/check_blocking_set_names_every_gate.py` reads the queue's
`**Blocked by:**` fields and fails if this table has no row for a gate. Each
table row links a `Q` that has a section on this page with at least two
options, written `* **(a) …`. Derive gates from the field, not from row prose.

One question looks like a blocker and is not:

- [`Q142`](#q142--must-every-presence-filtered-component-be-rollback-registered)
  appears in `ID-PEER` only as history.

## Gameplay and content

## Q36 — what are the authored standing heights of the puppy slug, stochastic parrot and burning flying shark?

The engine has a canonical-height contract; these remaining authored characters
need product values rather than inferred sprite dimensions.

## Q79 — how far may the camera zoom out before the fight stops being legible?

Choose the product legibility floor. Camera policy can then clamp against a named
limit rather than an arbitrary tuning value.

## Q153 — how does a second player join Ambition?

Split from Q151 when its death rule was decided (2026-10-03, see
[`maintainer-decisions.md`](maintainer-decisions.md)). Ambition has no join
road: only the primary seat gets a body in production (`avatar/bundles.rs`),
and the only production code that seats slot 1 is Smash's match activation.
The death rules count only `PlayerEntity` as a participant
(`session/death.rs`), so a seat-driven body that dies takes the enemy road
(`actor_hit.rs`): a "defeated" banner, a bounty coin and its authored respawn
policy. In the two-player fixtures the second body is a placement of its room,
so a rebuild of that room builds the body again without its seat; a join road
must give the seat's body a home that a rebuild does not replace, as
possession does through custody.

Measured 2026-10-05 on the world hot reload
(`a_world_reload_moves_the_body_that_stays_out_of_the_new_solids`): a
seat-driven body that was spawned while the game ran, and is not a placement,
is removed with its room and is not built again, and the room stays live with
no body in it.

Owner: the join road row in
[open-world runtime and residency](engine/open-world-runtime-and-residency.md).

* **(a) Join at the primary player's room:** the new seat's body is stamped
  into the room the primary player is in.
* **(b) Join at a chosen room or shrine:** the joining player picks where to
  enter, from the rooms the session has visited.
* **(c) Join at the start room** of the current save, independent of where
  the primary player is.

## Q154 — should a pickup that authors no policy be gone for the run once taken?

Filed 2026-10-04. Blocks nothing: the engine reads the default as
`OnRoomReload` today and serves an authored `Never` (open-world "Regrowth/restock").

`HazardRespawn`'s documented default is `Never` ("never respawn inside the
current run/session"), but until 2026-10-04 no pickup kept any record once
taken, so every pickup came back when its room was built again. Now a pickup
authored `Never` is remembered `Consumed` in the occurrence ledger (and in the
save); a death brings back only one taken after the checkpoint. A pickup that
authors nothing reads `OnRoomReload`, which keeps today's behaviour: of the
521 shipped `PickupSpawn`s (intro 9, sandbox 29, Mary-O 25, three Sanic
worlds 458; counted 2026-10-04 by parsing the `.ldtk` files), only the two
`basement_breakables` hearts author a policy, and `PickupSpec::new` /
`Pickup::new` now say `OnRoomReload`.

Owner: the regrowth/restock row in
[open-world runtime and residency](engine/open-world-runtime-and-residency.md).

* **(a) Keep `OnRoomReload` as the pickup default (current).** Hearts and coins
  come back on every visit; a designer writes `Never` for a one-time pickup.
* **(b) Make `Never` the default for authored pickups.** The persistent world
  remembers every pickup taken. Mary-O's level loop and Sanic's act cycle
  would then need `OnRoomReload` on their coins and rings (or a replay that
  restores them), or a second lap finds them gone.
* **(c) Per-world default:** a world declares its pickup default (Ambition
  `Never`, the arcade demos `OnRoomReload`).

## Architecture and engine policy

## Q94 — what residency-memory limit should the runtime target?

Needs a maintainer/hardware/product value. The residency mechanism can enforce a
budget once the budget exists. Report source, decoded CPU, prepared simulation
content and device residency separately. A8 instance isolation and A9 dependency
closure do not supply a hardware budget.

## Q135 — reopened 2026-10-03: the save is applied by the simulation, not before the timeline

The 2026-09-16 ruling (`maintainer-decisions.md`, Q135) says GGRS does not start
before the durable restore finishes, and `maintain_local_session` refused to
create a session while hydration was pending. On 2026-10-03 the restore chain
moved into the simulation schedule and the gate was removed
(BODY-BORN-ON-THE-TIMELINE in [`queue.md`](queue.md)). New evidence:

- Three compositions build their primary body on the timeline (the Sanic and
  Mary-O rollback fixtures). The gate has no body to wait for, so the save was
  applied from `Update` over a live timeline there.
- `AmbitionGameSave`, `SaveRestored` and every value the chain writes are
  rollback state, so a restore inside the simulation is deterministic: a rewind
  past it applies it again on the same tick.
- With the chain in the simulation the gate would deadlock: under the rollback
  host the simulation does not run until a session starts.

Whether the save belongs in what two peers agree on is decided (Q129,
2026-10-03, [`maintainer-decisions.md`](maintainer-decisions.md)): shared
durable world state is peer state, compared by its canonical semantic form.

The ruling's guarantee holds: no tick is simulated over an unapplied save. The
restore runs at the head of the first tick that has the body, before the core
step. `a_conversation_on_the_first_tick_of_a_session_is_counted_exactly_once`,
the test the ruling names, passes without the gate.

* **(a) Keep the restore in the simulation** (current). One road for every
  composition; the session may start before the save is applied.
* **(b) Restore the gate and the `Update` chain**, and give compositions whose
  body is born on the timeline another road. That is two roads for one fact.

## Q155 — is a hold of about 0.6 s at a peer room crossing acceptable, or must the crossing keep its session?

Under a peer session a room crossing now commits (2026-10-04, the "Remote
peers" row of
[`open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md)).
The simulation of each peer is held from the frame after the crossing was
recorded until the first frame of the next peer session. Measured in
`two_peers.rs` at a link latency of 3 updates each way (four runs, two
crossings): 23 to 43 updates, which is 0.38 s to 0.72 s at 60 Hz. 2 to 7 updates
are the wait for the confirmed frame and the readiness of the room. 21 to 36
are the handshake of the next GGRS session, which is five round trips
(`NUM_SYNC_PACKETS`, a constant of the pinned GGRS), so this part grows with
the latency. The hold is of the whole world: the room that nobody leaves stops
too. A local (sync-test) crossing holds nothing.

Owner: netcode ([`netcode.md`](engine/netcode.md)) and online play (A4 in
[`multiplayer.md`](game/multiplayer.md)). No queue row waits for this: the
crossing works, and the question is its cost.

* **(a) Keep it** (current). A door between rooms is a pause of about half a
  second for each online player. No new machinery.
* **(b) Shorten the handshake and keep the new session.** A session that
  follows another between the same peers needs one round trip, not five. That
  is a change in GGRS (a fork or an upstream option). Estimate: the handshake
  goes from 21..36 updates to about 8, the hold to about 10..15 updates.
* **(c) Keep the session (shape B).** Readiness bits ride in the input, the
  inputs of the held frames are a canonical null, the host stalls a few frames
  and runs the operation with no rebase. Estimate: a hold of about 6 frames
  plus the latency. It changes the wire input, it commits with no rebase (the
  old ring slots hold the room before the operation and stay safe only because
  no rollback can reach them), and the freeze of the whole world stays.

None of the three ends the whole-world hold. A hold of only the rooms of the
crossing is a separate change (the simulation gate is one condition for each
live room today).

## Q156 — when one online player's machine cannot prepare a room, what happens to the door, and to the respawn?

Under a peer session each peer prepares the room of a lifecycle operation on
its own machine, and commits alone behind the peer barrier (the "Remote peers"
row of
[`open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md)).
A preparation can fail on one machine and not on the other. No rule says what
happens then, and today the game stops with no error.

Measured 2026-10-04 in the `two_peers` fixture (link latency 3 updates, 600
updates, a probe that is not in the tree). A machine "fails" when its asset
work item goes `Failed` on each transaction; only a host with presentation has
that item, a headless peer skips it. Eight predictions were written first and
each held.

1. **One machine fails, a door.** The good machine commits and waits at frame
   0 of its next session for the rest of the run: its handshake has no
   partner. The failing machine stays in the old session, held, and its frame
   counter stops 8 frames past its confirmed frame. The two worlds differ
   (one has the player in the next room). `rollback_health()` is `Ok` on both:
   no desync is reported, because no checksum crosses two sessions. The
   failing machine opens a new transaction each two updates. The same result
   when it is the other machine that fails.
2. **One machine fails, a respawn** (a death gives a checkpoint operation).
   The same result. The failing machine also keeps a note that its
   preparation failed (`AbandonedCheckpointOperation`), which nothing spends
   under a peer session.
3. **Both machines fail** (a failure that each machine has: a room that cannot
   be built). Both stay in the old session, held, with no end. The worlds are
   equal and nothing is reported. On one machine (a sync test) the same
   failure does less harm: a failed door does not open and play goes on, and
   a failed respawn is cancelled.

Not measured: a host with a window (its failed transaction stays on the
loading screen for a retry or a cancel), and the disconnect timeout of GGRS
(no production code starts a peer session, so there is no value and no
handler for a lost peer).

**Built 2026-10-05 (the engineering half):** each peer puts its verdict
(`NotYet`, `Prepared` or `Failed`, for the operation that waits) in its input
(`PeerInput`, wire identity 4), and a peer commits only when each handle said
`Prepared` in a confirmed input ([`netcode.md`](engine/netcode.md)). Measured
by `two_peers::a_peer_does_not_commit_a_crossing_the_other_peer_could_not_prepare`:
when Bob's machine cannot prepare a door, neither peer commits, both stay held
on one frozen world, each peer knows that handle 0 said `Prepared` and handle
1 said `Failed` (`PeerVerdicts`), and the commit logs an error that names the
handle. Before the rule (the poison), Alice's peer committed alone into a
session that never started, as in (1) above. So outcomes (1) and (2) are now
outcome (3) with a report. What a `Failed` does next is still the question.

Owner: netcode ([`netcode.md`](engine/netcode.md)) and online play (A4 in
[`multiplayer.md`](game/multiplayer.md)). No queue row waits for this; no
production code starts a peer session yet.

**The door.** When a peer says "failed":

* **(D1) The door stays shut for each player.** The crossing is cancelled in
  the simulation, play goes on in the old room, and the players are told why.
* **(D2) The session ends for the player that failed.** The other player
  goes through alone.
* **(D3) Wait and try again, with a visible state and a time limit,** then D1
  or D2.

**The respawn.** When a peer says "failed":

* **(R1) The respawn is cancelled for each player.** ⚠ That is a death with
  no respawn: the body that died stays dead. One machine does this today
  (case 3 above).
* **(R2) The session ends for the player that failed.**
* **(R3) Wait and try again, with a visible state and a time limit,** then
  R2.

**For both:**

* **(X) Art does not hold a peer commit.** The failure that was measured is
  the art of the room (a presentation fact), and the simulation does not need
  it. Under a peer session the commit would not wait for the art, and a
  machine whose art failed would show the room with what it has. Then only a
  failure that each machine has (case 3) is left, and D1 or R1..R3 decides it.

## Q142 — must every presence-filtered component be rollback-registered?

Resolved by engineering. Kept only because
`scripts/check_presence_filtered_state_is_rollback_registered.py` (its
`ACKNOWLEDGED` table) still owes `PostBossNpc` and `SmirkingBehemothVictoryNpc`
to this section and fails if the section does not name them. Both are now
registered (`marker.post_boss_npc` and `content.cut_rope_victory_npc` in
`game/ambition_app/tests/rollback_schema_baseline.txt`), and the checker
reports 0 owed. Delete this section in the change that removes those two
`ACKNOWLEDGED` entries.

The rule is owned by
[simulation authority](engine/simulation-authority-and-determinism.md): a
component whose presence a query filter reads is authoritative, even when its
value is derived. To witness a presence latch across a rewind, census the latch
on each resimulated pass. A visible consequence that is not itself rollback
state cannot witness the defect.
