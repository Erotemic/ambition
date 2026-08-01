# Adding a capability (a custom mechanic)

Operational recipe. `crates/ambition_pulse` is the **worked example** — a
shockwave mechanic that does everything below and nothing else. Read it beside
this page; it is deliberately small.

## What a capability is

A crate that contributes behaviour to a game, plus up to four things the engine
knows how to receive:

```text
behaviour            your systems and components
+ authored schema    content a designer writes, validated by the compiler
+ semantic action    a verb a player can be given
+ rollback state     what a rewind must restore
+ causal facts       why it did what it did
```

None of these requires editing a central enum, an engine crate, or the actor
crate. If you find yourself doing that, the seam is missing — say so rather than
working around it.

## The integration, in full

```rust
impl GameModule for MyGame {
    fn define(&self, module: &mut ModuleDraft) {
        module.experience("my_game").launcher_route("home").gameplay_route("my_game/play");

        module
            .capability(my_mechanic::MyPlugin)                 // behaviour
            .actions(&[my_mechanic::MY_ACTION])                // the verb
            .requires_rollback(my_mechanic::REQUIRED_ROLLBACK);// what must rewind
    }
}
```

That is the whole wiring. The composition installs each one and **refuses** if
two capabilities claim the same action id, or if declared rollback state was
never registered.

## The rule that runs through all of it

> **A capability OFFERS. The composition INSTALLS. A conflict refuses.**

It holds for content schemas, semantic actions and rollback state alike, and it
is why a capability's dependency closure stays small: the registries belong to
whoever is composing, so a mechanic never has to link the thing that owns them.

⚠ the one that catches people: **do not register your own rollback state.** The
registration trait lives in `ambition_runtime`, and reaching for it drags the
whole simulation into a mechanic that uses none of it. `ambition_pulse` linked
133 crates that way and links 7 now.

## 1. Behaviour

Ordinary Bevy. Define your own components rather than borrowing the actor
crate's — `ambition_pulse` has `PulseBody` and `PulseAffected` instead of using
`BodyKinematics`, and that is what keeps `ambition_actors` out of its manifest.
A composition adapts its bodies to what you describe.

## 2. An authored schema

Implement `ContentSchemaHandler` and return a `SchemaRegistration`. See
[`validating-a-content-pack.md`](validating-a-content-pack.md) for the handler
rules — `deny_unknown_fields`, a semantic canonical form, and lowering only when
the facet is clean.

A game installs it beside the engine's:

```rust
let mut registry = ambition::content::engine_schemas();
registry.register(my_mechanic::my_schema())?;
```

## 3. A semantic action

```rust
pub const MY_ACTION: SemanticActionDef = SemanticActionDef {
    id: SemanticActionId("grapple"),
    capability: MY_CAPABILITY,
    kind: ActionControlKind::Button,
    contexts: &[GAMEPLAY_CONTEXT],
    doc: "Fire the grapple",
};
```

⚠ **it can be declared and queried; it cannot yet carry a device binding of its
own.** `InputMap` is still keyed by the engine's closed `SandboxAction`, so a
consumer fires your mechanic by writing your own request message — which is also
how a scripted sequence or an AI would. The migration that closes this is in
`docs/planning/authoring-loop-program-2026-07-31.md`; do not invent a private
binding path around it.

## 4. Rollback state

Declare it; do not register it:

```rust
pub const ROLLBACK_STATE: &str = "grapple.cooldown";

pub const REQUIRED_ROLLBACK: &[RequiredRollbackState] = &[RequiredRollbackState {
    owner: MY_CAPABILITY,
    name: ROLLBACK_STATE,
    why: "a cooldown that is not rewound lets the action fire twice from one charge",
}];
```

The `why` is not decoration. A host that hits the refusal needs to know whether
it is looking at a desync or an optional extra, and only you know.

A composition installs it in one line:

```rust
app.rollback_component_clone_probed::<GrappleCooldown>(
    my_mechanic::MY_CAPABILITY, my_mechanic::ROLLBACK_STATE,
    |c| u64::from(c.remaining_ticks));
```

## 5. Causal facts

```rust
log.record(
    CausalFact::new(domains::MOVEMENT, 0, FactDetail::new("grapple_fired", "fired"))
        .about(SubjectKey::Seat(seat))
        .from_content(format!("my_game:grapple_profile/{}", profile.name))
        .field("length", length),
);
```

Leave the tick `0` — the host stamps it. Publish **refusals** too: *"I pressed it
and nothing happened"* is the question people bring to an inspector. See
[`explaining-a-tick.md`](explaining-a-tick.md).

## Testing it

`cargo test -p my_mechanic` should cover the mechanic. Add one integration test
that mounts it through the facade (`ambition` as a **dev**-dependency, so the
capability's own closure is unaffected) — `ambition_pulse/tests/
composed_through_the_sdk.rs` is the template.

**Headlessly, against the real sim**: `SandboxSim::new_with_options(..).step(..)`
builds the actual app with rendering, audio and windowing stripped and the
systems intact. The doctrine — drive the real sim, assert invariants rather than
tuned values, treat replay tests as canaries not cages — is
[`../planning/engine/headless-verification.md`](../planning/engine/headless-verification.md).

⚠ **a capability's own tests do not need any of that.** `ambition_pulse` builds a
bare `App`, adds its plugin, and steps it; a mechanic that needed the whole
sandbox to be testable would be telling you its seams are wrong.

## Checking your closure

The measurement that tells you whether you built a capability or a plugin:

```bash
cargo tree -p my_mechanic --edges normal | grep -c 'ambition_actors\|ambition_runtime'
```

Zero is the target. Non-zero is not automatically wrong — but it should be a
decision you can defend, not a surprise.
