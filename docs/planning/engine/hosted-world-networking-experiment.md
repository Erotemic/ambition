# Hosted world versus GGRS — the authority experiment

**Status:** NA-1 and NA-2 done (2026-11-10); NA-2b's guest App measured through spawns, deaths, drops, presentation, room scope and room following (2026-11-12); NA-3 measured (2026-11-12: prediction, reconciliation, late join, NPC lives); NA-4 recorded in ADR 0036 (2026-11-12).
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

### NA-2b, first measurement (2026-11-10): a guest App that simulates nothing

Fixture: `game/ambition_app/tests/a_guest_app_applies_the_host.rs`,
`probe_what_a_guest_app_costs` (ignored; a measurement).

The guest is a second App, booted to the same room with the same two seats,
so its bodies carry the host's `SimId`s. Every simulation phase in it is gated
off. Each frame it takes the host's body rows (position, velocity, facing)
from three frames earlier, writes them onto its bodies by id, and updates.
The host is NA-2's, which applies Bob's input three frames late. One run,
240 frames:

| Room | Host ms a frame | Guest ms a frame (worst) | Applying a snapshot | Ids on one side only | Bodies the guest moved itself |
|---|---|---|---|---|---|
| `switch_lab` (2 bodies) | 8.97 | 4.64 (326) | 0.080 ms | 0 | 0 |
| `central_hub_complex` (18) | 9.76 | 4.50 (293) | 0.114 ms | 0 | 0 |

- **A guest that simulates nothing still costs about half a host frame.** In
  a headless debug build, an App with every simulation phase gated off spends
  about 4.5 ms an update, in either room. That is the App and its shell, not
  the bodies. It is not split yet.
- **Applying bodies is cheap:** 0.08–0.11 ms for 2–18 bodies.
- **The gate holds:** no body moved on the guest between snapshots, so the
  guest draws only what the host sent.
- **Bob sees his own input six frames late** (three to the host, three back)
  before any interpolation buffer. That delay is what NA-3's prediction is
  for.
- **Not yet exercised:** nothing was spawned in either window (Alice's attacks
  are melee), so spawn and despawn by `SimId` were never needed. A scene with
  shots or a death is the next measurement, and it needs a replicated spawn
  row per kind.

### NA-2b, the gated guest's update split (2026-11-11)

Probe `probe_where_a_gated_guest_spends_its_update` (same file, ignored):
the gated guest alone, warmed 30 updates, then 120 updates with each `Main`
schedule run in `MainScheduleOrder`'s order and timed, and 120 more through
`App::update`.

| Room | `App::update` | `Update` | `PreUpdate` | `PostUpdate` | `Last` | the rest |
|---|---|---|---|---|---|---|
| `switch_lab` | 2.11 ms | 1.93 ms (1,030 systems) | 0.07 | 0.09 | 0.05 | 0.07 |
| `central_hub_complex` | 2.24 ms | 1.92 ms (1,030 systems) | 0.06 | 0.09 | 0.09 | 0.07 |

- **Alone, the gated guest costs about 2.2 ms, not 4.5.** The first
  measurement timed the guest interleaved with a host App in the same
  process, and the gap is that interleaving and the snapshot's change
  detection, not the guest's own schedules.
- **Nearly all of it is `Update`'s 1,030 systems.** The sim schedule is
  `Update` here; gating the twelve simulation phases off still leaves the
  executor walking every system and evaluating its run conditions. The fixed
  loop is empty (one system).
- **Of the 1,030, 557 are in a gated phase and 473 still run** (by
  `ScheduleGraph::systems_in_set` over the twelve phases). By crate: 146 are
  `bevy_ecs`'s own (sync points and message updates), then the conversation
  (39), the game shell (31), the runtime (24), content (19), the Mary-O and
  Smash demos (18, 17: other games' systems in an Ambition guest), load
  presentation (16), the session (15), and a long tail. A gated phase still
  costs its systems a skip each; what runs is presentation and composition.
- ⇒ **A guest should be COMPOSED from fewer plugins, not gated.** The cost is
  the system count, which does not depend on the bodies; a guest built
  without the simulation plugins (presentation, the shell and the replication
  sink only) is the measurement that would say how much of the 2 ms is the
  gated simulation and how much is presentation.

**Without the gated systems (2026-11-11),** the same probe: after the gated
measurement, the twelve phases' systems are taken out of the sim schedule
(`Schedule::remove_systems_in_set`, keeping the phase sets and their order),
the nearest a booted App comes to having been composed without them. Then
30 warm updates and 120 timed through `App::update`. Three runs:

| Room | Gated | Without the 557 | Saved |
|---|---|---|---|
| `switch_lab` | 2.13 / 2.07 / 2.01 ms | 1.89 / 1.67 / 1.64 ms | 0.24–0.40 ms |
| `central_hub_complex` | 2.23 / 2.22 / 2.20 ms | 1.77 / 1.84 / 1.89 ms | 0.31–0.46 ms |

- **The gated simulation is under a fifth of the guest's update.** Taking
  its 557 systems out also takes out 138 of `bevy_ecs`'s automatic sync
  points (1,030 systems become 335, and no `bevy_ecs` system is left among
  them), and saves 0.24–0.46 ms of about 2.1.
- **The rest, about 1.7–1.9 ms, is 335 systems that are not simulation:**
  the conversation (39), the game shell (31), the runtime (24), content
  (19), the Mary-O and Smash demos (18 and 17: other games' systems in an
  Ambition guest), load presentation (16), the session (15) and a tail.
  About 5 µs a system whatever it does.
- ⇒ **The lever for a cheap guest is the composition, not the
  simulation.** Composing a guest without the simulation plugins buys at
  most the 0.4 ms above; a guest that installs only its own game (not the
  hub's other demos), and a conversation and shell that cost nothing while
  idle, is where the other 1.8 ms is.

### NA-2b, spawns and despawns by `SimId` (2026-11-11)

Probe `probe_what_a_guest_needs_to_draw_shots` (same file, ignored). The
NA-2b guest in `switch_lab`, gated as above. Alice fires a short shot (0.4 s)
every twelve frames from frame 10 to 200, so shots spawn and end all through
the run. Each snapshot (three frames late) carries the body rows and the ids
of the shots in flight. The guest writes the rows onto what it holds, builds
each shot it does not hold from a spawn row (id, kind, half extent, visual
index: 15 bytes) and removes each replicated shot the snapshot no longer
names (a 4-byte despawn row). One run, 240 frames:

| Shots spawned / despawned | Most in flight | Applying a snapshot | Frames whose ids differ after applying | Link a frame |
|---|---|---|---|---|
| 16 / 16 | 2 | 0.172 ms | 0 | 86.5 bytes (body rows 85.3, spawn 1.01, despawn 0.27) |

- **A spawn row per kind by `SimId` is enough to keep the guest's set equal
  to the host's:** after every applied snapshot the guest held exactly the
  ids the host sent, shots included.
- **Spawns cost little on the link:** about 1.3 bytes a frame here, against
  85 for the body rows that carry the shots' motion. The body rows are the
  cost; delta encoding (NA-1/NA-2) is where the bytes are.
- **Not yet exercised:** spawn rows for other kinds (items, debris), and
  sending only the guest's room.

**A death (2026-11-11),** probe `probe_what_a_death_on_the_host_does_to_a_guest`
(same file, ignored): an enemy (`npc_puppy_slug`, passive) is spawned beside
Alice on the host at frame 5 with one point of health, and her shots kill it.
The guest applies each late snapshot by the general rule: build every body
the snapshot names and it does not hold (from a spawn row), remove every
body it holds and the snapshot no longer names, whoever built it.

- The enemy entered the host's snapshot at frame 5 and the guest built it
  from a spawn row; the guest also built and removed the 16 shots, removed
  none of the bodies it booted with, and its ids equalled the host's after
  every snapshot.
- **A death is a row change, not a despawn.** The enemy died at frame 18
  (health 0) and its body stayed in the host's world, with its id and its
  motion, to the end of the run (240 frames). The body row (position,
  velocity, facing) says nothing of it, so the guest drew a living enemy.
  The body row needs the defeat (health, or a defeated flag and pose) before
  a guest can show a death; a despawn row is for what leaves the world.

**The defeat in the body row (2026-11-11),** the same probe: each snapshot
also carries every body's current health by `SimId` (an `i16`, two bytes),
and the guest writes it onto the body it holds (giving one it built from a
spawn row a health of its own). The shots fly at a crawler's height now: a
body's collision is fitted to its art, and the puppy slug is 14 units tall,
under the old line of fire, so the earlier run's kill no longer happened.

| Host's enemy dead | Guest's enemy defeated | Frames a held body's life differs after applying | The field a frame |
|---|---|---|---|
| frame 14 | frame 17 (the snapshot's 3 frames) | 0 | 5.97 bytes (three bodies) |

- **Health in the row is enough for the guest to hold the defeat:** its
  enemy is dead exactly one snapshot after the host's, and no body's life
  ever disagreed with the snapshot it had applied.
- **Holding it is not showing it.** What draws a death (the death beat, its
  pose, the corpse rule) is set by simulation systems the guest has gated
  off, so a guest that should draw the death needs the beat's state in the
  row too (its phase and time), or a presentation that reads health.

**The death beat in the body row (2026-11-11),** the same probe: each
snapshot also carries every body's `DeathBeat` by `SimId` (on the link, the
frames since its death as a `u16` and a byte for its `lived` and corpse
bits, 3 bytes; its row length is the body's sheet, a fact of its spawn row),
and the guest writes it onto the body it holds. The probe compares the
`visible` the view reads (`view_index`: a dead body is drawn only as its
beat says) on the guest, before and after applying, with the host's in the
snapshot.

| Host stops drawing the enemy | Guest stops | Frames `visible` differs before applying | After applying | The field a frame |
|---|---|---|---|---|
| never (Ambition's corpse rule is `Stay`) | never | 1 | 0 | 5.95 bytes (three bodies) |

- **Without the beat a guest never draws a dead body.** The one frame that
  differed before applying is the snapshot that brought the death: the
  guest's own beat had no `since_s`, and `visible(false)` is false without
  one. Nothing on the guest would ever set it: `advance_death_beats` runs in
  the gated simulation. With the beat applied the guest draws the corpse
  exactly as the host does.
- **In a `Stay` game the beat only says "dead since"; in a `Vanish` game it
  times the blink.** This probe's room states `Stay`, so the blink is not
  witnessed here; the frames since death are what time it, and they are in
  the row.
- **The kill's shake and hold do not travel with it.** `advance_death_beats`
  starts the kill's camera shake and its room's hold on the tick a beat
  first gets `since_s`. A guest that only applies the field draws the
  corpse but not that beat; a presentation event (the kill, by `SimId`)
  would carry it.

**The kill as a presentation event (2026-11-11),** probe
`probe_what_a_kill_on_the_host_shows_a_guest` (same file, ignored): the
death probe's run with two gated guests. Both apply the body rows, healths
and beats; one is also sent each frame's `CameraShakeRequest`s, read off the
host's message channel (a shake is one of the presentation effects a
simulation makes and presentation reads, `for_each_presentation_effect`), and
the other, the control, is not.

| Host's shakes (rooms) | Sounds, visuals the host made | First shake after the kill: host, guest, control | Frames a view's shake differs from the host's 3 frames earlier: guest, control | Link a frame |
|---|---|---|---|---|
| 3 (`None`, room 0) | 5, 6 | frame 15, 17, never | 1, 223 | 0.11 bytes |

- **Forwarding the host's presentation effects shows the kill:** the guest
  shakes with the host, and without them it never does. A room travels as
  its `LiveRoomInstance` number; host and guest booted the same room, so it
  named the same room on both. A guest whose room numbers differ from the
  host's needs them mapped, as bodies are by `SimId`.
- **The one differing frame is the host's own lag:** the host writes the
  kill's shake on frame 14 and applies it on its next update (the applier
  runs before the simulation that writes it), while the guest writes the
  forwarded shake before its update. So the guest shakes two frames after
  the host, not three.
- **The channel is cheap:** three shakes, five sounds and six visuals in
  240 frames. The kill needs no field of its own; the same channel would
  carry sounds and visuals, which a gated guest makes none of either.
- **Not measured:** a headless App has no shake decay (`tick_camera_shake`
  is the windowed host's), so a shake here never ends and "shaking" counts
  frames since the first one. Sounds and visuals were counted, not sent.

**What else a host spawns (2026-11-11),** probe
`probe_what_else_a_host_spawns` (same file, ignored): the death probe's run
(the enemy held at one point of health, so Alice's first hit kills it), and
a census of every entity the simulation keeps that appears after boot.

| Appeared | How many | In the body rows | Identity |
|---|---|---|---|
| the spawned enemy (`placement:…`) | 1 | yes | `SimId` |
| Alice's shots (`slot:0/#`) | 16, each lived 4–23 frames | yes | `SimId` |
| the enemy's bounty coin (`PickupFeature`, `PickupMagnet`) | 1 | no (no `BodyKinematics`) | none: `FeatureId` and `SpawnOrigin::Dynamic { parent, sequence }` |

- **A death drop has no `SimId`, so a guest that replicates by `SimId`
  never sees it.** This is deliberate (`ambition_platformer2d_items::loot`,
  `DropIdentity::Anonymous`): a coin, a heart and an ability pickup are not
  occurrences; only a dropped weapon mints `SimId::death_drop`. A guest needs
  them anyway (the coin is drawn and collected), so a pickup's spawn row has
  to be keyed by its origin (its parent's `SimId` and its sequence), which
  is peer-stable, and its despawn by the same key.
- **Nothing else appeared:** this room has no breakables, debris or
  summons, so their kinds are not measured here.

**A guest of its own game only (2026-11-11):** the hub installs the games
`AMBITION_HUB_GAMES` names (`game/ambition_app/src/app/hub_games.rs`; unset
is all, and an empty value is none), so `probe_where_a_gated_guest_spends_its_update`
run with `AMBITION_HUB_GAMES=ambition` measures a guest with no other game's
plugins (host and guest alike: the selection is the process's). Two runs of
each, alternating order (ms an update, gated guest / without the gated
systems):

| Composition | `Update` systems (gated) | `switch_lab` | `central_hub_complex` |
|---|---|---|---|
| every game, run 1 | 1,031 (558) | 2.46 / 1.66 | 2.91 / 1.60 |
| every game, run 2 | 1,031 (558) | 1.90 / 1.61 | 2.12 / 1.75 |
| Ambition only, run 1 | 895 (483) | 1.78 / 1.48 | 2.16 / 1.63 |
| Ambition only, run 2 | 895 (483) | 1.85 / 1.75 | 1.84 / 1.78 |

- **The other games cost a guest 136 systems and no measurable time:** the
  first every-game run is the outlier; the second sits inside the
  Ambition-only runs' range. Mary-O's 18 and Smash's 17 still-running sim
  systems, and the rest, are run conditions that answer no.
- **What is left is Ambition's own shell and presentation:** with the
  gated systems removed, 281 sim-schedule systems still run (the
  conversation 39, the game shell 31, the runtime 24, content 19, load
  presentation 16) and about 140 Bevy sync points, for about 1.5–1.8 ms an
  update, the same as with every game. A cheaper guest is not a shorter game
  list; it is the gated guest without the conversation, shell and load
  presentation it does not run, or those run on the host's word.

**A pickup by its origin (2026-11-12),** probe
`probe_what_a_guest_needs_to_draw_a_drop` (same file, ignored): the death
probe's run for 360 frames, so the enemy's bounty coin falls, drifts to
Alice on its magnet and is collected. Each snapshot also carries the host's
uncollected dropped pickups keyed by `SpawnOrigin::Dynamic` (the parent's
`SimId` and the drop's sequence: 5 bytes on the link), since a drop has no
`SimId`. A spawn row adds its kind, amount and centre (16 bytes); a motion
row (key and centre, 13 bytes) is sent only on a frame the drop moved; a
despawn row is the key. The guest builds each drop it does not hold with
the components the dropped-pickup view reads (`rebuild_dynamic_feature_views`,
run on its world because the guest gates the phase that runs it), moves the
ones it holds, and removes what the snapshot no longer names.

| Host: dropped, collected | Guest: built, removed | Frames its keys differ after applying | Frames a view would draw a drop on the guest | Frames its drawn drops differ from the host's | Link a frame |
|---|---|---|---|---|---|
| frame 14, 29 | frame 17, 32 | 0 | 15 | 0 | 0.38 bytes (spawn 0.04, motion 0.33, despawn 0.01) |

- **The origin is a key that works:** the guest built the coin one snapshot
  after the host dropped it, drew it where the host did on every frame, and
  removed it one snapshot after the host's player collected it. No peer-
  stable id was minted for it; the parent's `SimId` and the sequence were
  enough.
- **A collected pickup is a row change on the host, a despawn on the
  guest.** The host keeps a collected pickup with `Collected`; the snapshot
  names only uncollected ones, so the guest removes it. What the collection
  gives (the wallet, the sound) travels on the collector's rows and the
  presentation channel, not on the pickup's.
- **The magnet is the cost:** most of the 0.38 bytes is the drop's motion
  while it flies to its collector. A drop at rest costs nothing after its
  spawn row.
- **Not exercised:** authored pickups (both Apps boot with them, so taking
  one is a row change keyed by `SpawnOrigin::Authored`), hearts and ability
  drops (the same road, other kinds), and a drop in a room the guest is not
  in.

**A breaking wall (2026-11-12),** probe `probe_what_a_breaking_wall_spawns`
(same file, ignored; the census above is now one function run in either
room): `breakable_wall_lab`, Alice put before the cracked wall and swinging
every 25 frames, 300 frames.

| Appeared or changed | How many | Identity | What a guest needs |
|---|---|---|---|
| the wall broke (frame 78) | 1 | its own `SimId`, booted on both | a row change: `broken`, keyed by the `SimId` both Apps already hold |
| the wall's drop (`PickupFeature`, `PickupMagnet`) | 1 | `SpawnOrigin::Dynamic`, no `SimId` | the pickup row above, keyed by its origin |
| Alice's three swings' strike volumes (`slot:#/strike/…`) | 3, 5–7 frames each | `SimId`, no `BodyKinematics` | nothing to draw: a strike is drawn from the swinging body's move, which its rows would carry; only an inspector reading the host's combat geometry needs the volume |

- **A breakable is a row change, as a death is:** the wall is booted on
  both sides under the same `SimId`, and breaking it removes nothing and
  spawns no debris entity here; what it drops goes the dropped-pickup road.
- **Strikes are the one new `SimId` kind that is not a body:** minted per
  swing and gone in a few frames. A guest drawing the swinging body draws
  the strike; replicating the volume would only serve a combat inspector.
- **Still not measured:** a summon (no summoner in either room), and a
  breakable whose break spawns debris.

**A summon (2026-11-12),** probe `probe_what_a_summon_spawns` (same file,
ignored): in `switch_lab` an enemy is spawned beside Alice on frame 5 and
summons a flying shark on frame 10 through the summon road
(`SummonRequest`), 120 frames.

- **A summon is a body like any other:** its id is the summoner's with a
  sequence (`placement:…/#`), it has `BodyKinematics`, so the body rows
  already carry it and the general rule (build every body the snapshot names
  and the guest does not hold) applies. Its spawn row has to name what it is
  built from: its character (82 components on the host, nearly all from the
  prepared character), faction and calling.
- **Its strike is one more short-lived strike volume** (`…/#/strike/…`), as
  Alice's swings were: drawn from the body's move.
- **One anonymous pickup appeared,** keyed by its `SpawnOrigin` as above;
  this probe did not record which body it fell from.
- **That closes the census of what a host spawns** in these rooms: bodies
  (players, enemies, summons, shots) by `SimId`, drops by `SpawnOrigin`,
  breakables as row changes on booted `SimId`s, strike volumes not
  replicated.

**The guest's room only (2026-11-12),** probe
`probe_what_a_guest_in_one_room_is_sent` (same file, ignored): NA-2's
two-room scene (Alice leaves through the hub door on frame 30; Bob stays in
`switch_lab`), 220 frames. The guest is Bob's App; each snapshot carries
only the bodies in the room Bob's body is in on the host.

| Bodies outside Bob's room from | Body rows a frame: unscoped, Bob's room only | Guest built, removed | Frames its ids differ after applying |
|---|---|---|---|
| frame 32 | 398.5 bytes, 27.4 bytes | 0, 1 (Alice's body, `slot:0`) | 0 |

- **Scoping is most of the saving once the players part:** the hub's
  bodies are 93% of the unscoped rows, and Bob's App never needed them.
- **A body that leaves the guest's room is removed by the same rule** as a
  body that leaves the world: Alice's body, booted on the guest, went one
  snapshot after she crossed, with no id disagreement.
- **The room numbers agreed only because Bob's room was booted first on
  both** (`LiveRoomInstance(0)`). A guest that crosses into a room the host
  already holds under another number needs the number mapped, as the
  forwarded shakes do; that crossing is not exercised here.

**Sounds and visuals over the presentation channel (2026-11-12),** probe
`probe_what_a_guest_needs_to_hear_and_see_the_host` (same file, ignored):
the death probe's run with Alice also swinging every ten frames, so the
host makes shots, swings, hits and a kill. Each frame's `OwnedSfxMessage`s
and `VfxInRoom`s ride the snapshot and are written on the guest. A visual
that names a host body (a swing's owner, an aura's, a bolt's body end) is
sent with that body's `SimId` and written with the guest's body for it.

| Sounds | Visuals | Visuals naming a body | Mapped by `SimId` on the guest | The guest's own |
|---|---|---|---|---|
| 15 (`Play` 14, `Land` 1) | 16 (`Slash` 13, `Dust` 2, `Impact` 1) | 13 (every `Slash`) | 13 of 13 | none |

- **Only a body reference needs translating:** every other sound and visual
  carries positions, a listening space or a room. The swings name their
  swinging body, and each one maps to a body the guest holds when the
  snapshot arrives, since the body rows that carry it arrive with it.
- **A gated guest makes no presentation of its own**, so what its channels
  carry is exactly what was forwarded (31 messages here).
- **Rooms and listening spaces are numbers** (`LiveRoomInstance(0)`,
  `SfxSpace(0)`), equal here because both Apps booted the same room first;
  they need the same mapping as a guest's room number.
- **Not measured:** what a renderer and a mixer on the guest make of them
  (the headless harness has neither); the bytes (the messages are not yet
  encoded).

**A guest whose player crosses (2026-11-12),** probe
`probe_what_a_guest_does_when_its_player_crosses` (same file, ignored):
Alice and Bob stand on `switch_lab`'s door to the hub; Alice goes through on
frame 30 and Bob on frame 90, into the hub the host already holds. The guest
is Bob's App and receives Bob's room only.

| Frame | Host: live rooms, Bob's room | Guest: live rooms, Bob's room |
|---|---|---|
| 1 | 0, 0 | 0, 0 |
| 32 | 0 and 1 (the hub), 0 | unchanged |
| 92 | 1 (`switch_lab` retired), 1 | unchanged: 0, 0 |

- **A gated guest does not follow a crossing.** Its room set never changed:
  room changes run in the simulation it gates. After Bob crossed, the guest
  still held `switch_lab` (its collision, its art) with Bob's body stamped
  in it, and built the hub's 18 bodies from the snapshot with no room at all.
  Its ids agreed with the snapshot on every frame; its world did not.
- **The host's numbers are not the guest's:** the hub is the host's room 1,
  a number the guest never minted, and `switch_lab`'s 0 is retired on the
  host while the guest still holds it.
- **So the guest needs a room lifecycle of its own on the host's word:** a
  room row (the definition id and the host's number for the room its player
  is in) that the guest builds through its own room construction (ungated,
  as a crossing builds a room), retiring what it no longer needs, with the
  host's number mapped to its own for bodies, visuals and listening spaces.

**A guest that follows on the host's word (2026-11-12),** probe
`probe_whether_a_guest_follows_its_player_on_the_hosts_word` (same file,
ignored): the crossing probe's run, 300 frames, with each snapshot also
carrying a room row (the host's number for Bob's room and the definition it
builds). When the row names a definition Bob is not in on the guest, the
guest records a crossing of Bob's body into it at the position the body row
gives, as a door would; once its Bob stands in a room of that definition,
the host's number maps to the guest's. The room transition phase sits
inside `CoreSimulation`, so this guest gates the core's other phases one by
one and leaves it awake (`simulate_nothing_but_room_changes`). Run twice:
applying every snapshot, and deferring the bodies of a room the guest has
not mapped yet.

| | Guest recorded | Hub live on the guest (host: frame 92) | Host to guest numbers | Built bare | Held twice at the end |
|---|---|---|---|---|---|
| every snapshot applied | frame 95 | frame 97 | 0 to 0, 1 to 1 | 18 (the hub's bodies, before its room existed) | 0 |
| unmapped rooms deferred | frame 95 | frame 97 | 0 to 0, 1 to 1 | 1 (Alice's body: session-owned, no room builds it) | 0 |

- **The guest's own room lifecycle can follow the host:** a crossing
  recorded on the host's word was built by the guest's own room
  construction two frames later, five after the host, with `switch_lab`
  retired as on the host. With the core fully gated it was admitted but
  never committed; the room transition phase has to stay awake.
- **The room's own bodies come from the guest's construction, not the
  wire:** deferring the snapshot's bodies until the room is mapped leaves
  only what no room builds (another player's body) to a spawn row.
- **The numbers happened to agree again** (both sides minted 1 for the
  hub); the map is what would carry a disagreement.

### NA-3, guest prediction: first measurement (2026-11-12)

Probe `probe_how_far_a_guests_prediction_is_from_the_host` (in
`a_guest_app_applies_the_host.rs`, ignored). Bob's App simulates with
nothing gated and drives his seat with his own input at once; the host
applies it [`LATENCY`] (3) frames late, as NA-2. Each frame the guest
writes every other body from the newest snapshot it holds, so only Bob's
body is its own. A host snapshot of frame `h` reflects Bob's inputs up to
`h - 3` and arrives on guest frame `h + 3`; its correction is the distance
between Bob in it and Bob on the guest after the same inputs. "Reconciled"
moves the guest's Bob, and its stored prediction since that frame, by each
correction as it arrives. `switch_lab`, 240 frames, 234 snapshots compared;
Bob turns at his own rhythm and jumps.

| Alice | Not reconciled: mean, largest, frames over 0.5 | Reconciled: mean, largest, frames over 0.5 |
|---|---|---|
| still | 0, 0, 0 | 0, 0, 0 |
| holds right | 0, 0, 0 | 0, 0, 0 |
| fights (swings every ten frames) | 1.45, 5.85, 131 | 0.37, 1.48, 116 |

- **Alone, the prediction is exact:** the guest's own kernel against its own
  room's collision puts Bob where the host does, to the bit, with Alice still
  or walking away. Movement needs nothing more than the guest's room and
  its own input.
- **What other bodies do to Bob is what the guest gets wrong:** with Alice
  swinging beside him, her swings hit him on the host (two connects, each
  setting an impact freeze; his health falls to 0, so this harness's seats
  hurt each other). The guest's Alice is written from snapshots and never
  swings, so the strikes, their knockback and their freezes land only on the
  host. Offset reconciliation keeps the error under 1.5 units but
  corrects on half the snapshots; a correction that replays the inputs
  since the snapshot (the guest rewinding its own body) is the next thing to
  measure, and which contacts reach a body (strikes, pushes, carries) is
  the list a guest's prediction has to include or accept as corrections.
- **A first attempt at offset reconciliation diverged** (corrections of
  10^9): moving the body without moving the stored prediction asked for the
  same correction on every later snapshot. The stored history has to move
  with the body.

**Reconciling by replay (2026-11-12),** probe
`probe_how_far_a_guest_that_replays_is_from_the_host` (same file,
ignored): the same scenes, but a correction rewinds the guest's own room to
the frame the snapshot speaks of (the room saved every frame through
`RoomSnapshotRows` with the clock, ISL-2's room rewind, kept twelve frames),
writes Bob as the host had him there and every other body from the snapshot,
and replays Bob's inputs since (five frames at this latency).

| Alice | Reconciliation | Correction mean, largest | Snapshots over 0.5 | Frames replayed | Replay cost a frame (debug build) |
|---|---|---|---|---|---|
| still | replay | 0, 0 | 0 | 0 | 0 |
| fights | none | 1.45, 5.85 | 131 | — | — |
| fights | offset | 0.37, 1.48 | 116 | — | — |
| fights | replay | 0.068, 1.45 | 14 | 185 | 9.2 ms |

- **Replaying the guest's own room is the reconciliation that works:** the
  mean correction falls twentyfold from none and fivefold from offset, and
  only fourteen snapshots still ask for more than half a unit (each right
  after a strike the guest could not foresee).
- **The room rewind ISL-2 built is what a guest needs:** the same
  `RoomSnapshotRows` save and restore, on the guest's one room, with its own
  inputs as the replay's. Hosted authority between rooms and rollback inside
  the guest's room are one mechanism here.
- **Its cost is the replays, not the saves:** 37 corrections of five frames
  each, about 9 ms a frame averaged over the run in a debug build. A guest
  that replays only its own body (not the room's other systems) or
  corrects only past a threshold would pay less; not measured.

### NA-3, a late join (2026-11-12)

Probe `probe_what_a_late_guest_needs` (same file, ignored). The host plays
`breakable_wall_lab` and Alice breaks its cracked wall. On frame 200 two
guests boot in the lab: one adopts the host's save as a load from a file
does (the save's data, `SaveRestored` cleared, 90 frames with its
simulation awake), the control does not. Both are then gated and take the
host's body rows for 60 frames.

| Guest | Its wall at the join, at the end | Frames its ids differ after applying | Built, removed |
|---|---|---|---|
| without the host's save | whole, whole | 0 | none, none |
| adopting the host's save | broken, broken | 0 | none, none |

- **The host's save is what a late guest needs besides the rows:** a
  breakable's state is an occurrence in the save (`Spent`), so a guest that
  loads the host's save builds the room as the host has it, and the body
  rows agree with it at once. Without it the wall stands on the guest for
  good: no body row names it.
- **The same road serves a returning guest:** the host keeps NPC and room
  facts in its save and its ledgers (the persistence authority every room
  rebuild already reads), so a guest that leaves and comes back needs that
  save again, not a history of what changed while it was away. Not measured
  here: an NPC's life (`LifeLedger`) across a guest's absence.

**An NPC's life for a guest that arrives later (2026-11-12),** probe
`probe_what_a_guest_knows_of_a_rooms_lives` (same file, ignored): the host
plays `eureka_coil_lab` from five past ten until a scientist holds the
workbench (292 steps) and 30 more. Two guests then boot in the lab with the
host's day clock and settle 90 frames with their simulation awake; one
adopts the host's save (which keeps both lives, `LifeLedger`), the control
does not.

| Life | Host | Guest without the save | Guest with the save |
|---|---|---|---|
| `NpcSpawn-410911` | holding, claims the workbench, plan #2 | going to, no claim, plan #2 | holding, claims the workbench, plan #1 |
| `NpcSpawn-410910` | going to, no claim, plan #2 | going to, no claim, plan #2 | holding, no claim, plan #1 |

- **The save carries what a life holds:** the guest that adopted it has the
  same scientist on the bench as the host; the control's lives replanned
  from the authored start and nobody holds it.
- **It does not carry where a life is in its plan:** with the save, the
  other scientist resumed as holding at plan #1 while the host's had moved
  on to plan #2. A life's step is the host's alone; a guest that simulates
  lives from the save drifts from it, and a gated guest has none. So a
  guest that shows lives needs each life's step on the wire (as the death
  beat is), or accepts drift until the next save.

## Library evaluation (2026-11-13): Lightyear and `bevy_replicon`

Read from crates.io and docs.rs, no dependency declared. This tree is on Bevy
0.19.1. The newest releases (`bevy_replicon` 0.45.5, `lightyear` 0.31.0) need
Bevy 0.20; the last for 0.19 are `bevy_replicon` 0.44.x (0.41–0.44 support
0.19) and `lightyear` 0.30.x.

- **Lightyear builds its world replication on `bevy_replicon`.** What it adds
  is transport (UDP, and WebSocket, WebTransport and Steam through `aeronet`),
  per-tick client input buffering, client prediction with per-component
  rollback, snapshot interpolation and deterministic (input-only)
  replication.
- **`bevy_replicon`** is server-authoritative replication with no I/O of its
  own (backends: `bevy_replicon_renet`, `_renet2`, `_quinnet`,
  `aeronet_replicon`, `bevy_replicon_matchbox`). A server entity is replicated
  when it carries `Replicated`, with rules per component; the client maps
  server entities to its own (`ServerEntityMap`) and marks them `Remote`.
  `Signature` matches a replicated entity with one the client already made
  or loaded itself (a level both sides load). Client visibility uses filter
  components on both the client and the entity, like collision layers.
  Replication is sent when its `ServerTick` advances (by default in
  `FixedPostUpdate`), with per-entity and per-client priority.

How each measured need maps:

| Need (measured above) | `bevy_replicon` | Lightyear on top |
|---|---|---|
| Identity by `SimId`; the room's own bodies built by both Apps | `Signature` from the `SimId` | same |
| Spawn rows (shots, summons, drops by `SpawnOrigin`) | `Replicated` plus a rule; the client builds the body on `Remote` | same |
| The guest's room only (27 against 398 bytes a frame) | visibility filters keyed by room | rooms, through replicon |
| Prediction of the guest's own body, reconciled by replaying its room (0.068 units mean) | none; the room rewind (ISL-2) stays ours | per-component prediction and rollback: a third rewind beside GGRS and the room rewind |
| Input to the host | messages and events | tick-buffered input, `leafwing` integration |
| Transport with no service dependency | renet or quinnet (direct UDP or QUIC); matchbox needs a signalling server | its own UDP and `aeronet` |

**Decision (2026-11-13):** prototype on `bevy_replicon` 0.44 (Bevy 0.19)
alone. The identity, spawn and room needs fit its core, and its lack of I/O
leaves the transport open. Lightyear's value is its prediction, input and
transport. Its prediction would be a third rewind beside GGRS (Smash) and the
room rewind, which ADR 0036 names as Ambition's prediction (a guest
reconciles by rewinding its own room). So it is not taken now. Revisit its input buffering
and transport crates when the guest has a real link. Moving to Bevy 0.20 moves
both crates, so the prototype pins the 0.19 line.

## The prototype's first slice (2026-11-13): body rows through `bevy_replicon`

Crate `ambition_hosted_world` (`HostedWorldPlugin { role }`). The host marks
each body (`SimId` plus `BodyKinematics`) `Replicated` with
`Signature::of::<SimId>()` and writes its `BodyRow` (position, velocity,
facing) before the send. The guest signs the bodies it built itself, so
replication continues into them, and puts each changed row on its body. The
link in the witness is replicon's in-process `ServerTestAppExt`. Note that the
harness never calls `App::finish`, and replicon makes its protocol and
channels there, so a composition that adds it must finish the App.

Witness `a_hosted_guest_shows_the_hosts_bodies` (`switch_lab`, Bob beside
Alice, a gated guest, 90 frames of Alice moving and swinging): the guest
holds both bodies exactly as the host has them, with no body of its own
made. A guest never linked differs on both. The link carries 53.2 bytes a
frame for the two bodies, replicon's framing included.

### The life row (2026-11-13)

`LifeRow`: the body's health and its death's beat (seconds since it died,
whether it lived, what it leaves), written beside the body row and put on
the guest's `BodyHealth` and `DeathBeat`. The beat's `finale_due` is not
sent: what a death becomes (a blast, two halves) is the host's consequence,
and its results arrive as rows of their own.

Witness `a_hosted_guest_shows_a_death_on_the_host` (the same room; the host
sets Bob's health to 0 on frame 20): on every one of 90 frames the guest's
Bob holds the host's life row and is drawn or not as the host draws him.
The beat had run 1.08 s on the host by the end, and the guest runs none, so
its agreement came over the link. On the unlinked guest Bob never falls.

### The room as a visibility filter (2026-11-13)

`RoomOf(ordinal)`, an immutable component the host puts on each replicated
body (from its `InRoomInstance`) and on each guest's client entity (from the
room of the body its `GuestSeat` drives), is a replicon `VisibilityFilter`
over the whole entity: a body is sent to a client in its room, or to a client
whose room is not known yet. A body leaving a guest's view is removed there.

Witness `a_hosted_guest_is_sent_its_own_room` (the crossing scene: Alice
leaves `switch_lab` for the hub on frame 30, Bob stays; 220 frames): with
Bob's seat named, the guest ends holding exactly the host's bodies in Bob's
room, with their rows, and one replicated entity, at 30.6 link bytes a
frame. With no seat named (the control) it is sent 19 entities, the hub's
included, at 66.4 bytes a frame. The 17 hub entities it was sent there are
rows with no body to land on: the guest has not built the hub, which is the
spawn row's job.

### The spawn row (2026-11-13)

`SpawnRow` (the body's `SimId`, feature id, name, character, half size, and
whether it is an NPC), written once for each replicated body that no seat
drives. The guest builds each replicated entity that has one and no body
through the engine's staged-actor road (`spawn_staged_actor_into`, with a
passive brain the gated guest never runs anyway) into the replicated entity
itself, and gives it the host's `SimId`; its body and life rows then land on
it like on a body it built. The guest does not sign replicated entities.

Witness `a_hosted_guest_builds_what_the_host_spawned` (`switch_lab`; the host
spawns a puppy slug beside Alice on frame 5): the guest ends with the enemy
as a whole actor (health and death beat), with the host's `SimId`
(`placement:na_spawn_row`) and character, and holds all three bodies with
the host's rows. The unlinked guest never has it. In the crossing control
(no seat named) the guest now also builds two of the hub's bodies; the rest
of the 19 replicated entities there wear no character, and no spawn row is
written for them.

### A real link (2026-11-13): renet's netcode over UDP

`bevy_replicon_renet` 0.20 (the release for replicon 0.44 and Bevy 0.19),
behind `ambition_hosted_world`'s `renet` feature: `udp::host` opens the
host's socket (unauthenticated netcode, one protocol id) and `udp::join`
joins it; the Apps add `RepliconRenetPlugins` beside `RepliconPlugins`.
Replicon's channels become renet's, so the rows above need nothing new.

Witness `a_hosted_guest_shows_the_hosts_bodies_over_udp` (the first slice's
run, on loopback): the guest joins after 3 updates, and after each of the 90
frames (host update, then guest update, the packets crossing the kernel in
between) it holds the host's bodies; Alice moved, so the agreement is the
link's. Renet reports 3,982 bytes a second sent to the guest, about 66 bytes
a frame at 60 updates a second, against 53 counted inside replicon on the
test link: the difference is renet's and netcode's packet framing.

### The guest's input (2026-11-13)

`GuestInput(ControlFrame)`, a replicon client message on the reliable
ordered channel: each frame the guest sends the raw frame of its
`LocalSeat`, and the host drives the seat the sender's `GuestSeat` names
through `drive_slot_frame`, the seam a device or a test driver uses (on a
device host it folds into the seat's latch, so two frames in one update keep
both presses; on the harness the later frame wins).

Witness `a_hosted_guest_drives_its_seat_on_the_host`: Bob is driven only on
the guest App, and the host's bodies equal those of a reference App that
drives Bob locally with the same frames on all 90 frames. With no seat named
(the control), the host differs on 78, from the frame Bob first turns.

This is the production input path SIMULATION-ISLANDS' room rollback host
waits on: a guest's input now reaches the host's seat over the link.

What wears no character (probed 2026-11-13, the crossing scene at frame
120): the hub's 17 replicated entities with no `WornCharacter` are all its
ground items (`placement:ground_*`, `placement:hub_portal_gun`), authored
placements. A guest that builds the room itself (the room row, 2026-11-12)
holds its own copies under the same `SimId`, and `Signature` continues
replication into them, so they need no spawn row; a dropped item takes the
`SpawnOrigin` road (2026-11-12). The spawn row still missing is a player's:
a player's body that leaves a guest's room is removed there by the filter,
and when it comes back replication makes an entity the guest cannot build
from a character spawn row (a player body is built by its seat's identity,
not the staged-actor road).

### The room row (2026-11-13): a guest follows its player's crossing

Each replicated body carries its room's definition (`RoomNamed`). When the
guest's own body (its `LocalSeat`) is named into a room it does not stand in,
the guest records that crossing in its own room lifecycle at the body's row,
and its room construction builds the room (the 2026-11-12 road). Once every
identified body it built there is signed, it tells the host which room it
holds (`GuestRoom`, a client message), and only then does the host move the
guest's view (`GuestView { room, seat }`): the room's bodies arrive after
the guest built its own and continue into them. The filter never hides a
guest's own player (`RoomOf { room, seat }`): without that, the host hid the
guest's Bob the frame he crossed, while its view waited, and replicon
removed him from the guest. The guest also signs its bodies just before each
receive, for a body its construction made late in a frame.

Witness `a_hosted_guest_follows_its_players_crossing` (both on the door to
the hub; Alice crosses on the host at frame 30; Bob, driven only from the
guest, at frame 90; 300 frames): the guest's Bob ends in the hub, the guest
holds all 17 of the hub's bodies that no seat drives (two NPCs walking, 15
ground items) with the host's rows, none twice. The control guest names no
seat and stays in `switch_lab`. Alice is not held: she left the guest's view
at frame 30 and her body coming back needs a player's spawn row.

### A guest's leave (2026-11-13)

When a seated guest's client entity goes (it disconnected), the host writes
that seat's `ParticipantLeaveRequested` as a stamped host intent, the leave a
local seat's "Leave Game" writes: its membership and bodies end and what it
held is put down. Witness `a_hosted_guest_that_disconnects_leaves_the_session`
(LOCAL-PLAY-MODES' remote leave).

Next in the prototype: a player's spawn row, a guest App composed from fewer
plugins, and each NPC life's step as a row.

## Next

1. **NA-2b — a guest App.** First measurement above: body rows only, no
   spawns; the gated update is split (2026-11-11): 2.2 ms alone, nearly all
   `Update`'s 1,030 gated systems. Shots spawned and ended by `SimId`
   (2026-11-11): the guest's ids equal the host's after every snapshot, at
   about 1.3 bytes a frame of spawn and despawn rows. A death is a row
   change: the killed enemy's body stays in the host's world, so the body
   row must carry the defeat. The defeat in the row (2026-11-11): health by
   `SimId`, 2 bytes a body; the guest's enemy is defeated one snapshot after
   the host's, with no disagreement. The death beat in the row (2026-11-11):
   3 bytes a body; the guest draws the corpse as the host does (without it,
   never), but not the kill's shake and hold. Without the gated systems
   (2026-11-11): taking the simulation's 557 systems out of a booted guest
   saves 0.24–0.46 ms of about 2.1; the other 1.7–1.9 ms is 335 systems of
   composition and presentation (other games' demos, the conversation, the
   shell). The kill as a presentation event (2026-11-11): forwarding the
   host's camera shakes makes the guest shake with it (the control never
   does), at 0.11 bytes a frame; sounds and visuals would ride the same
   channel. A guest of Ambition alone (`AMBITION_HUB_GAMES=ambition`,
   2026-11-11): 136 fewer systems and no time measurably saved; what costs
   is Ambition's own conversation, shell and load presentation. What else a
   host spawns (2026-11-11): a kill's bounty coin has no `SimId` (by design:
   drops are anonymous), so a guest replicating by `SimId` never sees it.
   A pickup by its origin (2026-11-12): keyed by `SpawnOrigin::Dynamic`
   (the parent's `SimId` and the sequence), the guest builds, draws and
   removes the bounty coin one snapshot after the host, with no difference
   in what a view draws, at 0.38 bytes a frame (most of it the magnet's
   motion). A breaking wall (2026-11-12): a row change on a `SimId` both
   Apps already hold; its drop takes the pickup road; a swing's strike
   volumes are the only new non-body `SimId`s and need no row to be drawn.
   A summon (2026-11-12): a body with a `SimId` descended from its
   summoner, carried by the body rows; its spawn row names its character.
   The guest's room only (2026-11-12): once Alice leaves, Bob's room is
   27 bytes a frame of body rows against 398 unscoped, and Alice's body
   leaves the guest by the general rule. Sounds and visuals (2026-11-12): forwarded with
   the snapshot; only a visual that names a body (a swing's owner) needs
   translating, by `SimId`, and all 13 did. A guest whose player crosses
   (2026-11-12): it does not follow (room changes run in the simulation it
   gates); it keeps the old room and builds the new room's bodies with no
   room. Following on the host's word (2026-11-12): a room
   row (the host's number and the definition) makes the guest record its
   player's crossing, and its own room construction builds the room two
   frames later if the room transition phase stays awake; deferring an
   unmapped room's bodies leaves only another player's body to a spawn row.
   NA-2b's questions are answered; what remains is NA-3.
2. **NA-3 — guest prediction.** Run the guest's own body through the
   movement kernel against its room's collision, reconcile to each snapshot,
   and measure correction size under the same scripts. First measurement
   (2026-11-12, above): exact while no other body acts on the guest's; with
   Alice swinging beside Bob, mean 1.45 units unreconciled and 0.37 with
   offset reconciliation. The corrections are Alice's strikes
   on Bob. Reconciling by replaying the guest's own room since the snapshot
   (ISL-2's room rewind) brings the mean to 0.068 at about 9 ms a frame of
   replays (debug build). What else a prediction needs is what reaches the
   predicted body from others (here, strikes); the replay absorbs it.
3. **NA-3 — NPC facts and late join.** Host-owned NPC state across a guest's
   departure and return, using the existing save/persistence authority; a
   guest joining a running world from a host snapshot. A late join
   (2026-11-12, above): a guest that adopts the host's save builds the room
   as the host has it (a broken wall broken) and the rows agree at once;
   without it the wall stands for good. An NPC's life (2026-11-12, above):
   the save carries what a life holds (the scientist on the bench), not
   where it is in its plan; a guest that shows lives needs each life's step
   on the wire. NA-3 is measured.
4. **NA-4** once 1–3 have numbers.
