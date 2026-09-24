# Capability progression and world gating — Engine 1.0 program

**State:** OPEN. Route gating is established; the remaining work is capability
ownership and new world facts, not another generic gate mechanism.

## Goal

Make exploration and progression primarily emerge from **what the controlled body
can do, what it carries/equips, and what the world has physically become**.
Explicit story sequencing remains available, but it should not be the default
representation for mechanical progression.

The engine owns reusable facts and queries. Ambition content owns which abilities,
items, characters and world events carry progression meaning.

## Current route-gating model

Authored lock walls use one prepared condition line. A bare `gated_by` value keeps
the compatibility meaning `world.flag_set <value>`; a qualified line can select a
published condition such as `body.can`, `body.fits`, `inventory.holds` or
`world.switch_on`.

`ConditionCatalog` is the extension boundary. A new gate family should normally
publish a fact/question into that catalog rather than add another wall type or a
parallel route-gating interpreter.

The physical gate result is the world collision overlay. Body collision,
projectiles and rendering consume that same gate geometry, so the authored
condition must decide one mechanical wall state rather than three independent
consumer states.

### Gate families

- **body capability:** climb, fly, morph, blink, portal use, attack/tool ability;
- **body property:** size, mass class, locomotion type, damage/resistance facts;
- **item/equipment:** physical key, tool, wearable or held capability source;
- **world mechanism:** bridge repaired, machine powered, door/switch state;
- **soft systemic pressure:** danger, traversal difficulty, hostile population;
- **social/knowledge:** cooperation or durable learned information;
- **story gate:** explicit authored sequencing when story state is the real fact.

A family does not need a route predicate before it has an authoritative fact to
read. Do not create placeholder facts merely to complete this list.

## Body capability authority

The current body capability layers are:

| layer | current owner | role |
|---|---|---|
| authored defaults | character/gameplay authoring | source used to construct the body |
| `AbilityBase` | body component | intrinsic authored capability set |
| `BodyAbilities` | body component | effective set read by movement/gameplay |
| `AbilityContributions` | body component | keyed lends and ceilings, one key per source |
| admitted developer mask | mechanical edit domain | a ceiling contribution on the primary player |
| participant progression | none | not currently a separate authority |

`AbilityBase` exists so restrictions do not destroy the body's authored identity.
`BodyAbilities` is the runtime projection, recomputed every tick by
`ambition_platformer2d_core::project_body_abilities`:

```text
effective = (base ∪ every lend) ∩ every ceiling
```

A source owns one key in the body's `AbilityContributions` and never writes
`BodyAbilities`. The current sources:

| key | source | contribution | lifetime |
|---|---|---|---|
| `dev.editable_ability_mask` | `ambition_dev_tools::contribute_editable_ability_mask` | ceiling | while an admitted mask exists |
| `falling_sand.room_swim` | `falling_sand_sim::lend_room_swim` | lend `swim` | while the falling-sand room is active |
| `portal.transit` | `portal::withhold_wall_verbs_during_transit` | ceiling without the four wall verbs | while `PortalTransit` |

Sources write in `WorldPrepSet::BeforeIntegrate`; the projection runs between it
and `Integrate`. The contributions are declared rollback-derived: each source
rewrites its key from its own state every tick.

A grant that belongs to an activation rather than a live situation (Morph Ball,
the Ambition game's grant to its home body) is folded into `AbilityBase` when the
session is set up, through `HomeBodyAbilities`, and is not a contribution.

## Body gate semantics

### `body.can(verb)`

Reads the effective body capability set. It asks whether a body is currently
granted the named verb. This is a capability question, not an observation of the
current animation/action.

### `body.fits(height)`

Currently reads the body's current `BodyKinematics.size`, so crouching or another
posture change can change the answer on the next simulation tick. That is a
posture/state question, not a capability question.

Whether the route-facing body family should contain both meanings is a product
choice in [Q58](../awaiting-maintainer-decision.md#q58--does-the-body-gate-family-ask-what-a-body-can-do-or-what-it-is-doing).
Do not silently change `body.fits` to standing size or to "can reach a fitting
stance" before that decision.

## Co-op gate subject

The current route road evaluates body conditions over driven bodies, which means
one qualifying participant can satisfy the shared wall condition. Because the
wall is one mechanical object used by collision, projectiles and rendering,
per-player passability would require a different mechanism rather than a stricter
query.

The intended rule is open in
[Q54](../awaiting-maintainer-decision.md#q54--in-co-op-does-a-body-gate-open-for-the-party-the-acting-body-or-only-the-primary-body).

## World-mechanism facts

A gate can only ask about a world mechanism that publishes a durable/current fact.
For example, `world.switch_on` has a named switch authority. A broken arbitrary
breakable is not automatically a durable progression fact merely because its ECS
component is rollback state.

When a design needs "this mechanism is in state X", first decide the semantic
owner and lifetime of that fact. Then publish a condition over it. Do not persist
every transient object in order to make a gate expression possible.

## Item/equipment and progression

Item possession can already be queried through the published item/custody facts.
The unresolved product question is whether a unique capability item is itself the
entitlement or only one occurrence of an entitlement; see
[Q45](../awaiting-maintainer-decision.md#q45--is-a-unique-capability-item-an-entitlement-or-an-occurrence).

Permanent participant-level ability progression does not currently exist as a
separate authority. If the game needs it, prefer a grant that contributes to the
controlled body's effective capabilities rather than a second parallel answer to
"can this body do X?". Possession/body switching then becomes a defined grant
transfer rule instead of reconciliation between body and participant capability
stores.

## Compound requirements

The route gate prepares one condition line. It is not a general boolean
expression language. Dialogue can compose boolean conditions in its own language;
that does not imply route gates can or should do the same.

When authored content first needs a compound route rule, choose the smallest
model that satisfies the real use case. Options include a deliberately limited
`and`/`or` form or one named derived fact published by the owning content/domain.
Do not grow a universal scripting language pre-emptively.

## Social/knowledge and soft gates

Transient perceptual memory and durable social knowledge are different facts.
Perception already owns short-lived belief/observation state. Durable facts such
as "this character knows X" need a persistence owner before a route condition can
read them.

Likewise, a "soft gate" matters only when a route planner/navigation consumer can
reason about traversability cost. Do not build a soft-gate taxonomy before that
consumer exists.

## Engine/game boundary

Prefer typed engine facts and queries when multiple systems need the same
mechanical meaning. Keep story names, character identities and progression design
in game/content layers.

Do not turn every gate into a generic quest condition. Conversely, do not let
collision, AI, UI and authoring independently infer the same capability when one
published fact can serve them all.

## Open work

1. **Resolve Q58** before authoring `body.fits`/body-state gates broadly.
2. **Resolve Q54** before a co-op product depends on asymmetric traversal rights.
3. **Resolve Q45** before unique capability items become permanent progression.
4. When a second real temporary/permanent ability grant appears, design one
   contribution/projection road with the falling-sand swim customer and remove
   direct save/restore writes of the effective set.
5. Add world-mechanism, social/knowledge or soft-pressure conditions only after
   the authoritative fact they read exists.
6. Add compound route expressions only when a concrete authored gate cannot be
   represented cleanly by one published fact.

## Acceptance

- one route-gating mechanism consumes published facts rather than bespoke per-game
  wall logic;
- `AbilityBase` remains intrinsic authority and `BodyAbilities` remains the
  effective projection consumed by gameplay;
- new restrictions/grants cannot permanently erase intrinsic capability;
- co-op subject semantics are explicit before content depends on them;
- durable progression facts have a named owner and lifetime;
- route conditions do not become a second source of truth for the state they
  query.
