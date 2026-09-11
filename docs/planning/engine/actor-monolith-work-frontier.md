# Ownership migration packets

**Baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`, 2026-09-08.
**Authority:** [reassessment](architecture-reassessment.md),
[responsibility map](architecture-responsibility-map.md),
[decomposition rules](actor-monolith-decomposition.md).
**Priority is owned only by [the queue](../queue.md).** This file is a packet
catalog, not a second queue. A packet marked HOLD is not an implementation order.

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
    |
    +-- A3 construction placement adapter (independent; audit shared file edits)
    +-- A4 accepted control/body execution (writer map DELIVERED 2026-09-10)
    +-- A6 definitions / materialization (field census DELIVERED 2026-09-10)
    +-- A9 minimal-profile baseline now; closure changes by proven owner
    +-- A8 multi-instance world (HOLD for explicit customer/acceptance scenario)
    +-- A10 isolated publication (HOLD for required failure guarantee)
    +-- A12a raw flow validation (independent)
    +-- A11a installed support -> A11b exhaustive references
            -> A12b checked runtime -> A11c explicit activation
```

A3 and A9 measurements need not wait for A1. Do not implement A2c on top of known
geometry disagreement. A5 follows the resolved-contact contract because moving
all destructible state first would preserve an incorrect split interpretation.
There is no requirement to split accepted control across crates to make A4 green.

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
commits. Each commit must build/test at its own boundary. Avoid compatibility
re-exports under old internal paths. Do not broaden the packet because unrelated
lint, naming or policy work is nearby.

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

**HOLD on extraction:** first map writers and select production fixtures.
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

**Acceptance:** human -> possession -> brain -> human handoff on the same body;
competing claims, mount/dismount, removal of a controlled body, two participants,
rollback over handoff, action continuity and no double body tick. Distinguish
home-avatar, driver, camera and participant identities. Preserve legitimate
policy eligibility differences. Group an internal control/possession cycle if
that makes the invariant easier to inspect; do not require it to disappear.

## A5. Give destructible world objects their full transition authority

**HOLD until A2's contact contract is established and writer inventory is complete.**
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
(2026-09-10; 200 use sites, 27 fields, 9 consumer crates).
⇒ **And it REFUTES this packet's premise.** Nine fields are read by BOTH the
spawn road and the runtime, so the two-way split written below does not exist to
be finished — `autonomous_profile` alone is read by four crates at three moments.
Any A6 proposal starts from that table, not from this paragraph.

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
four crates with 3 of 16 sites in the crate that defines it. And `GroundItem` has
no constructor, so seven struct-literal sites across three crates each mint an
occurrence — including the death-drop path this packet's acceptance says must not
become an alternative minting road.

**HOLD:** after A1, enumerate item occurrence, holder, inventory and checkpoint
baseline writers. **Source:** monolith items/persistence/minted horizon,
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

**HOLD for a real simultaneous-room customer.** Current active-room specialization
is not itself a defect. Source: monolith world collision/active binding, world
RoomSet, shared SimId/lifecycle scopes, runtime room transition and portal/view
integration. Destination: spatial instance/residency authority plus coordinator.

Start with two live instances of one prepared room definition. Define stable
identity as a scoped occurrence, and keep definition revision, live instance,
session, participant and view identities separate. Do not introduce a universal
ID wrapper across unrelated domains. Determine save identity policy before
letting multiple instances write the same persistent occurrence key.

**Acceptance:** no cross-instance collision, lookup, observation or despawn;
portal/control transfer keeps body identity and ownership; unloading one instance
does not retire the other; same-room two-view rendering does not duplicate
simulation. Only then make required identity/query changes and budget residency.
Retain a small single-active-room host profile.

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

## A10. Constrain publication failure semantics only as far as required

**HOLD on architectural hardening; documentation correction is immediate.**
**Source:** shared construction executor/recipes, runtime prepare/commit/verify/
publish, domain construction services. **Problem:** raw Commands permit mutation
that the roster verifier can detect but cannot undo (F6).

Definition-level rejection and explicit activation are required by
[the authored-technique contract](authored-technique-admission.md); they do not
wait for inactive-world construction. For the stronger world guarantee, first
add fault-injection fixtures using a trusted invalid recipe: duplicate root,
root removal/identity mutation, undeclared occurrence and external-resource write.
Verify the host does not publish readiness or continue normal simulation after
failure. Preserve unchanged-world behavior on **preparation** failure.

If preserving the old live world across **commit** failure is required, create an
inactive candidate population and restrict writes to its scope. Stage non-entity
state explicitly and prove activation is the sole publication point. This may
justify a narrower construction context; it does not justify type-erased
executable registries or arbitrary World cloning. Do not claim a Rust plugin is
sandboxed, and do not call a unit-returning function infallible.

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
