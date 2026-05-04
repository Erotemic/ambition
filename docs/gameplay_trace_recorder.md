# Gameplay flight recorder

A rolling per-frame trace + auto-OOB dump tool for debugging the active
collision/out-of-bounds class of bugs.

## What it captures

Every frame `sandbox_update` ticks, the recorder pushes one
`GameplayTraceFrame` into a fixed-capacity ring buffer (default 240
frames, ~4 seconds at 60 Hz). The frame includes:

- tick / sequence index
- real `dt`, sim `dt`, and `time_scale`
- current `GameMode` (`Playing`, `Paused`, `Dialogue`, …)
- active LDtk area id and world bounds
- player position, velocity, AABB, size, facing
- locomotion state (Grounded / Airborne / Dashing / Blinking / WallSlide / …)
- body mode (Standing / Crouching / MorphBall / …)
- on_ground / on_wall / wall_clinging / wall_climbing / fly / fast_falling
- dash charges and air jumps remaining
- blink aim state and grace timer
- last known safe player position (set whenever the player is grounded
  outside damage frames)
- raw `ControlFrame` snapshot
- nearby collision shapes within ~220 px sorted by distance (capped to
  32 entries to keep dumps small)
- moving-platform state — pos, size, AABB, direction (+1/-1), riding flag, distance from player

A separate ring of 240 `GameplayTraceEvent`s captures discrete
sim-side events: jumps, dashes, blinks, attacks, damage, room
transitions, OOB detections, and (when wired) collision corrections.
The `record_frame_system` always pushes a frame; events are appended
when the simulation pipeline emits them or when OOB detection fires.

## Synthesized events (diff-based)

The recorder is a passive observer. Each tick, `record_frame_system`
diffs the current sim state against the previous frame's snapshot and
synthesizes events without touching `sandbox_update`'s phase pipeline.
This avoids threading a Vec collector through every helper while still
producing a useful timeline.

Currently emitted:

| Event                  | Trigger                                                                 |
| ---------------------- | ----------------------------------------------------------------------- |
| `RoomTransition`       | `active_area` changed since last tick                                   |
| `Reset`                | `player.resets` increased                                               |
| `CollisionCorrection`  | position delta exceeds the velocity-budget by more than 16 px and no Reset/RoomTransition fired this tick. Catches teleports / unexpected pos jumps. |
| `PlayerModeChanged`    | `LocomotionState` or `BodyMode` differs                                 |
| `Dash`                 | `dash_charges_available` decreased                                      |
| `DoubleJump`           | `air_jumps_available` decreased                                         |
| `Jump`                 | upward velocity edge while jump was pressed                             |
| `Blink` (start)        | `blink_aiming` flipped from false → true                                |
| `Blink` (precision)    | `blink_grace_timer` flipped from ≤0 → >0                                |
| `Damage` / `Death`     | `player_health.current` decreased                                       |
| `Attack`               | `attack_pressed` / `pogo_pressed` edge                                  |
| `InputEdge`            | for each `ControlFrame` button: previous-frame false, current true      |
| `OobDetected`          | `detect_oob` returns Some                                               |

Helpers may still call `buffer.push_event` directly for events that
aren't derivable from state (e.g. "pogo bounce found no valid orb"); the
diff-based path is the cheap default.

## Filename uniqueness

Dump filenames embed the unix seconds, the sub-second nanoseconds, and a
process-wide atomic sequence so two dumps in the same nanosecond still
get distinct paths:

```
debug_traces/ambition_trace_{secs}-{nanos}-{seq}_{Dd}d{HH}h{MM}m{SS}s.{json,md}
```

Lexical order matches chronological order, so `ls -1 debug_traces` lists
dumps in the order they were taken.

## Hotkeys and triggers

| Trigger              | Reason       | When                         |
| -------------------- | ------------ | ---------------------------- |
| `F8`                 | `Manual`     | User asked for a snapshot    |
| Auto OOB             | `OobAuto`    | First frame the player drifts outside the world envelope, has non-finite pos/vel, has absurd velocity, or sits inside a `Solid` |
| `request_dump(label)` (Rust) | `Programmatic` | Tests / tooling can request a dump from code |

The auto-dump is "armed" once. After it fires the recorder waits for
the player to return to a healthy state before re-arming, so a single
broken frame does not produce 60 dumps per second. Manual `F8` always
fires.

## OOB detection

`detect_oob` checks, in order:

1. position is non-finite,
2. velocity is non-finite,
3. velocity magnitude exceeds 8000 (absurd),
4. AABB is outside the world envelope by more than the OOB margin
   (default 96 px on x or y),
5. AABB strictly intersects any `Solid` block after movement
   resolution.

The first match wins. The reason is recorded as both a
`GameplayTraceEvent::OobDetected` event and (on auto-dump) the
`DumpReason::OobAuto`'s `reason` string.

## Output paths

Dumps are written under `debug_traces/` relative to the sandbox
working directory. Each dump produces two files:

```
debug_traces/ambition_trace_<unixsecs>_<DdHHhMMmSSs>.json
debug_traces/ambition_trace_<unixsecs>_<DdHHhMMmSSs>.md
```

The JSON is the machine-readable source of truth. The Markdown summary
includes:

- dump reason and timestamp,
- latest frame summary (player pos / vel / AABB / locomotion / body),
- nearby collision (top 16),
- first OOB event in the captured window (if any),
- last 120 frames as a compact one-line-each table,
- last 100 events,
- short hint paragraph (last_safe_pos vs OOB pos, last
  Blink/Dash/RoomTransition before OOB, etc.).

Both files share the same stem so a bug report can attach the `.json`
for the machine view and the `.md` for the summary.

## Integration shape

The recorder lives at the simulation seam:

- `crate::trace::GameplayTraceBuffer` — Bevy `Resource`, lives on the
  simulation half so headless and visible builds share the buffer.
- `crate::trace::record_frame_system` — runs `.after(sandbox_update)`,
  reads `SandboxRuntime`, `GameWorld`, `ControlFrame`, `RoomSet`, and
  `GameMode` and appends one frame.
- `crate::trace::flush_pending_dump` — runs after the recorder system,
  drains a pending `dump_request` and writes JSON + Markdown.
- `crate::trace::handle_trace_hotkey` — presentation-side, reads
  `Res<ButtonInput<KeyCode>>` for `F8`.

The engine-side primitives `LocomotionState` and `BodyMode` produce
the labels stored in each frame. Adding richer context (e.g.
collision-correction events) is a matter of:

1. adding the variant to `GameplayTraceEvent`,
2. emitting it from the relevant `sandbox_update` phase helper through
   the existing feedback Vec pattern, and
3. extending the Markdown formatter if you want a richer line in the
   summary.

## What the trace does NOT yet guarantee

- **Not** a deterministic replay. Real-time `dt` jitter,
  audio/VFX/HUD subscribers, and floating-point reductions across
  multiple Bevy systems mean two replays of the same `ControlFrame`
  sequence can drift. A future RL/replay layer (per
  `docs/headless_simulation.md` Phase 3) will need a fixed timestep
  and seeded RNG before the trace becomes a reproducer.
- **Not** an enemy/projectile timeline. Frame snapshots only carry
  the player view. Enemy and feature events are visible through
  `GameplayTraceEvent::Damage` etc. but a richer timeline (per-enemy
  state, per-projectile path) is a follow-up.
- **Not** a video recorder. Visual state is implied by gameplay data;
  for a visual baseline, see the planned
  `bevy_dev_tools::EasyScreenshotPlugin` integration noted in
  `docs/events_refactor_plan.md`.

## Attaching a trace to a bug report

For collision/OOB bugs:

1. Start the sandbox.
2. Reproduce the OOB. The recorder auto-dumps when it detects the
   problem.
3. If the bug is something subtler (player gets stuck, animation
   glitch, blink+platform interaction), press `F8` immediately after
   reproducing.
4. Attach the latest `debug_traces/ambition_trace_*.json` and `.md`
   pair to the bug report. The `.md` summary is enough for triage;
   the `.json` is for deeper inspection (jq queries on
   `frames[].player.vel`, etc.).

The recorder uses `eprintln!` to print the dump path on success or
the error on failure, so the terminal log includes the relevant
filename even if the HUD isn't visible.

## Tests

`crates/ambition_sandbox/src/trace.rs` tests cover:

- ring-buffer wraparound at capacity,
- `detect_oob` for inside-world / outside-x / outside-y / inside-solid /
  non-finite-pos / absurd-velocity,
- `record_frame` pushes an OOB event and arms a `OobAuto` dump,
- dump path generation does not panic,
- `write_dump` produces both files and the JSON header is well-formed.

Run with:

```bash
cargo test -p ambition_sandbox trace::
```

## Related docs

- `docs/mechanics_checklist.md` — `LocomotionState`, `BodyMode`,
  `ResourceMeter` Tier 1 backends used by the trace.
- `docs/headless_simulation.md` — the simulation half the recorder
  attaches to.
- `docs/events_refactor_plan.md` — the per-frame `Vec` collector
  pattern the trace's event channel mirrors.
