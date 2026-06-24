# Perfect Cellular Automaton — polygon-fit (experimental)

Fits a flat polygon sprite sheet to the concept art
`assets/concept_art/prefect-cellular-automaton-reference-image.png`, as a
candidate input for the real sprite generator.

> **Status: experimental / not wired into the generator.** Hooking the fitted
> geometry into a tack-on target is gated on Jon's sign-off.

## Provenance

The `*_v14.*` files are ChatGPT's hand-authored polygon soup + fit harness,
imported verbatim (commit "import ChatGPT v14 …"). It plateaued at **mean
IoU ≈ 0.80** because the geometry was being nudged by hand.

## What the auto-fit adds

| File | Role |
|---|---|
| `pca_fit.py` | Flatten v14 polygons → absolute-coord list; faithful renderer (pixel-identical to v14, see `--selftest`); FG-aware loss; staged descent (global affine → per-polygon → per-vertex). Also applies `PALETTE_FIX` and renders the `motif_segments` grid. |
| `pca_seg.py` | Flood-fill foreground mask: background = border-connected bg-coloured pixels, so the **dark helmet/forehead counts as foreground**. |
| `pca_detect_spots.py` | Connected-component detector for the dark-green carapace spots (top_back / top_side), emitted as motif rects. |
| `pca_finalize.py` | Stamps detected static detail onto a fitted sheet as a locked layer. |
| `pca_inspect.py` | Builds a `[target | candidate | overlay]` grid for eyeballing. |
| `pca_pipeline.py` | End-to-end driver (v14 → fit → finalize → sheet + IoU). |

Three insights broke the plateau:

1. **Palette black was the background.** v14 `black` = (23,24,24) ≈ backdrop
   (25,26,28), so the helmet was invisible and filling it barely changed the
   loss. `PALETTE_FIX` darkens it to (13,15,15). (`pca_fit.PALETTE_FIX`)
2. **The grids/cells were already detected but never drawn.** ChatGPT's
   `motif_segments` (abdomen grid + forehead cells, as axis-aligned rects)
   are now rendered as a **locked** layer — crisp squares the optimizer moves
   rigidly but never distorts. This fixes the missing/“not-square” belly grids
   (incl. air & jump) and adds forehead cells.
3. **FG-aware loss.** masked RGB-L1 **+** a coverage term scoring the
   candidate's own foreground vs the reference foreground, so under-fill (dark
   helmet) and over-fill (stray slivers in the backdrop) are both penalised —
   the optimizer can't game IoU with background-coloured noise.

## Results (harness IoU vs reference, higher is better)

```
pose          IoU_v14  IoU_v15   delta
top_front       0.839    0.931  +0.093
top_side        0.846    0.921  +0.074
top_back        0.871    0.938  +0.067
pose_idle       0.784    0.902  +0.118
pose_walk_1     0.773    0.889  +0.116
pose_walk_2     0.776    0.881  +0.105
pose_attack     0.783    0.881  +0.098
pose_jump       0.762    0.873  +0.112
pose_air        0.774    0.876  +0.102
pose_land       0.768    0.898  +0.130
MEAN            0.797    0.899  +0.101
```

RGB-diff inside the overlap roughly halved too (real, not noise).

## Run

```bash
# full run (~20 min); writes to the gitignored scratch dir
python pca_pipeline.py --out-dir ../../../agent-scratch/run

# single pose
python pca_pipeline.py --out-dir ../../../agent-scratch/run --poses pose_jump

# prove the flat renderer matches v14 exactly
python pca_fit.py --selftest <path-to-v14-sheet.png>
```

## Known remaining work

- **Eyes** (top_front / top_side) are still the v14 manual placement; auto-detect
  over-fired (helmet seams), so eye micro-placement is left as a manual nudge.
- Back/side spots render as squares vs the reference's hexes (stylistic).
- Optimizer is slow (~2 min/pose); the per-vertex stage dominates and could be
  downscaled.
