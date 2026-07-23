# Bevy system-parameter architecture — name the seams, do not pack the ceiling

> **State:** TRIAGE — PROPOSED DIRECTION, 2026-07-23.
>
> A targeted, multi-phase refactor is appropriate. Ambition repeatedly reaches
> Bevy's 16-top-level-parameter limit, and several systems already hide extra
> access behind tuples or broad `SystemParam` structs. The remedy is not one
> mechanical wrapping pass. Different signatures expose different architectural
> problems: unnamed entity views, unnamed world services, mixed decision and
> mutation phases, or lifecycle transactions that have not yet converged.
>
> **Not a queue card:** this document records the architectural direction and a
> migration plan. Promote bounded slices into [`../tracks.md`](../tracks.md)
> only when they can name the systems, invariants, performance measurements, and
> deletion target owned by that slice.

## Executive conclusion

The 16-parameter failures are real design feedback, not merely an arbitrary Bevy
annoyance. A source inventory of the current tree found:

- 27 functions with at least 13 non-`self` parameters;
- several production systems exactly at the 16-parameter ceiling;
- explicit tuple packing and comments about "one tuple slot" in hot gameplay,
  camera, menu, debug, and room-flow code;
- 35 derived `SystemParam` types but only 5 derived `QueryData` types;
- at least one derived `SystemParam` (`PlatformerPreparation`) itself at the
  16-field ceiling;
- many `#[allow(clippy::too_many_arguments)]` sites that mix healthy pure kernels,
  Bevy adapters, one-time setup, and genuine orchestration monoliths.

The counts are a snapshot, not a policy oracle. They show enough recurring
pressure to justify a coordinated refactor, but they do **not** imply that every
long signature should become a custom parameter or that every large system
should split.

The enduring direction is:

1. use **derived `QueryData`** to name stable entity views;
2. use **small domain-owned `SystemParam` values** to name cohesive world access;
3. use **ordinary value structs** for pure facts, policy, requests, and outcomes;
4. split systems only at real **phase or mutation-authority boundaries**;
5. use `&mut World` only for genuine **exclusive transactions**, never as a
   per-frame escape hatch;
6. preserve Ambition's unified actor, damage, projectile, movement, and lifecycle
   seams rather than solving the limit by recreating parallel paths.

No general ECS utilities crate is proposed. These types encode domain ownership
and should normally live beside the systems that use them.

## Why parameter packing alone is not enough

A custom `SystemParam` changes the Rust signature, but Bevy still observes the
union of all resource and component access inside it. A system with three broad
parameter bags may be harder to read and less parallel than a system with twelve
honest top-level parameters.

Packing can therefore create four failures:

1. **Hidden access.** A reader cannot tell which resources or components a system
   owns without opening several remote structs.
2. **Scheduler overreach.** A reused parameter bag may include fields one system
   never touches, creating unnecessary conflicts and reducing parallelism.
3. **False cohesion.** Unrelated inputs are grouped only because Bevy needs one
   tuple slot.
4. **Architectural fossilization.** A lifecycle or combat monolith becomes easier
   to compile without becoming easier to reason about, test, or eventually split.

The right question is not "how do we get below sixteen?" It is:

> What stable entity view, world service, pure decision, or transaction does this
> group of parameters represent?

If there is no good answer, the system probably needs decomposition rather than
packing.

## Existing patterns: what to preserve and what to stop

Ambition already contains examples of the intended direction.

### Good cohesive parameters

These name a real, narrow capability:

- `GravityCtx` — gravity field, zones, and base gravity as one frame service;
- `FrameEnv` — frame-relative physical environment;
- `ProjectileCollisionWorld` and `CollisionWorld` — the collision sources a
  consumer intentionally sees;
- `SfxWriter` — message output plus its attribution context;
- `DialogueDispatch` — the dialogue authority needed by interaction;
- `SessionCommands` — commands constrained by active session scope;
- `PortalCameraContinuityParams` — one presentation continuity concern.

These parameters improve both the signature and the conceptual model.

### Useful but capacity-driven parameters that need review

Several existing types explicitly say they exist to stay under the ceiling:

- `SandboxQueues`;
- `CombatRoomReset`;
- `ProgressionResources`;
- `RoomClock`;
- `FeatureDebugQueries`;
- menu and kaleidoscope parameter bundles;
- `ResetPlayState`;
- `PlatformerPreparation`.

Some may remain valid after review. Others combine unrelated authorities or
preserve a temporary orchestration shape. They should not become templates merely
because they compile.

### Patterns to stop adding

Do not introduce new examples of:

```rust
(
    query_a,
    query_b,
    catalog,
    another_query,
): (Query<...>, Query<...>, Res<...>, Query<...>)
```

solely to consume one Bevy parameter slot.

Do not create `FooParams1`, `FooParams2`, `MiscParams`, or `EverythingNeeded`.
Do not reuse a large `SystemParam` merely because two systems share half its
fields. Do not use `ParamSet` as a parameter-budget mechanism; it is for resolving
intentional access conflicts. Do not convert hot gameplay systems to exclusive
`&mut World` systems merely to remove the tuple limit.

## Four distinct architectural cases

### Case A — one entity concept has a large query tuple

Use derived `QueryData` when a tuple describes one stable entity role.

Examples already pointing this way include:

- `ActorClusterQueryData`;
- boss cluster query data;
- `BodyClusterQueryData`;
- `ActorSpriteData`.

Good candidates include projectile rows, projectile victims, damage victims,
portal rigs, followed camera bodies, encounter participants, and debug trace
views.

A named query view provides:

- field names instead of positional tuple destructuring;
- one place for required versus optional component policy;
- reusable read-only and mutable forms where Bevy supports them;
- better compiler errors;
- easier fixture construction and assertions;
- a natural home for query-item helper methods that do not mutate unrelated
  world state.

A `QueryData` type should describe an entity role, not one system's entire world.
Avoid combining mutually exclusive body families into one giant optional query.
Required-component semantics must remain explicit: making a field required can
silently remove entities from the query.

### Case B — one system uses several resources or queries from one authority

Use a local, cohesive `SystemParam`.

Examples of legitimate authorities include:

- projectile stepping environment;
- projectile output channels;
- camera observation configuration;
- camera observation output state;
- encounter command outputs;
- immutable preparation catalogs;
- preparation publication state;
- portal capture assets;
- portal-rig query access.

Each custom parameter must have a one-sentence ownership statement. Its fields
should normally be used together, have the same mutability character, and change
for the same reason.

Prefer separate read and write capabilities when that preserves scheduler
parallelism. For example, `ProjectileStepEnvironment` and `ProjectileStepOutputs`
are clearer than one `ProjectileStepContext` containing every query, resource,
writer, and command buffer.

A parameter type should remain in the owning module or crate. Moving it into a
shared support crate would hide dependency and access decisions that are useful
at the call site.

### Case C — the ECS adapter contains a large domain decision

Extract ordinary Rust facts and outcomes.

The Bevy system should:

1. gather components/resources into a value-level input;
2. call a pure or narrowly stateful domain function;
3. apply the returned mutations/messages through explicit outputs.

Examples:

```rust
struct ProjectileStepFacts { /* values, stable ids, geometry */ }
struct ProjectileStepOutcome { /* pose, despawn, damage, fx */ }

fn resolve_projectile_step(
    facts: ProjectileStepFacts,
    environment: &ProjectileEnvironment,
) -> ProjectileStepOutcome;
```

```rust
struct DamageFacts { /* attacker, victim, strike, frame */ }
struct DamageOutcome { /* accepted, hp delta, knockback, effects */ }
```

This does more than shorten signatures. It makes the difficult logic testable
without constructing a Bevy app, reduces fixture bloat, and lets multiple ECS
adapters share one decision without creating parallel gameplay paths.

Do not create a giant value struct that merely mirrors every component by
reference. Facts should be the values needed by the decision, not another ECS
access wrapper.

### Case D — one function coordinates a lifecycle transaction

Do not solve lifecycle pressure by adding increasingly broad parameter bags.

Reset, activation, transition, hot reload, and reconstruction are already moving
toward prepared transaction artifacts and one outer commit boundary. Systems such
as reset orchestration, room transition commit, content preparation, and world
reload should converge on that architecture.

Appropriate forms include:

- a prepared immutable request/plan;
- explicit transaction policy;
- a small outer system that obtains the commit authority;
- an exclusive-world commit only where the transaction genuinely requires one;
- publication after complete verification.

The transaction itself should own the long list of world changes. A
`RoomTransitionParams` containing fifteen independent mutable resources is not a
transaction; it is a hidden long signature.

## Architectural inventory and recommended treatment

The following inventory is directional. Re-check signatures before promoting a
slice because active campaigns are changing this code rapidly.

| Area | Current pressure | Architectural reading | Recommended treatment |
|---|---|---|---|
| `step_projectiles` | At the Bevy ceiling, explicit tuple-slot workaround, long deterministic loop | One unified projectile authority with unnamed row/target views and mixed resolve/apply logic | High-priority hot-path pilot: `QueryData`, narrow environment/output params, pure per-projectile outcome; preserve one globally ordered projectile loop |
| `apply_hitbox_damage` | Very large victim/attacker/output surface | Shared melee resolver is correct, but target views and damage decision are too implicit | Named attacker/victim views plus shared `DamageFacts`/`DamageOutcome`; do not split player and actor damage paths |
| `apply_player_hit_events` | At ceiling; player policy, room policy, inventory, effects, and health mutation meet | Victim adapter contains both generic damage application and home-player consequences | Separate generic victim damage result from home-player policy effects, chained in the same tick; retain one physical damage authority |
| `apply_feature_hit_events` | Large catalogs, writers, target queries | One event reducer serving multiple target families | Keep one reducer, name target/catalog/output capabilities, move per-target calculations into ordinary functions |
| `integrate_sim_bodies` | Near ceiling, large body cluster query | Unification is intentional and load-bearing | Keep one scheduled integration system; use named body query data and environment/output capabilities; do not split player versus actor |
| `integrate_home_body` | Long ordinary function rather than a Bevy-limit problem | One movement-kernel adapter with many scalar inputs | Consider `HomeBodyInput`, `HomeBodyEnvironment`, and output value structs only if they clarify policy; do not split the unified kernel call |
| `drive_wave_encounters` | Large and roughly 300 lines | Trigger detection, lifecycle commands, wave cadence, rewards, and spawn preparation coexist | Separate observation/decision from command application while preserving one encounter lifecycle reducer |
| `update_boss_encounters` | Large phase machine with many authorities | Similar to encounter orchestration; likely multiple internal phases | Extract pure phase transition and reward decisions; use named query/output capabilities before considering scheduled splits |
| `resolve_camera_observation` | At ceiling, but already calls a pure camera resolver | Mostly ECS gathering and publication around a coherent pure operation | Best low-risk pilot: followed-body `QueryData`, read-only observation inputs, mutable output state; keep one resolve system |
| `sync_portal_view_cones` | At ceiling and roughly 360 lines | Resource-heavy presentation lifecycle: retire, update, allocate, publish | Local asset/query parameters plus explicit retire/update/spawn phases if ordering remains same-frame; benchmark render/main-world cost |
| `apply_portal_camera_continuity` | Very long presentation coordinator | Multiple continuity cases and outputs in one adapter | Extract value-level continuity decisions first; split only if one authoritative state writer remains obvious |
| `PlatformerPreparation` | Derived `SystemParam` at its own 16-field ceiling | Immutable catalog capture and mutable transaction publication are two concerns | Separate preparation inputs/catalog snapshot from outputs/publication; eventually consume a frozen prepared-content authority |
| reset and room-flow systems | Repeated ceiling pressure and broad mutable access | Symptom of the still-active room transaction migration | Do not invent more packing; complete the transaction architecture and delete manual sequencing |
| `kaleidoscope_focus_nav` | At ceiling, action dispatch mixed with navigation and settings mutations | UI controller plus a broad action executor | Resolve navigation into a value-level action, then dispatch through one menu-action authority; local UI params are acceptable |
| trace/debug systems | At ceiling but development-only | Broad observation, little gameplay authority | Low-risk `QueryData` and debug observation bundles; lower priority unless compile or maintenance cost is material |
| demo/startup setup systems | Long one-shot signatures | Composition root wiring, not frame logic | Plain setup input structs are appropriate; optimize readability, not scheduler parallelism |

## Priority order

### Priority 1 — remove dishonest ceiling workarounds

Start with code that explicitly packs unrelated values into tuples to get one
slot. These sites are fragile because the next dependency can trigger another
compiler failure without improving the architecture.

For every site, classify the packed fields as:

- one entity view (`QueryData`);
- one world capability (`SystemParam`);
- pure facts/outcomes;
- a phase that belongs in another system;
- transaction state owned by an existing campaign.

Do not merely replace the tuple with a named struct unless the classification
identifies a coherent capability.

### Priority 2 — low-risk representative pilots

Use two pilots before touching all combat hot paths.

#### Pilot A: camera observation

`resolve_camera_observation` already gathers data and calls
`resolve_follow_camera_snapshot`. Refactor only the ECS adapter:

- `FollowedBodyView` as derived query data where it represents one role;
- `CameraObservationInputs` for read-only camera/configuration resources;
- `CameraObservationState` for mutable ease/output/local state;
- retain one scheduled system and the existing pure resolver;
- prove the scheduler access set did not broaden.

This pilot tests whether named parameters improve the code without changing
simulation behavior.

#### Pilot B: preparation boundary

Refactor `PlatformerPreparation` into coherent nested capabilities before another
field exceeds its own ceiling:

- immutable catalog/schema inputs;
- mutable epoch/session/publication outputs;
- load-command output;
- no change to canonical preparation or fingerprints.

This pilot tests composition of custom parameters at a lifecycle boundary. It is
not permission to create one mega-context.

### Priority 3 — projectile and damage hot paths

After the pilots establish conventions, tackle `step_projectiles` first because
it combines all three useful techniques and currently documents a tuple-slot
hack.

The target shape is:

- `LiveProjectileRow` query data;
- named body, feature, and boss target views;
- `ProjectileStepEnvironment` containing time, collision, gravity, immutable
  catalogs, and relation policy only where these are always used together;
- `ProjectileStepOutputs` containing commands and effect writers;
- a pure per-projectile resolve result;
- one stable ordering by `ProjectileSeq` across all factions;
- no restored player-projectile/enemy-projectile split;
- no query-iteration-order dependence.

Then apply the learned pattern to hitbox and hit-event consumers. The end state
should make physical hit resolution shared while keeping home-player death/reset,
boss progression, breakable behavior, and presentation policy as explicit
consumers of one result.

### Priority 4 — transaction-owned systems

Coordinate with the transactional-construction and room-lifecycle campaign.
Do not refactor reset/transition signatures in isolation while their authority is
moving.

The expected deletion is manual parameter threading and sequencing, not merely a
new `SystemParam` name. Once one prepared room transaction owns geometry,
construction, resource reseeding, player policy, verification, and publication,
its Bevy entry point should need only the transaction inputs and commit authority.

### Priority 5 — presentation, UI, and debug cleanup

Refactor portal views, menus, and tracing after the gameplay and lifecycle
conventions are proven. These are valuable for readability, but most are less
likely to cause simulation bugs.

Presentation systems may split into multiple same-frame systems when each phase
has a clear resource/component authority. Measure the result: additional systems
and deferred-command boundaries are not free.

## Naming conventions

Names should describe responsibility rather than parameter mechanics.

Recommended patterns:

- `FooView` / `FooRow` — derived `QueryData` for one entity role;
- `FooInputs` — cohesive read-only world access;
- `FooState` — mutable state owned by the operation;
- `FooOutputs` — messages, commands, and effects emitted by the operation;
- `FooCatalogs` — immutable registries consulted together;
- `FooFacts` — value-level decision inputs;
- `FooOutcome` — value-level decision result;
- `FooRequest` / `FooPlan` / `FooPolicy` — lifecycle or transaction data.

Avoid `FooParams` when a more precise ownership name exists. Avoid numeric or
"misc" group names.

## System splitting rules

Splitting is appropriate when all of these are true:

1. the phases have different data-access or mutation authorities;
2. the intermediate value is a real domain fact or message, not an artificial
   transport object;
3. same-frame ordering can be expressed explicitly with sets or `.chain()`;
4. the split does not duplicate a scan that must remain globally ordered;
5. the split improves independent testing or scheduler parallelism;
6. no one-body/one-path invariant is forked.

Keep one system when:

- it owns one deterministic ordered pass, such as all live projectiles;
- splitting would require rescanning the same entities and rebuilding the same
  transient index;
- one mutable authority must make the complete decision coherently;
- the apparent complexity is already delegated to pure functions;
- the system is a thin ECS adapter despite a moderately long signature.

A split must never add a frame of latency accidentally. Systems that replace one
operation should remain chained or ordered in the same schedule unless delayed
behavior is an explicit design change.

## Scheduler and performance requirements

Parameter cleanup can alter performance even when gameplay is unchanged.

For each pilot record:

- system access before and after;
- whether a custom `SystemParam` added fields not used by that system;
- ambiguity/conflict changes;
- hot-path wall time in the relevant benchmark or profile;
- headless gate timing;
- compile and link timing for the touched crate;
- number of scheduled systems and new deferred-command boundaries;
- allocation changes in per-frame paths.

Do not assume fewer top-level parameters means more parallelism. Bevy schedules
from actual access, including fields hidden in nested parameters.

Do not build temporary `Vec`s merely to make a system split unless the previous
system already needed the same stable ordering or the measured cost is acceptable.

## Test strategy

This refactor should reduce test difficulty, not trigger another explosion of
large Bevy harnesses.

Use three layers:

1. **Pure decision tests** for `Facts -> Outcome` functions;
2. **focused ECS adapter tests** proving query gathering and effect application;
3. **existing production-schedule tests** for ordering-sensitive end-to-end
   invariants.

Do not duplicate every gameplay matrix at all three layers. Move calculation
coverage downward and retain a small number of adapter and schedule canaries.

The proposed `ambition_test_support` work complements this plan: named app
profiles, schedule stepping, and roster assertions make adapter tests cheaper.
This plan should not wait for that crate, and the parameter types themselves do
not belong in test support.

Poison tests remain appropriate where a refactor could silently drop an entity
from a required-component query, broaden/narrow a victim set, change ordering, or
fork a unified path. Source-text tests that merely count parameters or demand a
specific type name are not appropriate.

## No hard parameter-count lint yet

A review budget is useful, but a global scanner would mistake ordinary pure
functions, setup roots, and generated Bevy adapters for the same problem.

Use this guideline during migration:

- **0–8 top-level parameters:** normally unremarkable;
- **9–12:** check whether stable entity views or capabilities are unnamed;
- **13–15:** require an architectural explanation in review;
- **16 or tuple packing:** must be classified and assigned a deliberate remedy;
- custom `SystemParam` fields count conceptually even when Bevy sees one slot.

Do not add a workspace source-text policy test solely for these numbers. After
the migration, reconsider a lightweight report if recurring ceiling regressions
remain materially harmful. Any future enforcement should permit an explicit,
reasoned exception for a genuinely cohesive adapter or pure kernel.

## Migration phases

### Phase P0 — inventory and ownership classification

Refresh the 13+ parameter inventory and classify every site as:

- query-shape debt;
- capability naming debt;
- pure-decision extraction;
- phase decomposition;
- transaction-owned;
- acceptable one-time setup;
- acceptable pure kernel.

Record current schedule sets and access relationships for production systems.
This inventory is migration scaffolding and should not become permanent duplicated
architecture documentation.

### Phase P1 — conventions and two pilots

Complete camera observation and preparation-boundary pilots. Add module-local
rustdoc explaining each new capability and prove no access broadening.

Evaluate whether the naming rules remain understandable to an unfamiliar coding
agent without navigating several crates.

### Phase P2 — projectile/damage slice

Refactor the unified projectile step, then shared hitbox and hit-event paths.
Preserve deterministic ordering and one physical damage path. Delete tuple-slot
workarounds and obsolete helper adapters as each slice lands.

### Phase P3 — body/encounter slice

Apply named query views and pure decisions to unified body integration, wave
encounters, and boss progression. Do not split the unified body integration
system by controller kind.

### Phase P4 — room lifecycle handoff

Let the room transaction campaign remove reset/transition parameter threading.
Do not create parallel parameter architecture while that work is active.

### Phase P5 — presentation/UI/debug

Refactor portal view-cone lifecycle, camera continuity, kaleidoscope navigation,
and trace observation based on the proven patterns.

### Phase P6 — evaluate policy and cleanup

Remove stale `too_many_arguments` allowances and comments that describe deleted
tuple hacks. Re-run the inventory, compile timings, test timings, and runtime
profiles. Decide whether any reporting tool is still needed.

## Acceptance criteria

The initiative is complete when:

- no production system uses anonymous tuple packing solely to evade Bevy's
  top-level parameter limit;
- systems at or near the ceiling have an explicit architectural classification;
- stable entity roles use named query data where that improves clarity;
- custom system parameters correspond to cohesive world capabilities and do not
  silently broaden access for reuse;
- projectile, hitbox, movement, and actor paths remain unified;
- lifecycle/reset systems consume the common transaction rather than hiding
  manual sequencing in parameter bags;
- pure gameplay calculations can be tested without a Bevy app where practical;
- no migrated system gains an accidental frame boundary;
- deterministic ordering and rollback-safe authorities are preserved;
- runtime and test-gate performance are measured rather than assumed;
- current-mechanism docs explain the new seams without treating parameter count
  itself as the architecture.

## Promotion recommendation

Promote this work as several bounded cards rather than one workspace-wide
refactor:

1. camera observation + preparation-boundary pilots;
2. unified projectile step;
3. shared damage consumers;
4. unified body and encounter adapters;
5. room-transaction parameter deletion;
6. presentation/UI/debug cleanup.

The first card should establish conventions and measurement. The projectile card
is the first high-value gameplay proof. The room-lifecycle card must remain
coordinated with the existing transactional-construction campaign.

The goal is not to make every signature small. The goal is to make every large
operation expose the few stable concepts it actually owns.
