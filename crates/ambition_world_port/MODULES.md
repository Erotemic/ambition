# `ambition_world_port` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_world_port** — Portable world observations, actions, and authored-object contracts.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`behavior`](src/behavior.rs) | Reusable object behavior bindings and directed method calls. |
| [`breakables`](src/breakables.rs) | Create fresh room-owned destructibles through the native world owner. |
| [`camera`](src/camera.rs) | Camera observations and framing intent. |
| [`consumption`](src/consumption.rs) | Consume a destructible from this invocation's native world contacts. |
| [`creation`](src/creation.rs) | Create a checked actor composition and initialize its declared behavior state. |
| [`geometry`](src/geometry.rs) | World geometry identity and confirmed movement contacts. |
| [`inventory`](src/inventory.rs) | Exact inventory exchanges and observations over native item authority. |
| [`object_target`](src/object_target.rs) | Typed occurrence targets and local names in a native construction batch. |
| [`story`](src/story.rs) | Portable room story observations and action requests. |

_9 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
