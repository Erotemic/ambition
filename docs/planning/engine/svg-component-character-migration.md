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
