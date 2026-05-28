# Goals

Long-arc directions for `ambition_sprite2d_renderer`. This doc is the
north star — read it before doing structural work, and update it when
a goal lands or shifts.

## 1. Standardized shape-primitive character authoring

**Where we are:** every character draws from scratch. `_pirate_common`
has its own rig for the 5 pirate roles; `toon_side` is a 3,500-line
monolith with 22 archetype switches inside one renderer; ghoul_skulker
/ weird_hermit / bear_mauler / etc. each draw bespoke geometry. The
helper layer is thin: `tackon_sheet` provides `build_sheet` +
`write_canonical` + drawing primitives, but the per-character work
still re-implements anatomy, pose math, and silhouette construction.

**Where we want to be:** every character is expressible as a small
declarative file on top of a shared shape-primitive rig:

- A single named-bone skeleton (head, torso, limbs, props, …) with a
  consistent pose-math API.
- Per-character files express *palette + body proportions + style
  choices*, not bespoke draw code. Adding a new character is one
  file, ~100 lines, that picks colors and tweaks bone lengths.
- The shared rig knows how to draw at any resolution — passing
  `scale=0.25` produces a 32×32 thumbnail; `scale=1.0` produces the
  full-size canvas.

**Why this matters:**

- **Test coverage becomes affordable.** Today every render test
  takes 1–4 seconds because targets only render at full size; the
  suite runs for minutes. With a `scale` parameter every test runs
  in milliseconds, so we can have a regression net that's actually
  *fast* enough to run on every commit. Without this, render tests
  are skipped by default and our regression net has holes.
- **Visual consistency.** Independent agents authoring new
  characters today produce wildly different silhouettes because
  they each invent their own drawing conventions. A shared rig means
  every character reads as belonging to the same world.
- **Diversity is still possible** — actually *more* so, because
  authoring a new character drops from days to hours and the
  cognitive cost of style consistency is paid by the rig, not the
  author.

**Signs we've gotten there:**

- All `targets/characters/` files import the same rig module and
  define their character by parameters, not draw code.
- `toon_side.py` is broken up into one file per archetype (today
  it's one giant `if archetype == "alice":` ladder).
- `_pirate_common.py` either becomes the shared rig or merges into it.
- Tests render every target at a small scale (e.g. 64×64) and assert
  invariants in <1 s per target.
- The `slow_render` pytest marker (see [Test strategy](#test-strategy))
  becomes unnecessary because there's no more "fast vs slow render"
  split.

**Concrete steps that move us toward this:**

- Extract a `tackon_sheet.rig` module from the patterns repeated in
  `_pirate_common`. Shared bone-rotation math, anatomy primitives
  (limb, torso, head + face), prop attachment points.
- Add a `scale` parameter to `build_sheet` / `write_canonical` that
  proportionally shrinks every dimension. Targets that opt in get
  fast tests; targets that don't stay on the slow path until ported.
- Each character migration is a self-contained PR that converts one
  bespoke target to declare a rig spec instead of running its own
  draw code.

## 2. Unified Target abstraction

**Where we are:** ✅ — `Target` Protocol with `TackonTarget` +
`AdapterTarget` impls landed in
[`target_registry.py`](ambition_sprite2d_renderer/target_registry.py).
Discovery walks tack-ons + main YAML configs + review NPC configs and
returns one unified `dict[str, Target]`. CLI consumes the registry
uniformly — `canonical <name>` / `sheet <name>` / `publish <name>`
work for any surface.

**Open follow-ups:**

- Bulk-port tack-on targets to expose the optional `render_canonical`
  hook (3-line wrapper around `tackon_sheet.write_canonical`). Drops
  the slow-fallback time for the gallery from minutes to seconds.
- Consider whether main YAML configs should still be a separate
  authoring path or fold into the rig from goal #1.

## 3. Test strategy

**Where we are:** the test suite is mostly "did the render pipeline
crash?" smokes. Each render takes seconds at full resolution, the
suite runs for minutes, and most assertions just check that
`render()` produced a file at the expected path. The genuine value
is in:

- Adapter animation contracts (`test_robot_target::test_robot_animation_contract`)
- Manifest schema checks (`test_creator_lab_props`, `test_town_tileset`)
- Visual contract checks (`test_raid_enforcer` — counts red/dark pixels)
- Pose-rig math (`test_robot_target::test_robot_run_pose_…`)

That's ~25% of the file count carrying ~80% of the regression value.

**Where we want to be:** every test runs in milliseconds because
every render path supports a low-resolution mode (see goal #1). No
need for fast/slow markers — the suite is uniformly fast.

**Interim policy:** tests that require full-resolution rendering and
have no low-res equivalent yet are marked with
`@pytest.mark.slow_render` and skipped by default. Run them
explicitly with `pytest --run-slow-render`. As targets get ported
to support low-res rendering, their tests come back online.

The point isn't to delete the slow tests — they catch real
regressions when run. The point is that running every render at full
size for the regression net was the wrong default while we have a
3,500-line `toon_side.py` that can only draw at one size.

## 4. Documentation

**Where we are:** README covers the CLI surface, the "Adding a new
sprite" walkthrough, and the category split. `docs/design.md`
captures architecture rationale. `target_registry.py` module
docstring is the authoritative Target API contract.

**Where we want to be:**

- README stays as the practical walkthrough.
- This file (`GOALS.md`) stays as the strategic direction.
- `docs/design.md` stays as architecture rationale, kept in sync
  with what's actually in the code.
- `target_registry.py` docstring stays as the API spec.

Each document has one job. None of them duplicates the others.
