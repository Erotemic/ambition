# Large-file refactor candidates

Status: 2026-05-10

This note tracks technical-debt pressure from oversized source files and keeps the
next refactor passes aligned with the current facade-module policy in
`docs/systems/code-structure.md`: keep the existing public `foo.rs` as a stable facade and
move implementation into `foo/*.rs` child modules.

## Refactor completed in this patch

`crates/ambition_sandbox/src/dialog.rs` was split without changing the public
`crate::dialog::*` surface:

- `dialog.rs` — stable facade, Yarn plugin hook, public re-exports.
- `dialog/content.rs` — authored dialogue branches, nodes, choices, and
  `DialogMode` routing.
- `dialog/runtime.rs` — `DialogState` and conversation navigation state.
- `dialog/systems.rs` — Bevy input systems and post-quest dialogue redirection.
- `dialog/ui.rs` — overlay components and Bevy UI construction.
- `dialog/tests.rs` — existing focused dialogue tests.

The original file mixed authored content, state machine behavior, Bevy systems,
Bevy UI building, Yarn registration, and tests in one ~1.5k-line edit surface.
The split keeps the same runtime semantics while making future work on content,
input, or UI easier to review independently.

## Next high-value Rust candidates

| File | Current pressure | Suggested split |
| --- | --- | --- |
| `crates/ambition_sandbox/src/pause_menu.rs` | Large UI file with menu model, input routing, rendering, and settings/control handoff in one place. | `pause_menu/model.rs`, `pause_menu/systems.rs`, `pause_menu/ui.rs`, `pause_menu/settings.rs`, `pause_menu/tests.rs`. |
| `crates/ambition_sandbox/src/inventory.rs` | Inventory state, item catalog, purchase/equip logic, and UI live together. | `inventory/model.rs`, `inventory/catalog.rs`, `inventory/transactions.rs`, `inventory/ui.rs`, `inventory/tests.rs`. |
| `crates/ambition_sandbox/src/music/director.rs` | Adaptive music scheduling, state, channel decisions, and test seams are tightly coupled. | `music/director/state.rs`, `music/director/transitions.rs`, `music/director/systems.rs`, `music/director/tests.rs`. |
| `crates/ambition_sandbox/src/features/runtime.rs` | Feature runtime spans content conversion, spawning/runtime state, and progression glue. | `features/runtime/model.rs`, `features/runtime/spawn.rs`, `features/runtime/progression.rs`, `features/runtime/tests.rs`. |
| `crates/ambition_sandbox/src/body_mode.rs` | Body-mode gameplay state and presentation glue are close enough to make edits expensive. | `body_mode/model.rs`, `body_mode/input.rs`, `body_mode/systems.rs`, `body_mode/ui.rs`. |

## Tooling candidates outside Rust

| File | Current pressure | Suggested split |
| --- | --- | --- |
| `tools/experimental/robot_sprite_component_tool/tools/robot_rig_sheet.py` | Data model, geometry helpers, rendering/export, and CLI behavior share one file. | Package under `robot_rig_sheet/` with `model.py`, `geometry.py`, `render.py`, `export.py`, and `cli.py`. |
| `tools/experimental/robot_sprite_component_tool/tools/rig_pose_editor_pyside.py` | Qt UI, pose model, interaction modes, and persistence live together. | `pose_editor/model.py`, `pose_editor/widgets.py`, `pose_editor/commands.py`, `pose_editor/io.py`. |
| `tools/ambition_ldtk_tools/ambition_ldtk_tools/area_authoring.py` | Authoring transformations, validation helpers, and CLI adapter logic are mixed. | `area_authoring/model.py`, `area_authoring/ops.py`, `area_authoring/validation.py`, `area_authoring/cli.py`. |
| `tools/ambition_music_renderer/ambition_music_renderer/musicir_renderer.py` | MusicIR parsing, scheduling, synthesis, and file rendering live in one module. | `musicir/parser.py`, `musicir/schedule.py`, `musicir/synthesis.py`, `musicir/export.py`. |

## Guardrails for follow-up splits

1. Preserve public module paths by keeping `foo.rs` as the facade.
2. Move one concern at a time and re-export existing public types/functions from
   the facade.
3. Prefer `pub(in crate::foo)` or `pub(crate)` only where sibling modules need
   internal access; avoid widening APIs to `pub` just to satisfy the split.
4. Keep tests near the split module and add small regression tests before moving
   more behavior.
5. Update this note and `docs/systems/code-structure.md` after each successful split so
   the next pass starts from the latest architecture map.
