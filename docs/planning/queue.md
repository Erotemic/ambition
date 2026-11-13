# The queue — live execution order

This file contains **executable, unresolved engineering work**, not a work log.
An item remains here only with a reproducible defect or concrete customer,
a next action, and a checkable acceptance condition. Re-measure the source
before starting a row; retire it upon completion. Focused owner documents
carry architecture, measurements and implementation history; Git history
carries closed campaigns. Unselected or customer-gated plans live in
[`tracks.md`](tracks.md), and product questions in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

## Current order and selection

1. **Network architecture (maintainer priority, updated 2026-11-10):** [NET-AUTHORITY-EXPERIMENT](#net-authority-experiment--compare-hosted-world-replication-with-ggrs) compares player-hosted authoritative Ambition (with client prediction) against existing GGRS; Smash keeps bounded rollback. [NET-INPUT-WIDTH](#net-input-width--remove-idle-room-consensus-from-every-frame) compacted the GGRS input (done 2026-11-10); its measurement says message count, not input width, drives that backend's link traffic. **Direction set 2026-11-10:** the metroidvania moves to a hybrid model, with rollback kept inside a room boundary while it is easy to maintain and its room-level cost measured, and hosted authority for the world around the rooms: [SIMULATION-ISLANDS](#simulation-islands--room-level-rollback-inside-a-hybrid-world) is selected and runs beside the experiment.
2. **Multiplayer correctness:** [MULTIPLAYER-SEAMS](#multiplayer-seams--make-local-multiplayer-safe-for-netplay) feature/tooling validation;
   then LOCAL-PLAY-MODES presentation (LOADOUT-AUTHORITY and the menu leave
   done 2026-11-10). Honor the
   2026-10-19 ruling order for CHARACTER-FORM, COOP-RECOVERY,
   PARTNER-RECOVERY and DIALOGUE-CHECKPOINT; do not re-implement
   already completed morph-ball visibility, joining, party travel, baseline
   split-view or peer room-commit mechanics.
3. **Selected product/asset fixes (2026-11-01 triage):** ASSET-SELECTION,
   SMASH-DEFECTS (done 2026-11-10), ITEM-CONTENT, ASSET-PUBLISH and
   ASSET-PREVIEW in that order.
   A9's completed profile proof is not an automatic invitation to rewrite the
   capability architecture. Continue AP14 ownership work before lower-priority
   ENEMY-NOTICE, WORLD-LAYERS and POSE-CONTINUITY; then ABILITY-VARIANTS and
   CHARGEN-SPECIES.
4. **NPC life and boss product work:** [RADIANT-NPC-HTN](#radiant-npc-htn--authored-npc-lives-and-replaceable-planning) tests text-authored HTN planning and persistent schedules. [BOSS-CINEMATIC-COVERAGE](#boss-cinematic-coverage--give-every-boss-a-wolfenstein-class-name-intro), [GNUTON-GNU2-FINALE](#gnuton-gnu2-finale--summon-a-second-gnu-through-a-real-portal), and [GNU-MOTION-PARTS](#gnu-motion-parts--diagnose-and-fix-non-smooth-giant-motion) are selected implementation customers; follow their owner plans and preserve existing shipped encounter phases until replacements pass.
5. **Developer iteration and inspection:** [GEOMETRY-OBSERVABILITY](#geometry-observability--render-the-effective-geometry-behind-gameplay) makes the effective geometry behind collisions, attacks, interactions, portals and navigation visible and machine-readable; it is a concrete inspection customer, not a broad spatial rearchitecture. [TEST-LANES](#test-lanes--keep-required-test-lanes-executable) and CONTENT-AS-DATA
   remain selected at their stated boundaries (TOOL-DISCOVERY and EDITOR-SHELL
   are complete: [tooling layout](engine/tooling-layout-and-editor-consolidation.md),
   T0-T5; retired from this queue 2026-11-11). Portal, rendering,
   combat and map rows below are independent bounded work, not a second
   priority list; select them after correctness constraints are respected.

Current source may be ahead of these dated ordering decisions. Do not treat
old measurements as an obligation to redo completed work. Detailed
architecture proposals in
[the reassessment](engine/architecture-reassessment-and-composable-multiplayer.md)
do not by themselves authorize networking/physics/rollback rewrites.

**Automation/link contracts:** `scripts/checks/check_blocking_set_names_every_gate.py`
reads each `**Blocked by:**` field through its next blank line, plus
`**Acceptance:**` clauses. A heading containing `✅`, `DONE` or `CLOSED` marks
a closed row for the consolidation/hold guards. Keep the **ID-PEER** and
**A10** closed receipts below: the consolidation checks use their IDs and
state. Inbound `queue.md#...` links require retaining exact heading anchors
or updating their source. Check with
`python3 scripts/checks/check_planning_anchors_resolve.py`.

## P0 — architecture and correctness

### NET-AUTHORITY-EXPERIMENT — compare hosted-world replication with GGRS

**Customer:** persistent Ambition multiplayer, NPC lives, late join and different-room participation have different cost/authority needs than Smash rollback. The App-wide GGRS rewind (and the per-message link cost measured by NET-INPUT-WIDTH) may be unnecessary for the flagship's intended player-hosted experience.

**Owner:** [authority-model experiment](engine/hosted-world-networking-experiment.md) and [network topology](engine/network-topology-and-authority-research.md).

**Next action:** NA-1 and a first NA-2 are measured (2026-11-10, [results](engine/hosted-world-networking-experiment.md#na-1-and-na-2-2026-11-10)): each GGRS peer spends about 4× the host's update per frame even without rollbacks, and room-scoped replication bytes depend on delta encoding. Half of that is `SaveWorld`: 1,196 systems, about 16 ms per run whatever the body count. The 2026-11-10 direction (hybrid; rollback kept per room while cheap) makes this the hosted half of [SIMULATION-ISLANDS](#simulation-islands--room-level-rollback-inside-a-hybrid-world). NA-2b's first measurement ([result](engine/hosted-world-networking-experiment.md#na-2b-first-measurement-2026-11-10-a-guest-app-that-simulates-nothing)): a guest App with every simulation phase gated off costs about 4.5 ms an update against the host's 9–10, and applying 2–18 body rows costs about 0.1 ms. The gated guest's update split (2026-11-11): alone it costs about 2.2 ms, nearly all `Update`'s 1,030 gated systems, so a guest should be composed from fewer plugins rather than gated. Done 2026-11-11: shots spawned and ended on the host are built and removed on the guest by `SimId` from a spawn row per kind; its ids equal the host's after every snapshot, at about 1.3 link bytes a frame ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). A death on the host is a row change, not a despawn (the killed enemy's body stays), so the body row must carry the defeat; done 2026-11-11 ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)): health by `SimId` (2 bytes a body) puts the guest's enemy down one snapshot after the host's with no disagreement, but what draws a death is gated simulation state; done 2026-11-11 ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)): the `DeathBeat` by `SimId` (3 bytes a body) makes the guest draw the corpse as the host does (without it, never), though the kill's shake and hold stay on the host. Without the gated systems (2026-11-11, [result](engine/hosted-world-networking-experiment.md#na-2b-the-gated-guests-update-split-2026-11-11)): taking the simulation's 557 systems out of a booted guest saves 0.24–0.46 ms of about 2.1, and the rest is 335 composition and presentation systems (other games' demos, the conversation, the shell), so a cheap guest is a narrower composition, not a missing simulation. Done 2026-11-11: the kill as a presentation event: the host's camera shakes, forwarded with each snapshot, make the guest shake with it and the control never does, at 0.11 link bytes a frame ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-11: a guest of Ambition alone (`AMBITION_HUB_GAMES=ambition`) has 136 fewer systems and costs no measurably less (1.8–2.2 ms an update against 1.9–2.9, two runs each); its cost is Ambition's own conversation, shell and load presentation ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-11: what else a host spawns: in the kill run, only the shots (by `SimId`) and the enemy's bounty coin, which has no `SimId` (drops are anonymous by design), so a guest replicating by `SimId` never sees it ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Done 2026-11-12: a dropped pickup keyed by its `SpawnOrigin` (the parent's `SimId` and the sequence): the guest builds, draws and removes the bounty coin one snapshot after the host, with no difference in what a view draws, at 0.38 link bytes a frame, most of it the magnet's motion ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: a breaking wall is a row change on a `SimId` both Apps hold, its drop takes the pickup road, and a swing's short-lived strike volumes are the only other new ids (drawn from the body's move, so no row) ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: a summon is a body descended from its summoner's `SimId` and already in the body rows; its spawn row names its character ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: a snapshot of the guest's room only is 27 bytes a frame of body rows against 398 unscoped once the players part, and a body that leaves the guest's room is removed by the general rule ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: the host's sounds and visuals forwarded with the snapshot; only a visual that names a body (a swing's owner) needs translating, by `SimId`, and every one did ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: a gated guest does not follow its player's crossing (room changes run in the simulation it gates): it keeps the old room and builds the new room's bodies with no room ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). Measured 2026-11-12: with a room row (the host's number and the definition) the guest records its player's crossing and its own room construction builds the room two frames later, if the room transition phase stays awake; an unmapped room's bodies are deferred, so only another player's body needs a spawn row ([result](engine/hosted-world-networking-experiment.md#na-2b-spawns-and-despawns-by-simid-2026-11-11)). NA-2b is measured. NA-3 first measurement 2026-11-12: a guest predicting its own body is exact while no other body acts on it; with Alice swinging beside Bob the corrections average 1.45 units, 0.37 with offset reconciliation ([result](engine/hosted-world-networking-experiment.md#na-3-guest-prediction-first-measurement-2026-11-12)). Measured 2026-11-12: the corrections are Alice's strikes on Bob, and reconciling by replaying the guest's own room since the snapshot (ISL-2's room rewind on the guest) brings the mean to 0.068 units at about 9 ms a frame of replays in a debug build ([result](engine/hosted-world-networking-experiment.md#na-3-guest-prediction-first-measurement-2026-11-12)). Measured 2026-11-12: a late guest that adopts the host's save builds the room as the host has it (a broken wall broken) and the rows agree at once; without the save the wall stands for good ([result](engine/hosted-world-networking-experiment.md#na-3-a-late-join-2026-11-12)). Measured 2026-11-12: the save carries what an NPC life holds, not where it is in its plan, so a guest showing lives needs each life's step on the wire ([result](engine/hosted-world-networking-experiment.md#na-3-a-late-join-2026-11-12)). NA-4 recorded 2026-11-12 in [ADR 0036](../adr/0036-synchronization-is-an-optional-game-capability.md#per-game-synchronization-na-4-2026-11-12): Smash keeps GGRS; Ambition is hybrid as directed on 2026-11-10. Next: production code (with SIMULATION-ISLANDS): a guest App composed from fewer plugins, a transport, the room row and spawn rows, and each life's step as a row. Evaluate compatible Lightyear and `bevy_replicon` APIs without declaring a dependency before the prototype.

**Acceptance:** runnable headless comparison, byte and CPU measurements, stable guest input/reconciliation, region-scoped replication, persistent host-owned NPC facts, no service dependency, and an explicit per-game synchronization decision. Do not remove GGRS or rewrite working multiplayer merely from prose.

### SIMULATION-ISLANDS — room-level rollback inside a hybrid world

**Customer:** the Ambition metroidvania's multiplayer: players in different rooms must not pay for, wait on or rewind each other's rooms. Maintainer direction 2026-11-10: move away from strict App-wide rollback toward a hybrid model; keep rollback within a room boundary while it is easy to maintain, and evaluate what keeping it at room level costs.

**Owner:** [simulation islands](engine/island-rollback-and-compact-peer-input.md#simulation-islands-independently-rewindable-active-areas); the hosted half is [NET-AUTHORITY-EXPERIMENT](#net-authority-experiment--compare-hosted-world-replication-with-ggrs).

**Done 2026-11-10 (ISL-1, census):** in a two-room scene every rollback entity but four has a room (22 in the hub, 2 in `switch_lab`; the four are the two room roots, the session root and a boss encounter). One `SaveWorld` run costs 16.1 ms over 1,201 systems for those 28 entities, so a history per room on the per-type mechanism would multiply that by the live rooms. Five resources couple rooms (impact hitstop and the clock scale, the projectile id sequence, the one conversation, the one cutscene) ([result](engine/island-rollback-and-compact-peer-input.md#isl-1-result-2026-11-10-what-has-a-room)).

**Done 2026-11-10 (ISL-2a, cost):** a room snapshot through the registered rows (`RoomSnapshotRows`) saves the hub (22 entities) in 1.35 ms and `switch_lab` (2) in 0.31 ms, against 16.3 ms for the App-wide `SaveWorld`; it holds no resources or checksum yet ([result](engine/island-rollback-and-compact-peer-input.md#isl-2a-result-2026-11-10-what-a-room-snapshot-costs)). Room-level rollback is affordable on a snapshot that follows the room's entities.

**Done 2026-11-10 (ISL-2b, first witness):** in a hosted world the hub rewinds to a saved frame and replays Alice's inputs alone, with Bob's room hidden (`Disabled`): the hub ends exactly as the first run left it and Bob's room is untouched; the control without hiding moves Bob ([result](engine/island-rollback-and-compact-peer-input.md#isl-2b-first-witness-2026-11-10-a-room-rewinds-alone)). The window spawns nothing and exercises no coupling resource.

**Done 2026-11-10 (ISL-1b, the freeze):** with several live rooms an impact hold or Player 1's hitlag no longer stops every room's clock; one live room keeps the freeze ([owner](engine/island-rollback-and-compact-peer-input.md#next-isl-2c-and-isl-1b)).

**Done 2026-11-10 (ISL-2c, shots):** a room restore despawns what was spawned after the save and re-creates what is gone (references remapped); in the rewind witness Alice's shots fired in the window are fired again by the replay and end where they did.

**Measured 2026-11-10 (ISL-2d):** a coin taken in a rewound window pays the wallet once (it is on the body and rewinds) but is recorded twice in the world's grant record; the chosen design (world records stamped with room and tick, retracted on a room rewind) is built for the grant record (an append store) and for the consumed-occurrence record with the occurrence ledger (keyed: an undo log of the rows each write replaced, witnessed by a heart taken only in the mispredicted window), and for the shared bag (each write inverted: a throw only in the window is put back, and its mint leaves the occurrence ledger), and for boss defeats (with the save row, quest steps and reward the death replay already retracts), and for the world-time schedule (a mirror that heals itself except for the first departure's due time; its undo log is bounded by a 600-tick rewind horizon); and for a body's cell use from the bag (a conversation's grant waits on the one-conversation coupling); and for the save's flags and the quest steps they move (each room's flag write logged with the value it replaced, its quest advance caused by it; a later write to the flag that stands keeps both; and for wave encounter clears: the switches they green and the quest steps they move; and a pickup's `ItemCollected` quest event, retracted under a cause naming its room and tick), and the room rollback host will call it ([result](engine/island-rollback-and-compact-peer-input.md#isl-2d-measured-2026-11-10-a-world-write-in-a-rewound-window)).

**Measured 2026-11-10 (ISL-2e):** a hosted world that predicts its guest and rewinds only his room on a late input ends where an on-time run ends, at 9.4–16.3 ms of host update a frame (the late-applying host: 8.4–9.9; each GGRS peer: 33–48) ([result](engine/island-rollback-and-compact-peer-input.md#isl-2e-measured-2026-11-10-what-room-level-rollback-costs-a-hosted-world)).

**Next action:** make the room rollback host production code once the hosted world has a production input path (NA-2b's guest App and transport; until then the rewind has no caller but tests): per-room input history, prediction, a rewind on a late input, the shared clock and message channels, the world-write retraction of ISL-2d. Meanwhile: ISL-2c (the hit on an NPC is done: `a_rewound_room_hurts_its_enemy_again`; an NPC life's ledger writes are done 2026-11-11: the rows mirror the rewound life, and the bookings a room's bodies add are taken back by its rewind); ISL-1b's couplings are done, each with a witness that an event in one room leaves the other unchanged (2026-11-11: bullet time slows Player 1's room only; a room's impact hold and clock scale rewind with it on its root's `RoomPace`; a room's rewind puts back its conversation, `ConversationsInRooms`, and its cutscene, `CutscenesInRooms`; a room's cutscene holds only its own room's players and dates its flags in its room; a conversation's grants and sales are its seat's, paid from that body's wallet and dated in its room). Still open: one conversation and one cutscene at a time (two at once need a text box and an overlay per view), projectile ids sequenced per room (deferred to a guest that mints its own shots, NA-3's prediction; NA-2b's guest mints none), and nothing else on the clocks: a module entry's `dt` follows its scope's room (2026-11-11, `ScopePace`), and the match clock, banners, room transitions and passages and the gate network stay on the shared pace by ruling (each is the session's, presentation, or spans two rooms). The fight's own clocks (strike windows, cooldowns, fuses) and, since 2026-11-11, the body-local and room-local ones follow the room through `RoomFreeze` (`ambition_platformer2d_shared_tangle::lifecycle`): items, platforms, breakables, mounts, mana, pets, captures, hazards, death interludes, boss phases, encounter waves and scripts, trails, and a room's cutscene.

**Acceptance:** per the owner's ISL-2–ISL-4 acceptance, with every step's cost measured against the App-wide baseline; Smash keeps its single rollback domain.

### RADIANT-NPC-HTN — authored NPC lives and replaceable planning

**Customer:** civilian NPCs need schedules, commitments, social interactions, smart-object affordances and meaningful offscreen life; the current `CharacterBrainTemplate` policy catalog is closed. Nikobolt and Coulomba share one laboratory and are the initial integrated customer.

**Owner:** [Radiant NPC planner](engine/radiant-npc-planning.md), [agentic character runtime](engine/agentic-character-runtime.md).

**Done 2026-11-10 (slice 1):** compared `bevy_htn` 0.0.1 (Bevy 0.15, text domains, reflected component state, random plan ids) and `bevy_bae` 0.1.0 (Bevy 0.18, Rust-bundle domains, `World` properties) with a standalone core, and chose the core: `crates/ambition_htn` (no Bevy; RON domain validated when read; facts in, an explained plan out; 4.9 us per plan on the scientists' benchmark day) ([result](engine/radiant-npc-planning.md#slice-1-result-2026-11-10-a-standalone-core)).

**Done 2026-11-10 (slice 2, first part):** a body can be sent to a place: `Visit` (rollback state), answered by the navigation advisor and settled `Arrived` / `NoRoute`; the basement dog witnesses both ([result](engine/radiant-npc-planning.md#slice-2-first-part-2026-11-10-a-body-sent-to-a-place)).

**Done 2026-11-10 (slice 2):** the executor (`body::life`): a game's `LifeBook` (Ambition's pack file `data/npc_lives.ron`, schema `npc_lives`, republished by a content reload) gives a character a `Life` (rollback state); it plans on each new hour from its own facts, each `go_to` is a `Visit` to a place the room names (signs `workbench` and `lunch` in the coil lab), other verbs hold for their authored hours, and a step done makes its effects facts. Nikobolt works at the bench and lunches at noon; Coulomba (no life) is the control; the day resimulates under the bench hand-over's sync test ([result](engine/radiant-npc-planning.md#slice-2-2026-11-10-nikobolt-lives-his-day)).

**Done 2026-11-10 (slice 3):** two scientists, one bench: a claim is simulation state (`Life::claim`; a lost claim replans at once and the life keeps why), observed facts (`Free`, `Present`) are read from the world when a body plans, and a spent plan replans when what it observes changes. Never two holders at once; both get a turn; the hand-over survives a sync test ([result](engine/radiant-npc-planning.md#slice-3-2026-11-10-two-scientists-one-bench)).

**Done 2026-11-10 (slice 4):** interrupt and replan: a conversation stops the talked-to body's plan (outside the gameplay gate, which dialogue stops) and it replans when the conversation ends; a refused route or a place gone replans at once and leaves `unreachable:<place>` / `absent:<place>` for the domain; each new plan says why ([result](engine/radiant-npc-planning.md#slice-4-2026-11-10-interrupt-and-replan)).

**Done 2026-11-10 (slice 6):** an edit to `data/npc_lives.ron`, published by a content reload, changes Nikobolt's next plan in the running shipped game with no Rust build; hours are `authored` facts read from the book at each plan ([result](engine/radiant-npc-planning.md#slice-6-2026-11-10-an-edited-life-in-the-running-game)).

**Done 2026-11-10 (slice 5, first part):** offscreen and re-entry: a `LifeLedger` keeps each character's facts and plan hour; a body built again catches up the hours it missed by schedule arithmetic and is put where the schedule sent it. Away from 9:30 to 19:30, Nikobolt is at the door gone home with 7 experiments and Coulomba at the bench with 8, both fed ([result](engine/radiant-npc-planning.md#slice-5-first-part-2026-11-10-offscreen-and-re-entry)).

**Done 2026-11-10 (slice 5, second part):** the lives are in the save (v6) and come back in a session started from it; a session without it is the control ([result](engine/radiant-npc-planning.md#slice-5-second-part-2026-11-10-the-lives-in-the-save)).

**Done 2026-11-10 (review findings 2, 3, 4, 6, 8):** `daily` facts start each day again (day-2 lunch witnessed); `say` speaks a line and a verb neither the executor nor `holds` knows is refused when the book is read; the save is written only when a life changed; the executor plans through the `ambition_life_planning::LifePlanner` contract (the HTN is one planner); offscreen hours book an exclusive thing by the hours its holders' steps take, in character-id order ([offscreen](engine/radiant-npc-planning.md#slice-5-first-part-2026-11-10-offscreen-and-re-entry)).

**Next action:** slice 7 waits on the [authority experiment](engine/hosted-world-networking-experiment.md)'s choice. Done 2026-11-10: the day clock is in the save (v7), so a session from a save goes on at the hour its lives planned in. Done 2026-11-10: offscreen bookings outlive the tick in the life ledger, so bodies that come back on different ticks share a thing's hours. Done 2026-11-10: the bookings are in the save (v8). Done 2026-11-11: a live holder's hours are booked, for the time held, for a body that catches up.

**Acceptance:** both scientists follow different authored schedules, share an exclusive lab affordance, react/replan, persist on departure and reentry, explain their plans and remain correct under save/reload and hosted authority. Change their activity policies without an engine-host compile. Do not require LLM inference or all-peer rollback.

### BOSS-CINEMATIC-COVERAGE — give every boss a Wolfenstein-class name intro

**State:** the roster-wide card landed 2026-11-10 (every boss shows its name after it wakes: the encounter's `BossNameCardCue` by default, its conductor's for Wolfenstein and the Inugami), and each local view shows the card of the boss in its own room (2026-11-10; see the [owner](game/boss-cinematics-gnu2-and-motion.md#name-cards-boss-cinematic-coverage)). What remains is acceptance evidence: a rendered witness of the card over several fights.

**Owner:** [boss cinematic and GNU2 plan](game/boss-cinematics-gnu2-and-motion.md), [boss content](game/bosses.md).

**Rendered 2026-11-10:** a GPU capture witness draws the card for two non-Wolfenstein bosses (the T-rex and the Clockwork Warden), counted in the name's own rectangle against a capture before the wake ([owner](game/boss-cinematics-gnu2-and-motion.md#name-cards-boss-cinematic-coverage)).

**Rendered two views 2026-11-10:** each of two split views in two boss rooms draws its own boss's card in its own half ([owner](game/boss-cinematics-gnu2-and-motion.md#name-cards-boss-cinematic-coverage)). The same captures show the boss health bar, a title banner and a dialogue box spanning both views (recorded there; per-view HUD work).

**Next action:** none on the card. No skip input exists; if one is added, it advances the phase state and the card follows.

**Acceptance:** each active boss gets a readable cinematic name/title reveal, at correct intro phase and view, with safe skip/replay/cleanup; bespoke intros still work without double presentation. Roster census and rendered witnesses cover multiple non-Wolfenstein fights.

### GNUTON-GNU2-FINALE — summon a second GNU through a real portal

**Failure:** GNU-ton's phase-3 well uses a real floor/overhead portal pair but changes the existing hall's sky and gravity to represent space. The preferred new finale is a large live portal summoning physically present GNU2, followed by simultaneous GNU and GNU2 combat. GNU2 art is currently an unpublished art-only prototype.

**Owner:** [boss finale and acceptance](game/boss-cinematics-gnu2-and-motion.md) and [GNU-ton debts](game/gnu-ton-fight-debts.md).

**Next action:** register published GNU2 character and combat geometry; author a same-active-area portal staging source, actual through-aperture entrance, combined fight and multi-boss victory policy. Replace obsolete sky/space-phase authority only after replay, reset and exit gating pass.

**Acceptance:** physically observed GNU2 portal emergence, simultaneous independently damageable GNU/GNU2 giants with readable counters, either defeat order supported, door gating on both plus rider yield, no duplicate spawn on retries, and both bosses safely cleaned on teardown. Keep intra-island portal semantics and existing phase 1/2 gameplay.

### GNU-MOTION-PARTS — diagnose and fix non-smooth giant motion

**Measured and partly fixed 2026-11-10** ([owner](game/boss-cinematics-gnu2-and-motion.md#the-giants-motion-gnu-motion-parts)): the walk row's clock now follows distance walked, so the hooves stand still as the body speeds up and slows down, and rest starts at its first frame after a blow. Measured 2026-11-11 ([owner](game/boss-cinematics-gnu2-and-motion.md#the-giants-motion-gnu-motion-parts)): phases 2 and 3 walk at the same 135 per row second, the rider's rows start at 0 and step by one frame, and the stomp plays stretched like the windups. Two hitches are portal transit's (the portals work): the giant falling out of the well's exit portal stands still on every second or third frame for about a quarter second, and a free fist in a portal pair jumps about 280 units between the pair's ends every frame. Rendered part transforms are not measured yet.

**Failure:** GNU-ton's giant appears to move non-smoothly. It already has part-sheet assets, but the conductor pins mount animation rows separately from its `MountWalk` motion, rider and fist poses. Root/interpolation/part-pose timing has not been measured; missing runtime part use is a hypothesis, not a confirmed cause.

**Owner:** [motion diagnosis and fix](game/boss-cinematics-gnu2-and-motion.md), [semantic-part rendering](engine/semantic-part-rendering-and-ragdolls.md).

**Next action:** capture frame-by-frame mount motion, row clocks, rendered part transforms, portal clipping, rider and fists. Identify root, animation or interpolation discontinuity; unify the responsible pose authority or shared render adapter rather than applying per-boss correction offsets.

**Acceptance:** walking, turning, buck, rider movement and portal strikes are visually continuous and align with collision, footholds, hits and camera, across fixed ticks, relevant render rates, reset/replay and quality levels.

### ITEM-CONTENT — remove retired items without letting fixtures retain them

**Done 2026-11-10 for the grid.** The Javelin and the Sealed Note are gone
from the item grid, `items.ron`, the intro dialogue and every test that
granted them. Their slots are "Empty" reserved cells (`Item::Empty3`,
`Item::Empty4`), as the Axe's and the Map Fragment's were, so no other item's
index or `dialog_id` moved. Replacements:

- **The intro handover:** Alice sends Bob a Spare Battery for his survey lamp
  (`intro.yarn`). Bob takes one only once Alice's flag is set, so a battery
  bought at the port is not hers. The flag names (`alice_route_note_carried`,
  `bob_field_survey_received`) are the save's and stay. Witness:
  `alices_battery_reaches_bob` (renamed from `the_note_travels_from_alice_to_bob`),
  with a new control: a bought battery is not offered to Bob.
- **The bag-minted throwable** of the checkpoint and save fixtures: the
  Gun-Sword (`death_restores_the_checkpoint`) and the Grapple
  (`two_persistence_authorities_for_one_item`, whose authored reward is
  already the gun-sword), thrown by Shield+Attack, which throws anything a
  hand holds.

**Usage list, 2026-11-10** (method: whole-word match of each item's
`dialog_id` and `held_item_id` in content files under
`game/ambition_content/assets`, `tools/ambition_ldtk_tools/specs` and
`game/ambition_map_assets`, `items.ron` excluded; an ability id such as `dash`
or `fireball` also matches move and boss names, so its count is an upper
bound, not a list of grants): Fireball 8 files, Eye Laser 1, Lightning 4,
Dash 20, Grapple 20, Mark / Recall 4, Gun-Sword 24, Puppy-Slug Gun 3, Bomb 24,
Health Cell 3, Mana Cell 2, Spare Battery 3 (quests, the port's shop, the
intro), Field Survey 1; Sword Stance, Axe Stance, Whip Stance, Precision
Blink, Contextual Blink, Dash II and Portal Gun name no content file (the
portal gun is placed by its own spawn kind; the stances and blinks are worn
through `items.ron` rows).

**Open:** the held object `javelin` (a held-item spec, not a grid item) is the
portal similarity lab's asymmetric gauge
(`tools/ambition_ldtk_tools/specs/portal_similarity_lab.yaml`), and the held
item and save unit tests still use it as a pure throwable. The lab is the
portals work's; when it picks another gauge, the spec, its art row and those
tests can switch to the coconut.

**Owner:** item content, held specs, world placements, intro dialogue and their
behavioral fixtures. Design: [ability loadout](game/ability-loadout.md).

**Acceptance:** the equipment and quest pages reflect actual content. No
placement, dialogue, recipe or save reader names a removed item. The retained
custody and persistent-world witnesses still exercise real production paths.

### ABILITY-VARIANTS — finish the authored loadout through shared capability seams

**Failure:** the loadout plan still has placeholder variants and no authored
upgrade fact.

**Owner:** equipment preparation, body control slots, content modules and
character landmarks. Design: [ability loadout, open work](game/ability-loadout.md#open).

**Done 2026-11-11:** the published `eye_laser` row draws, and the laser leaves
the visor its art draws ([loadout](game/ability-loadout.md#open)).

**Next action:** the laser's own sound waits for an SFX bank republish. Support
Blink and stance replacement through the existing technique/equipment seam,
with one authoritative upgrade fact. Author Contextual Blink, Precision Blink
tiers and the remaining variants in that model, not named engine branches.

**Acceptance:** each real variant changes its declared slot on any supported
body. Replacing or removing equipment restores the prepared intrinsic kit.
A placeholder stays visibly unavailable until it has an admitted implementation.
Tiers, equipment and checkpoint/save state agree. Keep quality selection out
of gameplay authority.

### CHARGEN-SPECIES — species parameters and asymmetric overlay views

**Failure:** the overlay mechanism is built, but species heads still expose
mainly muzzle and ear choices. Mirrored left cells move asymmetric details
to the wrong side.

**Owner:** renderer `chargen` specification, species geometry, overlay data and
studio applicability. Design: [species knobs and overlays](game/chargen-species-knobs-and-2d-overlays.md).

**Dog and pigeon done 2026-11-11 (renderer 93d8e69):** the dog has stop,
skull, flews, ear length, fold and set, and a mask, blaze or saddle marking;
the pigeon has beak curve, cere, neck sheen and eye ring
([knobs](game/chargen-species-knobs-and-2d-overlays.md)). Defaults draw the
old heads byte for byte and 3447 records keep every older field; no resident
is a dog and the four pigeon residents pin the defaults, so no published
product changed and nothing was republished.

**Next action:** the other listed species (horse, cat, bird, sea lion,
alligator). Keep parameter defaults, generation and studio controls
at their canonical owners. Add explicit left cells for asymmetric details;
reuse the existing overlay mechanism. New random draws follow existing draws.

**Acceptance:** head crops and posed clips show distinct valid shapes in every
view; heads stay attached and asymmetric features keep their side. Existing
seeds retain their established fields. Publish each changed product and its
quality variants through the current pipeline. No runtime shader redraws the
character or changes gameplay geometry.

### ASSET-SELECTION — one selected-file contract for runtime and tools

**Failure:** the inspector reads loose audio and font paths while the runtime
uses selected publications. Python packaging and Rust also differ on missing
publications and stale generation identities.

**Owner:** `ambition_asset_manager::content_library`,
`scripts/lib/content_libraries.py`, `tools/ambition_editor/asset_index`, and
provider library installation.
Design: [asset library review, A and E](engine/asset-library-implementation-review.md#a-selection-is-correct-in-the-runtime-but-not-uniform-in-the-tools).

**Done 2026-11-10:** the selected-file contract is six shared cases that the
Rust runtime reader and the Python packaging reader both pass (Python now
treats an empty store as an empty selection and a stale identity as an
advisory; the packager states its stricter "has no publication" refusal at
its own boundary). The inspector locates music, the registry, the SFX bank
and fonts through the selected set and lists shadowed loose copies as
unselected workspace products; on this checkout 19 of 19 published cues now
have their audio located, where none did
([review, A and E](engine/asset-library-implementation-review.md#a-selection-is-correct-in-the-runtime-but-not-uniform-in-the-tools)).

**Next action:** the old-publication clause of the acceptance at the runtime
(a selection written by an older tool starts the native game without a
rebuild) has a case at the reader, not a launch witness.

**Host registration landed in `fb1621179`:** providers import identities and retain the
host's selected source. Explicit conflicting selections still refuse. Guard:
`a_provider_import_keeps_the_host_selection_but_an_explicit_conflict_refuses`.
The native app with served-provider support and standalone served Mary-O step
successfully. Keep provider imports separate from explicit source selection.

**Acceptance:** shared behavioral fixtures produce the same selected membership
and origin in both languages. Published-only assets resolve in the inspector;
loose unselected copies remain visible as workspace products. A usable old
publication can run without a rebuild or a deep audit. A native host with
served-content support enabled admits its shared library once; standalone
served games still run.

### ASSET-PUBLISH — selection does not audit old bundle payloads

**Failure:** acceptance hashes the source and copied product; selection calls
`integrity()` for every selected bundle, including unchanged bundles.

**Owner:** renderer publication store and library producer scripts.
Design: [asset library review, B](engine/asset-library-implementation-review.md#b-publication-works-but-checks-the-past-on-every-change).

**Done 2026-11-10 (measured):** accept is one copy-and-hash pass (a 2.1 MB
unit: 6.3 MB read before, 2.1 MB after); publish selects from accepted
manifests with ownership checks and the atomic swap, and checks only that a
newly selected bundle has its members (onto 200 bundles: 1.1-1.2 s and 423
MB read before, 59-67 ms and 1.1 MB after); deep payload inspection is the
explicit `Store.audit()`
([review, B](engine/asset-library-implementation-review.md#b-publication-works-but-checks-the-past-on-every-change)).

**Next action:** none in this row. No release path calls `audit()` yet; the
release asset contract is ASSET-PREVIEW's boundary.

**Acceptance:** focused publication does not read or hash old payloads.
Concurrent independent publications survive the merge; a failed producer
leaves the previous selection usable. Stale parents remain advisory. Report
measured cost; do not add a new binary cache or store service.

### ASSET-PREVIEW — local composition and release audit are separate operations

**Failure:** linked web preview composition hashes every selected input and
then audits the composed tree as if it were a release.

**Owner:** `scripts/package/package_asset_guard.py` and web preview setup.
Design: [asset library review, D](engine/asset-library-implementation-review.md#d-the-web-development-packager-does-release-level-work).

**Done 2026-11-10 (measured):** `compose --policy local` composes the
selected file set without hashing, checks the tree by membership and reports
missing inputs; `build_for_web.sh --served` uses it for a linked tree. An
indexed alias lookup (the old per-declaration scan was 60 of 105 profiled
seconds) speeds both policies: release 59 s to 38.6 s on this checkout, local
22 s ([review, D](engine/asset-library-implementation-review.md#d-the-web-development-packager-does-release-level-work)).

**Next action:** run the Android, Steam Deck and static web release
packagers' final asset contracts on a machine with every generated family;
this VM lacks four, so release composition refuses here before and after.

**Acceptance:** local preview uses the same selected file set without hashing
all media. Missing required files have a useful diagnostic. Android, Steam
Deck and static web release packages still pass their final asset contracts.

### GEOMETRY-OBSERVABILITY — render the effective geometry behind gameplay

**Failure:** F1 currently draws a useful but hand-assembled set of world, body, combat, portal and feature gizmos. Some outlines represent base authored blocks or coarse AABBs rather than the resolved, subject-specific geometry that simulation actually tests; interaction reach and rejection conditions lack comprehensive visibility. New geometry domains do not automatically become discoverable debug layers.

**Owner:** [geometry observability and F1](engine/geometry-observability-and-f1.md), under the [inspection/workbench program](engine/inspection-diagnostics-and-workbench.md).

**Next action:** GOBS-0 is done (2026-11-10, [census and lab](engine/geometry-observability-and-f1.md#gobs-0-result-2026-11-10-census-and-the-first-lab)): F1's blocks layer draws 8 of the 18 solids a body meets in `basement_breakables`, and the lab found a broken platform's lingering pogo orb (fixed). GOBS-1's first slice landed 2026-11-10 (the `ambition_geometry_observation` registry, collision and combat providers, F1's Observed Geometry layer reading the same `observe_all` as headless; and the headless `inspect_geometry` binary with provider selection; and F1's per-provider selection, a Developer-page row that cycles all → each installed observer; and labels on the selected observer's pieces: GOBS-1 is done); GOBS-2 render effective per-body collision and contact decisions (first slice 2026-11-11: the `body.sweep` observer draws each body's swept box, its path and the ground, head and wall contacts the kernel reported, in F1 and headless); GOBS-3 cover combat, interaction, triggers, navigation and portal pieces (interaction done 2026-11-11: `interaction.reach` shows the reach, each candidate, the one the press is for and the rule's reason for every refusal; navigation done 2026-11-11: `navigation.graph` shows the advised graphs' surfaces, hops and drops and each body's route and goal; room regions done 2026-11-11: `world.regions` shows each loading, camera, gravity and encounter region and lock wall where it is authored); GOBS-4 provide practical F1 selection/presets and interpolation comparison; GOBS-5 close coverage and cost regressions. Preserve the already working F1 and combat-view paths.

**Acceptance:** a single representative two-room lab exposes actual collision/hit/hurt/interaction/portal/navigation geometry with source identity, state and why-not reasons; a new provider becomes discoverable in F1 and headless inspection without modifying a central game-specific draw switch; overlays are room- and view-correct, off by default, and do no per-frame diagnostic work when disabled. Tests distinguish absent, filtered, intangible and unsupported geometry. Do not confuse an authored block outline with a subject's effective collision world.

### ENEMY-NOTICE — complete first-sight beats on the existing brain paths

**Failure:** the dilophosaurus and other skirmishers have authored notice art
or sound but no notice interval in `SkirmisherCfg`. The enemy-pattern plan's
claim that all fighter-template enemies lack notice is stale: fighter
engagement already implements it.

**Owner:** `ambition_characters::brain::state_machine`, combat brain emission,
and authored character profiles. Design: [enemy patterns](game/enemy-patterns.md#not-yet).

**Done 2026-11-10:** skirmishers have the first-sight beat
(`SkirmisherCfg::notice_s`, read from the profile's `tactics.notice_s`, through
the shared `Brain::is_noticing` projection). The dilophosaurus is authored
(0.4 s and its notice cue).

**Audit 2026-11-10 (pirates and ninjas):** nothing to author yet. The oni
leader is the one authored (0.5 s, `enemy.oni_leader.notice`). Every other
pirate and ninja has no `notice` sheet row and no `.notice` cue recipe, so a
`notice_s` would be a pause with no pose and no sound, which the acceptance
below does not allow. Their brains: pirate strikers, brutes and the cutlass
viper run the `Smash` template (no notice beat; it is the match-seat
controller); the rangers, the raider and Iron Mary run `Skirmisher` (notice
ready, data missing); the shadow duelists and heavies run the catalog presets
`melee_brute_striker` / `melee_brute_brute` (`BrainPreset` carries no notice
field; it is pinned to 0). None uses fighter engagement.

**Next action:** when a pirate or ninja sheet gains a `notice` row and its cue
recipe is published (a publishing machine), author `tactics.notice_s` on its
`Skirmisher` profile; for a `Smash` or catalog-preset enemy, move it to a
profile whose template already carries the beat (brute or skirmisher) rather
than adding a notice to `Smash` or `BrainPreset`.

**Acceptance:** first sight produces one authored pose and cue before attack;
loss and reacquisition follow the declared policy. Hurt interruption and
ranged volley timing still work. A body without notice data keeps its declared
policy. Use the same beat projection for every controller.

### WORLD-LAYERS — fold the remaining long dimensions into connected regions

**Failure:** the current Jungle world is 25792 by 1888 pixels; Magical is
17408 by 2688. Both still have long horizontal layouts. Travel portal kinds,
reach checks and same-dimension rules already landed in `c818c0801`.

**Owner:** `tools/ambition_ldtk_tools/specs` generators and
`game/ambition_map_assets`. Design: [world layers](game/world-layers-and-travel.md),
[region composition](game/world-composition.md),
[remaining room detail](game/world-detail-24h.md),
[Art dimension follow-up](game/art-dimension.md#ideas-for-later), and
[History dimension follow-up](game/history-dimension.md#the-road-ahead).

**Next action:** fold Jungle into surface, underground and canopy regions,
then add Magical's sky and local arch network. Reuse the existing portal and
gate machinery. Keep authored rooms editable in LDtk. Recheck old room-detail
counts before selecting missing props, secrets or region links. Then recheck
Art's remaining publication, cues and places against the live assets. Keep its
completed first region.

**Acceptance:** the flat map matches room placement; neighboring regions have
loops and distinct vertical connections. Real movement reaches their exits,
shortcuts and arches. Hidden rooms and permanent breaks survive save/load.
Review the semantic LDtk diff and capture the changed places. Do not add a
second travel implementation or turn the target aspect ratio into a test.

### LEVEL-TRUTH — authored level data in the map submodule, one owner per room

Jon, 2026-11-04: "we should avoid putting the specs for levels and dimensions
in the main repo... I also want to avoid the issue of having more than one
source of truth for level data... we should likely be putting the authored
content into that submodule."

**Done:** a generator no longer overwrites a room edited in LDtk
(`<world>.generated.json`, `level ownership status|adopt|release`,
[region composition](game/world-composition.md#contents-one-owner-per-room)).
The History generator moved to
`game/ambition_map_assets/ambition_content/generators/history/`, and its
area specs are no longer committed.

**Failure:** every other generator (`tools/ambition_ldtk_tools/specs/*/make_specs.py`:
art, castle, eureka, evil moor, magical, ninja village, pirate port,
primeval, gate network and the rest) and its committed area specs are still
in the main repo, about 2.5 MB with 238 commits of history. Their worlds have
no record yet, so the guard is inert for them until their next apply.

**Survey (2026-11-10):** `level ownership survey <spec>... --ldtk <world>`
applies a generator's specs to a copy of its world and says, per room,
whether the live room is still what they make (`reproduced`) or was changed
after (`DIFFERS`). History's recorded generator reproduces its rooms (12 of
12 sampled): the control. It replays `area create` only, not a generator's
in-place steps after it (painting, `entity add`, `scenery_overlay.py`), so a
DIFFERS may be those steps. Treat it as LDtk-owned: that keeps the room as it
is, and costs the generator only a room it might have rebuilt.

**Jungle done (2026-11-10):** all twelve `dinosaur.ldtk` rooms are LDtk-owned
(`dinosaur.generated.json`). The six trail rooms and three of the six later
ones DIFFER from their specs, as the primeval generator's own note said (the
rooms were tuned in LDtk after it). Its scripts (`make_specs.py`,
`side_exits.py`, `treetops.py`, `tar_pits.py`) were deleted; the last version
is at `a910d3952e`.

**First survey of the rest (bare specs):** art 12 of 15 reproduced; castle
Godwin 3 of 6; castle kitchens 3 of 5; the breakable wall lab 0 of 2. The
loose top-level specs and edits are one-offs, but some are still read:
`hall_of_characters_area.ron` (`generate_hall_of_characters.py`),
`symmetry_room_area.ron` (`gen_symmetry_room.py`) and
`intro_raid_corridor_area.ron` (`test_spec_brains_resolve.py`).

**Every room has an owner (2026-11-10):** `generator_survey <generator>`
runs a whole generator (its `--apply`, with its committed specs and without
the ownership records) on the live worlds, fingerprints every room before and
after, and puts the worlds back from git; `--settle` records the rooms it
left as they were and adopts the rest. Run over the 14 room-making
generators: 103 rooms reproduced and are recorded as generated; every room
that DIFFERS (edited after its generator), that no generator made (the
sandbox's 80, intro, noodle) or whose generator cannot run (hall of biomes:
the sandbox already fails validation at `scroll_lab_door`, a LoadingZone with
no target) is LDtk-owned. `level ownership status` names an owner for every
room of every world but two History rooms, HAND-EDITED by e7ad541's
per-zone loading-zone presentation after History generated them (the guard
refuses to overwrite them).

**Generators moved (2026-11-10):** the 14 room-making generators live in
`game/ambition_map_assets/ambition_content/generators/<generator>/` beside
their worlds, as History's does; their area specs go to a temp folder and are
no longer committed; `district_kit`, `district_scenery` and the gate network
(`gates.py`, `make_gates.py`, which writes the three districts'
`gate_interface.yaml` into their generators' folders) stay in tools as shared
code. Each moved generator writes the same specs as before it moved (compared
with the lenient art box this VM needs: its art publication is missing, so a
generator here cannot `--apply`).

**One-offs deleted (2026-11-10):** 111 hand-written one-off specs and edits
(`*_area.ron`, `*.yaml`, `gnu_ton/`, `pirate_sky/`) are gone; every room they
made is LDtk-owned, and the code and docs that cited one for a number now
name the room. The dog example moved to `specs/examples/`.

**What stays in tools (2026-11-10):** besides shared code
(`district_kit`, `district_scenery`, `scenery_overlay`, `check_flat_map`,
the gate network) and the tool's examples: the hall of characters' spec and
order, written by the package's catalog-driven `generate_hall_of_characters.py`
(its room reproduces and is recorded as generated, so the hall can be made
again when the catalog grows; its test checks the committed spec is current);
`intro_raid_corridor_area.ron`, the fixture of `test_spec_brains_resolve.py`;
and the portal labs' specs (the portal peer's). The symmetry room DIFFERS from
its spec and is LDtk-owned; `gen_symmetry_room.py` and its spec were deleted
(last at `83b5cdfa78`).

**Next action:** none required by the acceptance below beyond the portal
peer's specs; a generator's next `--apply` on a machine with the art
publication is the check that each moved generator still makes its rooms.

**Acceptance:** `tools/ambition_ldtk_tools/specs/` holds only shared code and
checks; `level ownership status` names an owner for every room of every world.

### POSE-CONTINUITY — one visual transition path after the completed base poses

**State:** the foundational Smash art and wiring gates are complete. Phase 3
continuity and campaign-only coverage remain open.

**Owner:** sprite-sheet animation and rig presentation, with renderer-authored
visual bindings. Design: [expressive poses, Phase 3](demos/smash-expressive-fighter-poses.md#phase-3---improve-continuity-and-body-weight-polish).

**Next action:** measure clip transitions on one reference fighter. Establish
stable visual bindings only where that experiment needs them, then implement
compatible transitions and planted contacts on the shared presentation path.
Measure campaign sheet gaps before authoring more rows.

**Acceptance:** human and CPU bodies share the path; contact timing and sharp
attack poses remain readable. Check interruptions, rollback resets, quality
tiers and portals at game scale. Preserve Mary-O's classic home-game tall
poses. Visual rig work does not enable gameplay rig geometry.

### LOCAL-PLAY-MODES — two local modes, explicit join and menu leave

**Owner:** `ambition_input` (seating, `sources.rs`), Ambition's seating
declaration (`ambition_content::provider::declare_ambition_seating`), the join
road (`session/join.rs`) and the settings owner
([multiplayer](game/multiplayer.md), ruling Q153 2026-10-19).

**Built 2026-10-19 (slices 1 and 2):** Ambition's route decides a channel
plan, so devices never size the session (`ambition_route_seating`).
Single-player is `[Shared]`: the keyboard and every unclaimed pad drive player
one, the pad in use, also a pad connected late. The gameplay setting
`accept_local_joins` makes it `[Shared, Claimed]`: the second handle is
reserved from the start and idle until a pad that player one is not using
presses Start (`ambition_input::claim_local_seats`, `LocalSeatClaims`). The
claimed pad drives seat 1 only. The simulation joins a seat with no body on its
Start press (`session/join.rs`); Jump is not the gesture. A change of plan
rebuilds the local rollback session, with no restart. A standing claim keeps
its channel when the setting goes off. Witnesses: `local_play_modes.rs`,
`a_second_seat_joins_the_session.rs` (the rewound join; Jump builds nothing),
`a_joined_seat_has_a_view_and_a_hud.rs` (a real pad's Start, with two pads,
keyboard plus one pad, and a pad connected late; control: setting off),
`provider::tests::ambition_is_single_player_until_the_setting_accepts_a_join`.

**Built 2026-10-24 (first integration correction,
[local-coop integration](engine/local-coop-integration.md)):** the shipped
host builds a camera for each view the adaptive split opens
(`SplitViewRigsPlugin`, one capability for every host), and every player body's
picture follows its own pose and live room (`sync_visuals`). Witnesses:
`local_play_modes.rs` (left stick through the raw axis event; the split
camera's view, layers, screen effects and its own half of the display),
`room_placement_tests::each_player_visual_follows_its_own_body_in_its_live_room`.
The maintainer reports the co-op baseline now works in the main game
with two players (2026-10-24); see `MULTIPLAYER-SEAMS` for new playtest defects.

**Built 2026-10-28 (join placement, `1f0956abb`):** the common body recipe
now joins at a clear nearby position in the primary body's live room.
The search has a fixed limit and declines a blocked join. See the guards in
[local-coop integration](engine/local-coop-integration.md).

**Built 2026-11-10 (slice 3, Leave):** each seat's System menu offers
"Leave Game" to a joined local seat and "Remove Player N" to the primary seat
for each joined local seat (`SystemMenuModel::with_seat_departure`, attached
by `menu::departure::MenuDepartures`; only in a session that joins home
bodies). Choosing it writes `ParticipantLeaveRequested` through
`HostIntentWriter`, releases the seat's pad claim and closes the menu. The
simulation (`session::leave::retire_leaving_participants`, before the join on
the stamped tick) ends the membership, despawns the seat's player bodies, and
puts what each holds down where it stood (`ambition_held_items::put_down`; the
bag is session-wide). The primary seat cannot leave this way; Quit to Title is
its way out. Its menu, view and HUD row go with the membership and the body;
no room is reset; Start joins again. Witnesses: `a_seat_leaves_the_session.rs`
(the object lies once where the seat stood, also next door where its view
closes; the primary's leave is refused; a second join; the leave and rejoin
under a two-seat sync test),
`lunex_kaleidoscope_app_tests::remove_player_writes_a_stamped_leave_and_frees_the_pad`,
`system::tests::departure_rows_are_attached_per_seat_above_quit_to_title`.
The menu already answered to the seat that opened it (`SeatMenuFrames`, one
menu per member). Open: a remote peer's leave (this is local seats only).

**Slice 4 (presentation) is met by existing witnesses (checked 2026-11-10):**
the join opens the two-player presentation (seat 1's own HUD on the shared
view, `a_joined_seat_has_its_own_hud_on_the_shared_view`, drawn in the
rendered host by `a_joined_seat_has_a_drawn_hud_in_the_rendered_host`); its own
view opens when the two are apart and the adaptive merge closes it when they
regroup (`a_joined_seat_gets_its_own_view_when_the_primary_leaves_its_room`,
`a_second_view_opens_when_the_players_drift_apart_in_one_room_and_closes_when_they_regroup`);
the leave brings the single-player presentation back (`a_seat_leaves_the_session`).

**Next action:** none required. Nice to have: a portal that Player 2 drops
out of, presentation only. Open: a remote peer's leave.

**Acceptance:** the controller and multiplayer tests of the 2026-10-19
assignment (section 11), each a simulation-level witness with a negative
control.

### MULTIPLAYER-SEAMS — make local multiplayer safe for netplay

**Owner:** [local-coop integration](engine/local-coop-integration.md) and
[peer-room-commit protocol](engine/peer-room-commit-protocol.md), across
input, dialogue, view/UI, combat, room lifecycle, and rollback owners.

**State (2026-11-04):** the local isolation slices are built: participant-owned
dialogue, viewport routing, blink clocks, fixed/adaptive split, friendly fire,
shot allegiance, damage-box scope, and per-view diagnostics. Ordinary online
peer crossings now commit on the shared input timeline **without a global hold
or rebase**. Delayed-peer, corrected-combat, concurrent-loading, arrival-reset,
rendered combat, and input-timing witnesses pass (latest `dc303e611`).

**Verified 2026-11-10:** every gate 1-5 guard passes on the current tree,
GPU captures included; the run found and fixed a panic when a camera retired
in the frame Bevy's render debug overlay first saw it
([verification](engine/peer-room-commit-protocol.md#verification-2026-11-10)).
On 2026-11-11 the feature run found that one peer's crossing closed the
other peer's conversation; fixed
([dialogue](engine/peer-room-commit-protocol.md#dialogue-during-another-peers-crossing-2026-11-11)).
On 2026-11-12 a participant's menu was witnessed open and answering its pad
through another participant's crossing, in both orders
([menu](engine/peer-room-commit-protocol.md#a-menu-during-another-participants-crossing-2026-11-12)).

**Next action:** complete the broader **feature and tooling verification** for
input-driven peer room commits. Reproduce a normal crossing and a failed or
invalidated preparation with two peers; audit exact content/plan identity,
participant-specific interactions, replay, view/UI ownership, input ordering,
and cleanup. Keep full-party restores/rebuilds on their distinct global
barrier. Do not replace the native room constructor or reinstate a hold for
ordinary travel. The [protocol owner](engine/peer-room-commit-protocol.md)
contains the case-by-case gates and prior witness names.

**Acceptance:** both participants can continue independent movement, combat,
dialogue, menu and view presentation while one crosses/loads; two delayed
peers agree on application frames and outcomes across rollback, and refuse
mismatched or changed preparation without corrupting source rooms. Run the
focused harnesses and a rendered two-controller main-game playthrough.

**Blocked by:** none.

**Separate work:** menu leave is LOCAL-PLAY-MODES; participant recovery is
COOP-RECOVERY and PARTNER-RECOVERY. Do not duplicate their lifecycle rules here.

### Music playback gaps — diagnosis ready; death gap cause open

**Owner:** [music playback diagnostics](../systems/music-playback-diagnostics.md)
and the game/audio integration path.

**State:** request-versus-Kira instance observations and audio-context warnings
exist (`534a92549`, `b170a9238`). Dinosaur treetops and tar pits adaptive
binding gaps were fixed (`0f5b40487`). The death/respawn gap has **not** been
explained by these repairs.

**Next action:** reproduce a visible death/respawn music gap with the
instrumented run; correlate request, selected source, decoder, Kira instance,
audio context and speaker-audible playback. Separate intentional silence,
suspension, mute and one-shot completion from defects.

**Acceptance:** a reproduced unintended death/respawn gap is repaired with a
regression witness, or the observed event is classified with concrete playback
evidence. Keep diagnostics independent of playback repair.

### GATE-REDIAL — a new address closes the old wormhole and opens a new one (done)

**Owner:** the gate network (`docs/planning/game/gate-network.md`, the gate
state machine and its membrane presentation); portals owner. Jon 2026-11-12:
changing a gate's address while it is open only recolours the open
wormhole. A redial is two events: the open wormhole closes (its closing
beat), then the gate dials and opens a wormhole to the new address (its
opening beat), with the connection to the new destination only once open.

**Acceptance:** with a gate open to A, dialling B plays the close then the
open, the old destination stops being reachable at the close, B is
reachable only after the open, and a body in the membrane during the change
is not carried to B.

**Built 2026-11-12:** a redial's link waits (`GateLink::opens_after_close`)
until both its rings are Off; dialling the place a ring already leads to
changes nothing. Witness `a_redial_closes_the_open_wormhole_and_opens_a_new_one`.

### GATE-STARTUP-FLICKER — diagnose the wormhole at startup

**Owner:** [Gate network](game/gate-network.md#startup-rendering).
**State:** OPEN. Jon reports flicker on desktop and Steam Deck. Warmed High and
Ultra captures show an opening flare and continuous steady rendering. They do
not prove that cold startup is correct.
**Found and fixed 2026-11-11 ([startup rendering](game/gate-network.md#startup-rendering)):**
each tier landing at startup rebound the membrane and ring with a new
animator, unordered against the gate's own systems: one frame of the idle
cell, then the stable loop restarted at frame 0 (measured with a probe). The
rebind keeps the cursor and the gate's systems run after it.
**Next action:** a cold-start capture on desktop and the Deck to see whether
any flicker remains (the shader layer is unchecked).
**Acceptance:** the gate renders continuously after its opening on desktop and
Steam Deck. Keep intended opening animation separate from missing frames.

### CHARACTER-FORM — a change of form is the same body in the new form's geometry

**Owner:** the form application (`avatar/character_execution.rs`, the re-wear
road), `ambition_body_seed::PhysicalBaseline` / `BaselineBoundary`, the
construction paths (home body, NPC placement, summons, match fighters, joined
seats), and control projection (`control/authority.rs`). Ruling: Q159
(2026-10-19). Plans:
[controlled-character actor kernel](engine/controlled-character-actor-kernel.md),
[control authority](engine/control-authority-and-ai-policy.md).

**Current failure (measured 2026-10-06, Q159's table):** the player (a 30 by
48 box) wearing the companion dog is drawn 36 by 30 in the player's box,
while the dog NPC is built at 108.6 by 90.6 (the NPC's drawn quad; its
collider follows `standing_height` 64). `BaselineBoundary::Replacement`
applies no geometry.

**Read 2026-10-19 (the construction roads):** actors size through one
resolver, `BuiltGeometry::resolve` (`ambition_body_seed`): the posed sheet's
Idle box (`SpriteAuthored`), else the sheet's `body_metrics` scaled by
`standing_height`, else the placement box. The contradictions:

1. `BodySource::Explicit` sizes the home body, is only the placement box of a
   match seat (a sheet overrides it), and is ignored by the NPC, minion and
   enemy roads. No shipped character authors it (the catalog can express only
   `posed_body`).
2. A character with no body source (the dog, `npc_kernel_guide`, the
   pirates) is sized from its sheet as an NPC, and is 30 by 48 as a player,
   at the start and after a re-wear: the home body never reads the sheet.
3. At a re-wear, the prepared-body grant
   (`actor_spawn::character_body::grant_prepared_character_body`) resizes a
   `SpriteAuthored` body and keeps the old box for the others; their art is
   fitted to the kept box, and the hurtbox fallback and attack volumes follow
   that box, so they are undersized.
4. Two triggers: the gameplay half of a re-wear (`apply_worn_character_gameplay`)
   needs `RecharacterizeBody`, and the projection runs on
   `Changed<WornCharacter>`.
5. A re-wear never writes `AbilityBase`, so the new form's abilities do not
   arrive.
6. Keep worn form, home identity, driving authority, and custody separate.
   A form change must not replace the participant's driving or custody claims.
   Possession records and handoffs now apply to each participant seat.

Tests that pin the old behaviour and change with slice 1:
`a_body_drawn_from_its_box_states_the_quad_of_the_character_it_wears`
(`character_runtime/presentation/tests.rs`, "a re-wear keeps the box") and
`a_reworn_body_states_the_quad_it_is_drawn_at` (app tests). The doc of
`match_activation`'s `Explicit` seating test calls a projection that resizes a
live body "a second geometry authority"; the ruling makes the form's
configuration the input and the live body the authority.

**Built 2026-10-19 (slice 1, 86ae35083):** the prepared-body grant sizes a
form with no posed body from its sheet at its standing height
(`catalog_join::sprite_body_at_standing_height`, the NPC road's numbers), at
construction and at each change of form, and retracts it; damage stays; a home
body's base abilities are `HomeBodyAbilities::base_for` at build and at a
change of form. Witness `a_player_that_changes_form_takes_the_size_of_that_form`
(guide, dog, robot, guide; poison red at the guide's 30 by 48 against a built
40.4 by 65.8). Contradictions 2, 3 and 5 above are closed for forms with a
standing height. Review finding #8 (2026-10-19): the grant no longer sizes an
`Explicit` body from its sheet (`sheet_body_for`); it keeps its box, as before
slice 1. Explicit half-extents on a change of form are still slice 1b.

**Built 2026-11-12 (slice 1b):** `BodySource::Explicit` is the box its author
wrote on every road: the shared resolver (`BuiltGeometry::resolve`, the NPC,
minion, enemy and match-seat roads) and a change of form
(`form_body_for` in `actor_spawn::character_body`) take it before the sheet,
and the sheet draws it fitted to that box (`fitted_render_size`). Before, a
sheet with a standing height sized an explicit NPC or seat over its box, and a
change of form into an explicit character kept the old box. Witnesses:
`an_explicit_body_is_built_at_its_box_not_its_sheets` (body seed) and
`an_explicit_body_is_its_own_box_not_its_sheets` (actor spawn). A form with
neither an explicit box nor a standing height keeps the box its road gives
(the placement box for an NPC, the body's own box on a change of form) and is
drawn fitted to it: that is the one answer, since a live body has no placement
box to scale by. Contradiction 1 is closed.

**Measured and built 2026-11-12 (slice 2):** a census built each of seven
forms as an NPC (a placement), a summon (`SummonRequest`) and a worn form on
the player, in one room. Five agreed to the bit on all three roads (the dog,
the guide, the robot, the burning shark; collision, standing box and quad).
The puppy slug did not: 53.8 by 14.1 as an NPC, 40.3 by 10.6 as a summon and
the player's 30 by 48 worn, because a form with no standing height is scaled
by whatever box its road gives (`sprite_body_collision_for_sheet`'s
placement fallback). Five catalog rows had no intrinsic size; four now state
one, the height most of their placements already built them at (the puppy
slug 21.2, the stochastic parrot 38, Mary-O's cartesian and paper plane
snakes 28.2 and 30.2), and all seven forms agree on every road. The fifth,
`npc_gnu_ton_boss`, is built by the boss road (`resolve_sheet_body`, from the
boss catalog), which reads no character standing height: one form, two
sizing roads, open for GNUTON-GNU2-FINALE's summoned GNU.

**Next action, in slices:** (3) Distinct operations over
shared infrastructure: transformation (same body, new form), possession
(another existing body, same participant), skin (presentation only), body mode
(same body and form). Investigate the rename `WornCharacter` →
`CharacterForm` after the semantics are right.

**Acceptance:** the robot becomes the dog: the body is the dog's built size,
with the dog's moves and abilities, the same entity and identity, and its
damage kept; back to the robot restores the robot's geometry. A skin change
moves no collider, hurtbox or movement value. A possession leaves the original
body in the world, and the possessed body keeps its geometry and state.

### INTRO-RAID — one creator, a raid by hired goblins, a fight side by side

**Owner:** the intro rooms (`intro.ldtk`), `intro.yarn`, the intro route
(`ambition_content::intro`), Oiler's and the message board's rows. Jon
2026-11-12 (next after the COOP-RECOVERY slice in hand):

- **A cutscene holds the fight too.** Entering the second room forces a
  dialogue while the goblins keep attacking the player.
- **One creator.** The creator is spawned twice (one in each room) and does
  not die. He is one person with a persistent identity who follows the player
  from the opening room into the second.
- **The raid's motive:** goblins hired to raid the lab and take the robot and
  the research. In the second room they confront the two; after the opening
  dialogue the robot and the creator fight the goblins together.
- **Oiler** falls into the hole without noticing anyone. Getting close to him
  is the trigger (not entering the room), and he comes to the player.
- **The message board** can be attacked and fights back: an Easter egg to
  keep, with fight animations of its own.

**Built (2026-11-12):**

- *The hold* (ce2e093812): a playing room cutscene holds every body in its
  room (`ControlHold::Cutscene`, `Invulnerability::CUTSCENE`).
- *One creator, a fight together:* `intro_raid_corridor` is one room with the
  lab (`activeArea: intro_wake_room`; stairs where the two floors meet). The
  one creator keeps at the player's side (`follows_closely`, a `Roam`
  preset), and the corridor is `dangerous`, so he rallies to the robot's
  fight. The raid starts at a `CutsceneTrigger` volume at the head of the
  corridor (a new LDtk entity: a scene that starts where its people are),
  with the hired-goblins lines. A navigating body that stands on a slope
  plans from the surface the slope rises off (it waited on the cart ramp
  for a landing that had happened).

**Next:**

1. *The creator's end (built 2026-11-12):* his last words
   (`creator_final_fragment`) play at a `CutsceneTrigger` by the corridor's
   far end and end on a new `Fall` beat (a scripted death in the cutscene's
   room). A skip now keeps a script's flags and falls (it passes over the
   show, not the story). Decided: he always dies here; if he fell in the
   fight already, the words are the ones he had left. The fast and
   impossible variants in `intro.yarn` are not wired.
2. *The message board's fight rows (built 2026-11-12):* `slash`, `hit` and
   `death` on `news_board_v2` (renderer e3549b3); it leans on its feet.

*Oiler (built 2026-11-12):* he stands past the grate (x 720) with
`meets_the_player` (a `Roam` with the new `greet_within`: out of its rest
to meet a player within 260 px; it keeps to the graph, so no more walking
into the hole), and his line is `oiler_notices`, started by a
`CutsceneTrigger` around his post. The room's arrival keeps its fade and
banner.

### FINAL-BOSS — the bossofthegame on the shoulders of a giant

**Owner:** the final-boss subagent (Jon, 2026-11-12: "Put a dedicated
subagent on this"); design page `game/final-boss.md`. Request (Jon,
2026-11-12): the bossofthegame fight mirrors GNU-ton's, in that he stands on
the shoulders of giants. The giant is a square robot with an electric guitar.
It is the final boss of the game, so it is grand. A gate address goes directly
to the boss dimension. Brainstorm notes: `docs/brainstorms/original_brainstorm.md`
(the 4th-wall "vibe kill", typed characters falling on a climbing player, an
agent carrying the Boss on its shoulders) and
`docs/brainstorms/good_llm_ideas/creator_agents_and_endgame.md` (the Boss
acquiesces rather than dies; "until the day he dies, or retires").

**Design:** [`game/final-boss.md`](game/final-boss.md) (2026-11-12): the
Boss rides **Full Stack**, a giant amp-stack robot with a guitar; the
dimension `boss` (`boss.ldtk`: lobby, main stage, green room); the hidden
gate address **LEGACY**; phases "Make No Mistakes", "Manual Override", "The
Encore"; the end is his retirement, not his death. Q175 (the giant's
identity) and Q176 (is a resolve bar a voluntary retirement) hold defaults.

**Built 2026-11-12:** the dimension (`world.boss_ldtk`, map
`generators/boss_stage/make_specs.py`): `boss_lobby` with the ring
`boss_lobby_gate` (LEGACY, hidden), `boss_main_stage` (risers and truss,
the green-room grille on `boss.cleared boss_stage.bossofthegame`),
`boss_green_room`.

*The fight (built 2026-11-12):* the art (renderer targets
`bossofthegame_boss`, `full_stack_giant`, `boss_keycaps`), the catalog rows
`npc_full_stack` (a `giant` mount: its top and its guitar are ground) and
`npc_bossofthegame` (his voice), the encounter `bossofthegame` (60 resolve,
three phases, `corpse: Stay`: he sits down), the module
`ambition_content_modules::bossofthegame` (power chord, riff, feedback,
keystroke rain, backspace, subagents), the cutscenes `boss_vibe_kill` (the
stage's entry) and `boss_retirement` (the green room), the music fallback
(GNU-ton's adaptive cue) and the hall's practice copy
`hall_bossofthegame_stage`. Witness: `the_final_boss_stands_on_full_stack`.
The module's conductor draws Full Stack in its `strum`, `riff` and
`feedback` rows on those moves, its `idle` strum between.

*Built 2026-11-13:* the stage's camera zone (zoom 2.6, about 790 units
high: floor, risers, Full Stack and the Boss on one screen); the Boss is the
Director with a keyboard (Jon: "the director already existed"); the
dimension's own biome `boss_stage` (stadium backdrop, crowd foreground,
stage skin, decor, door, confetti); Full Stack's 1960s robot head and
straight strings; the three named agents Geppetto (called as phase 2 opens),
Clause and Wren (as the encore opens), drawn from the concept sheets on the
character generator, so each has the eight standard angles and the fighter
rows (Wren in a hood with a wrench; Q175 updated); the band (design page):
a guitar, bass, keytar and drumsticks every agent can play (`band_*` rows),
and a drum kit prop. The encore's Agent Swarm subagents, refused until then
as an unbuilt character, now drop in.

*Built 2026-11-13:* the band in the fight. As the show opens the Boss's
conductor summons Geppetto on the bass (west truss), Wren on the rhythm
guitar and Clause at the drum kit (east truss), each drawn in the
instrument's row. A call sends the called agent's band body away (a new boss
request, `ambition.boss.dismiss_summon`) as the fighter drops in. Witness:
`his_named_agents_join_the_fight` (rows at the open; Geppetto off at phase
2; the others off at the encore). A capture of the practice stage shows the
three at their places.

*Built 2026-11-13:* the Boss's microphone. It is in his far hand (the pen
stays in the near one), with four rows at it: `mic_sing`, `mic_point`,
`mic_call` and `mic_encore` (renderer b63728c). It also fixed a fairness
finding. Every tell of his but the rain and the backspace was `spike_halo`,
so nothing on screen told six attacks apart (`boss_fight_validator`; his
eight moves were also in no seed). Now each tells by its own row: the
chord by the pen raised, the riff by singing, the feedback by the mic
pointed out, the subagents by `talk`, Geppetto's call by the mic held up,
and the encore by both arms up. The moves are classified in
`boss_seeds.ron`, with three bands widened to what they measure. The
validator's pin holds at 3 errors and 5 warnings, the other bosses'.
Full Stack has two more band rows: `solo` while he holds his beat between
phases, and `outro` once he is down. Jon (2026-11-13): the stage camera
was far too wide, so its zoom is now 1.4 (about 430 units high), down from
2.6. The Boss holds no pen: it is the Director's sword in Smash (renderer
302b54c, map 8743626).

**Next action:** play it in a window and tune; his own anthem when a music registry is
at hand; the later-work list in the design page.

**Acceptance:** a ring dialed by its address reaches the boss dimension; the
fight runs with the Boss riding the guitar robot, in phases, with its intro,
music cue and art; the ending follows the retirement rule.

### PLAYER-DEATH — what a death is in Ambition

**Owner:** `session/death.rs` (the beat), the presentation owner, and the bag
(currency). Request (Jon, 2026-11-12): Ambition needs a presented player
death (Mary-O has one) and a rule for what a death costs: a Hollow Knight
style penalty undone by going back to where you fell, with a twist; or a
portal that yoinks you away before you bite it ("You're too valuable to be
destroyed"); or a lost currency you can win back, with a bigger penalty when
you cannot pay the safety fee. Q174 holds the rule; its default is the rescue
portal with a fee.

**Built 2026-11-12 (slice 1, the beat drawn):** Ambition's rooms hold a
death for 1 s (`replay_level_after_death_s`, was 0), so the robot's death row
plays, and declare a `death_look` (a new scoped rule, `DeathLook`): the
effect drawn where the body is taken when its window opens
(`digital_departure`, code strands dissolving) and where it comes back
(`digital_arrival`), by `runtime::death_look`. Measured with a temporary
probe in the hub: the departure on the death's tick at the body, the arrival
64 ticks later where it is back in play.

**Built 2026-11-13 (slice 2, the fee):** a scoped rule `DeathFee`
(`room_rules.ron`: `death_fee: Some((coins: 50))`), and
`runtime::death_fee`: a death owes the fee, and it is taken when the body is
back in play (a level that goes back puts the wallet back to its checkpoint,
which would undo a fee taken at the death). The coins taken lie where the
body was taken (its last safe ground) as a pickup drawn from the `DeathFees`
ledger (rollback state, schema 465): placed while its room is live, again
after the room goes back, off the ledger when fetched; one parcel a
participant, so a second death before the fetch loses the first. A wallet
that holds less gives all it has and goes into debt for the rest
(`BodyWallet::charge`: a negative balance, at most one fee; the HUD reads
`-$30`); the next coins earned pay the debt first. A fetched parcel is not a
reward the restore keeps (`ReturnedCoins`), or the next death would pay its
coins twice. Measured with a temporary probe in the hub: 120 → 70 at the
death, the parcel of 50 where the body was, 120 when fetched, 70 again at
the next death; with 20: −30 and a parcel of 20, −10 when fetched, −30 at
the next death.

**Built 2026-11-13 (slice 3, the rescue drawn):** the death look is the gate
network's own door (`rescue_portal_fx`, a sheet that draws its own start and
end): `rescue_portal_take` opens round the body, pulls it in and shuts, and
the body is not drawn from 0.42 s (`DeathLook::taken_after_s`, marked
`TakenByDeathLook` by the view, which `body_presentation_hidden` honours);
`rescue_portal_give` lets it out where it comes back. The fee's coins lie in
a sealed parcel (`rescue_parcel_prop`, `DeathFee::parcel_art`, a prop-sheet
row). Filmed in the hub (`capture_scene --player-dies 600 --purse 120`): the
door takes the robot and the spot is empty, the robot steps out of the
light at the respawn, the parcel lies where it fell.

**Built 2026-11-13 (slice 4, heard and said):** the door's sounds are the
wormhole's (`vfx.rescue_portal.rescue_portal_take`: the wormhole's swallow
and the door shutting behind the body; `_give`: its release and a two-note
chime), and the purse says what the death cost: a purse that drops shows
the drop beside it for 2.5 s (`$70  -50`), and one in debt reads owed
(`-$30 owed`). Not a line over the body: the body comes back at its room's
spawn, and in the hub the door plates and signs leave a speech line (the
last label family) no place, so the layout hides it; and the gameplay
banner is drawn only in the developer inspector. Filmed in the hub
(`capture_scene --player-dies 600 --purse 20 --include-ui --tick-per-frame`;
`--purse` now holds the wallet only until the death).

**Next action:** none. The acceptance's co-op half holds with the fee in
force: the 13 `a_second_seat_joins_the_session` witnesses (recovery, the
wipe, a departed seat, R5) pass in Ambition's rooms, which declare it.

**Acceptance:** a death in Ambition reads as one on screen; the fee is taken
and lies where the body fell until fetched; with too little to pay, the
bigger penalty applies; co-op recovery and the wipe keep their rules.

### HIT-FEEL — hits that answer

**Owner:** combat feel (`ambition_combat::feel`, kill beat, hit-stop), the
sound and effect recipes, and each move's poise. Request (Jon, 2026-11-12):
hits must feel more responsive: better hit sounds, a little recoil on the
attacker, better hit effects, and enemies with more knockback or super armor
depending on their move.

**Measured 2026-11-12** (a temporary probe: the robot's first slash on a
passive goblin, goblin brute and exploding mite in the hub): both bodies
froze 0.036 s (the engine's hitlag law is a share of a platform fighter's
reference, 11 damage, and Ambition's hits deal 1 to 3, so every hit got its
floor); no camera shake (the shake is the freeze above the weakest); no
attacker recoil; knockback (47, -34) px/s, 7 px of travel, the same on the
brute as on the goblin (the slash authors a 58 px/s launch, and no catalog
row authors a weight); two sounds and a slash, a speech bubble and an
impact spark.

**Built 2026-11-12 (slice 1, the freeze and the recoil):** a `hit_feel` room
rule (`ambition_combat::hit_feel`, declared apart from `CombatRules` as the
landing-lag share is). Ambition's: hitlag `Affine(0.06 + 0.015 per damage,
max 0.15)`, so an ordinary hit holds both bodies 0.075 s and a heavy one
0.105 s (which shakes); and `attacker_recoil: 110`, a melee connect in the
air pushing the attacker back from what it struck (measured: 110 px/s held
after the freeze, 2 px a tick). Not on the ground: the run law brakes a
pushed speed to 0 on the next tick, and holding it would be the recoil
lock, which reads as hitstun.

**Built 2026-11-13 (slice 2a, armor):** an enemy's `Swipe` or `Lunge` melee
can say `armored: true`: super armor (`WindowTag::Armor`) from halfway
through the windup to the end of the strike, so a hit lands its damage and
not its launch; the first half of the windup and the recovery stay open. The
heavy bodies wear it: the goblin brute and bruiser (`brute_hammer`), the
heavy pirate, the triceratops, living statue, clay golem, aurochs, steam
automaton, hippo, mammoth, diprotodon and `brute_lunge`. Measured on a goblin
brute: a hit in the window took 1 HP at (0, 0) px/s and no hitstun, and the
swing went on; the next hit after it launched it at 108 px/s with 0.18 s of
hitstun. A `Slam` has no armor flag yet (four heavy sets use one).

**Built 2026-11-13 (slice 2b, knockback):** `hit_feel.knockback_scale`
(the `base_knockback_scale` of a game that declares no combat ruleset).
Ambition's is 2.0: eight ticks after the robot's slash a goblin moves at
(83, -19) px/s, against (36, 0), and travels 21 px, against 7. A goblin and
a brute are launched alike (no catalog row authors a weight); the brute's
difference is its armored swing.

**Built 2026-11-13 (slice 3, the contact and the armor seen):** an actor's
hit spark is drawn where the blow lands (`HitEvent::contact_on_box`): in the
overlap of the strike and the body, on the side the blow came from. It was
half way between the strike's centre and the body's, which a blade that
covers the body put in its middle. Measured on a hub goblin: the robot's
slash marks (1760.9, 949.0), 6.6 px inside its near face at the robot's
height; half way between the centres gave (1774.1, 943.1), its middle. An
open armor window is drawn: a steel sheen on the body's overlay
(`FeatureView::armored` from `MovePlayback::armor_now`, after the flashes and
before the i-frame blink), so a heavy move's armor is seen. Filmed on the
steam automaton in `history_industrial_age`: the body turns steel-blue
through its punch's windup and strike; at 0.30-0.55 it washed the art out,
so it shimmers 0.20-0.42. The sounds already answer the
weight: the robot's slash names the material selector, and a hit of 3 or more
(Ambition's heavy line) on flesh plays `flesh.deep` (430 ms, a thump falling
from 170 to 62 Hz) in place of `flesh.light`; on metal, `gong` in place of
`chink`.

**Built 2026-11-13 (slice 4, the slams armored):** a `Slam` melee can say
`armored: true` as a `Swipe` or `Lunge` can (`SlamSpec::armored`), and the
four heavy slams wear it: `war_drum_slam`, `terracotta_chop`,
`kraken_arm_slam`, `cave_bear_maul` (damage 3, windups 0.8-0.9 s). Probed
on the built move: an `Armor` window 0.4-1.0 s on a 0.8 s windup and a
0.2 s strike, none unarmored. The light slams (stagehand, scorpion man,
mummy, the club) stay open. Codec shape unchanged.

The heavy spark was already there, and the premise "no separate heavy art"
was stale: a connect on the room's heavy line draws `hit_hard` (since
6420ab194d, 2026-10-29) where a light one draws `hit_soft`, and larger with
strength. One rule (`StrikeWeightRules::is_heavy`, `combat::util`) picks
both it and the heavy sound.

**Next action:** none.

**Acceptance:** a landed hit has a distinct sound, a hit effect at the contact,
attacker recoil and hit-stop; each enemy move declares knockback or armor,
and a heavy move's armor is visible.

### APPLE-PORTALS — space apple trees and GNU-ton's catching portals

**Owner:** the GNU-ton encounter (`boss_encounters/`) and the portal owner.
Request (Jon, 2026-11-12, polish): space apple trees with apples on them;
for apple rain, GNU-ton summons portals on the ground that catch the falling
apples, which fall out of the portal's other end.

**Built.** In the apple rain he holds two catching pairs (`ch::catch`,
`hold_catching_pairs`): each is a mouth in the floor and one in the air,
one over where the player stood when the rain was called and one beside it,
clear of the giant. They open over the telegraph and close after the last
apples have fallen. A floor mouth is never under the giant, where the
scholar lands for eureka, or under an air mouth. Three space apple trees,
each on a rock moored to the ceiling, hang over the arena
(`shoulders_apple_tree_*`, `gnu_ton_shoulders` only; the Hall of Bosses'
copy `gnu_ton_arena` has no generator and is unchanged). Witness:
`gnu_ton_fight::an_apple_that_falls_into_a_floor_mouth_falls_out_over_the_player_and_hurts`.

**Acceptance (met):** an apple that falls into a ground portal comes out of
its partner with its momentum carried, and hurts as it did.

### MATH-POLISH — the hotel's spectral wing, the service lift, the reaper's wall and the mites

**Owner:** the math-polish subagent (Jon, 2026-11-12: "You can use up to 2
subagents"). Reports (Jon, 2026-11-12):

1. **Built.** In the hotel's spectral wing, background tiles of different
   widths do not line up, with seams; the torches' glowing dots do not
   repeat smoothly. Was: a prop is drawn at its art's shape, so a wall panel
   cut short drew a small panel in bare wall; the gap had panels at a third
   of the scale; the art did not tile; the motes drifted and jumped back
   each loop. Now 3:4 panels on whole quarters of tiling art, equal floor
   strips, looping motes (494e0f184f, with 2 and 3).
2. **Built.** The service lift has no visible barrier. Was: its lock wall
   drew the guest-room door exactly over the painted lift door. Now the
   lift's shut gate (`hylbert_lift_gate`).
3. **Built.** In the gram reaper fight a door sprite is used as a wall.
   Was: the treasury seal drew `hylbert_room_door`. Now the treasury's
   sealed grille (`hylbert_treasury_seal`).
4. **Built.** The dividing mite does not die when it divides. Was: the
   halves came at the kill, 30 px out, over a death row, a hold and a
   blink. Now its death row is the division and the halves stand where its
   lobes are when the row ends; the parent is gone (`Corpse::Gone`) and
   shows no death burst.
5. **Built.** The exploding mites go off only when they die. Was: in reach
   they planted a fuse and ended themselves, the blast came at the kill and
   the fuse-shaped death row played after it. Now the mite snaps, and a
   death's blast comes at the end of its death row (the fuse);
   `death_effects::run_death_row_finales`. Q177 holds the choice.

**Acceptance:** each report fixed and seen in a render or a probe; the mites'
behaviour has a witness.

### SMASH-PORTAL-RECOVERY — GNU-ton's portal up-b, and a better recovery (done)

**Owner:** portals (mine) and the Smash movesets. Request (Jon, 2026-11-12):
GNU-ton gets the portal up-b in Smash; Alice may keep hers (two characters
may share a similar move); the portal up-b works better as a recovery.

**Built 2026-11-12:** measured with a probe on the live Smash stage (90 px
off the main platform's lip, facing it, falling, neutral stick): the pair
(exit 150 straight up, both mouths facing each other) caught the fighter in
a fall loop under the ledge from 0, 80, 160 and 240 px below the lip, and
she fell when it closed. A stick tilt turned both mouths and threw her back
out the way she came. Now the shaft leans toward her facing
(`lean_degrees`) and the mouths stay level, so one fall puts her up and
over: rise 190, lean 25 hangs her on the ledge from 0, 80 and 160 (240 is
too deep). Alice keeps the move; GNU-ton's up-b is the same pair
(`action_at_a_distance`, in place of `levity`). Witness:
`smash_it::the_portal_recovery_reaches_the_ledge`.

**Acceptance:** GNU-ton's up-b opens the portal pair; from below the stage
edge, a portal up-b returns the fighter to the ledge or the stage in the
cases measured, where it did not before.

### PLAYER-LIGHT — a wider, subtler player highlight (done)

**Owner:** the player light (render). Request (Jon, 2026-11-12): the player
lighting / highlight has a larger radius and is more subtle.

**Built 2026-11-12:** `ActorGlow::player_highlight` is radius 1.6 body
heights (was 0.85) at peak alpha 0.018 (was 0.042): the same light at half a
body height from the middle (0.013), now fading out over twice the distance
instead of ending near the outline.

### COOP-RECOVERY — independent respawn and a full-party wipe

**Owner:** `session/death.rs`, `sandbox_reset.rs`, the checkpoint restore
(`session/checkpoint.rs`). Rulings: Q151 amended and Q153 (2026-10-19, the
later row: independent multiplayer).

**Current state (the Q153 default, 2026-10-05):** a death sends back only the
primary's room, and only when nobody is in play there; a fallen seat 1 comes
back beside the primary; a fallen primary waits while a seat plays in its room.
The restore's subject is the primary body (`PrimaryPlayerOnly`), so the
operations are told apart by seat, not by the active set, and there is no
full-party-wipe condition as such. Ambition's death beat is 0 s
(`room_rules.ron`), so an individual return is immediate and a wipe needs two
deaths on one tick.

**Built 2026-11-11 (the wipe's restore):** a participant out of play is not
a survivor, so when every participant is out of play the checkpoint restore
spares no room and no participant (`resume_at_checkpoint_on_reset`). Measured
with a temporary probe: Bob takes the gun-sword in the hub after Alice's
checkpoint and carries it to `duel_arena`; both die on one tick. Before, Bob
was spared and kept it (held 1, ledger `InCustody`); after, it is back on its
pedestal (lying 1, no row), one copy, and both are back in play. The wipe
also now pins the facts (DIALOGUE-CHECKPOINT). With Ambition's 0 s beat a
wipe still needs two deaths on one tick (the declared delay is open).

**Built 2026-11-12 (slice 1: return alone, and the wait):** a participant
whose room a living participant holds comes back alone at that room's
respawn point (the checkpoint when it is in that room, else the room's
spawn), health full, with no restore; the partner is not touched
(`return_a_fallen_participant`, which replaces the return beside the
primary; the primary no longer waits for its room to empty). A death while
another participant plays waits `recovery_wait_s` (new `room_rules` field;
Ambition declares 2 s, a tunable) beyond the beat, so a partner that falls
in that time makes the wipe. Witnesses in `a_second_seat_joins_the_session`
(the seat back at the spawn with the primary walked away; the primary back
alone after the wait, its room not sent back, then sent back when it falls
alone; a fall inside the wait making a committed wipe restore) and the
restated `a_death_with_bob_holding_an_object_in_the_room_keeps_one_copy`
(carried out then Alice falls alone; both fall on one tick).

**Built 2026-11-12 (slice 2: local recovery):** a second seat that falls
alone in its room while another plays asks `RecoverAtCheckpoint(slot)`; the
restore runs with it as subject (`RestoreTo::Recover`), sparing every other
room and win, covering only its view. Any close with nobody in play is the
wipe. Witness `a_second_seat_that_dies_alone_in_another_room_recovers_in_its_own_room`.

**Built 2026-11-12 (slice 3: a participant that left):** the roster is
the live player bodies and a leave despawns the seat's, so a departed seat
holds no room. Measured with a temporary probe in the hub (frames from the
primary's death to a committed restore with the primary in play): no join 4;
join then leave 4; seat 1 dead and leaving inside its own window 4; seat 1
staying never (control: the primary comes back alone). One arm was wrong: a
primary that fell beside seat 1 kept the partner wait after seat 1 left, 124
frames. With nobody in play a window owes only its beat now
(`close_death_interlude` caps it at `interlude` in rooms that declare a
`recovery_wait`; a match's stock windows are untouched); after: 25.

**Built 2026-11-13 (R5: a local recovery duplicates no object):** the
witness `a_seat_that_recovers_alone_gives_back_what_it_took_since_the_checkpoint`
passes. Its block was the join: in `blink_run` the corridor is exactly the
robot's height (48), a resting body overlaps floor and ceiling by float
error (1.7e-5), and the spawn clearance's strict overlap called every place
beside the primary inside solid. The clearance now ignores 0.01 of contact
(`rooms::spawn::RESTING_SLACK`). Past the join, the witness counted every
ground item in every live room, and `portal_bridge` lays out its own: it
now names the item, and finds one copy, lying in seat 1's room.

**Next action:** none; the rows below are the design, for reference:

- **Individual recovery (the default):** the participant respawns by the
  single-player respawn-location policy, for itself. With nobody else active
  in its room, that is a local room recovery: its room goes back to its
  checkpoint and the other participants' rooms and wins are spared (the Q151
  restore, with the dying participant as its subject, not the primary). With
  a living participant in its room, the room does not go back; only the
  participant returns.
- **Full-party wipe:** every active participant is out of play; the
  checkpoint restore spares nothing.
- A participant that left is not counted. The wipe reads death state now,
  not past deaths.
- **The wipe stays reachable:** the respawn waits a declared delay while
  another participant plays (a `room_rules` field); a partner that falls in
  that time makes the wipe.

**Acceptance:** same room, no room reset and the survivor untouched;
different rooms, room B and its wins unchanged; both dead, the checkpoint
restores; a removed participant not counted; a local recovery duplicates no
object.

### PARTNER-RECOVERY — an optional respawn beside a living partner

**Owner:** as COOP-RECOVERY, plus the settings owner. Ruling: Q153
(2026-10-19, the later row).

**Built 2026-11-12 (the policy):** the Gameplay setting "Respawn Beside
Partner" (`coop_respawn_beside_partner`, off by default) is frozen into the
next Ambition session as the session rule `ParticipantRecovery` (prepared
content identity section `world.participant-recovery`, beside the
friendly-fire rule). With it on, while anyone plays, a fall sends no room
back (`close_death_interlude`) and the fallen participant comes back beside
a partner (`return_a_fallen_participant`): at the partner's place when they
share a room, else by a crossing into the partner's room (`NearBody`), then
back in play there. With nobody in play it is the wipe, as before.
Witnesses in `a_second_seat_joins_the_session`
(`with_the_setting_a_fallen_seat_comes_back_beside_its_partner`,
`with_the_setting_a_primary_alone_in_its_room_comes_back_beside_its_partner_next_door`);
their controls are the default's witnesses in the same file.

**Measured 2026-11-13 (the presentation):** the partner's room already shows
the fallen arriving. PLAYER-DEATH's arrival (`death_look::draw_death_arrivals`,
the gate's `rescue_portal_give` door and its sound) is drawn where and when
`OutOfPlay` comes off, in that body's room, and partner recovery takes it
off only after the crossing has put the body beside the partner. Probed in
both witnesses (temporary print, deleted): the seat's arrival in the shared
room, and the primary's at (1524, 1928) in seat 1's room, on the frame the
witness finds it back in play within 96 px of seat 1.

**Next action:** none. Teleporting while alive is a separate possible
feature, not in this row.

### DIALOGUE-CHECKPOINT — story facts and their objects share one horizon

**Owner:** the checkpoint (`OwnedItemsBaseline`, Q124), the world fact store
(`world.set_flag`, quest steps) and the dialogue effects. Ruling: Q164
(2026-10-19).

**Current failure (measured 2026-10-09, `a_playthrough_of_the_persistent_world`):**
the player gives Alice's note to Bob, gets the survey, and dies before any
checkpoint: the survey leaves the bag (objects go back), and
`bob_field_survey_received`, the quest step and the wall it opened stay (facts
have no checkpoint copy). The object is lost for good while the world says it
was received.

**Built 2026-11-11 (slice 1):** the save's flags and quest steps have a
checkpoint (`session::fact_horizon`): a baseline taken at each checkpoint
commit and when a session's save is applied (a New Game's wipe takes the fresh
save's), pinned into a restore that spares no participant, put back in
`CheckpointDomainApply` with the quest registry rehydrated by its boot rule
(`QuestRegistry::rehydrate`), mirrored into the save the rebuilt room is
prepared from, and verified. Measured with a temporary probe in
`a_death_sends_the_blink_back_to_where_it_lay`: before, a death before any
checkpoint left (survey 0, note flag on, survey flag on, quest step 2, lock
open); after, (survey 0, both flags off, step 0, lock standing). Dialogue
visits are not restored (Q134). A co-op death that spares a participant pins
no facts, as before.

**Built 2026-11-13 (slice 2):** `<<checkpoint>>` in Yarn banks what the
conversation did. The command writes `CheckpointEstablishRequested` (the
talking seat) through the narrative ledger. `PendingAuthoredCheckpoint` holds
it, ungated, since a conversation holds the game. On the next tick of play
`commit_authored_checkpoint` records the seat's body's place and raises
`CheckpointCommitted` before the capture, so the one baseline takes the
flags, the bag and the place the same lines moved. Schema 466. The Kernel
Guide's test menu has the authored customer (`hub_guide__test_checkpoint`).
Measured with a temporary probe in the shipped shell session: choose it,
then die. The banked flag stays set and the potion stays in the bag (4). A
flag written after the bank (the control) is taken back. Which story beats
should bank is content's choice; Bob's hand-over does not.

**Built 2026-11-13 (slice 3):** a co-op death already kept the facts
(slice 1). Their objects did not stay with them. The restore built the bag
from the checkpoint's plus the kept pickup, chest and defeat grants, so what
a conversation gave or took went back while its flag stayed. That was read
from the restore, not measured. `ConversationBagSinceCheckpoint` now records
each conversation's bag move (grant, purchase, sale and the payer's coins).
A restore that keeps the facts replays the moves onto the bag and the
subject's purse. One that pins the facts drops them. A checkpoint commit
forgets them, and a room rewind drops its own (the bag side of a rewind stays
`BagUsesInRooms`'s). Schema 467. Measured with temporary probes (grant 2
Health Cells, sell 1 for 5, set a flag, die). Two players, Bob spared: flag
on, cells 3→4, purse 0→5, all kept. Solo control: flag off, cells 3, purse
0.

**Built 2026-11-13 (slice 4):** dialogue history was already apart from the
facts: visits are not in the fact horizon, so a death keeps them (slice 1).
But it was one count for the whole session. A visit now names its seat
(`PersistedDialogVisit::seat`; a save written before reads as the primary
seat's). The conversation's input owner decides whose count rises and whose
history Yarn's `visit_count` reads (`ActiveConversation::history_seat`; a
conversation every seat drives is the primary's). Schema 468. A census of
the scripts: no branch on `visit_count` gives an item, a flag or a quest
step. Monty Carlo's show number only picks the car's curtain, and the choice
wins it.

**Done.** All four slices are in.

**Acceptance:** the dialogue arms of section 11: solo rollback consistent; an
authored checkpointing dialogue banks its consequences; a single co-op death
does not roll shared facts behind the survivor; a seen marker grants no reward
or quest step.

### GROUNDED-ATTACK-MOMENTUM — measure before changing the slide

**Owner:** [unified movement kernel](engine/unified-movement-kernel.md) and
the moveset (`MoveFrameData::roots_steering`). Ruling: Q169 (filed as Q166,
2026-10-19).

**Current state (2026-10-13):** a rooted grounded move takes the stick; each
law then brakes at its hands-off rate. The axis laws stop in about 3 px from
270 px/s; Sanic's surface-momentum law coasts 351 px from 530 px/s.

**Measured 2026-11-10, the robot:** a grounded attack takes the stick (it
walks through the swing at run speed) and keeps no speed of its own (hands off,
it brakes as standing does, in under a pixel). That is the authored Hollow
Knight model; no change is recommended
([table](engine/unified-movement-kernel.md#grounded-attack-momentum-grounded-attack-momentum-q169)).

**Measured 2026-11-11, Smash** (`fighter_motion demo`, [table](engine/unified-movement-kernel.md#grounded-attack-momentum-grounded-attack-momentum-q169)):
a grounded move keeps the run's speed and brakes at traction (a jab out of a
let-go run slides the 42.5 px stop slide), it does not take the stick, and
George's dash attack carries the run 115.6 px on its authored impulse. No
change recommended.

**Next action:** Sanic's surface-momentum law (its roll) on a flat floor,
a dash cancel, and a controlled ledge. Found on the way, for the portal
owner: a Sanic ball frozen at 1471 px/s by a `WorkLimit` motion refusal
every tick ([note](engine/unified-movement-kernel.md#grounded-attack-momentum-grounded-attack-momentum-q169)). Originally: measure, for
representative laws and moves: whether the move
takes the stick; whether it keeps horizontal speed; which braking law applies
while it plays; whether authored move motion adds travel; whether it can carry
its user off a ledge. Compare with Smash Ultimate (traction, dash cancels) and
Hollow Knight (Cyclone Slash, Dash Slash). Recommend; implement only what the
evidence justifies, as per-move data if a law cannot express it. The AI's
move commitment and ledge risk are separate work.

**Acceptance:** a measurement table in the kernel page, one witness per
distinguished fact (coast, brake, authored displacement, ledge carry), and a
recommendation.

### SMALL-VIEWPORT-UI — screens fit a phone in landscape

**Owner:** the menu backend (`crates/ambition_menu/src/render/bevy_ui/`) and
each pixel-laid screen. Ruling: Q165 (2026-10-19), screen by screen.

**Current failure (measured 2026-10-09 at 640x360):** the Smash select screen
shows 3 of 6 roster columns and no START; the dialogue box covers half the
screen; the touch stick covers each menu's left side.

**Next action:** one screen at a time, state what it does when small (scroll,
fewer lines, a floor for text and touch targets), with a `capture_scene`
witness at 640x360 and 1280x720.

### CONTENT-AS-DATA — content still written as Rust

**Owner:** [fast iteration implementation](engine/fast-iteration-implementation.md)
and the owning game pack. The major migration is built: game text, item and
rules tables, stages, world manifests, props, published art references, demo
module specials, and ordinary pack reload now have authored sources. Keep
engine-owned defaults and code-derived structure in Rust; do not move them
only to increase a census count.

**Remaining executable slice:** on a *confirmed* game-authored value still
read once at startup, give its schema a `Restater` or move the reader to use
time. First reproduce the edit/reload gap; change one schema at a time and
delete any duplicate Rust value. The [packet catalog](engine/fast-iteration-implementation.md)
owns candidate ports and checks; recheck the current source rather than using
the 2026-10-16 declaration count as a new inventory.

**Deferred:** game-specific movement VFX overrides have an agreed optional
`room_rules` policy but no game currently authors a different look. Promote
only for such a customer; do not copy the engine's defaults into every pack.

**Acceptance:** changing the authoritative data reaches the next applicable
session without a Cargo rebuild; a malformed value names the source and field;
no duplicate Rust reader remains. Report edit-to-test time for the slice.

### TEST-LANES — keep required test lanes executable

**Owner:** [test consolidation action plan](test-suite-consolidation-action-plan-2026-10-26.md)
and [test corpus evidence](test-corpus-static-audit-findings-2026-10-26.md).

**Selected next action:** reduce duplicated Smash/Versus fixture setup while
preserving distinct behavior witnesses, phase boundaries and negative controls.
Remove obsolete diagnostics only after checking their actual callers. Use
current `AGENTS.md` verification policy: no test-count/source-count floors or
new meta-test machinery as a proxy for behavior.

**Measured 2026-11-10 (boot cost):** in a sample of 152 `app_it` tests, the mean
was 29 s and ten took 100 s or more. A harness boot costs about 11 s, and most
tests boot at least once. Two pure parses ran again on every boot in one
process:
- the sprite sheet table (2.4 s), which now prepares against the process's last
  table (`content::libraries`);
- the LDtk world (about 0.75 s, twice per boot), whose parse is now kept per
  path, size and file time (`LdtkProject::load_from_path`).

Boots after the first now cost 7.0–7.7 s (`boot_budget::probe_what_one_boot_costs`).
What remains is not one hot spot:
- character preparation: 0.9 s over 212 characters;
- sheet text reads: 0.75 s;
- the asset catalog: 0.5 s;
- Yarn: 0.4 s;
- the first update: 0.9 s;
- the settle: 1.6 s.

**Red without the boot change (2026-11-10, this VM; each also fails at the
commit before it):**
- `boot_budget::no_system_is_registered_twice_in_one_schedule`: two copies of
  `ambition_held_items::project_item_body_presence` in `GgrsSchedule`, a
  composition defect.
- `an_edit_reaches_the_shipped_game::a_reload_that_moves_a_tracks_file_plays_the_new_file`
  and `developer_edits_under_rollback::god_blink_in_ambition_does_not_grant_blink_to_maryo`:
  no rendered music on this VM; the Mary-O route never activates a body.
- `ambition_platformer2d` lib, two tests in `game_assets::sheets::tests`: no
  published art on this VM.

**Red source guards (2026-11-10, measured on main):** five checks were red on
other agents' drift. Repaired here: `is_test_path` did not know a file its
parent declares `#[cfg(test)] mod name;` (34 files, among them
`camera_scenarios.rs`, whose scenario App counted as a second in-session
writer of `ControlledSubject`); five adjudicated Kaleidoscope/visual-quality
resources no longer have a writer (their verdicts are gone); `body_seed`'s
borrowed `BodyGeometry<'a>` view collided by name with the shared-tangle
resource (renamed `BodySizes`). The eight NEW
multi-writer resources it then named are adjudicated (`AudioPlaybackConditions`,
`BodyGeometry`, `EditableAbilitySet`, `LocalSeatClaims`, `PlayerGlowCatalog`,
`PromisedRoomPlans`, `SfxLoopPlayback`, `SfxOneShots`: disjoint fields, a
keyed registry, an editor plus the session edge, a player and its mixer).
Then the next layer: thirteen adjudicated resources had gained or lost a
writer since their verdicts; each verdict carries a dated addendum for its new
writer (or its departed one), `SeatMenuFrames` has a new verdict (the per-seat
menu pipeline replaced the tool-only one), and `MapMenuState`'s baseline
follows the file its verdict already named. The guard is green, and so is
`check_session_authority_none_arms` (fifteen rows written, two stale ones
gone, one read made required). Then the other three: the session-root
declarations follow the per-room rules fixture to its new crate and name the
menu's `#[cfg(test)]` hand seam; `FactionRelations` is composed and installed
by Ambition's content plugin rather than written through `resource_mut`
(the guard now sees a typed `insert_resource::<T>` install); `ActorGlow` and
`PresentationCover` are waived as presentation (render-crate `Update`
filters only). `check_level_box_readers` was red with 73 unclassified
`kin.aabb()` reads; its premise had gone (2026-11-10): since the body
material, `BodyKinematics::aabb()` is the material's envelope
(`material.half_extent(canonical_size * 0.5)`), which righting turns toward
the body's DOWN, so it IS the turned box the guard asked readers to use, and
`collision_box(last_step)` no longer exists. The guard is re-aimed at the
level box's present spelling, a world box built from the unturned half
(`Aabb::new(<not the origin>, ..canonical_size * 0.5)`); that population is
zero, so its baseline is empty and the check is green. Evidence that `aabb()`
is turned: `a_body_in_sideways_gravity_collects_the_pickup_its_own_box_touches`
(items) turns a body's material a quarter and collects a pickup only the
turned box touches.

**Red app tests seen in a focused lane (2026-11-10, on ae2e0e8bab, before and
after the ISL-2d bag slice):** `gnu_ton_fight::with_its_arms_on_a_blow_launches_him_and_the_third_blow_throws`
and `gnu_ton_fight::thrown_off_the_back_he_comes_back_and_he_goes_to_a_player_at_its_far_end`
("no blow landed"; the file's last commits are the portal owner's limb-sweep
work, so they are reported to it, not fixed here), and
`two_seats_two_items::two_local_seats_each_pick_up_their_own_bomb` ("the
match seats this fighter": a Smash match, which on this VM waits for
unrendered music). Also red on 316ff3ef7f with no local change (2026-11-11,
stashed and re-run): `smash_ride::a_shark_summoned_into_a_save_that_remembers_a_dead_one_is_still_alive`
("the opening ceremony never released the cast"), the same Smash start.

**Also red with no local change (2026-11-11, my files stashed and re-run):**
`shell_host_rendered::provider_relative_sfx_resolves_the_real_source_and_rejects_stale_work`
("Sanic resolves its authored procedural Dash") and
`smash_cpus_damage_each_other::the_goblin_and_the_pca_do_not_ask_for_the_same_sound_many_times_on_one_tick`
(the match never seated two bodies): the Sanic and Smash starts on this VM.

**Red at f18c6f4875 and every later commit checked (2026-11-11, checked out
and re-run):** `blink_run_reachability::blink_run_blinks_across_the_death_gaps`
("should have blinked across gap 1 onto the mid floor", one reset).
**Classified 2026-11-11, two causes.** (1) An engine defect, fixed: the
player is now 30 by 48 and `blink_run`'s corridor is 48 high, so the body
touches floor and ceiling, f32 rounding put its bottom face 1.5e-5 into the
floor, and `blink_target`'s embed check (an exact `strict_intersects`)
refused every blink in the corridor; it now allows 0.01 px
(`a_body_that_fills_its_corridor_blinks_along_it`). (2) Content against the
body: each gap is 128 px of hazard (8 tiles) and a blink carries 150, so a
30-wide body lands 8 px over the hazard at best (and the test's walk edges,
275 and 595, are past the 273 and 593 where its edge first touches the
hazard). `blink_run` is LDtk-owned (`level ownership status`). **Fixed 2026-11-11:**
each gap is 7 tiles (112 px; the far column is floor again) and the test
blinks from 266 and 586; it passes. Also red in the same lane, environmental:
`director_teleport_blink` (three) and
`smash_ride::a_saddled_body_advances_its_clocks_and_none_of_its_displacement`
("the match seats a first fighter", the Smash start).

**Kernel lift double carry (found 2026-11-11, Jon's "the shark up to the
Mockingbird vibrates"):** `mockingbird_fight::a_rider_rises_with_the_shore_shark_without_bouncing`
(ignored, known red) rides the shore shark (`mockingbird_arena_lift`, 300
px/s, a one-way lift). Of 122 carried ticks, one carries the rider twice: at
the shark's top 485 (tick 69) its feet go 490 to 480 and stay at 480 the
next tick. Every other tick they are within 6.2e-5 px of the top, and the
gap's f32 step changes at 535 and 510, so the second carry is a rounding
edge between the gravity axis' repair and its sweep's carry, not a
geometry contact (the shaft is clear). Not fixed: the collision kernel's
material casts. The steady stutter Jon saw was the platform's picture,
drawn at its tick position under a rider drawn between ticks: fixed
(`a_moving_platform_is_drawn_between_its_ticks`).

**Red with portal LDtk compiled out (2026-11-11, portal agent's area):**
`ambition_content` lib `intro::tests::each_intro_room_says_its_cutscene_in_data_and_the_library_has_it`:
"level 'central_hub_basement' PortalGunSpawn ... the portal LDtk converter
is compiled out (enable the `portal_ldtk` cargo feature)". The shipped
worlds no longer compose for a crate built without `portal_ldtk`. Reported,
not fixed (portals are the portal agent's).

**Red at 392defdb08 (where the test was last written) and every later
commit checked (2026-11-11, checked out with the current map submodule and
re-run):** `companion_dog::the_dog_climbs_to_a_player_who_stands_far_away_on_the_upper_deck`
("the dog's first place was (1335, 1888), not beside the player at
(420, 1608)") and
`companion_dog::a_brute_brain_in_the_dogs_body_climbs_to_its_target_and_one_that_does_not_navigate_stays_below`
("rose 0 px"). Older than the ISL-1b work. Not classified: the checkouts kept
today's map submodule, so a map change under an older test is the first
suspect.

**Red at 13ca8d5c34 and every later commit checked (2026-11-11, checked out
and re-run):**
`death_restores_the_checkpoint::a_death_takes_back_from_bobs_hand_what_alice_banked_in_hers`
fails its control ("Bob holds the gun-sword Alice put down, next door":
`(0, 0, 1, "Placed")`, the sword still placed). Older than the ISL-1b
work; not classified.

**Red guard reported to the portal owner (2026-11-11):**
`check_session_authority_none_arms.py` is red on
`game/ambition_app_tools/src/bin/capture_scene/portal_fixture.rs::place_portals`
(added in 9d6f9da4ff, "Render portal limbs from their physical roots"): it
reads `ActiveSessionScope` optionally with no `none-arm:` row. The portal
owner's file, so it is reported here, not fixed.

**Uncleared verification (2026-11-04):** the last full feature run recorded
30 failures in five targets. Subsequent selected app cases (seven) and spawn
owner cases (14) passed after repairs; **those focused passes do not clear the
full matrix**. Re-run/triage on a machine with the required published art and
rendered music. On the sampled VM, Smash and Sanic first rooms could not
start because their music tracks were not rendered; classify missing assets
separately from code failures. Keep failure evidence in the lane ledger.

**Compile ratchet reconciled (2026-11-10):** the baseline frozen at
`4ed566a24ad1` disagreed with its own crate table for `ambition_geometry`, the
monolith and the core. The gate had been red on a gating finding: the critical
path had grown from 16 crates to 24. The measured path is `ambition_geometry →
core → time → shared_tangle → world → portal2d → projectiles → combat →
held_items → abilities → shrine → construction → items → breakables →
projectile_combat → actor_monolith → session → sim_view → runtime →
rollback_ggrs → platformer2d → fighter_specials → content → app`. The links
it gained are AP14 carves stacked in series under the monolith. The baseline
is re-frozen with that reason at `7c6ca7f71e`, and the gate passes. If build
wall-clock matters, that series is where to cut. Thirty-nine new crates have
no measured compile time; they are priced at the median rate, a placeholder,
until `scripts/measure/compile_collect.py` measures them.

**Other precise follow-ups:** capture an assertion if the historical
`composes_through_the_sdk` in-company failure recurs. These are not grounds
for retries or enlarging fixed-frame waits. Optional broader cleanup belongs
in the owner plan, not as a second chronological task list here.

**Acceptance:** selected duplicated setup is removed without reducing tested
behaviors; required checks run with honest PASS/FAIL/NOT RUN classification;
the outstanding feature-matrix failures are reproduced, repaired or explicitly
classified against their prerequisites.

### AP14 — actor kernel decomposition

**Owner:** [`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
and [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md).
Jon's current instruction selects the remaining decomposition. Move each
responsibility to its owner and keep one body, control, and action path.

| # | Item | Next step |
|---|---|---|
| AP14 | Complete actor-monolith responsibility separation | The kernel module graph is a DAG. Lifecycle, publication, materialization, items, world objects, device input, observation, and global schedules have explicit owners. The public `body` API owns execution; `control` owns accepted control. Room technique grants follow each driven body. Real simulation covers two-seat possession, reaction clocks, controlled-body removal, action continuity, one-step free motion before and after seat releases, and mounted motion through timed dismount under rollback. Audit residual responsibilities and public operations. Keep accepted control and body execution together. Preserve rollback, publication barriers, and the number of active systems. See the [decomposition plan](engine/actor-monolith-decomposition.md#dependency-first-extraction-sequence). |

**Built:** `d541442ba` installs gate and lock-wall effects through
`SessionRoomVisualsPlugin`. It removes the game-host registrations and their
obsolete exceptions. The owner tests and installation scan pass; the kernel
module graph remains a DAG. Do not add host-installation exceptions for these
effects.

**Acceptance:** each cut moves state, behaviour and installation together and
deletes the old edge; `python3 scripts/measure/measure_kernel_module_graph.py --scc --cuts --edges 80`
is run after it.

### I2/I3 — finish independent content authoring and safe reload

**Owner:** [extension model](engine/extension-model.md) and
[fast iteration implementation](engine/fast-iteration-implementation.md).

**Built:** app-scoped pack selection; admitted generation publication;
no-op/stale/superseded edit handling; reload for moves, characters, audio,
cutscenes and quests; native/WASM procedural modules; `actor_think` S1-S3;
held traversal modules; and the `body_contact` extension route. Do not
re-enqueue the old missing-reload-family list or repeat the completed brain
migration.

**Next action:** finish the specifically named `body_contact` customers:
Mary-O snake-shell behavior and the Sanic badnik contact behavior, including
toucher roll/contact harm through their existing owners. The plant is a brain
and pose customer, not a contact rule. Recheck which customers are still native
before migrating them.

**Acceptance:** authored changes publish exactly once at an admitted
generation; stale generations are refused; these customers run through their
supported module road without a native duplicate or second source of truth.

### A7 — make item occurrence authority explicit where it carries a real invariant

**Owner:** [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).

**Current state:** `GroundItem` construction is sealed, but that seal does not own
occurrence creation. Death drops, match spawns and other roads still mint
identity/provenance/custody facts at their occurrence sites. Two mint sites
degrade on purpose and belong to the `ItemCustody` inventory leg:
`ambition_held_items`'s thrown-item mint and `puppy_slug_gun`'s minion mint.

**Concrete inventory customer:** [bomb pocketing](game/ability-loadout.md#open-for-bombs)
needs one occurrence-to-quantity exchange that the checkpoint can restore.
Record the received quantity and the object's retirement at the existing
custody/accounting owner. Prove conservation through throw, pickup, pocket,
death and save/load; do not add an unrelated grants mirror beside spends.

**Next implementation:** centralize only the occurrence decisions that share an
actual invariant (identity, custody, provenance, rollback ownership). Do not add a
generic request bus merely to reduce writer count.

**Acceptance:** reward policy consumes accepted occurrence outcomes and cannot
become an alternative minting authority; every remaining occurrence creator has
an explicit ownership reason.

### BRAIN — finish truthful fighter attack selection

**Owner:** fighter combat options and [brain behavior](engine/fighter-tactical-intent.md)
only where the current production option resolver needs geometry, not a
reactivation of the paused tactical-intent experiment.

**State:** whiff memory, startup-as-first-live-hit, pure-counter valuation,
stop-distance and coast-distance features are implemented. The 2026-10-13
Sanic rung-5 run still left the stage under grounded attack momentum; measure
and resolve that in GROUNDED-ATTACK-MOMENTUM (Q169), not by repeating a
fixed-range or stale-view brain workaround.

**Remaining roads, remeasure before implementing:**

1. Projectile launch value: a ranged move has `launch: 0` although its hit
   uses `HitKnockbackMagnitude::FeelScale`. Clarify the resolved launch and
   hazard-coverage features against real kit choices.
2. Placed traps: resolve the hazardous region relative to the attacker,
   rather than treating placement radius as attack reach.
3. Teleports: expose the resolved destination (`behind_nearest_foe`,
   `behind_gap`, aim) to the attack offer.
4. Difficulty ladder: measure `--rungs 3,4,6,7,8` and distinguish step size
   from the missing L3 planning step; shipped rungs set `rollout_depth: 0`.

**Acceptance:** representative CPUs choose movement-compatible attacks
across their authored kits without replacing attack admission with a second
rule, a reach floor, or fighter-specific evaluator exceptions.

### D72 — continue Smash parity from the inventory

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

**Current state:** the inventory is the source of feature/parity truth. Do not
turn this queue row back into a chronological parity diary.

**Next implementation:** take the next inventory row whose policy is settled,
implement it on the production path, update that inventory row and add the
production acceptance witness.

**Blocked where applicable by:** product rows named by the inventory. (Q62
and Q89 were ruled on 2026-10-04: Q62 is the LDTK-SEMANTIC-DIFF task, and a
stand-in may keep an incomplete kit.)

### PORTAL-SIMILARITY — canonical parts and scaled mirror portals

**Failure / customer:** unequal apertures and reflected bodies need one
physical material model for partial contact, combat and presentation. The
playable 80/40 slice and chained-limb witnesses pass. Full PSIM acceptance
remains open.

**Owner:** [portal parts, similarity, chirality, and numerical failure plan](engine/portal-parts-similarity-and-chirality.md).

**Next action:** continue implementation and testing on main. Native material
repair and support are fixed in `5f07fbb17`; independent captures and room
identity are fixed in `ae8f2b07a` and `b499e5090`. Adaptive capture storage
and material-proxy overflow are fixed in `1d551594b`. Measure portal-free
runtime cost and hardware/device capture cost. Complete the
remaining motion, geometry, numeric-fault and device requirements in the owner's
[implementation plan](engine/portal-parts-similarity-and-chirality.md#implementation-plan)
and its [status](engine/portal-parts-similarity-and-chirality.md).
This is local to one live simulation room; cross-island live portals are not
an implicit dependency.

**Acceptance:** all five PSIM slices have production-path and deterministic
witnesses; authored mismatched apertures scale correctly in both directions;
part geometry, collision, combat and masking agree; reflection persists as
correct actor chirality after transit; bounded rejected numeric transforms
show the typed IEEE 754 effect without poisoning simulation or replay.

### PORTAL-STRUCTURAL-FX — sprite and particle effects ignore the panes (done)

Jon, 2026-10-17: "it should not be structurally possible to add something
into the game that doesn't work with portals." Meshes built on the CPU now
have one road that folds them through pairs and out of far-side panes
(`ambition_render::folded_mesh`, used by every line look). The `fx.rs`
effects have none: particles, beams, impacts and slash sprites are drawn
whole at z 30 and up, over a pane on its far side, and are not cut at a
crossing. Measured the same day by reading `portal_compositing.rs` (they
carry no candidate marker). Next: give them that one road (a sprite effect
through the far-side and transit pieces by construction, or the folded
writer for quads), not a per-effect opt-in.


**Far side, done 2026-10-18:** a pane composites every top-level drawable of
its room drawn at or above the window (`publish_portal_compositing_candidates`),
not only the three marked kinds, so a particle, a beam or a slash behind a
pane is hidden and redrawn in its uncovered parts by construction. Two kinds
opt out: the compositor's own copies and the parallax layers (placed against
the view). Witness: `a_far_side_effect_is_composited_with_no_marker_of_its_own`
(red under the old marker filter, without the parallax opt-out, and without
the portal-room check). Cost (release, 2000 sprites, 1000 over the window,
one pane): 64.9 µs a frame, against 34.7 µs for the old rule, which took
only the 100 marked ones; with no pane 16.3 µs against 24.7 µs. A candidate
that stays one is written in place: an insert command for each was 179 µs.

**Crossing, done 2026-10-25 (63797b0ac):** the portal carry
(`ambition_portal2d_presentation::carry`) cuts every top-level `Sprite`,
`Mesh2d` and `Text2d` that reaches past an entry plane inside the opening, and
draws its part past the plane out of the exit. It needs no marker, so every
`fx.rs` particle, beam, impact and text is carried. A child goes with its
parent. Witness: `a_free_drawable_keeps_the_room_policy_beside_a_body_with_its_own_policy`
(a plain unmarked sprite across an entry is cut into pieces and captured).
`sync_portal_body_pieces`, named by the earlier open note, no longer exists.
No effect in `fx.rs`, `fx/`, `lines/` or the slash visuals draws with
gizmos or another kind, so nothing is left open here.

### MOUNT-RIDER-CUSTOMER — a shipped rider controls a shipped mount

**Owner:** `body/brain_command.rs` (`MountedBrainCache`, the source-only
arm) and the dismounted-rider road (`body/dismounted_rider.rs`).

**Ruling:** Q76 (2026-10-03): keep mount/rider support and give it an early
customer (a character riding the dog, or the robot commandeering a shark).

**Current state:** `MountedBrainCache` has no production constructor, so
`ControlClaimant::Mount` has no production writer. Nothing resumes the mount's
recorded brain when a ride ends; a mount death rebuilds a solo brain from
config.

**Next action:** author the customer, then specify and repair, each with a
witness: control transfer, the mount's previous brain, dismount and its
restoration, body and room lifetime, damage and death, participant ownership,
animation composition.

**Acceptance:** the customer mounts, the mount obeys the rider, and on
dismount the mount runs the brain it had before; the same after a rewind.

### RIG-IMPOSTOR-CONTAINMENT — a part-drawn body is drawn whole or refused

**The invariant:** every pixel a composited body draws lies in its cell. One
camera draws a whole page with no per-cell scissor, so a part past its cell
is cut from its own body and drawn into its neighbour's.

**Owner:** `ambition_render::rendering::actors::rigged` (`impostor_margin`,
`posed_reach`), `ambition_sprite_sheet::character::rigged` (`art_overhang`)
and `scripts/measure/measure_rigged_parity.py`. Plan:
[`engine/mary-o-part-realization.md`](engine/mary-o-part-realization.md).

**Scope since 37497a444 (2026-10-05):** a part-drawn body draws its parts in
the world, and uses an impostor cell only while something composes it
(`ComposedBodyDemand`: hit flash, portal pieces, deep dream, quasar) or its
frame fades as one picture. So the invariant applies to that composed road
only.

**Solved, by kind of draw:**

- A published frame (2026-10-05). The cell is the frame plus
  `impostor_margin`, which covers the flipbook's stated `art_overhang`. The
  oni leader's banner is drawn whole (`a_cell_holds_every_draw_of_its_body`).
- A tween's in-between (2026-10-06, review P2). `art_overhang` is a
  conservative envelope of every draw `tween_into` can make, taken when the
  flipbook is read: each moving draw is sampled finely enough that no corner
  moves more than half a pixel between samples, and that slack is added
  (`tween_overhang`). A part that turns between two frames reaches where
  neither frame does (`the_overhang_of_a_tweened_clip_covers_its_in_betweens`;
  the published corpus: `no_in_between_of_a_published_flipbook_reaches_past_its_overhang`).
  Measured on the 123 published flipbooks with a tweened clip: the in-betweens
  move the stated overhang of 12 of them, by 0.21 px at most; no cell changed.
- Parts placed by a `PartPose` (2026-10-06, review P2). A pose is in no file,
  so its reach is measured each frame (`posed_reach`). While it is more than
  the margin, the body is drawn directly, as a body whose frame fits no cell
  is: whole, and its readers see no image, with one warning. The hold that
  smooths composition demand smooths this too
  (`a_part_pose_that_reaches_past_its_cell_is_drawn_directly`).

**Open:**

1. `scripts/measure/measure_rigged_parity.py` still clips its oracle to the cell, so
   a cut part cannot read as a parity failure there. The clip was added when
   the cell was the frame plus 16 px (the oni leader's banner read as 1153
   wrong pixels of a correct draw). With the margin from `art_overhang` the
   clip should remove nothing. Remove it and run the harness on a GPU
   (`rigged_sprite_parity`); acceptance: the oni leader passes unclipped, and
   with `impostor_margin` poisoned back to 16 px it fails.
2. A posed body that reaches past its cell loses its composed readers (its
   hit flash overlay, its portal pieces) while it does. If a ragdoll must keep
   them, give the presentation a cell chosen from the pose's reach, or let a
   pose declare an envelope. No shipped body carries a `PartPose` today.
3. A two-body shared-page test that reads the pixels of both cells needs a
   GPU; the three proofs above are on the draw geometry.

### DENSE-MELEE-ROOM — author the dense-melee development room

**Owner:** content; first measured customer
[`engine/bounded-perception-and-attention.md`](engine/bounded-perception-and-attention.md#acceptance-for-the-open-increment).

**Ruling:** Q93 (2026-10-04): keep or create a deliberate dense-melee
development/stress room. It is a real scenario, not polished content.

**Current state:** no such room; the density script fakes density by widening
the viewport over the Hall's `stand_still` cast.

**Next action:** after the P1 correctness rows above, author one room (LDtk or
a generated spec) with many tactical-brain fighters in close melee, reachable
by an ordinary route and by `measure_perception_density.sh`.

**Acceptance:** in the room `kept` tracks population until the budget caps it,
and the census, frame time and camera framing are readable from one run.

### ASSET-PRODUCT-LAYOUT — runtime roots by tier; editor products apart

**Owner:** `ambition_asset_manager` path builders, `ambition_sprite_sheet`,
the publish scripts. Plan:
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md#products-are-laid-out-by-what-they-are-q82-q83).

**Ruling:** Q82, Q83 (2026-10-04).

**Current state:** full quality is the bare `sprites/` root and the reduced
tiers are suffixed siblings. ✅ 2026-10-15, the ultrapack's runtime product
is cut by runtime unit: only the pack-plan groups a runtime reader loads ship
(`runtime_groups`), 1.8 MB, was 695 MB; the whole pack is a source in the
renderer's cache (owner page).

**Next action:** audit the listed path builders, then move to
`<root>/{full,half,quarter,potato}/` in one change.

**Acceptance:** every tier is a named directory; the packager selects roots
and has no new exclusion; the packaged ultrapack holds only reachable pages;
the game draws the same at each tier.

## Receipts

Closed rows retained only where an open row, a script or an inbound link still needs their identity.

### SMASH-DEFECTS — select-screen, respawn and result lifecycle

Done 2026-11-10: SD-01 to SD-05 (off-page portraits drawn, default air jumps at respawn, a result clock that ran while paused, a stale mouse at route entry, invisible page controls), each with a headless test at its owner that a poison of its fix reddens. SD-06 (47 select-screen components marked changed on every idle frame, now 0) and SD-07 (one portrait resolution per occupied card per frame, now 0; a failed load restarted every frame, 8 ms a card on a VM missing two portraits) measured before and after; the windowed UI-layout saving of SD-06 is unmeasured and not claimed. Owner: [Smash defect review](demos/smash-defect-review-2026-10-31.md).

### NET-INPUT-WIDTH — remove idle room consensus from every frame

Done 2026-11-10: `PeerInput` is 80 bytes at wire identity 9 (was 168); the 96-byte verdict is an 8-byte token compared, never decoded, by each peer. All `two_peers` agreement cases pass. Measured link traffic fell only 0.5% in steady play and 4% across crossings, because GGRS already delta- and run-length-encodes inputs. Owner: [compact peer input](engine/island-rollback-and-compact-peer-input.md#result-2026-11-10).

### LOADOUT-AUTHORITY — equipment changes enter through simulation input

Done 2026-11-10: both menus turn an equip or a stow into a stamped host intent (`ItemEquipRequested`, and the new `ItemStowRequested`); the simulation applies an ability to the worn set (`apply_item_equips`) and a weapon or the portal gun to the hand (`ambition_held_items::apply_hand_requests`), on the body the requesting participant drives. The menu writes no body. Witnesses: `hand_request_tests` (own hand only, missing or ambiguous driver, portal pair kept), the menu parity tests through the real applier, and app_it `a_menu_equip_is_a_stamped_request` (equip and stow replay to one hand under the sync test). Schema 433. Owner: [ability loadout](game/ability-loadout.md#open-for-bombs).

### PARTY-TRAVEL-POLICIES — configurable group crossings and a leader view

Done 2026-11-04 (`4cb723b8c`, `5ed9bb73a`): independent/leader/any/all-member crossing policies, bidirectional gate idle timing, native body transfer and leader view are implemented. Default remains independent adaptive play; keep camera geometry out of travel authority. Owner: [game multiplayer](game/multiplayer.md).

### SHOT-END-REPLAY — a brain that saw its shot end replays differently — ✅ DONE 2026-10-11

Done 2026-10-11: the replay inconsistency was a stale body-motion read on rewind, not shot-end identity. See [integration audit](triage/2026-10-11-integration-source-audit.md).

### A4 — separate control authority from body execution on the real schedule — ✅ DONE 2026-10-09

Done 2026-10-09: accepted participant control is separated from body execution on the real simulation schedule. See [simulation authority](engine/simulation-authority-and-determinism.md).

### AUTHORITY-POLISH — one owner per mechanical fact, and no mirror in the rollback kernel — ✅ DONE 2026-10-09

Done 2026-10-09 (C11): no second mirror is retained as an authority for the surveyed mechanical facts. Reopen only for a concrete duplicate. See [consolidation plan](consolidation/consolidation-plan.md).

### SETTINGS-ROLLBACK — a settings change reaches simulation only at an admitted rebase — ✅ DONE 2026-10-08

Done 2026-10-08: settings reach simulation only through admitted rebase. Keep per-scope settings out of unvalidated mid-session writes.

### AUTHORED-INTERACTABLE-STATE — facing gates, per-chest and per-pickup persistence — ✅ DONE 2026-10-08

Done 2026-10-08: facing gates and chest/pickup state use explicit per-object persistence. See [authoring and tools](engine/authoring-and-tools.md).

### ID-PEER — remove host-local lineage from peer-stable mechanical identity — ✅ DONE 2026-10-03

Done 2026-10-03: peer-stable mechanical identity no longer inherits host-local lineage. Preserve the local/canonical identity boundary; consolidation ledger and checks depend on this closed row.

### A10 — candidate world / last-good-world publication — ✅ DONE, DEMOLITION CLOSED 2026-09-16

Done 2026-09-16: candidate-world and last-good-world publication are complete. A failed candidate cannot mutate the live published world. Keep this closed row for the consolidation-ledger guards.

### A12 — finish move-contact attribution and reflection identity — ✅ DONE 2026-09-19

Done 2026-09-19: move-contact and reflection attribution use occurrence identity; an outcome for another/no occurrence grants no move credit. This A12 is not the separate frontier A12 flow-validation task.
