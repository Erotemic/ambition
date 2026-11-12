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
   motion). Next: a census in a room with breakables and summons; send only the guest's room;
   sounds and visuals sent, with room numbers mapped.
2. **NA-3 — guest prediction.** Run the guest's own body through the
   movement kernel against its room's collision, reconcile to each snapshot,
   and measure correction size under the same scripts. The kernel already
   runs per body (`transit_body`, `BodyKinematics`); the open question is
   what else a prediction needs (contacts, abilities, held items).
3. **NA-3 — NPC facts and late join.** Host-owned NPC state across a guest's
   departure and return, using the existing save/persistence authority; a
   guest joining a running world from a host snapshot.
4. **NA-4** once 1–3 have numbers.
