# Actor-monolith decomposition — executable SCC frontier

**Baseline:** `625fa79af45e6eff40cbefabd8cdae33c5b5e9db`.

This is an implementation queue, not an architecture essay. Execute P1-P4 in
order. Do not start P5 implementation until P1-P4 have landed and the graph has
been remeasured.

Durable rules:
[`actor-monolith-decomposition.md`](actor-monolith-decomposition.md).
Post-P4 ledger:
[`actor-monolith-hard-core-edge-ledger.md`](actor-monolith-hard-core-edge-ledger.md).

## Before every packet

From the new HEAD:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
rg -n 'crate::(abilities|actor_spawn|character_runtime|construction|control|features|items|projectile|session|shrine|world)' \
  crates/ambition_platformer2d_actor_monolith/src -g '*.rs'
git status --short
```

Record the largest SCC and the exact edge being cut. If the expected edge has
already disappeared or a new edge changes the proposed ownership, stop and
update this frontier before editing source.

Use one commit per packet.

---

## P1 — move stocks-match settlement state to `ambition_match`

**Cut:** `character_runtime -> features`.

**Baseline production edge (1 ref):**

```text
crates/ambition_platformer2d_actor_monolith/src/character_runtime/live_match_clock.rs
    uses crate::features::stocks_match::StocksMatchSettled
```

**Expected graph receipt:** largest SCC **11 -> 9**. `actor_spawn` and
`character_runtime` should become a separate 2-module SCC.

### Ownership decision

`StocksMatchSettled` and `SuddenDeathEntered` are match receipts/latches. They
carry `MatchInstance` and `MatchVerdict`, not feature ECS behavior. Their owner is
`ambition_match`.

Move exactly these values and their pure query helper:

```text
StocksMatchSettled
SuddenDeathEntered
the_live_match_is_settled
```

Do **not** move `decide_stocks_match`, `SuddenDeathBegan`, winner-card systems or
other Smash/stocks policy in this packet.

### File operations

1. Create `crates/ambition_match/src/stocks_state.rs` containing the three items <!-- cite-ok: proposed path created by P1 -->
   above and their value-level tests.
2. Export them from `crates/ambition_match/src/lib.rs`.
3. Move the `SnapshotState` implementations for `StocksMatchSettled` and
   `SuddenDeathEntered` from
   `crates/ambition_platformer2d_actor_monolith/src/snapshot_impls.rs` into
   `crates/ambition_match/src/snapshot_impls.rs`. The orphan rule requires this
   once the types move.
4. Change monolith rollback registration to register the new type paths while
   preserving these exact wire IDs:

   ```text
   resource.stocks_match_settled
   resource.sudden_death_entered
   ```

5. Update `features/stocks_match.rs` to import the values from `ambition_match`.
6. Update `character_runtime/live_match_clock.rs` to import
   `ambition_match::StocksMatchSettled` directly.
7. Update direct external consumers, including Smash/app tests, to the
   `ambition_match` path. Do not retain `features::stocks_match` as the discovery
   path for the moved values.
8. Delete the moved definitions from `features/stocks_match.rs`.

### Forbidden end states

P1 is **not complete** if any production source still contains:

```text
character_runtime -> features::stocks_match
features::stocks_match::StocksMatchSettled
features::stocks_match::SuddenDeathEntered
```

Do not introduce `ambition_match -> actor_monolith`.

### Required acceptance

Run/retain the existing tests that prove:

- a settled match stops the live match clock;
- sudden death suppresses normal settlement/timeout behavior;
- the sudden-death latch does not carry into the next match;
- rollback restores the match receipt/latch with the same wire IDs;
- the assembled Smash host still reaches settlement/sudden death correctly.

Source receipt:

```bash
! rg -n 'crate::features::stocks_match' \
  crates/ambition_platformer2d_actor_monolith/src/character_runtime -g '*.rs'
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
```

Stop if `ambition_match` would need any dependency on the actor monolith to own
these values. That would invalidate the ownership choice.

---

## P2 — make projectile simulation consume generic feature-target capability

**Cut:** `projectile -> features`.

**Baseline production calls (2 refs total):**

```text
projectile/systems.rs
    -> features::ecs_hit_event_hits_breakable
    -> features::ecs_hit_event_hits_boss
```

Those calls decide **same-tick projectile termination** before the feature damage
consumer later drains `HitEvent`. Replacing them with a message round-trip is not
acceptable; it changes timing.

**Expected graph receipt after P1:** largest SCC **9 -> 8**.

### Ownership decision

Projectile flight may ask one lower-level question:

> Is there a live projectile-reactive feature target intersecting this shot, and
> what stable ignore key names it?

Projectile flight must not know boss catalogs, boss animation state, breakable
trigger policy, or `pogo_refresh` policy.

Use the already-canonical `DamageableVolumes` for geometry. Add only the missing
**projectile eligibility/identity** vocabulary to `ambition_projectiles`.

### Concrete API

Add a small component in `ambition_projectiles`, named for example:

```text
ProjectileFeatureTarget
    ignore_key: String
```

The exact public name may vary, but it must contain only projectile-facing
identity/eligibility. It must not import `BossConfig`, `BreakableFeature`,
`BossCatalog` or actor-monolith feature types.

The component is present only on feature entities that should terminate an
ordinary projectile hit. Geometry remains in `ambition_combat::DamageableVolumes`.

### File operations

1. Add the target component/API to `crates/ambition_projectiles` and export it.
2. Register it with rollback if the feature entity's static capability marker is
   part of rollback reconstruction. Do not rely on an unregistered component
   surviving entity restoration.
3. Stamp the component at the two existing construction sites, not from a
   later repair system:
   - `actor_spawn::spawn_boss_with_overrides_into` inserts
     `ProjectileFeatureTarget::new(format!("boss:{}", authored.id))` beside the
     boss's initial `DamageableVolumes`;
   - `features/ecs/spawn_static.rs::spawn_breakable_into` inserts
     `ProjectileFeatureTarget::new(format!("breakable:{}", authored.id))` only
     when `breakable.trigger.allows_hit() && !breakable.pogo_refresh`.
   This preserves the exact ignore-key grammar already consumed by
   `target_is_ignored`. Do not add a deferred per-frame marker repair that makes
   a fresh target intangible to projectiles for its first tick.
4. Keep `features/ecs/target_volumes.rs` as the owner of current geometry:
   - `refresh_boss_damageable_volumes` continues publishing active boss-part /
     authored hurtbox geometry;
   - `refresh_breakable_damageable_volumes` continues clearing geometry when
     broken and publishing current breakable geometry.
5. Rewrite `projectile::step_projectiles` to query only the generic target marker
   plus `DamageableVolumes` for unresolved feature termination. Remove its
   boss-specific and breakable-specific query parameters and remove the
   `BossCatalog` dependency from this decision path.
6. Preserve existing `HitEvent { target: UnresolvedFeatures, ... }`, splash,
   trace and projectile despawn behavior. Feature damage application remains in
   the feature owner.
7. Delete `ecs_hit_event_hits_breakable` / `ecs_hit_event_hits_boss` if no other
   production consumer remains. Do not move those feature-specific helpers into
   `projectile` merely to make the graph green.

### Forbidden end states

P2 is not complete if `projectile/systems.rs` still mentions any of:

```text
crate::features
BossCatalog
BreakableFeature
BossClusterRef
BossAttackState
```

for unresolved feature collision/termination.

It is also not complete if a pure pogo-refresh breakable now consumes ordinary
projectiles, or if boss collision falls back to the coarse boss envelope instead
of current `DamageableVolumes`.

### Required acceptance

Add/retain production poisons for all of these:

1. projectile intersects an ordinary on-hit breakable -> emits unresolved
   feature hit and terminates that tick;
2. projectile intersects a pure `pogo_refresh` breakable -> does **not** consume
   the projectile;
3. projectile intersects an active boss part -> consumes at the precise
   `DamageableVolumes`, not the coarse body envelope;
4. dead/broken target with empty damageable geometry -> does not consume;
5. ignored target key -> does not consume;
6. absorbed/parried projectile still cannot fall through into unresolved feature
   resolution (`an_absorbing_parry_consumes_the_shot_rather_than_returning_it`).

Source receipt:

```bash
! rg -n 'crate::features::ecs_hit_event_hits_(breakable|boss)' \
  crates/ambition_platformer2d_actor_monolith/src/projectile -g '*.rs'
! rg -n 'BossCatalog|BreakableFeature|BossClusterRef|BossAttackState' \
  crates/ambition_platformer2d_actor_monolith/src/projectile/systems.rs
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
```

Stop if the proposed generic target component needs feature-family policy to be
interpreted by the projectile system. That policy belongs at construction or the
feature publisher, not in flight simulation.

---

## P3 — move deterministic lifecycle-commit vocabulary to `shared_tangle`

**Cut:** `shrine -> session` (**6 baseline refs**).

The current file
`crates/ambition_platformer2d_actor_monolith/src/session/lifecycle_commit.rs`
is already a deterministic value/slot module. It does not need session execution
policy to define its state.

**Expected graph receipt after P2:** largest SCC **8 -> 7**.

### Ownership decision

Move the **entire deterministic file** to the shared lifecycle vocabulary owner:

```text
crates/ambition_platformer2d_shared_tangle/src/lifecycle/commit.rs
```

Move these definitions together:

```text
LifecycleIntent
RoomReconstitutionIntent
RoomTransitionIntent
Admission
PendingIntent
PendingLifecycleCommit
```

Keep lifecycle executors/admission producers in their current owning runtime or
content modules. P3 moves vocabulary and rollback state, not execution policy.

### File operations

1. Move `session/lifecycle_commit.rs` to
   `shared_tangle/src/lifecycle/commit.rs` <!-- cite-ok: proposed path created by P3 -->, including its pure slot tests.
2. Export the vocabulary from `shared_tangle::lifecycle`.
3. Move `SnapshotState for PendingLifecycleCommit` from the monolith snapshot
   file into the lower owner (or another file in `shared_tangle` owned by that
   crate). Preserve the existing wire representation.
4. Update the rollback registrar type path without changing its wire ID.
5. Update **every** producer/consumer to import the shared lifecycle path,
   including:
   - shrine checkpoint/transition producers;
   - world room-transition/reconstitution producers;
   - runtime sandbox-reset / room-transition loading;
   - session reset/setup code;
   - app-level replay/transition tests.
6. Delete `session/lifecycle_commit.rs` and its `mod` declaration.
7. Do not add a compatibility re-export under `session`.

### Forbidden end states

```text
crate::session::lifecycle_commit
ambition_platformer2d_actor_monolith::session::lifecycle_commit
```

must have zero production references after P3.

`shared_tangle` must not acquire a dependency on the actor monolith or runtime.

### Required acceptance

Keep production acceptance for:

- room transition admission and confirmed-frame execution;
- same-room replay/reconstitution;
- refusal when another lifecycle intent already occupies the slot;
- rollback across a pending transition/reconstitution;
- shrine-triggered transition/checkpoint path.

At minimum retain the established app-level poisons in:

```text
game/ambition_app/tests/rollback_room_transition.rs
game/ambition_app/tests/canonical_reconstitution.rs
game/ambition_app/tests/room_replay_seam.rs
```

Source receipt:

```bash
! rg -n 'session::lifecycle_commit' crates game -g '*.rs'
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
```

Stop if any moved type requires a session executor, provider, content catalog or
host object merely to exist. That would mean the file is not pure lifecycle
vocabulary as currently measured.

---

## P4 — move actor placement lowering from `world` to `construction`

**Cut:** `construction -> world` (**4 baseline refs**).

The current
`crates/ambition_platformer2d_actor_monolith/src/world/placements.rs` contains
actor-specific specialization over the generic external
`ambition_platformer2d_world::placements` API. It is construction policy, not a
world fact.

**Expected graph receipt after P3:** largest SCC **7 -> 6**.

### Ownership decision

Move the entire actor-specific specialization file to:

```text
crates/ambition_platformer2d_actor_monolith/src/construction/placements.rs
```

Move together:

```text
ActorPlacementContext
LoweringCtx
LoweringFn
PlacementLoweringRegistry
```

Do **not** move generic `PlacementRecord`, generic lowering plans or LDtk
placement vocabulary out of `ambition_platformer2d_world`.

### File operations

1. Move the contents of `world/placements.rs` to
   `construction/placements.rs`. <!-- cite-ok: proposed path created by P4 -->
2. Export the actor specialization from `construction`.
3. Update monolith production callers, including:
   - `construction/mod.rs`;
   - `features/ecs/summon.rs`;
   - `features/ecs/spawn/mod.rs`;
   - `features/ecs/spawn_static.rs`;
   - `session/setup.rs`;
   - `session/reset/mod.rs`;
   - `world/rooms/stage.rs`.
4. Update external callers to the new semantic path, including:
   - `ambition_platformer2d_provider/src/lifecycle.rs`;
   - `ambition_platformer2d_runtime/src/room_transition/loading.rs`;
   - `ambition_platformer2d_runtime/src/lib.rs` facade export;
   - `game/ambition_app/src/app/dev_runtime.rs`;
   - `game/ambition_app/src/app/world_flow/room_transition_assets.rs`;
   - current demo/test callers such as Sanic.
5. Update construction/spawn tests to import the new path.
6. Delete `world/placements.rs` and remove its module declaration.
7. Do not leave `world::placements::{ActorPlacementContext,...}` as a
   compatibility re-export.

### Forbidden end states

After P4, these must be zero in production source:

```text
crate::world::placements::ActorPlacementContext
crate::world::placements::PlacementLoweringRegistry
ambition_platformer2d_actor_monolith::world::placements::PlacementLoweringRegistry
ambition_platformer2d::actors::world::placements::PlacementLoweringRegistry
```

Generic references to `ambition_platformer2d_world::placements::PlacementRecord`
are expected and should remain.

### Required acceptance

Retain/execute tests covering:

- caller-supplied placement lowering registry;
- inert placement row skipped versus active row planned;
- committed placement stamping pickup/feature identity;
- placement reconstitution/respawn through the planner;
- room transition loading with the externally supplied actor lowering registry.

Source receipt:

```bash
! rg -n 'world::placements::(ActorPlacementContext|PlacementLoweringRegistry|LoweringCtx|LoweringFn)' \
  crates game -g '*.rs'
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
```

Stop if moving the actor specialization requires moving generic world placement
records. That would widen P4 beyond its ownership claim.

---

## P5 — mandatory hard-core edge ledger; no code changes yet

After P4, remeasure. The expected SCC is:

```text
abilities, control, features, items, session, world
```

Do not start a fifth carve from the old baseline counts.

### Produce this exact artifact

Update
[`actor-monolith-hard-core-edge-ledger.md`](actor-monolith-hard-core-edge-ledger.md)
from the **post-P4 HEAD**.

For every production reference where both source and destination are inside the
measured SCC, add one ledger row with:

```text
edge
source file + symbol
destination symbol
class
semantic owner
disposition
new path/API if changing
production acceptance
```

Do not restrict the ledger to two-way pairs. One-way links such as
`abilities -> features` can be essential links in a longer cycle.

Allowed dispositions are defined in the decomposition owner. No row may remain
`TBD`.

### Seed questions to resolve, not assumptions to preserve

The current baseline already shows these families and they must be rechecked:

```text
abilities <-> control
    possession/control authority and climb/ascend/descend input seams

control <-> features
    ActingParticipant consumers versus animation-overlay mutation

features <-> world
    overlay/world-prep and feature construction verification

items <-> session
    persistence/save-restored versus durable-horizon installation

world <-> session
    active content/session setup and room lifecycle facts

abilities -> features
    runtime minion spawn

features -> items
    item/persistence adapters and installation

items -> abilities
    item-granted ability installation

session -> abilities/features
    teardown/reset/content staging
```

### P5 exit

P5 finishes only when:

1. every post-P4 internal SCC edge is in the ledger;
2. every row has a non-TBD disposition;
3. the proposed package map is written at the bottom of the ledger;
4. one exact P6 implementation packet is written here, with files, symbols,
   destination, forbidden end state, tests and expected graph effect;
5. or the ledger concludes that the six modules form one coherent package and
   records why each retained edge is legitimate.

No hard-core source edit should land before this receipt exists.

---

## Satellite SCCs

Expected after P1:

```text
actor_spawn <-> character_runtime
```

Treat that pair as a grouped extraction candidate. Do not spend time deleting
its internal cycle until an external consumer needs one half independently.

Current independent SCC:

```text
assets <-> character_sprites
```

Also treat this as a grouped asset-domain question. It is not on the P1-P5
critical path.

## Per-packet final receipt

Before committing each P1-P4 packet:

```bash
git diff --check
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
python3 scripts/modules_md.py
python3 scripts/check_planning_citations.py
```

Then run the packet's focused Rust tests under the repository's guarded target /
disk-headroom workflow. Put the measured SCC transition and test receipt in the
commit message or adjacent planning receipt.
