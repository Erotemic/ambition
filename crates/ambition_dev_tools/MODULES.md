# `ambition_dev_tools` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_dev_tools** — Reusable developer-tooling state and simulation-side logic.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`brain_override`](src/brain_override.rs) | A measurement knob: replace the brain every authored actor spawns with. |
| [`dev_tools`](src/dev_tools/mod.rs) | Developer-facing tuning and inspection tools. |
| [`hot_reload`](src/hot_reload.rs) | A debounced mtime watch over the authored world file, and the transactional reload it offers the developer controls. |
| [`perception_census`](src/perception_census.rs) | How many peers each viewer actually KEEPS, per tick. |
| [`persistence`](src/persistence.rs) | Disk persistence for the [`DeveloperTools`] resource (developer.ron). |
| [`population_cap`](src/population_cap.rs) | A measurement knob: cap how many authored actors a room admits. |
| [`profiling`](src/profiling.rs) | Lightweight startup profiler. |
| [`runtime_census`](src/runtime_census.rs) | Profiling-only workload census: what Ambition asked the engine to do. |
| [`sim_plugin`](src/sim_plugin.rs) | `DevToolsSimPlugin` — the dev-tools DOMAIN plugin for the simulation App. |

_9 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
