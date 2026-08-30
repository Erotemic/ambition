# `ambition_sim_harness` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_sim_harness** — Programmatic harness for driving a platformer simulation headlessly.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`action`](src/action.rs) | Agent-facing action vocabulary and conversion into the engine-owned `ControlFrame`. |
| [`capture`](src/capture.rs) | ⭐⭐ PHOTOGRAPH AN EXACT SIMULATION TICK, with the GPU's latency held outside the clock. |
| [`move_exercise`](src/move_exercise.rs) | ⭐⭐ HOW YOU MAKE A FIGHTER PERFORM A MOVE, in one place. |
| [`observation`](src/observation.rs) | Owned simulation observations exposed to RL agents and scripted drivers. |
| [`options`](src/options.rs) | Construction, timestep, and GGRS rollback options for `Platformer2dSimHarness`. |
| [`random_policy`](src/random_policy.rs) | Small deterministic policies used by harness examples and stress tests. |
| [`reward`](src/reward.rs) | Example reward-shaping functions for the headless RL sim (TODO #198). |
| [`runtime`](src/runtime.rs) | Programmatic Ambition simulation runtime, including direct and GGRS-driven stepping. |

_8 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
