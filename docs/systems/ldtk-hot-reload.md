# LDtk hot reload

LDtk hot reload is a development convenience for applying the on-disk sandbox
world while the Bevy app is running. The canonical authoring path remains the
LDtk editor plus `ambition_ldtk_tools` validation and repair.

## Current workflow

Run the sandbox with hot reload enabled:

```bash
cargo run -p ambition_app --bin ambition_game_bin --features dev_hot_reload --release
```

The canonical developer deck provides:

- `F11`: validate and apply the on-disk LDtk file;
- `F12`: toggle auto-apply after file changes;
- `F5`: overview camera for large or stitched spaces.

The physical bindings live in
`crates/ambition_platformer_primitives/src/developer_hotkeys.rs`.

Before and after significant LDtk edits, run:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  game/ambition_content/assets/worlds/sandbox.ldtk
```

## GGRS session policy

A local developer GGRS session does not block hot reload. The reload transaction:

1. records whether the app owns a local SyncTest session;
2. queues complete removal of that session and cancels any active proof pulse;
3. validates and prepares the candidate LDtk world without mutating live state;
4. commits the world and prepared-content identity atomically on success;
5. starts a fresh zero-distance local GGRS baseline at frame zero.

If validation fails, live content and its epoch remain unchanged and the local
baseline is still restarted against that unchanged identity.

An external or P2P GGRS session is different: one peer cannot change content
unilaterally. Hot reload is rejected until a coordinated peer content barrier is
implemented. A non-GGRS composition applies the same content transaction without
any session stop/restart.

## Rules

- Do not hand-edit `sandbox.ldtk` JSON.
- Use `repair`, `roundtrip`, and `doctor` before committing tool-generated changes.
- Treat loading zones, collision IntGrid values, active-area metadata, camera
  zones, and transition arrivals as spatial review areas.
- If hot reload fails, fix the LDtk data/tooling rather than adding runtime
  leniency.

Related docs: `docs/recipes/ldtk-authoring.md`, `docs/tools/ldtk-tools.md`,
`docs/systems/ldtk-world-composition.md`.
