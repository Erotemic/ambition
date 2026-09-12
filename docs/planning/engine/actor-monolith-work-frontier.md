# Ownership migration packets

**Baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`, 2026-09-08.
**Authority:** [reassessment](architecture-reassessment.md),
[responsibility map](architecture-responsibility-map.md),
[decomposition rules](actor-monolith-decomposition.md).
**Priority is owned only by [the queue](../queue.md).** This file is a packet
catalog, not a second queue. A packet marked HOLD is not an implementation order.

⛔⛤ **A DISCHARGED HOLD IS REWRITTEN IN PLACE, NOT ANNOUNCED ABOVE ITSELF.** Four
rows discharged a hold and four chose a different spelling for it; **two of them
(A4, A7) added a DELIVERED banner at the top and left the `**HOLD:**` sentence
twenty lines below stating the same condition as live.** A reader who scrolls to
the hold — which is the sentence that decides whether to start — is told to wait
for something the row's own first paragraph says arrived. ⇒ When a hold is
discharged, edit the `**HOLD:**` line itself to name what discharged it and when;
a banner is a second copy of that fact, and the second copy is the one that
rots.

The old mandatory P2/P3/P4 sequence is retired. P1 settlement and the corrected
spawn extraction remain landed facts. A1-A12 name responsibilities, not SCC
scores. Proposed file/test/API names below are design targets, not existing code.

## Dependency and readiness map

```text
fresh source/behavior preflight for every packet
    |
    +-- A1a checkpoint admission characterization/repair (DONE 2026-09-08)
    |       -> A1b checkpoint restoration ownership (DONE 2026-09-08)
    |           -> A1c selected checkpoint through common commit (DONE; matrix closed 2026-09-08)
    |               -> A7 item horizon/custody separation (inventory DELIVERED 2026-09-10)
    |
    +-- A2a shared boss geometry -> A2b world obstruction -> A2c contact seam
    |                                                       -> A5 destructibles
    |                                                          (both halves DISCHARGED 2026-09-11)
    |
    +-- A3 construction placement adapter (independent; audit shared file edits)
    +-- A4 accepted control/body execution (writer map DELIVERED 2026-09-10)
    +-- A6 definitions / materialization (field census DELIVERED 2026-09-10)
    +-- A9 minimal-profile baseline now; closure changes by proven owner
    +-- A8 two-instance scope proof (customer established; OW1/FI9 first)
    +-- A10 bounded candidate construction (I3b reload customer; FI4)
    +-- A12a raw flow validation (independent)
    +-- A11a installed support -> A11b exhaustive references
            -> A12b checked runtime -> A11c explicit activation
```

A3 and A9 measurements need not wait for A1. Do not implement A2c on top of known
geometry disagreement. A5 follows the resolved-contact contract because moving
all destructible state first would preserve an incorrect split interpretation.
There is no requirement to split accepted control across crates to make A4 green.

## Relationship to fast iteration

The [extension packet catalog](fast-iteration-implementation.md) is the bounded
continuation for pure authoring, portable artifacts and procedural state. It uses
A6's field census, A9's resolved-closure method and A11/A12's existing admission.
Only specific domain ports wait on A2/A4 contracts. A8 now starts with the
long-term world's two-instance proof; A10 supplies I3b's bounded safe reconstruction.
Neither blocks I1/I2, and arbitrary-world undo is not the requirement. Do not expand I1's
pure helper move into an actor SCC extraction. The extension catalog and this
catalog both take priority from [the queue](../queue.md).

## Before any production edit

Record HEAD, working tree, ownership claim, callers, state writer(s), lifetime,
schedule phase/gates and current behavioral witness. On a newer base, re-read the
actual symbols; line numbers in this review are locators, not patch coordinates.

```bash
git rev-parse HEAD
git status --short
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
python3 -m pytest -q scripts/tests/test_actor_spawn_boundary.py
```

Use `rg` for candidate references, then inspect imports/re-exports, installers,
queries and registration. The textual module graph misses cross-crate and
implicit scheduling/state dependencies. Do not call a regex search a complete
compiler dependency graph.

Separate a semantic fix from a behavior-preserving ownership move into different
commits. Avoid compatibility re-exports under old internal paths. Do not broaden
the packet because unrelated lint, naming or policy work is nearby.

⛔⛤ **A LINE HERE USED TO READ "Each commit must build/test at its own boundary",
AND IT CONTRADICTED `AGENTS.md`.** The repository's own rule is *"`./run_tests.sh`
is the broad repository test backbone. **Prefer narrower checks when they cover the
touched invariant**"*, with the omission stated out loud. The sentence above was
read as "run the whole lane per commit" and cost most of a day on 2026-09-11 —
seven full `--rust` runs at ~25 minutes each where a `-p <crate>` run covered the
change. **`AGENTS.md` is the authority on test scope; this document does not
restate it**, and [the cheapest sufficient check](../../recipes/cheapest-sufficient-check.md)
already carries the full argument in Jon's own words: *"local targeted reruns only,
and then we DON'T run the entire thing again after."*

⚠ **A REVIEW ASKED FOR THE "each commit is independently valid" HALF TO COME BACK
under cheapest-sufficient wording. DECLINED.** Jon named that sentence for removal
by its own words, and any rewording of "each commit must build" re-imposes what he
struck. Commit hygiene that is NOT about verification breadth is already stated
above: separate a semantic fix from an ownership move, and do not broaden a packet.

## A1. Checkpoint restoration belongs to session lifecycle

**A1's acceptance matrix is complete: all seventeen rows have a witness that
fails for that row's property** (per-row receipts in the protocol). The
prefetched-plan arm closed on `a_checkpoint_outlook_refuses_a_plan_prepared_
without_one`, whose control arm found that the construction-plan prefetch cache
had never promoted anything — its identity had two keepers and only the host's
copy was ever set.

⛔ **CLOSURE STILL OWES THE ROLLBACK BOUNDARY OF THE TERMINAL ROAD.** A review on
2026-09-08 found that the host-side abandonment bridge — the note a failed
preparation leaves for a commit executor — crossed into rollback state before
both the session-ownership gate and confirmation. That is repaired and witnessed
(`an_abandoned_operation_waits_for_its_admitted_frame_to_be_confirmed`,
`a_note_from_a_rewound_branch_cannot_end_the_operation_that_reused_its_key`, both
poison-verified), and the note now names its operation by value because a rewound
sequence counter can mint the same key twice. ⚠ **The P2P road stays inert**: the
terminalization sits below `commit_confirmed_lifecycle`'s `LocalSyncTest` gate,
because ending an operation on a local asset failure is a lifecycle decision only
a host that owns that decision may make. A coordinated peer-level rule is a new
packet, not an A1 row.
A1c closes the cross-domain reset-admission defect. **Normative owner:**
[checkpoint restoration protocol](checkpoint-restoration-protocol.md). Follow its
state machine, source/destination table, commit ordering and acceptance matrix;
this synopsis is not an alternative implementation recipe.

The protocol fixes one semantic authority: selected checkpoint, original subject,
room reconstruction and participating domain snapshots must describe one admitted
operation. F1 covers the startup latch. F9 establishes that occurrence/custody/
owned-item consumers currently restore from the raw reset message even when room
admission can be refused. A move of shrine code alone cannot close F9.

**A1a — DONE 2026-09-08.** Startup routed progress now advances only on accepted
admission; occupied-slot, missing-subject and dedup fixtures are in
`shrine/tests.rs`. The denied-reset witness ran against the full
checkpoint-horizon composition and is stronger than the packet assumed: a refused
reset **destroys** an object acquired after the checkpoint, because custody
restoration and room reconstruction sit on opposite sides of the admission. A1c
therefore owes a live-entity assertion, not only ledger equality — see the
protocol's measured table.

**A1b — DONE 2026-09-08.** Restoration state, systems, installation, tests and
the rollback registration moved from `shrine`/item-pickup to
`session::checkpoint`; healing/capture stayed in `shrine`, the slot stayed in
`session`, the wire key stayed the same. `ActorCheckpointHorizonPlugin` now
composes a session offer and an item offer, and a checkpoint-only composition
resumes with neither held items nor a shrine entity. The carved system's schedule
edges are preserved verbatim and guarded from the item-pickup side; they are
inherited, not derived — see the protocol.

**A1c/1-2 — DONE 2026-09-08.** The raw domain restore readers are gone: one
admitted operation, published only with an `Admission` in hand, is what every
domain reducer reads. A refused reset changes no occurrence, custody or
owned-item state and is remembered rather than dropped. Wire format 167 -> 168.

**A1c/3a — DONE 2026-09-08.** Fresh and prefetched room preparation read the
accepted operation's pinned population, matched by intent, instead of a live
ledger the restore had swapped in order to be read. Wire format 168 -> 169.

**A1c/3b — DONE 2026-09-08.** Domain application moved onto the common
authorized commit path, run by both executors from installed inputs that are
removed on every path. The transitional admission token was deleted.

**A1c/4-5 — DONE 2026-09-08.** Post-apply verification against the accepted
snapshots, one terminal outcome per operation key, fail-closed gameplay blocking,
and startup completion folded into the same operation model
(`CheckpointResumeProgress` deleted). <!-- cite-ok: the row RECORDS the deletion --> What remains is widening verification
beyond the ledger, the bag and custody presence. Include
checkpoint replay consequences, deferred flushes, verification and the final
rollback baseline. Pin typed immutable checkpoint data on admission at the point
deferred application makes it load-bearing. No live-ledger swap to prepare a
candidate and no generic snapshot/restore registry. A trusted failure after
destructive application is fail-closed publication, not proof that arbitrary
Commands were undone.

Do not ship mixed raw-request and selected-candidate restoration in one profile.
A1 is complete only after busy-slot zero-mutation, candidate-prefetch coherence,
no-item composition and eager/confirmed restoration fixtures pass. No SCC number
is an acceptance condition.

Suggested focused commands remain:

```bash
cargo test -p ambition_platformer2d_actor_monolith --lib checkpoint
cargo test -p ambition_platformer2d_actor_monolith --lib shrine
cargo test -p ambition_platformer2d_runtime --lib checkpoint
python3 -m pytest -q scripts/tests/test_actor_spawn_boundary.py
```

Use the existing app integration harness for canonical reconstitution, carried
items and death restoration; check the actual selected test count. These commands
are implementation-agent obligations, not executions claimed by this review.

## A2. Establish coherent projectile contacts before removing family knowledge

**Ready:** A2a geometry and A2b obstruction characterization; A2c after those
contracts. **Normative owner:** [projectile contact protocol](projectile-contact-protocol.md).
It supplies the ownership table, exact sampling/ordering, response table, source
anchors and acceptance cases. Do not substitute a marker-only extraction.

**A2a:** one published authored/fallback/empty target geometry, including bosses
and the first eligible tick. Do not recompute a boss hull independently at
preflight and application.

**A2b:** use actual travel legs and finite-shape contact order against the same
world policy. Preserve ProjectileSeq ordering. World-object colliders carry
contributor identity so a solid destructible produces a compound contact rather
than becoming immune behind its own wall. The first sweep supports projectiles
against sampled stationary targets; full relative-motion/rotational CCD remains
outside this packet's claim.

**A2c:** direct contact names its recipient once. Interception and terminal flight
response happen during stepping; ordinary damage resolves in its existing later
phase from the targeted witness. No family re-query, same-tick recursive flow
execution or second area-style interpretation. Move pure flight helpers to
`ambition_projectiles`; contact integration can remain in a coherent monolith
module until receiver inputs justify a package boundary. Runtime owns ordering,
not contact algorithms.

Delete projectile uses of family predicates and UnresolvedFeatures after their
replacement is exercised. Other melee/area callers have separate scope. Preserve
return-leg hit memory, one-target-per-step returning behavior, splash inclusion,
allegiance and one-area-event cardinality. Land receiver-neutral return survival
and direct-before-splash ordering as the protocol's explicit semantic corrections,
with fixtures separate from the final adapter/file move.

The protocol's destruction/surface identity and single-transition tests are A5
prerequisites. They do not authorize merging every world object into one generic
feature or breakable crate.

## A3. Move authored placement lowering to the construction boundary

**PREFLIGHT DONE 2026-09-09, and it changed what this packet is.** Measured at
HEAD: `ambition_platformer2d_world` names no actor crate in its manifest and its
`LoweringCtx<C>` is already generic, so the CRATE-level edge this packet reads as
its headline was gone before the work started. Inside the monolith's `src/world/`
region the coupling is two files — `placements.rs` (119 lines: the context struct
and three aliases) and `rooms/stage.rs` (two production signatures) — and the
"lowering functions" below are already under `features/ecs/spawn/**`.

**What the preflight found instead landed the same day:** the character catalog
and the authored sheets were a SECOND CARRIER for authorities that already
travelled on `ActorConstructionContext`, threaded as bare positional arguments
through three signatures to be assembled into an `ActorPlacementContext` beside
three fields that arrived on the context. They ride on the context now.
`SimulationSetup` shed both fields with them. See the queue row for the
measurement and the poison.

**Ready:** focused preflight on the new HEAD. This preserves old P4's supported
ownership diagnosis. **Problem:** the world region hosts actor-specific catalogs,
prepared-character/materialization inputs and lowering functions.

**Source:** `crates/ambition_platformer2d_actor_monolith/src/world/placements.rs`.
**Destination:** the existing monolith construction region, in a proposed
`construction/placement_lowering.rs` module. <!-- cite-ok: proposed file path -->
Move `ActorPlacementContext`, actor-specific lowering implementations and their
registration adapter with actual callers. Keep immutable room/placement records
and the provider-neutral lowering protocol at their current world/provider owner
unless the caller inventory proves a different semantic owner. Do not move all
world placement vocabulary simply because it shares a file.

**Direction:** construction adapter -> world definition + prepared domain values;
world spatial runtime does not depend on actor construction or character sprites.
**Abstraction:** none beyond the existing typed lowering/provider seam. Do not
introduce a new universal placement enum or callback registry.

**Preflight:** enumerate registrations, duplicate/conflict behavior, function
pointers, materialization/readiness inputs and public aliases. Distinguish room
construction parameters (which include objects/summons/encounter parts) from
actor-only recipes; a rename to actor cannot certify those boundaries.

**Acceptance:** existing provider lowering and construction preflight tests;
unknown kind/conflicting registration; same prepared plan and construction
fingerprint; first-tick volume/SimId/provenance; save-based preparation; no extra
copy of catalog authority. Tests must exercise the real registered provider path.
Change all internal imports and remove obsolete world aliases. SCC change is
recorded without a numeric target. No live gameplay or ordering change is intended.

## A4. Co-locate accepted control and body execution

**THE MAP THIS HOLD ASKS FOR IS DELIVERED:**
[`accepted-control-writer-map.md`](accepted-control-writer-map.md) (2026-09-10).

✅ **AND THE ONE THING THE MAP LEFT OPEN IS CLOSED (2026-09-11).** The map found
no *"one authority per fact"* doubt and no split to make; what it did leave was a
warning — **durability was expressed as an ABSENCE**, so a third producer of
`InCustodyOf` became non-durable by default and silently. `InCustodyOf` now
carries `CustodyDurability { Restored, SessionOnly }`, which has no `Default`:
a producer cannot construct the relation without deciding. Guard and poison on
the map's own page.
⇒ **What remains of A4 is the REGROUPING** — *"coherent logical actor/control
modules in the same package first"* — and the map's last line still stands: no
new abstraction is warranted, because no missing narrow claim/result value was
found.

**HOLD DISCHARGED 2026-09-10** by the map above, which delivers BOTH halves the
hold asks for — the writer map and, under *"Production fixtures for A4's
acceptance list"*, the fixture selection. ⚠ This line read *"HOLD on extraction:
first map writers and select production fixtures"* until 2026-09-11, under a
banner saying the map was delivered — and then under a SECOND banner saying the
one thing it left open was closed. Three statements of this hold's status, two of
them added above a third that still said wait.
**Source regions:** `control/authority.rs`, `control/input_systems.rs`,
`abilities/traversal/possession.rs`, `body_custody.rs`, live actor clusters,
`avatar` integration and `features/ecs/actors/update.rs`, all inside the monolith.

**Responsibility:** accepted driver relation, input projection, live body execution
and custody reconciliation. **Destination:** coherent logical actor/control
modules in the same package first. Possession eligibility remains an optional
ability policy; the accepted relation is not that policy. Generic motion remains
at its established body owner. **New abstraction:** none until the writer map
shows a missing narrow claim/result value.

Move `advance_body_anim_overlays` out of the control -> features import according
to its actual simulation/presentation semantics and phase. Do not replace the call
with an event without preserving visibility and timing. Determine whether its
output affects simulation geometry before classifying it as cosmetic rendering.

⭐⭐ **THAT LAST SENTENCE IS ANSWERED, MEASURED 2026-09-12 — AND THE ANSWER IS
THE OPPOSITE OF WHAT THE SENTENCE INVITES. `advance_body_anim_overlays` IS NOT
COSMETIC RENDERING.** Three findings, and the second is the one a reader would
get wrong:

1. ⛔ **IT MUTATES ROLLBACK STATE.** Everything it writes lives in
   `BodyAnimFacts`, registered as `actor.animation_facts`
   (`ambition_characters/src/rollback_registration.rs:94`) — canonical sim state
   cloned and restored on every rewind. **Anything that writes it must run in the
   deterministic simulation**, so *"move it to presentation"* is not an available
   reading of this row no matter how the timers are named.

2. ✔ **BUT IT DOES NOT AFFECT SIMULATION GEOMETRY, WHICH IS THE NARROWER QUESTION
   THE ROW ACTUALLY ASKS — and the road is worth naming because it LOOKS like it
   should.** Authored attack volumes resolve against an ANIMATION ROW, so overlay
   timers that pick animations would reach hitbox geometry. They do not:
   `attack_support::player_attack_hitbox` takes its row from
   `attack_intent_animation(intent)` — a `match` on the attack INTENT — and never
   consults `BodyAnimFacts`. ⇒ *"Affects simulation geometry"* is specifically
   **false**, and it is false for a structural reason rather than by luck.

3. ⚠ **ONE TIMER IT DECAYS IS A SIM CLOCK WITH A SECOND WRITER.**
   `death_anim_timer` is armed every frame by
   `ambition_combat::death_rules::tick_death_interlude` while the death window is
   open (`anim.death_anim_timer = window.remaining.max(dt)`), and the anim view
   derives `v.dead` from it — so *"dead"* and *"out of play with a window open"*
   are the same fact. This function's decay governs it only AFTER the window
   closes. Two writers with a clear precedence, not a conflict, but not cosmetic
   either.

✅ **AND THE MOVE IS DONE, 2026-09-12 — A4's LAST NAMED TASK IS CLOSED.**
`advance_body_anim_overlays` now lives at
`ambition_characters::actor::advance_body_anim_overlays`, beside `BodyAnimFacts`,
which is the component it ticks. Both callers
(`control/input_systems.rs`, `features/ecs/anim_helpers.rs`) name the owning
crate instead of reaching into `crate::features`, and the monolith's
`features::movement_fx` re-export is gone. ⇒ **The function touched nothing from
the module it lived in** — only `BodyAnimFacts` fields and one local constant —
so the import edge existed for no reason at all. ⚠ **THE ARM/DECAY PAIR IS NOW
SPLIT ACROSS CRATES, AND BOTH ENDS SAY SO.** `arm_movement_anim_overlays` and
`arm_ground_contact_anim_overlay` stay in the monolith because they read engine
events (`ae::FrameEvents`, ground contact); the decay reads nothing but the
component. **Arming is engine-specific, decaying is a property of the data**, and
that is the line the split follows rather than convenience.

⇒ **THE MOVE THIS ROW WANTED WAS A DIRECTION, NOT A PHASE.** The coupling to cut
is the `control -> features` import (`control/input_systems.rs:334` reaching
`crate::features::advance_body_anim_overlays`); both ends are inside the monolith,
so this is an intra-crate module edge a census counted, not a crate edge. **The
phase must not move.** ⛔ Do not reclassify `BodyAnimFacts` as presentation on the
strength of its field names — that is the trap this row's own wording sets, and
the rollback registration is the fact that settles it.
**Acceptance:** human -> possession -> brain -> human handoff on the same body;
competing claims, mount/dismount, removal of a controlled body, two participants,
rollback over handoff, action continuity and no double body tick. Distinguish
home-avatar, driver, camera and participant identities. Preserve legitimate
policy eligibility differences. Group an internal control/possession cycle if
that makes the invariant easier to inspect; do not require it to disappear.

## A5. Give destructible world objects their full transition authority

**BOTH HALVES OF THE HOLD ARE SATISFIED — the enumeration is DELIVERED:**
[`destructible-writer-inventory.md`](destructible-writer-inventory.md) (six
production mutation sites, three crates; re-derived unchanged 2026-09-11). A2's
contact contract closed at `0157476ba`. ⚠ This row read *"HOLD until … writer
inventory is complete"* and linked neither page until 2026-09-11, which is the
third frontier summary to outlive its own enumeration.

⇒ **AND THE CENSUS ANSWERS THIS ROW'S OWN CONDITIONAL.** *"Falling chests and
switches are separate mechanisms unless they demonstrably share the same
transition authority"* — measured, they demonstrably do not: a breakable
transitions through a `#[must_use]` DOMAIN method, a chest through an ECS MARKER
COMPONENT (`Opened`, five sites, three crates), and a falling chest not at all —
it is a position tick. **There is nothing duplicated to consolidate.** ⛔ So the
destination below is not a move this packet has earned; see the page.

⇒ **What it did find — AND IT IS FIXED, 2026-09-12.** `Chest::state` was
WRITE-ONLY: authored as `ChestSpec.state`, lowered at spawn, and read by nothing
in production, while the runtime gate is the `ambition_combat::Opened` marker. An
authored `Opened` chest would therefore spawn with the runtime treating it as
CLOSED — **its reward grantable a second time.** Latent because LDtk's
`ChestSpawn`, the only producer of a `ChestSpec`, declares just `name` and
`reward`.

⭐⭐ **THE FIELD WAS DELETED, NOT GUARDED, AND THE CHOICE IS THE POINT.** The first
fix I built was a `warn!` trip-wire on a non-`Closed` authored state — a check on
a state no authoring surface can reach, which is the weak half of *make it
impossible, not checked*. Removing `ChestSpec.state`, `ChestStateSpec` and <!-- cite-ok: the DELETED chest-state vocabulary, named on purpose. These rows are the CENSUS that justified removing it (2026-09-12); a resolvable citation here would mean the deletion did not happen. See Q105. -->
`Chest::state` outright means the second recorder **cannot be expressed**, so it
cannot return by accident. `ChestSpec::new(reward)` is the only constructor and
there is no struct-literal site.

⚠ **THE CENSUS, RE-MEASURED AT HEAD RATHER THAN QUOTED:** no `.ron`, `.json` or
`.ldtk` in the tree — the `game/ambition_map_assets` submodule included — authors
a chest state (the only `Opening` token in any data file is a music comment in
`pulse_voyage_drift.ron`); `Chest` appears nowhere in `ambition_persistence`, so
it is not in the save format. ⛔ The first version of that census was a LINE-BASED
grep (`state:` on a line also matching `chest`) which cannot see a multi-line
spec, and it was widened before anything was deleted.

⇒ **What is NOT decided and was deliberately not guessed:** whether an authored
opened chest should spawn with the marker or be refused. Whoever wants it adds
the field and its lowering TOGETHER — a field without a lowering is exactly the
trap this removed. **Second write-only field RECORDED, not removed:**
`Chest::persistent` has the same shape (real authority:
`encounter_reward_looted_flag`) but no defect attached, so deleting it would be a
product-surface change with no correctness argument; the note is in
`ambition_interaction`'s own test.
And *"melee/projectile geometry agreement"* now has a guard
(`world/overlay.rs::breakable_geometry_agreement`, poison-verified): both
publishers read one `CenteredAabb`, and their ELIGIBILITY predicates diverge on
purpose.

**Source:** `ambition_interaction` breakable definition,
`crates/ambition_combat/src/breakables.rs`, monolith feature bundles/spawn/target
publication/damage, world placement and simulation-view adapters.
**Destination:** one logical destructible-object owner, initially a module;
independent reusable capability/package only if a minimal customer justifies it.

Move live state, health/broken/respawn transitions, collision contribution and
runtime installation together. Authored lowering stays a typed domain adapter;
render extraction consumes outcomes/geometry. Falling chests and switches are
separate mechanisms unless they demonstrably share the same transition authority.
Do not pull all interactive objects into a new breakables crate.

**Abstraction:** a bounded destructible state/outcome contract, only if current
components cannot already express it. No `FeatureKind` dispatcher for every game.
**Acceptance:** hit vs stand vs pogo eligibility, simultaneous contacts, one break
transition, respawn/collision restoration, melee/projectile geometry agreement,
room replay and save policy, no duplicate effects/rewards, headless operation.
Player-only stand eligibility is preserved until separately decided.

## A6. Separate prepared character definitions from live materialization/policy

**THE CENSUS THIS HOLD ASKS FOR IS DELIVERED:**
[`prepared-definition-field-census.md`](prepared-definition-field-census.md)
(200 use sites, 27 fields, 9 consumer crates; re-derived unchanged 2026-09-11 at
`d7d8aaee4`, member list diffed and saved).
⇒ **And it REFUTES this packet's premise.** Nine fields are read by BOTH the
spawn road and the runtime, so the two-way split written below does not exist to
be finished. Any A6 proposal starts from that table, not from this paragraph.

⛔⛔ **AND THE DUAL-READ AXIS IS NOT THE AUTHORITY AXIS — RULED 2026-09-11, AND IT
LANDS ON THIS ROW'S OWN ACCEPTANCE CLAUSE.** The census seals ONE struct, so all
200 sites read one authority and no pair of them can be two; all sixteen
production runtime reads resolve through `PreparedCharacterRegistry`. **Six of the
nine are two readers of one authority and need nothing** — `kit`, `death_traits`
and `mount` have no second home, `id` is the key, and `provider` and `sheet` have
one that `CharacterAuthorityConflict` already audits. The second authority is
`PreparedCharacterRegistry` vs `CharacterCatalog`, and only `autonomous_profile`
sits on it unwatched — held by a content test rather than the audit.

⛔⛤ **AND THAT LAST SENTENCE IS IMPRECISE — RE-DERIVED 2026-09-12 BEFORE SIZING
ANY WORK FROM IT, WHICH IS WHY IT DID NOT BECOME A PACKET.** The audit's four
variants cover display-name ambiguity, display-name disagreement, sheet and
provider (`character_runtime/audit.rs:111`); `autonomous_profile` is indeed not
among them. But the two sides are NOT two recorders of one fact:
`CharacterCatalogData.autonomous_profiles` is a per-provider-namespaced
NAME→profile LIBRARY — the resolution target for `AutonomousPolicy::Named`, not a
character→policy map — while `CharacterCatalogEntry.default_brain` names a
`BrainPreset`, **a different vocabulary the page already explains** (presets
author ABSOLUTE speeds, profiles author normalized effort against the body's own
`run_speed`). And the precedence between them is already ruled in code:
`character_catalog/mod.rs:145` says *"A character that names no preset has no
default brain to build — its definition's autonomous profile is the answer, and
this road is not it."*

⇒ So there is no unwatched authority split here; there is one resolution, in
`crates/ambition_characters/src/prepared.rs`'s `resolve_autonomous_profile` call. A fifth audit variant would be guarding a disagreement the
types do not allow. **What the re-derivation DID turn up is small and is NOT
worth a packet:** `CharacterCatalog::build_default_brain` has ZERO
readers anywhere, while its sibling `build_brain_from_preset` (both in
`crates/ambition_characters/src/actor/character_catalog/mod.rs`) is
production-live via `features/brain_command.rs:210`. A dead METHOD beside a live
one, not a dead road — and deleting twelve lines is the cleanup pattern this
project's own standing rules name as avoidance.

⚠ THE INSTRUMENT NOTE IS THE PART WORTH CARRYING: my first grep found zero
readers of `build_default_brain` and I was one step from reporting "the road was
built and the traffic never arrived" — this repository's own most-used template.
Widening to `default_brain` and `brain_from_preset` found the live sibling
immediately. **A finding that fits your template gets less scrutiny, and a
negative grep is a claim about the query.**

⭐⭐ **AND `movement_tuning`/`motion_model` ARE NOT ON IT AT ALL, MEASURED
2026-09-11 IN THE SHIPPED HOST — the acceptance line "no duplicate authored
movement/tuning authority" IS ALREADY MET.** The barrier FOLDS both
(`crates/ambition_characters/src/prepared.rs:1338`, `:1334`), so the registry is the catalog's fold and cannot
disagree with it: 147 catalog rows, 58 prepared, and **zero disagreements in the
overlap**. The read-site fall-back in `avatar/starting_character.rs` is reached
for 89 ids per boot and the catalog authors a value for **none** of them, so it
returns the default every time. ⇒ The residue is not a boundary and not an audit
variant — **the fold is spelled twice**. ⛔ **DELETING THE SECOND SPELLING WAS
TRIED AND REVERTED**: all five compositions measure ZERO orphans (shipped host
147/58, mary_o 7/7, twintrack 2/2, sanic 3/3, smash 3/3 — and in the four demos
the read-time fold is never even reached), yet removing it reddened SIX tests
across three files, all on the wear/re-wear road the fold serves, one of them
asserting the deleted behaviour outright. ⇒ **The open question is a ruling on
what an UNPREPARED id should inherit at wear time**, which is design, not cleanup.
`game/ambition_app/tests/authored_feel_reaches_the_prepared_cast.rs`
(poison-verified) keeps the orphan case from arising meanwhile; it could not have
caught the six, and the in-crate suite did.

⛔ Not a resolver, not a bus, and no type moves; see the page for why.

**HOLD (SATISFIED):** make a field/use census before moving types.
**Source:** `ambition_characters` actor/prepared/brain/moveset/technique schemas;
`ambition_combat` action/brain runtime; monolith `character_runtime` and
`avatar/starting_character.rs`; body-seed/spawn consumers.

**Destination:** logical prepared-definition, action-execution, intent-policy,
asset-materialization and session/match-activation owners. Keep tightly coupled
action schema/executor together when separating them would add a ceremonial
interface. Move texture readiness out of schema and deterministic simulation.
Generic techniques can remain reusable even when initially named for Smash.

**Dependency:** preparation -> domain values; executor -> prepared action;
brain -> action menu; materializer -> prepared visuals + asset service;
activation -> prepared identity + lifecycle. No brain/catalog path should require
host window/render installation merely to select an action.

**Acceptance:** same character definition supports headless and windowed runs,
player and CPU, alternate action scheme and equipment; no duplicate authored
movement/tuning authority; preparation errors name a field/source; hot revision
activation does not mix mechanical values and visual values from different
revisions. Record current unsupported combinations instead of defaulting them.

## A7. Separate item custody/accounting from lifecycle orchestration

**THE ENUMERATION THIS HOLD ASKS FOR IS DELIVERED:**
[`item-writer-inventory.md`](item-writer-inventory.md) (2026-09-10, 60 sites).
⇒ It reports the shape as the INVERSE of this packet's framing: the checkpoint
baseline family is 9 of 9 inside the monolith, while `OwnedItems` is written from
four crates with 3 of 16 sites in the crate that defines it.

⛔ **`GroundItem`'s SEVEN MINTING SITES ARE CLOSED; DO NOT RE-OPEN THE JOB.**
This paragraph said *"has no constructor, so seven struct-literal sites"* in the
PRESENT TENSE until 2026-09-11, fourteen lines above the linked page's own
`✔ SEALED`, and sent an agent at work that had landed in `7108a57b1`
(2026-09-10). The type is `#[non_exhaustive]` with `at_rest` and `released` and
those are the only two roads. POISON-VERIFIED at `8ce24653d`: reintroducing the
struct literal at the death-drop site (`damage_drops.rs:332`) fails
`cargo check -p ambition_platformer2d_actor_monolith` with `error[E0639]`.

⇒ **WHAT IS STILL OPEN IS NOT THE CONSTRUCTOR.** `drop_held_weapon` mints the
occurrence's identity, room scope, provenance and attempt state itself, so a
component-construction seal is not occurrence authority and this packet's
*"reward policy … does not become an alternative item minting path"* is not yet
met. ⛔ The remedy is NOT a generic item-request bus to move the count to one;
[`item-writer-inventory.md`](item-writer-inventory.md) holds why.

**HOLD DISCHARGED:** its precondition and its deliverable both landed — A1 closed
2026-09-09, and the enumeration this hold asks for is the page linked at the top
of this row (2026-09-10). ⚠ This line read *"HOLD: after A1, enumerate item
occurrence, holder, inventory and checkpoint baseline writers"* until 2026-09-11,
including through the 2026-09-11 commit that corrected this row's stale FINDING
and left its stale HOLD untouched. **Source:** monolith items/persistence/minted horizon,
`ambition_world_items`, `ambition_held_items`, combat held/worn state and shared
custody/occurrence vocabulary. **Destination:** domain-owned custody/accounting
modules plus explicit session horizon integration.

Release/pickup/use/throw/settle ordering remains item-owned. Session coordinates
capture/restore boundaries, not internal item mutation. Reward policy receives
accepted outcomes; it does not become an alternative item minting path.
**Abstraction:** only explicit lifecycle milestones already needed by both sides;
prefer current typed horizon sets. No generic save-every-component reflection.

**Acceptance:** world collectibles without held items; thrown persistent weapon
retains per-item identity; no double acquisition across resimulation; checkpoint
capture after settlement; death/reset/room retirement with foreign controlled
bodies; item absence does not suppress session restoration. Preserve count-based
inventory accounting per occurrence and the explicit scope of each baseline.

## A8. Prove live world-instance isolation before generalizing residency

**Customer established:** the persistent multi-room game and separated actors.
Start with OW1 in [open-world planning](open-world-runtime-and-residency.md) and
FI9 in [iteration acceptance](fast-iteration-acceptance.md). Do not wait for another
demo, and do not build full streaming before this bounded proof.

**Source:** monolith world collision/active binding, `RoomSet`, shared SimId and
lifecycle scopes, runtime room transition and portal/view integration.
**Destination:** spatial instance authority plus existing lifecycle coordination.

✅ **STEP 2's ANSWER FOR THE IDENTITY AXIS IS DELIVERED, AND IT DID NOT NEED
STEP 1 — `scripts/measure_identity_instance_scope.py` (2026-09-11).**

*"Identify the exact values that lose instance scope"* is answerable by reading
the CONSTRUCTORS: `SimId` is a newtype over `String` whose only road in is
`impl SimId`, so every live identity in the game is one of ten functions' output,
and an identity that cannot EXPRESS an instance loses it by construction whatever
the runtime does.

| verdict | n | constructors |
| --- | ---: | --- |
| INSTANCE-SCOPED | **0** | — |
| INHERITED (derives from another identity) | 4 | `spawned`, `death_drop`, `strike_volume`, `geometry` |
| NOT INSTANCE-SCOPED | 6 | `placement`, `player_slot`, `encounter`, `match_spawn`, `singleton`, `from_snapshot` |

⛔⛔ **NO IDENTITY CONSTRUCTOR TAKES A ROOM OR AN INSTANCE.**
`SimId::placement(id)` is `"placement:{id}"` — the map's own iid and nothing
else — so two instances of one prepared room mint **the same identity for every
authored placement**. `SimId::encounter(id)` is the same shape. The four
INHERITED ones carry whatever scope their parent had, which is none.

⭐ **THE GOOD NEWS IS THAT IT REFUSES RATHER THAN CORRUPTS.** The construction
planner's `IdentityAlreadyLive` (`crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs:819`) is what a second
instance would hit, so the failure mode today is a loud refusal — not two live
things behind one `SimId`. That is the precondition step 1 would otherwise have
to discover the hard way.

⛔⛤ **AND ONE CLAIM OF MINE WAS WRONG UNTIL I READ THE MINTING SITE.**
`GeoSource::TileLayer { layer: String }` looks unscoped and I wrote it up as a
cross-room collision. It is not: `ldtk/intgrid.rs::emit_collision_blocks_from_intgrid`
passes a LEVEL-SCOPED key, `"{level}/{layer}"`, *"because an active area can span
multiple levels that each carry this layer"*.
⇒ **The scope is a STRING CONVENTION inside a field named `layer`, and the type
cannot hold anyone to it.** `GeoId::tile_layer("Collision", 0)` compiles, and
this repository's own unit tests construct exactly that. ⇒ The A8-shaped fix is
to make the level a FIELD, not to add a rule about the string — same head as the
id-convention-spelled-twice class.

⚠ **WHAT THIS DOES NOT ANSWER:** geometry, contacts, observations and teardown
beyond identity; whether an instance-scoped identity is WANTED (that is the
ruling); and whether the occurrence ledger's `outlook_for(room: &str)` — room by
NAME — is the second place scope is a convention rather than a type.

**POISON-VERIFIED**, three poisons, each firing on its own claim, restore green:
adding a `placement_in(room, id)` moves INSTANCE-SCOPED 0 → 1; breaking the
signature pattern trips the anti-vacuity floor (*"parsed only 0 constructors"*)
instead of printing an empty table; renaming `GeoSource` makes the variant census
REFUSE rather than silently report some other enum's variants — which it did do,
once, before the scan was region-scoped: it reported `Clone` from a `#[derive]`.

1. Run two instances of one prepared room with identical local placement IDs.
   Trace construction, lookup, geometry, contacts, observations and teardown.
2. Identify the exact values that lose instance scope. Qualify them at the owner;
   distinguish definition, live instance, durable occurrence, construction attempt,
   session, body, participant and view. No universal ID wrapper across all domains.
3. Define the saved occurrence namespace before two instances can persist the same
   local placement. Temporary candidate/reload IDs must not become save identity.
4. Route queries and transfers through scoped owner services. Prove one body and
   one custody writer during handoff. Do not retain a separate singleton road.
5. Only then add residency interests, accepted population changes and measured
   budgets. Active membership changes preserve unaffected instances when the
   existing session timeline rebases.

**Acceptance:** no cross-instance collision, observation, lookup or despawn;
controlled/portal transfer preserves body and custody identity; unload of one
instance leaves the other intact; two views of one instance do not duplicate
simulation. The one-room profile is the same implementation with one instance.
**Poison:** select geometry by the global current room, key two placements by
only their definition, or retire both instances on one unload. Each has a runtime
witness. Q94 sets numeric budgets later; it does not hold the identity proof.

## A9. Prove public profiles and actual compile/runtime optionality

**The facade -> host -> render path this packet says to start with closed
2026-09-09.** It was one edge: the windowed host named `ambition_render`
unconditionally, so every consumer of the facade compiled a renderer including
one that selected no capability at all. The host's three presentation plugins sit
behind its own `render` feature now and the facade takes the host
`default-features = false`. Measured 52 -> 50 ambition crates in
`cargo tree -e normal --no-default-features -p ambition_platformer2d`, and
50 -> 49 when the world MAP became the runtime's `map` feature the same day (the
facade already offered `ambition_menu` as a named capability while the runtime
installed `MapStatePlugin` unconditionally — an optional capability with one
unconditional installer is not optional). Both are ratcheted by
`the-featureless-facade-links-none-of-these`, which walks the FEATURE-RESOLVED
tree because the manifest walk the other dependency contracts use counts optional
edges. See the queue row for the poison receipts.

⚠ `ambition_menu` is in that contract's forbidden set FOR THIS PROFILE only:
`ambition_platformer2d_host` legitimately links the menu crate under its `render`
feature, for the `MenuFont` handoff only a composition root can make. Crate
presence in a profile that renders and runtime INSTALLATION of the map are
separate questions with separate witnesses.

⚠ **THE STATED MINIMUM'S OWN LIST IS PARTLY STALE.** It excludes "renderer,
audio, inventory, encounters or game content"; measured 2026-09-09, the renderer
is gone and `ambition_inventory_ui` was ALREADY absent from the featureless
closure. Of the remainder, encounters/cutscene/dialog/items/persistence arrive
through the monolith-and-runtime hub, and audio's one non-monolith path
(`provider -> load_presentation -> game_shell -> audio`) rests on two GENUINE
uses — the provider's public authoring surface carries `LoadExperienceSpec`, and
the shell selects route audio. Neither is residue; both are ownership questions.
See the queue row.

⚠ The separate headless consumer workspace this packet asks for now EXISTS
(`fixtures/headless_profile`, landed 2026-09-09), so this next line is corrected
rather than deleted — the sentence it makes is still true of `minimal_game`: `fixtures/minimal_game` is the WINDOWED sentinel and legitimately links
the renderer, so it cannot answer the question. The 49 crates that remain have
been traced to their activating parents; what has not been established is which
of them a minimum profile has a right to expect.

✅ **THE CENSUS THIS PACKET WAS HELD FOR, MEASURED 2026-09-11 —
`scripts/measure_minimum_profile_closure.py`.**

| question | measured |
| --- | --- |
| featureless `ambition_platformer2d` closure | **49** `ambition*` crates |
| the stated minimum, priced (`core` + `shared_tangle` + `world` + `time` + `input`) | **11** |
| crates in the profile ONLY because the facade names them | **2** (`_host`, `_provider`) |
| the other direct edges, redundant with a path from underneath | **40** |
| `ambition_platformer2d_provider` alone | 47 of 49 (95.9%) |
| `_host` / `_runtime` / `sim_view` / `_actor_monolith` alone | 45 / 44 / 41 / 38 |

⛔⛔ **AND THE STRUCTURAL FINDING IS WHY EVERY PROPOSED CUT HAS MEASURED ZERO.**
Each crate is held by TWO roads at once — the facade names 42 of the 48 directly,
AND the hubs reach the same 42 from underneath — so cutting either road alone
changes the profile not at all. MEASURED: cutting `provider`, `host`, `runtime`,
`actor_monolith` and `sim_view` **together** takes 49 to **44**. ⇒ A census that
asks *"what does removing X save"* reports nothing here, for every X. The
instrument has to ask what a PROFILE COSTS (`--minimum`), not what a cut saves.

⛔⛤ **I GOT THIS WRONG FIRST AND THE MEASUREMENT CORRECTED ME.** Seeing
`provider` carry 47 of 49, I reasoned that gating that one edge would drop the
profile to about ten. It drops it to 48. The hub's size and the cut's saving are
different quantities, and only one of them is a decision.

⛔⛤ **AND THE INSTRUMENT'S OWN FIRST ANSWER WAS UNFALSIFIABLE.** "0 crates are in
the closure only because of the facade's edge" printed regardless of the tree,
because the computation unioned `reachable(child) | {child}` and every child is
in its own set. A poison that deleted every non-root edge still printed 0. The
real answer is 2. See the script's comment.

⇒ **THE RULING, FOR JON (Q108):** *38 crates sit between the stated minimum (11)
and what a featureless facade links today (49).* Naming the capability set is the
decision; the script prices any set in seconds. It is one manifest's feature
gates plus the matching hub gates — a pair of edits per capability, never one.

**Ready now:** record the manifest lower bound and establish a real independent
consumer fixture. **Implementation:** staged with the owners whose dependencies
need splitting; do not wait for every monolith region to be extracted.

**Source:** facade Cargo features/app builders, host/runtime dependencies,
`fixtures/minimal_game`, public namespace mirrors and full game composition.
**Destination:** explicit SDK profiles and host/simulation/presentation seams.
The intended minimum is a constructed body advancing against world geometry,
without renderer, audio, inventory, encounters or game content; this is a target,
not a passing current profile.

Create a separate consumer/workspace for that headless profile, not a second
manifest that inherits all workspace feature activation. Record actual
`cargo metadata --format-version 1` and `cargo tree -e features` for its manifest,
plus runtime installed resources/systems. Start with the concrete facade -> host
-> render dependency and then trace every alternate path. Correct the stale
minimal-game comment with the implementation. Do not infer absence from one
optional edge or from link-time dead-code removal.

Separate tests for: headless body/world; windowed body/world; combat without
inventory/bosses/dialogue; world collection without held use; generic encounter
without named boss content. Add one provider-owned prepared action/object through
physical input, validation and runtime outcome. Tests should fail on a missing
prerequisite with a diagnostic, not install a dummy sibling.

**API migration:** name supported capability namespaces, migrate fixture/demo
consumers, then delete internal mirror re-exports. Preserve an ergonomic facade;
no consumer must import 20 implementation crates. Measure build time and binary
footprint separately on an available toolchain/hardware before making claims.

## A10. Constrain candidate construction for reliable development reload

**Customer established:** I3b's repeated scene reconstruction after a content edit.
The current source uses raw Commands, so its post-commit verifier cannot undo
arbitrary mutation (F6). Preserve that source fact; do not preserve its limitation
as the finished reload design. I1/I2 and I3a do not wait for this packet.

**Source:** shared construction executor/recipes, runtime prepare/commit/verify/
publish, domain construction services. **Destination:** one restricted typed
candidate path under the existing lifecycle owner. Detailed steps and failure
classes are in [construction](construction-and-reconstitution.md) and
[generation/reload](content-generation-and-reload.md).

Inventory selected recipes, resource writes, hooks and observers. Factor typed
inactive candidate data from active materialization. Validate the full candidate
and pinned state mapping before retirement. The materializer cannot discover new
fallible IO or domain requirements after that boundary. Do not add a universal
recipe language, arbitrary World clone or second lifecycle coordinator.

**Acceptance:** FI4 exercises a nonempty scene, invalid candidate relationships,
duplicate identity, forbidden resource mutation and successful reconstitution.
Supported candidate refusal retains the active scene; explicit reconstruction of
the pinned prior scene reports recovered, not unchanged. Unexpected native faults
remain fail-stop and do not count as a successful reload.

A `Pending` marker does not isolate a candidate from queries, observers or hooks.
A same-World or separate-World strategy needs its own complete visibility/transfer
proof. Prefer typed inactive drafts first. General unsafe-plugin recovery and
arbitrary save/schema migration are outside this bounded packet.

### ✅ UNBLOCKED (`Q113` RULED YES) — AND STEP 1'S INVENTORY IS DELIVERED, 2026-09-12

**The isolation problem is MUCH smaller than the paragraph above implies, and the
difference is four measurements rather than an argument.**

**1. ⭐⭐ THE ENGINE SHIPS THE QUERY-ISOLATION PRIMITIVE AND THIS REPO DOES NOT USE
IT.** `bevy_ecs 0.19.1` has `entity_disabling`: `World::register_disabling_component::<T>()`
installs `T` into a global `DefaultQueryFilters`, and **every query that does not
NAME `T` stops seeing those entities** — engine-enforced, not a convention.
Bevy's own doc example calls the custom component `Prefab`. Entities keep their
place in the `World` and *"their relationships remain intact"*, and
`EntityCommands::insert_recursive` disables a whole tree at once. ⇒ *"A `Pending`
marker does not isolate a candidate from queries"* is true of a MARKER and false
of a **registered disabling component**, which is a different mechanism with the
same shape. Measured: zero uses in this workspace (the `Disabled` hits are
`ambition_asset_manager`'s unrelated asset-location enum).

**2. ✅ COMPONENT HOOKS: ZERO IN THE WORKSPACE.** No `on_add`/`on_insert`/
`on_remove`/`on_replace` component hook is declared anywhere. The packet's hook
warning is, at HEAD, a warning about an empty set.

**3. ✅ LIFECYCLE OBSERVERS: THREE, AND ONLY ONE IS AN `Add`.**
`ambition_combat/src/stocks.rs:101` (`Remove, RespawnGrace`),
`actor_monolith/src/features/empowerment.rs:311` (`Remove`), and
`ambition_touch_input/src/placement.rs:161` (`Add`) — the last is a touch surface
and is not on the construction road. The other 81 observer registrations are
keyed on pointer, dialog and custom domain events, which construction does not
emit. ⛔ **THE QUERY THAT FOUND THESE IS RECORDED BECAUSE MY FIRST ONE MISSED A
THIRD OF THEM**: `On<Remove` matches nothing when the trigger is written
`On<bevy::ecs::lifecycle::Remove, …>`. Use
`On<[a-z:]*\(Add\|Insert\|Remove\|Replace\|Despawn\)\b`.

**4. ✅ RECIPES CANNOT WRITE RESOURCES, STRUCTURALLY AND IN FACT.**
`ConstructionExecCtx` hands a recipe `{ commands, scope, session, services }` —
no `World`, no `ResMut` — and measured across all NINE recipes
(`authored-ground-item`, `staged-actor`, `summoned-minion`, `giant-host`,
`giant-hand`, `authored-enemy`, `authored-placement`, `authored-shrine`,
`authored-boss`) there is not one `insert_resource`/`init_resource`. ⚠ `Commands`
COULD carry one, so this is a population fact rather than a boundary — if A10
wants it to be a boundary, that is a small, nameable guard.

⭐ **AND THE EXECUTOR IS ALREADY CONSTRAINED IN THE DIRECTION THE PACKET WANTS.**
`ConstructFn` *"cannot choose the entity, return a different one, or hand back
something that was already alive"* — the executor mints the `ConstructionRoot` —
and **it cannot fail: it returns nothing.** So "the materializer cannot discover
new fallible IO or domain requirements after that boundary" is already true of
the recipe half; what is unproven is the PLAN half above it.

⇒ **WHAT THIS LEAVES AS THE REAL WORK**, which is now a design rather than a
discovery: mint candidate roots carrying a registered disabling component, prove
the three lifecycle observers are either unreached or explicitly tolerant, keep
the validation between construction and publication, and make retirement of the
old world explicit. **The visibility proof the packet demands is enumerable now
rather than open-ended.**

### ✅ STEP 2 LANDED: THE CANDIDATE LIFECYCLE EXISTS, 2026-09-12

`ambition_platformer2d_shared_tangle::construction` now carries the three
operations the ruling names, and the guarantee is structural rather than
procedural:

```text
last-good world N
    ├── commit_inactive   → candidate N+1 exists, unseen by every ordinary query
    ├── validate          → refuse → retire_candidate, and N never knew
    └── publish_candidate → N+1 visible; retiring N is then a separate, explicit act
```

* **`InactiveCandidate`** — registered through `register_inactive_candidate_filter`
  as a bevy DISABLING component, so `DefaultQueryFilters` hides it from every
  query that does not NAME it. The candidate keeps its `SimId`, provenance,
  `TransactionId` and relations while invisible, **which is what makes
  publication a component REMOVAL rather than a transfer** — nothing is copied,
  moved or re-identified at the boundary, so it cannot half-happen.
* **`ConstructionPlan::commit_inactive`** — REFUSES with
  `InactiveCommitRefused::FilterNotInstalled` when the filter was never
  registered. ⛔ That direction is the point: an unregistered `InactiveCandidate`
  is an ordinary inert component, so every root would be stamped and every root
  would be LIVE — the isolation silently doing nothing is the one failure nobody
  would see.
* **`publish_candidate` / `retire_candidate`** — both keyed on `TransactionId`,
  both returning how many roots they touched so a caller can assert it moved the
  transaction it built rather than an empty set.

⭐ **ONE EXECUTOR STILL, AND RECIPES CANNOT TELL.** The roots are stamped AFTER
the rows are built, so a recipe runs against the context it always did. There is
no candidate-aware fork of nine recipes.

⚠ **GUARDS AND WHAT THEIR POISONS PROVED.** Four arms — built-and-unseen (both
halves: invisible AND present), publication admits the whole transaction,
retirement leaves a NONEMPTY published world untouched, and the unregistered-filter
refusal building nothing. ⛔⛤ **AND ONE POISON PASSED, WHICH CORRECTED A CLAIM
RATHER THAN THE CODE:** I wrote `Allow<InactiveCandidate>` beside the `With` and
called it load-bearing; removing it left all four green. `DefaultQueryFilters`
excludes a disabling component only from queries that do not MENTION it, and
`With` mentions it — so `Allow` was redundant. The real hazard is one step over
and is poison-confirmed: a query that does not name the component at all returns
ZERO roots, and `publish_candidate` then reports success having admitted nothing.

### ✅ STEP 3: VALIDATION SITS BETWEEN CONSTRUCTION AND PUBLICATION — AND ONE OF THE VERIFIER'S OWN LIMITATIONS IS NOW FALSE

`AuthoritativeScope::gather` sees candidates (`Allow<InactiveCandidate>`), so the
EXISTING `verify_committed_roster` validates a candidate without a second
verifier being written. ⭐ **AND IT MAKES A SENTENCE IN THAT FUNCTION'S OWN DOC
FALSE, WHICH IS THE CLEAREST STATEMENT OF WHAT A10 BUYS:** it read *"a violation
here cannot be undone — it can only stop the transaction being PUBLISHED, and
leaves the world in whatever state the offending recipe produced."* True of a
LIVE commit, and false of a candidate: the offending state is invisible, so
retiring it is a DROP rather than a recovery. **The difference between the two
commits is exactly whether a detector's findings are actionable.** Guarded by
`a_candidate_that_fails_verification_is_caught_and_dropped_without_touching_the_live_world`.

⛔⛤ **AND THE ISOLATION IS NOT COMPLETE — MEASURED, GUARDED, AND STATED HERE
RATHER THAN DISCOVERED LATER.** `commit_inactive` stamps the roots the EXECUTOR
minted, which is every PLAN ROW. A recipe also holds raw `Commands` and may spawn
authoritative entities of its own — the giant hand's limbs do — and those are not
in the receipt, so nothing stamps them and **they stay visible while the rest of
the candidate is hidden.** That is a HALF-VISIBLE candidate, and it is precisely
the *"complete visibility/transfer proof"* this packet demands of a same-World
strategy. Pinned by
`a_recipe_that_spawns_its_own_entity_escapes_the_candidate_isolation`, written so
that the day the isolation becomes complete the arm inverts rather than rots.
⭐⭐ **AND IT IS LATENT: NO PRODUCTION RECIPE SPAWNS ITS OWN AUTHORITATIVE ROOT
(measured 2026-09-12).** Every `ctx.commands.spawn` in the tree is TEST code, and
the single case the verifier's own doc named — *"the giant hand limbs already do
the last of these"* — is STALE: the hands are plan rows (`giant_hand_plans` feeds
`giant_cluster_rows`). ⇒ **With no recipe minting its own roots, stamping every
PLANNED root isolates every candidate this engine actually builds**, so the
structural fix the verifier names — every authoritative root an explicit plan row
— is already TRUE of production and the remaining work is to make it
unexpressible rather than merely unused. The guard pins the shape so the day a
recipe starts spawning, the gap is a failing test rather than a half-visible
scene.

⚠ **THREE POISONS FAILED TO BITE `Allow`, SO ITS NECESSITY IS LABELLED REASONED
RATHER THAN MEASURED.** The reason is worth more than the filter:
`verify_committed_roster` reads most of what it checks DIRECTLY BY `Entity` from
the receipt, and direct `World` access is not filtered by `DefaultQueryFilters`
at all. ⇒ Receipt-driven checks see a candidate either way; only the
query-driven half (strays, duplicates, unowned identities) could need it, and a
stray is currently unstamped and therefore visible anyway. Kept because a scope
that cannot see what it is scoping is wrong on its face, and documented as
unproven rather than asserted.

### ✅ STEP 4: A RECIPE CANNOT SPAWN — THE SURFACE NO LONGER HAS `Commands`

**`ConstructionRootCtx` is what a recipe now receives, and it holds no
`Commands` at all.** Its whole API is `root()`, `insert(bundle)` and
`insert_in_session(bundle)` — all three bound to the row's own root. ⇒ For the
two fully-migrated domains (`gravity`, `portal2d`) a recipe minting an
authoritative entity is **unexpressible rather than unused**, which is what makes
`commit_inactive`'s isolation complete there: the executor hides every root it
minted, and there is no other way to mint one.

⚠ **NARROWED TO WHAT RECIPES ALREADY DID, MEASURED FIRST.** Across all four files
implementing a recipe, the entire production surface was
`entity(root).insert(..)` and `insert_room_in_session(session, root, ..)`. Both
are root-bound and both are on the new type. Two `type Ctx = ConstructionExecCtx`
aliases fell out DEAD in the process — those domains had only ever used the
executor's context for recipes.

✅ **A10 STEP 5, 2026-09-12: THE ESCAPE IS DELETED AND THE MINT IS NOW
UNEXPRESSIBLE IN EVERY RECIPE, INCLUDING THE MONOLITH'S.**
`ConstructionRootCtx::commands_escape()` is gone; `root_scope()` replaces it and
hands out a [`RootScope`] — root, three ownership-flavoured inserts, one
entity-bound deferred edit, and **no `spawn` of any kind**. The nine monolith
recipes go through it, and so does the placement-lowering road: `LoweringCtx`'s
three fields `commands` / `session_scope` / `root` collapsed into one
`scope: RootScope`, which is what closed the last one.

⭐⭐ **THE RECORDED BLOCKER WAS MUCH SMALLER THAN THE PACKET SAID, AND
RE-DERIVING IS WHAT FOUND THAT.** The hold read *"they delegate to helpers taking
`&mut Commands` … and changing those signatures is a separate packet"*, which
carried an unstated premise: that those helpers have other callers. **MEASURED at
HEAD:** of the five `spawn_*_into` entry points, FOUR have exactly ONE production
caller each and it is the construction recipe itself; every other reference is a
test. Only `spawn_interactable_into` has a second production caller
(`spawn_static.rs`), and it was already populating a root it had allocated. ⇒ The
signature change rippled to 11 functions across 3 crates and 9 test sites, not to
a second packet's worth of roads.

⭐⭐ **TWO TYPES, BECAUSE ONE OF THEM WOULD HAVE HAD TO LIE.** `RootScope` knows
the gameplay session that owns its root and can stamp it; `EntityScope` is the
session-less half, for a helper that FINISHES an entity whose ownership someone
else settled — `grant_prepared_character_body` and `GrantedBodyFacts::retract`,
whose other callers are the re-template pass and the match-seat materializer.
Collapsing them would have meant handing those callers a
`SessionSpawnScope::UNSCOPED` they never meant, i.e. **inventing an ownership to
satisfy a signature.**

⭐ **`queue_component_mut` IS WHAT KEPT `Commands::queue` OUT.** One production
site needed a DEFERRED edit (`switch_motion_model`, which must preserve shared
body facts rather than replace the component). A queued `&mut World` closure can
spawn, so exposing one would have given back everything the scope takes away;
the narrow form edits one component on the scope's own entity and skips if
absent.

⛔⛤ **POISON-VERIFIED AT THE COMPILER, THREE WAYS, ALL IN A REAL MONOLITH
RECIPE** (applied, checked, reverted, md5-verified):
1. `ctx.root_scope().spawn(SimId::placement("uninvited"))` →
   *no method named `spawn` found for struct `RootScope`*
2. `ctx.commands_for_sabotage()` from production →
   *no method named `commands_for_sabotage` found* (it is `cfg(test)`)
3. `ctx.root_scope().commands.entity(other).despawn()` →
   *no field `commands` on type `RootScope`*

⛔ **THE POLICY `engine.construction-recipes-do-not-spawn` IS DELETED, NOT
RETARGETED.** Its own rationale admitted it *"matches the escape by NAME, so it
counts doors rather than proving nothing walks through one — the structural half
is the absent `Commands` field"*. The structural half is now complete, and the
string it forbids exists nowhere in the tree: keeping it would be a **check that
cannot fail**. What regression looks like is a new accessor returning
`&mut Commands` from the construction surface, and that is recorded on
`root_scope`'s own doc where someone adding one would read it.

⭐ **AND A SECOND GUARD SPENT ITS ALLOWANCE RATHER THAN ROTTING.**
`engine.room-feature-spawns` allows `features/ecs/spawn/tests.rs` an EXACT count
of raw `commands.spawn`; the test lowering that used its 1 now inserts on the
row's own root, and the exact-count check went RED at 0 against a reviewed 1.
Lowered to 0 with the reason. A one-directional ratchet would have left that row
at 1 forever.

⚠ **WHAT THIS DOES NOT CLAIM.** The isolation still covers PLANNED roots and
nothing else, and the arm that pins it was renamed from
`a_recipe_that_spawns_its_own_entity_…` to
`an_authoritative_root_minted_outside_the_plan_…` because its population changed:
recipes can no longer reach that state, **every other system running in the same
frame still can.** Its doc used to assert *"a recipe also receives raw `Commands`
… the giant hand's limbs really do this"* — the second half was already false
when written (those limbs have been plan rows since `giant_hand_plans` fed
`giant_cluster_rows`), and both halves are corrected in place.

⇒ **WHAT REMAINS OF A10:** wire this to a real reload customer (I3b's scene
reconstruction), and make retirement of the OLD world explicit rather than
implied. The monolith migration is DONE (step 5 above).

## A11. Make installed technique support a preparation contract

**Ready:** A11a support/handler and preparation tests, independent of A1-A10.
**Normative owner:** [authored technique admission](authored-technique-admission.md).
This is a bounded validation catalog, not an executable service locator.

**A11a:** each typed capability installation supplies both its native handler and
its support declaration. Freeze the actual selected profile before semantic
preparation; reject duplicate, unknown, disabled, unsupported-site and invalid
parameter use. Paramless is explicit and admits only the canonical empty map.
Do not use function equality, last-write-wins or metadata alone as evidence that
a handler is installed.

**A11b:** one exhaustive visitor validates expanded timeline, sustained-window,
on-hit, flow and domain-declared nested effect sites. Reuse it for discovery and
reverse references. Guard all production prepared-definition insertion paths and
prove rejection leaves the active registry and generation unchanged.

**A11c:** after A12b, exercise edit -> selected-profile preparation -> headless
fixture -> review -> explicit session/reconstruction-boundary activation through
the supported authoring route. Last-good prepared-definition retention is required;
arbitrary last-good-world retention after destructive native failure is not.

## A12. Align flow validation, prepared representation and execution bounds

**Ready:** A12a raw validation now; A12b after A11's admission contract. The exact
algorithm/clock/delivery rules are in
[authored technique admission](authored-technique-admission.md).

**A12a:** a present version-1 flow has 1-256 reachable nodes, checked in-range
indices, finite positive wait timeouts and an acyclic graph. Both branches are
validated. Reject cycles even when another branch reaches Finish; reject
unreachable material rather than carrying unchecked code. This is a deliberate
authoring-policy tightening, not a claim that cyclic inputs were never accepted.
The three concrete game-source customers contain 3, 3 and 4 nodes and fit it.

**A12b:** introduce the private checked immutable representation, pin it on
MovePlayback, convert indices once and remove the per-tick graph clone. Preserve
effective proper-time, existing charge/repeat behavior, per-occurrence contact
latches and normal teardown. Finish does not end recovery, and an unfinished
flow does not extend the move. Resolve feedback is read on the next eligible
flow update. Do not create a new VM, fresh-per-beat signal bus or arbitrary code
registry to repair index narrowing.

The protocol defines independent expanded-reference bounds (1,024 sites, depth
16), defensive execution failure, diagnostic fields and complete acceptance
matrices. These are engineering policy limits, not measured speed or sandboxing
claims. Validate the actual prepared corpus before reporting completion.

## Packet receipt and stop conditions

Record: baseline/new head; moved responsibility and semantic owner; old internal
paths removed; state/lifetime/rollback changes; phase ancestry/gates/deferred
visibility; focused tests actually selected and run; optional/profile evidence;
source graph as a diagnostic; unresolved behavior or maintainer choice.

Run relevant Rust tests plus these cheap checks when available:

```bash
git diff --check
python3 -m pytest -q scripts/tests/test_actor_spawn_boundary.py
python3 scripts/check_doc_links.py
python3 scripts/check_planning_citations.py
```

Run module/source regeneration only when the source move requires it. Preserve
existing architecture policy arguments/anchors when changing their owner.
Unavailable Rust/GPU/network prerequisites are INCOMPLETE, not PASS. Citation
failures caused by a shallow history must be reported separately from new broken
paths; do not erase historical citations just to make a local gate green.

Stop and revise the packet if the claimed state has an undiscovered writer, the
move needs a generic registry to preserve every old dependency, an absence test
requires an undeclared sibling, or a proposed behavior change lacks a policy
answer. A smaller coherent packet or an accepted internal cycle is preferable to
a completed checklist with the wrong authority.
