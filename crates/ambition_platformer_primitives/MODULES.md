# `ambition_platformer_primitives` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer_primitives** — Reusable, content-free platformer runtime primitives.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`body`](src/body.rs) | Unified body kinematics for every controllable platformer body. |
| [`camera_ease`](src/camera_ease.rs) | Smoothed camera scale + world-target state with tunable ease rates. |
| [`camera_layers`](src/camera_layers.rs) | Presentation camera markers shared by host, render, and app wiring. |
| [`feature_overlay`](src/feature_overlay.rs) | Shared read resource for transient ECS-derived world collision overlays. |
| [`gravity`](src/gravity.rs) | Shared world physics applied to every actor body. |
| [`kinematic`](src/kinematic.rs) | Generic kinematic body — gravity + axis-separated sweep against a `World`. |
| [`lifecycle`](src/lifecycle/mod.rs) | Lifecycle vocabulary for entities spawned by reusable platformer systems. |
| [`markers`](src/markers.rs) | Generic entity-marker components shared by reusable mechanics. |
| [`math`](src/math.rs) | Pure portal-map vector math for platformer mechanics. |
| [`orientation`](src/orientation.rs) | Actor orientation under gravity (the "which way is down" upright reflex). |
| [`physics`](src/physics.rs) | Shared secondary-physics settings resource. |
| [`prelude`](src/prelude.rs) | Convenience imports for reusable platformer-runtime call sites. |
| [`projectile`](src/projectile/mod.rs) | Reusable, game-agnostic projectile physics primitive. |
| [`schedule`](src/schedule.rs) | Runtime schedule vocabulary that is independent of Ambition content. |
| [`shrine`](src/shrine.rs) | Shared presentation pulse state for save/heal shrines. |
| [`time`](src/time.rs) | Neutral simulation-time resource for the platformer runtime. |
| [`transit`](src/transit.rs) | Generic body-transit velocity math for platformer mechanics. |

_17 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
