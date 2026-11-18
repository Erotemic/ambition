# Create native character compositions

`create_actors` builds characters through the native construction planner. A
script can define a reusable factory that selects characters, initial behavior
state, status selectors and native relations. No new Rust adapter is needed for
each factory or character variation.

The characters must be in the active session's prepared cast. The character
catalog owns their bodies, movement, combat kit and default controller policy.
There is one body construction path. A giant gets its complete limb cluster;
scripts do not build its collision or limb rig themselves.

## Request and result

```rhai
fn sentry(key, at, period) {
    #{key: key, character: "practice_sandbag", at: at,
      allegiance: "neutral", starting_wallet: 3, holds: ["my_game.sentry"],
      state: [#{schema: "sentry.memory", value: #{
          title: key, elapsed: 0.0, period: period, shots: 0
      }}]}
}

this.requests.push(request("create_actors", #{
    reply: #{request: 1, selector: "my_game::defense/result"},
    actors: [sentry("left", [200.0, 100.0], 0.3),
             sentry("right", [400.0, 100.0], 0.7)]
}));
```

The submitting entry declares `create_actors` in `requests`. It also declares
every initialized schema in `writes`. `schema` is the complete local schema
key, without its provider or revision. The schema must be body-attached and
declared by the same module. An initializer supplies all fields, including
nested fields; misspelled, missing, duplicate or incompatible fields refuse the
batch. State is owned by the extension host and takes part in rollback.

`holds` selects ordinary native status behavior from birth. A `status_tick`
entry bound to that selector can use its seeded record on the new body. This
works for any controller. Each body has independent records. Libraries can
share the factory and behavior functions.

The action domain is `world.create_actors`. The callback is a normal
[`action_result`](action-results.md) entry in `actor_think`. On success, the
terminal result has stage `applied`, code `actors_created`, and `subjects` in
the same order as `actors`. Each subject is the actual root body's semantic
identity. Internal limbs do not add subjects. On refusal, stage is `rejected`,
code is `actor_creation_refused`, and `message` explains the failed condition.
No body is built and no spawn counter is spent by a refused batch. A repeated
request uses the existing action-ticket retry rules.

Creation runs when the native request adapter consumes the invocation. The
result names completed construction, not a promise to build later. Callbacks
arrive at the next eligible actor read cut. A source entry or recipient that
retires cannot receive a callback in a replacement lifetime.

## Wallet capability at birth

`starting_wallet` is an optional signed 32-bit balance on each actor node.
An integer creates or overrides that body's wallet. Zero creates an empty
wallet. A negative balance starts in debt, using the native wallet rules.
Omitting the field, or using `()`, keeps the character's authored birth state.
It does not copy the creator's money.

A character catalog row can supply `starting_wallet: Some(7)`. A registered
`CharacterDefinition::with_starting_wallet(7)` overrides the catalog row. The
creation request overrides either source for that occurrence. If no source
supplies a balance, construction adds no wallet; any host-provided wallet
remains intact. These rules do not depend on the controller or faction.

The shared native body applier distinguishes construction from replacement.
Construction applies the starting balance once. Live character changes and
catalog reloads preserve the body's current balance and wallet presence.
Changing a costume cannot refill money or create a wallet on an existing body.

The wallet is complete before the child's behavior runs or construction is
reported as applied. It can fund [inventory exchanges](inventory-exchanges.md)
and [contact consumption](contact-consumption.md). Actor creation, its balance,
and later exchanges rewind through the existing native snapshot path. Existing
session restore policies still own saved balances. This does not add durable
NPC wallets or durable dynamic actors.

The portable request is `ambition.world.create_actors@2`.

## Placement, allegiance and relations

`at` and `half_extents` describe the placement box in world units, with +Y down.
Half extents default to `[16.0, 24.0]` and must be finite and positive. The
prepared character can determine its final body size. `face_left` defaults to
false. The native character constructor owns body sizing and collision;
this interface does not write raw collider components.

`allegiance` can be `inherit`, `player`, `enemy`, `npc`, `boss` or `neutral`.
The default inherits the creator's faction. A room root with no faction supplies
neutral allegiance. This selects allegiance, not the controller: a body on a
player's side can still use an autonomous controller. An unpiloted giant's limbs
take its faction too.

Each node has a unique, nonempty `key` of at most 128 bytes. Optional `links`
name those keys:

```rhai
links: [#{from: "guard", to: "rival", kind: "grudge"}]
```

`grudge` and `mount` use the existing native relation rules. Mount compatibility,
conflicting seats, unknown endpoints and other invalid compositions refuse
before any body is created. References to existing bodies can be supplied in
state initializers. State does not yet support symbolic references to another
node's future identity; retain the returned identities for later coordination.

## Ownership, bounds and authoring

The creator must have a live session, room and semantic identity. New roots and
their limbs carry native dynamic provenance and the creator's current session
and room stamp. They have room lifetime. The creator can disappear without
removing already-created bodies. These runtime occurrences are not durable
authored placements. A saved state reference alone does not reconstruct them.
General retirement outcomes and durable dynamic actors remain separate work.

`ActorCreationLimits` governs each room. Engine defaults are 32 requested actors,
96 complete construction rows, 512 room bodies, 64 links, 16 status selectors
per actor, 16 state records per actor and 16,384 initial state bytes per batch.
Zero disables the corresponding allowance. Ordinary bodies and internal limbs
both count against the row and room bounds. Native callers receive the same
checks.

Games can supply an `actor_creation_rules` source in their content pack. Ambition
uses [actor_creation_rules.ron](../../game/ambition_content/assets/data/actor_creation_rules.ron)
and publishes valid edits through its existing content-reload boundary. These
rules govern Ambition's untagged rooms. Other games can declare their own
`ActorCreationLimits` with `DeclareRulesExt` and their own `RulesScope`.

Initializers use bounded logical data, not VM objects or closures. The transport
accepts at most 65,536 nodes and one MiB per initializer, with enough depth for a
complete SDK state record. The room's smaller state bounds also apply. Integer
values stay exact. Table keys have canonical order on the wire.

Portable Rust uses `extension::world::creation::{CreateActorsPort, CreateActors,
ActorNode, ActorStateSeed, ActorLink}`. Native host code can use
`extension::{create_actors, RuntimeActor, RuntimeLink, ActorCreationLimits}`.
Both reach the same checked body and relation constructors.

The loaded `scripted_actor_creation` fixture composes independent behavior on a
sandbag and a giant for controlled and autonomous creators. It checks actual
births, state initialization, native projectiles, refusal without partial births
or counter spending, limb identity reservation, and GGRS replay. Interactive
props, cross-object event routing and general retirement results remain open.

## Measured cost

One cached development run on 2026-11-18 measured 78–156 microseconds per
backend call and 101–209 microseconds per whole host invocation. These are
per-entry averages over 32–396 calls under GGRS. They include ABI conversion,
library environments and host state work. They exclude native reducers, physics
and rendering.

A direct native batch with a sandbag and giant took 2,835 microseconds in the
same fixture. This includes native preparation and construction of both roots
and the giant's limbs, without script conversion or state seeds. It is one
local development sample, not a release or device budget. The factory library
edit test separately checks the prebuilt-host loop on existing bodies.
