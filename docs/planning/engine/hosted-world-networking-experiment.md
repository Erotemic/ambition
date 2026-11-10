# Hosted world versus GGRS — the authority experiment

**Status:** NA-1 and a first NA-2 measurement done (2026-11-10); NA-2's guest view, NA-3 and NA-4 open.
**Execution:** [NET-AUTHORITY-EXPERIMENT](../queue.md#net-authority-experiment--compare-hosted-world-replication-with-ggrs).
**Research and candidates:** [network topology and authority](network-topology-and-authority-research.md) (models A–D, R0–R5, the comparison scenarios). This page owns the experiment's results, not the candidate list.
**Related:** [netcode](netcode.md), [compact peer input](island-rollback-and-compact-peer-input.md), [multiplayer and multiview](multiplayer-and-multiview.md), [ADR 0036](../../adr/0036-synchronization-is-an-optional-game-capability.md).

## The question

For persistent Ambition play with one to four participants who may be in
different rooms, does a **player-hosted authoritative world** (one machine
simulates; guests send inputs and receive state) cost less, or give a better
game, than the working **GGRS input-synchronized rollback**, in which every
peer simulates every live room? Smash keeps GGRS either way. The answer is a
per-game synchronization choice (NA-4), recorded in an ADR, and it is the
maintainer's to make from these measurements.

**Not in scope:** removing GGRS, adding a network dependency, or rewriting
working multiplayer before the comparison exists. No company-run server: a
player's own game hosts.

## Steps

- **NA-1 — GGRS baseline.** Link bytes, messages, rollbacks and update cost
  of the shipped GGRS path in the same scenes the hosted fixture runs.
- **NA-2 — minimal hosted fixture.** One host App with both seats and no
  rollback session; the guest's input arrives late and is applied once, in
  order; the host's state is replicated to the guest, scoped to the guest's
  room.
- **NA-3 — prediction, interest, NPC facts, late join.** The guest predicts
  its own body and reconciles to host snapshots; interest follows rooms; NPC
  state persists on the host; a guest joins a running world.
- **NA-4 — per-game decision.** A side-by-side matrix (R5 in the research
  page) and an ADR per game.

## NA-1 and NA-2 (2026-11-10)

Fixture: `game/ambition_app/tests/hosted_world.rs`,
`what_a_hosted_world_costs_beside_ggrs` (ignored; a measurement). Run:

```
cargo test -p ambition_app --test app_it --no-run
RUSTC_BOOTSTRAP=1 target/debug/deps/app_it-<hash> hosted_world --ignored --nocapture --test-threads=1
```

Each scene runs once hosted and once as a GGRS pair (`start_peer_session`
over `loopback_transports`, three updates of latency, prediction window 8,
no census instruments). Both topologies run the same scripts: Alice holds
right and attacks; Bob turns every 13 frames and jumps every 29, so GGRS
mispredicts him. In the hosted run Bob's input is applied three frames late.

Results of one run (2026-11-10). Update cost is milliseconds of App update per
simulated frame; GGRS is per confirmed frame, for each peer.

| Scene | Bodies (guest's room / world) | Host ms a frame | GGRS ms a frame (Alice / Bob) | GGRS rollbacks, resimulated frames | GGRS link bytes a frame, each way | Hosted: guest→host input bytes | Hosted: host→guest bodies, room / changed-only / whole world |
|---|---|---|---|---|---|---|---|
| `switch_lab`, inputs never change (control) | 2 / 2 | 8.4 | 34.7 / 32.6 | 1, 6 | 103 / 104 | 76 | 48 / 38 / 48 |
| `switch_lab`, Alice fights, Bob turns | 2 / 2 | 8.9 | 48.0 / 37.7 | 81, 152 | 108 / 109 | 76 | 48 / 38 / 48 |
| `central_hub_complex` with its NPCs | 18 / 18 | 9.9 | 47.6 / 45.2 | 80, 157 | 110 / 111 | 76 | 432 / 95 / 432 |
| Two rooms after Alice crosses to the hub | 1 / 18 | 12.8 | 50.3 / 38.6 | 40, 86 | 108 / 111 | 76 | 27 / 25 / 378 |

Read with the caveats below. In the two-room scene the host's mean includes
the crossing's room build (its worst update was 519 ms; each GGRS peer's was
over 600 ms).

### What the fixture does not do yet

- **No guest App.** The guest's snapshot is counted, not sent or applied, so
  the hosted column has no correction cost, interpolation or rendering cost.
  The guest's CPU is not zero; it is unmeasured.
- **Bodies only.** A snapshot is 24 bytes per body (`BODY_RECORD`: network
  id, position, velocity, facing). Animation, health, items, projectiles and
  room state are missing, so the snapshot bytes are a lower bound. The
  identified-entity count (everything with a `SimId` in the guest's room)
  bounds what a full replication would name.
- **No loss, jitter or acknowledgement.** Constant latency on a perfect
  link, for both topologies.
- **Debug build, one process.** Update times are wall clock in a debug test
  binary, with both topologies' Apps in one process; compare the ratio, not
  the milliseconds.

### What it says so far

- **Input width is not the GGRS cost.** NET-INPUT-WIDTH cut the input by
  52% and the link by 0.5–4% ([result](island-rollback-and-compact-peer-input.md#result-2026-11-10)).
  The link is about 3.8 messages a frame, about 105–110 bytes, in every scene.
- **GGRS pays most of its cost on every frame, not on rollback.** With no
  mispredictions (the control: one rollback in 240 frames) each GGRS peer
  still spent about 4 times the host's update per frame. Mispredictions add
  roughly a further 15–40% in these scenes.
- **Half of that per-frame cost is the snapshot, and it is per registered
  type, not per body.** One run of a peer's `SaveWorld` (run on its own after
  the session, 60 times) takes 16–17 ms in every scene, with 2 bodies or 18:
  the schedule holds **1,196 systems**, about one save and one checksum per
  registered rollback type, at roughly 13 µs each in a debug build. bevy_ggrs
  runs it on every frame it simulates, resimulated frames included. So the
  GGRS cost to cut first is the number of registered types, or the work per
  empty type, not the bytes of state. The remaining ~9 ms per frame (session
  polling, loads, derived rebuilds) is not split.
- **Each GGRS peer simulates every live room.** In the two-room scene both
  peers simulate the hub's eighteen bodies, although only Alice is there.
  The hosted guest simulates nothing in this fixture; with prediction (NA-3)
  it would simulate its own body in its own room.
- **Replication bytes depend on the room; GGRS bytes do not.** A full body
  snapshot of the hub (18 bodies) is 432 bytes a frame, four times the GGRS
  link; sending only the bodies that changed is 95. Scoped to a guest alone in
  a quiet room it is 25–27. Input synchronization is flat in the size of the
  world, and replication grows with what the guest can see, so the byte
  comparison turns on delta encoding and interest, not on the topology alone.

### Room rollback on the host (2026-11-10)

The hybrid direction (rollback within a room, hosted authority around it) adds
a third topology to these scenes: the host predicts the guest and rewinds only
his room when a late input says otherwise. It ends where an on-time run ends in
every scene, at 9.4–16.3 ms of host update a frame against 8.4–9.9 ms for
the late-applying host and 33–48 ms for each GGRS peer
([result](island-rollback-and-compact-peer-input.md#isl-2e-measured-2026-11-10-what-room-level-rollback-costs-a-hosted-world)).

## Next

1. **NA-2b — a guest App.** Apply the host's room-scoped snapshot to a guest
   App that runs no simulation of its own (interpolated remote bodies only),
   and measure its update cost and the visible delay of its own body. This
   needs a replicated-state schema: the body rows, then the identified
   entities, with spawn/despawn by `SimId`.
2. **NA-3 — guest prediction.** Run the guest's own body through the
   movement kernel against its room's collision, reconcile to each snapshot,
   and measure correction size under the same scripts. The kernel already
   runs per body (`transit_body`, `BodyKinematics`); the open question is
   what else a prediction needs (contacts, abilities, held items).
3. **NA-3 — NPC facts and late join.** Host-owned NPC state across a guest's
   departure and return, using the existing save/persistence authority; a
   guest joining a running world from a host snapshot.
4. **NA-4** once 1–3 have numbers.
