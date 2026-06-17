# ECS inventory tool

`scripts/ecs_inventory.py` builds a static inventory of the `ambition_gameplay_core` Bevy ECS surface.
It is meant for refactor planning and code-review diffs, not as a replacement for `cargo check`.

## Usage

From the repository root:

```bash
python3 scripts/ecs_inventory.py
```

By default this writes:

```text
target/ambition_ecs_inventory.json
target/ambition_ecs_inventory.md
```

Useful variants:

```bash
# Include test modules and test-only spawn/system evidence.
python3 scripts/ecs_inventory.py --include-tests

# Write inventory somewhere explicit.
python3 scripts/ecs_inventory.py \
  --json docs/generated/ambition_ecs_inventory.json \
  --markdown docs/generated/ambition_ecs_inventory.md

# CI / review guard: compare a generated inventory with a checked-in snapshot.
python3 scripts/ecs_inventory.py \
  --json target/ambition_ecs_inventory.json \
  --markdown target/ambition_ecs_inventory.md \
  --check-json docs/generated/ambition_ecs_inventory.json
```

## What it collects

The JSON and Markdown include:

- ECS item declarations that derive `Component`, `Bundle`, `Resource`, `Message`, or `Event`.
- `impl Plugin for ...` plugin declarations.
- Bevy registration calls such as `add_systems`, `configure_sets`, `add_message`, `add_event`, `init_resource`, `insert_resource`, and `add_plugins`.
- System-like function definitions, detected by Bevy system parameters such as `Commands`, `Query`, `Res`, `ResMut`, `MessageReader`, and `MessageWriter`.
- Entity archetype evidence from `spawn(...)` / `spawn_empty(...)` call sites, including identifiers and `Name::new(...)` labels found in the spawn expression.

## Interpretation guidelines

Static analysis cannot prove the exact runtime entity set. Treat the entity section as source evidence:
spawn sites, bundles, component identifiers, and labels. This is usually the right granularity for refactor planning because entity instances are data- and state-dependent.

The registration section intentionally keeps raw identifiers from registration expressions. Some identifiers are schedule sets, run conditions, plugins, or helper functions rather than systems. The raw evidence makes diffs reviewable when schedules are reorganized.

## Robustness strategy

The script avoids plain grep where it matters:

- masks Rust comments and string literals while preserving source offsets;
- tracks balanced parentheses for registration and spawn calls;
- supports multi-line attributes and common Rust visibility forms;
- writes deterministic JSON with sorted keys for code-review diffs;
- excludes `src/bin` and test modules by default, with `--include-tests` available.

For a future stricter version, consider replacing item extraction with a small Rust `xtask` built on `syn` or rust-analyzer crates. The current Python script is dependency-free so it can run in lightweight chat/overlay workflows and CI without building extra tooling.
