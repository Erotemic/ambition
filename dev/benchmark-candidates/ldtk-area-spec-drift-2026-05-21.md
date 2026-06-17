# LDtk area-spec world-coordinate drift

## Q: Before `area create --replace-existing` on a historical area spec, what should you reconcile first?

### Context

`tools/ambition_ldtk_tools area create <spec.yaml> --ldtk <file.ldtk> --replace-existing` rebuilds an existing LDtk level from a YAML spec. The spec carries `world_x` / `world_y` / `px_wid` / `px_hei`, and the tool writes those values into the level body when authoring.

In long-lived repos the live LDtk file gets edited / moved over time. The spec under `tools/ambition_ldtk_tools/specs/` is a rebuildable source — if no one re-applies it after a layout change, the spec drifts from the live LDtk.

Real example caught during intro-v1 Task 02 (2026-05-21):

```text
intro_escape_shaft_area.yaml  world_x: 104000   px_wid: 1280   px_hei: 512
intro.ldtk  intro_escape_shaft  worldX: 2624   pxWid: 1280   pxHei: 512
```

All five intro specs had this drift (`100000 / 102000 / 104000 / 106000 / 108000` vs live `0 / 1024 / 2624 / 3904 / 4928`). A prior repo refactor had moved the intro levels in the live LDtk without re-applying the specs.

### Failure mode

If you run `area create --replace-existing` on a drifted spec, the tool:

1. Finds the existing level by `level_id` and wipes it.
2. Rebuilds at the spec's `world_x` / `world_y` (the stale value).
3. Repairs and validates the file.

Nothing fails: validate and tests still pass. The runtime keeps using the level by id, so gameplay still works. But the LDtk editor view now has the level at the stale coordinate, far from the intended row. Adjacent operations (e.g. trying to add a new level at the original coordinate) silently overlap or produce confusing layout errors.

### Distractor moves that look reasonable

- **Trust the spec's `world_x`** because the tool default uses it. Bad — spec is rebuildable, live is canon.
- **Compare only `px_wid` / `px_hei`** between spec and live. Misses coordinate drift entirely.
- **Re-apply the spec and "see what happens"**. The bad state isn't loud; you find out hours later when a different room overlaps.

### Correct move

Diff `world_x` / `world_y` between the spec and the live LDtk before applying `--replace-existing`:

```bash
python3 - <<'EOF'
import json
d = json.load(open("crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk"))
for lvl in d["levels"]:
    print(f"  {lvl['identifier']}: {lvl['pxWid']}x{lvl['pxHei']} at ({lvl['worldX']},{lvl['worldY']})")
EOF
```

Then either:

a) Update the spec's `world_x` / `world_y` to match the live coordinates before re-applying.

b) Acknowledge the spec is no longer the source of truth for layout and use `--replace-existing` on a freshly-corrected spec.

### Adjacent gotchas in the same family

- `doctor` did not forward `--secondary-world`, so it false-positived on cross-world LoadingZones. Fixed by adding the flag to `roundtrip`'s argparse so `doctor` can delegate `rest` to both wrapped tools consistently.
- `tileset add-layer` errored when the layer def existed instead of backfilling missing per-level instances. After `area create --replace-existing` the new level was missing the Tiles layer instance and the `intro_levels_carry_painted_tileset_layers` regression failed. Fixed by making the existing-def branch call the same idempotent `add_empty_layer_instance_to_levels` backfill.

General rule: **tools that emit per-level shapes (Tiles instances, lock-wall blocks, …) should be idempotent** so `--replace-existing` is recoverable without hand-edits.
