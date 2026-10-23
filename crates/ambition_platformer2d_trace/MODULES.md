# `ambition_platformer2d_trace` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_trace** — Gameplay flight recorder / OOB debug logging.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`actor_oob`](src/actor_oob.rs) | Non-player-centric OOB recorder. |
| [`detect`](src/detect.rs) | OOB/anomaly detection + frame-snapshot construction for the trace recorder. |
| [`plugin`](src/plugin.rs) | Module-local Bevy [`Plugin`] for the gameplay trace recorder. |
| [`systems`](src/systems.rs) | Trace recorder Bevy systems + the headless entry point. |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
