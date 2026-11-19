# Live object references

SDK API 0.4 supplies `ObjectRef` and the Rhai state kind `object`. A reference
names one occurrence in one gameplay session. It contains no native entity bits.
Actors, props and room roots use the same address and resolver. Session-lifetime
objects use a separate allocation namespace.

A reference has three fields:

| Field | Meaning |
| --- | --- |
| `session` | Exact session identity, as unsigned decimal text in Rhai. |
| `allocation_room` | The allocation room number, or `()` for session allocation. |
| `sequence` | Exact occurrence sequence, as unsigned decimal text in Rhai. |

The allocation room does not change during travel. Room retirement does not
rebind the address to an equal authored ID. Native resolution distinguishes an
unbound address, a foreign session, retirement and duplicate occurrence stamps.
Object facets do not change the resolution population. A missing mailbox,
body or interaction capability is a separate domain refusal.

## Keep a reference in state

Use an optional object field or a bounded sequence:

```rhai
#{key: "memory", fields: [
    #{tag: 1, name: "peer", kind: "opt object"},
    #{tag: 2, name: "children", kind: "seq 8 object"},
]}
```

A native creation reply supplies subjects with `key` and `object` fields:

```rhai
if this.trigger.stage == "applied" {
    this.state.memory.children = this.trigger.subjects.map(|subject| subject.object);
}
```

`key` is a semantic domain key. It can name content or an authored object.
`object` is an optional live occurrence reference. Actor, breakable and tracked
projectile creation supply it. Results that name content, such as inventory
definitions, can supply only a key. Do not convert a key into a live address by
assuming that the object is still in its original room.

Portable records use `Option<ObjectRef>` or a bounded `Vec<ObjectRef>`. A bare
object field needs a valid explicit initial value. Its default is unbound.
An optional field starts absent.

## Lifetime and native authority

Object references can occur inside nested records, options and sequences.
Their records must be transient. Schema validation rejects both checkpoint and
durable capture, including an empty sequence whose element kind is `object`.
The reference and its target do not survive a new session through the save road.
Use an authored locator for durable intent, then bind it to a live occurrence.

Transient state still participates in rollback. It can attach to a body, room
root or session. A session-attached record can retain an address across room
visits in that session. Reconstitution restores the occurrence stamps and
allocation counters with the owning lifetime.

A valid address does not prove that an operation is allowed. Native domains
still check liveness, room, range, capabilities and their own admission rules.
A stale reference is useful state: a script can retain it and handle a refused
request. It must not substitute another object with the same authored name.

The older `body` and `item` state kinds retain authored semantic names. They
are useful locators; they do not have the lifetime semantics of `object`.
[Body queries](body-queries.md) accept exact occurrence references as well as
explicit authored-name filters. General object discovery, behavior bindings,
directed calls and scoped event subscriptions remain under implementation.
