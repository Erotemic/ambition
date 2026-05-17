---
id: sim-presentation-seam
aliases:
  - event seam
  - message seam
  - presentation adapter
  - gameplay effects
implemented_by:
  - crates/ambition_engine/src
  - crates/ambition_sandbox/src/presentation.rs
  - crates/ambition_sandbox/tests/scripted_gameplay.rs
related_adrs:
  - docs/adr/0012-sim-presentation-split-and-events-refactor.md
related_docs:
  - docs/archive/historical-roadmaps/events-refactor-plan.md
  - docs/systems/gameplay-effects.md
related_memory:
  - dev/benchmark-candidates/overlay-stale-feature-events-api-question-2026-05-12.md
last_verified: 2026-05-17
---

# Sim / presentation seam

## Definition

The sim/presentation seam separates reusable gameplay state and messages from sandbox-only rendering, audio playback, HUD layout, particles, debug windows, and temporary visual experiments.

## Core invariants

- Engine-side gameplay primitives should not depend on sandbox presentation resources.
- Presentation-only systems may consume sim messages, but sim systems should not need visual/audio adapters.
- Typed messages/events are part of the API surface; stale overlays must not revert them to older event shapes.
- Debug and HUD features are adapters, not gameplay authority.

## Edit protocol

1. Decide whether the change is simulation authority or presentation response.
2. If adding a gameplay event/message, update both producer and presentation consumers.
3. Preserve headless compatibility for sim-side changes.
4. Add tests at the seam when behavior crosses from sim into presentation messages.

## Validation

```bash
cargo test -p ambition_sandbox --test scripted_gameplay
cargo run -p ambition_sandbox --bin headless
cargo test -p ambition_sandbox --lib
```
