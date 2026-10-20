# Actor residual-kernel decomposition

**Scope:** the rules for carving responsibilities out of
`ambition_platformer2d_actor_monolith` (track D33), the carve safety map and the
exit condition.
**Doctrine and target authorities:** [engine architecture](architecture.md).
**Packets:** [ownership migration packets](actor-monolith-work-frontier.md).
**Priority:** [the queue](../queue.md).

## Goal

Keep a coherent body, control and action execution kernel. Move session
restoration, preparation, world-object behavior and content integration to
their owners. The residual crate is not the only mixed container: `combat`,
`characters`, `core` and `runtime` also hold mixed responsibilities.

## Current graph

Measure the intra-crate module graph with:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
```

**Re-measured at `be412a299768` (2026-10-20):** the largest component
has **ten** top-level modules: `abilities`, `avatar`, `body_whereabouts`,
`character_runtime`, `construction`, `features`, `items`, `projectile`,
`session`, `world`. The `assets` and `character_sprites` pair remains a
separate two-module cycle. The earlier eight-module count is no longer
current. The monolith has about 73,628 production source lines; `features`
accounts for about 26,665 of them. Size alone is not a carve reason.

| Directed edge | Qualified references | Meaning and likely action |
| --- | ---: | --- |
| `body_whereabouts -> session` | 3 | Lifecycle-aware custody adapter; **MOVE_ADAPTER** to transition integration |
| `construction -> session` | 1 | `CommitFactsSource::AfterTheRestore` checkpoint key; **MOVE_ADAPTER** for deferred facts selection |
| `avatar -> session` | 1 | Per-room rules in a generic body control gate; **HOLD** until the rules-input owner is clear |
| `character_runtime -> avatar` | 1 | Match activation reads initial-body policy; **MOVE_ADAPTER** with match/session setup |
| `items -> session` | 4 | Item setup and persistence reach lifecycle policy; **MOVE_ADAPTER** after A7 writer audit |
| `session -> items` | 20 | Restoration coordinates item state; **KEEP_DIRECT** while item domain owns the restored values |
| `features -> projectile` | 1 | Perception reads `ProjectileAllegiance`; **KEEP_DIRECT** |
| `projectile -> features` | 4 | Concrete boss/breakable contact work; **MOVE_ADAPTER** under A2 after contact parity |
| `world -> features` | 9 | Room feature plans and publication; **MOVE_AUTHORITY** as one room transaction packet |
| `world -> session` | 11 | Room publication reads checkpoint and lifecycle facts; **MOVE_ADAPTER** with that packet |
| `session -> world` | 3 | Session setup and publication handles; review as part of one transaction packet |
| `assets <-> character_sprites` | 2 / 3 | Catalog and art materialization; group or move adapter after its readers are known |

The counts come from `scripts/measure_kernel_module_graph.py`. Removing
`body_whereabouts -> session` and `construction -> session` from its in-memory
edge graph reduces the largest component from ten to eight. This is only a
**graph what-if**; no code was moved, and it does not prove that the resulting
crates can compile. One real cut can affect other paths not counted here.

The instrument counts textual `crate::<module>` paths inside one crate. It
misses many imported names, cross-crate dependencies, shared writers, message
timing, plugin prerequisites, macros and re-exports. Use
`scripts/module_graph.py --crate ambition_platformer2d_actor_monolith` to
inspect `use` edges as a second view. The generated ECS and Cargo indexes
show system registrations and declared/resolved crate dependencies, but the
resolved Cargo graph is feature-unified across the workspace. A consumer's
closure still needs its own check. A lower SCC count is evidence, not a goal.
Keep a cycle only when its shared invariant is stated.

## Landed carves

- Settlement state and the corrected actor-spawn extraction
  (`ambition_platformer2d_actor_spawn`, builders only). Do not move live actor
  views, provocation, fighter-ladder projection or dismounted-rider rebuild back
  into spawning. Guard: `scripts/tests/test_actor_spawn_boundary.py`.
- `ambition_conversation`, `ambition_encounter_features` and
  `ambition_abilities` (the wielded kit, the gravity grenade, traversal verbs and
  the shared cooldown).
- Checkpoint restoration lives in `session::checkpoint`; `shrine.rs` owns the
  interaction only.
- Placement lowering lives in `construction::placements`.

## Deliberately not carved

- `possession` (`control::possession`), `teleport`, `trapdoor` and `flyline`
  (`abilities/traversal/`) are control authority. The runtime registers their
  systems, and `body_custody`, `control::authority` and
  `control::input_systems` use them. Moving them would rename the coupling.
- `abilities/thrown/puppy_slug_gun.rs` spawns a body through the crate-private
  `features::spawn_runtime_minion`. The canonical seam is
  `ambition_vfx::Effect::Summon` with a `SummonSpec`, materialized as
  `ActorConstructionParams::SummonedMinion`. Routing the gun through it needs
  `SummonSpec` to carry `ActorAggression` and the ally marker. That is a
  behavior change, not a file move.

## Rules for every carve

1. **One packet, one ownership claim.** Separate a correctness fix from the
   mechanical move that follows it. List non-goals.
2. **The old internal path disappears.** Move consumers to the owner. Do not
   keep internal re-exports to avoid editing imports. A curated public facade
   re-export is a different, deliberate thing.
3. **State and lifetime travel together.** For each moved value, account for
   construction, writers, scope, retraction, rollback and checksum,
   reconstruction and observation. An unknown writer or lifetime holds that part
   of the packet.
4. **Preserve behavior and wire identity in a pure move.** Keep wire IDs and
   encoded meaning. A format change is a separate reviewed commit.
5. **Preserve scheduling semantics.** Record phase membership, ancestor gates,
   ordering, deferred flushes and the population that runs. Startup checkpoint
   restore must not move under a gameplay gate.
6. **A carved crate never depends back on the monolith.** Cargo forbids the
   production edge; the workspace policy also forbids the dev-dependency edge,
   which would drag the monolith's closure into the carved crate's tests.
7. **Put integration above the domains it joins.** For example, `interact`
   owns the interaction fact and asks a dialogue port for the dialogue fact, so
   `features` names no dialogue type.
8. **Tests witness consequences.** Use a production-path behavior fixture plus
   an absence or external-consumer fixture. A zero-test filtered run is not
   acceptance.
9. **Refresh citations in the same commit.** Update absence contracts (see the
   safety map), policy arguments and source citations for moved code.

A host that avoids a direct monolith dependency is a goal, not a ratchet. The
monolith still reaches the host through `ambition_platformer2d_runtime`.

### Edge dispositions

When a packet reviews a monolith edge, give it one disposition:

- **KEEP_DIRECT:** the consumer legitimately depends on this lower domain.
- **MOVE_AUTHORITY:** move behavior, state and installation together.
- **MOVE_ADAPTER:** keep the integration but move it to the integration owner.
- **GROUP_PACKAGE:** an internal cycle is part of one authority.
- **HOLD:** a named proof prerequisite is missing. No generic callback stands in.
- **REMOVE_AFTER_REPLACEMENT:** delete an obsolete projection only after its
  replacement covers every customer.

Standing examples: interactive objects reading `ActingParticipant` and
perception reading `ProjectileAllegiance` are KEEP_DIRECT. Session assembling
world and physics services is ordinary composition. `ActiveContentBinding`
stays with session; it is not a reason to add world IDs everywhere.

## Target package and integration boundary

Keep `ambition_platformer2d` as the public game-facing facade.
`ambition_platformer2d_runtime::PlatformerEnginePlugins` already composes
plugins and simulation phases. Do not add a second plugin-assembly crate.
Separate two kinds of integration work:

| Layer | Owns | Must not own |
| --- | --- | --- |
| `ambition_platformer2d_runtime` (exists) | Profile selection, installation, public phase ordering, schedule policy | Body algorithms and every feature transaction |
| `ambition_platformer2d_integration` (candidate) | Cross-domain simulation adapters and coordinated room-publication operations | General service lookup, named game content, domain state machines |
| Actor/body kernel | Accepted control, body custody, movement and action execution | Host load covers, checkpoint routing, content publishing |
| Construction, world, session, item and combat domains | Each domain's facts, transitions, validation and rollback state | Knowledge of the higher integration crate |
| Game providers and hosts | Selected content and device/presentation work | Engine-domain algorithms |

The name `ambition_platformer2d_integration` is **proposed**, not a crate
to create before it has a real owner packet. An `ambition_engine` catch-all
would not make the responsibilities clearer. Keep integration code in the
existing runtime where that is the narrowest present owner, then add the
integration crate **only if** the room-publication move gives it a stable
independent implementation and dependency direction.

The target dependency direction is host/provider -> runtime -> integration
-> domains -> foundations. Runtime may also install domain plugins directly.
Domains must not depend upward on runtime or integration. Dependency direction
does not dictate message flow or simulation order. A crate move must not add
scheduled systems by default. It may move a system body and leave its one
registration in the same phase.

## Dependency-first extraction sequence

These are **design packets**, not a new priority queue. AP14 in
[the execution queue](../queue.md#ap14--semantic-actor-monolith-scc-decomposition-continuous-deferred-2026-10-09)
selects when the work proceeds. The A-packet
[catalog](actor-monolith-work-frontier.md) owns the existing behavior proofs.

### K1 — Move transition custody interpretation upward

`body_whereabouts.rs::CustodyEndingAtCommit` depends on
`session::lifecycle_commit::LifecycleIntent` and
`rooms::another_player_stays`; the runtime transition loader is its
production consumer (`room_transition/loading.rs`). Move the adapter and
its tests near the room-transition integration path. Keep durable
whereabouts state and its ordinary readers in the domain. Do not make the
low-level body-location module import session operation types.

**Proof:** the same away-from-home population is released or retained after
crossing, checkpoint restore, same-room replay, and a crossing where another
player stays. Verify cancellation, rollback and two live rooms. Remove the
qualified edge and ensure no new registration or hidden query work is added.

### K2 — Move commit-facts *selection* above construction

`construction/mod.rs::CommitFactsSource` contains
`AfterTheRestore(CheckpointOperationKey)`. `world/rooms/stage.rs` resolves
that selector, while construction owns the resulting `PersistedFates`.
Move the *choice of when and where to get facts* to the publication/lifecycle
coordinator. Construction must still receive one resolved value for a commit.
Do not read a candidate session's save from the currently live session.
Do not evaluate checkpoint consequences before the operation is admitted.

**Proof:** a session prepared while another game runs; accepted and refused
checkpoint reset; replay after save changes; candidate failure; first-tick
spawn state. Remove `construction -> session` without adding a general facts
provider registry, new global resource, or duplicate saved state.

### K3 — Prepare actor construction for a domain owner

Recheck `construction -> fate_flags` and `construction -> shrine`, plus its
callers in `features` and `world`. Give death flags and shrine components a
semantic owner. Do not move a value to `shared_tangle` just to satisfy the
compiler. Check whether `ambition_platformer2d_actor_spawn` can own more
planning work without importing the monolith back into itself. If it cannot,
keep planning separate from spawn materialization. Reuse A3's prepared-plan,
`SimId`, provenance and preflight tests.

### K4 — Extract room-publication coordination as one packet

Inspect these together, not as separate file moves:

- `world/rooms/stage.rs` and `world/rooms/transaction.rs`;
- `features/ecs/spawn/mod.rs` (`RoomFeatureConstructionPlan` and receipt);
- `session/setup.rs`, checkpoint and durable-horizon admission;
- the current publication and rollback registration path.

Construction owns building a typed plan. Session owns admitted operations,
restore facts and retirement. World owns geometry and live-instance identity.
The coordinator owns preparing a hidden candidate, checking its receipt, and
publishing or refusing it as one operation. Keep `InactiveCandidate` and
`CandidateSessionRoot` protection, deferred flushes, rollback identity and
lifecycle cancellation intact. Do not export crate-private receipts merely
to relocate one file. Move the transaction with its required inputs and tests.
This is the first plausible reason to create the integration crate.

**Proof:** normal load; hot reload; checkpoint replay; failed candidate;
rollback-confirmed transition; two players in different live rooms; same
room definition in two instances when supported. Verify one publication
and one retirement, not two. A9 then verifies the selected consumer closure.

### K5 — Continue existing domain packets, then assess the residual kernel

Apply A7 to the `items <-> session` loop; keep item identities, custody and
accounting item-owned while session coordinates restore. Apply A2/A5 to
projectile contact arbitration; keep `ProjectileAllegiance` a legitimate
perception input. Apply A6 to session-specific character activation and
prepared definitions. Keep `avatar`, accepted control and body integration
together unless A4 finds an independent authority. Handle the
`assets <-> character_sprites` presentation cycle separately. Do not split
one coherent actor kernel only to make its internal module graph acyclic.

### Migration acceptance for each packet

Capture a before/after record with: removed file paths and edges, resolved
imports, API callers, type and state writers, construction/defaults,
rollback and checksum, ownership of plugin installation, phase order and
`apply_deferred` barriers, run conditions, teardown, tests and external
consumers. Re-run the graph and declared Cargo closure. A completed crate
move adds **zero active ECS invocations** unless a measured behavior need
justifies one. Keep normal and minimal headless consumer tests and compare
Android numbers if a hot system moved. See the
[system fusion plan](system-composition-audit.md#s7--consolidate-coherent-active-ecs-systems-proposed-2026-10-20).

## Index follow-up

The October 20 index is enough to select K1 and K2 and identify likely
owners for K3/K4. It does not yet prove the reason for each source dependency.
Extend the existing `scripts/module_graph.py` / agent index when a packet
needs it. A useful edge record has source and target module and symbol,
file and line, reference kind (`use`, type, function call, registration),
Cargo feature or cfg condition, and the active schedule/set for installed
systems. Human-reviewed edge disposition remains in this plan, not in a
machine classifier. Use the generation stamp to reject stale reports.

Do not write a new full-repository parser to start K1. Confirm its three
references and production consumer in source, implement the narrow move,
then use the result to decide if a durable edge report is needed. The
current `measure_kernel_module_graph.py` is a text-reference graph, and
`module_graph.py` only resolves `use` declarations; neither alone certifies
a Cargo carve.

## Packet receipt

Record the base and new head, the owner and moved operations, removed source
paths, the new dependency direction, state and registration changes, preserved
phase visibility, the tests that ran and the remaining limits. Record SCC
changes as diagnostics without a target.

## Exit

D33 exits when the residual crate owns only the body, control and action kernel,
every other responsibility has an explicit owner and no consumer needs the
crate's internal topology. A retained cycle has a stated shared invariant.
Independent capability profiles are a separate C2/SDK exit.

## Post-carve safety map

A carve that moves one of these owner files must update the corresponding absence
contract in the same commit. This table belongs here rather than in `queue.md`
because it is durable decomposition doctrine.

| If your carve moves… | Update these absence contracts |
|---|---|
| `crates/ambition_combat/src/moveset/mod.rs` | `ending-a-move-goes-through-the-one-teardown-path` |
| `crates/ambition_characters/src/brain/fighter`, `crates/ambition_characters/src/brain/state_machine/mod.rs`, `crates/ambition_characters/src/snapshot_impls.rs`, or `crates/ambition_characters/src/brain/mod.rs` | `the-generic-brain-does-not-grow-new-platform-fighter-edges` |
| `crates/ambition_platformer2d_actor_monolith/src/character_runtime/match_activation.rs` or `game/ambition_app/src/app/versus.rs` | `a-second-writer-of-a-match-global-must-answer-ownership` |
| `crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs` or `game/ambition_app/src/dev/rollback_observatory.rs` | `the-seat-topology-has-one-engine-side-creator` |
| `game/ambition_app/src/app/versus.rs` or `game/ambition_demo_smash/src/lib.rs` | `the-global-roster-is-retired-only-by-its-owner` |
| `tools/ambition_ldtk_tools/ambition_ldtk_tools/ldtk/paths.py` or `tools/ambition_ldtk_tools/tests/test_ldtk_core_helpers.py` | `the-worlds-path-is-confined-to-ldtk-paths` |
| `crates/ambition_characters/src/prepared.rs`, `crates/ambition_combat/src/worn_kit.rs`, or `crates/ambition_characters/src/actor/character_catalog/mod.rs` | `the-catalog-default-action-set-is-confined-to-one-file` |
| `crates/ambition_characters/src/smash_fighter/` | `the-smash-fighter-facet-is-read-only-by-its-owner` |
| `crates/ambition_platformer2d_actor_monolith/src/character_runtime/presentation.rs` | `the-provider-resolver-is-confined-to-one-file` |
| `crates/ambition_characters/src/actor/character_catalog/registry.rs`, `crates/ambition_platformer2d_actor_monolith/src/character_runtime/audit.rs`, or `game/ambition_app/tests/app_local_catalog_composition.rs` | `the-catalog-owners-map-is-not-a-provider-authority` |
| `crates/ambition_characters/src/prepared.rs`, `crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs`, or `crates/ambition_characters/src/actor/character_catalog/mod.rs` | `the-catalog-axis-tuning-is-confined-to-one-file` |
| `crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs` or `crates/ambition_platformer2d_actor_monolith/src/avatar/mod.rs` | `the-movement-tuning-resolver-is-confined-to-one-file` |
| `crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs`, `crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs`, or `crates/ambition_platformer2d_actor_monolith/src/world/rooms/stage.rs` | `only-the-candidate-builder-hides-a-root` |
| `crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs`, `crates/ambition_platformer2d_actor_monolith/src/world/rooms/transaction.rs`, or `crates/ambition_platformer2d_provider/src/lifecycle.rs` | `only-the-publication-authority-publishes-a-candidate` |
| `crates/ambition_platformer2d_provider/src/authoring.rs` or `crates/ambition_game_shell/` | `one-registrar-installs-the-session-teardown` |
| `crates/ambition_platformer2d_world/src/collision.rs`, `crates/ambition_platformer2d_world/src/platforms/mod.rs`, or `game/ambition_demo_mary_o/src/bricks.rs` | `only-the-collision-world-composes-collision` |
