
# Mechanics expressibility checklist

Use this as a compact current-status map. It intentionally avoids long wishlist prose; living idea expansion belongs in `docs/brainstorms/`.

Legend: `[x]` expressible now, `[~]` scaffolded but incomplete, `[ ]` not yet reusable backend.

## Movement

- [x] Kinematic controller, coyote/buffered jump, air jump, dash charges, wall cling/jump/climb, fast fall.
- [x] Blink/teleport targeting and safety checks.
- [x] Glide and fly/debug mode.
- [~] Ledge grab / mantle: behavior exists, polish and animation coverage remain incomplete.
- [~] Moving platforms: implemented path exists, but carry semantics need more validation.
- [ ] Grapple/tether constraints.

## Combat and interactions

- [x] Directional slash intents, including upward slash and downward slash / pogo.
- [x] Projectile backend with Fireball and Hadouken-style motion-input upgrade.
- [x] Shield/parry state and bubble-shield presentation.
- [x] Actor/faction/damage/interactable/breakable vocabulary.
- [~] Dialogue/commerce hooks: architectural seed exists, content pipeline is not final.

## Body and traversal

- [x] Crouch/crawl/slide/body-mode vocabulary.
- [x] Collision-safe shape checks for compact traversal and morph-ball-style modes.
- [~] Authored traversal rooms for body-mode mechanics need expansion.
- [ ] Spring-ball/bomb/spider-ball-style specialized traversal.

## Simulation and validation

- [x] Trace/replay/debug hooks for movement/combat validation.
- [~] Avian2D secondary physics for debris/props; not the primary player controller.
- [~] Time-domain vocabulary is documented; full per-entity proper-time gameplay is future work.
- [ ] Deterministic procedural/randomness backend for generated systems.

## Validation anchors

```bash
cargo test -p ambition_engine combat
cargo test -p ambition_engine projectile
cargo test -p ambition_engine movement
cargo test -p ambition_sandbox scripted_gameplay
```

Prefer exact tests named by concepts or benchmark candidates when available.
