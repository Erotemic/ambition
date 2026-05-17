# LDtk hot reload

LDtk hot reload is a development convenience for applying the on-disk sandbox world while the Bevy app is running. The canonical authoring path is still the LDtk editor plus `ambition_ldtk_tools` validation/repair.

## Current workflow

Run the sandbox with hot reload enabled:

```bash
cargo run -p ambition_sandbox --features dev_hot_reload --release
```

Useful dev hotkeys:

- `F11`: validate/apply the on-disk LDtk file.
- `F12`: toggle auto-apply after file changes.
- `F5`: overview camera for large/stitched spaces.

Before and after significant LDtk edits, run:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```

## Rules

- Do not hand-edit `sandbox.ldtk` JSON.
- Use `repair`, `roundtrip`, and `doctor` before committing tool-generated changes.
- Treat loading zones, collision IntGrid values, active area metadata, camera zones, and transition arrivals as spatial review areas.
- If hot reload fails, prefer fixing the LDtk data/tooling rather than adding runtime leniency.

Related docs: `docs/recipes/ldtk-authoring.md`, `docs/tools/ldtk-tools.md`, `docs/systems/ldtk-world-composition.md`.
