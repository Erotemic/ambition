# Physical body contact rules

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

An empty sample can establish separation from previously observed bodies.
When `truncated` or `incomplete` is true, absence cannot establish separation
from an omitted or unresolved identity. A geometry refusal supplies its body
identity and native reason in `refusals`. `refusals_truncated` reports omitted
diagnostics. A recipient with refused geometry produces no sample. Retirement ends the recipient's stream; it does not run a callback
on a retired body. Facts are derived again from canonical simulation state on
replay. Script records use the normal canonical state store.

## Game budgets and responses

A game's `room_rules` content can set:

```ron
(body_contact_events: Some((max_touchers: 16, max_surfaces: 4)))
```

The defaults are 32 touching bodies and 8 surface points per toucher. Both
limits must be positive. Content reload replaces or removes the declaration.
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
`BodyContactPort` is version 2. Native and loaded modules use the same records,
read cut, and responses. See the
[loaded host fixture](../../game/ambition_app/tests/scripted_body_contacts.rs)
for status activation, bounded samples, retained references, native bounce,
empty samples, controlled bodies, NPCs, and replay.
