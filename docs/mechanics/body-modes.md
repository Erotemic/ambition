# Body modes

Body modes describe traversal states that change collision shape, movement affordances, or player posture: crouch, crawl, slide, compact/morph-like movement, swim/sink variants, and future specialized traversal modes.

## Current status

- The sandbox has body-mode modules under `crates/ambition_sandbox/src/body_mode/`.
- Player-authoritative runtime state lives on ECS player components under `crates/ambition_sandbox/src/player/`.
- Reusable movement/collision vocabulary belongs in `ambition_engine`.
- Authored traversal examples should be LDtk rooms/specs, not hard-coded one-off checks.

## Design rules

- A body mode must define its collision shape and the rules for entering/exiting that shape safely.
- Shape changes must fail gracefully when the expanded shape would overlap collision.
- Input interpretation should produce a requested body mode; collision and movement systems decide whether the transition is legal.
- Presentation should reflect mode changes but not be the source of truth.
- Keep water, iron-boots, swim, sink, and murky variants as mode policy layered on top of the same shape/affordance vocabulary where practical.

## Current and likely modes

| Mode | Backend status | Notes |
|---|---:|---|
| Standing/running | Available | Default platforming shape. |
| Crouch/crawl | Available vocabulary | Needs more authored traversal rooms. |
| Slide | Available vocabulary | Tune around collision-safe transitions. |
| Compact/morph-like traversal | Available vocabulary | Needs stronger showcase coverage. |
| Swim/surface swim/sink/iron-boots water modes | Design direction | Should reuse body-mode + volume policy where possible. |
| Spider-ball/spring-ball/bomb traversal | Future | Needs explicit backend semantics before content authoring. |

## Validation anchors

```bash
cargo test -p ambition_engine movement
cargo test -p ambition_sandbox body_mode
cargo test -p ambition_sandbox scripted_gameplay
```
