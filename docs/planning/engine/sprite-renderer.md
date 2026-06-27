# Sprite renderer

The asset pipeline (a large Python tool) tamed behind a small, portable core. The
principle that matters to the *engine* is **measure-by-default**: a sprite ships the
geometry the gameplay layer needs, so the body and its hitbox can't silently disagree.

---

## The thesis

> A small, clear core with the sprawl tamed behind it.

One shared **PIL + stdlib core** (`core/`): supersample → crop → **measure** →
assemble → emit-manifest. Every authoring paradigm — drawers, imperative generators,
YAML adapters, rig docs — is a **thin adapter to `FrameSet`** (named animations →
frames on a known canvas) flowing through that one spine. Renders any target at any
scale (64×64 in milliseconds).

## Principles to keep

- **Plural authoring is preserved.** No authoring path is deleted or legacied until a
  replacement Jon likes *more* than the original exists — every style (and its charm)
  stays valid.
- **Dependencies confined to the edges.** The core uses only Pillow + the standard
  library (guarded by a test); PySide6 / rich live in the GUI/CLI only. A chatbot with
  `pip install Pillow` can render. This is the portability the engine's reusability
  goal wants from its tools too.
- **Manifest is RON via stdlib** — no YAML in the write path; YAML stays a read-only
  config edge.
- **Pixel-parity harness is the safety net** — a per-target render hash; drift goes to
  a scratch dir for blessing; `--strict` gates CI. (This is the *tooling* analogue of
  the engine's headless canaries.)

## Measure-by-default (the engine-facing principle)

The renderer **measures** each frame's canonical `body` / `feet` geometry (last
opaque row inclusive — the "door fix" generalized) and bakes it into the manifest, so
the gameplay layer reads geometry from data instead of guessing. Per-frame **hitbox
overlay** makes a melee animation and its hitbox visibly agree — the one expressiveness
improvement worth chasing inside the tool, because a swing that doesn't match its
hitbox is a bug you can only see.

## Status

The core consolidation (shared spine, dedup'd draw helpers, unified measurement, RON
emitter, portability guard, directory reorg) has landed; remaining duplication (two
sheet *assemblers*) is essential, not accidental. The live work is content (more
sprites) + the melee animation/hitbox agreement pass.
