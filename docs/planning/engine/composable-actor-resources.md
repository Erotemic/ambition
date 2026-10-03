# Composable actor resources and prepared bindings

**State:** partly built. Named resources live in a per-actor bank; Smash Limit
and Ambition Mana use it. Prepared handles, plan identity and the main-game
economy slice are open.

**Scope:** actor resources such as fuel, Mana, Limit, stamina, shield energy and
oxygen, and the binding from authored resource identity to the runtime systems
that use them. It does **not** define a universal stat set or require Health,
Mana, Limit or any named value on every actor.

Related owners: [character authoring](character-authoring-package.md) (the
`PreparedCharacterDefinition` boundary),
[content generation and reload](content-generation-and-reload.md) (generations
and mechanical reload), [extension model](extension-model.md),
[expressive move capabilities](expressive-move-capabilities.md),
[simulation authority](simulation-authority-and-determinism.md) (rollback and
peer determinism).

## Decision

Use one general **resource composition and binding model**, not one general
resource vocabulary.

```text
stable authored resource identity
        -> character / ruleset / build composition
        -> validation and deterministic preparation
        -> compact prepared resource handles
        -> dense actor-local mutable resource state
```

A simulation system never scans an actor's resources and never hashes a
resource name per tick. Preparation resolves names and roles once; the runtime
uses a prepared handle and direct indexed access.

Four identities, four different facts:

```text
stable authored resource identity  portable, durable, in save data    (R4)
ResourceLayoutId                   which slots, in what order         (R13)
PreparedActorResourcePlanId        that layout + costs + bindings     (R13)
prepared slot / handle             a local runtime address, never saved (R4)
```

Rollback restores the plan id (R10); the layout id is a projection of it. The
store behind a plan id is an arena with a generation lifetime (R14).

A character with no resources is valid. A Smash fighter with no Limit is valid.
Do not infer that all numeric character facts must use one `Attributes`
component.

## Current implementation

- `ResourceId`, `ResourceCost` and `ResourceDeclaration` live in the leaf crate
  `ambition_resource_spec`. The identity is a digest of the authored name.
- `ActorResources` is the per-actor bank: one `Arc<ResourceLayout>` plus a
  `Vec<ResourceLevel>`, registered as `body.resources`.
- Move prices are `MoveGates::costs: Vec<ResourceCost>`. Affordability
  (`can_pay`) and payment (`pay`, at move start in
  `ambition_combat/src/moveset/mod.rs`) read the same bank on the same tick and
  are atomic over every term. A term naming a resource the body does not hold is
  unaffordable; a body with no bank is refused unless the price is empty.
- **Limit:** Smash declares it in `MatchRules::resources`
  (`SMASH_LIMIT.declaration()`, capacity 60, starts empty). Seat preparation
  builds `ActorResources::declared(&rules.resources)`
  (`ambition_match/src/prepared.rs`). Limit fill systems and `smash.fill_meter`
  reach the slot by name, so a body with no Limit gains nothing.
- **Mana:** `ambition_abilities::mana::{MANA, POOL, REGEN_PER_SEC}` is declared
  by the Ambition provider through
  `PreparedPlatformerSource::with_home_body_resources(HomeBodyResources::declared(&[POOL]))`.
  Only the Ambition home body holds Mana. `PlayerManaRegen` has no default. Read
  models carry `Option`; the HUD prints `MP -` for a body with no Mana.
- Spawn and reset share one baseline: `ActorResources::reset_to_start` restores
  the declarations the seat was built from.
- `BodyMana`, `ResetMeter` and the implicit move meter are deleted.
  `ResourceMeter` survives only as the value type of non-body meters (projectile
  ammo).

Guards: `a_body_without_mana_cannot_spend_it`,
`no_stated_rate_refills_nothing_and_no_pool_gains_mana`,
`hud_facts_track_the_controlled_body`,
`a_match_that_declares_no_limit_fills_nothing`,
`a_body_that_holds_no_limit_gains_nothing_from_any_limit_source`, and the Smash
test that every seat's layout is exactly `{smash.limit}`.

**Not built:** cached prepared slot handles (access is a binary search over the
layout, `ResourceLayout::slot`; every shipped layout holds one resource, so the
search is one comparison), `PreparedActorResourcePlanId` and the plan arena
(R10, R13, R14), layout migration during play, and the main-game multi-resource
slice (Phase 4).

## Concepts

- **Resource identity:** a stable authored name in a content namespace
  (`ambition:mana`, `smash:limit`, `main:fuel`). Never a Bevy `ComponentId`, an
  entity id or a prepared slot.
- **Resource shape:** numeric representation only. The current shape is a
  bounded finite float. A bounded integer shape needs no new identity system.
- **Resource declaration:** a character or build owns a resource; capacity and
  initial-value choice. No regeneration, decay, damage or selection policy.
- **Capability resource requirement:** a capability role and its required shape
  (`jetpack.fuel -> main:fuel`). Several roles can bind to one resource without
  creating copies.
- **Prepared resource handle:** a compact local address valid only for its
  layout. Type it by numeric shape (`ResourceHandle<BoundedF32>`), never one Rust
  type per resource name.
- **Resource policy:** why and when a value changes (fill from damage,
  regenerate, decay, reset on stock loss). Policy belongs to the game, ruleset or
  capability, never the pool.

## Core invariants

- **R1. Zero resources is a complete composition.** No placeholder Mana or Limit
  to satisfy a generic query.
- **R2. Resource semantics are content-defined.** A new resource identity,
  capacity, initial value, binding or ordinary cost needs no Rust type and no
  engine rebuild once the content-pack road is complete.
- **R3. No search for a known resource.** Access is direct through a prepared
  handle, independent of how many resources the actor holds.
- **R4. Stable identity and runtime address are different facts.** No slot
  numbers in authoring or save data. Layout and plan identities are
  content-derived and peer-stable; they may cross to a peer and into a snapshot,
  but never into durable save data.
- **R5. One mutable authority per resource.** No HUD, role, cost or policy
  mirror of the live value.
- **R6. Missing required state fails closed.** A positive cost naming a missing
  resource is invalid at preparation when possible and refused at admission
  otherwise. Never a free move.
- **R7. Multi-resource payment is atomic.**
- **R8. Policy is outside storage.**
- **R9. Prepared layouts are immutable.** A changed resource set creates a new
  layout. Migration maps stable identities, never slot numbers.
- **R10. Rollback restores the values and the plan they are read through.**
  Rollback state is current values, per-instance capacities and the actor's
  active plan id. Plan contents are immutable generation data reached through
  that id. Restoring values alone lets a rewind pair layout A's bank with layout
  B's handles, and both sides pass their own checks. If the bank carries a
  layout id for its handle check, that copy is declared rollback-derived, not
  restored independently.
- **R11. Peer layout is deterministic.** No dependence on hash-map iteration,
  entity allocation, registration order or Bevy component ids.
- **R12. The bank is not a universal numeric-state mandate.** Walk speed,
  knockback weight, HP, velocity and timers stay with their owners.
- **R13. Two identities, each content-derived over its own content.**
  `ResourceLayoutId` covers the canonical slot layout.
  `PreparedActorResourcePlanId` covers the whole plan: layout, prepared move
  costs and capability-role bindings. Each is a digest of canonical bytes or a
  dense ordinal over a canonically sorted set of an admitted generation, never
  a cache or intern-table order (the `RollbackOrdered` insertion-index class, see
  the queue's ID-PEER row). Two plans that share a layout but differ in one cost
  or one binding have the same layout id and different plan ids.
- **R14. A plan a snapshot can name is an arena, not a cache.** Plans are
  immutable and append-only within a rollback generation; reclaim at a
  generation or timeline reset. A durable save never names a plan, so it never
  extends a plan's life.

## Authoring and ownership

A character package declares zero or more resources (syntax not selected):

```text
resources:
  fuel:     { shape: bounded_f32, capacity: 100, initial: full }
  catalyst: { shape: bounded_f32, capacity: 20,  initial: 5 }
```

Preparation rejects: a required binding with no resource; a wrong-shape
binding; incompatible declarations of one identity; duplicate mutable
authorities; a cost naming a resource the character cannot own; a layout that
cannot be encoded deterministically. Load order never resolves a conflict.

- **Character and build authoring own:** which resources exist, their stable
  identities, character-specific capacities and initial facts, intrinsic
  bindings, and build declarations that add a resource.
- **Capability and ruleset owners own:** the meaning of a role, fill, drain,
  decay and regeneration policy, cost policy, stock, respawn, checkpoint and
  possession policy.
- **Composition binds** a role to a character's resource; neither side invents
  the other's half. The prepared composition carries the resolved layout and
  bindings.

A Smash game must not install one global Limit meaning on every fighter. The
Limit resets on stock loss (Q67 ruling, 2026-10-03): the Smash Limit declares
`ResourceStart::Empty`, which applies on spawn and on every respawn. A future
game rule that carries the meter must say so explicitly.

## Preparation boundary

`PreparedCharacterDefinition` holds character-authored resource facts. It is not
always the final binding boundary: a match can grant a moveset and a loadout can
add a resource later. Finish the layout and bindings at the last composition
boundary that knows all inputs:

```text
prepared character facts + ruleset facets and grants + moveset + build/loadout
        -> PreparedActorResourcePlan (conceptual)
             resource_layout -> PreparedResourceLayout
             prepared moves  -> PreparedResourceCost handles
             prepared facets -> handles for capability roles
```

Do not serialize `PreparedCharacterDefinition` wholesale. Assign slots in a
canonical order of stable identity and shape. Slot 0 can mean different
resources in different layouts; the layout identity prevents cross-layout use.
Prepare one bound move view per distinct layout rather than keep a semantic
lookup in the move-start hot path.

## Runtime

Illustrative storage (names not selected):

```rust
struct ResourceBank { layout: ResourceLayoutId, pools: SmallVec<[BoundedF32; 4]> }
struct ResourceHandle<T> { layout: ResourceLayoutId, slot: ResourceSlot<T> }
```

Access is `bank.get(handle)` / `bank.get_mut(handle)` with a layout check in
development and test builds; a stale handle fails clearly. Do not store semantic
ids beside live values. Do not require one heap allocation per resource. A cost
visits its terms, not every resource on the actor.

**Costs.** Authoring supports an atomic list of terms (`8 fuel, 2 catalyst`);
preparation resolves each to a handle. A character ability may name a resource
directly; a reusable capability names a role that composition binds. Both
resolve to the same handle form. Validate the complete payment, debit all terms,
then start the move. Payment happens at the accepted start boundary and before
destructive teardown; trigger and cancel roads share it.

**Economy and progression.** Keep authored base cost -> economy policy ->
effective payment -> atomic transaction separate. When efficiency changes only
with equipment or progression, prepare a compact effective cost plan at that
change. No generic modifier language.

**Resource-set changes during play** create or select a new layout through an
explicit admitted transition that maps stable identities and applies an
explicit removal policy. Never copy by slot number; never mutate a layout in
place.

## Rollback, peers and saves

The bank is authoritative simulation state. Snapshots carry values, the
instance data to interpret them and the active plan id (R10). Decode validates
numeric invariants before it creates live state.

Peer determinism poisons, each failing differently: (1) permute unordered input
and the layout must not change; (2) prepare one plan on two peers after
different irrelevant histories and both ids must match (an intern ordinal passes
1 and fails 2); (3) two plans sharing a layout but differing in one cost or one
binding have equal layout ids and different plan ids.

Saves never use slot numbers. A save that must survive a layout change uses
stable identity and a versioned schema. Choose the contract when a real save
migration needs it.

## Content iteration

Resource declarations and costs are mechanical content and follow the generation
and admission rules. These edits need no host build or link: capacity change,
new resource, removed optional resource, amount change, rebinding a capability,
one-term to two-term cost.

## Physical layout options

Keep the logical model fixed while you measure physical layouts.

| Option | Use when | Risk |
| --- | --- | --- |
| P1 one dense bank component | default; one authority, direct access, simple atomic payment and rollback | coarse Bevy write conflicts |
| P2 a few banks by numeric shape | a second shape exists or measured conflicts justify it | never one bank per semantic resource |
| P3 resource entities | a resource needs independent identity or lifetime | lookup, cleanup, rollback identity and atomic payment get harder |
| P4 static component per resource | compile-time control only | breaks content-defined resources |
| P5 runtime map or tagged vector | dynamic-lookup control only | the lookup preparation exists to remove |
| P6 dynamic Bevy component per resource | only if P1's conflict is material and a prototype proves no host rebuild, scoped access, known write sets, deterministic rollback and clean reload | high |
| P7 external columnar store | only if profiling proves the ECS bank is a limit | reimplements lifetime, rollback, borrows, cleanup |

Bevy component ids are App-local addresses, never portable identities.

Do not create `ActorAttributes`. Promote a numeric fact to a generic attribute
family only when several capabilities bind to one content-defined fact, content
must add attributes without a rebuild, the value varies per instance, or
duplicate storage already exists. A later attribute family may reuse the
stable-id -> prepared-handle technique with its own storage.

Keep the resource facility's pure surface small because it can be a high fan-out
dependency. Do not move game vocabulary into `ambition_platformer2d_core`.

## Phases

**Phase 0 — prove the seam.** R10, R13 and R14 are fixed before the prototype.
Prepare and run three compositions on the real preparation boundary and the real
`trigger_moveset_moves` road: (A) a Smash fighter with no resources and no bank;
(B) a Limit fighter with fill policy and one priced move; (C) a stress character
with Fuel, Catalyst and unrelated resources, one Fuel-only and one Fuel+Catalyst
ability. Measure access cost by resource count, one- and multi-term payment,
capability fill and drain, rollback encode size and time, and scheduler
serialization. Record which commands run for each content edit. Select the
physical layout and write the measurements here before Phase 1.

**Phase 1 — authoring and preparation:** stable identity, checked shape,
declarations, deterministic layout, layout-specific handles, binding validation,
the selected bank, rollback registration with validated decode, inspection that
reports id, slot, value and layout without mutating.

**Phase 2 — explicit prices (landed).** Open: resolve cost terms to handles at
preparation.

**Phase 3 — Mana and Limit ownership (landed).**

**Phase 4 — main-game economy slice:** two or more independent resources,
abilities on different resources, one multi-resource ability, one progression or
loadout efficiency rule, and no engine edit when names, capacities, bindings or
base costs change. If a new identity needs an engine edit, reopen the design.

## Acceptance

- **Composition:** resource-less fighter; Limit fighter; a fighter with another
  resource and no Limit; a multi-resource actor; two roles bound to one resource;
  equal values stay independent; missing or wrong-shape binding refuses.
- **Access:** no scan over resource ids; unrelated resources do not change the
  lookup; no string lookup for prepared access; a one-term cost visits one term.
- **Costs:** free ability needs no bank; positive cost with no resource refuses;
  one debit per payment; all-or-nothing multi-term payment; refusal never pays
  partially; trigger and cancel agree; payment precedes destructive changes.
- **Lifecycle:** explicit initial state; stock policy outside the pool; exact
  checkpoint restore; migration by stable id; explicit removal policy.
- **Rollback and peers:** exact rewind; repeated payment and fill on
  resimulation; values feed the checksum; peers derive the same slots;
  permutation does not change layout; R13 agreement and separation arms; a
  rewind across a plan change read through a capability handle (R10); a rewind
  across a progression resolves the plan id (R14), with a generation reset as
  its control; invalid decode refuses.
- **Content iteration:** capacity, cost, new resource and binding edits need no
  host rebuild; an invalid candidate leaves the active generation unchanged.

## Poison tests

Each poison needs a nonempty control. Restore `missing => affordable`; scan
`Vec<(ResourceId, Pool)>` for known access; order a layout by map insertion; use
a layout-A handle on layout B; debit the first term of a failed payment; put
regeneration into the pool; give every Smash fighter a dummy Limit; require a
Rust component for a new resource; persist a slot number in a save; add a HUD or
role mirror; omit the plan id from the snapshot (R10); derive an id from an
intern or cache ordinal (R13); restore the bank's layout-id copy independently
(R10); derive the plan id from the layout and rewind across a cost-only plan
change (R13); evict a plan and rewind to a frame that names it, with the store's
miss path instrumented so a silent re-prepare cannot hide the defect (R14).

## Forbidden regressions

- No universal `BodyResource` on every actor; no central enum of resource names.
- No strings or hash maps in the prepared hot path; no slot numbers in
  authoring.
- No Rust type per content-defined resource; no duplicated value for a role, HUD
  or policy; no fill, decay or efficiency policy in storage.
- Absent required state never means free behavior.
- No in-place layout mutation; no slot-number migration.
- No `ActorAttributes` map; no immutable tuning moved into rollback state for
  uniformity; no generic modifier engine before a concrete policy.
- No compile or runtime win claimed without the measured lane and its control.

## Closure

Zero-resource actors are first-class; identities are content-defined; known
access is direct through prepared handles; layouts are deterministic and
immutable; missing costs fail closed; payment is atomic; policy is outside
storage; Limit exists only where composed; ordinary resource edits need no host
rebuild; rollback restores exact state and peers agree; main-game efficiency is
expressible without engine changes; no universal attribute bag exists.
