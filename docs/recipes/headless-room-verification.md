---
status: current
last_verified: 2026-11-10
related_docs:
  - docs/systems/headless-simulation.md
  - docs/concepts/testing-and-validation.md
---

# Verify a room or gameplay sequence

Use the real provider and runtime path when the question concerns playable
behavior. A hand-built ECS world can isolate a rule, but it cannot establish
that room construction, control, lifecycle, and presentation work together.

## Find an existing entrypoint

```bash
python scripts/agent_query.py "headless room load provider"
python scripts/agent_query.py tests "<room id or behavior>"
python scripts/agent_query.py crate ambition_sim_harness
```

Reuse an existing harness or scenario. Add a permanent test only for a
consequential regression or acceptance requirement not already covered.

## Choose the evidence you need

| Question | Tool | What it establishes |
|---|---|---|
| Does a movement, combat, transition, or persistence result occur? | Provider/runtime headless scenario | Actual simulation and lifecycle behavior |
| Where are the authored volumes and placements? | `render_room_geometry` | Spatial boxes without sprite art |
| How does the scene look through the real renderer? | `capture_scene` | Sprites, effects, camera, and UI when enabled |
| Is it readable and fun to control? | Play the game | Timing, feel, usability, and presentation together |

A geometry render cannot answer an art question, and a screenshot does not
prove what the simulation admitted. Use both when the question spans those
boundaries. Avoid constructing a second approximation of the render pipeline to
stand in for the real capture.

## Useful commands

```bash
# Select a relevant existing test, rather than running the whole suite.
./run_tests.sh -k <test_substring>

# Inspect collision and room geometry (no rendered sprites).
cargo run -p ambition_app_tools --bin render_room_geometry -- <ROOM_ID>

# Capture actual presentation. The tool prints its full option vocabulary.
cargo run -p ambition_app_tools --bin capture_scene -- --help
cargo run -p ambition_app_tools --bin capture_scene -- <ROOM_ID> player /tmp/room.png
```

`capture_scene` accepts `--route smash_gameplay --character <id>` for a fighter
under Smash rules. A character spawned into an exploration room is not a Smash
match. Use `--combat-overlay` when the hit and collision volumes must appear in
the image; use `AMBITION_QUALITY_PROFILE=high` or `ultra` when judging art.
`--screen-effect` explicitly enables post-processing in an isolated capture.
`--press` can drive input, including `hold:` / `release:` gestures. Consult
`--help` for timing, shooting, and specialized portal options.

For repeated deterministic frames, `--tick-per-frame`, `--frames`, and `--stride`
are available. Capturing a full rendered sequence is more expensive than
inspecting geometry; choose it when the result depends on actual presentation.

## Acceptance observations

In a gameplay scenario, first establish that the target situation occurred.
Then inspect the outcome that motivated the task: the relevant actor or object,
its spatial relationship, its gameplay effects, and what happens after cleanup,
reset, or transition when applicable. Avoid pinning incidental frame numbers
and coordinates as permanent contracts.

The default is to implement and playtest. Retain a focused automated regression
when the discovered defect is important and likely to recur.
