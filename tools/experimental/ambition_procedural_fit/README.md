# Ambition Procedural Fit Experiment

Experimental tool for matching a procedural shape template to a pre-rendered
reference image or concept-art crop.

The goal is not to reproduce a Stable Diffusion image exactly. The goal is to
recover a compact, editable, deterministic approximation: rectangles, ellipses,
superellipses, and line segments with normalized positions, sizes, colors, and
alpha. Those optimized parameters can then be ported back into the normal
PIL-style sprite and background generators.

## Why this is feasible

This is feasible as an art-direction and reverse-engineering aid, with one caveat:
the complete problem is not globally convex. Once primitive positions, sizes,
rotations, and draw order are variables, the loss has many local minima. The
practical compromise is:

1. Render shapes with a differentiable **soft rasterizer** instead of hard PIL
   masks. Edges are sigmoid falloffs, so gradient descent can move shapes toward
   the target instead of getting zero gradients at hard edges.
2. Use a smooth multi-scale loss: pixel MSE, downsampled pyramid MSE, Sobel edge
   loss, Laplacian detail loss, and coarse color-statistics loss.
3. Keep primitives mostly convex: rectangles, ellipses, superellipses, and
   segments. Complex architecture is approximated as many overlapping
   convex-ish pieces. Superellipses are especially useful for geometric sprites
   because they can smoothly morph between ellipse-like and rectangle-like fits.
4. Seed from the image with `init-template`, then refine with Adam. Use a few
   restarts when the local optimum is ugly.
5. Treat color optimization as the easy part and geometry as the hard part. For
   fixed geometry and alpha ordering, color matching is close to least squares;
   moving shapes is where the non-convexity lives.

This makes the tool useful for finding a good procedural layout, not for creating
a perfect vector trace.

## Install

From this tool directory:

```bash
python -m venv .venv
. .venv/bin/activate
pip install -e .
```

PyTorch is intentionally a dependency. CPU is fine for small 128-256 px fitting
runs; CUDA helps for larger or many-restart experiments. The CLI defaults to
`--threads 4` because tiny differentiable renders are often slower with very high
PyTorch thread counts.

## Quick start

Use a crop from a concept sheet, not a whole multi-panel board. A 256-512 px crop
of one candidate image is ideal.

```bash
cd tools/experimental/ambition_procedural_fit
python -m ambition_procedural_fit crop \
    --image /path/to/style_candidates.png \
    --box 0.46,0.48,0.76,0.88 \
    --out generated/collision_box_crop.png

python -m ambition_procedural_fit init-template \
    --target /path/to/concept_crop.png \
    --out generated/concept_seed.yaml \
    --size 192 \
    --rects 48 \
    --ellipses 10 \
    --superellipses 12 \
    --segments 16

python -m ambition_procedural_fit fit \
    --target /path/to/concept_crop.png \
    --template generated/concept_seed.yaml \
    --out-dir generated/concept_fit \
    --steps 500 \
    --lr 0.04 \
    --size 192 \
    --restarts 3 \
    --mode background \
    --save-debug \
    --debug-every 50 \
    --threads 4
```

Outputs:

- `optimized_template.yaml` - fitted primitive parameters.
- `optimized_render.png` - differentiable render of the fitted template.
- `comparison.png` - target / render / amplified absolute difference.
- `loss_curve.png` and `loss_curve.csv` - optimization trace.
- `metrics.json` - run metadata.

If `--save-debug` is enabled, the output directory will also contain structured per-restart folders so you can inspect the optimization without relying on the GIF alone:

- `debug/restart_001/renders/`
- `debug/restart_001/comparisons/`
- `debug/restart_001/templates/`
- `debug/restart_001/optimization.gif`

The `renders/` and `comparisons/` folders are ordered by step number so you can flip through them directly in a file browser like an animation sequence.

You can render any template without optimizing:

```bash
python -m ambition_procedural_fit render \
    --template examples/mathematical_fantasy_seed.yaml \
    --out generated/seed_preview.png \
    --width 384 \
    --height 384
```

## Template schema

Coordinates are normalized to `[0, 1]`. Colors are RGBA floats in `[0, 1]`.
Supported primitive kinds are:

- `rect`: `xy`, `wh`, `angle`, `color`
- `ellipse`: `xy`, `wh`, `angle`, `color`
- `superellipse`: `xy`, `wh`, `angle`, `exponent`, `color`
- `segment`: `p0`, `p1`, `width`, `color`

Each primitive has a `train` list. Only the listed fields are optimized. This is
important: locking background color or anchor shapes can make the fitting much
more stable.

## Notes for integrating back into sprite/background generators

- This experiment uses soft PyTorch rendering so the optimizer has gradients.
  Runtime art can still be drawn with PIL using hard-edged or antialiased masks.
- `optimized_template.yaml` is deliberately simple. A follow-up bridge should
  translate these primitives into whichever generator owns the final art target.
- For background art, start with broad locked masses and train details later.
- For character sprites, fit a single canonical frame first, then reuse the
  optimized proportions in the rig/spritesheet generator.

## Current limitations

- No automatic draw-order optimization.
- No Bezier curves, polygons, text, strokes, or procedural ornament fields yet.
- No perceptual model loss by default. LPIPS / CLIP / DINO could be added later,
  but the current loss is deterministic and lightweight enough for CI smoke tests.
- Optimization can converge to a plausible but wrong local minimum. Use better
  seeds, locked anchors, and restarts.


For crisp character/sprite fitting, use the sprite profile and slightly higher
primitive counts, especially segments and ellipses:

```bash
python -m ambition_procedural_fit init-template     --target ./test-img.png     --out generated/test_seed.yaml     --size 192     --rects 28     --ellipses 16     --segments 24

python -m ambition_procedural_fit fit     --target ./test-img.png     --template generated/test_seed.yaml     --out-dir generated/test_fit     --steps 600     --lr 0.03     --size 192     --mode sprite     --sharpness-start 32     --sharpness-end 240     --restarts 4     --save-debug     --debug-every 40
```

`--mode sprite` increases edge/detail loss and anneals sharpness from soft to
hard so optimization can first find the layout and then snap to crisper lines.
The `--save-debug` flag writes intermediate renders and comparison sheets under
`out-dir/debug/restart_*/`, plus an `optimization.gif` for each restart.


## Future work for articulated sprites

For backgrounds, the main goal is a compact, regeneratable manifest that visually matches the source image. For sprites, the higher-value direction is different: primitives should be grouped into semantically meaningful parts (head, torso, upper arm, forearm, weapon, etc.) so they can attach to a bone rig. A natural next step is to extend the manifest schema with `parts`, `parent`, `pivot`, and `bone` fields, then fit each part in a canonical pose before handing the result to the articulation system.


## Superellipse note

A `superellipse` is a morphable primitive controlled by an `exponent` parameter.
At `exponent ~= 2` it behaves like an ellipse, and as the exponent increases it
becomes progressively more rectangle-like. This gives the optimizer a smooth way
to move between rounded and boxy geometry instead of forcing a brittle early
choice between a hard `rect` seed and a hard `ellipse` seed.
