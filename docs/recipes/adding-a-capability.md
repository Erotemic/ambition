# Adding a capability (a custom mechanic)

`examples/capability_demo` is a complete worked example of an external
shockwave capability. Read it beside this recipe.

The example intentionally lives outside the engine workspace. Its dependencies
must declare versions rather than relying on `workspace = true`. An external
consumer that depends on `ambition_platformer2d_runtime` must also account for
the workspace's forked `bevy_ggrs` `[patch.crates-io]` entry; Cargo patches do
not propagate to consumers. `scripts/run_tests.py` includes the example
explicitly because `cargo test --workspace` does not.

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

A standalone mechanic should not need to edit central input enums or actor
logic. If a needed extension point is absent, add it at the owning boundary
rather than coupling the mechanic to an unrelated implementation.

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

Composition installs the declared contributions and refuses conflicting
action IDs or missing required rollback registrations.

## Composition and ownership

The capability declares its contributions; the composing game installs them
and resolves conflicts. Registries for content, semantic actions and rollback
belong to composition, keeping the mechanic's dependency closure small.

Declare required rollback state in the mechanic, but register the actual state
with the composing module. The registration interface lives in
`ambition_platformer2d_runtime`; importing it into an otherwise independent
mechanic unnecessarily pulls in the simulation runtime.

## 1. Behaviour

Ordinary Bevy systems — registered into the schedule **the host declares
authoritative**, never into bare `Update`:

```rust
use ambition_platformer2d_shared_tangle::schedule::{Platformer2dSimulationPhase, SimScheduleExt};

impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();          // the HOST answers; this seals it
        app.add_systems(
            sim,
            (tick_my_cooldowns, apply_my_effect)
                .chain()
                .in_set(Platformer2dSimulationPhase::GameplayEffects),   // one explicit phase
        );
    }
}
```

Use the host-selected simulation schedule instead of registering gameplay
systems directly in `Update`, `FixedUpdate`, or a GGRS schedule. Otherwise
cooldowns can follow render frames, and rollback can restore state without
replaying its producing systems. A bare `App` may not expose the mismatch
because `sim_schedule()` defaults to `Update` there.

Define your own components rather than borrowing the actor crate's —
`capability_demo` has `PulseBody` and `PulseAffected` instead of using
`BodyKinematics`, and that is what keeps `ambition_platformer2d_actor_monolith` out of its manifest.
A composition adapts its bodies to what you describe.

## 2. An authored schema

Implement `ContentSchemaHandler` and return a `SchemaRegistration`. See
[`validating-a-content-pack.md`](validating-a-content-pack.md) for the handler
rules — `deny_unknown_fields`, a semantic canonical form, and lowering only when
the facet is clean.

A game installs it beside the engine's:

```rust
let mut registry = ambition_platformer2d::content::engine_schemas();
registry.register(my_mechanic::my_schema())?;
```

The lowered content must reach the running capability. For example, a plugin
that only calls `init_resource::<Profiles>()` may use built-in defaults despite
the compiler accepting authored values.

Take the LOWERED artifact at mount time:

```rust
module.capability(my_mechanic::MyPlugin::from_prepared(&pack)?);
```

Consume the artifact returned by `FacetOutcome::lower` rather than parsing
authored input again. If required content was not prepared, return an error;
only use defaults when the composition explicitly requests them.

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

A provider binds physical keyboard keys to its semantic actions through
`ProviderBindings` (`ambition_input::semantic`). A press publishes
`SemanticActionPressed { id, participant }`, filtered by the action's contexts.
A scripted sequence or an AI can also fire your mechanic by writing your own
request message. Open input work (bindings source, remap UX) is in
[`../planning/engine/participant-action-system.md`](../planning/engine/participant-action-system.md);
do not invent a private binding path.

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

The MODULE supplies it, in the same declaration that requires it:

```rust
module
    .capability(my_mechanic::MyPlugin::default())
    .requires_rollback(my_mechanic::REQUIRED_ROLLBACK)
    .provides_rollback::<GrappleCooldown>(
        my_mechanic::MY_CAPABILITY,
        my_mechanic::ROLLBACK_STATE,
        |c| u64::from(c.remaining_ticks),
    );
```

⚠ **owner and name must MATCH the requirement** — a registration under another
owner satisfies nothing, which is what makes the two calls a contract rather
than two lists.

If your positive test inspects an `Err`, you are testing the refusal, not the
installation.

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

## Validation

Build the capability and exercise its mechanic in the intended game.
`examples/capability_demo/tests/composed_through_the_sdk.rs` shows how to
verify the composition boundary without making the mechanic depend on the
engine facade in production. Its tests use a bare Bevy `App` where sufficient.

Use `Platformer2dSimHarness` (`ambition_sim_harness`) when actual movement,
rollback or runtime scheduling is necessary to establish behavior. Add tests
selectively for important invariants or regressions; every capability does not
need parallel bare-app, host-app and replay suites.

## Checking your closure

The measurement that tells you whether you built a capability or a plugin:

```bash
cargo tree -p my_mechanic --edges normal | grep -c 'ambition_platformer2d_actor_monolith\|ambition_platformer2d_runtime'
```

Zero is the target. Non-zero is not automatically wrong — but it should be a
decision you can defend, not a surprise.
