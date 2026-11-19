# Create native room props

`create_breakables` creates destructible props through the same native placement
constructor as authored breakables. A script library can define bridges,
barriers, contact rewards, and other prop factories. Native systems own health,
collision, stand collapse, respawn, and presentation state.

## Request and result

```rhai
fn bridge(key, at, width) {
    #{key: key, at: at, size: [width, 16.0], health: 2,
      collision: "one_way", trigger: "stand", look: "cracked_stone"}
}

this.requests.push(request("create_breakables", #{
    reply: #{request: 1, selector: "my_game::bridges/result"},
    objects: [bridge("left", [200.0, 180.0], 96.0),
              bridge("right", [400.0, 180.0], 144.0)]
}));
```

Declare `create_breakables` in the entry's `requests`. The request is installed
in `actor_think`, `technique_execution`, `wielded_use`, `boss_conduct`,
`module_entity_tick`, `body_contact`, and `hit_reaction`. Room story entries
use the same `actor_think` adapter. The source needs a live session, a live room,
a semantic identity, and a native identity counter.

The action domain is `world.create_breakables`. A normal
[`action_result`](action-results.md) entry receives the terminal outcome at its
next eligible read cut. Stage `applied`, code `breakables_created`, supplies the
actual prop identities in request order. Stage `rejected`, code
`breakable_creation_refused`, supplies the reason and no subjects. An invalid
batch creates no props and spends no identity counter. Exact retries use the
existing action ticket and do not create another batch.

Creation uses the common native planner. Actor and prop requests share the
source's identity counter. Within each request phase, the actor adapter runs
before the breakable adapter. Scripts must retain returned identities; they
must not predict counter values or infer identity from a display label.
Native identities are qualified by live room. Independent room creators can
return equal text IDs in different rooms. Keep the creation scope with a
reference; a text ID alone is not a global or durable object address.

## Prop data

Each object has these fields:

| Field | Meaning and default |
| --- | --- |
| `key` | Unique, nonempty batch key, at most 128 UTF-8 bytes. |
| `label` | Optional display label. Defaults to `key`. |
| `at` | Centre in the source room's chart, in world units, with +Y down. |
| `size` | Full width and height. Both must be finite and positive. |
| `health` | Positive signed 32-bit health. Defaults to 1. Fresh props are intact at full health. |
| `collision` | `solid` (default), `one_way`, or `none`. The native collision owner supplies the surface. |
| `trigger` | `hit` (default), `stand`, or `either`. Standing bodies use the same native collapse rule for every controller. |
| `respawn_after` | Optional positive, finite delay in seconds after consumption. Omitted means no timed respawn. |
| `pogo_refresh` | Native pogo refresh policy. Defaults to false. |
| `look` | Optional prop sheet identity, at most 128 UTF-8 bytes. Missing or unavailable art uses the ordinary breakable fallback. |

Props expose normal native damage volumes and collision geometry. Creation
completes before its applied result. Movement and collision observations use
their normal publication phases; creation does not rerun a solver that already
finished. [`body_contact`](body-contacts.md) returns the actual geometry identity
on later movement. [`consume_contact`](contact-consumption.md) can then commit
a prop claim and inventory terms together.

Both authored and runtime breakables use one live visual discovery path. The
native feature carries its look. Presentation resolves that look as structure
art that fills the box, and follows native visible and broken state. Gameplay
does not require the art to load. Live feature art also demands its registered
sheet, even if that kind is scenery in a distant room.

## Bounds and lifetime

`BreakableCreationLimits` governs the request in the source's rules scope.
Defaults allow 32 objects per batch, 512 live breakables per room, and 128 bytes
per label. All native breakables in the same session and room count against the
population limit, including broken props that can still respawn. A zero bound
disables that allowance. Native calls receive the same checks.

Games can register `BreakableCreationLimits` through `DeclareRulesExt` and
`RulesScope`. The engine registers the `breakable_creation_rules` content-pack
schema for authored rule sources. Ambition loads
[breakable_creation_rules.ron](../../game/ambition_content/assets/data/breakable_creation_rules.ron)
for its untagged rooms and publishes valid edits at its content-reload boundary.
Without a matching rule, the adapter uses engine defaults.

The live room owns each prop. Creator removal does not remove its props.
Room retirement removes them. Re-entering a room does not reconstruct runtime
props from saved references. The new room root starts fresh script records and
a separate creation namespace. Timed respawn works only while their room remains
live. This contract does not expose durable runtime blueprints, custody,
general activation, or arbitrary component mutation. A durable runtime object
needs a native reconstruction and save contract; an identity string is not one.

Portable Rust uses `extension::world::breakables::{CreateBreakablesPort,
CreateBreakables, BreakableNode, BreakableCollision, BreakableTrigger}`.
Native host code uses `breakables::creation::{create_breakables,
RuntimeBreakable, BreakableCreationLimits}`. Both reach the same checked
placement constructor.

## Evidence and local cost

The loaded `scripted_breakable_creation` fixture composes two room props with an
actor, checks exact retries and refusal, stands a body on the resulting bridge,
and consumes native contacts for controlled and autonomous creators. It checks
actual wallets, collision, art facts, timed respawn, GGRS, authored limits,
room departure, independent room creators, and edits to a captured library in the same host.

Run it with:

```bash
cargo test -p ambition_app --test app_it -- scripted_breakable_creation --nocapture
```

One local development run on 2026-11-18 measured 80–268 microseconds per backend
call and 99–311 microseconds per complete host invocation. These are per-entry
averages over 12–328 calls under GGRS. They include library environments,
ABI conversion, and typed state. They exclude native reducers, physics, and
rendering. The parallel test run is not a release or device performance budget.
