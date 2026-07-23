# Editable SVG component scenes for character sprites

**Status: DIRECTION.** This is the intended migration direction, not an order to
rewrite the roster immediately. Profiling and ordinary character work may make
small enabling changes when they pay for themselves now.

## Decision

For most articulated characters, prefer an editable SVG component scene as the
long-term visual source while keeping Python as the freeform animation and
composition language. Bones, FK, IK, keyframes, and constraints are optional
helpers inside that model; they are not the required representation of a pose.

The migration targets two equivalence levels:

1. the same authored appearance and animation, with no intentional redesign;
2. nearly identical decoded RGBA frames and spritesheets, subject to small
   antialiasing differences where the SVG rasterizer cannot reproduce Pillow's
   edge pixels exactly.

Exact PNG file bytes are not a goal. Compression, chunk ordering, and metadata
may differ while decoded pixels remain identical.

## Why

The present procedural targets often mix anatomy, curves, colors, z-order,
pose math, animation timing, effects, sheet publication, and metadata. That
makes the characters expressive, but it also makes basic visual editing a code
change. The desired split is:

```text
editable SVG component scene
    owns anatomy, curves, colors, named parts, pivots, sockets, masks, variants

Python pose program
    owns timing, transforms, visibility, z-order, substitutions, IK/FK,
    procedural effects, and arbitrary animation logic

shared sheet pipeline
    owns crop, measurement, packing, manifests, previews, and publication
```

This preserves Python's expressiveness while making the character itself
editable in Inkscape or another SVG editor.

## Migration modes

Every migrating target has one explicit render authority:

- **legacy** — the existing Python/Pillow renderer ships. SVG export and
  comparison tooling may run, but cannot affect published output.
- **svg-shadow** — both paths render. Legacy still ships; the tool records image
  diffs, geometry diffs, timing, and cache behavior for review.
- **svg** — the SVG component scene plus Python pose program ships. The legacy
  definition remains temporarily available as an oracle until consolidation.

There must never be frame-by-frame fallback between authorities. A target's
published output comes from exactly one selected backend for the entire run.

## Legacy-to-SVG layer

The transition should reuse existing definitions instead of manually redrawing
every character from a flattened sheet.

The legacy adapter records supported drawing operations under stable semantic
component scopes:

```python
with component("head"):
    ... existing Pillow drawing operations ...
```

The recorder can convert circles, ellipses, polygons, capsules, strokes,
rounded paths, opacity layers, and embedded raster details into named SVG
content. Existing family helpers should supply component names where they
already know the semantic part. Unclassified operations may initially remain a
single embedded raster component rather than blocking migration.

A mechanically exported SVG is only the first representation. It becomes the
accepted source after its groups, pivots, sockets, and view structure are
reasonable to edit by hand.

This recorder now exists for the procedural (non-bone) roster:
`authoring/draw_recorder.py`'s `DrawRecorder` duck-types the `ImageDraw`
subset (`polygon`/`line`/`ellipse`/`arc`) that every `sheet_build` character
paints through, with a `component(name)` scope that groups elements under
named `<g>` layers. A character whose paint pass takes an injectable `draw`
(see `_pirate_common.paint_character`) captures its whole scene as SVG with no
redraw.

## Component-scene capabilities

The shared scene layer should support:

- named parts and authored views;
- visible SVG pivot and socket markers;
- arbitrary affine transforms;
- dynamic parenting and z-order;
- visibility and opacity channels;
- part and expression substitution;
- masks and clipping;
- optional FK, IK, and constraint passes;
- procedural Python overlays before or after component composition;
- deterministic scene and frame dumps;
- compile-once caches for parsed SVG, extracted parts, pivots, and rasterized
  component images.

The scene layer is a rendering primitive, not a mandatory humanoid skeleton.

## Equivalence harness

The existing renderer remains the migration oracle. For every target in
svg-shadow mode, compare:

- animation names, frame counts, durations, and page assignments;
- canvas sizes and crop offsets;
- decoded RGBA pixels;
- alpha silhouette and opaque bounds;
- feet anchors, body measurements, sockets, and hitbox metadata;
- canonical and portrait products;
- render time and cache behavior.

Classify accepted results as:

- **exact pixels** — decoded RGBA bytes match;
- **raster-equivalent** — differences are confined to a small antialiased edge
  tolerance and all geometry/metadata invariants match;
- **visually accepted** — explicit maintainer approval for a bounded difference.

The first two are the normal porting bar. The third is exceptional because this
campaign is not a redesign campaign.

## Execution sequence

### P0 — profiling and boundaries

Continue optimizing the current renderer. Add component names, stable drawing
scopes, centralized transforms, or reusable caches only when they simplify or
speed the current path as well. Do not add speculative abstraction solely for a
future migration.

### P1 — component scene and recorder

Build the retained component scene, SVG import/export, pivot/socket markers,
legacy drawing recorder, and dual-render comparison command. Keep every target
on legacy output.

### P2 — representative proof

Migrate three production-shaped cases without intentional visual change:

1. an existing SVG-backed character such as Oiler;
2. one shared procedural humanoid family;
3. one non-humanoid or topologically unusual articulated character.

The proof closes only when the SVGs are pleasant to edit, the comparison report
is useful, and the migrated code deletes more target-specific geometry than the
shared system adds.

### P3 — opportunistic migration

Convert families when a character is already being redesigned, fixed, or
profiled. Prefer family-level conversions over isolated copies. Keep legacy as
the shipping authority until each target reaches its accepted equivalence bar.

### P4 — consolidation

After a target ships from SVG and remains accepted, delete the superseded
anatomical drawing code, compatibility adapters, and migration-only fixtures.
Do not leave two permanent authoring authorities for the same character.

## Relationship to profiling

The supplied full-regeneration profile shows that frame production still
dominates the long tack-on batch, while packing is a smaller but material
secondary cost. The SVG direction can help only if parts are parsed and
rasterized once, cached by content and scale, and then transformed cheaply for
many frames. Re-rasterizing a complete SVG document per frame would make the
pipeline slower and is explicitly not the target architecture.

Use targeted profiles during optimization:

```bash
./profile_sprites.sh --target oiler
./profile_sprites.sh --target ninja_heavy
./profile_sprites.sh --suite representative
```

A full forced regeneration is a confirmation run, not the inner optimization
loop.

## Non-goals

- converting tiles, explosions, cellular effects, swarms, or algorithmic art
  merely for uniformity;
- replacing Python animation logic with a closed declarative clip language;
- requiring every character to use a humanoid skeleton;
- making generated SVG overwrite hand edits;
- changing runtime sprite formats as part of the authoring migration;
- preserving exact compressed PNG bytes.

## Reconsideration gate

Do not authorize a roster-wide campaign until the P2 proof demonstrates:

- practical manual editing;
- accepted pixel or raster equivalence;
- a simpler target implementation;
- no regression in metadata or runtime output;
- a credible render-time result after caching;
- a clean legacy-to-svg-to-consolidated lifecycle.

## Progress (2026-07-23)

Landed in `tools/ambition_sprite2d_renderer`:

- **Equivalence harness (P1 check).** `core/equivalence.py` +
  `equivalence_harness.py` compare two rendered output directories across the
  whole published contract — layout, animations, geometry (body bbox / feet /
  sockets), authored metadata, portraits, and registered per-frame pixels —
  and classify `exact-pixels › raster-equivalent › contract-match › differs`.
  Authority-agnostic: a pixel mismatch never fails on its own, only a broken
  contract does, so a redesign like Oiler is a valid `contract-match`. Measured
  geometry carries a small rasterizer tolerance; authored metadata stays exact.
- **PIL→SVG recorder (P1 recipe).** `authoring/draw_recorder.py` — the
  mechanical converter described above.
- **First non-bone port (P2, the shared procedural humanoid family).**
  `_pirate_common` split into `paint_character` / `draw_character` (raster,
  byte-identical) / `capture_character_svg` / `render_target_svg`. Its part
  clusters are bracketed with component scopes, so a captured pirate is a set of
  named `legs`/`body`/`arms`/`head`/`chest_motif` Inkscape layers, not a flat
  shape soup. The PIL-vs-SVG authority contract match is verified across
  `pirate_raider`, `pirate_admiral` and `pirate_lookout` — all **contract-match**:
  identical contract, pixels differ only in resvg-vs-Pillow outline/edge
  rendering (visually the same pirate). `export_svgs` /
  `equivalence_harness.py export` writes the editable per-frame SVG artifacts to
  disk.

### The authoring system (2026-07-23, second pass)

The end state Jon specified: PIL remains a first-class authoring language for
NEW sprites, but every target — characters, props, tiles — routes to an SVG
backend whose **parts are registered once in an editable scene file** and
whose frames are assembled from those parts; PIL generators are retired
per-target only after Jon approves both the SVG visuals *and* the layer
grouping in Inkscape. Landed:

- **Component scene** (`authoring/svg_scene.py`): ONE SVG per target — a
  visible `parts` gallery layer (labelled local-geometry groups) + hidden
  per-animation/per-frame layers of `<use>` placements. Editing a part in the
  gallery updates every frame that uses it. `load()` round-trips human edits;
  `frame_doc()` re-renders frames from the (edited) file through the normal
  sheet pipeline (`render_target_svg(scene_path=...)`, harness `rebuild`).
  Verified: pirate hat recolored ONCE in the gallery → all 38 frames change;
  rebuilt sheet stays contract-match.
- **Cooperative seam** (best grouping): `DrawRecorder.part(name, origin, deg)`
  records local-coordinate geometry, content-deduped; `PillowPartDraw` runs
  the SAME paint pass against Pillow. Pirates converted (8 semantic parts,
  22 defs incl. expression variants, 342 uses / 38 frames) with all 190
  pixel-oracle frames byte-identical.
- **Universal converter** (`authoring/auto_capture.py`): tees ImageDraw during
  ANY target's existing render (published raster untouched), folds scratch
  layers on alpha_composite, propagates GaussianBlur as native SVG filters,
  then discovers parts by rigid-motion congruence across frames. Harness
  `autoconvert` / `coverage` run it per-target / roster-wide, writing scenes
  to `tmp/sprite-drift/auto_scenes/` + `coverage.json`.
- **Verification is honest by construction**: sampled frames are compared
  against the actually-published sheet pixels, scale-normalized, judged on
  solid content (Pillow ImageDraw clobbers alpha; SVG composites properly —
  translucent glow is a documented divergence class). "partial" = needs
  review, never silently wrong.

### Revised end-state (Jon, 2026-07-23 evening): SVG as interchange, not replacement

For **non-rigged** characters, Python/PIL code is the better *compressed
representation* and may never retire; the SVG scene is **another output
format** — and, critically, the annotation medium: Jon edits the scene in
Inkscape to *show* an agent exactly what is wrong with a PIL sprite, the
agent back-ports the change into the PIL generator, regenerates, and the
equivalence harness verifies convergence (`export` -> human edit ->
`rebuild --scene` -> `compare --target X --against rebuilt/`). Scene SVGs are
redundant/regenerable and stay out of the repo (gitignored `tmp/`).

For **rigged** characters (Oiler, hunny_horror) SVG is where authoring
shines, and the non-rigged -> rigged transition is exactly where this interop
pays off: a reviewed scene with good part grouping is the natural starting
point for rigging a formerly procedural character.

Still open: per-target Inkscape review by Jon (the acceptance gate for
retiring any PIL generator); auto-discovered part grouping is `geomNNN`-named
(cooperative scopes give semantic names — convert paint passes opportunistically
for better grouping); bone-toolkit characters and multipart bosses run through
the universal path but their raster-composited layers (rotate/paste) are
capture gaps reported by coverage; assembler renormalization makes the
conservative verifier under-report matches.

### Discovery correctness pass (2026-07-23, GPT 5.6 review)

Three structural bugs in the universal converter's part discovery were fixed
(submodule `authoring/auto_capture.py` + `equivalence_harness.py`):

- **Transform-aware matching.** Occurrences that differed only by an ancestor
  transform (composite-fold `translate`, resize `scale`, layer `rotate`, or two
  separately-transformed sibling layers in one frame) collapsed onto a single
  placement — the second rendered at the first's location. Discovery now
  flattens every flattenable transform into geometry first (`_flatten_tree`),
  splicing away pure-positioning groups, while still peeling the shared
  sole-child outer wrapper to a prefix so matching stays in the large
  pre-downsample space (meaningful congruence tolerance + float precision).
  Only the non-peelable inner transforms — where the bug lived — are baked in;
  unflattenable residuals (rotated ellipse/image, `<use>`, filtered group) are
  kept and treated as opaque, never dropped.
- **Semantic identity is authoritative.** Named components (`left_thruster` /
  `right_thruster`) with identical geometry no longer merge — candidate
  identity is keyed on the full Inkscape label path. Unlabelled geometry keeps
  geometry-only "inferred reuse" keys (`geomNNN`), distinct from semantic ones.
- **Honest status.** `captured` now means *every* published frame was verified,
  not a ~6-frame sample. `sampled` = complete + clean capture, subset checked;
  `partial` = any gap. `autoconvert`/`coverage --full` verify every frame; the
  mockingbird boss reaches `captured 36/36` under `--full` (`sampled 6/6`
  otherwise). Poison tests pin all three in `tests/test_part_discovery.py` and
  `tests/test_status_levels.py`.
