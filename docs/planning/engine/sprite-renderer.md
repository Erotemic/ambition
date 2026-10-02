# Sprite renderer

The asset pipeline is a large Python tool, the `tools/ambition_sprite2d_renderer`
submodule (`github.com/Erotemic/ambition_sprite2d_renderer`). Paths below such
as `core/draw.py` are relative to the submodule root. A small publishing surface
hides its size. The principle that matters to the engine is
**measure-by-default**: a sprite ships the geometry the gameplay layer needs, so
the body and its hitbox cannot silently disagree.

## The thesis

> Plural authoring, one validated published-asset contract.

The runtime contract is the generated result: sprite-sheet image pages,
animation and frame layout, measured or authored actor metadata, and canonical
review output. A character's internal construction is not part of that
contract.

The tool keeps distinct authoring families because they express different
artistic needs: imperative per-character PIL, YAML/config-driven generators,
shared procedural family helpers, bone/rig documents and SVG parts
(`skeleton.py`, rigdoc, GUI, codegen), scene-graph or multipart targets, and
drawer props, tiles and icons.

A small **Pillow + stdlib core** (`core/`) owns the truly common operations:
draw and composite helpers, measurement, packing support and RON emission.
`FrameSet` and `FrameSource` are authoring seams for pipelines that fit them.
They are not the universal definition of a character, and they must not force
bespoke targets into a common pose model or rig.

## Principles

- **Plural authoring is kept.** No authoring path is deleted until a
  replacement Jon likes more than the original exists.
- **Rigs are optional.** Use a rig when articulated parts, reusable poses, IK
  or editor-backed authoring help the sprite. Do not move procedural or
  specialized characters onto a rig for consistency.
- **Families unify at the right scope.** Share anatomy, pose math, palettes,
  framing or part composition among related characters only.
- **The publishing boundary is universal.** Registry, generation, validation,
  install and runtime consumers work on sheets and metadata without knowing
  which family produced them.
- **Dependencies stay at the edges.** The core uses only Pillow and the
  standard library (`test_core_minimal_deps`). PySide6 and rich live in the
  GUI and CLI only.
- **The manifest is RON via stdlib.** No YAML in the write path.
- **The pixel-parity harness is the safety net.** It is a per-target render
  hash. Drift is not a hard failure. Jon blesses or rejects before/after diffs
  in `tmp/sprite-drift/`. `--strict` fails CI.

## Measure-by-default

The renderer measures each frame's canonical `body` and `feet` geometry,
including the last opaque row, and bakes it into the manifest. The gameplay
layer reads geometry from data instead of guessing.

Sockets, anchors, face guides and default poses may be authored, derived from
pixels, or exported from a rig. Their presence does not imply that the
character is rigged.

## Landmines

- **Alpha-clobber (the "gnu_ton rule").** `ImageDraw.Draw(img)` on an RGBA
  image replaces destination alpha instead of blending. Use a scratch layer and
  `Image.alpha_composite`, wrapped as `core/draw.overlay_draw`. Never draw a
  translucent fill straight onto a content image.
  `tests/test_no_raw_imagedraw.py` scans `targets` and `authoring` and requires
  a `# raw-draw-ok` marker with a reason. Gap: the scan does not reach `core`,
  and `core/pipeline.py`'s supersampling seam hands a raw `ImageDraw.Draw(img)`
  to a content callback. Widen the scan root and mark legitimate sites; do not
  exempt files by basename.
- **The `*_spritesheet.yaml` sidecar is load-bearing** (discovery, install,
  actor-sidecar generation, CLI freshness, tests). The manifest write is
  YAML-free. Removing the sidecar is a separate, larger rewire.
- **Harness coverage gaps.** Non-registry pipelines (the mockingbird multi-file
  boss, pirate standalone, `item_icons`, factions) are not covered, so changes
  there do not trip parity.
- **The two sheet assemblers are both needed.** Adapters union-crop across
  frames; tack-ons recenter each frame. Only grid-packing is shared. Do not
  merge them without a new concrete benefit.
- **Requested output size is not native rerendering.** Some sources resize a
  rendered raster. A caller that needs new high-resolution detail (dialog
  portraits) needs an explicit family or target capability.
- **A missing asset can look like a code bug.**
  `scripts/check_published_sheets_are_present.py` checks that every sheet the
  publish roster claims is on disk, so a roster/disk mismatch fails as itself.
  It counts rostered targets. The `ambition_sprite_sheet` floor counts
  published sheets. They are different populations.

## Portraits

Every registered character target publishes `<target>_portraits.png` and
`<target>_portraits.ron` with a required `default` clip.

- Config-driven generators rerender at portrait resolution and compose a face
  guide. Module-authored families may supply high-detail hooks. The fallback
  invokes the target's canonical authoring path. It never samples an installed
  gameplay sheet.
- Named clips (`visual.portraits.<name>.frame` / `frames`, `duration_ms`,
  `looping`) are baked into a cross-platform runtime registry
  (`ambition_sprite_sheet::portrait`).
- `PortraitSheetRegistry::resolve_still` returns one frame;
  `resolve_animated` returns a clip to play. A still of an animated clip is its
  first frame, unless the manifest names a `still_clip`. An animation of a
  one-frame clip is a held still.
- Speaker ids and clip names flow through `DialogState` and `DialogView`.
  Presentation never reverse-matches localized speaker labels. Yarn uses
  `present_speaker` and `portrait_clip`.
- `HudStanding` carries a source rect beside its image path.
- `scripts/regen/sprites.sh` validates every Hall portrait and writes
  `generated/portrait_gallery.png`.

Native portraits use family-specific rerendering, never enlarged crops of
gameplay sheets. A character whose gameplay frames compose effects owes those
effects to its portrait too. Remaining portrait work is per-character art.

## Open

- **Melee hitbox-agreement tooling** (a melee animation and its hitbox visibly
  agree) needs a spec from Jon: overlay-to-verify, or hitbox follows the
  authored part.

## Pointers

`core/draw.py` (`overlay_draw`), `core/measure.py`, `core/pipeline.py`,
`core/manifest_ron.py`; `sheet.py` and `sheet_build.py` (the two assemblers);
`skeleton.py`, rigdoc and `part_editor.py` (one optional bone/rig family);
`registry/discovery.py` and `registry/character_generators.py`;
`docs/actor_contract.md` (runtime-facing metadata).

## Related

- SVG scenes: [`svg-component-character-migration.md`](svg-component-character-migration.md).
  For non-rigged characters the PIL code stays the source and the SVG scene is
  an interchange and annotation format. For rigged characters the SVG scene is
  the source.
- Runtime part animation (transform flipbooks):
  [`runtime-rigged-sprite-animation.md`](runtime-rigged-sprite-animation.md).
