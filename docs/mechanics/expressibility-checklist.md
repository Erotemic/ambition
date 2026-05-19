# Mechanics expressibility checklist

Use this as a compact current-status map. It intentionally avoids the old 900-line wishlist, but it preserves enough future backend primitives that agents can find useful multi-hour tasks without re-reading deleted docs.

Legend: `[x]` expressible now, `[~]` scaffolded but incomplete, `[ ]` not yet reusable backend.

## Movement

- [x] Kinematic controller, coyote/buffered jump, air jump, dash charges, wall cling/jump/climb, fast fall.
- [x] Blink/teleport targeting and safety checks.
- [x] Glide and fly/debug mode.
- [~] Ledge grab / mantle: behavior exists, polish and animation coverage remain incomplete.
- [~] Moving platforms: implemented path exists, but carry semantics need more validation.
- [~] Climbable / ladder body mode: available, but ladder-top passthrough and jump/dash-off polish remain.
- [ ] Grapple/tether constraints.
- [ ] Gravity columns / rotated gravity policy.
- [ ] Parametric curve riding or spline/rail movement.

## Combat and interactions

- [x] Directional slash intents, including upward slash and downward slash / pogo.
- [x] Projectile backend with Fireball, charged Fireball, Hadouken, and HadoukenSuper motion-input upgrades.
- [x] Shield/parry state and bubble-shield presentation.
- [x] Actor/faction/damage/interactable/breakable vocabulary.
- [~] Dialogue/commerce hooks: architectural seed exists, content pipeline is not final.
- [~] Boss profiles and phase machines: playable, but movement and music binding need more data-driven wiring.
- [ ] Bubble shield dodge/roll policy.
- [ ] Falling-sand / fluid toy-room simulation.

## Body and traversal

- [x] Crouch/crawl/slide/body-mode vocabulary.
- [x] Collision-safe shape checks for compact traversal and morph-ball-style modes.
- [~] Authored traversal rooms for body-mode mechanics need expansion.
- [ ] Spring-ball/bomb/spider-ball-style specialized traversal.
- [ ] Ladder-through-solid or authored ladder-top passthrough rule.
- [ ] Swim/sink/iron-boots variants unified with body-mode and volume policy.

## World and authoring primitives

- [x] LDtk-authored rooms, loading zones, IntGrid lowering, and hot reload.
- [x] One-way platforms, damage volumes, climbable regions, and runtime encounter lock walls.
- [~] Stitched / side-scrolling room adjacency: schema vocabulary exists, but robust loading-zone-free traversal needs a prototype.
- [~] Generated sprites/music/backgrounds through tools: usable, but staging/publish workflow needs more clarity.
- [ ] Generic ray/shape cast query API exposed as a reusable mechanic primitive.
- [ ] Surface tangent/normal query helpers for mechanics that need slope/bounce/ledge semantics.
- [ ] Vector/scalar fields for wind, current, gravity, heat, or faction influence.
- [ ] Deterministic randomness streams for generated systems and replayable tests.

## Simulation and validation

- [x] Trace/replay/debug hooks for movement/combat validation.
- [x] Headless `SandboxSim` stepping path.
- [~] Avian2D secondary physics for debris/props; not the primary player controller.
- [~] Time-domain vocabulary is documented; full per-entity proper-time gameplay is future work.
- [ ] Headless screenshot / visual verification path.
- [ ] PyO3 or equivalent external research binding for `SandboxSim`.
- [ ] Reward-shaping examples for AI playtesting.

## Validation anchors

```bash
cargo test -p ambition_engine combat
cargo test -p ambition_engine projectile
cargo test -p ambition_engine movement
cargo test -p ambition_sandbox scripted_gameplay
```

Prefer exact tests named by concepts or benchmark candidates when available. Use `TODO.md` for the centralized accepted task list; use brainstorm docs for speculative mechanic ideas that are not ready for an agent session.
