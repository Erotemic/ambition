# YAML-driven Mockingbird boss sprite rig

This overlay changes the generator/editor workflow so the rig can be iterated visually instead of hardcoding positions in Python.

## Files

- `mockingbird_boss_parts.yaml`: source of truth for parts, parent connections, transforms, z order, primitive shapes, and animation metadata
- `mockingbird_boss_sprite_generator.py`: consumes the YAML and generates canonical renders, spritesheets, debug images, and manifests
- `mockingbird_boss_part_editor.py`: PySide6 editor for moving, rotating, scaling, z-ordering, and shape-editing the YAML rig

## Important behavior

The generator no longer clips to a fixed 256x256 work canvas. It renders on an oversized work canvas, crops to the alpha extent of all rendered parts, and then fits that into the requested output frame. The sprite frame is still a fixed size for game integration, but the bounds are derived from the full part extent first.

## Install dependencies

```bash
python -m pip install pillow pyyaml pyside6
```

## Render

```bash
python tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py render --force
```

Outputs are written to:

```text
tools/ambition_sprite2d_renderer/generated/mockingbird_boss/
```

## Open the editor

```bash
python tools/ambition_sprite2d_renderer/mockingbird_boss_part_editor.py
```

## Editor controls

Part-level editing:

- click a part in the canvas or in the part list to select it
- drag inside the selected bounding box to move it relative to its parent
- drag the corner handles to scale it
- drag the circular handle above the bounding box to rotate it
- right-drag or mouse wheel also rotates
- arrow keys nudge the selected part
- `[` and `]` rotate the selected part
- `Move Z Up` / `Move Z Down` changes render order in YAML

Focus / subcomponent editing:

- enable `Focus selected part` to isolate one part
- choose an item in the shape list
- in focus mode, drag the canvas to move the selected shape inside its parent part
- edit the selected primitive directly in the YAML text box and press `Apply Shape YAML`
- `Shape Up` / `Shape Down` changes primitive order inside the part

## Render + publish

```bash
python tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py render-publish --force
```

## Inspiration links

- https://archive.org/download/htkam/TKAM%28www.albinoblacksheep.com%29.swf
- https://archive.org/download/how-to-kill-a-mockingbird/how-to-kill-a-mockingbird.swf


## Higher-resolution editor mode

The editor now has two controls in the side panel:

- `Editor canvas`: medium / high / ultra work-canvas sizes
- `AA quality`: supersampling factor from 1 to 4

Recommended defaults:

- `high 1400x1000`
- `AA quality = 2`

For final polish, switch to `ultra 2000x1400` and `AA quality = 3` or `4`. That will be slower while dragging, but it gives much cleaner part boundaries and handles. The renderer still uses vector/YAML geometry, so this does not change the saved YAML coordinates; it only changes the editor preview resolution.


## v4 shape-editing notes

This overlay includes the YAML you pasted as the default `mockingbird_boss_parts.yaml`.

Important renderer/editor changes:

- Part scale is no longer inherited by child parts.
  - Scaling `body.scale_x` widens only the body geometry.
  - Children such as wings, head, legs, ribs, etc. keep their own transforms.
  - Parent translation and rotation still affect children, so the rig remains connected.
- Every primitive/component shape can now have its own YAML `transform`.
  - Example: `transform: {x: 1, y: -2, rotation: 12, scale_x: 1.2, scale_y: 0.8}`
  - This transform is applied to vector geometry before rasterization, not as a post-render image transform.
- In the editor:
  - enable `Focus selected part`
  - click a sub-shape inside the isolated part to select it
  - use the same move / rotate / scale handles to edit that sub-shape
  - edits are written into the selected shape's YAML `transform`
  - the shape YAML text box updates with those transform edits
- Shape bounds are drawn in cyan in focus mode.
- Part bounds are still drawn in magenta.

This is intended to behave more like an SVG editor: transforms are stored in the rig and reapplied during generation, avoiding repeated raster resampling / aliasing artifacts.


## v5 zoom + independent teeth rows

This overlay adds:

- Mouse wheel zooms the editor view around the cursor.
- Ctrl+mouse-wheel rotates the selected part or selected shape.
- Middle-drag, or Alt+left-drag, pans the editor viewport.
- `Reset View` resets zoom / pan.
- The combined `teeth` primitive has been split into two independent `teeth_row` shapes in the head:
  - upper teeth row
  - lower teeth row

Each teeth row has its own shape-level transform, so in `Focus selected part` mode you can click the upper or lower row separately and move / rotate / scale it without affecting the other row.

Renderer note: both part and shape transforms are applied to vector geometry before rasterization, not to already-rendered pixels.


## v6 zoom persistence fix

The editor viewport now stores zoom as an image-space `view_center` plus a scale factor. This keeps zoom stable across refreshes, part selection, dragging, and shape edits.

- Mouse wheel: zoom around cursor
- Ctrl + mouse wheel: rotate selected part/shape
- Middle drag or Alt + left drag: pan
- Side panel `Zoom` label shows the current zoom percentage
- `Reset View` returns to 100% centered view


## v7 component z-order editing

The editor now exposes component (shape) z-order more explicitly:

- The shape list is ordered **back to front**.
- Each row shows `z=<index>` so you can see component draw order directly.
- You can change component z-order in three ways:
  - `Component Backward` / `Component Forward` buttons
  - drag and drop rows inside the shape list
  - click a shape in focus mode, then reorder it in the list
- The selected component z-order is shown under the list.

For shapes inside a part, z-order is simply the order of that part's `shapes:` list in YAML. Earlier items draw behind later items.
