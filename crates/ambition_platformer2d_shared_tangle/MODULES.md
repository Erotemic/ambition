# `ambition_platformer2d_shared_tangle` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_shared_tangle** — Reusable, content-free platformer runtime primitives.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`app_finalization`](src/app_finalization.rs) | Finish a manually driven `App` the way a runner would. |
| [`authored_logic`](src/authored_logic/mod.rs) | Extensible authored condition and command contracts. |
| [`binding`](src/binding.rs) | THE BINDING BOUNDARY MOVED OUT OF THIS CRATE — this is only the door. |
| [`block_nudge`](src/block_nudge.rs) | A struck block flinches — the presentation half of hitting one. |
| [`body`](src/body.rs) | Unified body kinematics for every controllable platformer body. |
| [`camera_ease`](src/camera_ease.rs) | Smoothed camera scale + world-target state with tunable ease rates, plus the per-body [`PlayerBlinkCameraState`] the arrival ease reads. |
| [`camera_layers`](src/camera_layers.rs) | Shared presentation-camera markers and render-layer reservations. |
| [`class_b`](src/class_b.rs) | Per-frame ledger for discontinuous Class-B body remaps. |
| [`construction`](src/construction/mod.rs) | Content-free construction planning and explicit spawn provenance. |
| [`developer_hotkeys`](src/developer_hotkeys.rs) | Canonical keyboard deck for developer-only host actions. |
| [`feature_kind`](src/feature_kind.rs) | The feature-visual TAXONOMY shared by the sim (which stamps it at spawn) and every read-model/presentation consumer. |
| [`feature_overlay`](src/feature_overlay.rs) | Shared read resource for transient ECS-derived world collision overlays. |
| [`frame_env`](src/frame_env.rs) | Per-body movement-frame resolution. |
| [`gameplay_presentation`](src/gameplay_presentation/mod.rs) | Gameplay presentation policy: where the gameplay camera renders on the physical display, and where important subjects should stay inside it. |
| [`gravity`](src/gravity.rs) | Shared world physics applied to every actor body. |
| [`held_item_art`](src/held_item_art.rs) | Provider-contributed art declarations for inventory/held items (the ground pickup + in-hand icon of a `HeldItem`: an axe, a javelin, a gun-sword, a wielded-gauntlet ability prop). |
| [`lifecycle`](src/lifecycle/mod.rs) | Lifecycle vocabulary for reusable platformer entities. |
| [`markers`](src/markers.rs) | Content-free entity markers shared by reusable mechanics and presentation. |
| [`math`](src/math.rs) | Pure portal-map vector math for platformer mechanics. |
| [`orientation`](src/orientation.rs) | Actor orientation under gravity (the "which way is down" upright reflex). |
| [`physics`](src/physics.rs) | Shared secondary-physics settings resource. |
| [`prelude`](src/prelude.rs) | Convenience imports for reusable platformer-runtime call sites. |
| [`projectile`](src/projectile/mod.rs) | Content-free projectile physics: authored specs, kinematic state, and world collision. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_platformer2d_shared_tangle`. |
| [`safe_position`](src/safe_position.rs) | Where the player was last standing safely, and the gate that decides it. |
| [`schedule`](src/schedule.rs) | Runtime schedule vocabulary independent of game content. |
| [`shrine`](src/shrine.rs) | Shared presentation pulse state for save/heal shrines. |
| [`sim_id`](src/sim_id.rs) | Stable deterministic identity for snapshot, replay, and netcode. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`temporary_control`](src/temporary_control.rs) | Temporary-control state: whether an autonomous actor is currently masked by a transient controller (player possession or a mount), recorded by STABLE [`SimId`] so it survives a snapshot rewind in both directions. |
| [`time`](src/time.rs) | Neutral simulation-time resource for the platformer runtime. |
| [`transit`](src/transit.rs) | Generic body-transit velocity math for platformer mechanics. |
| [`world_item_art`](src/world_item_art.rs) | Provider-contributed art declarations for walk-into world items. |
| [`world_log`](src/world_log.rs) | Coarse `[game-mode]` / `[world-event]` logging. |

_34 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
