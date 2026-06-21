# Sprite renderer refactor — holistic plan

*Author: Claude Opus 4.8 (1M) · 2026-06-21 · status: PROPOSAL (not started)*

`tools/ambition_sprite2d_renderer` is ~72k LOC of Python that draws every
sprite/tile/prop the game consumes. It is one of the weakest parts of the
game and the hardest to author into (LLMs manipulate algebra far better than
pixels). This doc proposes a holistic refactor. It extends the renderer's own
[`GOALS.md`](../../tools/ambition_sprite2d_renderer/GOALS.md) (rig convergence,
a `scale` param, the unified `Target` abstraction) rather than replacing it.

It was motivated by a small task — making a door sprite stand on the ground
instead of floating with a stray circle — which turned out to be the whole
refactor in miniature (see *The door as template* below).

## The one idea: measure by default, declare only the exceptions

The door floated because its *visual* was welded to its *trigger box* and its
contact point was an artifact of where the artist happened to draw pixels. The
fix was to let geometry **emerge from the pixels**: a grounded sprite's "feet"
are its lowest opaque row, and the runtime plants that row on the box's floor
face. No per-placement nudging, no per-sprite Rust constant.

That is the whole refactor's organizing principle:

> **Whatever can be measured from the rendered pixels is measured by the
> renderer and written into one manifest. Authoring only *declares* what
> pixels cannot express (sockets, an explicit collision box, hit-active
> frame windows, the ground flag). The game reads the manifest and holds
> zero per-target sprite constants.**

Correctness becomes emergent (the renderer can't forget to update a Rust
constant because there is no Rust constant), and authoring shrinks to the
irreducible creative choices.

## Current state (from three deep-dives, 2026-06-21)

**Four overlapping authoring paradigms**, ~10% reusable core / ~90% periphery:

| Paradigm | Where | ~LOC | Note |
|---|---|---|---|
| Imperative per-character PIL | `targets/characters/*.py` | ~38k | each a bespoke 0.7–2.4k-LOC generator re-deriving anatomy/pose/silhouette |
| YAML-config adapters | `configs/*.yaml` + `adapters.py` | ~3.7k | 8 runtime + ~40 review configs over the same generators |
| Bone/skeleton rigs | `skeleton.py` + `rigdoc.py` + `gui/` | ~2.9k | **only 3 of ~50 targets use it**; the promising path |
| Drawer props/tiles/icons | `targets/{props,tiles,icons}/*.py` | ~12k | door lives here; 2 tilesets are 2.2–2.7k each |

**The metadata is split three ways and drifts:**

- *Measured & consumed* — `body_metrics` (body bbox, `feet_anchor_norm`,
  per-anim hurt/hitboxes) in `*_spritesheet.ron`, read by `SheetRecord`. Good;
  single source of truth.
- *Measured-able but hard-coded in Rust* — `collision_scale`/render size lives
  in `character_catalog.ron` + a Rust `DEFAULT_TUNING`, even though the renderer
  already measures the body fraction it's derived from. The
  `TODO(gen2d-collision-aware)` in `character_sprites/sheets/geometry.rs:11`
  asks for exactly this.
- *Authored but orphaned* — `*_actor.ron` carries the richest model that
  exists (explicit collision, hurtbox, **sockets** feet/muzzle/weapon_tip,
  anim event bindings) and **nothing reads it**.
- *Duplicated and drift-prone* — **bosses** hard-code `BossSheetSpec` (rows,
  durations, anchor, scale) as `&'static` Rust constants that the RON also
  carries; a regen can silently disagree. (Cf. the parser-drift lesson —
  Python RON writers are looser than Rust's `ron`.)

**Duplication hotspots:** the `supersample → downsample → crop → assemble →
emit` spine exists in ~3 places (two separate RON emitters), `rgba`/`bbox`/
`font` helpers are reimplemented ~21×, 40+ hand-listed `ANIMATIONS` dicts,
palettes scattered, contact-sheet/canonical builders in 3 shapes.

**The bone system is sound but nascent.** `skeleton.py` (FK + two-bone IK with
foot-trajectory authoring + keyframes + z-ordered painters) is clean; `rigdoc`
(`.rig.json`) + the GUI + the `rigdoc → Python` codegen bridge all publish
through the *same* `build_sheet` pipeline. Emmy/Noether is ~148 lines reusing
the robot's skeleton+IK+clips — proof of the "100-line character." Blockers to
broad adoption: the data shape vocabulary (polygon/capsule/circle) is narrower
than Python painter closures; it's strictly 2D side-view (parrot turnarounds
fall back to ~350 LOC); humanoid-biped assumptions are baked into `solve`.

## The unifying design — four pillars

### Pillar A — One contract, measured by default *(this is the "full metadata model")*
- Collapse `*_spritesheet.ron` + `*_actor.ron` into **one manifest schema** per
  sprite/sheet. Fold in the useful `actor.ron` fields: explicit collision box,
  hurtbox, **sockets**, anim event windows. Add `ground`/`feet` and the
  `tuning` block (already deserialized Rust-side, never emitted).
- Renderer **measures**: feet/contact row, body extent, alpha hurtbox, and
  **derives `collision_scale`/render size from the body fraction** (implements
  `TODO(gen2d-collision-aware)`). Authoring **declares** only sockets, explicit
  collision, hit-active frames, ground.
- Game **reads** the one manifest; **delete** catalog `sprite_tuning`, Rust
  `DEFAULT_TUNING`, and the hard-coded `BossSheetSpec` row/anchor/scale
  constants. Bosses load rows+anchor+scale like characters do.
- Generalize the door fix: entity/prop sprites carry `ground`/`feet`/anchor in
  the manifest, so the world renderer's grounded-sprite path is **data-driven**
  rather than a hard-coded `matches!(Door)`. This also fixes the door
  follow-up cleanly (the door slab's foot, not its decorative sill, is the
  measured contact row; the stoop/frame become declared parts).

### Pillar B — One rendering spine *(kill the duplication, no behavior change)*
- Extract a single pipeline: `produce frames → supersample → downsample →
  crop{tight|ground|tiled} → measure → assemble sheet → emit manifest →
  install`. One `Frame`/`FrameSet` seam that *all* paradigms feed.
- One manifest emitter (retire the second RON writer), one sheet assembler, one
  contact-sheet/canonical builder. Consolidate the ~21 helper copies into
  `common_draw.py` + one palette module.
- This is pure dedup behind a parity harness: large LOC drop, ~zero risk.

### Pillar C — Authoring converges on the rig/data model *(pragmatically)*
- Make `rigdoc` the documented **front door for new** characters *and* props
  (a prop is a one-bone rig). It already publishes uniformly.
- Close the rig gaps so it can absorb the imperative periphery: expand the data
  part vocabulary to what painter closures do today (shaded/gradient sub-parts,
  multi-bone parts, lines/decals/joint-caps) so codegen round-trips and Python
  closures become unnecessary; de-hardcode the biped assumptions (IK chains +
  contact points as data); add multi-view (authored pose-sets) so turn frames
  stop forcing an imperative fallback.
- **Do not big-bang the 38k LOC.** Migrate opportunistically behind the parity
  harness; turn-heavy/exotic targets keep rendering through Pillar B's spine
  until the rig can express them — no regression either way.

### Pillar D — Prune
- Delete `legacy/`, dead `generated/` experiments, and the `actor.ron` emitter
  once its fields move into the one manifest. Collapse the pirate/lasersword
  "common + thin-stub" families into rig specs.

## Sequencing — safety net first, then collapse, then converge

0. **Parity harness + drift guard.** A low-res (`scale`) render-hash per target
   (unblocks GOALS #1/#3 fast tests) **and** a Rust-side parse test for the
   unified manifest. Nothing else proceeds without this. *(Highest leverage;
   makes every later phase safe — cf. "build the differential harness first,
   then port boldly".)*
1. **Pillar B — one spine + shared helpers.** Behavior-preserving dedup proven
   by the harness. Biggest LOC win, lowest risk.
2. **Pillar A — measured metadata + retire Rust constants.** Emit `tuning`/
   collision from the body fraction; unify the manifest; make the game read it;
   delete catalog tuning + `BossSheetSpec` constants; data-drive the grounded
   sprite path (closes the door follow-up).
3. **Pillar C — rig vocabulary + generalization.** Expand parts, de-hardcode
   biped, multi-view; make rigdoc the front door.
4. **Pillar D + opportunistic migration.** Port bespoke targets one PR at a
   time behind the harness; collapse families; delete dead code.

## Rough impact
- LOC: ~72k → realistically ~45–50k after Pillars B+D and partial C, with
  *more* expressiveness (new character/prop ≈ a ~100-line rig spec or a GUI
  doc, not a 1k-LOC generator).
- Drift bugs (renderer↔Rust) structurally eliminated: zero per-target sprite
  constants in Rust.
- Tests go from minutes (skipped by default) to milliseconds (run every
  commit).

## Decisions for Jon
1. **Scope of v1:** all four pillars sequenced as above, or land Pillars 0→2
   (harness + spine + metadata model) first and treat rig convergence (3–4) as
   a follow-on? *(Recommend 0→2 first — it delivers the metadata model you
   asked for and the drift fix with the least new design surface.)*
2. **One manifest format:** keep RON (game-native, strict parser = free drift
   guard) and drop the YAML/`actor.ron` siblings, or keep a human-audit YAML
   alongside? *(Recommend RON-only + a generated human-readable report.)*
3. **Bosses now or later:** fold bosses into the data-driven path in Pillar A
   (kills the worst drift source) vs. deferring? *(Recommend now — it's the
   highest-value de-duplication.)*
4. **GUI investment:** is on-canvas polygon editing worth building, or is
   table + codegen-to-Python enough for agent authoring? *(Affects how far
   Pillar C goes.)*
