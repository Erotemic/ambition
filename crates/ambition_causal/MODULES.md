# `ambition_causal` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_causal** — **Why did this actor change on this tick?**

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`ecs`](src/ecs.rs) | The log as a Bevy resource. |
| [`fact`](src/fact.rs) | The fact vocabulary — enough identity to CORRELATE, and no more. |
| [`log`](src/log.rs) | The bounded log and the tick explainer. |
| [`sink`](src/sink.rs) | The scoped sink — how a pure function deep in a call graph publishes a fact without every caller between it and the ECS growing a parameter. |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
