# Robot Sprite Component Tool

This package contains a green-screen component sheet for the cute robot character, a deterministic extraction workflow for turning approximate annotations into clean reusable rig parts, and a starter procedural assembler for packing those parts into fixed-canvas sprite sheets.

The intended workflow is:

1. Generate or edit a **rough YAML metadata file** with approximate boxes.
2. Run the **refinement step** to find the true foreground extent inside each rough box.
3. Run the **slicer** to crop each refined component and remove the green-screen background.
4. Build a **contact sheet** for visual QA.
5. Use `metadata/robot_components.refined.yaml` and `output/slices/*.png` in the procedural rig.
6. Run the **assembler** to compose extracted components into production sprite sheets and YAML manifests.

## Contents

- `assets/robot_components_greenscreen.png` - source component sheet with a green-screen background.
- `metadata/robot_components.rough.yaml` - human/AI-authored rough boxes, pivots, anchors, tags, rig notes, and generator instructions.
- `metadata/robot_components.refined.yaml` - generated refined metadata after foreground extent detection.
- `metadata/robot_components.schema.json` - metadata schema for rough/refined files.
- `tools/robot_asset_tool.py` - CLI for validation, refinement, slicing, listing, and contact-sheet creation.
- `tools/robot_rig_sheet.py` - CLI for assembling extracted parts into fixed-canvas animation frames and sprite sheets.
- `examples/robot_rig_job.yaml` - starter assembly job.
- `output/slices/*.png` - extracted transparent components.
- `output/slices/slices.index.json` - machine-readable index of extracted slices.
- `output/refinement_report.json` - per-sprite refinement diagnostics.
- `output/contact_sheet.png` - visual QA sheet rendered over a checkerboard.
- `output/assembled/robot_assembled_spritesheet.png` - starter assembled sprite sheet.
- `output/assembled/robot_assembled_spritesheet.yaml` - manifest for the starter assembled sheet.
- `examples/example_pose_frame.json` - sample pose-frame JSON.
- `prompts/generator_instructions.md` - prompt/authoring notes for future image-generation passes.
- `docs/metadata_workflow.md` - detailed extraction workflow notes.
- `docs/assembly_workflow.md` - compositor / sprite-sheet assembly notes.

## Install

```bash
python -m pip install -r requirements.txt
```

The extraction script uses Pillow, NumPy, PyYAML, and either OpenCV or SciPy for connected components. `opencv-python-headless` is listed in `requirements.txt`; the script also falls back to SciPy if OpenCV is unavailable. The assembler uses Pillow and PyYAML.

## Validate the rough metadata

```bash
python tools/robot_asset_tool.py validate metadata/robot_components.rough.yaml
```

## Refine rough boxes

```bash
python tools/robot_asset_tool.py refine metadata/robot_components.rough.yaml \
  --out metadata/robot_components.refined.yaml \
  --report output/refinement_report.json
```

The refiner estimates the green-screen color from the image border, builds a foreground mask, finds connected components in each rough search window, selects the target component(s), and writes corrected crop rectangles.

## Slice and remove green-screen background

```bash
python tools/robot_asset_tool.py slice metadata/robot_components.refined.yaml --out output/slices
```

The slicer writes transparent PNGs and `output/slices/slices.index.json`.

## One-command extraction rebuild

```bash
python tools/robot_asset_tool.py build metadata/robot_components.rough.yaml
```

This runs refine, slice, and contact-sheet generation.

## Assemble parts into a sprite sheet

```bash
python tools/robot_rig_sheet.py spritesheet \
  examples/robot_rig_job.yaml \
  output/assembled/robot_assembled_spritesheet.png
```

This writes both:

- `output/assembled/robot_assembled_spritesheet.png`
- `output/assembled/robot_assembled_spritesheet.yaml`

The assembler reads the refined metadata plus `output/slices/*.png`, generates fixed-canvas frames, lays them out row-by-row, and writes per-frame sheet coordinates and timing metadata.

## Render one assembled frame

```bash
python tools/robot_rig_sheet.py single \
  examples/robot_rig_job.yaml \
  output/assembled/hit_02.png \
  --animation hit \
  --frame-index 2
```

The single-frame command also writes `output/assembled/hit_02.json`.

## Full rebuild from rough boxes to assembled sheet

```bash
python tools/robot_asset_tool.py build metadata/robot_components.rough.yaml
python tools/robot_rig_sheet.py spritesheet examples/robot_rig_job.yaml output/assembled/robot_assembled_spritesheet.png
```

## Build the QA contact sheet

```bash
python tools/robot_asset_tool.py contact-sheet output/slices --out output/contact_sheet.png
```

## Why rough boxes first?

The generated sheet is useful visually, but exact sprite boxes are still an annotation problem. The rough YAML gives the tool a semantic target for each component, while the refinement step corrects pixel extents programmatically. This avoids two common failure modes:

- A rough box cuts off part of the target sprite.
- A rough box includes a sliver of a neighboring sprite.

The rough boxes should be generous enough to include the target component center, but not so large that a neighbor's center lies inside the same box. The refiner then shrinks to the actual non-green connected component extent and adds controlled padding.

## Animation intent reminders

- `hit` means recoil/stagger only. It should recover to idle/guard and must not collapse into death.
- `death` is a separate collapse/downed state.
- `teleport` replaces the old action-row use of blink. Blink remains available only as a face expression.

## Current assembler status

`tools/robot_rig_sheet.py` is a starter compositor, not a final animation polish pass. It proves the architecture: extracted transparent parts can be procedurally transformed, layered, packed into fixed cells, and exported with a YAML manifest. For production animation, tune `animation_pose()` or replace it with data-driven pose curves.


## Assembly updates in v6

The assembler now keeps labels, uses per-part scale corrections, stabilizes the root/ground anchor, separates run/dash effects, makes teleport actually vanish/reappear, keeps hit as recoil-only, and writes per-frame bounds QA into the spritesheet YAML manifest. The default job has zero near-edge QA warnings.

The latest pass bakes non-default face expressions into the selected head sprite before rotation. It detects the actual dark visor plate in `head_front`, tilted heads, and squash heads, repaints that visor in place, and transfers only cyan expression strokes from the expression sprites. This prevents black face plates or scanline overlays from floating away from the visor on hit/death/teleport/blink frames.

### v8 run/dash head-mount fix

The compositor now records explicit `head_mount` diagnostics per frame in the assembled YAML manifest.  Forward-lean poses such as `run` and `dash` apply calibrated head offsets and partial torso-rotation inheritance so the head is seated into the torso neck socket instead of floating above it.  The regression tests check that run/dash head targets are pushed down into the neck socket and that their world angles follow the torso lean.


### v9 chibi proportion and run/dash x-alignment pass

The latest compositor pass makes the robot read more cute/chibi by increasing the default head scale relative to the torso and shortening the effective limb scale.  Run and dash now use separate forward head offsets so the head target is pushed both down and forward on the x-axis relative to the forward-lean torso neck socket.  The default job uses 192x192 frame cells to preserve the larger head and maintain zero edge/crop QA warnings.


### v10 sheet-locked jump and jump head seating

The jump row is now sheet-locked: every jump frame keeps the same root/pivot position in the sprite cell.  The game object or collision box should move along the jump arc at runtime; the sheet itself should not bake vertical travel into the sprite pixels.  Jump poses now communicate anticipation, launch, airborne tuck, descent, and landing through limb/torso poses only.  The jump head mount was also retuned so forward-lean anticipation frames push the large head down and forward into the torso neck socket, while upright airborne frames keep a small downward seating offset.

## Current focused preview

The current `examples/robot_rig_job.yaml` intentionally renders only the `run` row while arm z-order and hand placement are being tuned.  The full multi-animation job is preserved as `examples/robot_rig_job_full.yaml`.

```bash
python tools/robot_rig_sheet.py spritesheet examples/robot_rig_job.yaml output/assembled/robot_run_arm_preview_v14.png
python tools/robot_rig_sheet.py spritesheet examples/robot_rig_job_full.yaml output/assembled/robot_assembled_spritesheet_full_v14.png
```

The v14 compositor pass updates global arm mounting and run-side separation rather than only the rendered preview, so later full-sheet renders inherit the same shoulder-seat, hand-follow, and z-order fixes.

### v13 component-placement debug pass

The assembler now has a flat-color diagnostic mode for tuning component placement without sprite-art noise:

```bash
python tools/robot_rig_sheet.py debug-spritesheet \
  examples/robot_rig_job.yaml \
  output/assembled/robot_run_component_debug_v14.png
```

Debug colors:

- blue: head
- yellow: torso
- green: back arm / back hand
- red: front arm / front hand
- purple: back leg
- orange: front leg
- cyan: effects

Each pasted component also gets an anchor marker at the exact target used by the compositor. Debug sheets omit all text by default; use `--keep-sheet-labels` only when a labeled screenshot is needed. The debug manifest is written next to the PNG with the same per-frame `arm_mounts`, `head_mount`, root, bbox, and QA data as the production render.

This pass also changes the global shoulder-seat offsets so arms stay attached to the side pods instead of collapsing inward across the tiny torso, and moves the back arm chain behind the torso to reduce visual clutter. The run arm swing is intentionally more compact; the leg cycle and small speed streaks carry the locomotion while the hands stay attached and readable.

### v14 side-separated run placement pass

Run placement was updated so component anchors stay on their intended sides: front/right arms and legs rotate outward from the visible side pod, while back/left limbs stay behind and to the left. This avoids the earlier pile-up where limbs crossed through the torso. The no-text debug view is now the primary tool for verifying these component placements.

## v26 manual anchor editor

The current run tuning showed that hand-written/YAML anchors are the fragile part
of the workflow.  This package now includes a small Tk GUI for placing
component-local pivots and anchors directly on the extracted sprites.

```bash
python tools/anchor_editor.py metadata/robot_components.refined.yaml \
  --slices output/slices \
  --rough-metadata metadata/robot_components.rough.yaml \
  --zoom 6 \
  --background checker
```

Use the GUI to select a component, select `pivot` or a named anchor, then click
or drag the anchor to the desired pixel.  Arrow keys nudge the selected point by
one pixel.  Press `Ctrl+S` or the **Save** button to write the updated metadata.

The editor now supports **pivot follows anchor**.  Select a named anchor and press
**Use selected as pivot** (`Ctrl+P`), or pick that anchor in the `pivot follows`
dropdown.  This stores `pivot_anchor: <anchor-name>` and keeps the numeric
`pivot` synchronized for compatibility.  This avoids the overlap problem where
you could not click the pivot and an anchor into exactly the same pixel.

The right pane is now a larger scrollable live preview. It renders the configured
spritesheet after each edit from the current unsaved in-memory metadata, not from
the last saved YAML file. By default it previews `examples/robot_rig_job.yaml`
(the focused run row). Use `--preview-config examples/robot_rig_job_full.yaml`
for the full sheet, `Ctrl+R` to force refresh, or `--no-live-preview` /
`--manual-preview` on slow machines. The preview controls include enable/live
toggles, fit/native display, background, and max preview raster size. Native mode
keeps the rendered sheet at full size and uses scrollbars.
When `--rough-metadata` is supplied, the editor also writes equivalent rough-local
anchor positions so a future green-screen refinement pass preserves the manual
edits instead of reverting them.

A headless JSON inspection mode is available for CI or quick review:

```bash
python tools/anchor_editor.py metadata/robot_components.refined.yaml \
  --slices output/slices \
  --anchor-report output/anchor_report.json \
  --sprites torso_lean_forward leg_bent_right leg_bent_left hand_fist

python tools/anchor_editor.py metadata/robot_components.refined.yaml \
  --slices output/slices \
  --preview-config examples/robot_rig_job.yaml \
  --render-preview output/anchor_editor_preview.png
```

Recommended anchor workflow now becomes:

```text
rough YAML boxes -> programmatic crop refinement -> manual anchor edit GUI ->
component anchor QA sheet -> run/debug-frame preview -> final spritesheet
```


## Integrated rig / pose editor

Use the integrated PySide6 editor when you need to tune how component anchors affect actual animations.  The stable command is:

```bash
python tools/rig_pose_editor.py examples/robot_rig_job.yaml
```

That direct script now delegates to `tools/rig_pose_editor_pyside.py`, so older instructions no longer accidentally launch the stale Tk editor.

It shows an animation/frame/part tree, the selected component's local anchors, a live highlighted spritesheet preview, a per-frame z-order / visibility editor, per-instance art/rotation/endpoint controls, frame-count/timing controls, and an animated action preview with play/pause, previous/next, and a frame slider. It edits component metadata plus `metadata/robot_pose_overrides.yaml`, so one art asset can be reused by multiple logical instances such as `front_arm` and `back_arm` with different transforms.

Pose overrides are sparse keyframes. Numeric fields and 2D coordinate fields interpolate between authored frames, so setting frame 0 and the last frame gives a useful coarse pass for everything in between. Add a middle-frame edit only when you need more detail. Visibility toggles are stored as `hidden_parts` / `visible_parts` and are honored by both previews and exported sheets.

Useful non-GUI checks:

```bash
python tools/rig_pose_editor.py examples/robot_rig_job.yaml --render-preview output/assembled/rig_pose_editor_preview.png
python tools/rig_pose_editor.py examples/robot_rig_job.yaml --anchor-report output/assembled/rig_pose_editor_instances.json
```

## PySide6 rig pose editor

The primary interactive editor is now the PySide6 rig pose editor. Either command is valid:

```bash
python tools/rig_pose_editor.py examples/robot_rig_job.yaml
python tools/rig_pose_editor_pyside.py examples/robot_rig_job.yaml
```

This uses PySide6 rather than PyQt6 to avoid the PyQt licensing concern while keeping the Qt widget stack and API style. The old `tools/rig_pose_editor_qt.py` command remains as a compatibility shim that delegates to the PySide6 implementation.

Install GUI dependencies with:

```bash
pip install -r requirements.txt
```

The PySide6 editor replaces the Tk layout for the active rig-authoring workflow. It provides native draggable splitters, scrollable graphics views, an animation/frame/part tree, component-local anchor editing, logical part-instance editing, checkable z-order / visibility rows, frame count / frame-duration controls, relevant-animation filtering, highlighted spritesheet preview, and an animated action preview with play/pause plus timeline stepping. It renders from the current unsaved in-memory metadata and pose overrides, and every GUI edit queues an immediate preview refresh.

Headless CI/debug modes do not require PySide6:

```bash
python tools/rig_pose_editor_pyside.py examples/robot_rig_job.yaml \
  --render-preview output/assembled/rig_pose_editor_pyside_preview.png \
  --animations run

python tools/rig_pose_editor_pyside.py examples/robot_rig_job.yaml \
  --anchor-report output/assembled/rig_pose_editor_pyside_instances.json
```

The old Tk editor remains available only as a fallback via the console entrypoint:

```bash
robot-rig-pose-editor-tk-legacy examples/robot_rig_job.yaml
```

## v35 rig-constraint editor pass

The PySide6 pose editor now has a **Rig joint constraints for current frame**
panel.  For each rendered part it shows the semantic joint, child sprite anchor,
parent socket, visibility state, and snap error in pixels.  Use this panel to
answer questions like “does `front_arm.shoulder` actually snap to
`torso.shoulder_right`?” without inferring the relationship from colors or role
names.

The intended authoring policy is standard skeletal-rig style: keep component
anchors/sockets stable in local art space, then animate bones/endpoints, draw
order, visibility, and sparse keyframes.  Per-frame parent/child joint deltas
remain possible through advanced socket-offset fields, but they should be rare
corrective overrides rather than the default animation workflow.


## Scale editing policy

Scale defaults are intentionally all 1.0 for every logical instance. The renderer may report an effective solved scale for endpoint-constrained arms or legs, but the GUI does not write that solved value back into pose data unless the scale control itself is edited. Frame-level scale overrides are exact-frame corrective edits; use top-level or animation `defaults` for broad scale calibration.


## Canonical atlas update

- `metadata/robot_components.refined.yaml` now points to the new cute/stubby modular sheet.
- Legacy atlas metadata was preserved as `metadata/robot_components_legacy.refined.yaml`.
- Legacy extracted slices were preserved in `output/slices_legacy/`.

## SkelForm export

Export one editable pose to SkelForm:

```bash
python tools/skelform_export.py examples/robot_rig_job.yaml output/skelform/robot_pose_run_000.skf --animation run --frame-index 0
```

The exporter clips textures from the green-screen atlas using a threshold alpha ramp and writes a `.skf` package containing `armature.json`, `editor.json`, `atlas0.png`, `thumbnail.png`, and `readme.md`.
