# Sprite renderer

The asset pipeline (a large Python tool — ~38k LOC of imperative PIL + ~12k of
drawer code) is tamed behind a small publishing surface. The principle that
matters to the *engine* is **measure-by-default**: a sprite ships the geometry
the gameplay layer needs, so the body and its hitbox cannot silently disagree.

---

## The thesis

> Plural authoring, one validated published-asset contract.

The runtime contract is the generated result: sprite-sheet image pages,
animation/frame layout, measured or authored actor metadata, and canonical
review output. A character's internal construction is deliberately not part of
that contract.

The tool preserves distinct authoring families because they express different
artistic needs: imperative per-character PIL (~38k LOC, each character
0.7–2.4k), YAML/config-driven generators, shared procedural family helpers,
bone/rig documents and SVG parts (a clean island around `skeleton.py` + rigdoc +
GUI + codegen), scene-graph or multipart targets, and drawer props/tiles/icons
(~12k).

A small **Pillow + stdlib core** (`core/`) owns portable operations that are
truly common, such as draw/composite helpers, measurement, packing support, and
RON emission. `FrameSet` and `FrameSource` are useful authoring seams for
pipelines that naturally fit them; they are not the universal definition of a
character and must not force bespoke targets into a common pose model or rig.

## Principles to keep

- **Plural authoring is preserved.** No authoring path is deleted or legacied
  until a replacement Jon likes *more* than the original exists — every style
  and its charm stays valid.
- **Rigs are optional.** Use a rig when articulated parts, reusable poses, IK, or
  editor-backed authoring help the sprite. Do not migrate procedural or
  specialized characters onto a rig merely for consistency.
- **Families unify at the right scope.** Share anatomy, pose math, palettes,
  framing, or part composition among genuinely related characters. Do not turn
  a family abstraction into a repository-wide mandate.
- **The publishing boundary is universal.** Registry, generation, validation,
  install, and runtime consumers operate on sheets and metadata without knowing
  which family produced them.
- **Dependencies confined to the edges.** The core uses only Pillow + the
  standard library (guarded by `test_core_minimal_deps`); PySide6 / rich live in
  the GUI/CLI only. A chatbot with `pip install Pillow` can use the portable
  rendering utilities.
- **Manifest is RON via stdlib** — no YAML in the write path.
- **Pixel-parity harness is the safety net** — a per-target render hash. **Drift
  is NOT a hard failure**: small drift is fine, especially where correct
  behavior becomes emergent. Jon **blesses or rejects** before/after diffs in
  `tmp/sprite-drift/`; `--strict` fails CI. (The tooling analogue of the
  engine's headless canaries — re-baseline freely.)

## Measure-by-default (the engine-facing principle)

The renderer **measures** each frame's canonical `body` / `feet` geometry — the
inclusive last opaque row (the "door fix" generalized: the door was grounded
one row too high because measurement excluded the last opaque row) — and bakes
it into the manifest, so the gameplay layer reads geometry from data instead of
guessing.

Cross-family metadata such as sockets, anchors, face guides, and default poses
may be authored directly, derived from rendered pixels, or exported from a rig.
Their presence does not imply that the source character is rigged.

## Landmines (real footguns — know these before touching the tool)

- **Alpha-clobber (the "gnu_ton rule").** `ImageDraw.Draw(img)` on an RGBA image
  *replaces* the destination alpha instead of blending — translucent-over-content
  renders silently break. The fix is a scratch layer + `Image.alpha_composite`,
  wrapped as the canonical `core/draw.overlay_draw`. Use it; never draw a
  translucent fill straight onto a content image.
- **~139 un-audited `ImageDraw.Draw(img)` sites** may have this clobber. **The
  parity harness CANNOT catch them** — they render consistently wrong, so there
  is no before/after drift. Needs eyeball/heuristic, not the harness.
- **The `*_spritesheet.yaml` sidecar is LOAD-BEARING** (discovery / install /
  actor-sidecar generation / CLI freshness / ~10 tests). The *manifest write* is
  yaml-free (RON), but **removing the sidecar is a separate, larger rewire** —
  the portability intent is already met, so do not treat sidecar removal as a
  quick win.
- **Harness coverage gaps:** non-registry pipelines (the mockingbird multi-file
  boss, pirate standalone, `item_icons`, factions) are not covered — changes
  there will not trip parity.
- **Two sheet assemblers are essential, not accidental:** adapters union-crop
  across frames; tack-ons recenter each frame individually. The shared part is
  just grid-packing, entangled with differing label/preview/contact rendering —
  forcing a merge would be drift-prone. Left separate by analysis; do not
  re-attempt the merge without a new concrete benefit.
- **Requested output size is not necessarily native rerendering.** Some current
  compatibility sources resize a rendered raster. Callers that require new
  high-resolution detail, such as dialog portraits, need an explicit family or
  target capability rather than assuming every `FrameSource` can provide it.

## Status

The core consolidation (shared portable helpers, deduplicated draw utilities,
unified measurement, the RON emitter, the portability guard, and directory
reorganization) has landed and was parity-verified.

`FrameSet`/`FrameSource` adoption is intentionally opportunistic: use those seams
where they simplify a family or output builder, but do not make complete
migration a goal by itself. The two assemblers remain genuinely different, and
the registered target plus its published sheet/metadata products is the stable
boundary.

**Open / not done:** the **melee hitbox-agreement tooling** — making a melee
animation and its hitbox visibly agree (the one expressiveness improvement worth
chasing inside the tool) — is **blocked on a spec from Jon**
(overlay-to-verify vs. hitbox-follows-authored-part). Do not treat it as done.
Dialog portraits are a future published product and should use family-native or
bespoke rerendering rather than crops from gameplay sheets.

## Pointers

`core/draw.py` (`overlay_draw`), `core/measure.py`, `core/pipeline.py`,
`core/manifest_ron.py`; `sheet.py` / `sheet_build.py` (the two assemblers);
`skeleton.py` + rigdoc + `part_editor.py` (one optional bone/rig family);
`registry/discovery.py` + `registry/character_generators.py` (target discovery
and config-driven generators); `docs/actor_contract.md` (runtime-facing rich
metadata).
