# PCA polygon-sprite handoff

You are continuing work on a **polygon sprite sheet** for the *Perfect Cellular
Automaton* (a Cell-inspired robot). The job: author a clean, semantic, paper-doll
polygon character that matches the concept-art reference for all 10 poses, good
enough to eventually feed the sprite generator. **Read this whole file before
touching anything.** The most important section is "Jon's principles" — they are
hard constraints, learned the hard way over a long session.

## What / where

- Reference (RGB, 1448x1086): `assets/concept_art/prefect-cellular-automaton-reference-image.png`
- Jon's manual segmentation (RGBA, transparent bg, *imperfect*, 1920x1086):
  `assets/concept_art/pca-segment.png`
- All code + this file: `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/experimental/perfect_cellular_automaton/`
- **Run python with the venv** (has cv2/scipy/numpy/PIL):
  `tools/ambition_sprite2d_renderer/.venv/bin/python`. Scripts `import pca_*`
  modules by being run from that dir or with it on `sys.path`.
- Scratch (gitignored, all outputs go here): `tools/ambition_sprite2d_renderer/agent-scratch/`
- **NOT hooked into the generator yet — gated on Jon's explicit sign-off.** Do not
  wire it in until he says so.

## Pipeline (current best version = `10_anchored`)

1. `pca_crops.py` — from `pca-segment.png`: alpha>=127, drop <100px noise
   components, split the 10 figures by 2-row layout, **mask-crop** each (their
   AABBs overlap) -> `inputs/refs/<pose>.png` (the per-pose EVAL TARGETS).
2. `pca_vectorize.py` — k-means a 7-colour palette over all crops; `quantize()` is
   reused everywhere. (Legacy: it can also vectorize/substrate, but the paper-doll
   is the live path.)
3. `pca_paperdoll.py` — **the heart.** `build(pose, palette)` does the MANUAL
   construction: quantize -> per-colour connected components -> `pca_parts.label_part`
   (semantic label by colour+position) -> group same-part fragments (dilate-bridge)
   -> one clean polygon per instance. Cells are exact squares; horns triangles;
   dark torso `core` traced from the dark bodysuit (symmetric for front/back, z
   OVER the legs); dark `neck`; belly grid authored as a parametric NxM of equal
   squares; chest_plate single dark-green backing with pecs on top; eyes are
   detected slanted parallelograms. `render(polys,pal,w,h)` draws fills + thick
   black outlines on main parts (accents flat). `fill_gaps(...)` is the LAST-STEP
   completeness pass.
4. `pca_eyes.py` — eyes = dark slits inside the cream face blob (the face is "the
   cream blob that has eye-slits"). Front 2, profile 1, back 0.
5. `pca_optimize.py` — **gentle, bounded** per-part nudge (translate/tiny-scale/
   tiny-rotation only; ≤6px drift, 0.85–1.15x). Runs AFTER build, then calls
   `fill_gaps` as the final step. Optimizer is ONLY a guide (see principles).
6. `pca_eval.py` — the ONE standard diagnostic: `cand/<pose>.png` -> per-pose
   `reference | reconstruction | difference | outlined` panels + montage +
   `metrics.json` (IoU, mean colour diff, colour-match%, n_poly).
7. `pca_parts.py` — `label_part` (the band/colour heuristic) + `label_pose` +
   part display colours. 8. `pca_hierarchy.py` — the part-debug views.

### Run everything for all poses
```
PY=.venv/bin/python; EXP=ambition_sprite2d_renderer/experimental/perfect_cellular_automaton
for p in top_front top_side top_back pose_idle pose_walk_1 pose_walk_2 pose_attack pose_jump pose_air pose_land; do
  $PY $EXP/pca_paperdoll.py --pose $p --version 10_anchored
  $PY $EXP/pca_optimize.py  --pose $p --version 10_anchored --passes 3
  $PY $EXP/pca_hierarchy.py --pose $p --version 10_anchored
done
$PY $EXP/pca_eval.py --version 10_anchored
```

## Scratch layout (KEEP IT ORGANIZED — Jon insisted)
- `inputs/` stable refs: `reference_full.png`, `segment_raw/clean.png`,
  `refs/<pose>.png` (eval targets), `palette.json`.
- `versions/<NN_name>/` one per tactic, identical structure: `cand/<pose>.png`,
  `eval/{montage,<pose>,metrics.json}`, `hierarchy/`, `parts/`, `<pose>_polys.json`.
- `diagnostics/` ad-hoc analysis crops.
- `LATEST_*.png` titled, DISTINCT representatives — never clobber: `LATEST_vectorized`
  (reconstruction), `LATEST_eval_diff` (standard diff), `LATEST_hierarchy`,
  `LATEST_optimizer_effect` (ref | MANUAL | optimized+filled).
- **Each diagnostic image must carry a title naming the view.** When asked for a
  NEW diagnostic, write a NEW file; do not overwrite an old one.

## Jon's principles (HARD CONSTRAINTS — violating these wasted hours)

1. **Manual/authored placement is FIRST-CLASS. The optimizer only GUIDES.** Do
   manual placement, look at what the optimizer does, and keep its result only if
   it's in the spirit of the sprite; reject distortions, per-instance. The
   optimizer must stay gentle + bounded (it went "ham" with skew/large-scale and
   broke things). Never let it be the end-all.
2. **Semantic meaning is the regularizer.** Every polygon must belong to a known
   part (helmet/horn/face/eye/forehead_cell/neck/chest_plate/pec/belly_panel/
   belly_cell/core/shoulder/shoulder_spot/upper_arm/forearm/hand/thigh/knee/shin/
   foot/tail). Do not chase a higher match number by adding noise polygons.
3. **Matching the reference is an extremely important signal.** Its *interior is
   nearly pristine*; only edges are slightly noisy and even that isn't bad. So:
   **missing pieces are bugs.** 100% isn't the goal (drop the edge noise), but the
   end state is ~95%+ match and it should look *better* than the reference —
   consistent, cohesive, noise-free.
4. **Eyes are non-negotiable** — present in every face-bearing frame, exactly
   identified, and shaped as **slanted parallelograms** (top sheared OUTWARD so he
   reads *mean*, not sad).
5. **Paper-doll construction**: each part is its own polygon, assembled by
   **z-order layering**. NO single non-convex silhouette. Most parts are convex-ish
   and **low-edge — 5–12 sides, NOT 4** ("you took <10 too literally"); horns are
   triangles. A few parts are genuinely concave (lower leg) but can often be convex
   + layering. **Polygons may overlap with z-order** — use that (e.g. upper-arm
   extends UP under the shoulder; pecs over the chest_plate; core over the legs).
6. **Automaton cells are exact squares** (belly grid + forehead pattern), authored
   as a regular array — do this AFTER semantic labeling.
7. **Torso `core`**: trace carefully (~15 edges) with the hip/pelvis detail; it's
   the dark area the belly grid sits on; nearly SYMMETRIC in front/back (use the
   back view to inform the front — front/back are a mirror-flip). It is a z-layer
   **OVER the legs** (its lower pelvis/crotch outline shapes how the upper legs
   read). **Don't confuse the thin leg OUTLINES with the core** — the core ends at
   the crotch. There's also a distinct **dark neck** trapezoid below the chin.
8. **Main parts (arm/torso/leg/head) get a THICK BLACK OUTLINE** (line-art, like
   the reference); accents (cells, spots, eyes) usually don't. Outlines also give
   the optimizer edges to align.
9. **Never drop legitimate content.** Area-based caps dropped real feet/claws —
   that's forbidden. Clean over-fragmentation by MERGING, not dropping.
10. **Use vision.** Render to disk and LOOK at every result; back it with
    programmatic diffs against the right reference region. Watch for "noise that
    helps the metric by chance."
11. **Diagnostics**: a stable, consistent set (esp. the diff) applied to EVERY
    tactic; a **hierarchical part-debug view** (high-level head/torso/L-arm/R-arm/
    L-leg/R-leg, then sub-part drill-down) so the semantic labeling is verifiable.

## Current state

- **Live version: `10_anchored`** (supersedes `10_anchored`; code is shared, the
  version is just the output snapshot dir). See `versions/10_anchored/notes.md`
  and `LATEST_core_anchoring_before_after.png`.
- **Good:** `top_front` and `top_back` are close to the reference — neck,
  symmetric hourglass core over the legs, clean 4x4 belly grid, both pecs,
  parallelogram eyes, triangle horns, octagon helmet, single chest_plate, line-art
  outlines, clean limb counts. Eyes correct per view (2/1/0). Completeness fill
  keeps uncovered foreground low.
- **DONE in 10_anchored (roadmap #1/#2, the CORE/NECK part):** the dark `core` and
  `neck` are now **face-anchored** (largest opened dark blob below the detected
  face bottom; helmet cut above the face) instead of fixed `0.22h–0.67h` bands, so
  the torso is followed on crouch / dive / profile. The earlier "too-big head box
  swallowed the neck/torso" failure was avoided by anchoring to the face *bottom*
  (a cut line), not a head *box*. Also: belly-grid degenerate guard + `fill_gaps`
  no longer squares large green gaps (killed the side-view floating green block).
- **DONE (per-view pec + helmet bound):** the cream chest splits into TWO pecs
  ONLY in front view (eye-count == 2); profile reads ONE pec, back none — fixed
  the "2 pecs in profile" bug. The `helmet` mask is now clamped to the tight head
  box (`_in_head_tight` extent: face ±0.35fw in x, ~2fh above to 0.25fh below the
  face) so it TRACES the head instead of ballooning into a giant dark blob in
  profile. `top_side` IoU 0.894 -> 0.951, match 2% -> 31%. Front/back not
  regressed (front still 2 pecs).
- **Still needs work:** `label_part`'s CHEST/BELLY y-bands are STILL fixed
  fractions — on heavy crouch (attack/land) the green/cream there can mislabel.
  Anchor those bands to the detected core/torso extent next. The helmet is now
  bounded but its per-view SHAPE (roadmap #3) could be refined; side
  forehead/horn cluster still reads busy.

## Roadmap to near-perfect (modulo noise + artistic discretion)

1. **View-general, landmark-anchored labeling (THE big lever).** Replace the fixed
   y-band heuristics in `pca_parts.label_part` with body-relative coordinates
   anchored to detected landmarks: the **face** (`pca_eyes.detect`) gives the head;
   the foreground principal axis / centroid gives the torso/limb frame. Then
   head/torso/limb/leg regions are correct regardless of tilt or crouch. This fixes
   all 7 action poses + side at once. Bound the helmet to the head box (above the
   face, tight) so it never towers. Test that it does NOT regress front/back.
2. **Per-pose adaptive authoring.** Make `core`, `neck`, and the belly-grid band
   follow the detected torso (not fixed `0.22h–0.67h` fractions). Symmetrize only
   front/back.
3. **Helmet/head shape per view** — profile head is a different silhouette; shape
   it from the actual head dark bounded to the head box, capped to a sane octagon.
4. **Tighten coverage/connection** so the line-art reads cleanly: extend limb tops
   UNDER their neighbours via z-order (upper-arm under shoulder, thigh under core),
   so no white seams; keep `fill_gaps` as the safety net only.
5. **Push match to ~95%** by reviewing `LATEST_eval_diff` + `LATEST_optimizer_effect`
   per pose: keep optimizer nudges that help, hand-correct the manual parts that
   don't. Shading bands *inside* a labeled part are allowed (still that part) if
   they lift match without adding noise — but semantics stay the regularizer.
6. **Cross-view part consolidation (Phase D).** Build one canonical shape per part,
   transformed per frame, so the same `left_arm/forearm` is recognizably the same
   part across views — this is what enables authoring NOVEL poses and makes the
   character a reusable template/"new bodies" base.
7. **Then, and only with Jon's sign-off,** hook the fitted polygons into the
   actual sprite generator (the tack-on target `targets/characters/perfect_cellular_automaton.py`).

## Gotchas
- numpy 2.x: use `np.ptp(a)`, not `a.ptp()`. Heredoc-over-stdin python can get
  killed in this sandbox — prefer running script files or `python - <<'PY'` from
  the venv (works) but watch for it.
- Don't commit binary/generated data (PNGs, polys json) — scratch is gitignored;
  commit only the `.py` tools + docs. Work on `main`. Sign commits as the executing
  model with the Co-Authored-By trailer.
- The segment is imperfect at edges — adapt; don't trust its exact silhouette.
