# Structured script state

Scripts can compose bounded values in host-owned records. The same schema works
in Rhai and portable Rust. The host owns rollback, checksums and save policy.
A VM table, closure or library environment is not persistent state.

## State lifetime

`attachment: "body"` stores a record on the invocation's native owner. A body
keeps its records when it travels. A `story_tick` invocation uses its live room
root as the owner. Each new room instance gets a new root and initial records,
including when a visit returns to the same room definition. A crossing that
joins an existing live room keeps that room's records. Room retirement removes
its records and pending observer state together.

Use `attachment: "session"` for state that must span room visits. Session state
retains its values until session retirement. Durable save policy is a separate
choice and needs an installed save road. Room-root records have no durable save
road. All future-affecting records rewind with their native owner.

The native host can separate an invocation's subject from its private record
owner with `InvocationScope { subject, state }`. Observations, time pace and
domain actions use the subject. Body-attached records, action mailboxes, query
reply history and delivery receipts use the state owner. Two instances can use
the same entry and schema on one subject without sharing these records. Both
entities must remain live. See [object behaviors](object-behaviors.md) for script-authored attachment and
directed method calls.


## Rhai values

A library can return a reusable kind declaration:

```rhai
fn objective_kind() {
    #{ record: [
        #{ tag: 1, name: "title", kind: "text 64" },
        #{ tag: 2, name: "progress", kind: "u32" },
        #{ tag: 3, name: "target", kind: "opt body" },
    ] }
}

fn schema(key) {
    let objective = objective_kind();
    #{ key: key, attachment: "session", save: "durable", fields: [
        #{ tag: 1, name: "active", kind: #{ opt: objective } },
        #{ tag: 2, name: "queue", kind: #{ seq: 8, of: objective } },
    ] }
}
```

Use this schema in the module's `declare()` result. The composition must install
the selected save road. A nested value uses its outer record's scope and policy;
it has no separate identity or lifetime. Each body can have its own instances,
and a session record can share state across bodies.

An entry can write ordinary values:

```rhai
this.state.objectives.queue.push(#{
    title: "Guard the gate", progress: 0, target: ()
});
```

The final segment of the schema key names the state table. All returned record
fields must be present. Unknown fields are errors, including inside sequences.
A wrong field name cannot silently create unused state. Options use `()` for
absence. Lists keep their order.

Text uses `text <maximum bytes>`. It accepts empty UTF-8 text. The bound counts
bytes, not characters. Text is distinct from a body or item reference: the host
does not resolve text as an identity. Authored locators use `body`, `item`
or their optional forms. Nested locators take part in the existing whereabouts
observation. Live occurrence references use `object` and transient records.

The existing kinds remain available: `bool`, `u32`, `i32`, `u64`, `f32`, `vec2`,
`body`, `item`, `object`, `opt <kind>` and `seq <bound> <kind>`. Use table declarations to
compose records with options and sequences. Each kind can have at most 32 levels
of nesting. The Rhai engine's string, list, map and work limits also apply.

## Initial values

A field takes its kind's default unless `initial` supplies a value. Numbers
start at zero, text and sequences start empty, and options start absent.
A record starts with its declared child defaults. A bare reference needs an
explicit valid identity; use an option when the reference can be absent.

Composite initial values can include options, sequences and records. A record
initializer can omit children to use their declared defaults. Unknown child
names are errors. This default fill applies to declarations only; a running
entry must return all fields. The SDK validates declarations inside an empty
sequence or absent option too.

## Portable Rust

Use `record_value!` for a reusable value and `record!` for a stored schema:

```rust
use ambition_extension_sdk::{record, record_value, SchemaKey};

record_value! {
    pub struct Objective;
    1 title: String [max 64],
    2 progress: u32,
}

record! {
    pub struct Objectives = SchemaKey::new("example", "objectives", 1),
        per session, kept durable;
    1 active: Option<Objective>,
    2 queue: Vec<Objective> [max 8],
}
```

The generated types implement `RecordField`. They share one field codec.
`Objectives::load` and `store` use the invocation's declared record. A value
type also supplies `fields`, `to_record_value` and `from_record_value`.
Declare a bound for each sequence and text field. Wrap an element with its own
bounded fields in a value type when the outer sequence has a separate bound.

Low-level authors can use `FieldKind::Record`, `FieldKind::Text`, `Value::Record`
and `Value::Text`. A `Record` stores values in declaration order. The host checks
the complete staged record before it commits state or requests.

## Schema edits and durable state

Field tags identify fields within each record. Names are script access names
and diagnostics. Names do not enter the shape digest. The field order controls
the value layout. Reload and save adoption match tags recursively, including
records inside options and sequences.

A compatible edit can rename, reorder, add or remove fields. Sequence and text
bounds can grow. Retained fields cannot change kind or shrink their bounds.
The host refuses such a generation and keeps the current code and state. This
also protects saved values that are not live during the edit.

New children in an existing nested record take their new child defaults. A new
whole field takes its field initializer. A changed default does not reset a
compatible live value. An outer initializer does not replace retained children.
Use a new schema key for a different state contract.

The portable wire stores text as length-prefixed UTF-8. Nested record values
follow their schema order. Canonical checksums normalize the sign of float zero
at every depth; wire values keep the exact float bits. SDK API 0.2 adds the new
kind codes. Existing scalar encodings remain unchanged.

The loaded `scripted_libraries` fixture checks independent nested state for two
modules on controlled and autonomous bodies. It uses native inventory and
projectile outcomes, GGRS replay, live schema edits and incompatible edit
refusal. Host tests check durable save adoption after nested schema edits.
Arbitrary maps, enums and general object composition remain separate SDK work.

SDK API 0.3 adds required reply-entry declarations. Module descriptors carry
these paths through the portable ABI. See [action results](action-results.md#validate-reply-wiring).

Live addresses use the [object reference](object-references.md) kind. It supports
nested records, options and bounded sequences. Schemas that contain it must use
transient save policy. Authored `body` and `item` names remain separate locators.
