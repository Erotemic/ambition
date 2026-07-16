# `ambition_platformer_primitives` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer_primitives** — Reusable, content-free platformer runtime primitives.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`body`](src/body.rs) | Unified body kinematics for every controllable platformer body. |
| [`camera_ease`](src/camera_ease.rs) | Smoothed camera scale + world-target state with tunable ease rates. |
| [`camera_layers`](src/camera_layers.rs) | Presentation camera markers shared by host, render, and app wiring. |
| [`class_b`](src/class_b.rs) | **Class-B transit authority** — the per-frame remap ledger. |
| [`feature_kind`](src/feature_kind.rs) | The feature-visual TAXONOMY shared by the sim (which stamps it at spawn) and every read-model/presentation consumer. |
| [`feature_overlay`](src/feature_overlay.rs) | Shared read resource for transient ECS-derived world collision overlays. |
| [`frame_env`](src/frame_env.rs) | The authoritative per-body movement frame: resolved once, consumed everywhere. |
| [`gravity`](src/gravity.rs) | Shared world physics applied to every actor body. |
| [`held_item_art`](src/held_item_art.rs) | Provider-contributed art declarations for inventory/held items (the ground pickup + in-hand icon of a `HeldItem`: an axe, a javelin, a gun-sword, a wielded-gauntlet ability prop). |
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
| [`sim_id`](src/sim_id.rs) | **`SimId` — the one identity vocabulary for snapshot, replay, and netcode.** |
| [`time`](src/time.rs) | Neutral simulation-time resource for the platformer runtime. |
| [`transit`](src/transit.rs) | Generic body-transit velocity math for platformer mechanics. |
| [`world_item_art`](src/world_item_art.rs) | Provider-contributed art declarations for walk-into world items. |

_23 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
