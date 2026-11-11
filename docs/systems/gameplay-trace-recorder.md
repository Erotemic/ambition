---
status: current
last_verified: 2026-11-10
related_docs:
  - docs/systems/headless-simulation.md
  - docs/recipes/explaining-a-tick.md
---

# Gameplay trace recorder

The flight recorder captures bounded simulation snapshots and semantic events
for debugging, comparison, and replay. It observes authoritative state; traces
do not drive gameplay authority.

## Implementation

- `crates/ambition_gameplay_trace/` defines `GameplayTraceBuffer`, frame and
  event formats, and JSON/Markdown dump writers. Default rolling capacity is
  240 frames and 240 events; a caller may select another capacity.
- `crates/ambition_platformer2d_trace/` records body, world, input, collision,
  portal, and session observations and detects out-of-bounds/correction events.
- `game/ambition_app_tools/src/bin/headless.rs` can write a larger capture after
  stepping the simulator. `trace_replay` reads its JSON dump.
- `F8` requests a gameplay trace dump in a developer-visible host.

The ordinary recorder is a **bounded player/body-oriented flight recorder**,
not an exhaustive recording of every entity and every ECS mutation.
For an unfamiliar subsystem, inspect the event vocabulary before assuming
its facts are included.

## Capture and inspect

```bash
cargo run -p ambition_app_tools --bin headless -- 600 --dump-trace target/trace
```

This headless command records an idle-input baseline. For a particular action,
use the harness's typed input or capture the corresponding visible session.
The generated JSON filename includes a timestamp; see
[headless simulation](headless-simulation.md) for replay commands.

When investigating a failure, locate the relevant semantic event and the
preceding authoritative state change. Distinguish a missing event from a
scenario that never reached the expected action. Compare meaningful outcomes
and causal order instead of routinely snapshotting entire traces into tests.

The recorder's IO/dumps run outside historical rollback passes; replay should
not duplicate irreversible diagnostic writes.
