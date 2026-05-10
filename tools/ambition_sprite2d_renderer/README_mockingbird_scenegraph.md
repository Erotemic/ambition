# Mockingbird boss nested scene-graph rig

This overlay refines the scene graph into a stricter, logically grouped tree and adds group-level editing behavior to the PySide6 editor.

## Files

- `mockingbird_boss_scene.yaml` — source-of-truth nested scene schema
- `mockingbird_boss_parts.yaml` — compatibility copy of the same schema
- `mockingbird_boss_legacy_parts.yaml` — the uploaded legacy YAML preserved for reference
- `mockingbird_boss_sprite_generator.py` — vector-geometry renderer
- `mockingbird_boss_part_editor.py` — PySide6 scene graph editor

## Schema organization

Every node is one of:

- `kind: group`
- `kind: shape`

Both groups and shapes have:

- `id`
- `label`
- `visible`
- `locked`
- integer `z_order`
- `transform`

Shape nodes also have:

- `primitive`

Groups can contain shapes or other groups recursively. Render order is local: children are drawn in ascending `z_order`, so higher values are closer to the front.

The current rig avoids loose anonymous-looking shapes by grouping primitives by role:

- body hull core
- body sensor ports
- body lower red bay
- dorsal spikes
- tail boom, fins, flame
- head teeth, skull armor, lower jaw, eye
- left/right legs and foreclaws
- left/right rotor masts, crossbrace, left/right rotor discs

## Important fixes in this overlay

- Added Google-Slides-like group editing: select a group and press `Enter Group`, or double-click a group on the canvas. Canvas picking then edits only that group's immediate children until you press `Up Level` or `Root Level`.
- The tree still shows the whole scene, but the canvas selection model now respects the active edit level instead of selecting arbitrary deep descendants.
- Duplicate now deep-copies an entire group subtree, recursively renames IDs, inserts the copy next to the original, and offsets it by `(26, 18)` so it is easy to drag.
- Local z order remains an integer `z_order` property in the UI.
- The global post-composite outline is disabled by default with `render.global_outline: false`; outlines now come only from explicit shape primitives.
- Light rib/highlight strokes were muted so they do not read as new white outlines.
- Rotor rendering was changed from hard blade polygons to translucent spinning ellipses with a specular shine and hub.
- Rendering logs are available through `--log-level`.
- Default rendering is lower resolution for speed: 256x256 sheet frames, 512x512 canonical.
- Use `--quick` to render canonical/debug only.
- The steel plate color remains opaque: `steel_plate: [170, 184, 198, 255]`.
- All transforms are applied to vector geometry before rasterization.

## Install deps

```bash
python -m pip install pillow pyyaml pyside6
```

## Render quick with logs

```bash
python tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py \
    --log-level INFO \
    render \
    --quick \
    --force
```

## Render full sheet with logs

```bash
python tools/ambition_sprite2d_renderer/mockingbird_boss_sprite_generator.py \
    --log-level INFO \
    render \
    --force
```

## Open editor

```bash
python tools/ambition_sprite2d_renderer/mockingbird_boss_part_editor.py
```

## Editor controls

- Select a group in the tree or on the canvas.
- Press `Enter Group` or double-click the group to edit its immediate children.
- Press `Up Level` to leave the current group, or `Root Level` to return to the top.
- Drag selected node on the canvas to move it.
- Right-drag or Ctrl+wheel rotates the selected node.
- Mouse wheel zooms.
- Middle drag or Alt+left-drag pans.
- Add/remove/duplicate groups and shapes.
- Edit a shape's primitive YAML in place.

## Inspiration links

- https://archive.org/download/htkam/TKAM%28www.albinoblacksheep.com%29.swf
- https://archive.org/download/how-to-kill-a-mockingbird/how-to-kill-a-mockingbird.swf


## Focus / background / panel updates

- Added editable `render.background_rgba` support used by the editor canvas and canonical/debug renders.
- Added **Focus Node** / **Clear Focus**: the canvas can isolate the selected node subtree so only that node and its descendants are visible.
- The scene tree panel now starts wider and is placed on the left side for readability.
- The rotor rendering was softened to reduce bright white haloing and to read more like a spinning ellipse.


## navigation and outline updates

- Most outlines are intended to be dark by default. Light/white outlines should be used only where they are intentional, such as teeth highlights.
- Double-clicking empty canvas background selects the scene root, returns to root edit level, and clears focus.
- Escape moves up one level: first out of the current group edit level, then to the selected node's parent.
- Canvas picking now considers every currently visible rendered node, not only the current edit group's immediate children. Hidden nodes and nodes outside Focus mode remain unpickable.


## transform workflow update

- Added a checkable **Transform Node** button.
- In selection mode, canvas clicks only select nodes.
- In Transform Node mode:
  - drag the selected node to move it
  - drag corner handles to scale it
  - drag the round handle above it to rotate it
  - Ctrl + mouse wheel rotates the selected node
- Canvas picking now coerces deep hits to the current edit level. For example, at root level clicking an individual rib selects the ribs group; after entering the ribs group, clicking a rib selects that rib.
- Visible nodes outside the current edit group can still be selected, but they are selected at the current edit depth. Hidden or focused-out nodes remain unpickable.


## dock and transform bugfix update

- Fixed Transform Node dragging by using the existing canvas image_delta() conversion helper.
- Made the left side panel wider by default.
- Split the left side panel vertically so the tree can be resized independently from the property editor.
- Added Expand Tree and Collapse Tree controls.
- Selection now expands ancestors and scrolls the tree to the selected item.

## transform / visibility / layout pass

- Default background is now `43, 33, 40, 255`.
- The scene tree has an eye/visibility checkbox column. Toggling it updates `visible` for that node and refreshes the canvas.
- Transform Node mode now locks canvas selection. While active, clicking other visible nodes will not change selection.
- In Transform Node mode, left-dragging anywhere in the canvas translates the selected node by default; handles still scale/rotate.
- Escape exits Transform Node mode first. If transform mode is already off, Escape keeps the previous up-one-level behavior.
- Rear wing group no longer stores a negative `scale_y`; the mirror is pushed down onto the child primitive so the selectable group has a more stable pivot.
- Ribs remain grouped under the `ribs` node, and the rib primitives were respaced/aligned for easier group translation.

## animation fx pass

- The sprite sheet now uses vector-space per-node animation transforms instead of duplicating static frames.
- Living animations all get baseline hover motion: root/body bobbing, subtle head/wing/leg/foreclaw offsets, rotor strobing, and cyan flame flicker.
- Attack rows add extra motion: `bite` opens the lower jaw and lunges the head, `slash` sweeps the foreclaws, `thrust` stretches the flame and pitches the body, and `hit` shakes the rig.
- `death` collapses the body, droops wings/claws/legs, stops the flame, slows/fades the rotors, and draws a large X over the eye.
