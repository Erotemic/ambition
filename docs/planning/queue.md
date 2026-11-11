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
5. **Developer iteration and inspection:** [GEOMETRY-OBSERVABILITY](#geometry-observability--render-the-effective-geometry-behind-gameplay) makes the effective geometry behind collisions, attacks, interactions, portals and navigation visible and machine-readable; it is a concrete inspection customer, not a broad spatial rearchitecture. [TEST-LANES](#test-lanes--keep-required-test-lanes-executable), CONTENT-AS-DATA, TOOL-DISCOVERY and
   EDITOR-SHELL remain selected at their stated boundaries. Portal, rendering,
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

**Next action:** NA-1 and a first NA-2 are measured (2026-11-10, [results](engine/hosted-world-networking-experiment.md#na-1-and-na-2-2026-11-10)): each GGRS peer spends about 4× the host's update per frame even without rollbacks, and room-scoped replication bytes depend on delta encoding. Half of that is `SaveWorld`: 1,196 systems, about 16 ms per run whatever the body count. The 2026-11-10 direction (hybrid; rollback kept per room while cheap) makes this the hosted half of [SIMULATION-ISLANDS](#simulation-islands--room-level-rollback-inside-a-hybrid-world). NA-2b's first measurement ([result](engine/hosted-world-networking-experiment.md#na-2b-first-measurement-2026-11-10-a-guest-app-that-simulates-nothing)): a guest App with every simulation phase gated off costs about 4.5 ms an update against the host's 9–10, and applying 2–18 body rows costs about 0.1 ms. Next: NA-2b with spawns and despawns by `SimId`; NA-3 guest prediction, NPC persistent state and late join; NA-4 decide per-game backend and update the applicable ADRs. Evaluate compatible Lightyear and `bevy_replicon` APIs without declaring a dependency before the prototype.

**Acceptance:** runnable headless comparison, byte and CPU measurements, stable guest input/reconciliation, region-scoped replication, persistent host-owned NPC facts, no service dependency, and an explicit per-game synchronization decision. Do not remove GGRS or rewrite working multiplayer merely from prose.

### SIMULATION-ISLANDS — room-level rollback inside a hybrid world

**Customer:** the Ambition metroidvania's multiplayer: players in different rooms must not pay for, wait on or rewind each other's rooms. Maintainer direction 2026-11-10: move away from strict App-wide rollback toward a hybrid model; keep rollback within a room boundary while it is easy to maintain, and evaluate what keeping it at room level costs.

**Owner:** [simulation islands](engine/island-rollback-and-compact-peer-input.md#simulation-islands-independently-rewindable-active-areas); the hosted half is [NET-AUTHORITY-EXPERIMENT](#net-authority-experiment--compare-hosted-world-replication-with-ggrs).

**Done 2026-11-10 (ISL-1, census):** in a two-room scene every rollback entity but four has a room (22 in the hub, 2 in `switch_lab`; the four are the two room roots, the session root and a boss encounter). One `SaveWorld` run costs 16.1 ms over 1,201 systems for those 28 entities, so a history per room on the per-type mechanism would multiply that by the live rooms. Five resources couple rooms (impact hitstop and the clock scale, the projectile id sequence, the one conversation, the one cutscene) ([result](engine/island-rollback-and-compact-peer-input.md#isl-1-result-2026-11-10-what-has-a-room)).

**Done 2026-11-10 (ISL-2a, cost):** a room snapshot through the registered rows (`RoomSnapshotRows`) saves the hub (22 entities) in 1.35 ms and `switch_lab` (2) in 0.31 ms, against 16.3 ms for the App-wide `SaveWorld`; it holds no resources or checksum yet ([result](engine/island-rollback-and-compact-peer-input.md#isl-2a-result-2026-11-10-what-a-room-snapshot-costs)). Room-level rollback is affordable on a snapshot that follows the room's entities.

**Done 2026-11-10 (ISL-2b, first witness):** in a hosted world the hub rewinds to a saved frame and replays Alice's inputs alone, with Bob's room hidden (`Disabled`): the hub ends exactly as the first run left it and Bob's room is untouched; the control without hiding moves Bob ([result](engine/island-rollback-and-compact-peer-input.md#isl-2b-first-witness-2026-11-10-a-room-rewinds-alone)). The window spawns nothing and exercises no coupling resource.

**Done 2026-11-10 (ISL-1b, the freeze):** with several live rooms an impact hold or Player 1's hitlag no longer stops every room's clock; one live room keeps the freeze ([owner](engine/island-rollback-and-compact-peer-input.md#next-isl-2c-and-isl-1b)).

**Done 2026-11-10 (ISL-2c, shots):** a room restore despawns what was spawned after the save and re-creates what is gone (references remapped); in the rewind witness Alice's shots fired in the window are fired again by the replay and end where they did.

**Measured 2026-11-10 (ISL-2d):** a coin taken in a rewound window pays the wallet once (it is on the body and rewinds) but is recorded twice in the world's grant record; the chosen design (world records stamped with room and tick, retracted on a room rewind) is built for the grant record (an append store) and for the consumed-occurrence record with the occurrence ledger (keyed: an undo log of the rows each write replaced, witnessed by a heart taken only in the mispredicted window), and for the shared bag (each write inverted: a throw only in the window is put back, and its mint leaves the occurrence ledger), and for boss defeats (with the save row, quest steps and reward the death replay already retracts), and for the world-time schedule (a mirror that heals itself except for the first departure's due time; its undo log is bounded by a 600-tick rewind horizon); the narrative bag writes, quests and flags are next, and the room rollback host will call it ([result](engine/island-rollback-and-compact-peer-input.md#isl-2d-measured-2026-11-10-a-world-write-in-a-rewound-window)).

**Measured 2026-11-10 (ISL-2e):** a hosted world that predicts its guest and rewinds only his room on a late input ends where an on-time run ends, at 9.4–16.3 ms of host update a frame (the late-applying host: 8.4–9.9; each GGRS peer: 33–48) ([result](engine/island-rollback-and-compact-peer-input.md#isl-2e-measured-2026-11-10-what-room-level-rollback-costs-a-hosted-world)).

**Next action:** make the room rollback host production code once the hosted world has a production input path (NA-2b's guest App and transport; until then the rewind has no caller but tests): per-room input history, prediction, a rewind on a late input, the shared clock and message channels, the world-write retraction of ISL-2d. Meanwhile: ISL-2c, an NPC life that writes the ledger in the window (the hit on an NPC is done: `a_rewound_room_hurts_its_enemy_again`); ISL-1b, the remaining couplings (bullet-time's shared scale, the one conversation, the one cutscene), each with a witness that an event in one room leaves the other unchanged.

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

**Next action:** slice 7 waits on the [authority experiment](engine/hosted-world-networking-experiment.md)'s choice. Done 2026-11-10: the day clock is in the save (v7), so a session from a save goes on at the hour its lives planned in. Open: offscreen bookings shared across bodies that come back on different ticks.

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

**Measured and partly fixed 2026-11-10** ([owner](game/boss-cinematics-gnu2-and-motion.md#the-giants-motion-gnu-motion-parts)): the walk row's clock now follows distance walked, so the hooves stand still as the body speeds up and slows down, and rest starts at its first frame after a blow. Phases 2 and 3, the rider's rows and rendered part transforms are not measured yet.

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

**Failure:** the loadout plan still has placeholder variants, no authored
upgrade fact, and hard-coded eye-laser origin measurements.

**Owner:** equipment preparation, body control slots, content modules and
character landmarks. Design: [ability loadout, open work](game/ability-loadout.md#open).

**Next action:** after LOADOUT-AUTHORITY, recheck the published eye-laser row
and add its visor landmark and sound at their source owners. Then support
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

**Next action:** add meaningful dog and pigeon parameters first, then the
other listed species. Keep parameter defaults, generation and studio controls
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

**Next action:** GOBS-0 is done (2026-11-10, [census and lab](engine/geometry-observability-and-f1.md#gobs-0-result-2026-11-10-census-and-the-first-lab)): F1's blocks layer draws 8 of the 18 solids a body meets in `basement_breakables`, and the lab found a broken platform's lingering pogo orb (fixed). GOBS-1's first slice landed 2026-11-10 (the `ambition_geometry_observation` registry, collision and combat providers, F1's Observed Geometry layer reading the same `observe_all` as headless; and the headless `inspect_geometry` binary with provider selection; and F1's per-provider selection, a Developer-page row that cycles all → each installed observer; still open: labels); GOBS-2 render effective per-body collision and contact decisions; GOBS-3 cover combat, interaction, triggers, navigation and portal pieces; GOBS-4 provide practical F1 selection/presets and interpolation comparison; GOBS-5 close coverage and cost regressions. Preserve the already working F1 and combat-view paths.

**Acceptance:** a single representative two-room lab exposes actual collision/hit/hurt/interaction/portal/navigation geometry with source identity, state and why-not reasons; a new provider becomes discoverable in F1 and headless inspection without modifying a central game-specific draw switch; overlays are room- and view-correct, off by default, and do no per-frame diagnostic work when disabled. Tests distinguish absent, filtered, intangible and unsupported geometry. Do not confuse an authored block outline with a subject's effective collision world.

### TOOL-DISCOVERY — find supported commands by developer intent

**Failure:** the generated tool index lists immediate tool directories. It does
not identify supported script commands or Cargo binaries, so a workflow still
requires broad source search.

**Owner:** `scripts/generate_agent_index.py`, `scripts/agent_query.py`, and the
packages that own each command.
Design: [tooling layout, T0-T1](engine/tooling-layout-and-editor-consolidation.md).

**Priority:** after the selected asset-contract and A9/AP14 ownership work.

**Next action:** inventory supported capture, moveset, LDtk, asset-inspection
and ECS workflows on current main. Record their canonical command, help,
inputs, outputs, mutation policy and build or binary-reuse requirement.
Identify path consumers before any move. Extend the existing generated index
from tool-owned descriptions and existing entry points. Keep internal and
experimental commands distinct; do not add a second command registry.

**Acceptance:** intent queries for screenshots, deterministic capture, move
tuning, room editing, published assets and ECS inspection return runnable
commands and their owner documents. Missing commands have useful diagnostics.
Discovery stays fast and does not require a GUI, Cargo build or asset audit.
No tool or submodule move is part of this slice.

### EDITOR-SHELL — promote the existing inspector without a second authority

**Failure:** the Asset Inspector and specialist tools have separate entry
points, with no common navigation for existing inspection workflows.

**Owner:** the existing `tools/ambition_editor` implementation and its CLI.
Design: [tooling layout, T2](engine/tooling-layout-and-editor-consolidation.md).

**Depends on:** TOOL-DISCOVERY and its path-consumer inventory.

**Next action:** promote the inspector as Ambition Editor under
`tools/ambition_editor/`, with minimal internal renaming. Preserve its asset
browsing, JSON commands, cached previews, serve behavior and authored review
notes. Link to at least one existing specialist tool through its supported
command. Update known callers in the same change; keep only deliberate stable
launchers. Do not merge tool environments or reimplement domain operations.

**Acceptance:** one editor opens the existing asset and review workflows and
reaches another inspection capability. Agents can obtain the same facts and
invoke that capability without browser automation. Review IDs, provenance,
selected libraries and stale-preview access survive. Inspection does not gain
a mandatory build, regeneration or full-library audit.

T3-T5 remain proposed in the owner document. Select one relocation or cleanup
batch only when the discovery inventory shows its owner, benefit and callers.
Keep unique capabilities regardless of use counts; do not add forwarding
wrappers for every moved script.

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

### GATE-STARTUP-FLICKER — diagnose the wormhole at startup

**Owner:** [Gate network](game/gate-network.md#startup-rendering).
**State:** OPEN. Jon reports flicker on desktop and Steam Deck. Warmed High and
Ultra captures show an opening flare and continuous steady rendering. They do
not prove that cold startup is correct.
**Next action:** capture the reported gate from cold startup and locate the
first incorrect frame. Check sprite readiness, animation ownership and the
shader layer before changing art.
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

**Next action, in slices:** (1b) a form with no standing height (scaled by a
placement box today) and `BodySource::Explicit` get one answer on every road
(contradiction 1). (2) The construction paths
agree on intrinsic geometry for the same form. (3) Distinct operations over
shared infrastructure: transformation (same body, new form), possession
(another existing body, same participant), skin (presentation only), body mode
(same body and form). Investigate the rename `WornCharacter` →
`CharacterForm` after the semantics are right.

**Acceptance:** the robot becomes the dog: the body is the dog's built size,
with the dog's moves and abilities, the same entity and identity, and its
damage kept; back to the robot restores the robot's geometry. A skin change
moves no collider, hurtbox or movement value. A possession leaves the original
body in the world, and the possessed body keeps its geometry and state.

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

**Next action:** one recovery state machine over the active participant set,
symmetric for every participant:

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

**Next action (after COOP-RECOVERY):** a configurable co-op policy, off by
default, that respawns a fallen participant beside a living partner (the
2026-10-05 return beside the primary is this policy, generalized to any
partner). The assisted-respawn presentation and UI follow it. Teleporting while
alive is a separate possible feature, not in this row.

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

**Next action:** (1) the checkpoint copies the participant's unbanked facts
(flags, quest steps) with the bag, and a solo death restores both; (2) an
authored dialogue effect that banks (a checkpoint action through the existing
persistence road, one transaction with the facts and objects it moves);
(3) in co-op, shared facts a survivor holds are not rolled back, and their
objects stay consistent; (4) dialogue history (seen, per participant, Q134)
kept apart from the world facts, so a seen marker grants nothing.

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

**Next action:** the same for the Smash traction law and Sanic's
surface-momentum law, and a controlled ledge. Originally: measure, for
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
filters only). Still red: `check_level_box_readers`. Its 27 gone
baseline lines are deleted (the code they named was rewritten, mostly into
`kin.aabb()`), leaving 13 classified; 73 reads are unclassified, largely
the same sites under the new spelling, and each needs the guard's question
asked of it (is the level box right there, or should it ask
`collision_box(last_step)`?) — its own pass, with turned gravity in mind.

**Red app tests seen in a focused lane (2026-11-10, on ae2e0e8bab, before and
after the ISL-2d bag slice):** `gnu_ton_fight::with_its_arms_on_a_blow_launches_him_and_the_third_blow_throws`
and `gnu_ton_fight::thrown_off_the_back_he_comes_back_and_he_goes_to_a_player_at_its_far_end`
("no blow landed"; the file's last commits are the portal owner's limb-sweep
work, so they are reported to it, not fixed here), and
`two_seats_two_items::two_local_seats_each_pick_up_their_own_bomb` ("the
match seats this fighter": a Smash match, which on this VM waits for
unrendered music).

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

**Failure / customer:** linked apertures currently collapse to their smaller
size so transit can remain an isometry; the N-piece partitioner and several
consumers still use rigid maps or specialized presentation/collision paths.
A GNU-ton multi-portal limb and a body partially crossing unequal apertures
need correct scaling, reflection and physically consistent clipping.

**Owner:** [portal parts, similarity, chirality, and numerical failure plan](engine/portal-parts-similarity-and-chirality.md).

**Next action:** continue implementation and testing on main (Jon, 2026-11-08).
Jon released main and requested the tested slot 1 work to be merged. All VM
CPUs are available. Finish live root transit, exact contact projections and
source admission for every mover. Follow the owner's
[implementation plan](engine/portal-parts-similarity-and-chirality.md#implementation-plan)
and [draft status](engine/portal-parts-similarity-and-chirality.md#draft-status).
PSIM-1A and PSIM-1B: prove canonical-material partition invariants,
add bounded similarity composition/parity, and make budget refusal stop the
source motion before extending transit, combat and presentation. Reuse the
existing `frame::thread` worklist. This is local to one live simulation room;
cross-island live portals are not an implicit dependency.

**Acceptance:** all five PSIM slices have production-path and deterministic
witnesses; authored mismatched apertures scale correctly in both directions;
part geometry, collision, combat and masking agree; reflection persists as
correct actor chirality after transit; bounded rejected numeric transforms
show the typed IEEE 754 effect without poisoning simulation or replay.

### PORTAL-STRUCTURAL-FX — sprite and particle effects ignore the panes

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
that stays one is written in place: an insert command for each was 179 µs. **Open:** an effect is still not cut at a crossing
(the transit pieces, `sync_portal_body_pieces`, take only bodies).

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
