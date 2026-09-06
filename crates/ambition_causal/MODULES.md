# `ambition_causal` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_causal** — Typed observer-only facts for explaining why simulation state changed.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`ecs`](src/ecs.rs) | The causal log as a Bevy resource. |
| [`fact`](src/fact.rs) | The fact vocabulary — enough identity to CORRELATE, and no more. |
| [`log`](src/log.rs) | The bounded log and the tick explainer. |
| [`sink`](src/sink.rs) | Thread-local causal-fact sink for instrumenting pure call trees without threading a recorder through every intermediate function. |
| [`unclaimed`](src/unclaimed.rs) | Detect velocity steps larger than the caller's integrator can produce when no operation on that tick claims the change. |
| [`velocity`](src/velocity.rs) | Who wrote this body's velocity. |

_6 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
