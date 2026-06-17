# Unified Tabbed Menu — COMPLETE (2026-06-08)

The unified menu landed: one content model, two renderers (cube + grid) in the
`ambition_menu` crate, now unconditional (the `oot_inventory` flag and the old
`pause_menu` / `inventory::ui` / `bevy_ui_grid_menu` paths are gone).

- **Current menu ownership & systems:** `docs/systems/ui-navigation-and-pause.md`.
- **Active cleanup candidates:** `docs/planning/refactor-candidates.md`.

The detailed phase-by-phase execution log and the settings-IR coverage diff were
pruned — they were forensic notes for a completed design process; the outcome is
in the code and git history.
