# `ambition_gameplay_trace` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_gameplay_trace** — Gameplay flight-recorder format — the reusable, content-free core of the trace recorder.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`actor_trace`](src/actor_trace.rs) | Non-player-centric body trace: a rolling timeline of EVERY simulated body's kinematic state (player, boss, enemy, NPC — no privileged observer) plus a per-body out-of-bounds classifier and a dump-on-OOB writer. |
| [`buffer`](src/buffer.rs) | The `GameplayTraceBuffer` resource: a rolling ring buffer of per-frame snapshots and discrete events that the game's recorder systems push into. |
| [`dump`](src/dump.rs) | Dump writers: serialize a `GameplayTraceBuffer` to a timestamped markdown + JSON pair (`write_dump`, path/label helpers). |
| [`model`](src/model.rs) | Serializable trace data shapes: the per-frame `GameplayTraceFrame` (player + platform + control state) and the discrete `GameplayTraceEvent` / `DumpReason` / `OobReason` enums, plus serde-friendly geometry mirrors (`TracePoint`, `TraceAabb`) that avoid leaking `bevy_math`/engine types into the JSON shape. |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
