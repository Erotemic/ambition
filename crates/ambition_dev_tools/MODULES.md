# `ambition_dev_tools` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_dev_tools** — Reusable developer-tooling state + logic (E1d carve out of `ambition_actors`).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`dev_tools`](src/dev_tools/mod.rs) | Developer-facing tuning and inspection tools. |
| [`persistence`](src/persistence.rs) | Disk persistence for the [`DeveloperTools`] resource (developer.ron). |
| [`profiling`](src/profiling.rs) | Lightweight startup profiler. |

_3 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
