# Physical body and world contact rules

`body_contact` supplies native physical contact samples after movement and
before shared contact damage. It works for controlled and autonomous bodies.
A recipient binds the trigger selector through a held status or its custom
brain key. A repeated key binds the rule once.

A script can enable a rule with an existing `hold_status` request:

```rhai
this.requests.push(request("hold_status", #{ kind: "my_game.spring", held: true }));
```

For example, a `technique_use` entry can hold this status when its control is
pressed. The contact entry declares:

```rhai
#{ key: "spring", phase: "body_contact",
   trigger: #{ port: "body_contact", selector: "my_game.spring" },
   requests: ["contact_bounce"], limit: 1 }
```

It can bounce a touching body through the native movement owner:

```rhai
fn spring() {
    if this.trigger.touchers.len() == 0 { return; }
    this.requests.push(request("contact_bounce", #{ toucher: 0, speed: 300.0 }));
}
```

The index refers to this invocation's sample. It is not a durable reference.
The bounce follows the toucher's own resolved gravity. `pogo_cue: true` also
plays its native pogo cue. `hold_status` is available in `body_contact`, so a
one-use rule can release its own status there.

## Read cut and identities

Each bound recipient receives one sample per gameplay step, including a
sample with no touchers. A held room produces no sample. Disabled, dead,
out-of-play, and untouchable depth-plane bodies are excluded.

The trigger contains `observed_at`, optional live `room`, recipient `body_id`,
`driven`, physical `body`, optional gravity `down`, ordered `touchers`,
`truncated`, `incomplete`, bounded `refusals`, and `refusals_truncated`.
The same sample includes bounded native movement contacts in `world`.
All touchers share the recipient's live room and session. They use simulation
identity order. Each toucher contains:

- `id`: a semantic body identity. Keep it in a declared `body` or `opt body`
  record field when it must survive this invocation.
- `body`: physical centre, full envelope size, and velocity in that body's
  root room chart, in world units with +Y down.
- `driven`: whether a participant currently drives it. A game can use this
  fact for a rule that applies only to controlled bodies.
- `relation`: the shared combat relation, or absent when it cannot be read.
  Teams, factions, and grudges use the same owner as body queries.
- `surfaces`: native contact points, outward normals, and signed separation
  in the recipient's root room chart. Penetration has negative separation.
- `surfaces_truncated`: whether the surface budget omitted further points.

Physical material and aperture partitions supply the geometry. Rotated
bodies use their actual shape. Through an aperture, surface points and
normals return to the recipient's root chart. Do not derive a contact normal
by subtracting two body centres.

These are samples of current physical contact, including resting faces. They
do not report every crossing along a motion path. They do not prove accepted
damage, a shield result, or a projectile hit. Those facts belong to their
combat consumers.

The native collector indexes physical material pieces with the engine's spatial
broadphase. Each session and live room has a separate index. Portal pieces use
their actual projected bounds. Candidate order returns to simulation identity
order before exact contact checks. Refused material remains an incomplete read,
even when it has no usable spatial bounds.

An empty sample can establish separation from previously observed bodies.
When `truncated` or `incomplete` is true, absence cannot establish separation
from an omitted or unresolved identity. A geometry refusal supplies its body
identity and native reason in `refusals`. `refusals_truncated` reports omitted
diagnostics. A recipient with refused geometry produces no sample. Retirement ends the recipient's stream; it does not run a callback
on a retired body. Facts are derived again from canonical simulation state on
replay. Script records use the normal canonical state store.

## World geometry contacts

`world` contains the movement solver's contacts, in solver order. Repeated
contacts remain separate. It does not perform another overlap query.

Each contact supplies `role` (`support`, `head`, `side`, or `attachment`),
`point`, `normal`, `toi`, `surface_velocity`, `impact_speed`, and `involuntary`.
The solver assigns the role against the body's resolved frame. `toi` is local
to the attempted solver step; it is not a timestamp for the whole simulation
tick. The point and
surface outward normal use the body's root room chart. The normal points
from the surface toward the body. `impact_speed` is the body's closing speed
before collision resolution, in pixels per second. `surface_velocity` carries
the native surface displacement for this step. Do not treat it as a speed.

Exactly one source is present:

- `block`: the block material and `geometry` identity. `blink_tier` and
  `rebound_impulse` retain the parameters of those material kinds; they are
  absent for other kinds. `origin` is `placement`,
  `tile_layer`, `generator`, `delta`, or `anonymous`. `key` contains the
  placement ID, tile-layer name, or generator placement ID for the first three
  origins. `operation` contains the delta operation number for delta geometry.
  Other source fields are absent. `index` is the source's emission ordinal.
- `chain`: the current collision-world chain index and segment index. These
  indices apply to this sample's live room and collision generation. Do not
  store them as durable object references.

A display name does not identify a block. Two geometry sources can have the
same key; compare `origin`, `key`, `operation`, and `index` together, within
`room`. Anonymous geometry has no durable identity.

`world_available` means that this body published a movement step for exactly
`observed_at`. A missing or old step sets it to false and supplies an empty
list. A current step with no world contacts sets it to true. A movement refusal
appears in `world_refusal`; any returned contacts can then be partial.
`world_truncated` reports contacts omitted by the world-contact budget.
Neither an unavailable nor a partial sample proves separation.

For example, a reusable rule can select head contacts by authored placement:

```rhai
let sample = this.trigger;
if !sample.world_available || sample.world_refusal != () { return; }
for hit in sample.world {
    if hit.role != "head" || hit.block == () { continue; }
    let geometry = hit.block.geometry;
    if geometry.origin == "placement" && geometry.key == "my_block_iid" {
        // Submit a domain action and use its confirmed result.
    }
}
```

These facts do not grant authority to remove geometry or pay a reward.
Atomic world-object transitions and durable claims remain separate work.

## Game budgets and responses

A game's `room_rules` content can set:

```ron
(body_contact_events: Some((max_touchers: 16, max_surfaces: 4, max_world_contacts: 16)))
```

The defaults are 32 touching bodies, 8 surface points per toucher, and 32
world contacts. All limits must be positive. Content reload replaces or removes the declaration.
These limits bound returned facts, not all collection work. Dense overlaps,
large material pieces and refused geometry can retain many candidates. The
collector builds its derived index again on each step, including replay. See
[observer cost](../../dev/benchmark-candidates/scripted-contact-observer-scaling.md)
for a local sample and its limits.
Removing it restores the defaults. Returned facts and native response indices
use the same bounded selection.

`defeat_body` immediately retires the recipient before shared contact damage:

```rhai
this.requests.push(request("defeat_body", #{}));
```

It has no drops, score, or death sequence. Use a combat action when those
combat consequences are required. A request cannot defeat another body by
passing its identity to this port.

The portable contracts are in `ambition_platformer2d::extension::combat`.
`BodyContactPort` is version 3. Rebuild portable modules that declare version 2.
World geometry values are in `ambition_platformer2d::extension::world::geometry`. Native and loaded modules use the same records,
read cut, and responses. See the
[loaded host fixture](../../game/ambition_app/tests/scripted_body_contacts.rs)
for status activation, bounded samples, retained references, native bounce,
empty samples, controlled bodies, NPCs, world contacts that start native
actor creation, and replay.

The [local cost measurement](../../dev/benchmark-candidates/scripted-world-contact-reaction-cost.md)
separates loaded invocation cost from native physics and construction.

For a destructible prop, [contact consumption](contact-consumption.md) commits
its native state and inventory exchange together. A new request must use an
index from the current sample. The native movement stamp includes the tick,
room, and session. A step from another scope is unavailable, even at the same
tick.
