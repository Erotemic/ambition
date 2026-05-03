# gen3d blender lab design

This generator is Blender-first. There is no parallel PIL character renderer.
Pillow is allowed only after Blender has rendered character frames, for packing,
labeling, and contact sheets.

## Path policy

The package is designed to live in `tools/generators/gen3d`. All default paths
resolve relative to the module root, not the repository root or current working
directory.

```text
GEN3D_BLENDER_LAB_ROOT or Path(gen3d_blender_lab.__file__).parents[1]
```

Defaults:

```text
<root>/gen3d_blender_lab/configs
<root>/assets
```

## Render policy

- Blender builds 3D character components.
- A fixed side-scroller-friendly orthographic camera renders frames.
- Cel shading is implemented in Blender materials.
- Sprite sheet composition is post-processing only.

## Iteration loop

1. Render canonicals with `gen3dlab canonical-all`.
2. Tune camera, proportions, and primitive construction in the Blender backend.
3. Render full sheets with `gen3dlab draw-all`.
