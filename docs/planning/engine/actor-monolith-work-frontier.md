# Ownership migration packets

**Baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`, 2026-09-08.
**Authority:** [reassessment](architecture-reassessment.md),
[responsibility map](architecture-responsibility-map.md),
[decomposition rules](actor-monolith-decomposition.md).
**Priority is owned only by [the queue](../queue.md).** This file is a packet
catalog, not a second queue. A packet marked HOLD is not an implementation order.

When a hold is discharged, rewrite the `**HOLD:**` line itself to name what
discharged it and when. Do not add a banner above the row: the banner is a second
copy of the fact, and the hold line below it then states a condition that is no
longer true. `scripts/check_discharged_holds_are_rewritten.py` enforces this.

The old mandatory P2/P3/P4 sequence is retired. P1 settlement and the corrected
spawn extraction remain landed facts. A1-A12 name responsibilities, not SCC
scores. Proposed file/test/API names below are design targets, not existing code.

## Dependency and readiness map

```text
fresh source/behavior preflight for every packet
    |
    +-- A1 checkpoint restoration ownership (closed; verification widening open)
    |       -> A7 item horizon/custody separation (inventory delivered)
    |
    +-- A2a shared boss geometry (landed) -> A2b world obstruction (obstruction half landed)
    |       -> A2c contact seam -> A5 destructibles
    |
    +-- A3 construction placement adapter (independent; audit shared file edits)
    +-- A4 accepted control/body execution (regrouping and identity work open)
    +-- A6 definitions / materialization (field census delivered)
    +-- A9 minimal-profile baseline now; closure changes by proven owner
    +-- A8 two-instance scope proof (customer established; OW1/FI9 first)
    +-- A10 bounded candidate construction (closed)
    +-- A12a raw flow validation (landed)
    +-- A11a installed support (landed) -> A11b exhaustive references (landed)
            -> A12b checked runtime (two of four) -> A11c explicit activation
```

A3 and A9 measurements need not wait for A1. Do not implement A2c on top of known
geometry disagreement. A5 follows the resolved-contact contract because moving
all destructible state first would preserve an incorrect split interpretation.
There is no requirement to split accepted control across crates to make A4 green.

## Relationship to fast iteration

The [extension packet catalog](fast-iteration-implementation.md) is the bounded
continuation for pure authoring, portable artifacts and procedural state. It uses
A6's field census, A9's resolved-closure method and A11/A12's existing admission.
Only specific domain ports wait on A2/A4 contracts <!-- hold-ok: the half still held is A2a/A2b/A2c, the geometry, obstruction and recipient-naming contracts owned here; A2's construction-identity hole is closed and is a different subject -->. A8 starts with the
long-term world's two-instance proof. Neither A8 nor A10 blocks I1/I2, and
arbitrary-world undo is not the requirement. Do not expand I1's pure helper move
into an actor SCC extraction. The extension catalog and this catalog both take
priority from [the queue](../queue.md).

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
compiler dependency graph. A negative grep is a claim about the query: widen the
query before you report that something has no readers.

Separate a semantic fix from a behavior-preserving ownership move into different
commits. Avoid compatibility re-exports under old internal paths. Do not broaden
the packet because unrelated lint, naming or policy work is nearby.

`AGENTS.md` is the authority on test scope; this document does not restate it.
See [the cheapest sufficient check](../../recipes/cheapest-sufficient-check.md):
run targeted checks for the touched invariant, not the whole lane per commit.

## A1. Checkpoint restoration belongs to session lifecycle

**State:** closed. All seventeen rows of the acceptance matrix have a witness
that fails for that row's property. **Normative owner:**
[checkpoint restoration protocol](checkpoint-restoration-protocol.md). Follow its
state machine, source/destination table, commit ordering and acceptance matrix;
this synopsis is not an alternative implementation recipe.

The protocol fixes one semantic authority: selected checkpoint, original subject,
room reconstruction and participating domain snapshots describe one admitted
operation. Every domain reducer reads one admitted operation; a refused reset
changes no occurrence, custody or owned-item state. Restoration lives in
`session::checkpoint`; healing/capture stays in `shrine`.

The host-side abandonment note crosses into rollback state only after the
session-ownership gate and confirmation, and names its operation by value
(`an_abandoned_operation_waits_for_its_admitted_frame_to_be_confirmed`,
`a_note_from_a_rewound_branch_cannot_end_the_operation_that_reused_its_key`).
The P2P road stays inert: terminalization sits below
`commit_confirmed_lifecycle`'s `LocalSyncTest` gate. A coordinated peer-level rule
is a new packet, not an A1 row.

**Open remainder:** widen post-apply verification beyond the ledger, the bag and
custody presence. Include checkpoint replay consequences, deferred flushes,
verification and the final rollback baseline. Pin typed immutable checkpoint data
on admission at the point deferred application makes it load-bearing. No
live-ledger swap to prepare a candidate and no generic snapshot/restore registry.
A trusted failure after destructive application is fail-closed publication, not
proof that arbitrary Commands were undone.

Do not ship mixed raw-request and selected-candidate restoration in one profile.
No SCC number is an acceptance condition.

```bash
cargo test -p ambition_platformer2d_actor_monolith --lib checkpoint
cargo test -p ambition_platformer2d_actor_monolith --lib shrine
cargo test -p ambition_platformer2d_runtime --lib checkpoint
python3 -m pytest -q scripts/tests/test_actor_spawn_boundary.py
```

Use the existing app integration harness for canonical reconstitution, carried
items and death restoration; check the actual selected test count.

## A2. Establish coherent projectile contacts before removing family knowledge

**Ready:** A2c, and A2b's swept-target half. A2a and A2b's obstruction half have
landed: `apply_boss_hit` takes the published `DamageableVolumes`, and both
projectile branches sweep the shot's box under the shot's own `WorldHitPolicy`.
See `F2`/`F3` in [review findings](architecture-review-findings.md).
**Normative owner:** [projectile contact protocol](projectile-contact-protocol.md).
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

**State:** preflight done. `ambition_platformer2d_world` names no actor crate in
its manifest and its `LoweringCtx<C>` is already generic, so the crate-level edge
is gone. The character catalog and authored sheets now ride on
`ActorConstructionContext` instead of bare positional arguments. Inside the
monolith's `src/world/` region the remaining coupling is `placements.rs` and
`rooms/stage.rs`; the lowering functions are already under
`features/ecs/spawn/**`.

**Ready:** focused preflight on the new HEAD. **Problem:** the world region hosts
actor-specific catalogs, prepared-character/materialization inputs and lowering
functions.

**Source:** `crates/ambition_platformer2d_actor_monolith/src/construction/placements.rs`.
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

**Hold discharged 2026-09-10** by
[`accepted-control-writer-map.md`](accepted-control-writer-map.md), which gives
the writer map and the fixture selection. The map found no split to make.
`InCustodyOf` carries `CustodyDurability { Restored, SessionOnly }` with no
`Default`, so a producer must decide durability.

**Source regions:** `control/authority.rs`, `control/input_systems.rs`,
`control/possession.rs`, `body_custody.rs`, live actor clusters,
`avatar` integration and `features/ecs/actors/update.rs`, all inside the monolith.

**Responsibility:** accepted driver relation, input projection, live body execution
and custody reconciliation. **Destination:** coherent logical actor/control
modules in the same package first. Possession eligibility remains an optional
ability policy; the accepted relation is not that policy. Generic motion remains
at its established body owner. The claim/result value the writer map asked for is
`ControlClaims`/`ControlClaimant` (see
[`control-authority-and-ai-policy.md`](control-authority-and-ai-policy.md)); add
no further abstraction.

`advance_body_anim_overlays` lives at
`ambition_characters::actor::advance_body_anim_overlays`, beside `BodyAnimFacts`.
Arming stays in the monolith because it reads engine events; decay reads only the
component. `BodyAnimFacts` is rollback state (`actor.animation_facts`), so its
writers must run in the deterministic simulation. Do not reclassify it as
presentation because of its field names. It does not affect hitbox geometry:
`attack_support::player_attack_hitbox` takes its row from the attack intent.

**Acceptance:** human -> possession -> brain -> human handoff on the same body;
competing claims, mount/dismount, removal of a controlled body, two participants,
rollback over handoff, action continuity and no double body tick.

These items already have production arms:

| acceptance item | arm |
|---|---|
| competing claims | `a_seat_claimed_by_two_bodies_drives_neither_and_recovers_when_one_vacates` (`competing_control_claims.rs`) |
| mount/dismount | `a_player_pilots_a_mount_end_to_end`, `a_dead_mount_rebuilds_its_riders_brain_through_the_real_schedule` (`player_pilots_mount_end_to_end.rs`) |
| rollback over handoff | `possession_survives_the_real_rollback_window` (`rollback_provoked_actor.rs`) and `a_mount_dying_under_a_possession_survives_rewinds` (`carried_item_crosses_rooms.rs`) |
| two mechanisms releasing independently | `a_mount_dying_under_a_possession_leaves_the_player_driving` |

**Open remainder:** the regrouping into coherent actor/control modules; the
distinction of home-avatar, driver, camera and participant identities; and arms
for removal of a controlled body, two participants, action continuity and no
double body tick. Preserve legitimate policy eligibility differences. Group an
internal control/possession cycle if that makes the invariant easier to inspect;
do not require it to disappear.

## A5. Give destructible world objects their full transition authority

**Hold discharged 2026-09-11:** the writer inventory is
[`destructible-writer-inventory.md`](destructible-writer-inventory.md) (six
production mutation sites, three crates), and A2's contact contract closed at
`0157476ba`.

The inventory answers this row's conditional: breakables, chests and falling
chests do not share a transition authority. A breakable transitions through a
`#[must_use]` domain method, a chest through the `Opened` marker component, and a
falling chest not at all (it is a position tick). There is nothing duplicated to
consolidate, so the destination below is not yet earned; see the inventory page.

Write-only authored fields were deleted rather than guarded, so they cannot
return. `ChestSpec::new(reward)` is the only chest spec constructor. If a product
needs an authored opened chest, add the field and its lowering together. Real
authority for chest persistence remains `encounter_reward_looted_flag`.
`features/ecs/world_overlay.rs::breakable_geometry_agreement` guards
melee/projectile geometry agreement: both publishers read one `CenteredAabb`, and
their eligibility predicates diverge on purpose.

**Source:** `ambition_interaction` breakable definition,
`crates/ambition_combat/src/breakables.rs`, monolith feature bundles/spawn/target
publication/damage, world placement and simulation-view adapters.
**Destination:** one logical destructible-object owner, initially a module;
independent reusable capability/package only if a minimal customer justifies it.

Move live state, health/broken/respawn transitions, collision contribution and
runtime installation together. Authored lowering stays a typed domain adapter;
render extraction consumes outcomes/geometry. Do not pull all interactive objects
into a new breakables crate.

**Abstraction:** a bounded destructible state/outcome contract, only if current
components cannot already express it. No `FeatureKind` dispatcher for every game.
**Acceptance:** hit vs stand vs pogo eligibility, simultaneous contacts, one break
transition, respawn/collision restoration, melee/projectile geometry agreement,
room replay and save policy, no duplicate effects/rewards, headless operation.
Player-only stand eligibility is preserved until separately decided.

## A6. Separate prepared character definitions from live materialization/policy

**Hold discharged 2026-09-11** by
[`prepared-definition-field-census.md`](prepared-definition-field-census.md).
The census refutes the packet's premise: nine fields are read by both the spawn
road and the runtime, so a two-way split does not exist to be finished. Any A6
proposal starts from that table.

The dual-read axis is not the authority axis (ruled 2026-09-11). All sites read
one struct, and all production runtime reads resolve through
`PreparedCharacterRegistry`. The only second authority is
`PreparedCharacterRegistry` versus `CharacterCatalog`:

- `autonomous_profile` has no authority split: the catalog's
  `autonomous_profiles` is a name-to-profile library for
  `AutonomousPolicy::Named`, and `default_brain` names a `BrainPreset`, a
  different vocabulary. One resolution exists, in
  `crates/ambition_characters/src/prepared.rs`.
- `movement_tuning`/`motion_model` are folded by the barrier, so the acceptance
  line "no duplicate authored movement/tuning authority" is met. The fold is
  spelled twice: the read-time fall-back in `avatar/starting_character.rs`.
  Removing it reddened six tests on the wear/re-wear road.
  **Open question:** what an unprepared id should inherit at wear time. That is
  design, not cleanup.
  `game/ambition_app/tests/authored_feel_reaches_the_prepared_cast.rs` keeps the
  orphan case from arising meanwhile.

Not a resolver, not a bus, and no type moves; see the census page for why.

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

**Hold discharged:** A1 closed 2026-09-09, and the enumeration is
[`item-writer-inventory.md`](item-writer-inventory.md) (2026-09-10). It reports
the inverse of this packet's framing: the checkpoint baseline family is written
only inside the monolith, while `OwnedItems` is written from four crates with a
minority of sites in the crate that defines it. Re-run the page's instrument for
current counts.

Closed, do not reopen: `GroundItem` is `#[non_exhaustive]` with `at_rest` and
`released` as its only constructors. All four death drops go through one spawn
road, `spawn_death_drop`, which owns `SpawnOrigin::Dynamic`, `RoomScopedEntity`
and `SpawnedThisAttempt` and takes a `DropIdentity` with no default.
`every_death_drop_is_room_scoped_and_states_its_parent` guards the class. The
remedy is not a generic item-request bus; the inventory page says why.

**Source:** monolith items/persistence/minted horizon, `ambition_world_items`,
`ambition_held_items`, combat held/worn state and shared custody/occurrence
vocabulary. **Destination:** domain-owned custody/accounting modules plus
explicit session horizon integration.

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

**Identity axis, measured** (`scripts/measure_identity_instance_scope.py`): `SimId`
is a newtype over `String` whose only constructors are on `impl SimId`. No
constructor takes a room or an instance. `SimId::placement(id)` is
`"placement:{id}"`, so two instances of one prepared room mint the same identity
for every authored placement; `SimId::encounter(id)` has the same shape, and the
derived constructors inherit no scope. Today a second instance fails loudly: the
construction planner refuses with `IdentityAlreadyLive`
(`crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs`). Run the
script for the per-constructor table.

`GeoSource::TileLayer { layer: String }` is level-scoped only by a string
convention (`"{level}/{layer}"` in `ldtk/intgrid.rs`). The A8 fix is to make the
level a field, not to add a rule about the string. Open: whether the occurrence
ledger's `outlook_for(room: &str)` is a second place where scope is a convention
rather than a type.

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

**State:** the facade -> host -> render edge is cut: the host's presentation
plugins sit behind its `render` feature, and the world map is the runtime's `map`
feature. `the-featureless-facade-links-none-of-these` ratchets this over the
feature-resolved tree. `ambition_menu` is forbidden only in the featureless
profile; the host links it under `render` for the `MenuFont` handoff. The
separate headless consumer workspace exists (`fixtures/headless_profile`);
`fixtures/minimal_game` is the windowed sentinel and legitimately links the
renderer.

`scripts/measure_minimum_profile_closure.py` prices a profile. Each crate in the
featureless closure is held by two roads at once: the facade names most of them
directly, and the hubs reach the same crates from underneath. So a cut of any one
edge saves almost nothing; ask what a profile costs (`--minimum`), not what one
cut saves. Encounters/cutscene/dialog/items/persistence arrive through the
monolith-and-runtime hub; audio's one non-monolith path rests on two genuine uses
(the provider's `LoadExperienceSpec` and shell route audio). Both are ownership
questions, not residue.

**Maintainer ruling needed (Q108):** name the capability set of the minimum
profile. The script prices any set. Each capability is a pair of edits: one
manifest's feature gates plus the matching hub gates.

**Ready now:** record the manifest lower bound and establish a real independent
consumer fixture. **Implementation:** staged with the owners whose dependencies
need splitting; do not wait for every monolith region to be extracted.

**Source:** facade Cargo features/app builders, host/runtime dependencies,
`fixtures/minimal_game`, public namespace mirrors and full game composition.
**Destination:** explicit SDK profiles and host/simulation/presentation seams.
The intended minimum is a constructed body advancing against world geometry,
without renderer, audio, inventory, encounters or game content; this is a target,
not a passing current profile.

Record actual `cargo metadata --format-version 1` and `cargo tree -e features`
for the headless consumer manifest, plus runtime installed resources/systems.
Trace every alternate path. Do not infer absence from one optional edge or from
link-time dead-code removal.

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

**State:** closed. Every road that can change the authoritative world crosses its
own publication's verdict. The owner is
[construction and reconstitution](construction-and-reconstitution.md); the
acceptance arms are in the queue's A10 row.

Standing design facts from this packet:

- Candidate isolation uses a registered Bevy disabling component,
  `InactiveCandidate`, so `DefaultQueryFilters` hides candidates from every query
  that does not name it. Publication is a component removal, not a transfer.
  `ConstructionPlan::commit_inactive` refuses with
  `InactiveCommitRefused::FilterNotInstalled` if the filter is not registered,
  because an unregistered disabling component isolates nothing.
- A query that does not name `InactiveCandidate` returns zero candidate roots.
  `publish_candidate` and `retire_candidate` return how many roots they touched so
  a caller can assert that it moved the transaction it built.
- Recipes receive `ConstructionRootCtx` / `RootScope`, which hold no `Commands`
  and offer no `spawn`. A recipe cannot mint an authoritative entity outside the
  plan. `EntityScope` is the session-less half for helpers that finish an entity
  whose ownership someone else settled. `queue_component_mut` exists so that no
  `Commands::queue` closure is exposed. Do not add an accessor that returns
  `&mut Commands` from the construction surface.
- Isolation covers planned roots only. Other systems running in the same frame
  can still mint roots; the `an_authoritative_root_minted_outside_the_plan_…` arm
  pins that shape.

Out of scope for this packet: general unsafe-plugin recovery and arbitrary
save/schema migration.

## A11. Make installed technique support a preparation contract

**Ready:** A11c only. A11a and A11b have landed.
**Normative owner:** [authored technique admission](authored-technique-admission.md).
This is a bounded validation catalog, not an executable service locator. `F7` in
[review findings](architecture-review-findings.md) carries the verification of
A11a and A11b.

| sub-packet | state | what holds it |
|---|---|---|
| **A11a** | landed | `TechniqueSupport` is declared by whoever installs the handler; `declare` refuses a second claim on one key. The composition's table is `InstalledTechniques`. Refusals: unknown → `TechniqueRefusal::Unknown`; unsupported site → `WrongSite`; invalid parameters (including non-finite) → `BadParams`; paramless → `TechniqueParams::None` + `UnexpectedParams`. A disabled capability is `Unknown` by construction, because a key is in the table only if its handler is installed |
| **A11b** | landed | `MoveSpec::effect_refs` destructures without `..`, so a new effect site is a compile error. `unsupported_authored_effects` joins it to the table. `activate_staged_revision` returns `RevisionAdmission::Refused` against the whole candidate and leaves the active registry unchanged. `the_barrier_closes_through_the_checked_road_under_the_real_lifecycle` drives `finish()`/`cleanup()`/`update()` |
| **A11c** | ▢ open | the end-to-end authoring route. `an_edited_pack_reaches_the_cast_the_shipped_composition_plays` is adjacent, not this: it does not exercise review or an explicit activation boundary |

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

This A12 is flow validation, representation and execution bounds. `queue.md`'s
[`A12`](../queue.md#a12--finish-move-contact-attribution-and-reflection-identity)
is move-contact attribution and reflection identity, a different subject. Both
live on `MovePlayback`. When you write about either, say *flow bounds* (here) or
*contact attribution* (the queue).

**Ready:** A12b's remaining half. A12a has landed. The exact
algorithm/clock/delivery rules are in
[authored technique admission](authored-technique-admission.md). `F8` in
[review findings](architecture-review-findings.md) carries the verification of
A12a.

| sub-packet | state | what holds it |
|---|---|---|
| **A12a** | landed | `TechniqueFlow::problems` enforces `MAX_TECHNIQUE_FLOW_NODES`, in-range edges from `Self::successors`, a finite positive wait timeout, reachability of `Finish`, and `first_cycle` for the acyclic rule |
| **A12b** | ◐ two of four | Done: edges convert once (`FlowNode`'s edges are `u16`), and the per-tick graph clone is gone (`Arc::clone(&pb.spec)`). ▢ Prepared constructors are still public and infallible. ▢ No prepared revision is pinned on the playback: `MovePlayback` has no such field, and `Arc` gives a stable reference, not an identity. The move-start deep clone also remains: `StartingMove.spec` is a `MoveSpec` by value, once per accepted move |

The 1-256 node bound is a readability contract, not cursor safety.

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
Unavailable Rust/GPU/network prerequisites are incomplete, not a pass. Citation
failures caused by a shallow history must be reported separately from new broken
paths; do not erase historical citations just to make a local gate green.

Stop and revise the packet if the claimed state has an undiscovered writer, the
move needs a generic registry to preserve every old dependency, an absence test
requires an undeclared sibling, or a proposed behavior change lacks a policy
answer. A smaller coherent packet or an accepted internal cycle is preferable to
a completed checklist with the wrong authority.
