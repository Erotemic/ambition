# Ownership migration packets

**Scope:** the A1-A12 packet catalog for moving engine responsibilities to
their owners. Each packet names a responsibility, not an SCC score.
**Doctrine:** [engine architecture](architecture.md) and
[actor residual-kernel decomposition](actor-monolith-decomposition.md).
**Priority is owned only by [the queue](../queue.md).** This file is a catalog,
not a second queue. Proposed file and API names below are design targets.

When a hold is discharged, rewrite the hold sentence in place to name what
discharged it. Do not add a banner above it.
`scripts/check_discharged_holds_are_rewritten.py` enforces this.

## Readiness map

```text
fresh source/behavior preflight for every packet
    +-- A1 checkpoint restoration (closed; verification widening open)
    |       -> A7 item custody/accounting separation
    +-- A2a geometry, A2b obstruction and swept targets, A2c contact
    |   recipient (landed) -> A5 destructibles
    +-- A3 construction placement adapter (lowering moved; acceptance open)
    +-- A4 accepted control/body execution (regrouping open)
    +-- A6 definitions / materialization (field census delivered)
    +-- A8 live world-instance isolation (several live rooms landed)
    +-- A9 minimal profiles
    +-- A10 bounded candidate construction (closed)
    +-- A11a/A11b installed support (landed) -> A12b checked runtime
            -> A11c explicit activation
```

A3 and A9 do not wait for A1. Do not build A2c on top of a geometry
disagreement. A5 follows the resolved-contact contract. Do not split accepted
control across crates to make A4 green. The
[extension packet catalog](fast-iteration-implementation.md) uses A6's census,
A9's closure method and A11/A12's admission; only specific domain ports wait on
A2/A4 contracts.

## Before any production edit

Record HEAD, the working tree, the ownership claim, callers, state writers,
lifetime, schedule phase and gates and the current behavioral witness. Line
numbers are locators; re-read the symbols on the working head.

```bash
git rev-parse HEAD
git status --short
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
python3 -m pytest -q scripts/tests/test_actor_spawn_boundary.py
```

A regex search is not a compiler dependency graph. Widen a query before you
report that something has no readers. Separate a semantic fix from a
behavior-preserving move. Avoid compatibility re-exports under old paths. Run
the [cheapest sufficient check](../../recipes/cheapest-sufficient-check.md) for
the touched invariant.

## A1. Checkpoint restoration belongs to session lifecycle

**State:** closed. **Owner:**
[checkpoint restoration protocol](checkpoint-restoration-protocol.md).
Restoration lives in `session::checkpoint`; healing and capture stay in
`shrine`. Domain reducers read one admitted operation; a refused reset changes
no occurrence, custody or owned-item state. The P2P road stays inert below
`commit_confirmed_lifecycle`'s `LocalSyncTest` gate. A coordinated peer-level
rule is a new packet.

**Open remainder:** widen post-apply verification beyond the ledger, the bag and
custody presence: checkpoint replay consequences, deferred flushes and the final
rollback baseline. Pin typed immutable checkpoint data on admission when
deferred application makes it load-bearing. No live-ledger swap and no generic
snapshot registry.

## A2. Establish coherent projectile contacts before removing family knowledge

**Owner:** [projectile contact protocol](projectile-contact-protocol.md), which
holds the live status. **Landed:** A2a (`apply_boss_hit` and the preflight read
the published `DamageableVolumes`; a boss has no fallback hull) and A2b's
obstruction half (both branches sweep the shot's box under its own
`WorldHitPolicy`). Projectile roads no longer use family predicates or construct
`UnresolvedFeatures`. A2b's swept-target half and A2c have also landed: contacts
over the traveled segment are ordered by time of impact, and a direct contact
names its recipient (`HitTarget::Feature`). The protocol doc's "Current shape"
table lists the witnesses. **Open:** only the optional move of pure flight helpers
to `ambition_projectiles`; do it when it removes an authority or a dependency
edge.

**A2c:** a direct contact names its recipient once. Interception and terminal
flight response happen during stepping; ordinary damage resolves in its later
phase from the targeted witness. No family re-query, no same-tick recursion and
no area-style reinterpretation. Pure flight helpers move to
`ambition_projectiles`; runtime owns ordering, not contact algorithms.

Preserve return-leg hit memory, one target per step on return, splash inclusion,
allegiance and one area event per hit. Land receiver-neutral return survival and
direct-before-splash ordering as explicit semantic corrections with their own
fixtures. Do not sort endpoint centers and call it continuous collision. The
destruction and surface identity tests are A5 prerequisites; they do not justify
one generic breakable crate.

## A3. Move authored placement lowering to the construction boundary

**State:** the lowering context lives in
`crates/ambition_platformer2d_actor_monolith/src/construction/placements.rs`.
`ambition_platformer2d_world` names no actor crate and `LoweringCtx<C>` is
generic. The character catalog and authored sheets ride on
`ActorConstructionContext`. Keep immutable room and placement records and the
provider-neutral lowering protocol at the world owner. Do not add a universal
placement enum or callback registry.

**Acceptance:** provider lowering and construction preflight tests; unknown kind
and conflicting registration; the same prepared plan and construction
fingerprint; first-tick volume, `SimId` and provenance; save-based preparation;
no second copy of catalog authority, exercised through the real registered
provider path. No gameplay or ordering change.

## A4. Co-locate accepted control and body execution

**State:** the writer map found no split to make. `InCustodyOf` carries
`CustodyDurability { Restored, SessionOnly }` with no `Default`, so every
producer decides durability. `ambition_held_items` writes `Restored`;
`body_custody::project_body_custody` writes `SessionOnly` (riders, limbs,
possessions). The claim value is `ControlClaims` / `ControlClaimant`
([control authority](control-authority-and-ai-policy.md)); add no further
abstraction.

**Source regions:** `control/authority.rs`, `control/input_systems.rs`,
`control/possession.rs`, `body_custody.rs`, live actor clusters, `avatar` and
`features/ecs/actors/update.rs`. **Destination:** coherent actor and control
modules in the same package. Possession eligibility stays optional ability
policy. Generic motion stays at its body owner.

`BodyAnimFacts` is rollback state (`actor.animation_facts`); its writers run in
the deterministic simulation. Decay lives at
`ambition_characters::actor::advance_body_anim_overlays`; arming stays in the
monolith because it reads engine events.

**Standing invariant:** `control::body_driving_seat` refuses a seat held by two
bodies, and the refused seat writes `ActorControlFrame::neutral()` because
`ActorControl` is latched. Recovery comes from the claim going away, so the
refusal is a per-tick resolution, not a latch.
`project_driving_participant` retracts a stale holder only during a possession
and only for `PlayerSlot::PRIMARY`; a duplicate claim on another seat has no
retraction road.

**Production arms:**

| acceptance item | arm |
|---|---|
| human -> possession -> brain -> human | `possession_end_to_end::a_player_can_possess_drive_and_release_an_actor_end_to_end` |
| competing claims | `competing_control_claims::a_seat_claimed_by_two_bodies_drives_neither_and_recovers_when_one_vacates` |
| mount and dismount | `a_player_pilots_a_mount_end_to_end`, `a_dead_mount_rebuilds_its_riders_brain_through_the_real_schedule` |
| rollback over handoff | `possession_survives_the_real_rollback_window`, `a_mount_dying_under_a_possession_survives_rewinds` |
| two mechanisms releasing independently | `a_mount_dying_under_a_possession_leaves_the_player_driving` |

**Open:** the regrouping; the distinction of home-avatar, driver, camera and
participant identities; arms for removal of a controlled body, two
participants, action continuity and no double body tick (the schedule-level
`no_system_is_registered_twice_in_one_schedule` cannot see one body ticked by
two systems).

## A5. Give destructible world objects their full transition authority

**State:** A2's contact contract is established and the writer inventory is
done. `Breakable::apply_damage` (`ambition_interaction`) owns the breakable
state machine; ECS writers orchestrate it. Breakables, chests and falling chests
do not share a transition authority: a breakable transitions through a
`#[must_use]` domain method, a chest through the `Opened` marker, a falling
chest not at all. There is nothing duplicated to consolidate yet.

Write-only authored fields are deleted. `ChestSpec::new(reward)` is the only
chest constructor; chest persistence is `encounter_reward_looted_flag`.
`features/ecs/world_overlay.rs::breakable_geometry_agreement` guards melee and
projectile geometry agreement. A published surface plus a damageable volume
gives one compound contact (Q96).

**Destination:** one logical destructible-object owner, initially a module.
Move live state, health and respawn transitions, collision contribution and
installation together. Do not pull all interactive objects into a breakables
crate. No `FeatureKind` dispatcher.

**Acceptance:** hit, stand and pogo eligibility; simultaneous contacts; one
break transition; respawn and collision restoration; geometry agreement; room
replay and save policy; no duplicate effects or rewards; headless operation.
Player-only stand eligibility stays until it is separately decided.

## A6. Separate prepared character definitions from live materialization/policy

**State:** [`prepared-definition-field-census.md`](prepared-definition-field-census.md)
refutes a two-way split: nine fields are read by both the spawn road and the
runtime. All production runtime reads resolve through
`PreparedCharacterRegistry`; the only second authority is the registry versus
`CharacterCatalog`. `autonomous_profile` has one resolution in
`crates/ambition_characters/src/prepared.rs`. Movement tuning is folded at the
barrier, and the wear road reads the prepared cast only.

**Destination:** logical prepared-definition, action-execution, intent-policy,
asset-materialization and session-activation owners. Keep a tightly coupled
action schema and executor together. Move texture readiness out of schema and
deterministic simulation. No brain or catalog path requires a render install to
select an action.

**Acceptance:** one character definition supports headless and windowed runs,
player and CPU, alternate action schemes and equipment; no duplicate movement
authority; preparation errors name a field and source; hot activation does not
mix mechanical and visual values from different revisions.

## A7. Separate item custody/accounting from lifecycle orchestration

**State:** A1 is closed. [Item custody and accounting](item-custody-and-accounting.md)
owns the item writers. The checkpoint baseline is written only inside the monolith;
`OwnedItems` is written from four crates. `GroundItem` is `#[non_exhaustive]`
with `at_rest` and `released` as its only constructors. Every death drop goes
through `spawn_death_drop` with a `DropIdentity` that has no default
(`every_death_drop_is_room_scoped_and_states_its_parent`). Not a generic item
request bus.

**Destination:** domain-owned custody and accounting modules plus explicit
session horizon integration. Item ordering stays item-owned. Session
coordinates capture and restore boundaries only. No save-every-component
reflection.

**Acceptance:** collectibles without held items; a thrown persistent weapon
keeps per-item identity; no double acquisition across resimulation; capture
after settlement; death, reset and room retirement with foreign controlled
bodies; item absence does not suppress session restoration.

## A8. Prove live world-instance isolation before generalizing residency

**State:** several live rooms are landed (see
[engine architecture](architecture.md#converged-shape) and
[open-world planning](open-world-runtime-and-residency.md), OW1).
`SimId::placement(id)` is `"placement:{id}"` with no room or instance scope; a
second live instance of one prepared room is refused at publication
(`DefinitionAlreadyLive`, OW3). `GeoSource::TileLayer { layer }` is
level-scoped only by a string convention. Run
`scripts/measure_identity_instance_scope.py` for the per-constructor table.

**Open:**

1. Two instances of one prepared room with identical placement IDs: qualify the
   values that lose instance scope at their owner. No universal ID wrapper.
2. Define the saved occurrence namespace before two instances persist one local
   placement. Candidate and reload IDs never become save identity.
3. Route queries and transfers through scoped owners; one body and one custody
   writer during handoff.
4. Only then add residency interests, population changes and budgets (Q94).

**Acceptance:** no cross-instance collision, observation, lookup or despawn;
transfer preserves body and custody identity; unloading one instance leaves the
other intact; two views of one instance do not duplicate simulation. The
one-room profile is the same implementation with one instance.

## A9. Prove public profiles and actual compile/runtime optionality

**State:** the facade -> host -> render edge is cut (the host's presentation
plugins sit behind its `render` feature; the world map is the runtime's `map`
feature). `the-featureless-facade-links-none-of-these` ratchets this.
`fixtures/headless_profile` is the headless consumer; `fixtures/minimal_game` is
the windowed sentinel. `scripts/measure_minimum_profile_closure.py --minimum`
prices a profile; cutting one edge saves little because the hubs reach the same
crates.

**Maintainer ruling needed (Q108):** the capability set of the minimum profile.
Each capability is a pair of edits: manifest feature gates and hub gates.

**Open:** separate tests for headless body and world; windowed body and world;
combat without inventory, bosses or dialogue; collection without held use;
generic encounters without boss content. A missing prerequisite fails with a
diagnostic, not a dummy sibling. Then migrate consumers off internal mirrors and
delete them. Measure build time and binary size separately.

## A10. Constrain candidate construction for reliable development reload

**State:** closed. **Owner:**
[construction and reconstitution](construction-and-reconstitution.md).
Candidate roots carry `InactiveCandidate`, a registered disabling component;
publication removes it. `ConstructionPlan::commit_inactive` refuses with
`InactiveCommitRefused::FilterNotInstalled` when the filter is not registered.
`publish_candidate` and `retire_candidate` return how many roots they touched.
Recipes receive `ConstructionRootCtx` / `RootScope`, which expose no `Commands`;
do not add an accessor that returns `&mut Commands`. Isolation covers planned
roots only. General unsafe-plugin recovery and arbitrary save migration are out
of scope.

## A11. Make installed technique support a preparation contract

**Owner:** [authored technique admission](authored-technique-admission.md).
A11a (support declared by the installer, `InstalledTechniques`) and A11b
(`MoveSpec::effect_refs`, refusal before publication) are landed.
**Open: A11c**, the end-to-end authoring route after A12b: edit ->
selected-profile preparation -> headless fixture -> review -> explicit activation
at a session or reconstruction boundary. Last-good prepared definitions are
required; last-good-world retention after native failure is not.

## A12. Align flow validation, prepared representation and execution bounds

This A12 is flow validation. `queue.md`'s
[`A12`](../queue.md#a12--finish-move-contact-attribution-and-reflection-identity)
is move-contact attribution, a different subject on the same `MovePlayback`. Say
*flow bounds* or *contact attribution*.

**Owner:** [authored technique admission](authored-technique-admission.md).
A12a is landed (`TechniqueFlow::problems`). A12b is half landed: edges are
`u16` and the per-tick clone is gone (`Arc<MoveSpec>`). **Landed 2026-10-07:** a structurally invalid flow is refused at admission and
its character withheld (an earlier note here said the boundary already did; it
only reported). No stored checked-flow type: see the invariant in the owner doc.
**Still open:** no prepared revision is pinned on the playback (the road it
guards is closed today); the move start deep-clones a `MoveSpec` (median
0.81 us); prepared definitions' fields are still public. Do not create a VM,
a per-beat signal bus or a code registry to fix this.

## Packet receipt and stop conditions

Record the base and new head, the moved responsibility and owner, removed
internal paths, state, lifetime and rollback changes, phase ancestry and gates,
the focused tests that ran, profile evidence and open questions. Run:

```bash
git diff --check
python3 -m pytest -q scripts/tests/test_actor_spawn_boundary.py
python3 scripts/check_doc_links.py
python3 scripts/check_planning_citations.py
```

Stop and revise the packet if the claimed state has an unknown writer, the move
needs a generic registry, an absence test needs an undeclared sibling or a
behavior change has no policy answer. A smaller coherent packet beats a
completed checklist with the wrong authority.
