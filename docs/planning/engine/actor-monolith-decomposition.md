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

The largest cyclic component has eight modules (`abilities`, `avatar`,
`character_runtime`, `features`, `items`, `projectile`, `session`, `world`).
A second component holds `assets` and `character_sprites`. Re-run the command
before you quote these numbers.

The instrument counts textual qualified paths inside one crate. It does not see
cross-crate ownership, shared writers, message timing, plugin prerequisites,
macros or re-exports. The same SCC result accompanied both a wrong and a
corrected spawn boundary. Diff the members, not the count. A `--cuts` line that
shrinks the component is a fact, not a mandate: cut an edge only when a
responsibility argument also supports it.

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
