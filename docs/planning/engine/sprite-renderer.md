# Sprite renderer

The asset pipeline (a large Python tool — re-derived 2026-09-17 at the submodule
pin `7b8e845`: **103k lines across 169 files under `targets/characters/`**, plus
**25k** of drawer props/tiles/icons across 50, against the ~38k and ~12k this
line carried) is tamed behind a small publishing surface. The principle that
matters to the *engine* is **measure-by-default**: a sprite ships the geometry
the gameplay layer needs, so the body and its hitbox cannot silently disagree.

---

> **Guard pointer, added `0ac499bb1` (2026-09-02); re-run 2026-09-17 and still
> green at all 173 rostered targets.**
> `scripts/check_published_sheets_are_present.py` checks that every sheet the
> publish roster claims is actually on disk. ⛔ Its docstring records why it
> exists, and the reason generalises: a missing ASSET failed a test in a way that
> READ LIKE A CODE BUG
> (`a_left_drawn_character_faces_the_way_they_are_going_like_a_right_drawn_one`
> depends on `goblin_cave_dagger` being present). A roster/disk mismatch should
> fail as itself, not as whatever behaviour happened to need the file.
>
> ⚠ **DIFFERENT POPULATION FROM THE `ambition_sprite_sheet` FLOOR** that the
> queue's lane row reports red (*"780 sheet(s), below the floor of 800"*). This
> one asks whether every ROSTERED TARGET has art; that one counts published
> SHEETS in this checkout. Two numbers, two units, and neither is evidence about
> the other.

## The thesis

> Plural authoring, one validated published-asset contract.

The runtime contract is the generated result: sprite-sheet image pages,
animation/frame layout, measured or authored actor metadata, and canonical
review output. A character's internal construction is deliberately not part of
that contract.

The tool preserves distinct authoring families because they express different
artistic needs: imperative per-character PIL (103k lines over 169
files, the largest single character 2.8k), YAML/config-driven generators, shared procedural family helpers,
bone/rig documents and SVG parts (a clean island around `skeleton.py` + rigdoc +
GUI + codegen), scene-graph or multipart targets, and drawer props/tiles/icons
(25k: props 18.5k, tiles 5.2k, icons 1.5k).

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
- ✔ **THE "~139 UN-AUDITED SITES" ARE GONE, AND THE HEURISTIC THIS BULLET ASKED
  FOR IS NOW A TEST.** Re-derived 2026-09-17 against the submodule pin
  `7b8e845`: `ImageDraw.Draw(` appears **69** times in the whole tool, **39 of
  them in its own tests**, and **ZERO** anywhere under `targets/characters/` —
  the population this bullet was written about. `tests/test_no_raw_imagedraw.py`
  scans `targets` and `authoring`, requires a `# raw-draw-ok` marker with a
  justification, and poisons its own regex (28 markers, each with a reason). The
  one shipped-art site left is `targets/props/robot_slash.py`, marked and
  correct: the destination is mode `"L"`, which has no alpha to clobber.
  ⚠ **ONE GAP, RECORDED RATHER THAN FIXED BECAUSE THE TOOL IS A SUBMODULE**
  (`github.com/Erotemic/ambition_sprite2d_renderer`, and this repo holds only the
  pin): the guard's `SCANNED = ("targets", "authoring")` does not reach `core`,
  and `core/pipeline.py`'s supersampling seam hands a raw `ImageDraw.Draw(img)` to a caller-supplied
  content callback on a fresh supersample canvas — the class the bullet describes,
  outside the scan root and carrying no marker. `core/draw.py`'s five sites are
  the fix itself. ⇒ Whoever owns the submodule should widen the root and mark the
  legitimate sites, not exempt `draw.py` by BASENAME the way `ALLOWED` already
  does — a basename exemption applies to every file with that name.
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
Stage 2 default dialog-portrait coverage has landed:

- every registered character target publishes an independent
  `<target>_portraits.png` / `<target>_portraits.ron` product with a required
  named `default` clip;
- config-driven generators rerender at portrait source resolution and compose a
  logical face guide;
- module-authored families may provide custom high-detail hooks, while the
  common fallback freshly invokes the target's canonical authoring path and
  never samples an installed gameplay sheet;
- `sheet_build` families use a canonical-only authoring mode so fallback
  portraits do not rerender and repack every animation;
- multipart and fan-out publishers declare all portrait products and their
  install subdirectories explicitly;
- catalog portrait paths conventionally derive from gameplay-sheet paths, with
  explicit metadata retained only for true exceptions;
- `scripts/regen/sprites.sh` validates every Hall portrait and emits
  `generated/portrait_gallery.png` for visual review.

Stage 3 named portrait presentation has landed:

- config-driven jobs may declare named still or animated clips through
  `visual.portraits.<name>.frame` / `frames`, `duration_ms`, and `looping`;
- bespoke procedural and SVG/rig families may publish the same clip vocabulary
  through their existing target-level portrait hooks;
- portrait manifests are baked into a cross-platform runtime registry, so
  desktop, Android, and web builds resolve clip rectangles without filesystem
  discovery;
- stable speaker character ids and requested clip names flow through
  `DialogState` and `DialogView`; presentation never reverse-matches localized
  speaker labels;
- Yarn authors select speakers and expressions with the presentation-neutral
  `present_speaker` and `portrait_clip` commands;
- the Ambition presenter plays looping clips and one-shots that hold their last
  frame, with deterministic fallback to the catalog/manifest default.

Stage 4 split the request in two. A portrait sheet answers two questions --
what PLAYS and what a UI box draws -- and a consumer now names which one it is
asking:

- `PortraitSheetRegistry::resolve_still` returns exactly one frame;
  `resolve_animated` returns a clip to play. `resolve_clip`, which returned a <!-- cite-ok: records a removed API by name -->
  record and let each caller improvise, is gone;
- both degradations are stated by the API rather than left to the call site: a
  still of an animated clip is its first frame, and an animation of a one-frame
  clip is a held still;
- manifests may name a `still_clip`, so a target with a looping default can say
  which pose a select cell or HUD panel should draw instead of taking wherever
  the loop starts. It is optional; omitting it falls through to the default
  clip's first frame;
- `HudStanding` carries a source rect beside its image path, because a path
  alone cannot say which frame it means -- the HUD had been drawing whole
  portrait pages into 56px panels.

⚠ **RE-DERIVED 2026-09-17: it is TWENTY-ONE, not twelve, and the population is
the PUBLISHED manifests rather than the source.** Of 146 `*_portraits.ron` under
`crates/ambition_platformer2d_actor_monolith/assets/sprites`, 40 declare a
looping clip and 21 name a `still_clip` — and the two sets nest exactly, so "a
looping default and a named still" is the 21. Whether a character SHOULD is a
content decision, not a gate: there is deliberately no check that every target
animates its portrait, which is why this number moves with content and has to be
re-derived rather than quoted.

Native portrait production must continue to use family-specific or bespoke
rerendering, never crops enlarged from gameplay sheets. A character whose
gameplay frames compose effects owes those effects to its portrait too -- Emmy's
ethereal hum is what she looks like, and her close-up was the one place in the
game it was missing. Remaining portrait work is polish and additional
per-character expression art rather than architecture.

## Pointers

`core/draw.py` (`overlay_draw`), `core/measure.py`, `core/pipeline.py`,
`core/manifest_ron.py`; `sheet.py` / `sheet_build.py` (the two assemblers);
`skeleton.py` + rigdoc + `part_editor.py` (one optional bone/rig family);
`registry/discovery.py` + `registry/character_generators.py` (target discovery
and config-driven generators); `docs/actor_contract.md` (runtime-facing rich
metadata).

## Direction: editable SVG component scenes

For most articulated characters, the desired long-term source is an editable
SVG component scene driven by freeform Python animation. This is broader than a
bone rig: direct transforms, substitutions, z-order, opacity, authored views,
procedural overlays, FK, and IK remain peer tools.

The migration is gradual. Existing Python/Pillow definitions remain the
shipping authority until an SVG shadow render reaches accepted visual and
raster equivalence. The legacy path is deleted only after the target switches
and the migration is accepted. See
[`svg-component-character-migration.md`](svg-component-character-migration.md).

