# Actor-monolith semantic edge ledger

**Decision baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`.
This replaces the post-P4 placeholder ledger. It records decisions at the current
nine-module SCC, rather than assuming that a future six-module SCC is the right
unit of design. Use the [responsibility map](architecture-responsibility-map.md)
and [work packets](actor-monolith-work-frontier.md) for the target and execution
contracts. No packet is authorized solely by this table.

## Evidence and limits

The current module-path instrument reports the cycle through abilities,
construction, control, features, items, projectile, session, shrine and world,
plus the separate assets/character_sprites cycle. Reproduce it with:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 200
```

The instrument counts textual qualified paths, excludes named test files and
certain test modules, and does not resolve Rust imports, macros, generated code,
feature activation, re-exports or cross-crate authority. Its brace-based inline
test filter is not a Rust parser. A deleted edge is not proof that the underlying
state, scheduling or semantic dependency disappeared.

The rows below group **responsibility families**, not every individual reference.
They are a reviewed map of the current cycle, not a certified exhaustive
production call graph. Before a packet, enumerate its actual callers, tests,
installers, state writers, rollback declarations and lifetime consumers. Preserve
line numbers only as locators; symbols and assertions are the durable evidence.

## Disposition vocabulary

**KEEP_DIRECT:** the consumer legitimately understands this lower domain.
**MOVE_AUTHORITY:** move behavior, state and installation together.
**MOVE_ADAPTER:** retain the concrete integration but move it to the owner of that
integration. **GROUP_PACKAGE:** an internal cycle may be part of one authority.
**HOLD:** the row names a specific proof prerequisite; no generic abstraction is
approved. **REMOVE_AFTER_REPLACEMENT:** remove an obsolete projection/dispatch only
after its semantic replacement covers its customers.

HOLD is deliberate, not an invitation to substitute a callback. No unresolved
writer or lifetime obligation may be omitted from a packet's acceptance.

## Current responsibility families

Paths in the locator column are relative to
`crates/ambition_platformer2d_actor_monolith/src/`; they identify existing source.
A target authority is logical unless a work packet gives an exact physical move.

| Reference family and locator | Kind / authority | Disposition and intended direction | Required proof before moving |
| --- | --- | --- | --- |
| `abilities/traversal/possession.rs`: `body_driving_seat`, `controlled_frame_down` | Accepted control and current input facts; live tick / rollback | KEEP_DIRECT or GROUP_PACKAGE with accepted control. Possession reads the same arbitration result as other body execution | A4: all seats, mounts, possession, release and competing claims consume the same accepted relation |
| `control/authority.rs`: `PossessionState` | Accepted relation writer depends on a particular control mode | GROUP_PACKAGE is allowed; HOLD a separate capability protocol | Distinguish proposed claims from committed ownership and enumerate all writers before extracting the mode |
| `control/input_systems.rs`: ascend/descend possession helpers | Controlled traversal policy | HOLD under A4, preferably co-locate the policy rather than inject arbitrary closures | No loss of mount/possession precedence, body-local clocks or mode-specific motion rules |
| `control/input_systems.rs`: `advance_body_anim_overlays` from features | Body animation/gameplay projection | MOVE_AUTHORITY out of the feature bucket when its gameplay versus presentation writers are established | Preserve attack/clock-driven animation facts and avoid a second simulation writer hidden in presentation |
| `features/ecs/chests.rs`, `features/ecs/interact.rs`: `ActingParticipant` | Interactive objects read an accepted actor-control fact | KEEP_DIRECT toward control facts | Two bodies remain independently actionable; object interaction must not arbitrate control itself |
| `abilities/thrown/puppy_slug_gun.rs`: `spawn_runtime_minion` | Item/ability-specific summon policy plus live construction | MOVE_ADAPTER toward existing construction integration, not spawn crate live mutation | Preserve ownership/ally attribution, failure consumption and the one structural body builder; no universal spawn message required |
| `items/pickup/mod.rs`: `fire_puppy_slug_gun_system` installation | Held-item ability composition | MOVE_ADAPTER to explicit item/ability composition once the ability owns its installer | Item-absent and ability-absent profiles remain meaningful; current scheduling is equivalent |
| `features/ecs/spawn/mod.rs`: ground-item, staged-actor, authored-static and authored-actor request construction | Prepared room construction combines distinct domain recipes | MOVE_ADAPTER toward construction assembly; do not place all recipes in a generic feature authority | Domain-typed parameters, authored occurrence identity and preparation failures remain explicit |
| `features/ecs/spawn/mod.rs`: `ActorConstructionPlan`, services, registry, preflight, verify | Construction transaction coordinator | MOVE_AUTHORITY with staging/receipt responsibilities when A3 prerequisites hold | F6: verification is publication admission, not arbitrary command rollback; preserve validation before mutation |
| `features/ecs/summon.rs`: summoned-minion request, services and preflight | Runtime construction adapter | KEEP_DIRECT toward typed construction; MOVE_ADAPTER out of feature bucket with actual callers | Failed construction does not consume an item or publish a half-built live actor outside current failure policy |
| `features/ecs/spawn/mod.rs`: `MintedItemBaseline` | Item occurrence baseline participates in room construction | HOLD under A7; construction reads/returns item-owned baseline facts | Distinguish authored room baseline from session minted history and same-room checkpoint restoration |
| `features/mod.rs`: item grants and shop-transaction installation | Economy/item policy installed in interactive-world aggregation | MOVE_ADAPTER to explicit interaction/item integration, not a global effect router | Debit/grant order, one interaction per acting body and item entitlement/occurrence semantics remain defined |
| `features/ecs/perception.rs`: `ProjectileAllegiance` | Perception reads projectile threat facts | KEEP_DIRECT toward projectile facts | No projectile behavior or collision policy moves into perception merely to avoid the import |
| `features/ecs/spawn/mod.rs`, `features/ecs/summon.rs`: `ActorPlacementContext` | Actor/catalog preparation bridge | MOVE_ADAPTER under A3 toward construction integration | World provider can remain actor-agnostic; catalog-backed builders retain required preparation context |
| `features/ecs/spawn_static.rs`: world lowering context | Authored world-object translation | KEEP_DIRECT to provider-neutral placement input; split only actor-bearing context in A3 | Every current static placement variant still validates and lowers; unsupported authored fields are diagnosed |
| `features/ecs/mod.rs`: world overlay re-export | Compatibility surface for spatial contribution facts | REMOVE_AFTER_REPLACEMENT when callers directly name spatial owner | Do not move overlay identity into unrelated common vocabulary to erase the re-export |
| `features/mod.rs`: construction-verification re-export and `FeatureWorldOverlaySet` | Publication evidence and scheduling ancestry | MOVE_ADAPTER toward construction/spatial owner; no private-system coupling | Accepted overlay updates become visible at the same phase and include required deferred barriers |
| `construction/mod.rs`: `HealShrine` construction | Concrete rest-point content recipe | KEEP_DIRECT toward the content component initially; no generic engine shrine abstraction | A1 removes lifecycle work from shrine without changing heal/capture policy or construction identity |
| `construction/mod.rs`: `ActorPlacementContext` and lowering function/context | Actor-specific authored-world bridge is in the wrong region | MOVE_ADAPTER under A3 | Remove world knowledge of actor catalogs without replacing typed lowering with erased callbacks |
| `items/persist.rs`, `items/minted_horizon.rs`: `SaveRestored` | Items participate in session-owned persistence phase | KEEP_DIRECT toward explicit lifecycle milestone, or a concrete integration adapter | A7: item baseline, inventory, minted occurrence and restored save each have one writer/lifetime |
| `items/pickup/mod.rs`: checkpoint progress initialization and startup restore | Session restoration incorrectly installed with pickup | MOVE_AUTHORITY under A1 to session checkpoint installer | Restoring a checkpoint cannot require installing item pickup; startup is not accidentally gameplay-gated |
| `items/pickup/mod.rs`: `heal_save_shrine_system` | Rest-point interaction and item/capture phase integration | KEEP_DIRECT in the full composition for A1; later content-owned installer can replace this narrow residual | Healing every interacting body and one deterministic capture remain unchanged; do not expand A1 into all item restructuring |
| `projectile/systems.rs`: feature breakable/boss hit predicates | Contact admission tied to historical target families | REMOVE_AFTER_REPLACEMENT under A2; projectile reads published target/contact facts | Shared authored geometry, target/world ordering, present-empty semantics and consumption/reaction rules must precede deleting fallbacks |
| `session/reset/mod.rs`, `session/teardown.rs`: ally and possession state | Capability retraction at session/reset boundary | MOVE_ADAPTER toward capability-owned teardown hooks only when the existing phase contract suffices | Do not replace known state reset with a broad service locator; restoration, attribution and rollback enrollment stay correct |
| `session/reset/mod.rs`: construction registry | Session coordinates building a replacement room | KEEP_DIRECT to typed prepared construction | Session may know that construction exists; it should not know every entity recipe or conceal raw-Commands failure limits |
| `session/{setup,reset/mod}.rs`: room staging registry/context from features | Session depends on a catch-all container for its construction coordinator | MOVE_AUTHORITY/ADAPTER with construction staging, not into a second runtime monolith | Preparation, admission, old-room teardown, verification and publication retain explicit ownership |
| `session/durable_horizon.rs`: restore/persist inventory and minted items | Lifecycle coordinates several durable authorities | KEEP_DIRECT composition until item owner exposes bounded installation | Never move inventory semantics into lifecycle just because it orders save operations |
| `session/teardown.rs`: `MintedItemBaseline` clear/reset | Item-owned baseline lifetime | HOLD under A7; same owner must define restore and reset | Save restore, same-room replay, room exit and new-session behavior require a written matrix |
| `session/{setup,reset/mod}.rs`: physics/world setup | Session assembly of spatial capability | KEEP_DIRECT toward world/spatial services | Ordinary composition is allowed; deleting this dependency has no inherent architectural benefit |
| `session/{setup,reset/mod}.rs`: `ActiveContentBinding` | Current room/provider revision binding | HOLD physical move; session selects the active binding, prepared-world provider interprets it | A8 only after two-instance witness: no cross-room or mixed-revision lookup; do not blindly add world IDs everywhere |
| `shrine.rs`: pending lifecycle slot and room transition intents | Session startup/reset restoration living in rest-point source | MOVE_AUTHORITY under A1; slot stays session-owned | Retry only latches after admitted routing; exact phase, primary-avatar policy and snapshot wire IDs remain covered |
| `world/rooms/reconstitution.rs`: `SpawnedThisAttempt` | Construction attempt ownership / verification | MOVE_AUTHORITY with construction attempt lifecycle, not generic world state | Failed-attempt cleanup is scoped; a spawn marker does not promise rollback of arbitrary side effects |
| `world/rooms/{stage,transaction}.rs`: feature construction plan and receipt | World/lifecycle wrapper delegates to construction coordinator | MOVE_ADAPTER with A3 follow-up, after plan and receipt owner is explicit | No cyclic public API remains merely renamed; prepare/commit/verify/publication ordering stays observable |
| `world/rooms/systems.rs`: pending lifecycle slot and room transition | Room traversal requests session admission | KEEP_DIRECT in explicit room/session integration | Generic geometry must not import session; the room-transition adapter legitimately does |

## Beyond the measured SCC

The SCC excludes dependencies through other crates. Always inspect
`ambition_combat`, `ambition_characters`, `ambition_platformer2d_core`,
`ambition_platformer2d_shared_tangle` and runtime/host while following a row.
A component moved into shared_tangle may still have every old semantic writer.
A facade that re-exports a builder can conceal mandatory render reachability.
Neither improvement is visible in the nine-module count.

The assets/character_sprites cycle is preparation integration until its concrete
prepared-data and device-resource responsibilities are separated. Do not invent
an asset registry just to break a two-module cycle. Preparation, residency and
mechanical versus visual revision ownership supply the actual test.

## Packet release rule

A packet records the state owner, mutation authority, scope/lifetime, accepted
inputs, public outputs, schedule/visibility edges, optional dependencies, rollback
registration and behavioral fixture. Its source move can proceed only when these
are closed for **that packet**, not when every future architectural question in
the engine has been answered.

A1 is the best next ownership move. A2 and A3 have independent staged prerequisites.
A4-A8/A10 have explicit holds in the frontier. A11/A12 address admission defects
outside this SCC and should not wait for a graph milestone. Existing cycles within
one coherent authority can remain. Any claim that a cut reduced coupling must
explain which unrelated knowledge a caller no longer needs.
