# Public SDK 1.0

**State:** OPEN successor program.

## Goal

Design the engine surface a game developer should actually want to use.

The public SDK should express game concepts and supported extension points, not
require consumers to understand Ambition's internal crate history. Internal
architecture remains free to change aggressively until the semantic surface has
proved itself across real consumers.

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
- persistence/audio/network host services where installed;
- diagnostics, preparation errors and content provenance.

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

## Acceptance

A competent Rust/Bevy developer should be able to create a small game with a
world, character, movement/combat capability, participant input and room
transition by reading public docs/examples rather than Ambition migration plans
or importing internal implementation crates.
