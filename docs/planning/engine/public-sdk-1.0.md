# Public SDK 1.0

**State:** OPEN successor program.

> **⭐ THIS PROGRAM IS RATCHETED, AND THE PAGE DID NOT SAY SO** (measured
> `0a29e23fe`, 2026-09-02). `scripts/check_absence_contracts.py` runs FOUR
> independent module-allowlist contracts, one per consumer, each measuring what
> that consumer names through the `ambition_platformer2d` facade:
>
> | contract | consumer |
> |---|---|
> | `outlander-names-only-the-public-sdk` | `fixtures/external_consumer/` |
> | `minimal-game-names-only-the-public-sdk` | `fixtures/minimal_game/` |
> | `sim-harness-names-only-the-public-sdk` | `crates/ambition_sim_harness/` |
> | `capability-demo-names-only-the-public-sdk` | `examples/capability_demo/tests/` |
>
> ⭐ **All four report `0 of 0 baseline modules still named`** — four independent
> consumers, tests included, naming NOTHING outside the reviewed public surface.
> The mechanism is a frozen baseline that may only shrink, and the script says
> why in place: *"an allowlist entry is a compatibility commitment, not a ratchet
> escape hatch"*, so *"an empty allowlist converges monotonically toward the
> public SDK."*
>
> ⇒ **So this program's acceptance is already installed and already green, and a
> reader of this page could not tell.** That changes what "OPEN" means here: the
> question is no longer whether consumers can avoid internal crates — four
> demonstrably do — but whether the surface they are held to is the RIGHT one.
> ⛔ And the ratchet cannot answer that: it measures what is named, not whether
> what is named is worth naming. `outlander-does-not-hand-order-its-own-composition`
> is a fifth contract in the same file, and green, which is a different question
> again.

> **MEASURED 2026-09-03 — ADR 0031's OWN NUMBERS, RE-TAKEN, and they answer the
> question the block above leaves open.** That ADR's Context is the case for this
> program, and it is quantitative: *"`crates/ambition_platformer2d/src/lib.rs` is
> 114 lines. Fifty of them are `pub use`, and roughly forty are
> `pub use ambition_x as x`"*, under the heading **"the public API of this engine
> is currently the list of crates it happens to be built from … a namespace
> mirror"**.
>
> | ADR 0031 (Context) | at HEAD |
> |---|---|
> | lib.rs 114 lines | **998** |
> | 50 `pub use` | **159** |
> | ~40 `pub use ambition_x as x` | **51** |
>
> ⭐ **The remedy landed and the symptom grew, and both halves are true.** Of
> those 998 lines, **456 are doc comments** and 39 are `#[cfg]` gates; the module
> doc opens *"the supported API is organized by game concepts (`actor`,
> `character`, `participant`, `session`, `sim`, `world`…)"*. That is a curated,
> documented, feature-gated facade — not the namespace mirror 0031 described.
> ⛔ But the one number 0031 named as the DEFECT — crates re-exported under their
> own names — went from about forty to **fifty-one**. The concept organisation was
> added ALONGSIDE the mirror rather than in place of it.
>
> ⇒ **So the surface is now two surfaces**, and the ratchet above cannot see the
> difference: a consumer naming `ambition_platformer2d::actor` and one naming
> `ambition_platformer2d::encounter` (the crate, aliased) both pass, while only
> the first is the API this program is trying to build. ⇒ *"Whether the surface
> they are held to is the RIGHT one"* has a concrete first answer: it is the
> right one plus fifty-one crate aliases.
>
> ✔ **AND 0031'S SECOND COST WAS PAID IN FULL, which is the happier half.** That
> ADR also measured composition: *"`build_windowed_app` is ~65 lines a consumer
> must write in a specific order"* — asset source before `DefaultPlugins`, then
> `init_engine_states`, then `PlatformerEnginePlugins::fixed_tick()`, then
> `PlatformerHostPlugins`, then the shell, then `PlatformerAssetsPlugin`, each
> for a reason the consumer had to know. At HEAD that function is **9 lines**
> (`fixtures/external_consumer/src/lib.rs:523`) and holds no order at all:
> `PlatformerApp::windowed(…)`, optionally `.without_gpu()`, `.mount(…)`,
> `.build()`. The sequence moved inside the builder, where it belongs.
>
> ⇒ **So this ADR has one complaint decisively closed and one quietly worse.**
> Worth holding both: a reader who only sees the 9-line builder concludes the
> facade problem is solved, and a reader who only counts crate aliases concludes
> nothing was done. The composition leak is gone; the namespace mirror is not.
>
> ⚠ **This is the COMPATIBILITY question, not the linking one, and they have
> different answers.** A facade alias makes the crate graph part of the public
> API — 0031's actual concern. It does NOT decide what a consumer LINKS: the
> actor monolith reaches every `never_asked_for` crate on its own, so cutting a
> facade edge changes no footprint number. See
> [`capability-and-runtime-composition.md`](capability-and-runtime-composition.md),
> where I got that backwards and retracted it. Same edges, two unrelated
> consequences.

## Goal

Design the engine surface a game developer should actually want to use.

The public SDK should express game concepts and supported extension points, not
require consumers to understand Ambition's internal crate history. Internal
architecture remains free to change aggressively until the semantic surface has
proved itself across real consumers.

For the Godot-class 2D target, the SDK is the proof that engine capability is
actually reusable. A feature that only Ambition can reach through private crates
is not yet a competitive engine capability, even if the implementation exists.
The SDK does not need to wrap every Bevy API; it needs to make the supported
composition story obvious and stable.

See [`godot-class-2d-capability.md`](godot-class-2d-capability.md).

## Primary customers

1. **Ambition** — deepest integration and primary product driver.
2. the external consumer fixture — adversarial proof that supported composition
   works outside the workspace;
3. acceptance/secondary games such as Mary-O, Sanic, TwinTrack and Smash;
4. future first-class games that may grow out of those customers.

## Desired surface families

A consumer should be able to discover coherent APIs for:

- experience/game composition;
- capability selection;
- authored content/provider registration;
- characters and reusable body definitions;
- actions/input participants;
- worlds/rooms/spatial authoring backends;
- sessions, construction and transitions;
- simulation queries/events and headless stepping;
- multiplayer participant/control declarations;
- local views/presentation policies;
- semantic animation/VFX/audio/UI hooks where Ambition adds policy beyond Bevy;
- asset identity, readiness/preparation and provider registration;
- persistence/audio/network host services where installed;
- diagnostics, preparation errors and content provenance;
- supported project/target composition needed to build and package an external
  game.

That list is a product map, not a request to create one giant SDK crate.

## Method

Grow the API from real consumer friction:

1. attempt the feature through the supported facade from a real game/customer;
2. record the engine fact the consumer was forced to rediscover or the internal
   crate it had to reach into;
3. decide whether the gap is public semantic capability, provider policy or a
   customer-specific concern;
4. add the narrow supported seam;
5. migrate the consumer and remove the internal dependency/duplicate path that
   made the seam necessary.

Current receipt: `ambition_sim_harness` now reaches body, participant, session,
settings and engine concepts only through semantic facade modules, and the
capability-demo host tests do the same. Both have zero implementation-module
baseline in the existing consumer-module ratchet. The migration also deleted
three facade mirrors with no consumers (`interaction`, `sfx_bank`, and the raw
`renderer` module) and renamed the facade's `session_world` module to `session`.
The remaining crate-shaped mirrors are still open migration surface; this slice
does not claim they are public SDK.

No blind-agent ritual or source-text allowlist is required for every slice.
Consumer code, dependency closure, API docs and behavioral tests are the main
evidence.

## Phases

### A1 — publish the post-D73 character authoring path

Make the public story for `CharacterDefinition` / preparation / placement clear
and remove remaining recipes or facade names that teach the deleted archetype
model.

### A2 — world/LDtk authoring API

Expose backend-neutral world concepts while making the LDtk adapter/tooling an
excellent supported backend.

### A3 — participants and views

Publish semantic participant/control/view APIs as the multiplayer/multiview
program matures instead of exposing singleton player-camera internals.

### A4 — capability composition

Make optional capability selection and service requirements understandable from
the facade.

### A5 — diagnostics and inspection

A consumer should be able to inspect prepared content/capabilities, validation
failures and simulation facts without importing internal debug crates.

### A6 — documentation examples

Maintain small complete examples for the common paths. Examples must build
against the same public surface external games use.

### A7 — machine-readable discovery

The same public concepts exposed to Rust consumers should be discoverable by
agent-native tooling: capabilities, provider vocabulary, schemas, diagnostics and
example entry points should not exist only as prose or implementation knowledge.
This does not require a universal reflection framework; domain-owned descriptors
may compose into read-only discovery.

### A8 — external project build/package proof

Keep at least one clean external/minimal consumer that can configure capabilities,
prepare content, build, run representative tests and produce a target artifact
through supported tooling. This is the engine-product counterpart to facade
compile tests.

## Acceptance

A competent Rust/Bevy developer **or capable LLM agent** should be able to create
a small 2D game with a world, character, movement/combat capability, participant
input, presentation, assets and room transition by using public docs/discovery
rather than Ambition migration plans or internal implementation crates. The same
project should run headlessly for tests and have a supported noninteractive path
to a release artifact on at least one declared desktop target.
