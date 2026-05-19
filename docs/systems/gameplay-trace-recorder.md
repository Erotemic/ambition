# Gameplay trace recorder

The gameplay trace recorder captures recent control frames, simulation messages, player snapshots, and diagnostic context so movement/combat bugs can be replayed or inspected instead of guessed at.

## Current paths

```text
crates/ambition_sandbox/src/dev/trace.rs
crates/ambition_sandbox/src/dev/trace/
  buffer.rs
  detect.rs
  dump.rs
  model.rs
  systems.rs
  tests.rs
crates/ambition_sandbox/src/bin/trace_replay.rs
```

The crate root re-exports `dev::trace` as `ambition_sandbox::trace` for compatibility. Prefer the `src/dev/trace/` path in new docs.

## What it captures

Every simulation tick, the recorder pushes a `GameplayTraceFrame` into a fixed-capacity ring buffer. The frame is meant to answer "what did the game think was true just before this bug?" without requiring a video capture.

Typical frame data includes:

- tick / sequence index,
- real `dt`, simulation `dt`, and time-scale context,
- current game mode,
- active LDtk area id and world bounds,
- player position, velocity, AABB, size, and facing,
- locomotion state and body mode,
- grounded/wall/ledge/blink/fly/fast-fall/climb-relevant flags,
- dash / jump / resource counters where available,
- last known safe player position,
- raw `ControlFrame` snapshot,
- nearby collision shapes sorted by distance and capped to keep dumps small,
- moving-platform state when present.

A separate event ring captures discrete events: jumps, dashes, blinks, attacks, damage, room transitions, projectile events, OOB detections, and collision-correction style anomalies.

## Synthesized events

The recorder should stay mostly passive. `record_frame_system` can diff the current frame against the previous snapshot and synthesize high-value events without threading collectors through every gameplay helper.

Important synthesized event classes:

| Event | Trigger |
|---|---|
| `RoomTransition` | Active area changed since the previous tick. |
| `Reset` | Player reset counter increased. |
| `CollisionCorrection` | Position delta exceeds the velocity budget by a suspicious amount and no Reset/RoomTransition explains it. |
| `PlayerModeChanged` | `LocomotionState` or `BodyMode` changed. |
| `Dash` / `DoubleJump` / `Jump` | Resource/velocity/input edges imply the action happened. |
| `Blink` | Blink aim/grace state changed. |
| `Damage` / `Death` | Health decreased or death event was observed. |
| `Attack` / `InputEdge` | Relevant `ControlFrame` buttons changed. |
| `OobDetected` | `detect_oob` returned a reason. |

Helpers may still push direct events when state diffs are insufficient, for example projectile fired/expired, encounter events, or special mechanic failures.

## Hotkeys and triggers

| Trigger | Dump reason | When |
|---|---|---|
| `F8` | `Manual` | User wants a snapshot of the last few seconds. |
| Auto OOB | `OobAuto` | The player drifts outside the world envelope, has non-finite position/velocity, has absurd velocity, or sits inside a solid. |
| `request_dump(label)` | `Programmatic` | Tests or tooling request a dump from code. |

The auto-dump should be armed once per unhealthy episode. After it fires, wait for the player to return to a healthy state before re-arming so one broken frame does not produce a dump every tick. Manual `F8` should always work.

The HUD exposes trace status and reminds developers that `F8` dumps the current buffer.

## OOB detection

`detect_oob` checks, in order:

1. position is non-finite,
2. velocity is non-finite,
3. velocity magnitude is absurd,
4. AABB is outside the world envelope beyond the configured OOB margin,
5. AABB strictly intersects a `Solid` block after movement resolution.

The first match wins. The reason is recorded as both a `GameplayTraceEvent::OobDetected` event and the auto-dump reason string.

Do not fix trace-visible teleports by only widening margins. If a trace shows a collision correction with a delta larger than the frame's velocity budget, explain the correction source and add a regression test.

## Output paths

Dumps are written under `debug_traces/` relative to the sandbox working directory. Each dump produces two files with the same stem:

```text
debug_traces/ambition_trace_{secs}-{nanos}-{seq}_{Dd}d{HH}h{MM}m{SS}s.json
debug_traces/ambition_trace_{secs}-{nanos}-{seq}_{Dd}d{HH}h{MM}m{SS}s.md
```

The JSON is the machine-readable source of truth. The Markdown summary is for triage and should include:

- dump reason and timestamp,
- latest frame summary,
- nearby collision shapes,
- first OOB event in the captured window,
- compact frame timeline,
- recent event timeline,
- hints such as last safe position versus OOB position and last Blink/Dash/RoomTransition before OOB.

Attach both files to movement/combat bug reports. The Markdown is enough for quick review; the JSON is for replay or deeper `jq`/script inspection.

## Integration shape

The recorder lives at the simulation seam:

- `GameplayTraceBuffer` is a Bevy resource shared by visible and headless builds.
- `record_frame_system` observes current gameplay state and appends one frame.
- `flush_pending_dump` drains pending dump requests and writes JSON + Markdown.
- `handle_trace_hotkey` reads the visible-build `F8` hotkey and requests a manual dump.
- `trace_replay` reads a trace JSON and drives a fresh `SandboxSim` where supported.

Adding richer context usually means:

1. add a serializable field or `GameplayTraceEvent` variant,
2. emit or synthesize it at the sim seam,
3. extend the Markdown formatter only if it improves triage.

## What the trace does not guarantee

- It is not automatically a perfect deterministic replay. Real-time dt jitter, runtime subscribers, and floating-point reduction order can still matter.
- It is not a video recorder. Visual state is inferred from gameplay data; screenshot tooling is tracked separately in `TODO.md`.
- It is not yet a complete enemy/projectile timeline, although projectile and encounter events can be emitted when needed.

## When to use it

Use traces for bugs involving:

- unexplained collision correction,
- wall cling / ledge / blink edge cases,
- loading-zone or transition placement,
- out-of-bounds movement,
- attack/projectile timing that depends on a sequence of control frames,
- platform carrying/riding edge cases,
- touch/controller input bugs that depend on a control-frame sequence.

## Attaching a trace to a bug report

1. Start the sandbox and reproduce the issue.
2. Let auto-OOB dump if the bug leaves the world or enters a solid.
3. For subtler bugs, press `F8` immediately after reproducing.
4. Attach the newest matching `.json` and `.md` pair from `debug_traces/`.
5. Add a focused regression test when the trace exposes a durable failure mode.

## Rules

- Keep trace records compact enough to attach to bug reports.
- Record enough geometry and player-state context to distinguish a collision bug from an input bug.
- Add a regression test when a trace exposes a durable failure mode.
- Do not fix trace-visible teleports by only widening OOB margins; explain the collision correction.

## Validation anchors

```bash
cargo test -p ambition_sandbox trace
cargo run -p ambition_sandbox --bin trace_replay -- --help
```

Related docs: `docs/planning/tech-debt-log.md`, `docs/systems/headless-simulation.md`, `dev/journals/lessons_learned.md`.
