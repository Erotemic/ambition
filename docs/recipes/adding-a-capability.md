# Adding a capability (a custom mechanic)

Operational recipe. `examples/capability_demo` is the **worked example** — a
shockwave mechanic that does everything below and nothing else. Read it beside
this page; it is deliberately small.

⚠ **it lives OUTSIDE the engine's workspace, on purpose** (moved there
2026-08-01). A capability that proves an outside author can write one has to be
built the way an outsider builds it: its own lock, its own `[workspace]`, no
feature unification from the engine. Two things broke the moment it moved, and
both are costs an outside author really pays:

* **`workspace = true` dependencies do not exist out there.** `ron` and `serde`
  are declared with versions, like anyone else's manifest.
* ⛔ **`[patch.crates-io]` is not inherited.** The engine pins a forked
  `bevy_ggrs` rev; outside, `ambition_platformer2d_runtime` resolved the released crate and
  failed on a missing `GgrsFrameTiming`. Any consumer that reaches
  `ambition_platformer2d_runtime` must repeat the patch — invisible while the crate lived
  inside and inherited it for free.

`scripts/run_tests.py` runs it explicitly, because leaving the workspace drops a
crate from `cargo test --workspace` silently.

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
registration trait lives in `ambition_platformer2d_runtime`, and reaching for it drags the
whole simulation into a mechanic that uses none of it. `capability_demo` linked
133 crates that way and links 8 now (the eighth is
`ambition_platformer2d_shared_tangle`, for the schedule seam in §1).

## 1. Behaviour

Ordinary Bevy systems — registered into the schedule **the host declares
authoritative**, never into bare `Update`:

```rust
use ambition_platformer2d_shared_tangle::schedule::{Platformer2dSimulationPhaseMonolith, SimScheduleExt};

impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();          // the HOST answers; this seals it
        app.add_systems(
            sim,
            (tick_my_cooldowns, apply_my_effect)
                .chain()
                .in_set(Platformer2dSimulationPhaseMonolith::GameplayEffects),   // one explicit phase
        );
    }
}
```

⛔ **`Update` is the mistake this recipe exists to prevent, and the sentinel
made it anyway** (found by review, 2026-08-01). Two failures, neither visible in
a bare-`App` test because `sim_schedule()` DEFAULTS to `Update`:

* a **fixed-tick** host ages your cooldowns once per rendered frame, so your
  timing follows the frame rate;
* a **rollback** host replays the sim schedule, so your systems never
  resimulate — a rewind restores your state without re-running what produced it.
  Snapshotting does not save you here; the state comes back and the behaviour
  does not.

⚠ **do not name GGRS or `FixedUpdate` yourself.** `sim_schedule()` is the whole
interface; the host has already chosen.

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

⛔ **and then it must reach the RUNNING capability, which is a separate step and
the one that gets forgotten.** `capability_demo` registered its schema, compiled
and lowered packs correctly, and its plugin still called
`init_resource::<PulseProfiles>()` — the built-in defaults. A game could author a
radius, watch the compiler accept it, mount the capability, and pulse at the
default radius forever. **A compiler that validates content the runtime ignores
is worse than no compiler**, because it certifies the wrong thing.

Take the LOWERED artifact at mount time:

```rust
module.capability(my_mechanic::MyPlugin::from_prepared(&pack)?);
```

⚠ consume the artifact `FacetOutcome::lower` produced — never re-read the
authored file. A second parse is a second authority over the same bytes.
⚠ and make "the pack prepared nothing" a REFUSAL. Falling back to defaults
silently is the bug above wearing a different hat; a composition that meant the
defaults can say so by mounting `MyPlugin::default()`.

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
own.** `InputMap` is still keyed by the engine's closed `Platformer2dInputActionMonolith`, so a
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

⛔ **the `requires` half shipped without the `provides` half**, so for a while a
module could declare what must rewind and had no supported way to supply it: a
rollback game mounting such a capability could not be composed at all. The
capability's own acceptance test papered over that by asserting on a REJECTED
app and reading the resources its failed installation had already written. If
your positive test is inspecting an `Err`, you are testing the refusal.

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
that mounts it through the facade (`ambition_platformer2d` as a **dev**-dependency, so the
capability's own closure is unaffected) — `capability_demo/tests/
composed_through_the_sdk.rs` is the template.

**Headlessly, against the real sim**: `Platformer2dSimHarness::new_with_options(..).step(..)`
builds the actual app with rendering, audio and windowing stripped and the
systems intact. The doctrine — drive the real sim, assert invariants rather than
tuned values, treat replay tests as canaries not cages — is
[`../planning/engine/headless-verification.md`](../planning/engine/headless-verification.md).

⚠ **a capability's own tests do not need any of that.** `capability_demo` builds a
bare `App`, adds its plugin, and steps it; a mechanic that needed the whole
sandbox to be testable would be telling you its seams are wrong.

## Checking your closure

The measurement that tells you whether you built a capability or a plugin:

```bash
cargo tree -p my_mechanic --edges normal | grep -c 'ambition_platformer2d_actor_monolith\|ambition_platformer2d_runtime'
```

Zero is the target. Non-zero is not automatically wrong — but it should be a
decision you can defend, not a surprise.
