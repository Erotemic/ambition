# The queue — live execution order

This file is the current executable engineering queue. It is not a work log, a
review transcript or an archive. Git history keeps completed investigations.
Durable design and measurements belong in the linked owner document. Product
decisions belong in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

Each row states the current failure, the owner, the next action and the
acceptance. A row stays here only while an engineer can act on it. When a row
closes, delete it in the same change. Keep a receipt of 1 to 5 lines (what was
wrong, what fixed it, the guard, any standing prohibition) only when an open
row, a script or an inbound link depends on it. Receipts are at the end.

Re-measure a row before you implement it. Check a row's owner against who is
actually running before you wait on them. When a row converges, rewrite it
around its result. Do not append to it.

Scripts read this file:

- `scripts/check_blocking_set_names_every_gate.py` reads each `**Blocked by:**`
  field to the next blank line, and each `**Acceptance:**` clause.
- A heading that contains `✅`, `DONE` or `CLOSED` marks its row closed for
  that script, for `scripts/check_discharged_holds_are_rewritten.py` and for the
  two consolidation-ledger checks. The ledger holds work on `ID-PEER` by its
  row id, and a ledger test needs `A10` to stay a closed row.
- Other documents and source comments link to row headings. Do not rename a
  heading without fixing its inbound links (`rg -n "queue.md#" .`).

Measure the per-row mass with:

```sh
awk '/^### /{if(n)printf "%s %s\n", c, n; n=$2; c=0} {c++} END{printf "%s %s\n", c, n}' \
  docs/planning/queue.md | sort -rn | head
```

## P0 — architecture and correctness

### SYNC-POINT-SENSITIVE-RESIM — a command sync point moves the death-reset replay

**Owner:** rollback determinism.

**Current state:** a lead, not a reproduced defect. On 2026-09-22 one added
schedule edge (`ensure_sim_id → mint_spawned_sim_ids → heal_projectile_owners`
ordered `.before(SimClockHead)`) made two `rollback_lifecycle_reset` death tests
fail with a GGRS sync-test mismatch. The edge touches no clock state; it only
moves where Bevy applies that chain's `Commands`. Re-measured 2026-09-23, the
same edge passes all five tests. Part of the first reading was an audit defect
(frame-keyed history compared across a lifecycle rebase), fixed and held by
`a_rebased_timeline_is_not_compared_against_the_one_it_replaced`. Nobody knows
whether later schedule changes fixed the sensitivity or only moved it.

**Next action:** probe with an edge that moves a sync point on the death →
checkpoint road, under `RollbackRestoreAudit`.

**Acceptance:** a probe either reproduces a mismatch and names the system that
reads unrestored state (then fix it), or the probes find none and the row is
deleted.

### ID-PEER — remove host-local lineage from peer-stable mechanical identity

**Owner:** deterministic identity / rollback architecture. The identity map is
in [`consolidation/architecture-census.md`](consolidation/architecture-census.md).
The input-payload contract is in
[`engine/netcode.md`](engine/netcode.md#the-input-payload-two-peers-exchange),
and schema negotiation is netcode's
[`N3`](engine/netcode.md#n3--contentschema-negotiation).

**Current state:** every road that touches `SessionRoot`, activation or
provenance is closed, and each one has an arm in
`game/ambition_app/tests/id_peer_audit.rs` or `shell_host_lifecycle.rs`. The
closed roads are: the smash roster seed, `SessionScopedEntity` in the checksum,
the match ordinal, the `MatchInstance`-stamped resources, `SessionMatchOrdinal`,
`random_context`, the checkpoint operation keys, the session root's `SimId`
(the constant `SimId::singleton("session", "root")`), `TransactionId`
provenance, `GameplayElapsed`, the startup-resume checksum, perception's
`Entity`-index fallback, and the GGRS carrier order
(`rebase_rollback_carrier_order`). The `ControlFrame` shape is pinned by a
ratchet. The peer-identity checkpoint that C03 and C05 waited on is discharged.
`two_local_histories_compute_the_same_ggrs_component_checksums` still shows two
rows that differ between hosts: `SimTick` (`Q128`) and `AmbitionGameSave`
(`Q129`).

**Open roads:**

1. **The snapshot schema fingerprint hashes prose.** `compute_schema_fingerprint`
   hashes all of `schema_dump()`, including each row's prose `detail`. Ruled
   2026-09-19 (`Q122`): mechanical identity fingerprints mechanical facts, not
   explanatory prose. Next: split each row's `detail` into the mechanical facts
   the fingerprint hashes and the explanation it does not. ⛔ Do not just drop
   `detail`. Most rows carry facts that `kind` does not encode (entity, set or
   map remapping; the canonical checksum style; custom-checksum descriptions).
2. **The canonical timeline.** `SimTick` is an absolute per-App counter and is
   registered `resource-canonical`, so two Apps that ran for different times
   disagree from the first compared frame. It needs a session-relative tick,
   rebased when peers agree to start. This waits on `Q128` and on netcode's
   [`N2`](engine/netcode.md#n2--first-real-externalp2p-session), because no
   P2P session exists. The rebase is an activation moment: if `Q128` starts
   while a C03 or C05 migration is in flight, coordinate the two.
3. **The 25 unchecksummed float rows** (S7 in
   [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md)).
   The state half is covered:
   `two_local_histories_agree_about_the_sharp_unchecksummed_rows` compares the
   11 reachable sharp rows across two hosts at every tick of five walks. The
   timeline half needs a real two-peer session (`N2`).

A negotiated input version does not exist. It is not owed while netcode is at
`N2`.

**Standing prohibitions:**

- Keep the session term in the rendered `TransactionId`. Only its peer
  projection drops it, because the construction scope's gather filter and A10's
  candidate-vs-live separation read the local stamp.
- Do not project `TransactionId` to `{room}` alone. The peer content term
  (`PeerContentIdentity`) must stay in the projection.
- Do not replace a raw `SessionScopeId` with the nearest canonical-looking value.
  `SimTick` looks canonical and is host-local.
- Do not mint a local `PeerSessionIdentity`. A peer session identity can only
  come from a session handshake.
- An id that crosses to a peer is a function of content or of a canonically
  sorted set. It is never a function of insertion or allocation order
  (`RollbackOrdered`, an `Entity` index).
- `id_peer_audit` censuses type names, so it cannot see a provenance defect.
  Hold provenance with value-level arms in the crate that mints the identity.
  `scripts/one_owner_per_canonical_identity.py` keeps each constant identity to
  one production mint.

**Blocked by:** nothing.

**Acceptance:** two Apps that have burned different numbers of local session
activations can enter the same deterministic match and produce the same
canonical mechanical identity/checksum. The witness must first assert that their
local counters differ.

### ROLLBACK-MUTATOR-POPULATION — the mutator guard sees a quarter of rollback state

**Owner:** rollback scheduling (`scripts/check_rollback_mutators_run_in_sim.py`).

**Current state (measured 2026-10-02):** the guard reads every rollback
registration in every supported parameter spelling, including writes in the
body of an exclusive-world system. It reports 488 systems that mutate rollback
state and 3 acknowledged offenders, all owed to this row:

- `adopt_occurrence_checkpoint_from_save` and `complete_durable_restore`: one-shot
  latches on `SaveRestored` in `Update`. They write only before a timeline
  starts, because the `Q135` gate refuses to start GGRS while durable hydration
  is pending (see DURABLE-HORIZON-CHECKSUM). Do not waive them on the activation
  argument: GGRS start and the restore chain wait on different facts.
- `reconcile_roster_with_frozen_topology` (`game/ambition_app/src/app/versus.rs`,
  in `Update`): writes the versus roster while rollback freezes the seat
  topology.

The exit code means "no new offender", not "clean". `ACKNOWLEDGED` names drift
that is real and the row that owes it. `WAIVERS` carry an argument. A banked
name that the scan stops reporting is fatal.

**Next action:** for each acknowledged offender, either move the write onto the
rewinding schedule (or onto a host intent), or record a measured argument and
move it to `WAIVERS`.

**Known limits of the guard:**

- `Transform` writes are excluded by name. Ruled 2026-09-19 (`Q139`): do not
  grow architecture to satisfy this census. A green says nothing about
  `Transform`.
- A write inside a helper is not attributed to the registered system that calls
  it (for example `reload_ldtk_world_from_disk` under `handle_ldtk_hot_reload`).
  One hop of attribution needs real call resolution. A bare-name match imports
  false pairs, because helper names collapse to `tick`, `apply` and `install`.
- A run condition is not a reachability proof.

⛔ Do not demote `derived`-documented clone registrations on a keyword match. A
component whose presence a query filter reads is authoritative even when its
value is derived. A waiver is a claim about the tree: re-read the function it
cites when you touch it.

**Blocked by:** nothing.

**Acceptance:** the population is every rollback registration, not one
registration spelling; `handle_ldtk_hot_reload` is visible without its waiver
being deleted; a poison that respells a write in any supported param form still
reddens the guard; and the population floor fails when a spelling stops matching.

### SETTINGS-ROLLBACK — finish the settings/mechanics admission boundary

**Owner:** rollback/mechanical-policy owners.

**Current state:** no simulation system reads `UserSettings`
(`scripts/measure_user_settings_in_simulation.py`). The frame-mode half is
closed: `ControlFrame` carries `control_frame_modes`, stamped at capture in
`populate_seat_control_frames`, and GGRS replays it per frame. The damage half is
open: `project_player_damage_policy` writes `PlayerDamagePolicy` from
`UserSettings.gameplay` in `Update`, the policy has no rollback registration, and
three simulation systems read it (`apply_player_hit_events`,
`apply_feature_hit_events`, `charge_projectile_input`). The in-game System
overlay can change `Difficulty`, `Assist` and `PlayerDamage` during a live
timeline, so a resimulation of frame N reads the policy that holds now.

**Ruling (`Q127`, 2026-09-19): deprioritised.** There is no generic
one-dimensional difficulty architecture. Difficulty is game policy expressed as
presets. Participant handicaps and CPU brain levels are separate concepts from
match policy. Spend no substantial effort here until the default game plays
exceptionally well, and do not box the design in.

**Next action (when picked up):** use the `PortalTuning` precedent (the `Q120`
admission protocol). The settings road writes a mirror and proposes in
`MechanicalEditSet::Propose`, and the publisher is the only writer of the
authority. A match-wide policy is admitted at match activation; a
per-participant policy travels with deterministic per-seat input. Do not
reintroduce simulation reads of mutable `UserSettings`. Peers must not have to
share accessibility settings, so input interpretation travels with the input.

**Blocked by:** nothing.

**Acceptance:** rewinding/resimulating frame N observes the policy admitted for
that timeline, not whatever the settings UI contains now; the settings-to-policy
projection remains witnessed end to end; and no `sim`-schedule system takes
either policy resource as a parameter.

### THROW-MODIFIERS — route throws through rage and staleness policy

**Owner:** Smash combat/knockback policy.

**Ruling (`Q133`, 2026-09-19):** for the Smash-like game, follow Smash. Ordinary
scaling throws obey rage, and set knockback keeps its set-knockback semantics.
Rage is game-level combat policy that the engine must be able to express.

**Current state:** the rage half is landed. `ambition_entity_catalog::launch::launch_speed`
owns the rule that a set launch declines rage, and the throw road calls it
(`a_hurt_captor_throws_farther_and_a_set_throw_is_immune`). The CPU duel tapers
with raging throws; its `A_REAL_FIGHT` floor was recalibrated to `0.125` as a
"did a fight happen" check.

The staleness half is open. `apply_capture_throws` applies throw damage directly
and never writes `LandedBodyHit`, and wear is recorded only in
`mark_move_playback_landed_hits` from `LandedBodyHit`. So a throw never records
its own use, and a throw-only move's `occurrences` is always 0. Routing staleness
into the throw's launch alone changes nothing.

**Next action:** decide the mechanic (does a throw stale the throw, or the
grab?). Then record the throw's use on the throw road, then route the read.
⛔ A witness that seeds `BodyStaleMoves` by hand proves the arithmetic only.

**Acceptance:** a controlled throw witness shows the intended rage/staleness
change through the shipped throw road, and a neutral arm proves base authored
throw behavior is unchanged when both modifiers are neutral.

### DUEL-GUARD-RUNG — the CPU duel guard fails at rung 5 on main today

**Owner:** [BRAIN](#brain--finish-truthful-fighter-attack-selection). The
diagnostic half is done: `[dealt]` and the passenger assertion count only damage
dealt to a seat.

**Current state:** `two_cpus_in_the_shipped_composition_damage_each_other` passes
at its default rung 9 and fails at rung 5 (`AMBITION_DUEL_RUNG=5`, about 0.2
against the pair floor). Rung 5's fighters spend most of their damage on
summoned bodies, because the brain answers 71–84% of its decisions with
`grapeshot` and `call_the_shark`. That is a brain selection defect, owned by
BRAIN.

⛔ Do not fix this by lowering the floor or by normalising the metric.

**Acceptance:** the guard passes at all five published rungs at HEAD, because
rung 5's CPUs fight each other.

### A4 — separate control authority from body execution on the real schedule

**Owner:** accepted control writer map and actor-monolith frontier.

**Current state:** the prerequisite writer census is complete and did not find a
competing control authority. The old `PlatformerRuntimeSet` vocabulary is gone; <!-- cite-ok: the DELETED `PlatformerRuntimeSet` vocabulary, named on purpose: a resolvable citation here would mean the deletion did not happen -->
the real realization places body integration inside
`WorldPrepSet::Integrate`. Prior prose that mapped old and new set names by name
is not an implementation guide.

**Next implementation:** size the packet against the actual schedule seam:
control production, accepted control authority, then body execution/integration.
Keep the measured invariant that a body is advanced once per tick.

**Acceptance:** one accepted control fact feeds one body execution road; no
second body tick or hidden writer is introduced; schedule witnesses are placed
between actual neighboring phases rather than only `.after(...)` an abstract set.

### AUTHORITY-POLISH — one owner per mechanical fact, and no mirror in the rollback kernel

**Owner:** the architecture-completion campaign (C11 in
[`consolidation-plan.md`](consolidation/consolidation-plan.md)). Evidence for
`W0xx` rows stays in [`architecture-warts/README.md`](architecture-warts/README.md)
until the row closes. This row is the execution order. It does not displace
ID-PEER, A4 or the rollback rows.

**The question every item answers:** what owns this semantic fact? Target shape:
authoring/ruleset → preparation → one construction or transition → one canonical
live authority → disposable read models → presentation. Prefer deleting the
field, the writer and the reconciler over a stronger sync.

**Current state (2026-09-27):** steps 1–5 of the review order are done. Step 6
was reassessed: `measure_kernel_module_graph.py --scc` gives the 6-module kernel
(`abilities features items projectile session world`) plus
`assets ↔ character_sprites`, and each kernel leg has a recorded verdict in
AP14. The discovery passes then pointed at game expression. Closed items
(receipts in Git history): AP3, AP8, AP10–AP13, AP15, AP17–AP19, AP28–AP34,
AP36, AP37, AP45, AP48, AP54, AP56–AP58, AP62, AP66–AP71, AP76–AP85, AP93,
AP101, AP102, AP104, AP108, AP109, AP128, AP133, AP137, AP140, AP146–AP148.

**Open items:**

| # | Item | Next step |
|---|---|---|
| AP9 | Stale architecture docs (continuous) | Remove closed wart rows as items land. Keep a one-line receipt only where another row depends on it. |
| AP14 | Semantic actor-monolith SCC decomposition (continuous, Jon 2026-09-24) | The residual kernel is the 6-module SCC. Its legs were walked and judged genuine; the one misplaced leg, `abilities→features` (the puppy-slug gun asks the actor domain to spawn a minion), is not inverted only for the graph. Open question: is the runtime-mint description in `session→items` (`MintedItemBaseline`, `OwnedItemsBaseline`, `ItemCheckpointRestoreInputs`) item knowledge or occurrence-lifecycle knowledge? When an owner is clear, move state, behavior and installation together and delete the old edge (no callbacks, no compatibility re-exports). After each migration run `python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80`. See [`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md) and [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md). |
| AP147 | A hold writes the held body's `gravity_scale` and restores a copy | Mount and capture halves are done: the frame resolver gives a body with `PoseOwnedExternally` or `CapturedBy` zero pull, and the saved copies are deleted. Handoff: the Gnu-ton conductor's fists (`game/ambition_content/src/bosses/gnu_ton/conductor.rs`) still write `0.0` while posed and `1.0` on release. Before you move them onto `PoseOwnedExternally`, measure whether fists take tumbling launches, because the kernel stages a tumbling launch until the hold ends. |

`WorldTime` is recomputed from `Time.delta` × `ClockState.time_scale` at the
head of each step. That is the canonical clock, not a defect.

**Acceptance for the lane:** no mechanical fact has two mutable canonical
owners; rollback rows are authorities, not projections; construction publishes
no plausible-but-incomplete object; required mechanical policy does not fail
open. Close with a fresh census rather than a checked list.

## P1 — ownership, composition and iteration

### GATE-PER-ACTOR — a body/capability gate is solid or open for each actor

**Owner:** [`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md)
jointly with the gated-wall road (`gated_lock_walls.rs`, the per-room collision
overlay).

**Ruling (`Q54`, 2026-10-01, [`maintainer-decisions.md`](maintainer-decisions.md)):**
the gate is evaluated per actor. Alice in Phase Boots passes a phase wall; Bob
without them collides with it.

**Current state (landed 2026-10-01):** a condition can publish a subject form
(`SubjectConditionEvaluator`, `ConditionCatalog::ask_for`); `body.can` and
`body.fits` publish one. A gated wall with a subject form always stands in its
room's `gate_solids`, and the publisher writes a `GatePass` for each body that
satisfies it. Body steps read `ComposedRooms::solids_for(collision, room, body)`.
A projectile, a dropped item and any reader that names no body meet the wall as
solid. An undriven body is asked as itself. The crouch/morph clearance check
still meets the wall as solid. A wall gated on a population fact
(`world.flag_set`, `inventory.holds`) has one answer for every body. Witnesses:
`a_body_gate_is_open_only_for_the_bodies_that_satisfy_it` and
`a_gate_open_for_one_body_is_missing_only_from_that_body_s_walls`.

**Open:** perception and AI path decisions (the decide pass in `update.rs`) read
the shared walls, so an NPC that can pass a wall does not yet plan through it.

**Next action:** make the brain's path decisions read the walls for that body.

**Acceptance:** in one live room, a body that satisfies a wall's body condition
passes and a body that does not collides, in the same tick; a projectile and an
undriven body follow the same per-actor rule stated for them; witnessed with
two seats and with a control where both qualify; and an NPC that can pass a
gated wall plans through it.

### BOSS-REPLAY-RETRACTION — a replay that un-defeats a boss un-defeats it for every family

**Owner:** the generic boss-progress road (`crates/ambition_boss_encounter`)
jointly with the save's replay policy
(`crates/ambition_persistence/src/save_data.rs`,
`every_durable_family_says_whether_a_replay_retracts_it`).

**Ruling (`Q51` and `Q56`, 2026-10-01, [`maintainer-decisions.md`](maintainer-decisions.md)):**
if a replay or rewind makes a boss undefeated again, the consequences its defeat
created after that point are undone too, for every boss family. A rollback
boundary that does not yet enforce this is a known issue, not an open question.

**Current state (landed 2026-10-01):** `BossDefeatsSinceCheckpoint`
(`ambition_boss_encounter::retraction`, rollback state
`boss.defeats_since_checkpoint`) records each placement cleared since the last
committed checkpoint. `retract_boss_defeats_on_replay` takes the entries of the
replay's live room, puts each placement back to `Untouched`, despawns its
unopened reward chest and announces `BossDefeatRetracted`. The item domain
(`retract_mints_of_retracted_boss_defeats`) despawns the mints whose parent is
that boss and retracts their ledger rows. `reset_cut_rope_attempt_on_replay`
is now only the "try again" re-fight road, keyed by the replay's live room.
Witnesses are in `game/ambition_app/tests/boss_replay_retraction.rs`.

**Known issues (open under the `Q51` ruling):**

- A replay does not take back coins the defeat put in `BodyWallet`.
- An ability or item the defeat granted into `OwnedItems` stays after a manual replay (a death restores the bag; "try again" and a reset-key replay do not).
- An opened reward chest stays opened.
- `QuestAdvanceEvent::BossDefeated` progress stays: quest progress is keyed by archetype and has no baseline.
- A death in a room other than the boss's retracts nothing in the boss's room until that room is replayed.
- A "try again" that the lifecycle refuses leaves the re-fight latched until the next admitted replay of that room.

**Next action:** take the known issues in order, each with a witness and a
control (a consequence from before the baseline survives the replay).

**Acceptance:** ✅ one generic retraction on `RoomReplayAdmitted`, keyed by the
replay's live room, for every boss family, with the cut-rope special case
deleted into it, the minted reward retracted with it, a witness per family shape
and a control. Open: every consequence in the known-issues list is retracted or
explicitly ruled out of scope.

### MENU-OVER-DIALOGUE — an overlay opened during a conversation must not end it

**Owner:** `crates/ambition_dialog/src/systems.rs` (dialogue input) jointly
with the menu input owner (`crates/ambition_input/src/menu.rs`,
`MenuControlFrame`).

**Ruling (`Q75`, 2026-10-01, [`maintainer-decisions.md`](maintainer-decisions.md)):**
pause, map and inventory may open during dialogue. The dialogue stays live
underneath without navigation input. Map and inventory are mutually exclusive
primary overlays.

**Current state (re-checked 2026-10-02):** `apply_dialog_menu_input` closes the
conversation on `menu.back || menu.start`, so the Start press that opens the
pause menu also ends the dialogue. The readers of `MenuControlFrame`
(`dialog_input`, the map's `input.rs`, the grid and kaleidoscope menus) have no
ordering or focus between them, so one press can reach the dialogue and an
overlay in the same frame.

**Acceptance:** a Start press during a conversation opens the pause menu and the
conversation is still live, at the same line, when the menu closes; while an
overlay is open, the dialogue reads no navigation; opening the map while the
inventory is open (and the reverse) is refused or swaps, by one stated rule,
with a witness for each.

### CANDIDATE-GENERATION-ORDER — a candidate session is prepared from the generation before its own activation

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) (content
reload) jointly with the session-lifecycle owner,
`crates/ambition_platformer2d_provider/src/lifecycle.rs`.

**Current shape:** live session A stays on published generation N. Candidate B
is built and verified from transaction-local N+1, which
`PendingGenerationInputs` (`ambition_platformer2d_runtime/src/content_identity.rs`)
hands to the transaction's own preparation, keyed by its `load_id` claim.
Adoption makes B/N+1 authoritative atomically. Construction runs
`.before(AmbitionGameShellSet::Pending)` and the content commit runs after it, so
the channel, not an ordering edge, is how a candidate sees N+1. The channel
carries the admitted cast (`characters`), the boss catalog (`bosses`) and the
character catalog (`catalog`). The fighter ladder and the encounter waves are
standing projections that converge after the commit, so they are not frozen.
Guard: `the_candidate_is_built_before_the_router_advances_and_providers_only_adopts`.

**Next action:** bring each remaining construction input that a candidate must
see at N+1 into the channel. A boss's HP, phase triggers, death seconds, music
and reward seed still come from the App catalog (see I2/I3).

⛔ Not by an ordering edge and not by re-fingerprinting. Do not reopen A10.5's
guarantee that a candidate that cannot be built never retires the live session.
This is engineering, not a maintainer question.

**Blocked by:** nothing.

**Acceptance:** every generation input a candidate must see at N+1 reaches it
through the transaction-local channel rather than through global publication,
and a value that is a standing projection is stated as one rather than frozen.

### I2/I3 — finish independent content authoring and safe reload

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) and content
reload/preparation owners. Implementation state:
[I3](engine/fast-iteration-implementation.md#i3---coordinated-generation-publication-and-local-reload)
and [I4](engine/fast-iteration-implementation.md#i4---small-procedural-sdk-and-one-native-semantic-reference).

**Current state:** a prebuilt host loads edited content without Cargo. Reload
has explicit outcomes, a no-op revision does not advance the generation, and a
stale attempt carries the generation it was prepared against. A build that reads
content off disk watches the pack's sources (`ambition_content::content_watch`)
and reloads on save. Participating families include moves, boss profiles and
encounters, the character catalog, fighter facets, the boss seed library,
validator bands, items and audio registries. The procedural tier (I4) runs
technique, boss-special and wielded-item modules on the linked and WASM roads,
with hot reload through the mechanical-edit protocol.

**Rulings (2026-09-19):** `Q110`: mechanical registry changes use explicit
lifecycle/replacement semantics; do not invent a universal silent overwrite.
`Q104`: content-authored movesets are the long-term authority, and duplicate
Rust move tables are migration scaffolding.

**Open work:**

- Converge the remaining reloadable registries on one prepare/admit/publish contract.
- A boss's HP, phase triggers, death seconds, music and reward seed come from the App catalog, not the frozen generation.
- I4: save eligibility; ports for body motion so the remaining wielded items (dive, blink, grapple, mark/recall) can become modules; GNU-ton's conductor as a module.

**Blocked by:** nothing.

**Acceptance:** changed content publishes exactly once under a new admitted
generation; identical content is a no-op; stale work refuses rather than folding
against a generation it did not read; no runtime road silently falls back to a
second authoring source.

### A9 — establish truthful minimal engine profiles

**Owner:** public SDK/composition architecture.

**Current state:** capability-footprint and absence-contract tooling measure
what a profile links and installs. The three composition probes in
`composes_through_the_sdk` pin the fixed step and assert that `FixedUpdate` ran.
A profile witness must assert that it stepped, not only that it built.

**Ruling (2026-09-19, `Q100`, `Q106`, `Q108`, `Q97`, with `Q146` and `Q144`):**
capabilities are optional and composable. A capability that authored production
content requires and the composition lacks must refuse that content or its
admission. Reduced tools and tests may omit capabilities explicitly. The ruling
says: implement this architecture rather than continuing to census hypothetical
composition variants.

**Next implementation:** encode supported profiles as named capability
contracts, then make construction/step witnesses and absence guards test those
contracts.

**Blocked by:** nothing.

**Acceptance:** each supported profile constructs and steps a real subject; its
promised absent capabilities are absent from installation and resolved dependency
closure; the full Ambition composition remains intact.

### A7 — make item occurrence authority explicit where it carries a real invariant

**Owner:** [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).

**Current state:** `GroundItem` construction is sealed, but that seal does not own
occurrence creation. Death drops, match spawns and other roads still mint
identity/provenance/custody facts at their occurrence sites. Two mint sites
degrade on purpose and belong to the `ItemCustody` inventory leg:
`ambition_held_items`'s thrown-item mint and `puppy_slug_gun`'s minion mint.

**Next implementation:** centralize only the occurrence decisions that share an
actual invariant (identity, custody, provenance, rollback ownership). Do not add a
generic request bus merely to reduce writer count.

**Acceptance:** reward policy consumes accepted occurrence outcomes and cannot
become an alternative minting authority; every remaining occurrence creator has
an explicit ownership reason.

### BRAIN — finish truthful fighter attack selection

**Owner:** [`engine/fighter-brain.md`](engine/fighter-brain.md), which holds the
modeling rules and the evaluation-rig contract.

**Current state:** the option layer prices what a move can do from one combat
model:

- A move's hittable region (`MoveFrameData::coverage`) and its push region (`push_coverage`) are separate. A pure shove is priced by `Features::displacement_value` (push coverage × the foe's proximity to a blast line × whether the push sends them that way).
- Admission is absolute: the opponent must lie inside the move's own region, with `ADMISSION_SLACK_PX` on each side. Admission leads the foe over `Perceived::staleness_s()` plus startup. Scoring stays on the observed position, so `reaction_ms` stays the difficulty axis.
- The brain and the hit resolver spend one launch law, `ambition_entity_catalog::launch::launch_speed`. A hazard is a value (`MoveHazard`, `ThreatTravel` in `ambition_entity_catalog/src/hazard.rs`), and an unresolvable ranged request is no offer.
- The shipped ladder's weights still order the rungs at least as well as any single-weight change tried (2026-09-21 refit sweep), so do not refit them.

Instrument: the `#[ignore]`d sweep `every_fighter_on_the_grid_can_fight_its_mirror`
(21 mirror matches of 3600 ticks; `AMBITION_GRID_TRACE` for one fighter). Run it
before and after a change, on one binary, and report both columns.

**Open roads, in order:**

1. **A brain cannot decline to attack.** The decision takes `options.attacks.first()` whenever the body is free, so the move that survives at range is thrown until the world changes. The brain does not remember its own last move. A decline gate on decision ticks was built and was a no-op, because presses are bounded by move duration (24 ticks), not the decision cadence (5). Next: a refusal that lasts as long as the move would have (`frames.total_s`), which needs one more piece of brain state and its snapshot projection.
2. **A counter and a buff cannot be chosen between.** Pricing them needs a defensive feature ("is the opponent committed to a swing"), not a wider admission rule.
3. **Rung 5's CPUs do not fight each other** (DUEL-GUARD-RUNG).
4. **`sanic` @5 leaves the stage early** (about 640–780 ticks, both seats airborne). That is a stage defect, not the one-move lock. The guard for it is a bout that ends early, not move variety.
5. **A ranged move scores `launch: 0`.** A projectile hit writes a dimensionless `HitKnockbackMagnitude::FeelScale(0.85)`, not `LaunchSpeed`. Settle whether the feel reference belongs on `LaunchConditions`, whether `0.85` leaves the projectile stepper, and what `max_knockback` means for a launcher. A launcher also needs a hazard-coverage feature (its `reach_fit` is zero at every range).
6. **A placed trap has a position, and `reach` is a radius.** Carry the dangerous region relative to the body at the resolved-offer seam, not another reach scalar.
7. **A teleport's destination does not reach the brain** (`TeleportParams`: `behind_nearest_foe`, `behind_gap`, aim). It belongs in the same resolved offer.
8. **The ladder's step per rung.** All shipped rungs author `rollout_depth: 0`, so `read_weight` is inert (`Q90`) and the L3 step the engine ladder takes at level 6 is missing. Measure `--rungs 3,4,6,7,8` to separate step size from one pair.

**Standing prohibitions:**

- Do not fix the lock with a second, softer admission rule or a `reach_fit` floor.
- Do not feed push coverage back into `reach_fit`.
- Do not patch the evaluator with a fighter-specific exception. Keep press generation separate from move utility.
- A two-move fixture cannot price a move against a kit. Pin a feature against the kit `attack_kit_of` builds.
- A fixture that shares an app with a live CPU measures the CPU too.

**Acceptance:** representative CPUs select movement-compatible and
movement-transition attacks from their authored menu across the intended
difficulty ladder, with no regression to the press/move identity contract.

### D72 — continue Smash parity from the inventory

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

**Current state:** the inventory is the source of feature/parity truth. Do not
turn this queue row back into a chronological parity diary.

**Next implementation:** take the next inventory row whose policy is settled,
implement it on the production path, update that inventory row and add the
production acceptance witness.

**Blocked where applicable by:** [Q62](awaiting-maintainer-decision.md#q62--keep-or-discard-the-epoch-captured-4741-line-mary_oldtk-delta),
[Q89](awaiting-maintainer-decision.md#q89--what-special-should-each-robot-stand-in-have),
and other product rows named by the inventory.

### D166 — make character authoring boundaries load-bearing

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

**Next implementation:** for each remaining duplicated authored/runtime value,
choose one authoring owner and make every runtime representation a projection or
admitted prepared value. Prefer deleting the second truth to synchronizing it.

**Acceptance:** the owner document can name one authoritative authored value for
each migrated fact, and production consumers cannot bypass its preparation or
projection boundary.

### ROLLBACK-DEAD-SESSION — an invalidated GGRS session stops the clock in silence

**Owner:** the simulation harness (`crates/ambition_sim_harness/src/runtime.rs`).

**Failure:** a sync-test session that invalidates keeps accepting `sim.step()`
and stops advancing `SimTick`. Nothing panics, and every later assertion runs
over a frozen world. The usual cause is a system that writes a
rollback-registered resource outside its sanctioned road, which desyncs the sync
test.

**Ruling (`Q138`, 2026-09-19):** an invalidated harness must refuse or fail
rather than silently produce frozen observations.

**Current state (re-checked 2026-10-02):** `Platformer2dSimHarness::step` does
not consult `rollback_health()` yet. Meanwhile
`scripts/a_rollback_arm_must_refuse_a_frozen_world.py` (a `--maintenance` job)
requires each sync-test arm to read the health API or to state what a frozen
world breaks in it. It prints the current census. The example
`game/ambition_app/examples/hall_bench.rs` consumes a frozen world silently; it
is a benchmark, not a test.

**Next action:** make the harness step methods consult `rollback_health()` and
refuse on an invalidated session.

⛔ Do not add `rollback_health()` to an arm whose own assertions already fail on
a frozen world, only to raise a count.

**Blocked by:** nothing.

**Acceptance:** stepping an invalidated session fails with the session's error,
witnessed by an arm that invalidates a session on purpose.

### DURABLE-HORIZON-CHECKSUM — the save mirrors write hashed state from `Update`

**Owner:** `ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs`.

**Current state:** every writer of hashed save state runs in the simulation
schedule: the three `persist_*_to_save` mirrors and
`count_the_dialogue_visit_when_a_conversation_opens`.
`resources_crossing_the_rewind_boundary.py` reports that `AmbitionGameSave` does
not cross the rewind boundary. `AuthoredOccurrences` is rollback state with a
peer-stable checksum. The restore chain's three `Update` residents
(`adopt_occurrence_checkpoint_from_save`, `restore_inventory_from_save`,
`complete_durable_restore`) write only while `SaveRestored` is false, and the
`Q135` gate keeps GGRS from starting while `durable_hydration_is_pending`. The
gate and the chain ask one population question (`bodies.single().is_err()`),
held by `a_population_the_restore_cannot_complete_on_is_written_to_by_nobody`.
The chain's two offenders stay acknowledged in ROLLBACK-MUTATOR-POPULATION.

**What is left:** `Q129` (must the save file be part of what two peers agree
on?). It is open and does not block this row. Whether that closes the row is a
maintainer call.

**Acceptance:** Q129 is answered and the three mirrors follow the ruling; ✅ the
dialog increment has its own answer, which was not the mirrors'; ✅ the one-shot
pair's ordering against GGRS start is characterised rather than assumed; and ✅
Q135 is answered and the restore chain's three `Update` residents have their
road — the session-start gate, plus the population fix that makes the gate's
promise hold for every population.

### TEST-LANES — keep required test lanes executable

**Owner:** test runner / app integration lane. Operational rules and what a
green lane does not clear:
[`running-the-heavy-app-it-lane.md`](../recipes/running-the-heavy-app-it-lane.md).
How a check can fail to run: [`checks-that-did-not-run.md`](../recipes/checks-that-did-not-run.md).

**Current state:** the `app_it` lane runs (`713 passed / 0 failed / 45 ignored`
on 2026-09-17). Cargo diagnostics are read through
`scripts/lib/cargo_output.py`, which disables colour and strips ANSI codes, and
`scripts/tests/test_cargo_diagnostics_are_read_plain.py` holds every script that
reads cargo output to it.

**Open items:**

1. **The compile-cost ratchet fails the full gate** (`scripts/compile_ratchet.py`, measured 2026-09-18). Its baseline records commit `b3bd00a4a` (2026-09-05), which no ref reaches, and it disagrees with itself in three places. Over budget: `ambition_platformer2d_actor_monolith`'s largest unit (100,742 → 115,105 lines) and edit cost, and `ambition_geometry`'s worst edit cost (94.9% of the workspace). ⛔ Do not re-freeze to go green. Next: find which part of the monolith's largest unit belongs in its own crate, and repair the baseline's self-disagreement before any deliberate re-freeze. <!-- cite-ok: `b3bd00a4a` is quoted BECAUSE it resolves nowhere; it is `dev/compile_ratchet_baseline.json`'s own recorded `commit` field -->
2. **An arm fails only in company** (see [the triage page](triage/a-composition-acceptance-that-only-fails-in-company.md)): `composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps` failed once on 2026-09-10, and its assertion was never captured. Three other instances of the signature were per-arm measurements reading process-global state (`app_it` runs arms as threads of one process); they are fixed, and `scripts/a_test_static_is_a_channel_between_arms.py` guards the class. Next: capture this arm's assertion. The next candidate of the class is `hall_redecode_census.rs`, which asserts over a delta of a process-wide counter. ⛔ Do not add a retry.
3. **One older session-root handoff failure** did not reproduce in four full runs, and its assertion was never captured. Both candidate arms (`the_shipped_app_never_holds_two_session_roots_across_a_handoff`, `a_candidate_session_replaced_while_pending_is_discarded`) assert their own premises, so a new failure carries its cause. The next step is not more runs.

The published-sheet floor in `ambition_sprite_sheet` (780 below a floor of 800
on one checkout) is machine state. ⛔ Do not lower the floor.

**Acceptance:** the failing population is reproducible or explicitly classified,
and the production cause is fixed or the harness proves why the failure is not a
production invariant.

## Receipts

Closed rows that an open row, a script or an inbound link still names.

### NEW-GAME-RESYNC — a New Game after durable hydration fails the sync test — ✅ DONE 2026-09-29

A New Game is a checkpoint resume to the fresh baseline at the start room, on
the confirmed-frame road the death restore takes (`resume_at_checkpoint_on_reset`
reads `NewGameRequested`; the fresh-run reducers run inside
`CheckpointDomainApply`). ⛔ Do not register the four `derived` body types for
rollback to hide a rebuild on a speculative frame. Witness:
`a_new_game_asked_for_by_the_host_commits_once_under_a_rewind`.

### A10 — candidate world / last-good-world publication — ✅ DONE, DEMOLITION CLOSED 2026-09-16

A failed candidate world leaves world N playable and unchanged, and N+1 is
verified before N is retired. Every road that changes the authoritative world
crosses its own publication verdict. Owner and witnesses:
[construction and reconstitution](engine/construction-and-reconstitution.md).
⛔ Not part of A10: peer-stable identity (ID-PEER), the defensive
`DepartureAuthority::Custodian` fallback, and the refused-door player signal.

### CUTSCENE-ROLLBACK-DECISION — two session-scoped cutscene values cross into simulation with no rollback decision — ✅ DONE 2026-09-28

The cutscene dismiss and skip are seat intents on the `ControlFrame`, which
`tick_active_cutscene` reads from `SlotControls`, and `CutsceneSkipHold` is
rollback state. The host-side `Update` request is deleted. Guards:
`a_cutscene_dismiss_on_the_seats_input_survives_the_rewind_once` and
`a_cutscene_skip_is_a_hold_of_the_seats_cancel`.

### ROLLBACK-KIND-SPELLING — one registration, one kind, spelled once — ✅ DONE 2026-09-16

`ambition_platformer2d_core::rollback_kind::spelling` holds every (kind,
sentence) pair once, and both registration roads reference it. Guard:
`scripts/check_rollback_kind_spelled_once.py`.

### A12 — finish move-contact attribution and reflection identity

✅ DONE 2026-09-19. A late projectile or melee outcome, a reflected shot and an
ability contact cannot credit the wrong move occurrence. An outcome that names
another occurrence, or none, credits no move (ruled: an ability contact is
independent by default). Witnesses: `an_outcome_naming_another_occurrence_credits_no_move`,
`an_unclaimed_outcome_credits_no_move`,
`a_move_occurrence_reaches_the_same_number_with_and_without_a_rewind`. ⛔ The
frontier's `A12` (flow validation) is a different subject.

### ROLLBACK-BAG-DESYNC — `AmbitionGameSave` disagrees with its own rollback replay — ✅ REPAIRED 2026-09-16, acceptance MET; the authority/representation split is DEFERRED and Q129 is open

The three live→save mirrors ran in `Update` and wrote a hashed resource that a
replay did not re-derive. They register through `app.sim_schedule()` now. Guards:
`no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves` and
`the_saves_hashed_snapshot_tracks_the_frames_it_is_compared_at`. ⛔ Do not remove
`AmbitionGameSave` from the checksum: most of its writers run inside rewinding
schedules. Deferred: the authority/representation split. Open: `Q129`.

### MENU-RESET-MIDSESSION — the menu writes rollback state from `Update` — CLOSED 2026-09-19

Menu presses reach the simulation through `HostIntentLedger<M>`
(`crates/ambition_platformer2d_actor_monolith/src/session/host_intents.rs`),
released at the head of the stamped tick in every pass. Ruled (`Q140`): one frame of stale UI is acceptable. ⛔ Do not add
duplicate authoritative inventory state or optimistic reconciliation to hide it.
Witness: `a_health_cell_used_from_the_menu_heals_once_and_spends_one_cell`.
