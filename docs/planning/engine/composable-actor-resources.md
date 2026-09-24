# Composable actor resources and prepared bindings

**State:** partly built. Named resources live in a per-actor bank; Smash Limit
and Ambition Mana use it. Prepared handles, plan identity and the main-game
economy slice are open.

This page owns actor resources such as fuel, Mana, Limit, stamina, shield energy,
and oxygen. It also owns the binding from authored resource identity to the
runtime systems that use the resource.

It does **not** define one universal set of character stats. It does not require
Health, Mana, Limit, or any other named value on every actor.

Related owners:

- [character authoring](character-authoring-package.md) owns the coherent
  character package and the final `PreparedCharacterDefinition` boundary;
- [content generation and reload](content-generation-and-reload.md) owns portable
  artifacts, preparation, generations, and mechanical reload;
- [extension model](extension-model.md) owns the no-host-relink content and
  extension boundary;
- [expressive move capabilities](expressive-move-capabilities.md) owns the wider
  move-mechanic inventory;
- [simulation authority](simulation-authority-and-determinism.md) owns rollback
  and peer determinism.

## Decision summary

Use one general **resource composition and binding model**, not one general
resource vocabulary.

The target flow is:

```text
stable authored resource identity
        |
        v
character / ruleset / build composition
        |
        v
validation and deterministic preparation
        |
        v
compact prepared resource handles
        |
        v
dense actor-local mutable resource state
```

A simulation system must not scan an actor's resources to find the value it
needs. A simulation system must not hash a resource name on each tick. The
preparation step resolves names and roles once. The runtime uses a prepared
handle and direct indexed access.

There are several identities in that flow, and they are different facts:

```text
stable authored resource identity  portable, durable, in save data    (R4)
ResourceLayoutId                   which slots, in what order         (R13)
PreparedActorResourcePlanId        that layout + costs + bindings      (R13)
prepared slot / handle             a local runtime address, never saved (R4)
```

Rollback restores the PLAN id ([R10](#r10-rollback-restores-the-values-and-the-plan-they-are-read-through));
the layout id is a projection of it, because many plans may share one layout and
never the reverse. The store behind a restored plan id is an arena with a
generation lifetime, not a cache
([R14](#r14-a-prepared-plan-a-snapshot-can-name-is-an-arena-not-a-cache)).

A character with no resources is valid. A Smash fighter with no Limit is valid.
A fighter with Limit has the Limit resource and the capability that owns its
fill policy. Another fighter can have a different resource. The main game can
compose several resources and can give different abilities different costs.

The general mechanism is the **prepare-and-bind model**. Do not infer that all
numeric character facts must use one `Attributes` component.

## Current implementation

Built (see Phase 3 below for detail):

- `ResourceId`, `ResourceCost` and `ResourceDeclaration` live in the leaf crate
  `ambition_resource_spec`. The identity is a digest of the authored name.
- `ActorResources` is the per-actor bank: one `Arc<ResourceLayout>` plus a
  `Vec<ResourceLevel>`, registered as `body.resources`.
- Move prices are `MoveGates::costs: Vec<ResourceCost>`. Affordability and
  payment use `ActorResources::can_pay`/`pay`, atomic over every term. A term
  that names a resource the body does not hold is unaffordable.
- Smash declares the Limit in `MatchRules::resources`; the Ambition provider
  declares Mana on the home body only. Reset restores the same declarations
  (`reset_to_start`).

Not yet built: cached prepared slot handles (access is a binary search over the
layout), `PreparedActorResourcePlanId` and the plan arena (R10, R13, R14),
layout migration during play, and the main-game multi-resource slice (Phase 4).

## Required distinctions

Keep these concepts separate in both authoring and runtime design.

### Resource identity

A resource identity is a stable authored name in a content namespace.

Examples:

```text
ambition:mana
smash:limit
main:fuel
main:catalyst
```

The exact syntax is not selected here. The identity must be stable in portable
content. Do not use Bevy `ComponentId`, an entity id, or a prepared slot as the
portable identity.

### Resource shape

The shape answers only numeric representation questions.

The first implementation needs the shape required by current Mana and Limit.
That is a bounded finite float. Do not add a numeric trait framework before a
second shape needs it.

A future discrete resource can justify a bounded integer shape. Adding that
shape must not require a new semantic identity system.

### Resource declaration

A declaration states that a character or composed build owns a resource. It
contains the facts needed to create valid instance state, for example capacity
and the authored initial-value choice.

A declaration does not own regeneration, decay, damage, death, or ability
selection policy.

### Capability resource requirement

A capability can require a resource input. The requirement states the required
shape and the role the capability needs.

Examples:

```text
jetpack.fuel       -> main:fuel
limit_gain.meter   -> smash:limit
move.default_cost  -> main:fuel
```

The left side is a capability role. The right side is a resource identity.
Several roles can bind to one resource. This does not create several mutable
copies.

### Prepared resource handle

Preparation resolves a stable identity to a compact local address. The handle
is valid only for the prepared resource layout that created it.

Prefer type safety by **numeric shape**, not by game semantic. An illustrative
API can use `ResourceHandle<BoundedF32>` and later
`ResourceHandle<BoundedI32>`. This creates a small number of shape
specializations. It does not create one Rust type per resource name.

The runtime uses the handle. It does not repeat semantic lookup.

### Resource policy

Policy answers why and when a value changes.

Examples include:

- fill from damage;
- regenerate with world time;
- decay after a delay;
- start empty;
- start full;
- reset on a stock loss;
- preserve across a stock loss;
- change ability cost because of progression.

Policy belongs to the game, ruleset, or capability that owns the rule. It does
not belong in the generic resource pool.

## Core invariants

### R1. Zero resources is a complete composition

A moving actor can have no resource storage. Do not add an empty Mana, Limit, or
resource placeholder to satisfy a generic body query.

### R2. Resource semantics are content-defined

Adding a new resource identity in a supported content pack must not require a new
Rust component type or an engine rebuild.

Changing a resource capacity, initial value, binding, or ordinary ability cost
must not require an engine rebuild after the content-pack road is complete.

### R3. The simulation does not search for a known resource

After preparation, a system that needs one known resource performs direct
indexed access through a prepared handle.

Adding unrelated resources to an actor must not make that access proportional to
the number of resources on the actor.

### R4. Stable identity and runtime address are different facts

A stable resource id is portable and can survive preparation. A prepared slot is
a local runtime address.

Do not put slot numbers in authoring or durable save data.

A layout or plan identity is neither of those two things. It is not a runtime
address: it is content-derived and peer-stable, so it may cross to a peer and into
a snapshot. It is not an authored identity either, so it must not appear in
durable save data, which outlives the generation that gave it meaning.
[R13](#r13-there-are-two-identities-and-each-is-content-derived-over-its-own-content)
owns how each is derived; [R14](#r14-a-prepared-plan-a-snapshot-can-name-is-an-arena-not-a-cache)
owns how long what it names survives.

### R5. One mutable authority per resource

A resource has one live mutable value. HUD state, a capability role, a prepared
cost, and a policy must not mirror the current value into another component.

### R6. Missing required state fails closed

An ability with no resource cost needs no resource bank.

A positive cost that refers to a missing resource is invalid during preparation
when preparation has enough information. A runtime absence that still reaches
move admission must refuse. It must not become a free move.

### R7. Multi-resource payment is atomic

If an ability costs several resources, payment changes all required values or
changes none of them.

Do not debit one term before the complete payment is known to succeed.

### R8. Policy is outside storage

The generic resource value contains no regeneration rate, decay rate, damage
rule, death rule, stock-loss rule, or efficiency rule.

### R9. Prepared layouts are immutable

A prepared layout does not reorder or insert slots in place. A changed resource
set creates a new layout.

A transition to a new layout is an explicit state migration. It maps stable
resource identities, not old slot numbers.

### R10. Rollback restores the values and the plan they are read through

Rollback state is: current resource values, any per-instance capacities, and the
identity of the actor's active prepared plan.

Everything else the plan holds (layout metadata, capability-role handles,
prepared move costs) is immutable generation data, looked up by that identity. Do
not copy it into a snapshot, and do not leave the pointer to it out of one.

If only the values are restored, a build change that switches plans mid-session
can rewind to layout A's bank while the capability still presents layout B's
handle. A handle check compares the layout, and two plans can share one layout,
so both sides believe they agree.

Use one pointer, not two. The active plan identity is the single restored
pointer; the layout identity is a projection of it (many plans may share one
layout, never the reverse). If the bank carries a layout id for its handle check,
that copy is derived and registers through the `declare_rollback_derived_*` road
rather than being restored independently. This rule is unconditional: it applies
whether or not a given session changes plans.

### R11. Peer layout is deterministic

Two peers with the same admitted mechanical generation and the same prepared
character/build must derive the same resource layout and bindings.

Canonical layout construction must not depend on hash-map iteration, entity
allocation, registration order, or Bevy component ids.

### R12. Resource storage is not a universal numeric-state mandate

Walk speed, knockback weight, HP, velocity, timers, and other numeric values do
not move into the resource bank because they are numeric.

A second state family can reuse the prepare-and-bind pattern later if it has a
real customer and the same semantics.

### R13. There are TWO identities, and each is content-derived over its OWN content

```text
ResourceLayoutId            = identity of the canonical SLOT LAYOUT
                              (which resources, in which order, with what
                              per-slot metadata)

PreparedActorResourcePlanId = identity of the WHOLE PREPARED PLAN
                              (that layout, plus the prepared move costs, the
                              capability-role bindings, and every other piece of
                              immutable generation data the plan selects)
```

Each must be a function of ITS OWN canonical content: either a digest over the
canonical bytes, or a dense ordinal assigned from the canonically sorted set an
admitted generation declares. Neither may be the order in which a cache, an
intern table or a preparation queue first saw the thing.

The plan id is derived from the plan's content, not the layout's. Two plans can
share a layout and still differ:

```text
plan A:  fuel -> slot 0, stamina -> slot 1    dash costs 3 fuel
plan B:  fuel -> slot 0, stamina -> slot 1    dash costs 7 fuel
```

They have the same `ResourceLayoutId` and must not share a
`PreparedActorResourcePlanId`: a rewind that restores the id has to say which
cost and binding set to read the values through. Two plans that differ only in a
capability-role binding are the same failure and are harder to see.

R11 forbids unordered iteration in construction; this rule also covers the id.
Two peers can derive byte-identical layouts and still disagree if the id is an
intern-table ordinal: the id is what the checksum sees. The repository has
already paid for this class once: `RollbackOrdered.order(rollback_id)` is an
App-lifetime insertion index that GGRS hashes beside each value (see
[simulation authority](simulation-authority-and-determinism.md) and the queue's
ID-PEER row). An interned layout ordinal is host-local lineage with a
mechanical-sounding name.

The acceptance needs two arms that point opposite ways, because each alone passes
for the wrong reason:

1. **Agreement across irrelevant history.** Two peers reach the same full
   prepared plan by different preparation orders and agree on both ids. This arm
   alone is satisfied by returning a constant.
2. **Separation within one layout.** Two plans that share a layout but differ in
   a prepared move cost, and two that differ only in a capability-role binding,
   have the same `ResourceLayoutId` and different
   `PreparedActorResourcePlanId`s. This arm alone is satisfied by a counter.

A dense-ordinal scheme must satisfy both: ordinals are assigned from a canonical
ordering of the full plan, not of the layout and not of the encounter or cache
order.

### R14. A prepared plan a snapshot can name is an arena, not a cache

Prepared plans are immutable and append-only within a rollback generation. A plan
may not be replaced, mutated or evicted while any rollback snapshot, confirmed
frame, or other generation-bound admitted state can still name it. The
reclamation boundary is a generation or timeline reset, which is the moment
nothing admitted can point backwards any more.

"Generation-bound" is exact. A durable save never names a plan (R4), so it never
extends a plan's life; an exact-generation transient (a rollback snapshot, an
in-memory checkpoint) always does.

This follows from R10: once the restored pointer is a plan id, the store behind
it has a lifetime contract. A cache that may evict fails like this:

```text
actor progresses A -> B, the cache replaces A
rewind restores PreparedActorResourcePlanId(A)
A is gone
```

The values come back correct and there is nothing to read them through, on a
single machine. So the store is an arena, and that is settled before Phase 0: a
cache with an eviction policy and an arena with a generation boundary are
different objects.

The witness is a rewind across a progression, and it must name the plan rather
than the values: advance an actor from plan A to plan B, rewind past the change,
and assert that the restored `PreparedActorResourcePlanId` resolves.

Plans are still reclaimed. Append-only within a generation with a reset boundary
is a bounded arena, not a leak; the population is the set of plans an admitted
generation can declare, which R13 already requires to be canonically enumerable.

## Authoring model

A character package can declare zero or more resources.

Example only:

```text
resources:
  fuel:
    shape: bounded_f32
    capacity: 100
    initial: full

  catalyst:
    shape: bounded_f32
    capacity: 20
    initial: 5
```

This syntax is not selected. The important contract is that the source declares
semantic identities and valid creation facts. It does not declare runtime slot
numbers.

A character facet or reusable capability can also state a required resource
role. Composition binds that role to one declared resource.

Preparation must reject:

- a required binding with no resource;
- a binding to the wrong resource shape;
- two incompatible declarations for one stable resource identity;
- duplicate mutable authorities for one resource;
- an ability cost that names a resource the prepared character cannot own;
- a layout that cannot be encoded deterministically.

Do not use load order as conflict resolution. If two fragments can override one
resource fact, the override must be explicit in the authoring contract.

## Character and ruleset ownership

Use the existing character-authoring ownership split.

### Character and build authoring own

- which character-specific resources exist;
- stable resource identities in the character package;
- character-specific capacities and initial numeric facts when these are truly
  facts about the character;
- bindings that are intrinsic to the character or one of its facets;
- progression/loadout declarations that add a resource to a concrete build.

A reusable facet can contribute a resource declaration when the resource is part
of that facet. The final composition still has one declaration authority for the
resource identity. A requirement and a declaration are different facts.

### Capability and ruleset owners own

- the meaning of a resource role;
- fill, drain, decay, and regeneration policy;
- cost policy;
- stock, respawn, checkpoint, and possession policy;
- operations that consume the resource.

### Game/provider composition owns

The general list is in [character authoring](character-authoring-package.md),
which applies the same ownership split to resources. Resource-specific rules:

- a capability owns the meaning of a resource role and its fill/spend policy;
  composition binds that role to the character's resource, and neither may
  invent the other's half;
- the final prepared composition carries the resolved layout and bindings, so
  runtime systems never search authored names again.

A Smash game must not install one global Limit meaning on every fighter. A
fighter that does not author or compose a Limit capability has no Limit resource
and no Limit policy.

The existing Q67 stock-loss question remains a product policy question. The
resource storage design must support either answer without changing the generic
pool.

## Preparation boundary

`PreparedCharacterDefinition` is the natural boundary for character-authored
resource facts. It is not always the final binding boundary. A match can grant a
moveset or another ruleset fact after character preparation, and a loadout can
add a resource after the base character definition exists.

Do not serialize `PreparedCharacterDefinition` wholesale into the portable
content format. Lower portable resource declarations into prepared character
facts, then finish resource layout and binding at the last composition boundary
that knows all required inputs.

That final boundary can include:

```text
prepared character facts
+ selected ruleset facets and grants
+ selected moveset
+ current build/loadout facts
        |
        v
PreparedActorResourcePlan   (conceptual name only)
```

The prepared result needs two kinds of information:

1. immutable layout metadata shared by all instances with that layout;
2. prepared bindings used by runtime capabilities and prepared moves.

A conceptual result is:

```text
PreparedActorResourcePlan
    resource_layout -> PreparedResourceLayout
    prepared moves  -> PreparedResourceCost handles
    prepared facets -> resource handles for capability roles
```

If the character definition already contains all required inputs, this plan can
be prepared once as part of character preparation. If a ruleset or build supplies
more inputs, finish it at that later composition boundary. Do not resolve a
resource slot before all facts that can change the layout are known.

The plan store is an arena, not a cache: see
[R14](#r14-a-prepared-plan-a-snapshot-can-name-is-an-arena-not-a-cache).
Prepare once; do not evict.

The exact type name, fields, arena owner, and crate placement are Phase 0
outputs. Retention is fixed by R14.

### Deterministic layout

Preparation assigns compact slots. Use a canonical ordering derived from stable
resource identity and shape. Do not use insertion order from an unordered map.

For a layout:

```text
main:catalyst -> slot 0
main:fuel     -> slot 1
```

another character can have:

```text
smash:limit   -> slot 0
```

Local slot zero can mean different resources in different layouts. That is
correct. The layout identity prevents a handle for one layout from being used on
another.

Prepared move costs are also layout-specific. If one ruleset grants the same
portable moveset to actors with different resource layouts, prepare one bound
move view per distinct layout. Do not keep one semantic id lookup in the
move-start hot path to avoid this preparation step.

## Runtime storage hypothesis

The first physical layout to prototype is one optional actor component that owns
a dense resource bank.

Illustrative shape only:

```rust
struct ResourceBank {
    layout: ResourceLayoutId,
    pools: SmallVec<[BoundedF32; 4]>,
}

struct ResourceSlot<T> {
    index: u16,
    _shape: PhantomData<fn() -> T>,
}

struct ResourceHandle<T> {
    layout: ResourceLayoutId,
    slot: ResourceSlot<T>,
}
```

The final API does not need these names or this container type.

The required access shape is:

```text
bank.get(handle)
bank.get_mut(handle)
```

with a layout check in development/test configurations. A stale handle must fail
clearly. It must not read another resource that happens to occupy the same local
slot.

A release build can use a more compact handle only after measurement and after
another mechanism proves that the handle cannot cross layouts.

### Do not store semantic ids beside every live value

The prepared layout already knows which semantic id owns each slot. Repeating an
id in every actor pool wastes memory and encourages runtime lookup.

### Do not require one heap allocation per resource

Phase 0 must compare practical bank containers. The logical design does not
select `SmallVec`, `Vec`, boxed slices, or another compact container in advance.

Measure common actor counts and common resource counts.

## Runtime access patterns

A capability stores or can obtain the prepared handle for the resource role that
it consumes.

Example:

```text
PreparedJetpack
    fuel -> BoundedF32Handle(layout A, slot 1)
```

The runtime system performs:

```text
ECS component lookup
-> layout check when enabled
-> direct slot access
```

It does not perform:

```text
for each resource:
    if id == fuel:
        ...
```

It also does not perform a string or hash-map lookup for `fuel` on each tick.

A system can iterate the terms in an ability cost. That work is proportional to
the number of cost terms, not to the number of resources the actor owns.

This is a required performance property, not an optional optimization.

## Ability costs

Move prices are `MoveGates::costs: Vec<ResourceCost>`, which name their resource.
The target model adds prepared costs that resolve each term to a handle.

The portable authoring form must support at least an atomic list of resource
terms:

```text
cost:
    8 fuel
    2 catalyst
```

The prepared form resolves each semantic reference to a resource handle:

```text
PreparedCost
    [(fuel_slot, 8), (catalyst_slot, 2)]
```

A zero-cost move has an empty cost and does not require a resource bank.

### Direct resource reference and reusable role are different needs

A character-owned ability can name a specific semantic resource such as
`main:fuel`.

A reusable capability can instead define a role such as `jetpack.fuel` and let
composition bind the role to a character resource.

Do not force every direct cost through a global role registry. Do not force every
reusable capability to hard-code a game resource name.

Preparation resolves both cases to the same runtime handle form.

### Payment commit

Move selection can inspect affordability while the move is still refusable.
Payment must occur before any destructive move-start teardown and at the one
accepted start boundary.

For several terms:

```text
validate complete payment
-> if all terms can pay, debit all terms
-> start the move
```

A failed payment changes no resource and does not tear down the current move.
Trigger and cancel roads use the same payment semantics.

## Resource economy and progression

The main game needs more than storage. It can have several resources, different
ability costs, and progression that changes how efficiently a build uses those
resources.

Do not put this policy in `ResourceBank`.

Keep these stages separate:

```text
authored base cost
        |
        v
resource-economy policy
        |
        v
effective payment
        |
        v
atomic resource transaction
```

Examples of future game policy include:

- a movement family uses less fuel;
- a technique family uses less catalyst;
- a build changes the ratio between two resources;
- a progression choice gives an ability another payment route.

Do not implement a generic formula or modifier language in this campaign.

When an efficiency rule changes only when equipment, progression, or a loadout
changes, prefer to prepare a compact effective cost plan when that build changes.
Do not recompute a general expression on every simulation tick.

A temporary runtime effect can require a live policy input later. Add that road
from a concrete mechanic. Do not create a second mutable copy of each ability's
cost.

## Resource-set changes during play

A prepared layout is immutable, but a game can later need to add or remove a
resource from an actor because of progression, transformation, or equipment.

That operation creates or selects a new prepared layout. The transition must be
explicit.

Migration uses stable resource identity:

```text
old main:fuel -> new main:fuel
new main:catalyst -> its declared creation rule
removed resource -> explicit removal policy
```

Do not copy by slot number.

The transition must update the bank and any layout-specific prepared bindings as
one admitted actor-state change. A later implementation can rebuild a prepared
actor/build object if that is the cleanest owner. Do not mutate slot assignments
in place.

This design permits runtime progression without making ordinary per-tick access
dynamic.

## Rollback, checksum, and persistence

### Rollback

The resource bank is authoritative mutable simulation state.

Snapshot data contains the live resource values, the instance data required to
interpret them, **and the actor's active prepared-plan identity** — see
[R10](#r10-rollback-restores-the-values-and-the-plan-they-are-read-through). The
plan's contents are generation data reached THROUGH that identity, so the
snapshot carries one small pointer rather than a copy of the layout.

Without the pointer, the actor holds layout A's values while its capabilities
hold layout B's handles; both sides pass their own checks and the reads are
silently wrong. A test that asserts "rewind restores exact resource values"
passes in that state, so the arm in
[acceptance](#rollback-and-peer-determinism) reads a value through a capability
handle after a rewind across a plan change.

Decode must validate bounded numeric invariants before it creates live state.

### Peer determinism

The admitted mechanical content identity covers resource declarations, resource
bindings, prepared costs, and the rules that can change mechanics.

Two peers that admit the same content must derive byte-equivalent canonical
layouts, and the same identity for them
([R13](#r13-there-are-two-identities-and-each-is-content-derived-over-its-own-content)).
Three poisons, because they fail differently:

1. change unordered input iteration and prove the prepared layout does not
   change;
2. prepare the same plan on two peers after different irrelevant histories and
   prove both identities are unchanged. An intern-table ordinal passes (1) and
   fails (2), and (2) is the one that reaches the checksum;
3. prepare two plans that share a layout and differ in one prepared move cost,
   and two that differ only in a capability-role binding, and prove the
   `ResourceLayoutId`s match while the `PreparedActorResourcePlanId`s differ.
   Without this one, (1) and (2) are both satisfied by deriving the plan id from
   the layout.

### Save data

Do not make local slot numbers the durable save identity.

If save data must survive a different prepared layout, save by stable semantic
resource identity and a versioned resource schema. If a save is deliberately
bound to one exact mechanical generation, that contract can use the generation
identity to interpret a compact form.

Choose the persistence contract when a real save migration needs it. Do not use
rollback slot layout as an accidental long-term save format.

## Content packs and edit-to-play latency

The resource design must use the existing content-generation model.

A resource declaration and an authored resource cost are mechanical content.
Preparation lowers them to immutable layout and binding data. A valid edit must
follow the same generation/admission rules as other mechanical content.

The target developer loop is:

```text
edit resource or ability content
-> prepare affected content section and dependents
-> validate bindings
-> admit a new mechanical generation
-> run with no host Cargo/link step
```

The following edits must be explicit Phase 0/Phase 1 acceptance cases:

- change a resource capacity;
- add a new resource identity to a character;
- remove an optional resource from a character;
- change one ability's amount;
- bind a reusable capability to a different resource;
- change a main-game ability from one resource term to two.

If any ordinary edit above requires an engine rebuild, the content-pack seam is
not complete for resources.

## ECS and scheduler cost

One `ResourceBank` component can cause false Bevy write conflicts. Two systems
that mutate different slots still borrow the same component.

Do not ignore this cost. Also do not solve it before it is measured.

The first spike compares at least these physical options while it preserves the
same authoring and prepared-handle model:

### P1. One dense resource bank

One optional component owns all current pools for the actor.

Strengths:

- one mutable authority;
- direct indexed access;
- simple atomic multi-resource payment;
- simple rollback and lifecycle;
- good locality for actors with a small number of resources.

Risk:

- coarse Bevy write conflict.

### P2. A small number of storage-class banks

Separate banks by numeric shape only if another shape exists or the scheduler
measurement justifies the split.

This can reduce some false conflicts. It must not introduce a bank per semantic
resource.

### P3. Resource entities

Each resource is an entity related to its actor.

Do not select this without a resource that needs independent entity identity or
lifetime. It makes lookup, cleanup, rollback identity, and atomic payment more
complex.

### P4. Static Rust component per semantic resource

This is the compile-time control.

It has excellent Bevy query behavior. It fails the content-defined-resource goal
because a new resource semantic requires a new Rust type and rebuild.

### P5. Runtime map or tagged vector

This is the dynamic-lookup control.

Examples are `HashMap<ResourceId, Pool>` and `Vec<(ResourceId, Pool)>`.

Do not select this for the hot runtime if the prepared dense layout has
comparable or better cost. Preparation exists specifically so the simulation
does not need repeated key lookup.

### P6. Dynamic Bevy components per resource

A content loader could register one runtime ECS component id per resource
semantic. This can give the scheduler more precise write sets than one bank.

Do not select this without a prototype that proves all of these points:

- content can add the component without a host rebuild;
- static capability systems can access the dynamic component without broad
  `World` access;
- the scheduler can know the dynamic read/write set before it runs the system;
- rollback registration and decode remain deterministic;
- content reload can add or replace layouts without leaving stale query state.

Bevy component ids remain App-local implementation addresses. They are never the
portable resource identity.

### P7. External columnar resource store

A separate store can group resource values by layout or resource and give very
compact columnar access.

Do not select this unless profiling proves the ECS bank is a real limit. A
separate store must reimplement or bridge entity lifetime, rollback, query
borrows, cleanup, migration, and inspection. That cost is larger than the array
lookup this campaign is trying to optimize.

## Relation to generic attributes

Do not create `ActorAttributes` as part of this campaign.

Resources have a clear shared lifecycle: they are per-instance quantities that
are filled, drained, tested, and paid. That lifecycle gives the dense bank a
coherent job.

Other numeric facts can have different owners:

- immutable walk speed can stay in prepared locomotion data;
- knockback weight can stay in the platform-fighter facet that owns its meaning;
- HP can stay in the Health capability while its numeric pool can later reuse a
  small bounded-value primitive;
- velocity and timers remain mechanic state.

A later attribute family can reuse the stable-id -> prepared-handle technique if
it has a concrete need for content-defined dynamic identity and per-instance
storage.

Promote a numeric fact into a generic attribute family only when at least one of
these conditions is true:

1. several independent capabilities need to bind to the same content-defined
   numeric fact;
2. content must add new semantic attributes without a Rust rebuild;
3. the value must vary per actor instance and cannot stay in immutable prepared
   capability data;
4. repeated custom lookup or duplicate storage is already present.

Do not move a value only because its Rust type is numeric.

## Compile-cost position

Compile cost still matters for engine work, but content edit latency has higher
priority for ordinary game tuning.

The target architecture avoids monomorphizing gameplay systems once per resource
semantic. Runtime systems operate on `ResourceBank` and prepared handles.

Phase 0 still measures engine-edit compile cost because the resource facility can
be a high-fan-out dependency. Keep its pure value and identity surface small.
Do not move game vocabulary into `ambition_platformer2d_core` to avoid one new
crate or module.

Crate placement is a Phase 0 result. Prefer the smallest owner that preserves the
content-pack and runtime dependency direction.

## Phase 0 — prove the seam before migration

Build a small vertical prototype against real Ambition paths. Do not start with a
large `BodyMana` migration.

Three contracts are decided before 0A, because they are cheap to state now and
expensive to retrofit under a live save format and a live checksum:

1. **What the snapshot carries:** values, capacities and the active plan
   identity, with layout metadata, capability-role handles and prepared move
   costs as generation data keyed by it
   ([R10](#r10-rollback-restores-the-values-and-the-plan-they-are-read-through)).
   A prototype that snapshots only values passes its own rewind arm and hides the
   defect until an actor changes build mid-session.
2. **How the identity is derived:** content digest, or a dense ordinal over a
   canonically sorted set inside the admitted generation; never the order a cache
   first saw the layout
   ([R13](#r13-there-are-two-identities-and-each-is-content-derived-over-its-own-content)).
3. **How long a named plan survives:** append-only within a rollback generation,
   with a generation/timeline reset as the reclamation boundary
   ([R14](#r14-a-prepared-plan-a-snapshot-can-name-is-an-arena-not-a-cache)).

Phase 0 may choose the type, the field names and the arena owner. It does not
choose these three; 0B/0C exercise them.

### 0A. Three character compositions

Prepare and run these cases:

**Case A — resource-less Smash fighter**

```text
Smash combat
no Limit
no Mana
no ResourceBank
```

**Case B — Limit fighter**

```text
Smash combat
Limit resource
Limit fill policy
one priced move
```

**Case C — main-game resource stress character**

```text
Fuel
Catalyst
at least one extra unrelated resource
one Fuel-only ability
one Fuel + Catalyst ability
```

The stress fixture can add many unrelated resources. A Fuel-only access must
still use one direct prepared slot access.

### 0B. Use the real preparation boundary

The prototype must lower from content-shaped resource declarations into the same
kind of prepared character boundary used by production character content.

Do not prove the design with a unit-only map that bypasses content preparation.

### 0C. Use the real move-start path

Prototype prepared resource cost on the real `trigger_moveset_moves` / move-start
road or a faithful extracted unit with the same ownership and borrow shape.

Prove trigger and cancel paths use the same affordability and payment contract.

### 0D. Runtime measurements

Measure at least:

- resource-less actors;
- actors with one resource;
- actors with four resources;
- a stress layout with many resources;
- one-term payment;
- multi-term payment;
- direct fill/drain by a capability;
- rollback encode/decode size and time;
- scheduler serialization caused by mutable bank access.

Record access cost as resource count grows. A known-resource access must not turn
into a linear scan.

### 0E. Content-edit measurements

Using the content-pack road, show which commands run for:

- capacity-only edit;
- new resource declaration;
- new resource binding;
- cost-only edit;
- one-resource to two-resource cost edit.

The target has no host build/link step for these edits.

### 0F. Physical-layout decision

Select P1, P2, or another measured physical layout without changing the logical
resource identity and prepared-binding contract. Prototype P6 only if P1's
scheduler conflict is material in the measured workload. Do not build P7 unless
an ECS-backed option has a measured runtime or memory limit.

Do not choose a runtime map because it is easier to prototype. Do not choose a
static generic component because its Rust API looks cleaner if it breaks the
content-defined-resource requirement.

Write the measurements and selected physical layout into this page before Phase
1 starts.

## Phase 1 — establish resource authoring and preparation

1. Add the stable resource identity in a pure content-safe owner.
2. Add the minimal current resource shape with checked construction.
3. Add character resource declarations.
4. Add deterministic prepared layout generation.
5. Add layout-specific prepared handles.
6. Add explicit capability/resource binding validation.
7. Add the selected actor runtime bank.
8. Add rollback registration and validated decode.
9. Add inspection that can report stable id, prepared slot, current value, and
   owning layout without becoming a mutation authority.

Acceptance:

- a character with zero resources prepares and runs;
- a content-defined resource creates no Rust type;
- two independent resources on one actor do not alias;
- stale cross-layout handles fail clearly;
- peers derive the same layout from the same content;
- invalid numeric state cannot enter through safe authoring or rollback decode.

## Phase 2 — replace the implicit move meter

Landed. Prices are explicit `ResourceCost` terms; a missing positive-cost resource
is refused, payment is atomic, and there is no compatibility reading in which a
missing resource means free. Open: resolve cost terms to prepared handles at
preparation (see 0D).

## Phase 3 — migrate Mana and Limit ownership

Landed. `BodyMana`, `ResetMeter` and the implicit move meter are deleted.

**Limit.** Smash declares the Limit in `MatchRules::resources`
(`SMASH_LIMIT.declaration()`, capacity 60, starts empty). Seat preparation puts
`ActorResources::declared(...)` on the seed, and the bank is inserted in the
spawn flush. The Limit fill systems and the `smash.fill_meter` technique reach
the Limit slot by name, so a body that holds no Limit gains nothing. An
exploration goblin holds no Limit, so its dive always takes the uncharged
variant.

**Mana.** `ambition_abilities::mana::{MANA, POOL, REGEN_PER_SEC}` is declared by
the Ambition provider through
`PreparedPlatformerSource::with_home_body_resources(HomeBodyResources::declared(&[POOL]))`.
Only the Ambition home body holds Mana; a possessed body, an NPC, a demo avatar
and a Smash seat hold none, so Mana abilities are refused on them. The Mana
abilities, shrine, mana cell, dev panel and regen reach the level through
`mana::spend`/`mana::level`. `PlayerManaRegen` has no default: a composition
that states no rate refills nothing. Read models carry `Option`; the HUD prints
`MP -` for a body with no Mana.

Tests: `a_body_that_holds_no_mana_cannot_fire_the_beam`,
`no_stated_rate_refills_nothing_and_no_pool_gains_mana`,
`hud_facts_track_the_controlled_body`, and the Smash A/B that asserts every
seat's layout is exactly `{smash.limit}`.

`ambition_platformer2d_core::player_state::ResourceMeter` survives only as the
value type of non-body meters, which is a real owner.

### Verified 2026-09-24 — the Smash Limit slice holds (campaign item 3)

Re-derived against the source, not this page:

- **Authoring → layout → dense storage.** `MatchRules::resources` declares the
  Limit (`SMASH_LIMIT.declaration()`); seat preparation builds
  `ActorResources::declared(&rules.resources)` (`ambition_match/src/prepared.rs`);
  the bank is one `Arc<ResourceLayout>` plus a `Vec<ResourceLevel>` per body.
- **Costs name their resource.** `MoveGates::costs: Vec<ResourceCost>`; the
  afford check (`resources::can_pay`) and the move-start pay
  (`ambition_combat/src/moveset/mod.rs`, `bank.pay`) read the same bank on the
  same tick. A term naming a resource the body does not hold is unaffordable,
  and a body with no bank is refused unless the price is empty. The pay site
  discards `pay`'s bool on purpose, as its comment says: the refusal happened at
  afford time, and the two cannot disagree within one tick.
- **Spawn and reset share one baseline.** `reset_body_clusters` calls
  `ActorResources::reset_to_start`, which restores the SAME declarations the
  seat was built from. `BodyMana` and `ResetMeter` are deleted, so there is no
  full-Mana frame followed by a Limit normalization.
- **Absence.** `a_match_that_declares_no_limit_fills_nothing` and
  `a_body_that_holds_no_limit_gains_nothing_from_any_limit_source`.

Open, and REASONED rather than measured: the "prepared handle" is still a
binary search over the body's layout (`ResourceLayout::slot`) at each access,
not a slot cached at preparation. Every shipped layout holds ONE resource
(`{smash.limit}` on a seat, `{mana}` on the home body), so the search is one
comparison. The 0D measurement comes due when a shipped layout holds several.
`ResourceMeter` survives only as the value type of non-body meters (projectile
ammo), which is a real owner, so step 7's deletion does not apply.

## Phase 4 — main-game resource economy vertical slice

Prove that the architecture supports the intended main-game direction without a
new engine resource type.

Build one content-authored slice with:

- at least two independent resources;
- abilities that use different resources;
- one ability that needs more than one resource;
- one progression or loadout rule that changes resource efficiency;
- no Rust engine edit when resource names, capacities, bindings, or base costs
  change.

The efficiency rule remains game policy. The resource bank remains generic
storage and transaction support.

If a new resource identity needs an engine edit in this slice, reopen the
resource identity/preparation design before adding another workaround.

## Future attribute work

Do not block resource implementation on a general attribute framework.

If a later customer needs content-defined mutable attributes such as a generic
speed or armor value, first test whether the resource preparation machinery can
be extracted into a small shared slot-layout primitive. Keep separate domain
storage when the lifecycle differs.

A successful future extraction can look like:

```text
stable semantic id
-> deterministic prepared layout
-> typed local handle
```

with separate owners for:

```text
resources
attributes
other state families that genuinely match
```

Do not combine those families into one mutable map only to reuse lookup code.

## Acceptance tests

### Composition

- resource-less Smash fighter;
- fighter with Limit;
- fighter with a different optional resource and no Limit;
- main-game actor with several independent resources;
- two capability roles intentionally bound to one resource;
- two resources with equal numeric values remain independent;
- missing required capability binding refuses preparation;
- wrong-shape binding refuses preparation.

### Access complexity

- one known resource access performs no scan over resource ids;
- adding unrelated resources does not change the lookup algorithm;
- runtime source contains no string lookup for prepared resource access;
- a one-term ability visits one cost term, not every actor resource.

### Costs

- free ability needs no bank;
- positive cost with no resource refuses;
- one-resource payment debits exactly once;
- multi-resource payment is all-or-nothing;
- fallback/refusal does not partially pay;
- trigger and cancel paths agree;
- payment happens before destructive move-start changes.

### Lifecycle

- explicit empty/full/authored initial state;
- respawn/stock policy is owned outside the pool;
- checkpoint restore is exact;
- layout migration preserves resources by stable id, not old slot;
- a removed resource follows an explicit removal policy.

### Rollback and peer determinism

- rewind restores exact resource values;
- resimulation repeats payment and fill results;
- a resource value change affects the authoritative checksum as required by the
  rollback registry;
- two peers prepare the same slots from the same canonical content;
- permuting unordered source insertion does not change prepared layout;
- two peers that prepare the same plan after different irrelevant preparation
  histories agree on both identities (R13);
- two plans that share a layout and differ only in a prepared move cost, and two
  that differ only in a capability-role binding, have the same
  `ResourceLayoutId` and different `PreparedActorResourcePlanId`s (R13);
- a rewind across a plan change restores the active plan identity with the
  values, witnessed by reading a resource through a capability-role handle after
  the rewind, not by comparing the bank (R10);
- a rewind across a progression resolves the restored plan identity (R14). Its
  control is a generation reset, after which reclaiming the old plan is correct;
- invalid decode refuses instead of constructing an invalid pool.

### Content iteration

- capacity edit: no host rebuild;
- cost edit: no host rebuild;
- resource add: no host rebuild;
- binding edit: no host rebuild;
- invalid candidate keeps the active generation unchanged.

## Poison tests

The implementation is not complete until deliberate regressions fail for the
intended reason.

Add poisons that:

1. restore `missing resource => affordable`;
2. scan `Vec<(ResourceId, Pool)>` for a prepared known-resource access;
3. make layout order depend on unordered map insertion;
4. use a handle from layout A on layout B;
5. partially debit the first term of a failed multi-resource payment;
6. put regeneration or decay policy back into the generic pool;
7. give every Smash fighter a dummy Limit resource;
8. make a new content resource require a new Rust semantic component;
9. persist a local slot number as the semantic save identity;
10. create a second mutable projection for HUD or capability-role state;
11. leave the active plan identity out of the snapshot and rewind across a plan
    change (R10);
12. derive the layout identity from an intern-table or cache ordinal instead of
    the layout's content (R13);
13. restore the bank's own layout-id copy independently instead of declaring it
    derived (R10) — two restored copies of one fact;
14. derive the plan identity from the layout's content, then rewind an actor
    whose two plans share a layout and differ in a prepared move cost (R13) —
    the values come back and are read through the wrong costs;
15. evict or replace a prepared plan when the actor progresses past it, then
    rewind to a frame whose snapshot names it (R14). Instrument the plan store's
    miss path when you run this poison: a store that silently re-prepares an
    equivalent plan on a miss hides the defect.

Each poison must have a nonempty control that proves the witness exercised the
subject.

## Forbidden regressions

Do not:

- replace `BodyMana` with one universal `BodyResource` that every actor carries;
- create a central enum of all game resource names;
- use strings or hash maps in the prepared simulation hot path when a handle is
  already available;
- expose runtime slot numbers in authoring;
- make a content-defined resource require a Rust type;
- duplicate a resource value for a role, HUD, or policy;
- put fill/decay/efficiency policy into generic storage;
- make absent required state mean free behavior;
- mutate a prepared layout in place;
- copy layout migrations by slot number;
- create a universal `ActorAttributes` map as part of this migration;
- move immutable character tuning into rollback state only to make the APIs look
  uniform;
- add a generic modifier/expression engine before a concrete policy requires it;
- claim a compile or runtime win without the measured lane and control.

## Closure condition

Close this design campaign only when all of these are true:

```text
zero-resource actors are first-class

resource identities are content-defined and do not require Rust semantic types

known-resource runtime access is direct through prepared handles

resource layouts are deterministic and immutable

positive missing costs fail closed

multi-resource payment is atomic

resource policy is outside generic storage

Limit exists only on characters that compose it

Mana and Limit no longer share BodyMana authority

ordinary resource and cost edits use the content-pack road with no host rebuild

rollback restores exact resource state and peers derive the same layout

main-game multi-resource efficiency can be expressed without changing the engine

no universal ActorAttributes bag was introduced without a separate customer
```
