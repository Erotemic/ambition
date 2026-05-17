# Mechanics expressibility checklist

This is the compact current checklist for reusable backend expressibility. It intentionally avoids exhaustive wishlist prose; use `docs/brainstorms/` for idea expansion and `docs/archive/historical-roadmaps/` for old snapshots.

Legend:

- `[x]` implemented or meaningfully expressible in current code/tests.
- `[~]` scaffolded but not a complete polished mechanic.
- `[ ]` not yet expressible as reusable backend.

## Current reusable primitives

- [x] Kinematic player controller with coyote/buffered jump, dash, double dash, air jump, wall cling/jump/climb, fast fall, glide, fly/debug mode.
- [x] Blink/teleport targeting and safety handling.
- [x] Directional slash intents, including upward slash and downward slash / pogo behavior.
- [x] Projectile backend with Fireball and Hadouken-style motion-input upgrade.
- [x] Shield/parry state and bubble-shield presentation.
- [x] Body modes and collision-safe body-shape checks for crouch/crawl/slide/morph-ball-style traversal.
- [x] Resource meters for dash/projectile-style gating.
- [x] Actor/faction/damage/interactable/breakable vocabulary.
- [x] Boss-pattern and encounter vocabulary.
- [x] Trace/replay/debug hooks for validating movement/combat behavior.

## Partially scaffolded

- [~] Ledge grab / mantle: backend and sandbox behavior exist, but animation/polish and broad validation are still evolving.
- [~] Moving platforms: data and systems exist; carry-velocity and edge cases need more validation.
- [~] Body-mode traversal: backend exists; more authored rooms and polish needed.
- [~] Dialogue/commerce hooks: architecture seed exists; content pipeline is not final.
- [~] Avian2D secondary physics: intended for debris/props, not the primary player controller.

## Missing high-value primitives

- [ ] Grapple/tether constraint backend.
- [ ] Generic ray/shape-cast targeting API beyond current specialized uses.
- [ ] Parametric/curve movement backend and preview renderer.
- [ ] Vector/scalar field sampling for mathematical movement/world rules.
- [ ] Per-entity/local-clock gameplay backend beyond current time-domain design docs.
- [ ] Deterministic randomness/probability backend for procedural systems.
- [ ] Robust moving-platform carry semantics.

## Validation anchors

Use focused tests where possible:

```bash
cargo test -p ambition_engine combat::
cargo test -p ambition_engine projectile::
cargo test -p ambition_engine movement::
cargo test -p ambition_engine --test body_shape_fits_at
cargo test -p ambition_sandbox scripted_gameplay
```

Run broader tests only after focused validation is green.
