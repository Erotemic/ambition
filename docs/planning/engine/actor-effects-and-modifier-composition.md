# Actor effects and modifier composition

**State:** PLANNED. This plan is ready for selection, but
[`queue.md`](../queue.md) alone selects execution order.

**Scope:** numeric actor modifiers, timed buffs/debuffs, effect lifetime and the
boundary between effects, capabilities, resources and intrinsic actor tuning.

## Maintainer direction

Characters need a general way to carry buffs and debuffs without adding one
field for each item, pickup or game mechanic.

The durable rule is:

```text
intrinsic actor facts
+ source-owned live effects
-> resolved actor semantics
```

A source must add and remove only its own contribution. It must not save an old
resolved value, overwrite the live value and restore the saved value later.

"Buff" and "debuff" are useful game and presentation words. The engine concept
is neutral. An effect can help in one context and hurt in another.

Do not build one untyped bag that mixes numeric attributes, capabilities,
resources and arbitrary behavior. Keep those semantic domains separate and let
one active effect contribute to more than one domain when needed.

## Why this work exists

The repository already has several correct pieces of this model, but each piece
is local to one feature.

### Equipment numeric modifiers

`ambition_characters::equipment` already defines:

```text
ParamModifier
ModifierOp::Add / ModifierOp::Mul
ModifierScope::Body / Move / Verb
```

`resolved_param` applies all matching additions before all matching
multiplications:

```text
(base + sum(additions)) * product(multiplications)
```

The fold is independent of equipment order. It is also a pure read. Equipment
does not write the resolved value back into the base value.

This arithmetic is not inherently equipment-specific. Equipment is one possible
source of a numeric modifier.

### Capability contributions

`ambition_platformer2d_core::AbilityContributions` already implements the
source-ownership rule for discrete capabilities:

```text
BodyAbilities = (AbilityBase union every lend) intersect every ceiling
```

Each live source owns one keyed contribution. A source removes only its own key.
It never restores a value that another source owns.

Keep this as a typed capability domain. Do not replace it with numeric modifier
machinery.

### Invulnerability reasons

`ambition_characters::actor::Invulnerability` is another source-owned
composition precedent. Transformation, empowerment, scripted protection, move
windows, respawn and submersion hold independent reasons. One reason ending does
not clear the others.

Keep invulnerability as a typed combat semantic unless a later design proves a
more general typed relation. Do not turn it into a numeric attribute only to make
one system look uniform.

### Speed Shoes

Sanic's Speed Shoes currently install one `MomentumBoost` on
`SurfaceMomentumMotion`:

```text
top_speed_scale = 1.4
ground_accel_scale = 1.5
remaining_s = 8.0
```

The movement kernel folds the live boost into `MomentumParams` and spends the
boost timer. This is already better than the old save/mutate/restore pattern.
The authored movement parameters remain unchanged.

The remaining limitation is composition: `boost: Option<MomentumBoost>` is one
special slot for one source and one fixed pair of movement parameters.

### Timed gravity modifier

`AxisManeuverState` has another local timed modifier:

```text
gravity_modifier_scale
gravity_modifier_timer
```

The movement domain owns the timer and the read rule. This also avoids restoring
a saved gravity value. However, the current rule is an implicit replacement
rule: the last caller wins because stacking has not been authored.

This should become an explicit effect/stacking policy rather than remain another
special timed modifier representation.

### EffectRef

`ambition_entity_catalog::EffectRef` is an authored effect reference and
parameter payload. It is a useful delivery seam for moves and content, but it is
not the runtime authority for an actor's active effects.

A content-owned `EffectRef` can eventually ask an effect handler to add, refresh
or remove an actor effect. The active effect state then owns its lifetime and
contributions.

### BODY-PROFILE and intrinsic tuning

BODY-PROFILE work and `AuthoredMovementTuning` describe intrinsic/prepared actor
configuration. They are not buffs or debuffs.

Keep this distinction:

```text
intrinsic/prepared tuning
    = what this actor normally is

live effect contributions
    = what currently changes that actor

resolved semantics
    = what the consuming system uses now
```

## Target model

The names below describe responsibilities. They are not approved Rust type or
crate names.

### 1. Active effect state owns lifetime and provenance

An active effect instance needs enough identity to answer:

- which semantic effect is active;
- which source owns this instance;
- when it expires, if it is timed;
- how another application of the same effect composes with it;
- which typed contributions it provides.

The runtime identity must be deterministic and rollback-safe. Do not copy
`AbilityContributions`' current `&'static str` key type blindly if content-defined
or independently repeated effect instances need a richer identity.

Effects can have different lifetime policies:

- while equipment is worn;
- for a simulation duration;
- while a world condition is true;
- until explicitly consumed;
- until an actor lifecycle boundary.

The owning effect or source controls that lifetime. Consumers must not own effect
expiry.

### 2. Numeric contributions use one deterministic resolver

Equipment, temporary pickups, statuses, forms and environment effects must not
each invent their own arithmetic for the same numeric fact.

The first target is a source-owned numeric contribution model. It should preserve
the useful equipment rule:

```text
resolved = (base + sum(additions)) * product(multiplications)
```

An attribute owner may add semantic clamps or validation after the common fold.
Do not make a global clamp rule that ignores what the attribute means.

A source ending removes only that source's contribution. There is no inverse
operation such as "divide by 1.4" and no saved old value to restore.

### 3. Keep typed semantic domains

The target is not one `HashMap<String, Value>` for the whole actor.

Use separate domains:

```text
numeric attributes
    composable numeric facts such as movement scales or body scale

capabilities
    discrete permissions/verbs through AbilityContributions or another typed owner

invulnerability / defense reasons
    typed combat facts with independent causes

resources
    stateful quantities such as Mana, Limit, Fuel or Catalyst

behavioral effects
    typed event/periodic behavior such as damage-over-time or heal-on-hit
```

One active effect may contribute to several domains. For example, a future heavy
armor effect could reduce movement speed and add a defense capability. This does
not make the domains one authority.

### 4. Resources stay resources

Do not model Mana, Limit, stamina, ammo or another stateful quantity as a numeric
attribute modifier.

[`composable-actor-resources.md`](composable-actor-resources.md) owns stateful
resource pools, costs and transactions. An effect may change a resource-related
policy through a named semantic seam, but a resource value is still authoritative
state with its own lifecycle.

### 5. Intrinsic values stay intrinsic

Do not move authored movement tuning, BODY-PROFILE fields or other immutable base
facts into active effect state only to make resolution uniform.

The normal shape is:

```text
base semantic value
+ live contributions
-> resolved value
```

The resolved value may be cached if measurement requires it, but the cache is a
derived projection. It must not become a second semantic authority.

## Attribute vocabulary

Do not create one universal `ActorAttributes` string map.

Start with real semantic values that already have multiple modifier sources or
special modifier machinery. The initial numeric family should cover the minimum
needed to migrate the current customers, for example:

- surface-momentum top speed;
- surface-momentum ground acceleration;
- gravity multiplier;
- existing equipment body parameters when the generic resolver takes ownership
  of them.

If content-defined attributes later need to grow without a host rebuild, use the
same stable-id -> prepared-handle lesson as actor resources, but keep a separate
storage and lifetime model. Do not introduce dynamic lookup in a movement hot
path only to make the authoring vocabulary look general.

## Stacking policy

Stacking is semantic policy. Do not let component shape or insertion order decide
it accidentally.

An effect kind must be able to state the policy needed by its game meaning. The
minimum useful policies are likely:

- **refresh:** one semantic effect exists; a new application refreshes/replaces
  its duration or payload;
- **replace:** one source wins by an explicit deterministic rule;
- **independent:** repeated instances remain independent and each contributes;
- **strongest:** only the strongest eligible instance contributes while all
  instances can retain their own lifetime.

Do not implement every policy before a real effect needs it. The runtime model
must, however, avoid making one implicit "last writer wins" rule the only shape
that can ever exist.

Speed Shoes should initially preserve their current re-wear behavior. The gravity
modifier should preserve current behavior during migration unless a maintainer or
authored mechanic deliberately selects a new stacking rule.

## Time and rollback

Timed effects are simulation state.

- Spend their timers on the correct simulation/gameplay clock.
- Register authoritative effect instances for rollback when they affect
  simulation.
- Do not use wall-clock time.
- Re-simulation must produce the same active instances and resolved values.
- A derived resolved-attribute cache must either be rollback-derived or restored
  consistently from its authoritative inputs.

Do not store only the final number if the semantic state is "Speed Shoes from
source X are active until tick Y". The effect instance is the reason the number
has its current value.

## Preparation and authoring

Authoring can name effects and numeric parameters. Preparation must validate and
resolve those names before a hot simulation path uses them when practical.

The target authoring flow is:

```text
authored effect / equipment / move
-> preparation and validation
-> typed/prepared contribution plan
-> admitted active effect instance
-> deterministic projection
-> movement/combat/other consumer
```

A missing required attribute/effect handler must refuse at preparation or
admission. Do not silently fall back to an engine default that changes the
meaning of the effect.

## Migration plan

### Phase 0 — freeze current behavior with focused witnesses

Before moving ownership, add or identify tests for these existing semantics:

1. equipment addition/multiplication is order-independent;
2. Speed Shoes change top speed and ground acceleration for their timed life;
3. reapplying Speed Shoes has the current refresh/replace behavior;
4. a movement/base-tuning change while Speed Shoes are live does not require an
   old value to be restored later;
5. the gravity modifier expires to the intrinsic gravity result;
6. independent `AbilityContributions` sources do not clear one another.

These are migration witnesses, not reasons to preserve accidental type names.

### Phase 1 — extract the numeric modifier algebra

Move the generic `Add`/`Mul` vocabulary and fold out of equipment ownership.

Equipment remains an effect/modifier **source**. `WornEquipment` still owns what
is worn and equipment-specific rules such as slots, grants and armor spending.
It no longer owns the generic meaning of numeric addition/multiplication.

Keep move/verb-scoped modifier semantics if they remain useful, but do not force
body attributes and move parameters into one runtime storage model merely because
they share arithmetic.

### Phase 2 — introduce source-owned numeric actor contributions

Add the smallest contribution representation that can express current body-level
numeric customers without a scan over unrelated actor state in the movement hot
path.

Prove:

```text
source A changes attribute X
source B changes attribute X
A ends
-> B still contributes
```

and prove that contribution order does not change the answer.

### Phase 3 — migrate Speed Shoes

Replace `SurfaceMomentumMotion::boost: Option<MomentumBoost>` with the generic
source-owned effect/contribution road.

The movement kernel should consume resolved movement semantics. It must not know
that Speed Shoes exist.

Preserve these behaviors:

- top speed and ground acceleration both change;
- the effect expires on the simulation clock;
- reapplication follows the selected Speed Shoes stacking rule;
- changing the underlying movement tuning while the effect is live changes the
  base under the modifier; there is no restore step.

Delete `MomentumBoost` when no other semantic purpose remains.

### Phase 4 — migrate the timed gravity modifier

Replace the dedicated `gravity_modifier_scale` + `gravity_modifier_timer` pair
with the same active-effect/contribution model.

Make its replacement/stacking rule explicit. Do not accidentally change the
current last-application behavior during the ownership migration.

Delete the dedicated pair after all producers and consumers use the new road.

### Phase 5 — prove cross-source composition

Add one acceptance fixture with at least two independent numeric sources on the
same actor and same attribute family. Good candidates are:

- equipment plus a timed movement effect;
- Speed Shoes plus a slowing status/environment effect.

The fixture must prove that ending either source leaves the other source's
contribution intact.

### Phase 6 — use the model for a true status effect

When a real poison, slow, haste or similar status effect needs runtime state,
make it an `ActiveEffect` customer instead of creating another dedicated
modifier/timer pair.

An authored `EffectRef` may deliver the request, but it does not become the
active-state authority.

## Non-goals

This plan does not require:

- one universal actor property map;
- converting every boolean/capability into a number;
- converting resources into attributes;
- moving intrinsic tuning into rollback state;
- replacing typed combat semantics such as invulnerability reasons;
- a scripting language for arbitrary effect behavior;
- every possible stacking mode before a mechanic needs it;
- preserving the current `MomentumBoost` or gravity-modifier type names.

## Acceptance

The program is complete when the following are true:

- equipment numeric modifiers use the shared numeric modifier vocabulary;
- Speed Shoes use a source-owned actor effect/contribution instead of a special
  movement boost slot;
- the timed gravity modifier uses the same lifetime/contribution model instead
  of its dedicated scale/timer pair;
- two independent sources can modify the same numeric actor semantic and either
  can end without removing the other;
- resolved values are deterministic and independent of insertion order where the
  selected stacking policy says order must not matter;
- the movement kernel consumes resolved semantics and contains no Speed Shoes or
  other named-content knowledge;
- authoritative base values are never overwritten and later restored as effect
  implementation;
- timed effects rollback and re-simulate deterministically;
- resources, capabilities and invulnerability keep their own typed authorities;
- a new timed numeric buff/debuff can be added without adding another dedicated
  modifier field to a body or movement-policy struct.

## Review traps

Do not accept these shapes as completion:

```text
one Option<SpecialBoost> per mechanic
```

```text
apply effect -> mutate base value -> save old value -> restore later
```

```text
one generic HashMap<String, Value> for every actor fact
```

```text
active effect list + independently mutable resolved attribute authority
```

```text
resource value treated as an attribute because both are numbers
```

Prefer one source-owned effect lifetime and one deterministic projection into the
typed semantic domain that consumes it.
