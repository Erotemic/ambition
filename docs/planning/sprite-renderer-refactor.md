# Sprite renderer refactor — maintainability & portability

*Author: Claude Opus 4.8 (1M) · 2026-06-21 · status: PROPOSAL (thinking, not started)*

Scope corrected after Jon feedback. This is **about the Python tool**
(`tools/ambition_sprite2d_renderer`, ~72k LOC), which is *large and hard to
maintain* — that is the certain problem to solve. Game-side intent wiring
(sockets, hand positions, boss-attack contracts) is sensible but **deferred**;
we focus on reorganizing/refactoring the tool first.

## Goals (in priority order)

1. **Maintainability & navigability.** The tool should be easy for a human or an
   agent to find their way around and change without fear. Reducing LOC is a
   *win but a side effect* — the target is a small, clear core with the sprawl
   tamed behind it.
2. **Minimal-dependency core (new constraint).** A chatbot agent that has only
   `pip install Pillow` (PIL + Python stdlib, no YAML/rich/PySide6) must be able
   to author and render a sprite. Heavy deps stay at the edges. *Relaxable only
   if it demonstrably blocks great sprites.*
3. **Plural authoring, charm preserved.** Drawers, imperative PIL, YAML adapters,
   and bone rigs all stay valid. Bone rigs are *one* good, maintainable method,
   not a mandate. **No path is deleted/legacied until a replacement Jon likes
   *more* than the original exists.** The goofy charm is a feature.
4. **Make melee authoring tractable.** Bespoke special effects are good and stay
   as content code; their animations are easy to make "agree" with the effect.
   The painful case is **melee: getting the animation to agree with the hitbox
   and timing.** That is an authoring-tool problem and the one expressiveness
   improvement worth chasing inside the tool.

## What's wrong today (verified by deep-dives, 2026-06-21)

**The render spine is duplicated.** `supersample → downsample → crop → measure →
assemble sheet → emit manifest` exists in ~3 places, with **two separate RON
emitters** (`sheet.py` for adapters, `tackon_sheet.py` for tack-ons). Color/
bbox/font helpers are reimplemented **~21×**; there are **40+ hand-listed
`ANIMATIONS` dicts** and scattered palettes; the contact-sheet/canonical builder
exists in 3 shapes. `common_draw.py` is 129 LOC when it should be the shared
core.

**Dependencies are tangled into the render path** (blocks goal 2):
- **`rigdoc.py` imports PySide6** → the data-driven rig *render* path is coupled
  to the GUI dependency. A chatbot agent can't render a `.rig.json` without Qt.
- `yaml` is imported in **15 files** — both config *reading* and manifest
  *writing* (`yaml.dump`). Writing doesn't need the lib; reading is a true edge.
- `rich` (declared dep) is only CLI prettiness; `numpy` appears in
  `target_registry.py`. Neither should sit under "render a sprite."

**Four authoring paradigms, ~10% core / ~90% periphery.** Imperative per-
character PIL (~38k LOC, each a bespoke 0.7–2.4k-LOC generator), YAML adapters,
bone rigs (clean island: `skeleton.py` + `rigdoc` + `gui` + codegen, used by
only 3 targets), drawer props/tiles/icons (~12k, two tilesets 2.2–2.7k each).
Plus `legacy/` and ~130 `generated/` subdirs (some stale experiments).

## The shape of the refactor

The unifying move is **one small PIL-only core that every authoring method feeds
through**, not a unification of *how* sprites are authored. Each paradigm becomes
a thin adapter that produces a `FrameSet` (named animations → frames on a known
canvas); the core does supersample/crop/measure/assemble/emit *once*. This kills
the duplication, makes the core portable, and leaves every authoring style (and
its charm) intact.

```
authoring (plural)                     core (small, PIL + stdlib only)
  drawer fns ───────┐
  imperative gens ──┤   produce        ┌───────────────────────────────┐
  YAML adapters ────┼──  FrameSet  ──▶ │ supersample→crop→measure→      │ ─▶ PNG + ONE manifest
  rig docs / GUI ───┘   (frames+meta)  │ assemble sheet→emit manifest   │
                                       └───────────────────────────────┘
  edges: yaml (read configs) · rich (CLI) · PySide6 (gui only)
```

## Phases — safety net first, then collapse inward, then reorganize

0. **Safety net + portability probe.**
   - Parity harness: a low-res (`scale`) render-hash per target (also delivers
     GOALS.md's fast-tests). Behavior-preserving phases are proven against it.
   - A test asserting the **core imports and renders with only Pillow + stdlib**
     (locks goal 2 so it can't regress).
   - Keep `regen_sprites.sh` green on a fresh clone (existing invariant).
1. **Extract the PIL-only `core/`.** Move the render spine, the shared draw
   helpers (color/shape/font/**one palette module**), the `Frame`/`FrameSet`/
   `Sheet`/`Manifest` model, and **one** manifest emitter into `core/`. Point all
   ~3 spine copies and ~21 helper copies at it. Pure dedup; harness proves no
   pixel change.
2. **Decouple the deps (goal 2).** Break `rigdoc.py`'s PySide6 coupling (rig
   *render* = PIL only; Qt lives solely in `gui/`). Confine `yaml` to a config-
   reading edge module; make manifest writing stdlib-only. Make `rich` optional
   (plain-print fallback); remove/justify `numpy`. Net: `pip install Pillow`
   renders any drawer or rig doc.
3. **Make each paradigm a thin adapter to core.** Drawers, imperative gens, YAML
   adapters, and rig docs all just build a `FrameSet`. Collapse the 40+
   `ANIMATIONS` dicts onto the shared animation vocabulary. De-dupe contact-sheet/
   canonical generation.
4. **Reorganize for navigability.** A clear top-level split (`core/` ·
   `authoring/` methods · `targets/` content · `gui/` · `configs/`), one job per
   module (GOALS.md #4). Delete `legacy/` and dead `generated/` experiments.
5. **Authoring ergonomics — melee focus (goal 4).** Per-frame hitbox
   overlay/measurement so a melee animation and its hitbox visibly *agree*
   (author sees the box on each drawn frame; measured-from-pixels where it can
   be). Rig docs become the recommended front door for new characters/props.
   *(Open: whether a small "boss pattern" authoring vocabulary helps melee
   readability — uncertain, explore only if it earns its keep.)*
6. **Opportunistic collapse, taste-gated.** Fold the pirate/lasersword
   "common + thin-stub" families into rig/declarative specs; port a charming
   target only when the new result is one Jon likes *more*.

## Explicitly deferred (not this pass)
- Un-orphaning `actor.ron` sockets / hooking render-computed hand positions into
  the game (sensible, Jon agrees — but after the tool is maintainable).
- Any boss-attack *data* contract. Bespoke specials stay as content code.
- Retiring Rust sprite constants / boss-spec drift (game-side; later).

## Decisions (resolved 2026-06-21)
1. **Manifests: RON/JSON, no YAML in the write path.** Core = Pillow + stdlib;
   the single emitter writes RON (game-native) via stdlib. A human-readable
   report can be generated separately. ✅ decided.
2. **YAML configs stay** as a read-only edge. New authoring (drawers, rig JSON)
   needs no yaml. ✅ decided.
3. **Drift policy:** pixel drift is **not** a hard failure — small drift is fine,
   especially where correct behaviour becomes emergent. The parity harness dumps
   before/after (+ a side-by-side compare) into `<repo>/tmp/sprite-drift/` for
   Jon to bless or reject; `--strict` fails for CI. ✅ decided.

Still open: reorg aggressiveness (full restructure vs in-place dedup — leaning
restructure since the harness makes it safe) and whether melee tooling comes
before or after the structural work.

## Ideal end-state (the target)
One small **Pillow+stdlib core** every authoring style flows through via a
`FrameSet` (named animations → frames on a logical canvas). The core does
`supersample → crop → measure → assemble → emit` once, with a `scale` param so
every target renders at 64×64 in ms (fast tests by default). Authoring stays
plural — drawers, imperative generators, YAML adapters, rig docs are thin
adapters to `FrameSet`. `pip install Pillow` renders any drawer or rig doc.
Melee hitboxes ride a rig part/socket so the box and the animation can't drift
("agreement by construction"); the renderer emits per-frame boxes the game
already consumes. Layout: `core/` (no heavy deps) · `authoring/` · `targets/` ·
`configs/` (yaml edge) · `gui/` (Qt edge).

## Build order (minimize rework)
Build shared infra before its consumers; bake each decision in once; touch each
file once; tidy directories last; the harness pins pixels so consolidation isn't
throwaway. Order: (1) pin pixels + write the seam as types → (2) core bottom-up:
draw toolkit → pipeline (scale) → one RON emitter → (3) route the two big shared
spines (`sheet.py`, `tackon_sheet.py`) onto core → (4) migrate remaining
paradigms touching each file once (break rigdoc↔PySide6 in the same visit) →
(5) confine leftover deps → (6) reorganize dirs + delete legacy/dead →
(7) additive: melee tooling, rig front-door, taste-gated family collapses.

## Progress
- **2026-06-21 — Step 1 landed.** `parity_harness.py` (capture/check, drift →
  `tmp/sprite-drift/` per the policy). Pre-refactor baseline captured: **117
  registry targets, 0 errors** (`.parity-baseline/`, gitignored). Seam written
  as types: `core/frameset.py` (`FrameSet`) + `core/manifest.py` (RON read-
  contract mirror). Portability guard: `tests/test_core_minimal_deps.py` (core
  imports with heavy deps blocked). *Known gaps:* harness doesn't yet cover the
  non-registry pipelines (mockingbird multi-file boss, pirate standalone,
  item_icons, factions) — extend before touching those.
- **2026-06-21 — Step 2a/2b landed.** `core/draw.py` (the ~21-times-copied
  primitives) — `entities.py` rewired onto it, parity-clean. `core/pipeline.py`
  (`render_frame`: scale-parametric rasterize→crop, modes tight/ground/none) —
  `entities._render_supersampled` routed through it, parity-clean; `scale` proven
  (0.5 → 64×64 in ms). Fixed the spurious `noether` actor-metadata test
  (rig-doc targets exempted). Helper swaps for the other ~16 files are deferred
  into each file's paradigm migration (touch-once).
- **2026-06-21 — harness upgraded to cover manifests.** It now hashes/diffs
  `*.ron` + `*.yaml` alongside PNGs (manifest drift → unified text diff in
  `tmp/`), so measurement/emitter changes are verifiable. Baseline re-captured.
- **Next phase — measurement + emitter consolidation (will produce blessed
  drift).** The two spines compute feet *differently*: `sheet._measure_body_extent`
  uses `feet_y = y_max-1` (inclusive last opaque row — matches the door fix's
  "lowest opaque pixel"); `tackon_sheet.alpha_bbox_metrics` uses `feet_y = y2`
  (one-past) + rounds. Unify on the inclusive version into `core/measure.py`
  (a genuine correctness fix), then one RON emitter (`core/manifest.py` schema),
  guarded by a Rust parse test. This shifts feet metadata ~1px on the tackon
  path → review the `tmp/` manifest diffs and bless.
