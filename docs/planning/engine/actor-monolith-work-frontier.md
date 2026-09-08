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
    +-- A1a checkpoint admission characterization/repair
    |       -> A1b checkpoint restoration ownership
    |           -> A7 item horizon/custody separation (after writer inventory)
    |
    +-- A2a shared boss geometry -> A2b world obstruction -> A2c contact seam
    |                                                       -> A5 destructibles
    |
    +-- A3 construction placement adapter (independent; audit shared file edits)
    +-- A4 accepted control/body execution (HOLD until writer map and fixtures)
    +-- A6 definitions / materialization (HOLD until per-field dependency census)
    +-- A9 minimal-profile baseline now; closure changes by proven owner
    +-- A8 multi-instance world (HOLD for explicit customer/acceptance scenario)
    +-- A10 isolated publication (HOLD for required failure guarantee)
    +-- A11 installed-technique admission (independent, real flow customer)
    +-- A12 flow representation/bounds (independent validation fixes)
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

**Ready:** A1a characterization; A1b after A1a establishes the behavior.
**Problem:** startup/reset restore is filed as shrine behavior and installed
partly through held-item pickup, although it coordinates session admission,
loading and body placement. The pending slot is already at the coherent owner.
**New abstraction:** at most one session-owned installer and one semantic startup
ordering set if existing lifecycle phases do not express the current ordering.
No new crate, generic slot, provider registry or message bus.

### A1a: characterize and fix denied startup admission

Source:
`crates/ambition_platformer2d_actor_monolith/src/shrine.rs` and its tests.
Implement F1's occupied-slot / same-generation / cancelled-incumbent fixture.
Determine whether production permits the state and explicitly document the
startup guarantee. Recommended repair: update `routed_for` only after admission.
Do not change reset semantics, healing or the slot's first-admitted-wins rule.

Acceptance includes missing subject, successful admission deduplication, same-room
resume, missing destination and continued waiting while construction completes.
Do not introduce post-admission cancellation retry without a clear lifecycle
outcome; that is a separate state-machine extension.

### A1b: move restoration as one authority

**Move from** `shrine.rs`:
`CheckpointResumeProgress`, `restore_checkpoint_on_session_start`,
`resume_at_checkpoint_on_reset`, and restore-specific tests.
**Move to** proposed `src/session/checkpoint.rs` in the same monolith crate, with
its own test module. <!-- cite-ok: proposed file path -->
Leave `HealShrine`, `heal_save_shrine_system`, rest interaction, heal cues and
checkpoint capture trigger in shrine. Keep the current primary-avatar restore
policy; all eligible interacting bodies still heal, and one checkpoint is recorded
with the existing deterministic selection.

**Update these current source owners:**

| File | Required change |
| --- | --- |
| `crates/ambition_platformer2d_actor_monolith/src/session/mod.rs` | Publish the session-owned checkpoint module/installer through the intended internal owner |
| `crates/ambition_platformer2d_actor_monolith/src/items/pickup/mod.rs` | Remove progress initialization and startup-restore installation; leave item behavior and shrine healing unchanged in this narrow packet |
| `crates/ambition_platformer2d_actor_monolith/src/checkpoint_horizon.rs` | Separate the item-domain offer from session restoration; an item offer must not install session checkpoint state |
| `crates/ambition_platformer2d_actor_monolith/src/lib.rs` | Update typed installer exports, deleting internal stale paths |
| `crates/ambition_platformer2d_runtime/src/checkpoint_horizon.rs` | Install session restoration and common horizon ordering without unconditionally installing item-domain behavior |
| `crates/ambition_platformer2d_runtime/src/lib.rs` | Full composition explicitly includes the item horizon alongside item capability installation; the focused checkpoint profile omits it |
| `crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs` | Update progress type path, retain clone/checksum semantics and registration identity |
| `crates/ambition_platformer2d_actor_monolith/src/snapshot_impls.rs` | Check all moved-type references; slot snapshot behavior remains unchanged because the slot stays put |

Search tests/facade references and update actual callers. A proposed
`SessionCheckpointPlugin` is an installer, not a new service abstraction.
<!-- cite-ok: proposed plugin name -->
The existing actor-horizon wrapper can be removed if it only forwards to the
item offer after the move; do not preserve a wrapper with a false actor label.

### Schedule and lifetime invariants

1. Reset routing remains in `CheckpointRestore`, under PlayerInput and before
   `RoomReplayAdmission`. Preserve emitted `RoomReplayAdmitted` behavior on
   successful reset admission.
2. Startup restoration remains at its current simulation phase and is **not**
   accidentally placed under `GameplayGated`: it must work while the session is
   waiting for loading/body construction. Characterize the old ordering before
   replacing its incidental membership in `ItemPickupSet::CoreHeldItems`.
3. In the full item composition, placement/restore occurs before held-item release
   and checkpoint capture observes settled custody. Express the integration edge
   in the full composition, not as an unconditional item prerequisite of session.
4. Missing bodies leave restoration retryable; admitted cross-room transitions
   keep their original subject SimId; later control changes cannot retarget them.
5. Generation and rollback behavior of progress remain explicit. Preserve wire
   keys `resource.checkpoint_resume_progress` and
   `resource.pending_lifecycle_commit` in this ownership-only commit. This is a
   diff-isolation rule, not a cross-build compatibility promise.

### What becomes simpler / forbidden result

A checkpoint-only session no longer needs to know item pickup or construct a
shrine. A reader of shrine only needs rest/heal/capture semantics. The slot stays
owned by the lifecycle coordinator; moving it into shared_tangle fails A1.

Test a focused app containing only checkpoint prerequisites: save state, live room
selection, primary body/identity, lifecycle slot and required schedules. Omit
held items and shrine entities. Run startup and reset, then full-composition
custody/capture and rollback fixtures. Removing the item offer must not erase
session restoration; removing checkpoint restoration must make the fixture fail.

Suggested existing test selection plus newly added tests:

```bash
cargo test -p ambition_platformer2d_actor_monolith --lib checkpoint
cargo test -p ambition_platformer2d_actor_monolith --lib shrine
cargo test -p ambition_platformer2d_runtime --lib checkpoint
python3 -m pytest -q scripts/tests/test_actor_spawn_boundary.py
```

Confirm that filters actually selected the intended tests; a zero-test run is
not a passing receipt. Use repository harness instructions for the established
rollback lifecycle fixture rather than guessing a Cargo integration target.

## A2. Establish coherent projectile contacts before removing family knowledge

**Ready:** A2a/A2b targeted characterization; A2c only after their contract is
established. **Owners:** published simulation geometry, projectile travel,
victim reaction. **Direction afterward:** projectile -> spatial/contact facts;
victim reaction -> selected contact and its own state; no projectile -> named
boss/breakable behavior.

### Source/destination and staged edits

A2a touches target publication, boss preflight/application and associated tests:

- `crates/ambition_platformer2d_actor_monolith/src/features/ecs/target_volumes.rs`
- `crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage_predicates.rs`
- `crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage/boss_hit.rs`
- `crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage/mod.rs`
- `crates/ambition_combat/src/hitbox/mod.rs` only if the existing shared view needs
  a concrete correction; do not broaden the view preemptively.

Use one authored/fallback/empty geometry contract, as described in F2. Keep its
runtime publisher at the current integration owner until the invariant is proven.
Do not create a new geometry copy for projectiles.

A2b touches `crates/ambition_platformer2d_actor_monolith/src/projectile/systems.rs`
and the current projectile/world collision tests. First make F3's earlier world
obstruction constrain feature contact with the same finite body and world-hit
policy. Record any deliberate gameplay change separately from a later move.

A2c moves projectile contact selection/lifetime behavior toward
`ambition_projectiles` only after its lower inputs are sufficient. It removes
`ecs_hit_event_hits_breakable`, `ecs_hit_event_hits_boss` and boss catalog/query
knowledge from projectile stepping **when no production consumer remains**.
Victim-specific state mutation remains with the victim owner. Integration can
stay temporarily in the monolith without the projectile owner importing it.

### Interface constraints

Use current target geometry and body/occurrence identity where possible. Add a
small eligibility/contact-result type only for a semantic distinction existing
state cannot express. Document who writes eligibility and when it changes. A
`ProjectileFeatureTarget { ignore_key: String }` marker that preserves family
strings and unresolved re-query is not the target. A registry whose callback
runs arbitrary boss logic is worse.

Selected direct contact should identify its victim once; its consumer must not
re-run a broad family query and affect a different population. Existing transient
Entity handles may be used within one schedule; stable simulation identity is
needed for ordering, replay/longer-lived references and disappearance checks.
Do not replace every Entity with a string. Splash/area effects have explicit
multi-target semantics. Contact-without-damage must remain representable.

### Invariants and acceptance matrix

Test authored override/empty volume; initially constructed targets on their first
tick; broken/non-hit/pogo-only destructibles; phase invulnerability; reflection
and absorption; returning shots; splash; already-hit retirement; thin wall,
corner clip, one-way and fast moving-body contact; equal-time tie order independent
of spawn order; body disappearance between selection and consumption. Preserve
trace/effect cardinality and attribution. Do not route synchronous projectile
termination through next-frame messages.

Measure source references after the behavioral tests, not instead of them.
Completion means a projectile reader no longer needs to understand boss animation,
rewards or breakable trigger implementation. It does not require an SCC of eight
or mandate a separate contact crate.

## A3. Move authored placement lowering to the construction boundary

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

**HOLD:** make a field/use census before moving types.
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

First add fault-injection fixtures using a trusted invalid recipe: duplicate root,
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

**Ready:** source/caller confirmation and focused admission tests; independent of
A1-A10. **Existing owner:** O4 in authored gameplay and authoring tools.
**Problem:** F7; the parameter registry is disconnected while a live flow can emit
String-keyed effects. **Source:** entity-catalog schema/registry,
character/content preparation, actual capability installers and technique
translators such as `game/ambition_demo_smash/src/capture.rs`.

**Destination:** a bounded installed-technique declaration owned by the same
capability offer that installs its handler. The current entity-catalog may hold
small metadata/validation vocabulary if its dependency direction stays downward;
profile composition selects the installed offers. Do not introduce an engine-wide
executable registry, move handler logic into preparation or install every game
technique merely so validation passes.

**Required contract:** each installed key declares owner, schema/revision,
parameter policy (explicitly paramless or validator), supported context and source
metadata. Unknown and not-installed fail preparation; known paramless succeeds.
Domain validators cover semantic ranges/unknown fields as appropriate, not only
whether serde can hydrate a value. One effect-reference traversal feeds validation,
discovery and reverse references; do not maintain independent hand-kept lists.

Default duplicate registration is an error even for function-valued entries.
Function equality is not needed to detect duplicate keys. If repeat installation
is supported, define and test explicit ownership/idempotence rather than comparing
function addresses or treating equal metadata as proof of equal implementation.
Freeze support before content preparation/activation. Runtime typed handlers keep
their current schedule and delivery semantics.

**Staging:** first reject an invalid flow Emit in a focused preparation fixture;
then wire the real installed subset and all effect-reference positions. Add the
public discovery projection after the support contract is authoritative. No claim
of complete validation until the reference traversal includes every applicable
nested payload; unsupported opaque payloads must declare that limit.

**Acceptance:** valid current `read_and_seize` behavior unchanged; unknown key,
invalid params, known-but-disabled capability and duplicate owner fail before a
body is spawned; a new external provider can declare/install/validate one effect
without editing an engine key switch. Keep runtime defensive diagnostics for
trusted Rust callers that bypass preparation. The source comment claiming
Conflict is impossible must be corrected in the implementation commit.

## A12. Align flow validation, prepared representation and execution bounds

**Ready:** targeted F8 tests. **Source:**
`crates/ambition_entity_catalog/src/lib.rs`, its tests, prepared moveset admission,
and `crates/ambition_combat/src/moveset/mod.rs` plus tests.
**Destination:** the existing move-scoped flow owner, not a new VM package.
**Abstraction:** a checked prepared index/flow representation only if needed to
ensure every execution path has validated data. No generic scripting runtime.

Reject nonfinite/nonpositive wait timeouts and graphs/targets not representable
by the executor. Establish one documented graph/work limit from the supported
authoring contract; the current u16 cursor is a representability ceiling, not a
performance budget recommendation. Enumerate current authored graph sizes before
choosing a smaller practical cap. Keep valid existing flows behavior-identical.

Do not claim existential Finish reachability proves termination of every branch.
Specify whether cycles are allowed under an explicit tick/work/move-duration
bound. Preserve proper-time clock, current per-occurrence contact latches and
normal move teardown. A second Wait after one contact currently reuses that
latched contact; per-beat signals require a separate scoped event identity design.

Remove the per-tick full flow clone only as a separately measured/isolated
implementation change, using immutable prepared data or disjoint field borrowing.
Do not promise frame-time gains before profiling. A wider runtime cursor changes
rollback state and is not the default solution to invalid authored input.

**Acceptance:** index 65,536 cannot wrap through a valid prepared flow;
representable boundary values are handled intentionally; positive infinity, NaN
and nonpositive timeouts fail; normal Emit/Wait/Branch/Finish, bounded cycles,
timeout and contact-latch fixtures still pass. Check direct trusted constructors
as well as content preparation so invalid data cannot be marked prepared by
calling a different public entry point.

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
