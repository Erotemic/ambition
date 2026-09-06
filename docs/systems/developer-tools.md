---
status: current
last_verified: 2026-07-18
---

# Developer tools in the running game

Developer tools are optional host/presentation adapters over stable simulation
and provider interfaces. They may inspect or request changes through explicit
commands, but must not become hidden gameplay authority.

## Ownership model

- `ambition_dev_tools` and focused tool crates own reusable debug resources,
  commands, profiling, overlays, and diagnostics.
- The host/app wires desktop-only input or windows behind features/policy.
- Render/UI adapters visualize read models and debug geometry.
- Simulation domains expose typed commands/queries/traces rather than granting a
  debug panel arbitrary mutable access.
- Provider content may register provider-specific debug destinations or IDs.

Use generated navigation for the current hotkey and command implementation:

```bash
python scripts/agent_query.py "developer tools hotkey debug overlay"
python scripts/agent_query.py crate ambition_dev_tools
```

Do not preserve a hotkey list in this page; hotkeys are implementation details
and should be discoverable from current source/help/UI.

## Rules

- Debug features compile out or remain inert in production personas as intended.
- A developer command has a typed request and deterministic simulation-side
  handler when it affects authoritative state.
- Tools use stable provider IDs, not raw Bevy entity handles, for durable targets.
- Headless diagnostics have a non-visual output (trace, report, snapshot, or
  structured read model).
- Teleports, room switches, grants, and resets use canonical lifecycle and
  construction seams.
- Debug UI cannot silently alter tuning every frame.

## Validation

```bash
./run_tests.sh -p ambition_dev_tools
./run_tests.sh -k dev_tool
python scripts/check_doc_links.py
```

For profiling workflows see [`../recipes/profiling.md`](../recipes/profiling.md).
