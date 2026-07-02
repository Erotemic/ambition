# One FrameSource, one pipeline

Status: in progress (2026-07-01). Follows
[canonical-sprite-generators.md](canonical-sprite-generators.md).

## The honest problem (from the audit)

Dissolving the adapter layer made the nine procedural PIL generators read well,
but a close reviewer would still see hodgepodge across the *whole* renderer:

- **Two build pipelines** doing the same crop → pack → measure → emit work:
  `authoring/sheet.py::build_spritesheet` (procedural generators) and
  `authoring/tackon_sheet.py::build_sheet` (everything else — props, bone-rig
  characters, multi-target modules).
- **Two incompatible frame contracts**: `CharacterGenerator.render_frame(spec,
  animation, frame_index, size, job)` (a method) vs. the tackon/rig
  `render_fn(animation, frame_idx, nframes)` (a free callable). Same idea, two
  shapes.
- **Three registries**: the hard-coded `GENERATORS` dict, the discovered
  `TackonTarget`s, and per-module `TARGETS` dicts (`rigged`, `ai_era_enemies`) —
  plus a `Target` protocol bridging them and an `ADAPTER_HELPER_STEMS` frozenset
  that exists only to *hide* helper modules from discovery (a smell).
- The CLI special-cases the two worlds (`list-targets` prints `GENERATORS`
  separately from discovered targets).

The drawing *backends* are legitimately diverse — a goblin's bespoke PIL, a bone
rig's transforms, an SVG raster. That diversity is correct and must stay; forcing
them into one drawing model would be the very impedance-adapter hack we're
removing. What is NOT legitimate is the diverse *envelope* around them.

## The canonical spine: `FrameSource`

One thing the pipeline consumes — "a character/prop ready to render at any size":

```python
class FrameSource(Protocol):
    target: str
    def animations(self) -> dict[str, dict]          # {name: {"frames", "duration_ms"}}
    def frame(self, animation, index, count, size) -> Image
    def canonical_pose(self) -> tuple[str, int]
    def attack_hitboxes(self, size) -> dict           # gameplay geometry, default {}
    def hurtbox_parts(self, size) -> dict             # default {}
    def body_inset(self) -> dict | None               # default None
    def actor_metadata(self) -> dict | None           # sidecar, default None
```

Each backend **produces** a `FrameSource` — it does not get reshaped into one:

- **Procedural** — `CharacterGenerator.frames_for(job) -> FrameSource`: samples
  the spec once, closes over (spec, job), draws in `frame()`. The bespoke PIL
  lives inside `frame`; the envelope is uniform.
- **Bone rig** — a `RigDocument` *is* a `FrameSource` (clips → animations,
  `frame()` renders a clip frame).
- **Callable** — a target that already has `(rows, render_fn)` is a
  `FrameSource` directly. This is not an adapter over a different abstraction; a
  frame callable + row list *is* a frame source in its simplest form.

## One pipeline

```python
def render_sheet(source: FrameSource, *, policy, scale) -> SheetBuild   # pure compute
def write_sheet(build: SheetBuild, out_dir, *, emit_diagnostics) -> SheetFiles  # IO
```

`render_sheet` is the single crop/pack/measure/emit core. `write_sheet` is the
single IO boundary (and it routes the canonical / preview / transparent
*diagnostics* out of the runtime root, closing the loop with the publish
boundary from `data-driven-sprites-and-characters.md`).

Every entry point becomes thin:

- `write_spritesheet(job)` → `get_generator(job.target).frames_for(job)` →
  `render_sheet` → `write_sheet`.
- tackon `build_sheet(target, rows, render_fn, …)` → build a callable
  `FrameSource` → `render_sheet` → `write_sheet` (kept as a compat shim so the
  33 tackon modules and the rig codegen need no change).
- rig `render_sheet_for_doc(doc)` → `render_sheet(doc)` → `write_sheet`.

The two 300-line orchestrators collapse into one core plus thin constructors.

## One registry / one Target (DONE)

`TackonTarget` + `AdapterTarget` are now one `Target` class with
`from_module` / `from_config` constructors and a `kind` field (renderer commit
`eba4e21`, byte-identical across 120 targets). The `tackon_sheet.py` module is
renamed `sheet_build.py` (`096c82e`) — the "tackon" name is gone.

## The keystone remaining squash: invert module targets onto FrameSource

An indirection audit (Target → first PIL draw) found the paths are lean *within*
a category (icons/tiles ~2 hops; module props/chars ~6; config chars ~8–11; rig
~9, justified by bones/IK). The one fat seam that spans categories: **~40
module-authored targets each hand-roll `def render(out_dir): return
build_sheet(NAME, ROWS, render_fn, …)`.** That `render_fn(anim, i, n) -> Image`
callback *is a FrameSource in disguise*.

Invert it — a module *declares* its FrameSource (rows + frame callable +
geometry) and one `render_sheet(FrameSource)` builds it — and three things
collapse at once:

1. the ~40 `render()` wrappers disappear;
2. `build_spritesheet` (config) and `build_sheet` (module) merge into one
   pipeline — the last real duplication;
3. the per-frame / arbitrary-resolution API (`render_all_frames`, contact
   sheets, per-frame export) works for **every** target, not just the 9 config
   generators.

This is the natural completion of both the FrameSource contract and the
per-frame work. Do it harness-gated for byte-stability like the rest.

## What the merge actually is (honest scoping)

Reading both 300-line orchestrators in full changed the plan. They are *not* one
algorithm with two inputs. They genuinely differ in higher-level semantics:

- frame production: generator `render_frame` with `render_scale` supersampling vs
  a `render_fn(anim, i, n)` callable;
- per-animation geometry: the generator derives hurtboxes from *source* alpha
  bboxes keyed by row name and applies `body_inset`; the tackon path derives them
  from *cropped* frames keyed by a gameplay `animation_key_map`;
- body measurement (`measure_body_metrics` vs `alpha_bbox_metrics`) and the actor
  sidecar (`write_actor_contract_for_adapter` vs `_for_tackon`).

Forcing those into one function would trade duplication for a mode-switching
`if source_is_generator:` union — not more beautiful. So the merge is scoped to
what genuinely IS duplicated, and done **provably** (harness byte-diff):

- The generators inlined their own copy of the layout+packer while the tackon
  path used `layout_sheet_rows` — whose docstring already *claims* to be "the ONE
  sheet-layout seam". That claim was a lie. Routing `build_spritesheet` through
  `layout_sheet_rows` + the shared `records_to_ron` emitter makes it true and
  deletes ~75 lines of duplicated packing/grid code. Proven byte-identical RON +
  page PNGs across all adapter targets (the shared emitter ignores the per-rect
  `index`/`duration_ms` the inline path carried; every adapter target is
  `trim=True` so the packer call is identical). The only change is the *throwaway*
  YAML sidecar's `animations:` → `rows:` key — and the one dev-tool that reads it
  (`_animation_rows_from_manifest`) already accepts both.

The `FrameSource` contract still stands as the one frame-production contract; the
two paths now also share the one layout seam and the one RON emitter. The
remaining per-path differences are legitimate authored-geometry / measurement
semantics, left as focused code rather than a conditional-laden merge.

## Execution order (checkpoints, suite green throughout)

1. `FrameSource` protocol + `CharacterGenerator.frames_for(job)`.
2. Extract the shared `render_sheet` / `write_sheet` core; make `build_sheet`
   and `write_spritesheet` thin callers. **Keep per-target output stable** —
   verified with the opt-in `--run-slow-render` suite, not just the fast tests.
3. Route the bone-rig path through `render_sheet`.
4. Unify the registry + `Target`, delete `ADAPTER_HELPER_STEMS` and the CLI
   special-case.

Safety net: `.venv/bin/python -m pytest --run-slow-render` (the full-render
tests that actually assert on pixels). The one pre-existing `test_ldtk_manifest`
failure is unrelated.
