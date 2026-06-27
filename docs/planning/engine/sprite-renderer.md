# Sprite renderer

The asset pipeline (a large Python tool — ~38k LOC of imperative PIL + ~12k of drawer
code) tamed behind a small, portable core. The principle that matters to the *engine*
is **measure-by-default**: a sprite ships the geometry the gameplay layer needs, so the
body and its hitbox can't silently disagree.

---

## The thesis

> A small, clear core with the sprawl tamed behind it.

One shared **PIL + stdlib core** (`core/`): supersample → crop → **measure** →
assemble → emit-manifest. Every authoring paradigm flows through that one spine as a
**thin adapter to `FrameSet`** (named animations → frames on a known canvas). Renders
any target at any scale (64×64 in milliseconds).

The four authoring paradigms (kept, distinct complexity): imperative per-character PIL
(~38k LOC, each character 0.7–2.4k), YAML adapters, bone rigs (a clean island —
`skeleton.py` + rigdoc + GUI + codegen, 3 targets), drawer props/tiles/icons (~12k).

## Principles to keep

- **Plural authoring is preserved.** No authoring path is deleted or legacied until a
  replacement Jon likes *more* than the original exists — every style (and its charm)
  stays valid.
- **Dependencies confined to the edges.** The core uses only Pillow + the standard
  library (guarded by `test_core_minimal_deps`); PySide6 / rich live in the GUI/CLI
  only. A chatbot with `pip install Pillow` can render — the portability the engine's
  reusability goal wants from its tools too.
- **Manifest is RON via stdlib** — no YAML in the write path.
- **Pixel-parity harness is the safety net** — a per-target render hash. **Drift is NOT
  a hard failure**: small drift is fine, especially where correct behavior becomes
  emergent. Jon **blesses or rejects** before/after diffs in `tmp/sprite-drift/`;
  `--strict` fails CI. (The tooling analogue of the engine's headless canaries —
  re-baseline freely.)

## Measure-by-default (the engine-facing principle)

The renderer **measures** each frame's canonical `body` / `feet` geometry — the
inclusive last opaque row (the "door fix" generalized: the door was grounded one row
too high because measurement excluded the last opaque row) — and bakes it into the
manifest, so the gameplay layer reads geometry from data instead of guessing.

## Landmines (real footguns — know these before touching the tool)

- **Alpha-clobber (the "gnu_ton rule").** `ImageDraw.Draw(img)` on an RGBA image
  *replaces* the destination alpha instead of blending — translucent-over-content
  renders silently break. The fix is a scratch layer + `Image.alpha_composite`, wrapped
  as the canonical `core/draw.overlay_draw`. Use it; never draw a translucent fill
  straight onto a content image.
- **~139 un-audited `ImageDraw.Draw(img)` sites** may have this clobber. **The parity
  harness CANNOT catch them** — they render consistently wrong, so there's no
  before/after drift. Needs eyeball/heuristic, not the harness.
- **The `*_spritesheet.yaml` sidecar is LOAD-BEARING** (discovery / install / actor-
  sidecar generation / CLI freshness / ~10 tests). The *manifest write* is yaml-free
  (RON), but **removing the sidecar is a separate, larger rewire** — the portability
  intent is already met, so don't treat sidecar removal as a quick win.
- **Harness coverage gaps:** non-registry pipelines (the mockingbird multi-file boss,
  pirate standalone, `item_icons`, factions) aren't covered — changes there won't trip
  parity.
- **Two sheet assemblers are essential, not accidental:** adapters union-crop across
  frames; tack-ons recenter each frame individually. The shared part is just
  grid-packing, entangled with differing label/preview/contact rendering — forcing a
  merge would be drift-prone. Left separate by analysis; don't re-attempt the merge.

## Status

The core consolidation (shared spine, dedup'd draw helpers, unified measurement, the
RON emitter, the portability guard, directory reorg) has landed, parity-verified.

**Open / not done:** the **melee hitbox-agreement tooling** — making a melee animation
and its hitbox visibly agree (the one expressiveness improvement worth chasing inside
the tool) — is **blocked on a spec from Jon** (overlay-to-verify vs.
hitbox-follows-rig-part). Do not treat it as done. The per-paradigm `FrameSet`
migration is partially blocked (the two assemblers are genuinely different) and
lower-payoff than hoped.

## Pointers

`core/draw.py` (`overlay_draw`), `core/measure.py`, `core/pipeline.py`,
`core/manifest_ron.py`; `sheet.py` / `tackon_sheet.py` (the two assemblers);
`skeleton.py` + rigdoc + `part_editor.py` (the bone-rig island); `target_registry.py`.
